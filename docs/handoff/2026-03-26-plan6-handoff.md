# Plan 6 (src-desktop Tauri shell) — 引継ぎドキュメント

作成日: 2026-03-26
安定化確認日: 2026-03-26
ワークスペース全テスト: **160 passed, 0 failed**

---

## 引継ぎ先がまず実行すること

```sh
# ワークツリーに移動
cd /mnt/d/project/gloss/.worktrees/src-desktop

# 状態確認
git log --oneline -6
cargo check -p src-desktop   # エラーなしを確認
cargo test --workspace       # 全 160 テスト PASS を確認
```

---

## 現在のブランチ状態

| 項目 | 値 |
|------|----|
| 作業ブランチ | `feat/src-desktop` |
| ワークツリーパス | `/mnt/d/project/gloss/.worktrees/src-desktop` |
| メインブランチ | `main` |
| ワークツリー状態 | クリーン（`src-desktop/gen/` のみ未追跡 = Tauri ビルド生成物） |

### コミット履歴 (feat/src-desktop)

```
583e924  fix(src-desktop-layout): update test to include doc_id in FileLoaded event
7fd1c11  fix(src-desktop): DocId mismatch, unused import, sentinel, crate-type, icon
038cf35  feat(src-desktop): dispatch Tauri command + event loop
3a67303  feat(src-desktop): AppState and create_state()
848c20d  feat(src-desktop): TauriIme with VecDeque queue
9c2fd40  feat(src-desktop): NativeClipboard; fix Clipboard::get_text to &mut self
f37d210  feat(src-desktop): Tauri project scaffold
```

---

## 完了タスク一覧 (Tasks 1–5)

| Task | 内容 | 実装ファイル |
|------|------|------------|
| 1 | Tauri プロジェクトスキャフォルド | `src-desktop/Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `src/main.rs`, `src/lib.rs`, `icons/icon.png` |
| 2 | NativeClipboard | `src-desktop/src/clipboard.rs`, `src-desktop-types/src/traits.rs` (get_text → `&mut self`) |
| 3 | TauriIme | `src-desktop/src/ime.rs` |
| 4 | AppState + state.rs | `src-desktop/src/state.rs` |
| 5 | dispatch コマンド + イベントループ | `src-desktop/src/commands.rs`, `src-desktop-types/src/events.rs` |

### 計画書との重要な差異（実装済み）

以下は計画書のコードと実装が異なる箇所。次のタスクを実装する際に**この差異を前提とする**こと。

#### 差異 1: `AppEvent::FileLoaded` に `doc_id: DocId` フィールドが存在する

```rust
// src-desktop-types/src/events.rs — 実際の定義
FileLoaded { path: VfsPath, content: Vec<u8>, doc_id: DocId },
```

**理由:** AppCore と layout が別々の DocId カウンターを持つと同じファイルに別 ID が割り当てられるバグを修正。`dispatch_shell_cmds` が `ReadFile` を処理する際に `core.open_bytes()` を呼んで canonical DocId を取得し、それを event に含める。

#### 差異 2: `commands.rs` の imports と Model 操作

```rust
// 先頭 use 文
use src_desktop_types::{AppCmd, AppEvent, Clipboard, DrawCmd, ImeEvent, ImeSource};
use src_desktop_layout::{update, view, Model};

// Model は Clone 未実装なので std::mem::replace を使う
let model = std::mem::replace(&mut s.model, Model::new(0, 0));
let (m, cmds) = update(model, event);
s.model = m;
```

#### 差異 3: `Cargo.toml` の crate-type

```toml
crate-type = ["staticlib", "cdylib", "rlib"]   # rlib がないと main.rs が lib を使えない
```

---

## 残タスク一覧 (Tasks 6–9)

### Task 6: Frontend scaffold + renderer.ts

**参照:** `docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md` の Step 6.1〜6.11

作成するファイル（全て `src-desktop/frontend/` 以下）:
- `package.json` — `@tauri-apps/api@^2`, `typescript@^5`, `vite@^5`
- `tsconfig.json`
- `vite.config.ts` — `server.port: 1420`
- `index.html` — `#tab-bar`, `#main-area > #editor-canvas + #preview-pane`, `#status-bar`
- `src/types.ts` — DrawCmd / AppEvent の TypeScript 型定義
- `src/renderer.ts` — DrawCmd を各モジュールに振り分け
- `src/main.ts` — キーボード入力 → `invoke('dispatch', {event})` → `applyDrawCmds()`
- `src/style.css` — ダークテーマ
- `src/editor-canvas.ts` (stub)、`src/preview.ts` (stub)、`src/chrome.ts` (stub)、`src/ime-bridge.ts` (stub)

**確認コマンド:**
```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend
npm install && npx tsc --noEmit
```

---

### Task 7: view.rs カーソル位置修正 + editor-canvas.ts

**参照:** Plan 6 Step 7.1〜7.7

**Rust 変更 (`src-desktop-layout/src/view.rs`):**
```rust
// 追加する定数（pub にして TypeScript 側と共有する値として文書化）
pub const CHAR_W: f32 = 8.0;   // ASCII 1文字の幅 (px)
pub const LINE_H: f32 = 20.0;  // 1行の高さ (px)

// emit_editor_frame を修正: cursor_x/y を vm.cursor から計算
fn emit_editor_frame(cmds: &mut Vec<DrawCmd>, pane_id: PaneId, bounds: Rect, vm: &EditorViewModel) {
    let cursor_x = vm.cursor.visual_col as f32 * CHAR_W - vm.scroll.x;
    let cursor_y = vm.cursor.line       as f32 * LINE_H - vm.scroll.y;
    cmds.push(DrawCmd::EditorFrame {
        pane_id, bounds,
        lines:     vm.visible_lines.clone(),
        cursor:    CursorDraw { x: cursor_x, y: cursor_y, height: LINE_H },
        selection: None,
        preedit:   vm.preedit.clone(),
        scroll:    vm.scroll,
    });
}
```

**テスト注意:** 計画書の test で `VfsPath::new()` を使っているが**存在しない**。`VfsPath::from()` を使うこと。

```rust
// 正しい
path: VfsPath::from("/test.n.md"),
// 計画書のコード（修正済み）も VfsPath::from になっている
```

**TypeScript (`src-desktop/frontend/src/editor-canvas.ts`):**
- Canvas 2D API でテキスト・カーソル・選択範囲・preedit を描画
- `CHAR_W = 8`, `LINE_H = 20` は Rust の定数と一致させること
- CJK 文字は `CHAR_W * 2` として幅計算

---

### Task 8: preview.ts + chrome.ts + ime-bridge.ts

**参照:** Plan 6 Step 8.1〜8.5

| ファイル | 役割 |
|---------|------|
| `preview.ts` | `innerHTML` で HTML をマウント、KaTeX 再レンダリング |
| `chrome.ts` | タブバー・ステータスバーの DOM 操作 |
| `ime-bridge.ts` | `compositionstart/update/end` → `invoke('push_ime_event', {event})` |

**ime-bridge.ts の要点:**
- `canvas` の click で hidden `<input>` にフォーカス
- `compositionend` で `Commit` event を push してから `dispatch` で Key event を送り再描画を起こす
- Rust からの `ime-cursor-area` カスタムイベントで hidden input の位置を更新

---

### Task 9: src-cli リファクタリング (spec §10)

**参照:** Plan 6 Step 9.1〜9.6

**目的:** `src-cli` の変換パイプラインを `AppCore<NativeFs, NativePluginHost>` に統一する。
`src-core`・`src-plugin` の直接 import を全て削除。HTML_HEAD/HTML_TAIL 定数は維持。

**`src-cli/Cargo.toml` の変更:**
```toml
[dependencies]
src-desktop-core   = { path = "../src-desktop-core" }
src-desktop-native = { path = "../src-desktop-native" }
src-desktop-types  = { path = "../src-desktop-types" }
# src-core と src-plugin の依存を削除
```

**`src-cli/src/main.rs` の新パイプライン（骨子）:**
```rust
use src_desktop_types::VfsPath;
use src_desktop_core::AppCore;
use src_desktop_native::{NativeFs, make_plugin_host, load_app_config};

let config = load_app_config(config_path);
let host   = make_plugin_host(&config.plugins);
let mut core = AppCore::new(NativeFs, host, config);

let vpath  = VfsPath::from(input_path.as_str());   // ← VfsPath::new() は存在しない
let doc_id = core.open_file(&vpath).unwrap_or_else(|e| { eprintln!(...); exit(1) }).0;
let (html_body, _, warnings) = core.render_full(doc_id).unwrap_or_else(|| { ... });

for w in &warnings { eprintln!(...) }
fs::write(output_path, format!("{HTML_HEAD}{html_body}{HTML_TAIL}"))?;
```

**注意:** `PluginWarning` の `line`/`col` フィールドは `u32` 型。
`render_full` は `Option<(String, Vec<u64>, Vec<PluginWarning>)>` を返す。

---

## 各タスクの完了後の確認コマンド

```sh
# Task 6 完了確認
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend
npx tsc --noEmit

# Task 7 完了確認
cargo test -p src-desktop-layout cursor_position
cargo test --workspace 2>&1 | grep "FAILED"   # 0 件

# Task 8 完了確認
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend
npx tsc --noEmit

# Task 9 完了確認
cargo test --workspace 2>&1 | grep "FAILED"   # 0 件
cargo run -p src-cli -- /tmp/test.n.md /tmp/test.html 2>&1
grep "nm-ruby" /tmp/test.html
```

---

## 全タスク完了後のマージ・リリース手順

```sh
# 1. ワークスペース全テスト確認
cd /mnt/d/project/gloss/.worktrees/src-desktop
cargo test --workspace 2>&1 | grep "FAILED"   # 0 件

# 2. main にマージ（ローカル）
git checkout main
git merge feat/src-desktop --no-ff -m "feat: src-desktop Tauri shell + src-cli AppCore pipeline"

# 3. タグを打つ
git tag cli/v0.2.0     # src-cli リファクタ含む
git tag desktop/v0.1.0 # 最初のデスクトップリリース（pre-alpha）

# 4. プッシュ
git push && git push --tags

# 5. GitHub リリース作成
gh release create cli/v0.2.0     --title "CLI v0.2.0 (AppCore pipeline)" --notes "..."
gh release create desktop/v0.1.0 --title "Desktop v0.1.0 (Tauri shell, pre-alpha)" --notes "..."
```

---

## ワークスペース構成（現時点）

```
/mnt/d/project/gloss/
├── src-core/               # no_std パーサ・HTML生成
├── src-cli/                # CLI バイナリ ← Task 9 でリファクタ
├── src-web/                # WASM ビルド
├── src-desktop-types/      # 共有型 (AppEvent, DrawCmd, DocId, …)
├── src-desktop-core/       # AppCore<Fs, Ph> — ドキュメント管理・レンダリング
├── src-desktop-layout/     # Model, update(), view() — Elm アーキテクチャ
├── src-desktop-native/     # NativeFs, NativePluginHost (std 依存)
└── .worktrees/src-desktop/ ← feat/src-desktop ブランチ
    └── src-desktop/        # Tauri クレート
        ├── Cargo.toml      # crate-type: [staticlib, cdylib, rlib]
        ├── build.rs
        ├── tauri.conf.json # frontendDist: ../frontend/dist, devUrl: localhost:1420
        ├── capabilities/default.json
        ├── icons/icon.png  # 32×32 プレースホルダー
        └── src/
            ├── main.rs         # fn main() → src_desktop_lib::run()
            ├── lib.rs          # Tauri Builder, manage(SharedState), invoke_handler
            ├── clipboard.rs    # NativeClipboard(arboard::Clipboard)
            ├── ime.rs          # TauriIme { queue: VecDeque<ImeEvent> }
            ├── state.rs        # AppState, SharedState, create_state()
            └── commands.rs     # dispatch, push_ime_event, dispatch_shell_cmds
```

---

## 参照ドキュメント

| ドキュメント | 内容 |
|------------|------|
| `docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md` | Plan 6 — Task 1〜9 の完全なコード付き実装計画（修正済み） |
| `docs/superpowers/specs/2026-03-24-src-desktop-design.md` | 設計仕様 §8 (Tauri shell), §10 (src-cli refactor) |
| `CLAUDE.md` | バージョニングポリシー、リリースワークフロー |
