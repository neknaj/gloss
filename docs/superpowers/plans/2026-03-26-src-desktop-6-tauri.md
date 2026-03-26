# src-desktop (Plan 6 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `src-desktop` Tauri v2 crate — a native desktop GUI editor for `.n.md` files with a canvas-based text editor, HTML preview, IME support, and plugin hooks; also refactor `src-cli` to use `AppCore`.

**Architecture:** Tauri v2 shell holding `AppState { model, core, clipboard, ime }` behind `Arc<Mutex<_>>`. A single `dispatch(event: AppEvent)` Tauri command runs the Elm-style `update()` + `execute_cmds()` loop and returns `Vec<DrawCmd>` serialized as JSON to the JS frontend. The TypeScript frontend (Vite) renders `DrawCmd`s onto a `<canvas>` (editor pane) and via `innerHTML` (preview pane).

**Tech Stack:** Rust + Tauri v2, TypeScript + Vite, Canvas 2D API, KaTeX (preview math), arboard (native clipboard).

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md` §8, §10

---

## ⚠️ 実装済み逸脱 — Tasks 1–5 は完了済み。以下の変更が適用されている

Tasks 1–5 のコードはこのプランどおりには **実装されていない**。
コードレビューで発見された修正が適用された。次のセッションが Tasks 6–9 を実装する際に
コンフリクトを起こさないよう、変更内容を以下に記す。

| # | 変更箇所 | 内容 |
|---|---------|------|
| 1 | `src-desktop-types/src/events.rs` | `AppEvent::FileLoaded` に `doc_id: DocId` フィールドを追加 |
| 2 | `src-desktop/src/commands.rs` | `ReadFile` ハンドラが `core.open_bytes()` を呼んでから `FileLoaded` を emit |
| 3 | `src-desktop/src/commands.rs` | `s.model.clone()` → `std::mem::replace` (Model は Clone 未実装) |
| 4 | `src-desktop/src/commands.rs` | `ImeSource`, `Clipboard`, `Model` を use に追加 |
| 5 | `src-desktop/src/commands.rs` | `ScheduleRender` — `DocId(0)` センチネルを廃止、active doc がなければスキップ |
| 6 | `src-desktop/Cargo.toml` | `crate-type` に `rlib` 追加 (binary が lib を使うため必須) |
| 7 | `src-desktop/icons/icon.png` | Tauri ビルドに必要な 32×32 プレースホルダー PNG を追加 |
| 8 | `src-desktop-layout/src/update.rs` | `update_file_loaded` が `doc_id` を引数で受け取り `next_doc_id` カウンター非使用に |

このプランのコードは上記修正を反映済みである。

---

## Scope Note

This plan covers two independent subsystems (Rust backend and TypeScript frontend). Tasks 1–5 are pure Rust; Tasks 6–8 are pure TypeScript. Task 9 refactors `src-cli`. Each group is independently buildable and testable.

---

## File Map

| File | Change | Responsibility |
|------|--------|----------------|
| `Cargo.toml` (root) | Modify | Add `src-desktop` to workspace members |
| `src-desktop/Cargo.toml` | Create | Tauri v2 + arboard + tokio deps |
| `src-desktop/build.rs` | Create | `tauri_build::build()` |
| `src-desktop/tauri.conf.json` | Create | Tauri app configuration |
| `src-desktop/capabilities/default.json` | Create | Permission declaration |
| `src-desktop/src/main.rs` | Create | Entry point: `fn main()` |
| `src-desktop/src/lib.rs` | Create | Tauri Builder + plugin registration |
| `src-desktop/src/clipboard.rs` | Create | `NativeClipboard(arboard::Clipboard)` newtype + `impl Clipboard` |
| `src-desktop/src/ime.rs` | Create | `TauriIme { queue: VecDeque<ImeEvent> }` + `impl ImeSource` |
| `src-desktop/src/state.rs` | Create | `AppState`, `SharedState = Arc<Mutex<AppState>>`, `create_state()` |
| `src-desktop/src/commands.rs` | Create | `dispatch` + `push_ime_event` Tauri commands + `dispatch_shell_cmds` |
| `src-desktop/frontend/package.json` | Create | npm: Vite, TypeScript |
| `src-desktop/frontend/tsconfig.json` | Create | TypeScript config |
| `src-desktop/frontend/vite.config.ts` | Create | Vite static build config |
| `src-desktop/frontend/index.html` | Create | App shell HTML |
| `src-desktop/frontend/src/main.ts` | Create | Entry: initial dispatch, draw event listener |
| `src-desktop/frontend/src/types.ts` | Create | TypeScript types mirroring DrawCmd / AppEvent |
| `src-desktop/frontend/src/renderer.ts` | Create | Dispatch `DrawCmd[]` to sub-modules |
| `src-desktop/frontend/src/editor-canvas.ts` | Create | Canvas 2D text editor rendering |
| `src-desktop/frontend/src/preview.ts` | Create | HTML preview mount / patch |
| `src-desktop/frontend/src/chrome.ts` | Create | Tab bar, status bar, file tree DOM ops |
| `src-desktop/frontend/src/ime-bridge.ts` | Create | `compositionstart/update/end` → `push_ime_event` IPC |
| `src-desktop/frontend/src/style.css` | Create | App styling (dark theme, nm-* classes) |
| `src-desktop-types/src/traits.rs` | Modify | Change `Clipboard::get_text` to `&mut self` (arboard compat) |
| `src-desktop-layout/src/view.rs` | Modify | Fix `emit_editor_frame` to compute real cursor pixel position |
| `src-cli/Cargo.toml` | Modify | Add `src-desktop-core`, `src-desktop-native` deps |
| `src-cli/src/main.rs` | Modify | Rewrite pipeline to use `AppCore<NativeFs, NativePluginHost>` |

---

## Task 1: Tauri project scaffold

**Files:**
- Create: `src-desktop/Cargo.toml`
- Create: `src-desktop/build.rs`
- Create: `src-desktop/tauri.conf.json`
- Create: `src-desktop/capabilities/default.json`
- Create: `src-desktop/src/main.rs`
- Create: `src-desktop/src/lib.rs` (stub)
- Modify: `Cargo.toml` (root)

- [ ] **Step 1.1: Add src-desktop to workspace**

In root `Cargo.toml`, add `"src-desktop"` to the `members` array.

- [ ] **Step 1.2: Create `src-desktop/Cargo.toml`**

```toml
[package]
name = "src-desktop"
version = "0.1.0"
edition = "2021"

[lib]
name = "src_desktop_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[[bin]]
name = "src-desktop"
path = "src/main.rs"

[dependencies]
tauri         = { version = "2", features = [] }
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
arboard       = "3"
tokio         = { version = "1", features = ["rt-multi-thread"] }
src-desktop-types  = { path = "../src-desktop-types" }
src-desktop-core   = { path = "../src-desktop-core" }
src-desktop-layout = { path = "../src-desktop-layout" }
src-desktop-native = { path = "../src-desktop-native" }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 1.3: Create `src-desktop/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 1.4: Create `src-desktop/tauri.conf.json`**

```json
{
  "productName": "Gloss",
  "version": "0.1.0",
  "identifier": "com.gloss.desktop",
  "app": {
    "windows": [
      { "title": "Gloss", "width": 1200, "height": 800 }
    ],
    "security": { "csp": null }
  },
  "build": {
    "frontendDist": "../frontend/dist",
    "devUrl": "http://localhost:1420"
  }
}
```

- [ ] **Step 1.5: Create `src-desktop/capabilities/default.json`**

```json
{
  "identifier": "default",
  "description": "Default capability",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 1.6: Create `src-desktop/src/main.rs`**

```rust
fn main() {
    src_desktop_lib::run();
}
```

- [ ] **Step 1.7: Create stub `src-desktop/src/lib.rs`**

```rust
mod clipboard;
mod ime;
mod state;
mod commands;

pub fn run() {
    tauri::Builder::default()
        .manage(state::create_state())
        .invoke_handler(tauri::generate_handler![
            commands::dispatch,
            commands::push_ime_event,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Also create empty stub files so the crate compiles:
- `src-desktop/src/clipboard.rs` — `pub struct NativeClipboard;`
- `src-desktop/src/ime.rs` — `pub struct TauriIme;`
- `src-desktop/src/state.rs` — see step below
- `src-desktop/src/commands.rs` — see step below

Stub `state.rs`:
```rust
use std::sync::{Arc, Mutex};
use src_desktop_types::AppConfig;
use src_desktop_core::AppCore;
use src_desktop_layout::Model;
use src_desktop_native::{NativeFs, NativePluginHost, make_plugin_host, load_app_config};
use crate::clipboard::NativeClipboard;
use crate::ime::TauriIme;

pub struct AppState {
    pub model:     Model,
    pub core:      AppCore<NativeFs, NativePluginHost>,
    pub clipboard: NativeClipboard,
    pub ime:       TauriIme,
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn create_state() -> SharedState {
    let config = load_app_config("gloss.toml");
    let host = make_plugin_host(&config.plugins);
    let core = AppCore::new(NativeFs, host, config.clone());
    let mut model = Model::new(1200, 800);
    model.config = config;
    Arc::new(Mutex::new(AppState {
        model,
        core,
        clipboard: NativeClipboard,
        ime: TauriIme,
    }))
}
```

Stub `commands.rs`:
```rust
use crate::state::SharedState;
use src_desktop_types::{AppEvent, DrawCmd};

#[tauri::command]
pub async fn dispatch(
    _state: tauri::State<'_, SharedState>,
    _event: AppEvent,
) -> Result<Vec<DrawCmd>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn push_ime_event(
    _state: tauri::State<'_, SharedState>,
    _event: src_desktop_types::ImeEvent,
) {}
```

- [ ] **Step 1.8: Verify `cargo build -p src-desktop` succeeds**

```sh
cd /mnt/d/project/gloss && cargo build -p src-desktop 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 1.9: Commit**

```sh
git add src-desktop/ Cargo.toml Cargo.lock
git commit -m "feat(src-desktop): Tauri project scaffold"
```

---

## Task 2: NativeClipboard

**Files:**
- Modify: `src-desktop-types/src/traits.rs` (change `get_text` to `&mut self`)
- Create/Replace: `src-desktop/src/clipboard.rs`

**Why:** `arboard::Clipboard::get_text()` requires `&mut self`. The existing trait has `get_text(&self)`. Fix the trait before implementing.

- [ ] **Step 2.1: Write failing test for NativeClipboard**

Add to `src-desktop/src/clipboard.rs`:

```rust
#[cfg(test)]
mod tests {
    use src_desktop_types::Clipboard;

    #[test]
    fn native_clipboard_implements_trait() {
        // Just verify the type compiles and the trait methods exist
        fn _assert_impl<T: Clipboard>() {}
        _assert_impl::<super::NativeClipboard>();
    }
}
```

Run: `cargo test -p src-desktop 2>&1 | tail -5`

Expected: **compile error** — `NativeClipboard` is currently a unit struct that doesn't implement `Clipboard`.

- [ ] **Step 2.2: Change `Clipboard::get_text` signature in `src-desktop-types/src/traits.rs`**

Line ~27, change:
```rust
fn get_text(&self) -> Option<String>;
```
to:
```rust
fn get_text(&mut self) -> Option<String>;
```

- [ ] **Step 2.3: Implement `src-desktop/src/clipboard.rs`**

After changing the trait to `&mut self`, `NativeClipboard` can hold the clipboard directly — no `RefCell` needed (AppState is already behind `Mutex`):

```rust
use src_desktop_types::Clipboard;

pub struct NativeClipboard(arboard::Clipboard);

impl NativeClipboard {
    pub fn new() -> Self {
        NativeClipboard(arboard::Clipboard::new().expect("failed to open clipboard"))
    }
}

impl Clipboard for NativeClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.get_text().ok()
    }
    fn set_text(&mut self, text: &str) {
        let _ = self.0.set_text(text);
    }
}

#[cfg(test)]
mod tests {
    use src_desktop_types::Clipboard;

    #[test]
    fn native_clipboard_implements_trait() {
        fn _assert_impl<T: Clipboard>() {}
        _assert_impl::<super::NativeClipboard>();
    }
}
```

Update stub in `state.rs` to use `NativeClipboard::new()`.

- [ ] **Step 2.4: Run tests**

```sh
cargo test -p src-desktop native_clipboard 2>&1 | tail -5
```

Expected: PASS.

- [ ] **Step 2.5: Verify full workspace still compiles**

```sh
cargo build 2>&1 | grep "^error" | head -5
```

Expected: no errors.

- [ ] **Step 2.6: Commit**

```sh
git add src-desktop/src/clipboard.rs src-desktop-types/src/traits.rs
git commit -m "feat(src-desktop): NativeClipboard; fix Clipboard::get_text to &mut self"
```

---

## Task 3: TauriIme

**Files:**
- Create/Replace: `src-desktop/src/ime.rs`

- [ ] **Step 3.1: Write failing test**

```rust
// In src-desktop/src/ime.rs (replace stub):
#[cfg(test)]
mod tests {
    use src_desktop_types::{ImeSource, ImeEvent};
    #[test]
    fn tauri_ime_push_then_poll() {
        let mut ime = super::TauriIme::new();
        assert!(ime.poll_event().is_none());
        ime.push(ImeEvent::Start);
        assert_eq!(ime.poll_event(), Some(ImeEvent::Start));
        assert!(ime.poll_event().is_none());
    }
}
```

Run: `cargo test -p src-desktop tauri_ime 2>&1 | tail -5`

Expected: **compile error** — `TauriIme` is a unit struct with no methods.

- [ ] **Step 3.2: Implement `src-desktop/src/ime.rs`**

```rust
use std::collections::VecDeque;
use src_desktop_types::{ImeEvent, ImeSource};

pub struct TauriIme {
    queue: VecDeque<ImeEvent>,
}

impl TauriIme {
    pub fn new() -> Self {
        TauriIme { queue: VecDeque::new() }
    }
    pub fn push(&mut self, event: ImeEvent) {
        self.queue.push_back(event);
    }
}

impl ImeSource for TauriIme {
    fn poll_event(&mut self) -> Option<ImeEvent> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use src_desktop_types::{ImeSource, ImeEvent};
    #[test]
    fn tauri_ime_push_then_poll() {
        let mut ime = super::TauriIme::new();
        assert!(ime.poll_event().is_none());
        ime.push(ImeEvent::Start);
        assert_eq!(ime.poll_event(), Some(ImeEvent::Start));
        assert!(ime.poll_event().is_none());
    }
}
```

Update `state.rs` stub to use `TauriIme::new()`.

- [ ] **Step 3.3: Run tests**

```sh
cargo test -p src-desktop tauri_ime 2>&1 | tail -5
```

Expected: PASS.

- [ ] **Step 3.4: Commit**

```sh
git add src-desktop/src/ime.rs src-desktop/src/state.rs
git commit -m "feat(src-desktop): TauriIme with VecDeque queue"
```

---

## Task 4: AppState and state.rs

**Files:**
- Replace: `src-desktop/src/state.rs`

The stub from Task 1 is already close to final. This task fleshes it out and runs a compile check.

- [ ] **Step 4.1: Replace `src-desktop/src/state.rs` with final version**

```rust
use std::sync::{Arc, Mutex};
use src_desktop_layout::Model;
use src_desktop_core::AppCore;
use src_desktop_native::{NativeFs, NativePluginHost, make_plugin_host, load_app_config};
use crate::clipboard::NativeClipboard;
use crate::ime::TauriIme;

pub struct AppState {
    pub model:     Model,
    pub core:      AppCore<NativeFs, NativePluginHost>,
    pub clipboard: NativeClipboard,
    pub ime:       TauriIme,
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn create_state() -> SharedState {
    let config = load_app_config("gloss.toml");
    let host = make_plugin_host(&config.plugins);
    let core = AppCore::new(NativeFs, host, config.clone());
    let mut model = Model::new(1200, 800);
    model.config = config;
    Arc::new(Mutex::new(AppState {
        model,
        core,
        clipboard: NativeClipboard::new(),
        ime: TauriIme::new(),
    }))
}
```

- [ ] **Step 4.2: Verify `cargo build -p src-desktop`**

```sh
cargo build -p src-desktop 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 4.3: Commit**

```sh
git add src-desktop/src/state.rs
git commit -m "feat(src-desktop): AppState and create_state()"
```

---

## Task 5: dispatch Tauri command

**Files:**
- Replace: `src-desktop/src/commands.rs`

This is the main event loop (spec §8.5). Runs inside `tokio::task::spawn_blocking` so blocking I/O is safe.

**Intentional deviation from spec §8.5:** The spec emits DrawCmds via `window.emit("draw", ...)`. This plan returns them as the command's return value instead (`invoke('dispatch', event)` → `DrawCmd[]`). This is simpler (no separate event listener) and equivalent in behavior.

- [ ] **Step 5.1: Replace `src-desktop/src/commands.rs`**

```rust
use src_desktop_types::{AppCmd, AppEvent, Clipboard, DrawCmd, ImeEvent, ImeSource};
use src_desktop_layout::{update, view, Model};
use crate::state::{AppState, SharedState};

// ── Main dispatch command ─────────────────────────────────────────────────────
//
// IMPORTANT: Model は Clone を実装していない。update() に渡すには
// std::mem::replace で一時的に取り出し、戻り値で上書きする。

#[tauri::command]
pub async fn dispatch(
    state: tauri::State<'_, SharedState>,
    window: tauri::WebviewWindow,
    event: AppEvent,
) -> Result<Vec<DrawCmd>, String> {
    let state_arc: SharedState = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut s = state_arc.lock().unwrap();

        // 1. Drain IME queue first
        while let Some(ime_ev) = s.ime.poll_event() {
            apply_editor_mutation(&mut s, &AppEvent::Ime(ime_ev.clone()));
            let model = std::mem::replace(&mut s.model, Model::new(0, 0));
            let (m, cmds) = update(model, AppEvent::Ime(ime_ev));
            s.model = m;
            let (events, shell_cmds) = s.core.execute_cmds(cmds);
            let extra = dispatch_shell_cmds(&mut s, shell_cmds, &window);
            for ev in events.into_iter().chain(extra) {
                let model = std::mem::replace(&mut s.model, Model::new(0, 0));
                let (m, _) = update(model, ev);
                s.model = m;
            }
        }

        // 2. Main event — up to 8 re-dispatch rounds (bound prevents infinite loops)
        let mut pending = vec![event];
        for _ in 0..8 {
            if pending.is_empty() { break; }
            let mut next = Vec::new();
            for ev in pending.drain(..) {
                apply_editor_mutation(&mut s, &ev);
                let model = std::mem::replace(&mut s.model, Model::new(0, 0));
                let (m, cmds) = update(model, ev);
                s.model = m;
                let (events, shell_cmds) = s.core.execute_cmds(cmds);
                let extra = dispatch_shell_cmds(&mut s, shell_cmds, &window);
                next.extend(events);
                next.extend(extra);
            }
            pending = next;
        }

        // 3. Return draw commands
        Ok(view(&s.model))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── IME event push ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn push_ime_event(state: tauri::State<'_, SharedState>, event: ImeEvent) {
    state.inner().lock().unwrap().ime.push(event);
}

// ── Editor mutation helper ────────────────────────────────────────────────────

fn apply_editor_mutation(s: &mut AppState, ev: &AppEvent) {
    let doc_id = match s.model.active_doc_id() {
        Some(id) => id,
        None => return,
    };
    let vm = match ev {
        AppEvent::Key(k)   => s.core.apply_key(doc_id, k),
        AppEvent::Ime(ime) => s.core.apply_ime(doc_id, ime.clone()),
        _ => None,
    };
    if let Some(vm) = vm {
        s.model.workspace.editors.insert(doc_id, vm);
    }
}

// ── Shell command handler ─────────────────────────────────────────────────────

fn dispatch_shell_cmds(
    s: &mut AppState,
    cmds: Vec<AppCmd>,
    window: &tauri::WebviewWindow,
) -> Vec<AppEvent> {
    use tauri::Emitter;

    let mut extra = Vec::new();
    for cmd in cmds {
        match cmd {
            AppCmd::CopyToClipboard { text } => {
                s.clipboard.set_text(&text);
            }
            AppCmd::PasteRequest => {
                if let Some(text) = s.clipboard.get_text() {
                    extra.push(AppEvent::ClipboardText(text));
                }
            }
            AppCmd::SetImeCursorArea { rect } => {
                let _ = window.emit("ime-cursor-area", rect);
            }
            AppCmd::OpenUrl { url } => {
                let _ = open::that(url);
            }
            AppCmd::Quit => {
                let _ = window.close();
            }
            AppCmd::ReadFile { path } => {
                match std::fs::read(path.as_str()) {
                    Ok(content) => {
                        // Register with AppCore first to get the canonical DocId.
                        let (doc_id, vm) = s.core.open_bytes(path.clone(), content.clone());
                        s.model.workspace.editors.insert(doc_id, vm);
                        extra.push(AppEvent::FileLoaded { path, content, doc_id });
                    }
                    Err(e) => extra.push(AppEvent::FileError {
                        path,
                        error: e.to_string(),
                    }),
                }
            }
            AppCmd::LoadConfig { path } => {
                let config = src_desktop_native::load_app_config(path.as_str());
                extra.push(AppEvent::ConfigLoaded(config));
            }
            AppCmd::ScheduleRender { pane_id, delay_ms: _ } => {
                // TODO: honour delay_ms for debouncing; for now run immediately
                if let Some(doc_id) = s.model.active_doc_id() {
                    let (evs, _) = s.core.execute_cmds(vec![AppCmd::RunRender { pane_id, doc_id }]);
                    extra.extend(evs);
                }
            }
            AppCmd::ShowOpenFileDialog | AppCmd::ShowSaveFileDialog { .. } => {
                // TODO: implement native file dialog via tauri-plugin-dialog
            }
            // AppCore-handled commands should not appear here, but ignore safely
            _ => {}
        }
    }
    extra
}
```

Add `open = "5"` to `src-desktop/Cargo.toml` dependencies.

- [ ] **Step 5.2: Verify `cargo build -p src-desktop`**

```sh
cargo build -p src-desktop 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 5.3: Commit**

```sh
git add src-desktop/src/commands.rs src-desktop/Cargo.toml
git commit -m "feat(src-desktop): dispatch Tauri command + event loop"
```

---

## Task 6: Frontend scaffold + renderer.ts

**Files:**
- Create: `src-desktop/frontend/package.json`
- Create: `src-desktop/frontend/tsconfig.json`
- Create: `src-desktop/frontend/vite.config.ts`
- Create: `src-desktop/frontend/index.html`
- Create: `src-desktop/frontend/src/main.ts`
- Create: `src-desktop/frontend/src/types.ts`
- Create: `src-desktop/frontend/src/renderer.ts`
- Create: `src-desktop/frontend/src/style.css`

- [ ] **Step 6.1: Create `src-desktop/frontend/package.json`**

```json
{
  "name": "gloss-desktop-frontend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@tauri-apps/api": "^2",
    "typescript": "^5",
    "vite": "^5"
  }
}
```

- [ ] **Step 6.2: Create `src-desktop/frontend/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "outDir": "dist"
  },
  "include": ["src"]
}
```

- [ ] **Step 6.3: Create `src-desktop/frontend/vite.config.ts`**

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

- [ ] **Step 6.4: Create `src-desktop/frontend/index.html`**

```html
<!doctype html>
<html lang="ja">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Gloss</title>
  <link rel="stylesheet" href="/src/style.css" />
</head>
<body>
  <div id="app">
    <div id="tab-bar"></div>
    <div id="main-area">
      <canvas id="editor-canvas"></canvas>
      <div id="preview-pane"></div>
    </div>
    <div id="status-bar">
      <span id="status-left"></span>
      <span id="status-right"></span>
    </div>
  </div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

- [ ] **Step 6.5: Create `src-desktop/frontend/src/types.ts`**

This mirrors the Rust types. Copy exactly — these must stay in sync with `src-desktop-types`.

```typescript
export type PaneId = { id: number };
export type DocId  = { id: number };
export type Rect   = { x: number; y: number; width: number; height: number };

export type KeyCode =
  | { Char: string }
  | 'Enter' | 'Backspace' | 'Delete' | 'Escape' | 'Tab'
  | 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight'
  | 'Home' | 'End' | 'PageUp' | 'PageDown'
  | { F: number };

export type Modifiers = { ctrl: boolean; shift: boolean; alt: boolean; meta: boolean };

export type KeyEvent = { key: KeyCode; mods: Modifiers; text: string | null };

export type ImeEvent =
  | 'Start'
  | 'Cancel'
  | { Update: { preedit: string; cursor: [number, number] | null } }
  | { Commit: { text: string } };

export type AppEvent =
  | { Key: KeyEvent }
  | { Ime: ImeEvent }
  | { Resize: { width: number; height: number } }
  | { FileLoaded: { path: string; content: number[] } }
  | 'Quit';

export type TextSpan = { text: string; color: number; bold: boolean; italic: boolean };
export type EditorLine = { line_no: number; spans: TextSpan[] };
export type CursorDraw = { x: number; y: number; height: number };
export type SelectionDraw = { rects: Rect[] };
export type PreeditDraw = { text: string; underline_range: [number, number] | null };
export type ScrollOffset = { x: number; y: number };
export type TabInfo = { doc_id: DocId; title: string; dirty: boolean };
export type PaneKind = 'Editor' | 'Preview' | 'FileTree' | 'PluginManager';
export type PanelLayout = { pane_id: PaneId; bounds: Rect; kind: PaneKind; visible: boolean };
export type HtmlPatch = { block_id: number; html: string };
export type WarningInfo = { code: string; message: string; line: number | null };

export type DrawCmd =
  | { SetLayout: { panels: PanelLayout[]; dividers: unknown[] } }
  | { SetTabBar: { pane_id: PaneId; tabs: TabInfo[]; active_tab: number } }
  | { EditorFrame: {
      pane_id: PaneId; bounds: Rect; lines: EditorLine[];
      cursor: CursorDraw; selection: SelectionDraw | null;
      preedit: PreeditDraw | null; scroll: ScrollOffset;
    }}
  | { PreviewMount: { pane_id: PaneId; html: string } }
  | { PreviewPatch: { pane_id: PaneId; patches: HtmlPatch[] } }
  | { PreviewScroll: { pane_id: PaneId; offset_y: number } }
  | { SetStatusBar: { left: string; right: string; warning_count: number } }
  | { SetWarnings: { warnings: WarningInfo[] } }
  | { SetImeCursorArea: { rect: Rect } }
  | { SetFileTree: unknown }
  | { SetPluginList: unknown }
  | { ShowDialog: unknown }
  | { ShowTooltip: unknown }
  | 'HideTooltip';
```

- [ ] **Step 6.6: Create `src-desktop/frontend/src/renderer.ts`**

```typescript
import type { DrawCmd } from './types';
import { renderEditorFrame } from './editor-canvas';
import { renderPreviewMount, renderPreviewPatch, renderPreviewScroll } from './preview';
import { renderTabBar, renderStatusBar, renderFileTree } from './chrome';

export function applyDrawCmds(cmds: DrawCmd[]): void {
  for (const cmd of cmds) {
    if (typeof cmd === 'string') continue;
    if ('SetLayout' in cmd) {
      // Layout changes: update CSS layout based on panel bounds
      applyLayout(cmd.SetLayout.panels);
    } else if ('SetTabBar' in cmd) {
      renderTabBar(cmd.SetTabBar.pane_id, cmd.SetTabBar.tabs, cmd.SetTabBar.active_tab);
    } else if ('EditorFrame' in cmd) {
      renderEditorFrame(cmd.EditorFrame);
    } else if ('PreviewMount' in cmd) {
      renderPreviewMount(cmd.PreviewMount.pane_id, cmd.PreviewMount.html);
    } else if ('PreviewPatch' in cmd) {
      renderPreviewPatch(cmd.PreviewPatch.pane_id, cmd.PreviewPatch.patches);
    } else if ('PreviewScroll' in cmd) {
      renderPreviewScroll(cmd.PreviewScroll.pane_id, cmd.PreviewScroll.offset_y);
    } else if ('SetStatusBar' in cmd) {
      renderStatusBar(cmd.SetStatusBar.left, cmd.SetStatusBar.right, cmd.SetStatusBar.warning_count);
    } else if ('SetFileTree' in cmd) {
      renderFileTree(cmd.SetFileTree as any);
    }
    // Other cmds (SetWarnings, SetImeCursorArea, etc.) handled by ime-bridge / chrome
  }
}

function applyLayout(panels: import('./types').PanelLayout[]): void {
  const canvas = document.getElementById('editor-canvas') as HTMLCanvasElement | null;
  const preview = document.getElementById('preview-pane') as HTMLElement | null;
  for (const panel of panels) {
    const el = panel.kind === 'Editor' ? canvas : panel.kind === 'Preview' ? preview : null;
    if (!el) continue;
    el.style.position = 'absolute';
    el.style.left   = `${panel.bounds.x}px`;
    el.style.top    = `${panel.bounds.y}px`;
    el.style.width  = `${panel.bounds.width}px`;
    el.style.height = `${panel.bounds.height}px`;
    if (el instanceof HTMLCanvasElement) {
      el.width  = panel.bounds.width;
      el.height = panel.bounds.height;
    }
  }
}
```

- [ ] **Step 6.7: Create `src-desktop/frontend/src/main.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DrawCmd, AppEvent, KeyEvent, Modifiers } from './types';
import { applyDrawCmds } from './renderer';
import { setupImeBridge } from './ime-bridge';

async function dispatchEvent(event: AppEvent): Promise<void> {
  const cmds = await invoke<DrawCmd[]>('dispatch', { event });
  applyDrawCmds(cmds);
}

function keyboardEventToAppEvent(e: KeyboardEvent): AppEvent | null {
  const mods: Modifiers = {
    ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey,
  };
  let key: import('./types').KeyCode;
  switch (e.key) {
    case 'Enter':     key = 'Enter'; break;
    case 'Backspace': key = 'Backspace'; break;
    case 'Delete':    key = 'Delete'; break;
    case 'Escape':    key = 'Escape'; break;
    case 'Tab':       key = 'Tab'; break;
    case 'ArrowUp':   key = 'ArrowUp'; break;
    case 'ArrowDown': key = 'ArrowDown'; break;
    case 'ArrowLeft': key = 'ArrowLeft'; break;
    case 'ArrowRight':key = 'ArrowRight'; break;
    case 'Home':      key = 'Home'; break;
    case 'End':       key = 'End'; break;
    case 'PageUp':    key = 'PageUp'; break;
    case 'PageDown':  key = 'PageDown'; break;
    default:
      if (e.key.length === 1) { key = { Char: e.key }; }
      else { return null; }
  }
  const kev: KeyEvent = { key, mods, text: e.key.length === 1 ? e.key : null };
  return { Key: kev };
}

async function main(): Promise<void> {
  // Initial resize event
  await dispatchEvent({ Resize: { width: window.innerWidth, height: window.innerHeight } });

  // Keyboard input
  window.addEventListener('keydown', async (e) => {
    if (e.isComposing) return; // IME handles these
    const ev = keyboardEventToAppEvent(e);
    if (ev) {
      e.preventDefault();
      await dispatchEvent(ev);
    }
  });

  // Window resize
  window.addEventListener('resize', async () => {
    await dispatchEvent({ Resize: { width: window.innerWidth, height: window.innerHeight } });
  });

  // IME bridge
  setupImeBridge(dispatchEvent);
}

main().catch(console.error);
```

- [ ] **Step 6.8: Create minimal stub files so TypeScript compiles**

Create stub `src-desktop/frontend/src/editor-canvas.ts`:
```typescript
export function renderEditorFrame(_frame: any): void {}
```

Create stub `src-desktop/frontend/src/preview.ts`:
```typescript
export function renderPreviewMount(_pane_id: any, _html: string): void {}
export function renderPreviewPatch(_pane_id: any, _patches: any[]): void {}
export function renderPreviewScroll(_pane_id: any, _offset: number): void {}
```

Create stub `src-desktop/frontend/src/chrome.ts`:
```typescript
export function renderTabBar(_pane_id: any, _tabs: any[], _active: number): void {}
export function renderStatusBar(_left: string, _right: string, _count: number): void {}
export function renderFileTree(_tree: any): void {}
```

Create stub `src-desktop/frontend/src/ime-bridge.ts`:
```typescript
export function setupImeBridge(_dispatch: any): void {}
```

- [ ] **Step 6.9: Create `src-desktop/frontend/src/style.css`**

```css
* { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg: #0b0f19;
  --fg: #e6edf3;
  --muted: #aab6c3;
  --card: #121a2a;
  --border: #23304a;
  --accent: #7aa2f7;
  --tab-h: 32px;
  --status-h: 24px;
  font-size: 14px;
}

html, body { background: var(--bg); color: var(--fg); height: 100vh; overflow: hidden; }

#app { display: flex; flex-direction: column; height: 100vh; }

#tab-bar {
  height: var(--tab-h); background: var(--card); border-bottom: 1px solid var(--border);
  display: flex; align-items: center; overflow: hidden; flex-shrink: 0;
}

.tab {
  height: 100%; padding: 0 12px; display: flex; align-items: center; gap: 6px;
  border-right: 1px solid var(--border); cursor: pointer; font-size: 13px;
  color: var(--muted); white-space: nowrap;
}
.tab.active { color: var(--fg); background: rgba(255,255,255,0.04); }
.tab .dirty::after { content: '●'; font-size: 8px; color: var(--accent); margin-left: 4px; }

#main-area { flex: 1; position: relative; overflow: hidden; }

#editor-canvas { position: absolute; background: var(--bg); cursor: text; outline: none; }
#preview-pane  { position: absolute; overflow-y: auto; padding: 16px; background: var(--bg); }

#status-bar {
  height: var(--status-h); background: var(--accent); color: #1a202e;
  display: flex; justify-content: space-between; align-items: center;
  padding: 0 8px; font-size: 12px; flex-shrink: 0;
}

/* Preview styles — nm-* classes from web-playground */
#preview-pane a { color: var(--accent); }
#preview-pane hr { border: none; border-top: 1px solid var(--border); margin: 24px 0; }
#preview-pane .nm-sec { padding: 0.5em; padding-left: 2em; margin: 1em;
  border-left: 3px solid var(--border); border-radius: 1em; }
#preview-pane code { font-family: ui-monospace, monospace; font-size: 0.9em; }
#preview-pane .nm-code { background: var(--card); padding: 12px; border-radius: 8px;
  overflow-x: auto; margin: 16px 0; }
#preview-pane ruby rt { font-size: 0.65em; color: var(--muted); }
```

- [ ] **Step 6.10: Install deps and verify TypeScript compiles**

**注意:** ワークツリー内のフロントエンドディレクトリを使うこと。

```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend && npm install && npx tsc --noEmit 2>&1
```

Expected: no TypeScript errors.

- [ ] **Step 6.11: Commit**

```sh
git add src-desktop/frontend/
git commit -m "feat(src-desktop): frontend scaffold, types, renderer.ts, style"
```

---

## Task 7: Fix view.rs cursor position + editor-canvas.ts

**Files:**
- Modify: `src-desktop-layout/src/view.rs` (fix cursor pixel position)
- Replace: `src-desktop/frontend/src/editor-canvas.ts`

**Why:** `emit_editor_frame` in view.rs hardcodes `CursorDraw { x: 0.0, y: 0.0, height: 16.0 }`. Fix it to use `vm.cursor.{line, visual_col}` with shared constants `CHAR_W = 8.0` and `LINE_H = 20.0`.

- [ ] **Step 7.1: Write failing test for cursor position**

Add to `src-desktop-layout/src/view.rs` tests:

```rust
#[test]
fn cursor_position_reflects_line_and_col() {
    use src_desktop_types::{DocId, DocMeta, Tab, VfsPath, EditorViewModel, CursorDisplay};
    let mut m = Model::new(800, 600);
    // Add an editor pane with a doc open
    let doc_id = DocId(1);
    m.workspace.open_docs.insert(doc_id, DocMeta {
        path: VfsPath::from("/test.n.md"),
        title: "test".into(),
        dirty: false,
    });
    if let crate::model::PanelNode::Leaf(ref mut p) = m.layout.root {
        p.tabs.push(Tab { doc_id, title: "test".into(), dirty: false });
    }
    let vm = EditorViewModel {
        doc_id,
        cursor: CursorDisplay { line: 2, visual_col: 5, blink: true },
        ..Default::default()
    };
    m.workspace.editors.insert(doc_id, vm);
    let cmds = view(&m);
    let frame = cmds.iter().find_map(|c| {
        if let DrawCmd::EditorFrame { cursor, .. } = c { Some(*cursor) } else { None }
    }).expect("no EditorFrame emitted");
    // line=2, col=5: x = 5 * CHAR_W, y = 2 * LINE_H
    assert!((frame.x - 5.0 * super::CHAR_W).abs() < 0.1, "x was {}", frame.x);
    assert!((frame.y - 2.0 * super::LINE_H).abs() < 0.1, "y was {}", frame.y);
}
```

Run: `cargo test -p src-desktop-layout cursor_position 2>&1 | tail -5`

Expected: **FAIL** — cursor x and y are both 0.

- [ ] **Step 7.2: Fix `emit_editor_frame` in `src-desktop-layout/src/view.rs`**

Add constants at the top (before `const DIVIDER_SIZE`):

```rust
pub const CHAR_W: f32 = 8.0;
pub const LINE_H: f32 = 20.0;
```

Replace the `emit_editor_frame` function:

```rust
fn emit_editor_frame(cmds: &mut Vec<DrawCmd>, pane_id: PaneId, bounds: Rect, vm: &EditorViewModel) {
    let cursor_x = vm.cursor.visual_col as f32 * CHAR_W - vm.scroll.x;
    let cursor_y = vm.cursor.line as f32 * LINE_H - vm.scroll.y;
    cmds.push(DrawCmd::EditorFrame {
        pane_id,
        bounds,
        lines:     vm.visible_lines.clone(),
        cursor:    CursorDraw { x: cursor_x, y: cursor_y, height: LINE_H },
        selection: None,
        preedit:   vm.preedit.clone(),
        scroll:    vm.scroll,
    });
}
```

- [ ] **Step 7.3: Run the test**

```sh
cargo test -p src-desktop-layout cursor_position 2>&1 | tail -5
```

Expected: PASS.

- [ ] **Step 7.4: Verify full workspace**

```sh
cargo test 2>&1 | grep -E "FAILED|^error" | head -5
```

Expected: no failures.

- [ ] **Step 7.5: Implement `src-desktop/frontend/src/editor-canvas.ts`**

```typescript
import type { EditorLine, CursorDraw, SelectionDraw, PreeditDraw, Rect, ScrollOffset, TextSpan } from './types';

// Must match CHAR_W / LINE_H in src-desktop-layout/src/view.rs
const CHAR_W  = 8;
const LINE_H  = 20;
const FONT    = `${LINE_H * 0.7}px ui-monospace, "Cascadia Code", monospace`;

const BG_COLOR = '#0b0f19';

interface EditorFrameCmd {
  pane_id:   unknown;
  bounds:    Rect;
  lines:     EditorLine[];
  cursor:    CursorDraw;
  selection: SelectionDraw | null;
  preedit:   PreeditDraw | null;
  scroll:    ScrollOffset;
}

export function renderEditorFrame(frame: EditorFrameCmd): void {
  const canvas = document.getElementById('editor-canvas') as HTMLCanvasElement | null;
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  // Background
  ctx.fillStyle = BG_COLOR;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.font = FONT;
  ctx.textBaseline = 'top';

  const LINE_PADDING_X = 4; // px left margin for line numbers / gutter

  // Draw selection
  if (frame.selection) {
    ctx.fillStyle = 'rgba(122, 162, 247, 0.25)';
    for (const r of frame.selection.rects) {
      ctx.fillRect(r.x + LINE_PADDING_X, r.y, r.width, r.height);
    }
  }

  // Draw lines
  for (const line of frame.lines) {
    const y = line.line_no * LINE_H - frame.scroll.y;
    if (y + LINE_H < 0 || y > canvas.height) continue; // off-screen

    let x = LINE_PADDING_X - frame.scroll.x;
    for (const span of line.spans) {
      ctx.fillStyle = colorToHex(span.color);
      ctx.font = spanFont(span);
      ctx.fillText(span.text, x, y + 2);
      x += measureSpanWidth(span.text);
    }
  }

  // Draw preedit (underlined text at cursor)
  if (frame.preedit) {
    const cx = frame.cursor.x + LINE_PADDING_X;
    const cy = frame.cursor.y;
    ctx.fillStyle = '#e6edf3';
    ctx.font = FONT;
    ctx.fillText(frame.preedit.text, cx, cy + 2);
    // Underline
    ctx.strokeStyle = '#7aa2f7';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx, cy + LINE_H - 2);
    ctx.lineTo(cx + measureSpanWidth(frame.preedit.text), cy + LINE_H - 2);
    ctx.stroke();
  }

  // Draw cursor (blinking handled by CSS / TODO: blink timer)
  ctx.fillStyle = '#7aa2f7';
  ctx.fillRect(frame.cursor.x + LINE_PADDING_X, frame.cursor.y, 2, frame.cursor.height);
}

function spanFont(span: TextSpan): string {
  const weight = span.bold ? 'bold' : 'normal';
  const style  = span.italic ? 'italic' : 'normal';
  return `${style} ${weight} ${LINE_H * 0.7}px ui-monospace, "Cascadia Code", monospace`;
}

function measureSpanWidth(text: string): number {
  // Approximate: ASCII = CHAR_W, CJK = 2*CHAR_W
  let w = 0;
  for (const ch of text) {
    w += ch.charCodeAt(0) > 0x2E7F ? CHAR_W * 2 : CHAR_W;
  }
  return w;
}

function colorToHex(color: number): string {
  const r = (color >> 16) & 0xFF;
  const g = (color >> 8)  & 0xFF;
  const b =  color        & 0xFF;
  return `rgb(${r},${g},${b})`;
}
```

- [ ] **Step 7.6: Verify TypeScript compiles**

```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend && npx tsc --noEmit 2>&1
```

Expected: no errors.

- [ ] **Step 7.7: Commit**

```sh
git add src-desktop-layout/src/view.rs src-desktop/frontend/src/editor-canvas.ts
git commit -m "feat(src-desktop): fix cursor pixel position in view.rs; implement editor-canvas"
```

---

## Task 8: preview.ts + chrome.ts + ime-bridge.ts

**Files:**
- Replace: `src-desktop/frontend/src/preview.ts`
- Replace: `src-desktop/frontend/src/chrome.ts`
- Replace: `src-desktop/frontend/src/ime-bridge.ts`

- [ ] **Step 8.1: Implement `src-desktop/frontend/src/preview.ts`**

```typescript
import type { HtmlPatch, PaneId } from './types';

export function renderPreviewMount(_pane_id: PaneId, html: string): void {
  const el = document.getElementById('preview-pane');
  if (!el) return;
  el.innerHTML = html;
  renderKaTeX(el);
}

export function renderPreviewPatch(_pane_id: PaneId, patches: HtmlPatch[]): void {
  const el = document.getElementById('preview-pane');
  if (!el) return;
  for (const patch of patches) {
    const target = el.querySelector(`[data-block-id="${patch.block_id}"]`);
    if (target) {
      target.outerHTML = patch.html;
    }
  }
  renderKaTeX(el);
}

export function renderPreviewScroll(_pane_id: PaneId, offset_y: number): void {
  const el = document.getElementById('preview-pane');
  if (el) el.scrollTop = offset_y;
}

declare const katex: {
  render(tex: string, el: HTMLElement, opts: { displayMode: boolean; throwOnError: boolean }): void;
} | undefined;

function renderKaTeX(root: HTMLElement): void {
  if (typeof katex === 'undefined') return;
  root.querySelectorAll<HTMLElement>('.math-inline, .math-display').forEach(el => {
    const texSpan = el.querySelector<HTMLElement>('.math-tex');
    if (texSpan) {
      katex.render(texSpan.textContent ?? '', el, {
        displayMode: el.classList.contains('math-display'),
        throwOnError: false,
      });
    }
  });
}
```

- [ ] **Step 8.2: Implement `src-desktop/frontend/src/chrome.ts`**

```typescript
import type { PaneId, TabInfo } from './types';

export function renderTabBar(_pane_id: PaneId, tabs: TabInfo[], active_tab: number): void {
  const el = document.getElementById('tab-bar');
  if (!el) return;
  el.innerHTML = tabs.map((t, i) => {
    const active = i === active_tab ? ' active' : '';
    const dirty  = t.dirty ? '<span class="dirty"></span>' : '';
    return `<div class="tab${active}" data-tab-index="${i}">${escHtml(t.title)}${dirty}</div>`;
  }).join('');
}

export function renderStatusBar(left: string, right: string, warning_count: number): void {
  const l = document.getElementById('status-left');
  const r = document.getElementById('status-right');
  if (l) l.textContent = left;
  if (r) r.textContent = warning_count > 0 ? `${right}  ⚠ ${warning_count}` : right;
}

export function renderFileTree(_tree: unknown): void {
  // TODO: implement file tree panel
}

function escHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
```

- [ ] **Step 8.3: Implement `src-desktop/frontend/src/ime-bridge.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { AppEvent, ImeEvent } from './types';

async function pushIme(event: ImeEvent): Promise<void> {
  await invoke('push_ime_event', { event });
}

export function setupImeBridge(dispatch: (ev: AppEvent) => Promise<void>): void {
  const canvas = document.getElementById('editor-canvas');
  if (!canvas) return;

  // Hidden input element for IME — placed at cursor position by SetImeCursorArea
  const ime_input = document.createElement('input');
  ime_input.style.cssText = 'position:absolute;opacity:0;pointer-events:none;width:1px;height:1px;';
  document.body.appendChild(ime_input);

  canvas.addEventListener('click', () => ime_input.focus());

  ime_input.addEventListener('compositionstart', () => {
    pushIme('Start').catch(console.error);
  });

  ime_input.addEventListener('compositionupdate', (e: CompositionEvent) => {
    pushIme({ Update: { preedit: e.data, cursor: null } }).catch(console.error);
  });

  ime_input.addEventListener('compositionend', (e: CompositionEvent) => {
    const text = e.data;
    ime_input.value = '';
    if (text) {
      pushIme({ Commit: { text } })
        .then(() => dispatch({ Key: { key: { Char: '' }, mods: { ctrl:false, shift:false, alt:false, meta:false }, text: '' } }))
        .catch(console.error);
    } else {
      pushIme('Cancel').catch(console.error);
    }
  });

  // SetImeCursorArea event from Rust — move hidden input to cursor position
  document.addEventListener('tauri://event', (e: any) => {
    if (e.type === 'ime-cursor-area' && e.detail) {
      const r = e.detail.payload;
      ime_input.style.left = `${r.x}px`;
      ime_input.style.top  = `${r.y}px`;
    }
  });
}
```

- [ ] **Step 8.4: Verify TypeScript compiles**

```sh
cd /mnt/d/project/gloss/.worktrees/src-desktop/src-desktop/frontend && npx tsc --noEmit 2>&1
```

Expected: no errors.

- [ ] **Step 8.5: Commit**

```sh
git add src-desktop/frontend/src/preview.ts src-desktop/frontend/src/chrome.ts src-desktop/frontend/src/ime-bridge.ts
git commit -m "feat(src-desktop): preview, chrome, IME bridge frontend modules"
```

---

## Task 9: src-cli refactoring (§10)

**Files:**
- Modify: `src-cli/Cargo.toml`
- Replace: `src-cli/src/main.rs`

The spec (§10) replaces the hand-rolled parser pipeline with `AppCore`. The HTML_HEAD/HTML_TAIL constants stay; only the pipeline changes. After this refactor `src-cli` no longer directly uses `src-plugin` or `src-core`.

- [ ] **Step 9.1: Update `src-cli/Cargo.toml`**

Add dependencies, remove direct `src-core` and `src-plugin` deps:

```toml
[package]
name = "src-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
src-desktop-core   = { path = "../src-desktop-core" }
src-desktop-native = { path = "../src-desktop-native" }
src-desktop-types  = { path = "../src-desktop-types" }
```

- [ ] **Step 9.2: Write a failing test (compile check)**

The old code uses `src_core::parser::Parser` directly. After removing `src-core` from deps, it must not compile until we rewrite main.rs.

Run: `cargo build -p src-cli 2>&1 | head -5`

Expected: **compile error** — `src_core` not found.

- [ ] **Step 9.3: Rewrite `src-cli/src/main.rs`**

```rust
use std::{env, process};
use src_desktop_types::VfsPath;
use src_desktop_core::AppCore;
use src_desktop_native::{NativeFs, make_plugin_host, load_app_config};

const HTML_HEAD: &str = r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>Gloss Markdown Preview</title>
<style>
:root {
  --bg: #0b0f19; --fg: #e6edf3; --muted: #aab6c3;
  --card: #121a2a; --border: #23304a; --code: #0f1626; --accent: #7aa2f7;
}
html, body { background: var(--bg); color: var(--fg);
  font-family: system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial;
  line-height: 1.65; margin: 0; }
main { max-width: 980px; margin: 24px auto; padding: 0 16px; }
a { color: var(--accent); }
hr { border: none; border-top: 1px solid var(--border); margin: 24px 0; }
.nm-sec { padding: 0.5em; padding-left: 2em; margin: 1em;
  border-left: 3px solid var(--border); border-radius: 1em; }
h1,h2,h3,h4,h5,h6 { margin: 18px 0 10px; }
p { margin: 10px 0; }
ul,ol { margin: 10px 0 10px 22px; }
strong { font-weight: 700; }
em { font-style: italic; }
del { text-decoration: line-through; color: var(--muted); }
.nm-code { background: var(--code); padding: 12px; overflow: auto; margin: 16px 0;
  border-radius: 8px; border: 1px solid var(--border); }
.nm-code code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px; white-space: pre; }
.nm-code-inline { background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.10);
  border-radius: 8px; padding: 1px 6px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 0.9em; }
.nm-card-link { display: block; border: 1px solid var(--border); border-radius: 10px;
  padding: 12px 16px; margin: 16px 0; background: var(--card); color: var(--fg);
  text-decoration: none; }
.nm-card-url { display: block; font-size: 0.85em; color: var(--muted); word-break: break-all; }
ruby rt { font-size: 0.65em; color: var(--muted); }
.nm-ruby { ruby-position: over; }
.nm-anno { ruby-position: under; }
.math-inline { color: var(--muted); }
.math-display { display: block; padding: 8px 10px; margin: 8px 0;
  background: rgba(255,255,255,0.03); border: 1px dashed var(--border);
  border-radius: 10px; overflow-x: auto; }
</style>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js"></script>
<script>
document.addEventListener("DOMContentLoaded", function() {
  document.querySelectorAll('.math-inline, .math-display').forEach(function(el) {
    var texSpan = el.querySelector('.math-tex');
    if (texSpan) { katex.render(texSpan.textContent, el,
      { displayMode: el.classList.contains('math-display'), throwOnError: false }); }
  });
});
</script>
</head>
<body><main>
"#;

const HTML_TAIL: &str = "\n</main></body></html>\n";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.n.md> [output.html] [--config <path>]", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = if args.len() >= 3 && !args[2].starts_with("--") {
        args[2].clone()
    } else if let Some(stem) = input_path.strip_suffix(".n.md") {
        format!("{stem}.html")
    } else if let Some(stem) = input_path.strip_suffix(".md") {
        format!("{stem}.html")
    } else {
        format!("{input_path}.html")
    };

    // Config path: --config <path> or default gloss.toml
    let config_path = args.windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| w[1].as_str())
        .unwrap_or("gloss.toml");

    let config = load_app_config(config_path);
    let host = make_plugin_host(&config.plugins);
    let mut core = AppCore::new(NativeFs, host, config);

    let vpath = VfsPath::from(input_path.as_str());
    let doc_id = match core.open_file(&vpath) {
        Ok((id, _)) => id,
        Err(e) => {
            eprintln!("Error reading {input_path}: {e:?}");
            process::exit(1);
        }
    };

    let (html_body, _, warnings) = match core.render_full(doc_id) {
        Some(r) => r,
        None => {
            eprintln!("Error: render_full returned None for {input_path}");
            process::exit(1);
        }
    };

    for w in &warnings {
        eprintln!("\x1b[33m[{}:{}:{}] {} — {}\x1b[0m",
            input_path, w.line, w.col, w.code, w.message);
    }

    let final_html = format!("{HTML_HEAD}{html_body}{HTML_TAIL}");
    if let Err(e) = std::fs::write(&output_path, final_html) {
        eprintln!("Error writing {output_path}: {e}");
        process::exit(1);
    }

    println!("Successfully compiled {input_path} -> {output_path}");
}
```

- [ ] **Step 9.4: Run workspace tests**

```sh
cd /mnt/d/project/gloss && cargo test 2>&1 | grep -E "test result|^error" | head -20
```

Expected: all `test result: ok`, no errors.

- [ ] **Step 9.5: Smoke-test the CLI**

```sh
cd /mnt/d/project/gloss && echo "# Hello\n[漢字/かんじ]" > /tmp/test.n.md
cargo run -p src-cli -- /tmp/test.n.md /tmp/test.html 2>&1
grep "nm-ruby" /tmp/test.html | head -2
```

Expected: HTML output contains `nm-ruby`.

- [ ] **Step 9.6: Commit**

```sh
git add src-cli/src/main.rs src-cli/Cargo.toml Cargo.lock
git commit -m "refactor(src-cli): rewrite pipeline using AppCore + src-desktop-native (spec §10)"
```

---

## Final: merge and release

After all tasks complete and tests pass:

- Merge `feat/src-desktop` to `main`
- Tag `cli/v0.2.0` (refactored CLI)
- Tag `desktop/v0.1.0` (first desktop app release, pre-alpha)
