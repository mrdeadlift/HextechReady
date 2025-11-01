# LoL Auto Accept (Rust)

Rust/egui desktop client that watches the League of Legends lobby for the **Match Found** dialog, auto-clicks the *Accept* button, and streams logs back into the GUI. This repository is the starting point for the Python → Rust rewrite described in the project brief.

## Highlights

- ✅ Native Windows UI written with `egui`/`eframe` (macOS support planned)
- ✅ Background worker captures the chosen monitor via `screenshots`, runs NCC-based template matching, and clicks using `enigo`
- ✅ Configurable threshold, polling interval, cooldown, monitor index and click-offset
- ✅ Live log console (driven by `tracing`) and simple status telemetry
- ✅ Persisted `confy`/`toml` configuration in `%APPDATA%/lol-auto-accept-rs`
- ✅ Basic regression tests for template matching with sample assets
- ✅ Resources folder with placeholder template + mock screenshots to unblock development

## Getting Started

```bash
rustup default stable          # requires Rust 1.70+ (eframe 0.28 baseline)
cargo run                      # launches the GUI
cargo test                     # runs regression tests
cargo fmt && cargo clippy      # optional hygiene checks
cargo build --release          # produces target/release/lol-auto-accept-rs.exe
```

### Runtime Dependencies

- Windows 10/11 (tested); macOS support will require additional QA
- League of Legends running windowed or borderless (monitor capture only)
- Template image located at `resources/templates/accept_button.png` or supplied through the GUI

## User Workflow

1. **Launch** the binary (see build command above). The GUI opens with start/stop buttons and current status.
2. **Configure** monitoring parameters if desired:
   - Threshold (0–1, default 0.88)
   - Polling interval in milliseconds (default 120)
   - Click cooldown to avoid multi-fire (default 4000 ms)
   - Monitor selection + click offsets
   - Template path override (blank = auto-locate bundled template)
3. **Start** monitoring. A background thread captures the monitor, runs template matching, and issues clicks when `score >= threshold` and cooldown has elapsed.
4. **Observe logs** in the lower panel. Detection and click events update the status line while detailed trace messages stream in the log console.
5. **Stop** monitoring at any time. Exiting the app will automatically stop the worker and close the window.

Known limitations, GPU capture caveats, and troubleshooting steps live in `docs/USER_GUIDE.md`.

## Configuration

Settings live in `%APPDATA%/lol-auto-accept-rs/config.toml` (Windows) or the OS equivalent handled by `confy`. Defaults can be edited live in the GUI or directly in the file.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `threshold` | `f32` | `0.88` | NCC score required to trigger the accept click |
| `interval_ms` | `u64` | `120` | Delay between capture/detect cycles |
| `cooldown_ms` | `u64` | `4000` | Minimum time between successive clicks |
| `monitor_index` | `usize` | `0` | Index into the enumerated monitor list |
| `click_offset_x`/`click_offset_y` | `i32` | `0` | Pixel offset applied to the detected template center |
| `template_path` | `Option<Path>` | `null` | Optional custom template image path. Empty = auto search `resources/templates/accept_button.png` near the binary |

## Project Structure

```
src/
 ├─ main.rs           # eframe bootstrap + native options
 ├─ lib.rs            # crate exports for integration tests
 ├─ app.rs            # egui UI, worker orchestration, channel plumbing
 ├─ capture.rs        # monitor enumeration + RGBA→grayscale conversion
 ├─ detect.rs         # NCC matching using imageproc
 ├─ input.rs          # Enigo click helper
 ├─ config.rs         # Confy-backed configuration helpers
 └─ logpipe.rs        # tracing subscriber that fans out to GUI + stderr
resources/
 ├─ templates/accept_button.png          # placeholder accept button template
 └─ samples/{positive,negative}_mock.png # mock data for tests
tests/
 └─ detect_tests.rs   # regression checks against mock assets
docs/
 ├─ USER_GUIDE.md
 └─ DEVELOPMENT.md
```

## Testing

- `cargo test` – runs integration tests with mock assets
- `cargo test --test detect_tests -- --nocapture` – observe detailed scores
- (Future) add automated GUI smoke tests / benchmarking harness

## Packaging & Release

1. `cargo build --release`
2. Bundle `target/release/lol-auto-accept-rs.exe` with the `resources/` directory
3. (Optional) Zip artifacts for distribution, add README/licence to archive
4. For GitHub releases, automate using GitHub Actions (workflow skeleton TODO)

macOS binaries can be produced with the same command once platform-specific capture/input tweaks are validated.

## Next Steps / TODO

- Replace placeholder template with production assets from the Python version
- Expand monitoring to support multiple template variants (resolution/language)
- macOS input/capture validation + notarization pipeline
- Performance instrumentation (ensure <150 ms scan/click loop)
- GitHub Actions workflows (`cargo fmt`, `cargo clippy`, release build artifacts)
- Optional OpenCV backend or SIMD-optimized NCC for higher accuracy

## Licensing

This project currently ships with an MIT license (see `LICENSE`). Update if corporate/commercial requirements differ.

---

Questions or ideas? See `docs/DEVELOPMENT.md` for architecture details or reach out via the project issue tracker.
