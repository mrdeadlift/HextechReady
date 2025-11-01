use std::path::Path;

use anyhow::{Context, Result};
use image::{GrayImage, ImageBuffer, Luma};
use imageproc::template_matching::{MatchTemplateMethod, match_template};

#[derive(Clone)]
pub struct Template {
    image: GrayImage,
}

impl Template {
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn as_image(&self) -> &GrayImage {
        &self.image
    }
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub score: f32,
    pub position: (u32, u32),
}

pub fn load_template(path: &Path) -> Result<Template> {
    let dyn_img = image::open(path).with_context(|| format!("Failed to load template {path:?}"))?;
    Ok(Template {
        image: dyn_img.into_luma8(),
    })
}

pub fn detect(frame: &GrayImage, template: &Template) -> Option<Detection> {
    if frame.width() < template.width() || frame.height() < template.height() {
        return None;
    }

    let result: ImageBuffer<Luma<f32>, Vec<f32>> = match_template(
        frame,
        template.as_image(),
        MatchTemplateMethod::CrossCorrelationNormalized,
    );

    find_peak(&result).map(|(score, x, y)| Detection {
        score,
        position: (x, y),
    })
}

fn find_peak(result: &ImageBuffer<Luma<f32>, Vec<f32>>) -> Option<(f32, u32, u32)> {
    let mut best: Option<(f32, u32, u32)> = None;
    for (x, y, pixel) in result.enumerate_pixels() {
        let score = pixel[0];
        match best {
            Some((best_score, _, _)) if score <= best_score => {}
            _ => best = Some((score, x, y)),
        }
    }
    best
}
