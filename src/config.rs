use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "lol-auto-accept-rs";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub threshold: f32,
    pub interval_ms: u64,
    pub cooldown_ms: u64,
    pub monitor_index: usize,
    pub click_offset_x: i32,
    pub click_offset_y: i32,
    pub template_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            threshold: 0.88,
            interval_ms: 120,
            cooldown_ms: 4_000,
            monitor_index: 0,
            click_offset_x: 0,
            click_offset_y: 0,
            template_path: None,
        }
    }
}

impl AppConfig {
    pub fn resolve_template_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.template_path {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(anyhow!(
                "Configured template path {:?} does not exist",
                path
            ));
        }

        default_template_search_paths()
            .into_iter()
            .find(|path| path.exists())
            .ok_or_else(|| anyhow!("Template image not found in default locations"))
    }

    pub fn set_template_path_from_str(&mut self, value: &str) {
        if value.trim().is_empty() {
            self.template_path = None;
        } else {
            self.template_path = Some(PathBuf::from(value));
        }
    }
}

pub fn load_or_default() -> Result<AppConfig> {
    let cfg: AppConfig = confy::load(APP_NAME, None).context("Failed to load configuration")?;
    Ok(cfg)
}

pub fn store(config: &AppConfig) -> Result<()> {
    confy::store(APP_NAME, None, config).context("Failed to persist configuration")
}

fn default_template_search_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(
                dir.join("resources")
                    .join("templates")
                    .join("accept_button.png"),
            );
            candidates.push(dir.join("templates").join("accept_button.png"));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(
            current_dir
                .join("resources")
                .join("templates")
                .join("accept_button.png"),
        );
    }

    candidates
}
