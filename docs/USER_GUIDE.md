# User Guide

This document explains how to operate the Rust rewrite of **LoL Auto Accept**. It assumes you already have a working build (see `README.md`).

## 1. Launching the App

1. Run `lol-auto-accept-rs.exe` (Windows) or the equivalent macOS binary once available.
2. The window shows:
   - **Start / Stop / Exit** buttons
   - Status line (current state, last detection/click)
   - Config panel (threshold, interval, cooldown, monitor, template, offsets)
   - Live log console fed by the background worker

> **Note:** The binary looks for `resources/templates/accept_button.png` next to the executable by default. Provide your own template via the GUI if you have a better capture from your client.

## 2. Basic Workflow

1. **Select monitor**: pick the display where the League client lives. Use *Refresh* after plugging in or re-arranging monitors.
2. **Adjust settings** (optional):
   - Threshold: higher = fewer false positives, lower = more sensitivity.
   - Polling interval: shorter = quicker reaction, higher CPU cost.
   - Cooldown: prevents multi-fire on laggy clients.
   - Click offset: shift the click if the detected center differs from the actual accept button location.
   - Template path: point at a custom PNG; leave blank to auto-discover.
3. **Start monitoring**: the worker thread runs until you press Stop/Exit.
4. **Watch the log/status**:
   - Detection status shows match score and coordinates.
   - Click events and cooldown skips are reported in the status line and logs.
5. **Stop** or **Exit** when you're done.

## 3. Configuration File

Settings are saved automatically to:

- Windows: `%APPDATA%\lol-auto-accept-rs\config.toml`
- macOS (future): `~/Library/Application Support/lol-auto-accept-rs/config.toml`

Edit the file manually or use the GUI + 'Save configuration' button.

## 4. Template Assets

- Bundled placeholder template lives at `resources/templates/accept_button.png`.
- Replace with a crisp capture from your client (PNG recommended, no scaling).
- For multiple resolutions/languages, plan to add a template selector/UI in a future iteration.

## 5. Troubleshooting

| Problem | Possible Cause | Suggested Fix |
| --- | --- | --- |
| Status shows `Template image not found` | Template path invalid | Use the **Reset** button or point to a valid PNG |
| High CPU usage | Polling interval very low | Increase `interval_ms` (e.g., 150-200 ms) |
| Missed matches | Threshold too high / template mismatch | Lower threshold slightly or capture a new template |
| Wrong monitor clicked | Monitor index or offsets off | Refresh monitor list and adjust offsets |
| Enigo click unsupported | Elevated privileges required | Run as administrator or reconfigure game window focus |
| Capture fails with AMD/NVIDIA screen recorders | Driver/GPU overlay conflict | Disable conflicting overlays, fall back to windowed mode |

## 6. Safety Notes

- The app simulates mouse movement/clicks. Stop monitoring if you plan to tab out or use the mouse manually.
- No attempt is made to bring the client window to the foreground; you remain responsible for focus management.
- Respect Riot's terms of service when using automation.

## 7. Logging

- Logs mirror to stderr (console) and to the GUI log panel.
- For deeper debugging, run with `RUST_LOG=debug cargo run` - the env filter is respected.
- Future work may add file-based logging if required.

## 8. Known Limitations

- Only tested on single-monitor setups with LoL in 100% DPI scaling.
- Template matching is NCC-based without pyramids; extreme resolution changes need new templates.
- macOS build path is unfinished - input/capture code compiles but needs QA.
- No auto-updater; distribution is manual for now.

---

Need more? Refer to `docs/DEVELOPMENT.md` for architecture internals or open an issue describing your environment.

---

## 日本語ガイド

このドキュメントは、Rust で書き直された **LoL Auto Accept** の操作方法を説明します。ビルド手順については `README.md` を参照し、実行可能ファイルが用意済みであることを前提とします。

### 1. アプリの起動

1. Windows の場合は `lol-auto-accept-rs.exe` を実行します。macOS 向けバイナリは今後提供予定です。
2. ウィンドウには次の要素が表示されます:
   - **Start / Stop / Exit** ボタン
   - ステータスライン (現在の状態、直近の検出/クリック)
   - 設定パネル (threshold, interval, cooldown, monitor, template, offsets)
   - バックグラウンドワーカーから送られるライブログコンソール

> **補足:** 既定では実行ファイルと同じ階層にある `resources/templates/accept_button.png` を参照します。より適したテンプレートがある場合は、GUI から任意のファイルを指定してください。

### 2. 基本的な流れ

1. **Select monitor**: League クライアントが表示されているディスプレイを選択します。モニター構成を変更したら *Refresh* を押してください。
2. **Adjust settings** (任意):
   - Threshold: 高くすると誤検出が減り、低くすると感度が上がります。
   - Polling interval: 短くすると反応が速くなりますが、CPU 負荷が増えます。
   - Cooldown: ラグのあるクライアントで多重クリックが発生するのを防ぎます。
   - Click offset: 検出した中心と実際の Accept ボタン位置がずれる場合にクリック位置を補正します。
   - Template path: 独自の PNG を指定します。空欄なら自動検出を行います。
3. **Start monitoring**: Stop/Exit を押すまでワーカースレッドが監視を続けます。
4. **Watch the log/status**:
   - 検出状況には一致度と座標が表示されます。
   - クリックイベントやクールダウンによるスキップはステータスラインとログに記録されます。
5. 作業が終わったら **Stop** または **Exit** を押します。

### 3. 設定ファイル

設定は自動的に次の場所へ保存されます:

- Windows: `%APPDATA%\lol-auto-accept-rs\config.toml`
- macOS (予定): `~/Library/Application Support/lol-auto-accept-rs/config.toml`

ファイルを直接編集するか、GUI の 'Save configuration' ボタンを使用してください。

### 4. テンプレート素材

- 同梱のプレースホルダーテンプレートは `resources/templates/accept_button.png` にあります。
- ゲームクライアントから高品質なキャプチャを取得し、PNG 形式 (拡大縮小なし) で差し替えてください。
- 解像度や言語が複数ある場合は、将来的にテンプレート選択 UI を追加する計画です。

### 5. トラブルシューティング

| 問題 | 想定される原因 | 対処方法 |
| --- | --- | --- |
| ステータスに `Template image not found` と表示される | テンプレートパスが無効 | **Reset** ボタンを押すか、有効な PNG を指定する |
| CPU 使用率が高い | Polling interval が短すぎる | `interval_ms` を増やす (例: 150-200 ms) |
| マッチングに失敗する | Threshold が高すぎる / テンプレートの不一致 | Threshold を少し下げるか、新しいテンプレートを取得する |
| 誤ったモニターでクリックする | モニター番号またはオフセットがずれている | モニター一覧を更新し、オフセットを調整する |
| Enigo でクリックできない | 管理者権限が必要 | 管理者として実行するか、ゲームウィンドウのフォーカス設定を見直す |
| AMD/NVIDIA の画面録画でキャプチャできない | ドライバーや GPU オーバーレイの競合 | 競合するオーバーレイを無効化し、ウィンドウモードに切り替える |

### 6. 注意事項

- アプリはマウスの移動とクリックを擬似的に行います。別作業をする場合は監視を停止してください。
- クライアントウィンドウを前面に持ってくる処理は行いません。フォーカス管理はユーザーの責任です。
- 自動化ツールを使用する際は Riot の利用規約を遵守してください。

### 7. ログ

- ログは stderr (コンソール) と GUI のログパネルに出力されます。
- さらに詳細なデバッグが必要な場合は `RUST_LOG=debug cargo run` を実行すると、環境変数のフィルターが反映されます。
- 必要であれば将来的にファイル出力のログ機能を追加する予定です。

### 8. 既知の制限

- LoL を 100% DPI スケーリングで動かす単一モニター構成でのみ検証しています。
- テンプレートマッチングは NCC ベースでピラミッドを使用しません。大幅な解像度変更には新しいテンプレートが必要です。
- macOS 向けビルドパスは未完成で、入出力/キャプチャコードはコンパイルできますが QA が必要です。
- 自動アップデートは未対応で、現状は手動配布です。

---

詳しく知りたい場合は `docs/DEVELOPMENT.md` を参照するか、環境を添えて Issue を作成してください。
