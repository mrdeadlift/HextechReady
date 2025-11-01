# Development Notes

This document captures the architectural decisions and technical details for the Rust rewrite of **LoL Auto Accept**.

## 1. Architecture Overview

```
┌────────────┐        ┌──────────────┐        ┌─────────────┐
│ egui UI    │◄──────►│ crossbeam tx │◄──────►│ Worker loop │
│ (app.rs)   │ logs   │ / rx         │ events │ (thread)    │
└─────┬──────┘        └──────────────┘        └─────┬───────┘
      │ tracing subscriber (logpipe.rs)             │
      ▼                                             ▼
 ┌──────────────┐                           ┌────────────────┐
 │ Rerender tick│                           │ capture.rs      │
 │ + status     │                           │ detect.rs       │
 │              │                           │ input.rs        │
 └──────────────┘                           └────────────────┘
```

- `main.rs` boots `eframe` and hands over the initial config + log receiver to `LolAutoAcceptApp`.
- `app.rs` owns UI state, config editing, log buffer, and the worker lifecycle.
- `logpipe.rs` sets up a `tracing_subscriber` that writes to both stderr and an in-memory channel consumed by the GUI.
- The worker thread captures using `screenshots`, converts RGBA → grayscale, runs the NCC matcher, and calls `enigo` to click when ready. It streams structured events back to the UI thread via `crossbeam-channel`.

## 2. Module Responsibilities

| Module | Responsibility |
| --- | --- |
| `app.rs` | egui widgets, state management, worker orchestration, handling events/logs |
| `capture.rs` | Monitor enumeration (via `display-info`) and RGBA → grayscale capture |
| `detect.rs` | Template loading + normalized cross correlation (via `imageproc`) |
| `input.rs` | Cross-platform mouse click helper (`enigo`) |
| `config.rs` | `confy`-backed persistence, default values, template resolution |
| `logpipe.rs` | `tracing` subscriber that feeds the GUI log panel |
| `tests/detect_tests.rs` | Regression checks using bundled mock assets |

## 3. Worker Loop

`run_worker` (in `app.rs`) performs the following steps:

1. Load template (`detect::load_template`) and compute initial metadata.
2. Loop while `stop_flag` is false:
   1. Capture grayscale frame for the configured monitor (`capture::capture_monitor_gray`).
   2. Run NCC matching (`detect::detect`) to get the best score and coordinates.
   3. If `score >= threshold` and cooldown elapsed, compute click point (template center + offset + monitor origin) and `input::click_at`.
   4. Broadcast `WorkerEvent`s (Detection, Clicked, CooldownActive, Error) for the UI to render/log.
   5. Sleep for `interval_ms` before the next pass.
3. Send `WorkerEvent::Stopped` and exit.

The worker logs every significant step via `tracing`, so the GUI and console get real-time feedback.

## 4. Template Matching Details

- Uses `imageproc::template_matching::match_template` with `CrossCorrelationNormalized`.
- Template and captured frames are grayscale `ImageBuffer<Luma<u8>>`.
- The matcher returns an `ImageBuffer<Luma<f32>>` of scores; we pick the maximum.
- Thresholds map directly to NCC scores (1.0 = perfect correlation).
- For future accuracy/performance improvements:
  - Switch to multi-scale template search for differing resolutions.
  - Provide multiple template assets and aggregate across them.
  - Investigate SIMD-accelerated NCC or GPU-based correlation.

## 5. Configuration Persistence

- `confy::load/store` uses the app name `lol-auto-accept-rs`.
- The struct is `Serialize` + `Deserialize`, so adding fields requires default handling.
- `AppConfig::resolve_template_path` attempts: custom path → executable dir → current dir. It errors clearly if nothing is found.

## 6. Logging

- `logpipe::init_logging` builds two `fmt` layers: stderr + GUI channel.
- `EnvFilter` honors `RUST_LOG` (falls back to `info`).
- Any `tracing::info!` / `warn!` / `error!` statements in worker or UI flow directly to the GUI log buffer.
- The GUI stores the latest 500 log lines (ring buffer). Adjust by modifying `MAX_LOG_ENTRIES`.

## 7. Testing & Mock Assets

- Assets under `resources/templates` and `resources/samples` are 32×16 (template) and 160×90 (mock screens).
- Integration tests ensure NCC scores stay above/below guard rails. Replace with real captures as soon as they exist.
- Future additions:
  - Use `rstest` to parameterize multiple sample images.
  - Add bench harness to track NCC performance with larger resolutions.

## 8. CI / Automation (TODO)

Recommended GitHub Actions workflow:

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release
      - uses: actions/upload-artifact@v4
        with:
          name: lol-auto-accept-rs-windows
          path: |
            target/release/lol-auto-accept-rs.exe
            resources/**
            README.md
            LICENSE
```

Add signing/ notarization steps for macOS once the port is ready.

## 9. Troubleshooting (Dev)

- **Enigo build failures** – make sure platform-specific dependencies (X11, macOS frameworks) are available when targeting those OSes.
- **`screenshots` build errors** – Windows builds require the Windows SDK; macOS may need additional entitlements.
- **`eframe` panics** at startup – confirm GPU drivers support the default renderer; switch to WGPU if needed (`NativeOptions.renderer = Renderer::Wgpu`).
- **Hot reload of template** – currently template loads only once when the worker starts. Re-run the worker after changing the file.

## 10. Future Enhancements

- Hotkey to pause/ resume without touching the UI.
- Optional overlay (egui panel) to visualise match score.
- Logging to file (rotating) and crash reporting.
- Real template calibration flow (capture from within the app).
- Integration with vectors for multiple accept button states (e.g., ARAM, normals, event modes).

---

Keep this document up to date as new modules land or architectural assumptions change.
