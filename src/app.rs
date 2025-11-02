use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use egui::{Align, ComboBox, Layout, RichText};
use tracing::{error, info, warn};

use crate::{
    capture::{self, CapturedFrame, MonitorInfo},
    config::{self, AppConfig},
    detect::{self, Template},
    input,
};

const MAX_LOG_ENTRIES: usize = 500;

pub struct LolAutoAcceptApp {
    config: AppConfig,
    saved_config: AppConfig,
    monitors: Vec<MonitorInfo>,
    running: bool,
    worker: Option<WorkerHandle>,
    events_rx: Option<Receiver<WorkerEvent>>,
    log_rx: Receiver<String>,
    logs: VecDeque<String>,
    last_detection: Option<DetectionSnapshot>,
    status_line: String,
    exit_requested: bool,
    template_path_input: String,
    last_config_error: Option<String>,
}

impl LolAutoAcceptApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: AppConfig,
        log_rx: Receiver<String>,
    ) -> Self {
        let monitors = capture::enumerate_monitors().unwrap_or_default();
        let mut config = config;
        if !monitors.is_empty() && config.monitor_index >= monitors.len() {
            config.monitor_index = 0;
        }
        let template_path_input = config
            .template_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        Self {
            saved_config: config.clone(),
            config,
            monitors,
            running: false,
            worker: None,
            events_rx: None,
            log_rx,
            logs: VecDeque::new(),
            last_detection: None,
            status_line: "Idle".to_string(),
            exit_requested: false,
            template_path_input,
            last_config_error: None,
        }
    }

    fn start_monitoring(&mut self) {
        if self.running {
            return;
        }

        if let Err(err) = self.apply_template_path_from_input() {
            self.last_config_error = Some(err.to_string());
            self.status_line = "Template path error".to_string();
            error!(error = ?err, "failed to parse template path");
            return;
        } else {
            self.last_config_error = None;
        }

        match self.spawn_worker() {
            Ok(_) => {
                self.running = true;
                self.status_line = "Monitoring...".to_string();
                info!("Monitoring started");
            }
            Err(err) => {
                self.status_line = format!("Failed to start: {err:#}");
                error!(error = ?err, "failed to start worker");
            }
        }
    }

    fn stop_monitoring(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.request_stop();
            worker.join();
            self.status_line = "Stopped".to_string();
            info!("Monitoring stopped");
        }
        self.running = false;
    }

    fn refresh_monitors(&mut self) {
        match capture::enumerate_monitors() {
            Ok(list) => {
                self.monitors = list;
                if !self.monitors.is_empty() && self.config.monitor_index >= self.monitors.len() {
                    self.config.monitor_index = 0;
                }
                info!("Monitor list refreshed");
            }
            Err(err) => {
                error!(error = ?err, "failed to refresh monitor list");
                self.status_line = format!("Monitor refresh failed: {err:#}");
            }
        }
    }

    fn poll_logs(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        while let Ok(line) = self.log_rx.try_recv() {
            self.push_log(line);
            updated = true;
        }
        if updated {
            ctx.request_repaint();
        }
    }

    fn poll_events(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        if let Some(rx) = &self.events_rx {
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            if !events.is_empty() {
                updated = true;
            }
            for event in events {
                self.handle_event(event);
            }
        }
        if updated {
            ctx.request_repaint();
        }
    }

    fn check_worker_lifecycle(&mut self) {
        if let Some(worker) = self.worker.as_mut() {
            if worker.is_finished() {
                worker.join();
                self.worker = None;
                self.running = false;
                self.status_line = "Worker exited".to_string();
            }
        }
    }

    fn push_log(&mut self, line: String) {
        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(line);
    }

    fn handle_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Detection {
                score,
                image_coords,
                screen_coords,
                template_size,
                scale,
            } => {
                self.last_detection = Some(DetectionSnapshot {
                    timestamp: Instant::now(),
                    score,
                    image_coords,
                    screen_coords,
                    template_size,
                    scale,
                });
                self.status_line = format!(
                    "Detected @ ({}, {}) score {:.3} scale {:.2}",
                    screen_coords.0, screen_coords.1, score, scale
                );
            }
            WorkerEvent::Clicked { screen_coords } => {
                self.status_line = format!("Clicked at ({}, {})", screen_coords.0, screen_coords.1);
            }
            WorkerEvent::CooldownActive {
                remaining_ms,
                score,
            } => {
                self.status_line = format!(
                    "Cooldown active ({remaining_ms} ms remaining), last score {:.3}",
                    score
                );
            }
            WorkerEvent::Error(message) => {
                self.status_line = format!("Worker error: {message}");
                warn!("Worker error: {message}");
            }
            WorkerEvent::Info(message) => {
                self.status_line = message;
            }
            WorkerEvent::Stopped => {
                self.running = false;
                self.status_line = "Worker stopped".to_string();
            }
        }
    }

    fn spawn_worker(&mut self) -> Result<()> {
        let config = self.config.clone();
        let template_path = config
            .resolve_template_path()
            .context("Template image lookup failed")?;
        let template = detect::load_template(&template_path)?;
        let (tx, rx) = crossbeam_channel::unbounded();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = stop_flag.clone();

        let handle = thread::Builder::new()
            .name("lol-auto-accept-worker".to_string())
            .spawn(move || run_worker(config, template, tx, worker_stop))
            .context("Failed to spawn worker thread")?;

        self.worker = Some(WorkerHandle {
            stop_flag,
            thread: Some(handle),
        });
        self.events_rx = Some(rx);
        Ok(())
    }

    fn save_configuration(&mut self) {
        if let Err(err) = self.apply_template_path_from_input() {
            self.last_config_error = Some(err.to_string());
            self.status_line = "Template path error".to_string();
            return;
        }

        match config::store(&self.config) {
            Ok(_) => {
                self.saved_config = self.config.clone();
                self.status_line = "Configuration saved".to_string();
                self.last_config_error = None;
                info!("Configuration saved");
            }
            Err(err) => {
                self.status_line = format!("Failed to save config: {err:#}");
                error!(error = ?err, "failed to save configuration");
            }
        }
    }

    fn apply_template_path_from_input(&mut self) -> Result<()> {
        let trimmed = self.template_path_input.trim();
        if trimmed.is_empty() {
            self.config.template_path = None;
        } else {
            let path = PathBuf::from(trimmed);
            if !path.exists() {
                return Err(anyhow::anyhow!("Template path {trimmed} does not exist"));
            }
            self.config.template_path = Some(path);
        }
        Ok(())
    }

    fn render_status_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("LoL Auto Accept (Rust)");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("Start"))
                    .clicked()
                {
                    self.start_monitoring();
                }
                if ui
                    .add_enabled(self.running, egui::Button::new("Stop"))
                    .clicked()
                {
                    self.stop_monitoring();
                }
                if ui.button("Exit").clicked() {
                    self.exit_requested = true;
                }
            });
        });
        ui.separator();
        ui.label(RichText::new(&self.status_line).strong());
        if let Some(snapshot) = &self.last_detection {
            ui.label(format!(
                "Last detection: {:.3} score at screen ({}, {}) – image ({}, {}) – template {}x{} (scale {:.2}) – {} ago",
                snapshot.score,
                snapshot.screen_coords.0,
                snapshot.screen_coords.1,
                snapshot.image_coords.0,
                snapshot.image_coords.1,
                snapshot.template_size.0,
                snapshot.template_size.1,
                snapshot.scale,
                format_duration(snapshot.timestamp.elapsed())
            ));
        } else {
            ui.label("No detections yet");
        }
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Monitoring Settings")
            .default_open(true)
            .show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.config.threshold, 0.5..=0.99)
                        .text("Match threshold")
                        .suffix(" score"),
                );

                ui.horizontal(|ui| {
                    ui.label("Polling interval (ms)");
                    ui.add(egui::DragValue::new(&mut self.config.interval_ms).speed(5));
                });

                ui.horizontal(|ui| {
                    ui.label("Cooldown (ms)");
                    ui.add(egui::DragValue::new(&mut self.config.cooldown_ms).speed(10));
                });

                ui.horizontal(|ui| {
                    ui.label("Click offset X");
                    ui.add(egui::DragValue::new(&mut self.config.click_offset_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.config.click_offset_y).speed(1));
                });

                ui.horizontal(|ui| {
                    ui.label("Monitor");
                    let count = self.monitors.len();
                    ComboBox::from_id_source("monitor_selector")
                        .selected_text(monitor_label(
                            self.config.monitor_index,
                            self.monitors.get(self.config.monitor_index),
                            count,
                        ))
                        .show_ui(ui, |ui| {
                            for (index, info) in self.monitors.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.config.monitor_index,
                                    index,
                                    monitor_label(index, Some(info), count),
                                );
                            }
                        });

                    if ui.button("Refresh").clicked() {
                        self.refresh_monitors();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Template path");
                    let response = ui.text_edit_singleline(&mut self.template_path_input);
                    if response.changed() {
                        self.last_config_error = None;
                    }
                    if ui.button("Reset").clicked() {
                        self.template_path_input.clear();
                        self.config.template_path = None;
                    }
                });

                if let Some(err) = &self.last_config_error {
                    ui.label(RichText::new(err).color(egui::Color32::RED));
                }

                let dirty = self.config != self.saved_config
                    || self.template_path_input
                        != self
                            .saved_config
                            .template_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();

                ui.horizontal(|ui| {
                    if ui.button("Save configuration").clicked() {
                        self.save_configuration();
                    }
                    if dirty {
                        ui.label(RichText::new("Unsaved changes").italics());
                    }
                });
            });
    }

    fn render_logs(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Logs")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.logs {
                            ui.label(line);
                        }
                    });
            });
    }
}

impl eframe::App for LolAutoAcceptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_logs(ctx);
        self.poll_events(ctx);
        self.check_worker_lifecycle();

        if self.exit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            self.render_status_panel(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_settings(ui);
            ui.separator();
            self.render_logs(ui);
        });

        if self.running {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

struct WorkerHandle {
    stop_flag: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl WorkerHandle {
    fn request_stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    fn join(&mut self) {
        if let Some(handle) = self.thread.take() {
            if let Err(err) = handle.join() {
                error!("Worker thread join error: {err:?}");
            }
        }
    }

    fn is_finished(&self) -> bool {
        matches!(&self.thread, Some(handle) if handle.is_finished())
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.request_stop();
        self.join();
    }
}

#[derive(Debug)]
struct DetectionSnapshot {
    timestamp: Instant,
    score: f32,
    image_coords: (u32, u32),
    screen_coords: (i32, i32),
    template_size: (u32, u32),
    scale: f32,
}

enum WorkerEvent {
    Detection {
        score: f32,
        image_coords: (u32, u32),
        screen_coords: (i32, i32),
        template_size: (u32, u32),
        scale: f32,
    },
    Clicked {
        screen_coords: (i32, i32),
    },
    CooldownActive {
        score: f32,
        remaining_ms: u64,
    },
    Error(String),
    Info(String),
    Stopped,
}

fn run_worker(
    config: AppConfig,
    template: Template,
    events_tx: Sender<WorkerEvent>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut last_click = None;
    let cooldown = Duration::from_millis(config.cooldown_ms);
    let interval = Duration::from_millis(config.interval_ms.max(10));
    info!(
        monitor = config.monitor_index,
        threshold = config.threshold,
        cooldown_ms = config.cooldown_ms,
        interval_ms = config.interval_ms,
        "worker started"
    );

    if events_tx
        .send(WorkerEvent::Info("Monitoring active".to_string()))
        .is_err()
    {
        return;
    }

    while !stop_flag.load(Ordering::Relaxed) {
        match capture::capture_monitor_gray(config.monitor_index) {
            Ok(frame) => handle_frame(
                &config,
                &template,
                &events_tx,
                frame,
                &mut last_click,
                cooldown,
            ),
            Err(err) => {
                error!(error = ?err, "screen capture failed");
                let _ = events_tx.send(WorkerEvent::Error(format!("Capture failed: {err:#}")));
                thread::sleep(Duration::from_millis(250));
            }
        }

        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        thread::sleep(interval);
    }

    let _ = events_tx.send(WorkerEvent::Stopped);
    info!("worker stopped");
}

fn handle_frame(
    config: &AppConfig,
    template: &Template,
    events_tx: &Sender<WorkerEvent>,
    frame: CapturedFrame,
    last_click: &mut Option<Instant>,
    cooldown: Duration,
) {
    if let Some(result) = detect::detect(&frame.image, template) {
        if result.score < config.threshold {
            return;
        }

        let now = Instant::now();
        if let Some(last) = last_click {
            let elapsed = now.duration_since(*last);
            if elapsed < cooldown {
                let remaining = cooldown.saturating_sub(elapsed);
                let _ = events_tx.send(WorkerEvent::CooldownActive {
                    score: result.score,
                    remaining_ms: remaining.as_millis() as u64,
                });
                return;
            }
        }

        let template_half_w = (result.template_size.0 as i32) / 2;
        let template_half_h = (result.template_size.1 as i32) / 2;

        let screen_x =
            frame.origin.0 + result.position.0 as i32 + template_half_w + config.click_offset_x;
        let screen_y =
            frame.origin.1 + result.position.1 as i32 + template_half_h + config.click_offset_y;

        let _ = events_tx.send(WorkerEvent::Detection {
            score: result.score,
            image_coords: result.position,
            screen_coords: (screen_x, screen_y),
            template_size: result.template_size,
            scale: result.scale,
        });

        if let Err(err) = input::click_at(screen_x, screen_y) {
            error!(error = ?err, "failed to click accept button");
            let _ = events_tx.send(WorkerEvent::Error(format!("Click failed: {err:#}")));
            return;
        }

        info!(
            score = result.score,
            scale = result.scale,
            template_width = result.template_size.0,
            template_height = result.template_size.1,
            screen_x,
            screen_y,
            "accept button clicked"
        );
        let _ = events_tx.send(WorkerEvent::Clicked {
            screen_coords: (screen_x, screen_y),
        });
        *last_click = Some(now);
    }
}

fn monitor_label(index: usize, info: Option<&MonitorInfo>, total: usize) -> String {
    match info {
        Some(monitor) => format!(
            "#{index} • {}x{} @ {:.0}%{}{}",
            monitor.width,
            monitor.height,
            monitor.scale_factor * 100.0,
            if monitor.is_primary {
                " • primary"
            } else {
                ""
            },
            if total > 1 {
                format!(" • id {}", monitor.id)
            } else {
                String::new()
            }
        ),
        None => format!("#{index} (disconnected)"),
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs > 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else if secs > 0 {
        format!("{secs}s")
    } else {
        format!("{}ms", duration.as_millis())
    }
}
