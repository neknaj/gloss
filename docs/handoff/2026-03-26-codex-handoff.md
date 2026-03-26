# Codex への引継ぎ指示 — Gloss src-desktop Tauri Shell (Plan 6 残タスク)

作成日: 2026-03-26

---

## あなたへの依頼

Rust製Markdownツールチェーン **Gloss** のデスクトップアプリ実装を継続してください。
バックエンド（Tauri Rustシェル）は完成済みです。残りはフロントエンド実装（Tasks 6–8）とCLIリファクタ（Task 9）です。

---

## リポジトリの場所と確認コマンド

```sh
# 作業ブランチ（Tauriシェル開発用ワークツリー）
cd /mnt/d/project/gloss/.worktrees/src-desktop

# 状態確認
git log --oneline -5
cargo check -p src-desktop
cargo test --workspace   # → 160 passed, 0 failed を確認
```

**メインリポジトリ:** `/mnt/d/project/gloss/`（ブランチ: `main`）
**作業ブランチ:** `feat/src-desktop`（ワークツリー: `.worktrees/src-desktop/`）

---

## プロジェクト概要

Gloss は独自Markdownダイアレクト（Ruby/Anno/Nest/Math/Lint 拡張）のRustライブラリ＋ツールチェーン。

### ワークスペース構成

```
/mnt/d/project/gloss/
├── src-core/               # no_std パーサ・HTML生成（コアライブラリ）
├── src-cli/                # CLI バイナリ ← Task 9 でリファクタ
├── src-web/                # WASM ビルド
├── src-desktop-types/      # 共有型 (AppEvent, DrawCmd, DocId, AppCmd, …)
├── src-desktop-core/       # AppCore<Fs, Ph> — ドキュメント管理・レンダリング
├── src-desktop-layout/     # Model, update(), view() — Elm アーキテクチャ
├── src-desktop-native/     # NativeFs, NativePluginHost (std 依存)
└── .worktrees/src-desktop/ ← feat/src-desktop ブランチ
    └── src-desktop/        # Tauri クレート（← ここを主に触る）
        ├── Cargo.toml
        ├── tauri.conf.json
        ├── frontend/       # ← Tasks 6–8 で作成するフロントエンド（まだ存在しない）
        └── src/
            ├── main.rs, lib.rs
            ├── clipboard.rs, ime.rs, state.rs, commands.rs
```

### アーキテクチャ概念

- **Elm アーキテクチャ:** `update(Model, AppEvent) -> (Model, Vec<AppCmd>)` + `view(&Model) -> Vec<DrawCmd>`
- **AppCore:** `AppCore<NativeFs, GlossPluginHost>` がドキュメント管理・レンダリングを担当
- **Tauri Shell:** `dispatch` コマンドで JS → Rust → JS の往復
  1. JS がキーボード/マウス入力を `invoke('dispatch', {event})` で送信
  2. Rust が `update()` → `execute_cmds()` → `view()` → `Vec<DrawCmd>` を返す
  3. JS が DrawCmd を解釈してCanvas/DOMを更新

---

## 参照すべきドキュメント（優先順）

| ドキュメント | 内容 | 重要度 |
|------------|------|--------|
| `docs/handoff/2026-03-26-plan6-handoff.md` | **最重要。** 実装済み差異・残タスク仕様・完了確認コマンド一覧 | ★★★ |
| `docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md` | Plan 6 完全実装計画（コード付き）。Tasks 6–9 の詳細手順 | ★★★ |
| `docs/superpowers/specs/2026-03-24-src-desktop-design.md` | 設計仕様書 §8(Tauriシェル), §10(src-cliリファクタ) | ★★ |
| `CLAUDE.md` | バージョニングポリシー、リリースワークフロー | ★ |

---

## 現在の実装状態（完了済み Tasks 1–5）

| Task | 内容 | 状態 |
|------|------|------|
| 1 | Tauri プロジェクトスキャフォルド | ✅ 完了 |
| 2 | NativeClipboard | ✅ 完了 |
| 3 | TauriIme | ✅ 完了 |
| 4 | AppState + state.rs | ✅ 完了 |
| 5 | dispatch コマンド + イベントループ | ✅ 完了 |

**全ワークスペーステスト: 160 passed, 0 failed**（確認済み 2026-03-26）

---

## 計画書との重要な差異（実装済み — これを前提として作業すること）

### 差異 1: `AppEvent::FileLoaded` に `doc_id: DocId` フィールドがある

```rust
// src-desktop-types/src/events.rs — 実際の定義
FileLoaded { path: VfsPath, content: Vec<u8>, doc_id: DocId },
```

計画書のコードにない場合でも `doc_id` フィールドは必須。

### 差異 2: `Model` は `Clone` 未実装 → `std::mem::replace` を使う

```rust
let model = std::mem::replace(&mut s.model, Model::new(0, 0));
let (m, cmds) = update(model, event);
s.model = m;
```

### 差異 3: `Cargo.toml` の crate-type に `rlib` が必要

```toml
crate-type = ["staticlib", "cdylib", "rlib"]
```

### 差異 4: `VfsPath::new()` は存在しない → `VfsPath::from()` を使う

```rust
VfsPath::from("/path/to/file")  // ✅
VfsPath::new("/path/to/file")   // ❌ コンパイルエラー
```

---

## 残タスク一覧（Tasks 6–9）

### Task 6: Frontend scaffold + renderer.ts

**作成場所:** `src-desktop/frontend/` （ゼロから作成）

作成するファイル:
- `package.json` — `@tauri-apps/api@^2`, `typescript@^5`, `vite@^5`
- `tsconfig.json` — `strict: true`, `moduleResolution: bundler`
- `vite.config.ts` — `server.port: 1420`
- `index.html` — `#tab-bar`, `#main-area > #editor-canvas + #preview-pane`, `#status-bar`
- `src/types.ts` — DrawCmd / AppEvent の TypeScript 型定義（Rust の enum と1対1対応）
- `src/renderer.ts` — DrawCmd を各モジュールに振り分ける switch 文
- `src/main.ts` — キーボード入力 → `invoke('dispatch', {event})` → `applyDrawCmds()`
- `src/style.css` — ダークテーマ
- `src/editor-canvas.ts` (stub)、`src/preview.ts` (stub)、`src/chrome.ts` (stub)、`src/ime-bridge.ts` (stub)

**完了確認:**
```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend
npm install && npx tsc --noEmit
```

### Task 7: view.rs カーソル位置修正 + editor-canvas.ts

**Rust変更 (`src-desktop-layout/src/view.rs`):**

```rust
pub const CHAR_W: f32 = 8.0;
pub const LINE_H: f32 = 20.0;

// emit_editor_frame: cursor の pixel 座標を vm.cursor から計算する
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

テスト追加時は `VfsPath::from()` を使うこと（`VfsPath::new()` は存在しない）。

**TypeScript (`editor-canvas.ts`):**
- Canvas 2D API でテキスト・カーソル・選択範囲・preedit を描画
- `CHAR_W = 8`, `LINE_H = 20`（Rust 定数と一致させること）
- CJK文字は `CHAR_W * 2` として幅計算

**完了確認:**
```sh
cargo test -p src-desktop-layout cursor_position
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend && npx tsc --noEmit
```

### Task 8: preview.ts + chrome.ts + ime-bridge.ts

| ファイル | 役割 |
|---------|------|
| `preview.ts` | `innerHTML` で HTML をマウント、KaTeX 再レンダリング |
| `chrome.ts` | タブバー・ステータスバーの DOM 操作 |
| `ime-bridge.ts` | `compositionstart/update/end` → `invoke('push_ime_event', {event})` |

**ime-bridge.ts の要点:**
- canvas の click で hidden `<input>` にフォーカス
- `compositionend` で `Commit` event を push してから `dispatch` で Key event を送り再描画
- Rust からの `ime-cursor-area` イベントで hidden input の位置を更新

**完了確認:**
```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend && npx tsc --noEmit
```

### Task 9: src-cli リファクタリング

**目的:** `src-cli` の変換パイプラインを `AppCore<NativeFs, NativePluginHost>` に統一。
`src-core`・`src-plugin` の直接 import を全て削除。

**`src-cli/Cargo.toml` 変更:**
```toml
[dependencies]
src-desktop-core   = { path = "../src-desktop-core" }
src-desktop-native = { path = "../src-desktop-native" }
src-desktop-types  = { path = "../src-desktop-types" }
# src-core と src-plugin の依存を削除
```

**`src-cli/src/main.rs` 新パイプライン骨子:**
```rust
use src_desktop_types::VfsPath;
use src_desktop_core::AppCore;
use src_desktop_native::{NativeFs, make_plugin_host, load_app_config};

let config  = load_app_config(config_path);
let host    = make_plugin_host(&config.plugins);
let mut core = AppCore::new(NativeFs, host, config);

let vpath  = VfsPath::from(input_path.as_str());   // ← VfsPath::from() を使うこと
let (doc_id, _vm) = core.open_file(&vpath).unwrap_or_else(|e| { eprintln!("{e}"); exit(1) });
let (html_body, _, warnings) = core.render_full(doc_id).unwrap_or_else(|| { eprintln!("render failed"); exit(1) });

for w in &warnings { eprintln!("warning [{}:{}] {}", w.source, w.line, w.message); }
fs::write(output_path, format!("{HTML_HEAD}{html_body}{HTML_TAIL}"))?;
```

注意: `PluginWarning` の `line`/`col` は `u32` 型。`render_full` は `Option<(String, Vec<u64>, Vec<PluginWarning>)>` を返す。

**完了確認:**
```sh
cargo test --workspace 2>&1 | grep "FAILED"   # 0件
cargo run -p src-cli -- /tmp/test.n.md /tmp/test.html 2>&1
grep "nm-ruby" /tmp/test.html
```

---

## 全タスク完了後のマージ・リリース手順

```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop

# 1. 全テスト確認
cargo test --workspace 2>&1 | grep "FAILED"   # 0件

# 2. main にマージ
git checkout main
git merge feat/src-desktop --no-ff -m "feat: src-desktop Tauri shell + src-cli AppCore pipeline"

# 3. タグを打つ
git tag cli/v0.2.0
git tag desktop/v0.1.0

# 4. プッシュ
git push && git push --tags

# 5. GitHub リリース作成
gh release create cli/v0.2.0     --title "CLI v0.2.0 (AppCore pipeline)"      --notes "..."
gh release create desktop/v0.1.0 --title "Desktop v0.1.0 (Tauri shell, pre-alpha)" --notes "..."
```

---

## 開発時の注意事項

1. **作業ディレクトリ:** 常に `/mnt/d/project/gloss/.worktrees/src-desktop` で作業すること（`main` ブランチのリポジトリ `/mnt/d/project/gloss/` を直接触らない）
2. **コミット先:** `feat/src-desktop` ブランチに対してコミットする
3. **テスト:** 各タスク完了後に `cargo test --workspace` を実行し 0 FAILED を確認
4. **TypeScript:** `npx tsc --noEmit` で型エラーがないことを確認
5. **計画書のコードに VfsPath::new() が出てきたら全て VfsPath::from() に読み替える**
6. **フロントエンドパス:** `src-desktop/frontend/` は `.worktrees/src-desktop/src-desktop/frontend/` のこと
