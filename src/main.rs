use anyhow::{Result, anyhow};
use lol_auto_accept_rs::{app, config, logpipe};

fn main() -> Result<()> {
    let log_rx = logpipe::init_logging()?;
    let initial_config = config::load_or_default()?;

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([520.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "LoL Auto Accept (Rust)",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(app::LolAutoAcceptApp::new(
                cc,
                initial_config.clone(),
                log_rx.clone(),
            )))
        }),
    )
    .map_err(|err| anyhow!("{err}"))?;

    Ok(())
}
