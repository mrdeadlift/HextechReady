use std::path::PathBuf;

use lol_auto_accept_rs::detect;

fn template_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("templates")
        .join("accept_button.png")
}

fn sample_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("samples")
        .join(name)
}

#[test]
fn positive_sample_has_high_score() {
    let template = detect::load_template(&template_path()).expect("template loads");
    let sample = image::open(sample_path("positive_mock.png"))
        .expect("positive sample loads")
        .into_luma8();

    let detection = detect::detect(&sample, &template).expect("match not found");
    assert!(
        detection.score > 0.85,
        "expected high score, got {:.4}",
        detection.score
    );
}

#[test]
fn negative_sample_is_below_threshold() {
    let template = detect::load_template(&template_path()).expect("template loads");
    let sample = image::open(sample_path("negative_mock.png"))
        .expect("negative sample loads")
        .into_luma8();

    if let Some(detection) = detect::detect(&sample, &template) {
        assert!(
            detection.score < 0.88,
            "negative sample produced high score {:.4}",
            detection.score
        );
    }
}
