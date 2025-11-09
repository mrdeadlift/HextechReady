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

| Module                  | Responsibility                                                             |
| ----------------------- | -------------------------------------------------------------------------- |
| `app.rs`                | egui widgets, state management, worker orchestration, handling events/logs |
| `capture.rs`            | Monitor enumeration (via `display-info`) and RGBA → grayscale capture      |
| `detect.rs`             | Template loading + normalized cross correlation (via `imageproc`)          |
| `input.rs`              | Cross-platform mouse click helper (`enigo`)                                |
| `config.rs`             | `confy`-backed persistence, default values, template resolution            |
| `logpipe.rs`            | `tracing` subscriber that feeds the GUI log panel                          |
| `tests/detect_tests.rs` | Regression checks using bundled mock assets                                |

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

# 開発メモ（日本語版）

このドキュメントでは、「LoL Auto Accept」の Rust によるリライトに関するアーキテクチャの判断と技術的な詳細をまとめています。

## 1. アーキテクチャ概要

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

- `main.rs` は `eframe` を起動し、初期設定とログ受信機を `LolAutoAcceptApp` に引き渡します。
- `app.rs` は UI 状態、設定編集、ログバッファ、ワーカーのライフサイクルを管理します。
- `logpipe.rs` は stderr と GUI が消費するインメモリチャネルの双方に書き込む `tracing_subscriber` を構成します。
- ワーカースレッドは `screenshots` でキャプチャし、RGBA をグレースケールに変換して NCC マッチャーを実行し、準備が整えば `enigo` でクリックします。イベントは `crossbeam-channel` 経由で UI スレッドにストリームされます。

## 2. モジュールごとの責務

| モジュール              | 役割                                                           |
| ----------------------- | -------------------------------------------------------------- |
| `app.rs`                | egui ウィジェット、状態管理、ワーカーの制御、イベント/ログ処理 |
| `capture.rs`            | モニタ列挙（`display-info`）と RGBA→ グレースケールキャプチャ  |
| `detect.rs`             | テンプレート読み込みと正規化相互相関（`imageproc`）            |
| `input.rs`              | クロスプラットフォームなマウスクリックヘルパー（`enigo`）      |
| `config.rs`             | `confy` バックエンドの永続化、デフォルト設定、テンプレート探索 |
| `logpipe.rs`            | GUI ログパネルへ流す `tracing` サブスクライバー                |
| `tests/detect_tests.rs` | 同梱モックアセットを用いたリグレッションテスト                 |

## 3. ワーカーループ

`app.rs` の `run_worker` は次の手順を実行します。

1. テンプレート（`detect::load_template`）を読み込み、初期メタデータを計算します。
2. `stop_flag` が `false` の間ループします。
   1. 設定されたモニタからグレースケールフレームをキャプチャします（`capture::capture_monitor_gray`）。
   2. NCC マッチング（`detect::detect`）を実行し、最高スコアと座標を取得します。
   3. `score >= threshold` かつクールダウンが経過していれば、クリックポイント（テンプレート中心 + オフセット + モニタ原点）を算出し、`input::click_at` を呼びます。
   4. `WorkerEvent`（Detection, Clicked, CooldownActive, Error）をブロードキャストし、UI が描画/ログできるようにします。
   5. 次のループまで `interval_ms` だけスリープします。
3. `WorkerEvent::Stopped` を送信して終了します。

ワーカーは重要なステップをすべて `tracing` でログ出力するため、GUI とコンソールの双方でリアルタイムに確認できます。

## 4. テンプレートマッチングの詳細

- `imageproc::template_matching::match_template` を `CrossCorrelationNormalized` で使用します。
- テンプレートとキャプチャしたフレームはどちらもグレースケールの `ImageBuffer<Luma<u8>>` です。
- マッチャーはスコアの `ImageBuffer<Luma<f32>>` を返し、最大値を採用します。
- 閾値は NCC スコアに直接対応します（1.0 = 完全一致）。
- 将来の精度/性能改善案:
  - 解像度の違いに対応する多段階テンプレート検索。
  - 複数のテンプレートアセットを用意し、結果を集約する。
  - SIMD を用いた NCC の高速化や GPU ベースの相関演算を検討する。

## 5. 設定の永続化

- `confy::load/store` はアプリ名 `lol-auto-accept-rs` を使用します。
- 構造体は `Serialize` と `Deserialize` を実装しているため、フィールド追加時はデフォルト値の扱いが必要です。
- `AppConfig::resolve_template_path` はカスタムパス → 実行ファイルのディレクトリ → 現在のディレクトリの順に探索し、見つからない場合は明示的にエラーを返します。

## 6. ログ出力

- `logpipe::init_logging` は stderr と GUI チャネル向けの 2 つの `fmt` レイヤーを構築します。
- `EnvFilter` は `RUST_LOG` を尊重し、指定がなければ `info` を既定とします。
- ワーカーや UI フローでの `tracing::info!` / `warn!` / `error!` は GUI のログバッファへ直接流れます。
- GUI は最新 500 行のログ（リングバッファ）を保持します。必要に応じて `MAX_LOG_ENTRIES` を変更してください。

## 7. テストとモックアセット

- `resources/templates` と `resources/samples` 配下のアセットはそれぞれ 32×16（テンプレート）と 160×90（モック画面）です。
- 統合テストは NCC スコアが閾値を上下することを確認します。実キャプチャが揃い次第、差し替えてください。
- 将来の追加案:
  - `rstest` を使って複数のサンプル画像をパラメータ化する。
  - 高解像度での NCC 性能を追跡するベンチハーネスを追加する。

## 8. CI / 自動化（TODO）

推奨する GitHub Actions ワークフロー:

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

macOS 版が整い次第、署名/公証ステップを追加してください。

## 9. トラブルシューティング（開発者向け）

- **Enigo のビルド失敗** – 各 OS 固有の依存関係（X11 や macOS Framework など）が揃っているか確認してください。
- **`screenshots` のビルドエラー** – Windows ビルドには Windows SDK が必要で、macOS では追加のエンタイトルメントを設定する場合があります。
- **起動時の `eframe` パニック** – GPU ドライバが既定レンダラーをサポートしているかを確認し、必要なら `NativeOptions.renderer = Renderer::Wgpu` へ切り替えてください。
- **テンプレートのホットリロード** – 現状ではワーカー開始時に一度だけテンプレートを読み込みます。ファイル変更後はワーカーを再実行してください。

## 10. 今後の拡張

- UI に触らずに一時停止/再開できるホットキー。
- マッチスコアを可視化するオーバーレイ（egui パネル）。
- ログファイル出力（ローテーション）とクラッシュレポート。
- アプリ内キャプチャによるテンプレート校正フロー。
- さまざまなマッチ状態（ARAM、ノーマル、イベントなど）に対応する複数テンプレートとの連携。

---

新しいモジュールが追加されたりアーキテクチャの前提が変わった場合は、随時このドキュメントを更新してください。
