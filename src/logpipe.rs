use std::io::{Result as IoResult, Write};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() -> Result<Receiver<String>> {
    let (tx, rx) = unbounded();
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tracing=warn"));

    let gui_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(GuiMakeWriter { sender: tx.clone() })
        .with_target(false);

    let stdout_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(gui_layer)
        .try_init()?;

    Ok(rx)
}

#[derive(Clone)]
struct GuiMakeWriter {
    sender: Sender<String>,
}

impl<'a> fmt::MakeWriter<'a> for GuiMakeWriter {
    type Writer = GuiWriter;

    fn make_writer(&'a self) -> Self::Writer {
        GuiWriter {
            sender: self.sender.clone(),
            buffer: Vec::new(),
        }
    }
}

struct GuiWriter {
    sender: Sender<String>,
    buffer: Vec<u8>,
}

impl Write for GuiWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let msg = String::from_utf8_lossy(&self.buffer).trim().to_string();
        if !msg.is_empty() {
            let _ = self.sender.send(msg);
        }
        self.buffer.clear();
        Ok(())
    }
}

impl Drop for GuiWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
