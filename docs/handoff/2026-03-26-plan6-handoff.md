# Plan 6 (src-desktop Tauri shell) — 引継ぎドキュメント

作成日: 2026-03-26

---

## 現在の状態サマリ

| 項目 | 状態 |
|------|------|
| Plan 6 プラン文書 | ✅ 作成済み (`docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md`) |
| Task 1–5 (Rust バックエンド) | ✅ 実装・レビュー完了・コミット済み |
| Task 6–8 (TypeScript フロントエンド) | ⏳ 未着手 |
| Task 9 (src-cli リファクタ) | ⏳ 未着手 |
| `cargo check -p src-desktop` | ✅ エラーなし |
| システムライブラリ | ✅ インストール済み (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`) |

---

## 作業リポジトリ

```
メインブランチ: main (origin)
作業ブランチ:   feat/src-desktop
ワークツリー:   /mnt/d/project/gloss/.worktrees/src-desktop
```

作業は全て `feat/src-desktop` ブランチで進めること。
完了後は `main` にローカルマージする。

---

## コミット履歴 (feat/src-desktop)

```
7fd1c11  fix(src-desktop): DocId mismatch, unused import, sentinel, crate-type, icon
038cf35  feat(src-desktop): dispatch Tauri command + event loop
3a67303  feat(src-desktop): AppState and create_state()
848c20d  feat(src-desktop): TauriIme with VecDeque queue
9c2fd40  feat(src-desktop): NativeClipboard; fix Clipboard::get_text to &mut self
f37d210  feat(src-desktop): Tauri project scaffold
```

---

## 残タスク一覧

### Task 6: Frontend scaffold + renderer.ts
**ファイル:** `src-desktop/frontend/` 以下の全ファイル

プラン `2026-03-26-src-desktop-6-tauri.md` の Step 6.1〜6.11 に完全なコードが記載されている。
作業ディレクトリは `src-desktop/frontend/` になる点に注意。

```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend
npm install
npx tsc --noEmit   # エラーなしを確認
```

### Task 7: view.rs カーソル位置修正 + editor-canvas.ts
**ファイル:**
- `src-desktop-layout/src/view.rs` — `CHAR_W`/`LINE_H` 定数追加、`emit_editor_frame` 修正
- `src-desktop/frontend/src/editor-canvas.ts` — Canvas 2D テキスト描画実装

プランの Step 7.1〜7.7 に完全なコードが記載されている。

### Task 8: preview.ts + chrome.ts + ime-bridge.ts
**ファイル:**
- `src-desktop/frontend/src/preview.ts`
- `src-desktop/frontend/src/chrome.ts`
- `src-desktop/frontend/src/ime-bridge.ts`

プランの Step 8.1〜8.5 に完全なコードが記載されている。

### Task 9: src-cli リファクタ
**ファイル:**
- `src-cli/Cargo.toml` — `src-desktop-core`, `src-desktop-native` 依存追加、`src-core`/`src-plugin` 依存削除
- `src-cli/src/main.rs` — `AppCore<NativeFs, NativePluginHost>` を使うパイプラインに書き直し

プランの Step 9.1〜9.7 に完全なコードが記載されている。
HTML_HEAD/HTML_TAIL 定数はそのまま残す。`src_core`/`src_plugin` の直接 import を全て削除する。

---

## 計画ファイルからの重要な逸脱 (コードレビューで修正済み)

計画書のコードと実際の実装が一部異なる。以下の点は**実装済みの正しい形**を使うこと。

### 1. `AppEvent::FileLoaded` に `doc_id: DocId` フィールドが追加された

**背景:** コードレビューで「layout と AppCore が別々に DocId カウンターを持ち、同じファイルに異なる DocId が割り当てられる」致命的バグが発見された。

**修正内容:**

`src-desktop-types/src/events.rs`:
```rust
// 変更前 (計画書のコード)
FileLoaded  { path: VfsPath, content: Vec<u8> },

// 変更後 (実装済みの正しいコード)
FileLoaded  { path: VfsPath, content: Vec<u8>, doc_id: DocId },
```

`src-desktop/src/commands.rs` の `dispatch_shell_cmds` の `ReadFile` アーム:
```rust
AppCmd::ReadFile { path } => {
    match std::fs::read(path.as_str()) {
        Ok(content) => {
            // AppCore に登録して canonical DocId を取得してから emit
            let (doc_id, vm) = s.core.open_bytes(path.clone(), content.clone());
            s.model.workspace.editors.insert(doc_id, vm);
            extra.push(AppEvent::FileLoaded { path, content, doc_id });
        }
        Err(e) => extra.push(AppEvent::FileError { path, error: e.to_string() }),
    }
}
```

`src-desktop-layout/src/update.rs` の `update_file_loaded`:
- `doc_id` を引数で受け取り、`next_doc_id` カウンターを使わない形に変更済み。

### 2. `src-desktop/src/commands.rs` — `s.model.clone()` は使えない

`Model` は `Clone` を実装していない。計画書のコードは `s.model.clone()` を多用しているが、実装では `std::mem::replace` を使う:

```rust
// 計画書のコード (コンパイルエラーになる)
let (m, cmds) = update(s.model.clone(), event);

// 正しいコード (実装済み)
let model = std::mem::replace(&mut s.model, Model::new(0, 0));
let (m, cmds) = update(model, event);
s.model = m;
```

`Model::new(0, 0)` はプレースホルダーで、直後に `s.model = m` で上書きされるので問題ない。

### 3. trait import が必要

`commands.rs` の先頭に以下の import が必要:
```rust
use src_desktop_types::{AppCmd, AppEvent, Clipboard, DrawCmd, ImeEvent, ImeSource};
use src_desktop_layout::{update, view, Model};
```

### 4. `Cargo.toml` の `crate-type` に `rlib` が必要

```toml
# 計画書のコード
crate-type = ["staticlib", "cdylib"]

# 正しいコード (実装済み)
crate-type = ["staticlib", "cdylib", "rlib"]
```

`rlib` がないと `main.rs` から `src_desktop_lib::run()` が呼べない。

### 5. `icons/icon.png` が必要

`tauri::generate_context!()` マクロが `src-desktop/icons/icon.png` を参照する。
プレースホルダーの 32×32 PNG を `src-desktop/icons/icon.png` に作成済み。

---

## 作業を続ける手順

```sh
# ワークツリーに移動
cd /mnt/d/project/gloss/.worktrees/src-desktop

# 現在の状態確認
cargo check -p src-desktop    # エラーなしを確認
git log --oneline -5

# Task 6 開始 (フロントエンドスキャフォルド)
# → docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md の Task 6 を参照
mkdir -p src-desktop/frontend/src
# ... (プランの Step 6.1〜6.11 を実行)
```

---

## 完了後の作業

全 Task 完了後:
1. `feat/src-desktop` を `main` にローカルマージ
2. `cli/v0.2.0` タグ (src-cli リファクタ含む)
3. `desktop/v0.1.0` タグ (最初のデスクトップリリース)

```sh
git checkout main && git merge feat/src-desktop
git tag cli/v0.2.0 && git tag desktop/v0.1.0
git push && git push --tags
gh release create cli/v0.2.0 --title "CLI v0.2.0 (AppCore pipeline)" --notes "..."
gh release create desktop/v0.1.0 --title "Desktop v0.1.0 (Tauri shell)" --notes "..."
```

---

## 関連ドキュメント

| ドキュメント | 内容 |
|------------|------|
| `docs/superpowers/plans/2026-03-26-src-desktop-6-tauri.md` | **Plan 6** — Task 1〜9 の完全なコード付き実装計画 |
| `docs/superpowers/specs/2026-03-24-src-desktop-design.md` | 設計仕様 §8 (Tauri shell), §10 (src-cli refactor) |
| `CLAUDE.md` | バージョニングポリシー、リリースワークフロー |

---

## ワークスペース構成 (現在)

```
/mnt/d/project/gloss/
├── src-core/               # no_std パーサ・HTML生成
├── src-cli/                # CLI バイナリ (Task 9 でリファクタ予定)
├── src-web/                # WASM ビルド
├── src-desktop-types/      # 共有型 (AppEvent, DrawCmd, etc.)
├── src-desktop-core/       # AppCore<Fs, Ph> — ドキュメント管理・レンダリング
├── src-desktop-layout/     # Model, update(), view() — Elm アーキテクチャ
├── src-desktop-native/     # NativeFs, NativePluginHost (std)
└── .worktrees/src-desktop/ # feat/src-desktop ブランチのワークツリー
    └── src-desktop/        # Tauri クレート (Tasks 1-5 完了)
```
