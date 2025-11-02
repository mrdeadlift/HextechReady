use std::path::Path;

use anyhow::{Context, Result};
use image::{GrayImage, ImageBuffer, Luma, imageops::FilterType};
use imageproc::template_matching::{MatchTemplateMethod, match_template};

const TEMPLATE_SCALE_FACTORS: &[f32] = &[
    0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3,
];

#[derive(Clone)]
pub struct Template {
    variants: Vec<TemplateVariant>,
}

impl Template {
    fn variants(&self) -> &[TemplateVariant] {
        &self.variants
    }
}

#[derive(Clone)]
struct TemplateVariant {
    scale: f32,
    image: GrayImage,
}

impl TemplateVariant {
    fn scale(&self) -> f32 {
        self.scale
    }

    fn width(&self) -> u32 {
        self.image.width()
    }

    fn height(&self) -> u32 {
        self.image.height()
    }

    fn as_image(&self) -> &GrayImage {
        &self.image
    }
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub score: f32,
    pub position: (u32, u32),
    pub template_size: (u32, u32),
    pub scale: f32,
}

pub fn load_template(path: &Path) -> Result<Template> {
    let dyn_img = image::open(path).with_context(|| format!("Failed to load template {path:?}"))?;
    let base = dyn_img.into_luma8();
    Ok(Template {
        variants: build_variants(&base),
    })
}

pub fn detect(frame: &GrayImage, template: &Template) -> Option<Detection> {
    let mut best: Option<Detection> = None;

    for variant in template.variants() {
        if frame.width() < variant.width() || frame.height() < variant.height() {
            continue;
        }

        let result: ImageBuffer<Luma<f32>, Vec<f32>> = match_template(
            frame,
            variant.as_image(),
            MatchTemplateMethod::CrossCorrelationNormalized,
        );

        if let Some((score, x, y)) = find_peak(&result) {
            let detection = Detection {
                score,
                position: (x, y),
                template_size: (variant.width(), variant.height()),
                scale: variant.scale(),
            };

            if best
                .as_ref()
                .map_or(true, |current| detection.score > current.score)
            {
                best = Some(detection);
            }
        }
    }

    best
}

fn build_variants(base: &GrayImage) -> Vec<TemplateVariant> {
    let mut variants = Vec::new();
    for &scale in TEMPLATE_SCALE_FACTORS {
        if scale <= 0.0 {
            continue;
        }

        let new_w = ((base.width() as f32 * scale).round() as i32).max(1) as u32;
        let new_h = ((base.height() as f32 * scale).round() as i32).max(1) as u32;
        if new_w < 4 || new_h < 4 {
            continue;
        }

        let image = if (scale - 1.0).abs() < f32::EPSILON {
            base.clone()
        } else {
            image::imageops::resize(base, new_w, new_h, FilterType::Lanczos3)
        };

        variants.push(TemplateVariant { scale, image });
    }

    variants
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
