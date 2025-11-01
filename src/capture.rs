use anyhow::{Context, Result, anyhow};
use image::{DynamicImage, GrayImage, ImageBuffer, Rgba};
use screenshots::{Screen, display_info::DisplayInfo};

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
    pub origin_x: i32,
    pub origin_y: i32,
}

impl From<DisplayInfo> for MonitorInfo {
    fn from(value: DisplayInfo) -> Self {
        Self {
            id: value.id,
            name: format!("Display {}", value.id),
            width: value.width,
            height: value.height,
            scale_factor: value.scale_factor,
            is_primary: value.is_primary,
            origin_x: value.x,
            origin_y: value.y,
        }
    }
}

pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>> {
    let mut monitors = Vec::new();
    for display in DisplayInfo::all().context("Failed to enumerate displays")? {
        monitors.push(display.into());
    }
    Ok(monitors)
}

pub struct CapturedFrame {
    pub image: GrayImage,
    pub origin: (i32, i32),
    pub scale_factor: f32,
}

pub fn capture_monitor_gray(monitor_index: usize) -> Result<CapturedFrame> {
    let screens = Screen::all().context("Unable to list screens")?;
    let screen = screens
        .get(monitor_index)
        .with_context(|| format!("Monitor index {monitor_index} is out of bounds"))?;

    let rgba = screen.capture().context("Failed to capture screen")?;
    let (width, height) = (rgba.width(), rgba.height());
    let raw = rgba.into_vec();
    let rgba_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, raw)
        .ok_or_else(|| {
            anyhow!(
                "Unable to rebuild image buffer for monitor {}",
                screen.display_info.id
            )
        })?;
    let gray = DynamicImage::ImageRgba8(rgba_buffer).into_luma8();

    Ok(CapturedFrame {
        image: gray,
        origin: (screen.display_info.x, screen.display_info.y),
        scale_factor: screen.display_info.scale_factor,
    })
}
