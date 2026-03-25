# src-desktop-core (Plan 4 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `src-desktop-core` crate — a `no_std + alloc` application logic layer with `AppCore<Fs,Ph>`, document management, edit operations, rendering pipeline, and `execute_cmds`.

**Architecture:** Two modules: `app_core` (AppCore struct + all methods) and `renderer` (PluginAwareHtmlRenderer). AppCore is generic over `Fs: FileSystem` and `Ph: PluginHost`. No I/O — uses trait objects injected at construction. Rendering pipeline: `buffer.as_str() → Parser → filter warnings → to_plugin_events → plugin lint → PluginAwareHtmlRenderer → html`.

**Tech Stack:** Rust `no_std + alloc`, `src-core` (Parser, HtmlRenderer), `src-editor` (EditorState, Highlighter), `src-desktop-types`, `src-plugin-types`.

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md` §6

**Reference:** For `PluginAwareHtmlRenderer`, read `/mnt/d/project/gloss/src-plugin/src/renderer.rs` — the new version uses `PluginHost` trait instead of `GlossPluginHost` but the event interception pattern is the same.

---

## File Map

| File | Responsibility |
|------|----------------|
| `src-desktop-core/Cargo.toml` | Deps: src-core, src-editor, src-desktop-types, src-plugin-types |
| `src-desktop-core/src/lib.rs` | `#![no_std]`, module declarations, re-exports |
| `src-desktop-core/src/app_core.rs` | `AppCore<Fs,Ph>`, `DocumentState`, all methods |
| `src-desktop-core/src/renderer.rs` | `PluginAwareHtmlRenderer<Ph>`, `render_card_output` |

---

## Task 1: Crate scaffold + AppCore struct + DocumentState + view model helper

**Files:**
- Create: `src-desktop-core/Cargo.toml`
- Create: `src-desktop-core/src/lib.rs`
- Create: `src-desktop-core/src/renderer.rs` (stub)
- Create: `src-desktop-core/src/app_core.rs`
- Modify: `Cargo.toml` (workspace root)

**Note:** If `HighlightContext` in `src-editor/src/highlighter.rs` does not derive `Clone`, add `#[derive(Clone, Copy)]` to it as part of this task — it is needed by the view model builder.

- [ ] **Step 1.1: Add `src-desktop-core` to workspace**

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    "src-core", "src-web", "src-cli",
    "src-plugin-types", "src-plugin",
    "src-desktop-types", "src-editor",
    "src-desktop-layout", "src-desktop-core",
]
```

- [ ] **Step 1.2: Create `src-desktop-core/Cargo.toml`**

```toml
[package]
name = "src-desktop-core"
version = "0.1.0"
edition = "2021"

[dependencies]
src-core          = { path = "../src-core" }
src-editor        = { path = "../src-editor" }
src-desktop-types = { path = "../src-desktop-types" }
src-plugin-types  = { path = "../src-plugin-types" }

[dev-dependencies]
src-desktop-types = { path = "../src-desktop-types", features = ["test-utils"] }
```

- [ ] **Step 1.3: Create `src-desktop-core/src/lib.rs`**

```rust
#![no_std]
extern crate alloc;

pub mod app_core;
pub mod renderer;

pub use app_core::{AppCore, DocumentState};
pub use renderer::PluginAwareHtmlRenderer;
```

- [ ] **Step 1.4: Create `src-desktop-core/src/renderer.rs` stub**

```rust
use src_desktop_types::PluginHost;
use src_core::Event;

pub struct PluginAwareHtmlRenderer<'a, Ph: PluginHost> {
    pub host: &'a mut Ph,
}

impl<'a, Ph: PluginHost> PluginAwareHtmlRenderer<'a, Ph> {
    pub fn new(host: &'a mut Ph) -> Self { Self { host } }

    pub fn render<'ev>(
        &mut self,
        _events: &[Event<'ev>],
        _out: &mut alloc::string::String,
        _source: &str,
        _markdown: &str,
    ) {
        // stub — implemented in Task 4
    }
}
```

- [ ] **Step 1.5: Write the failing test**

Create `src-desktop-core/src/app_core.rs` with just the test module:

```rust
#[cfg(test)]
mod tests {
    use src_desktop_types::{AppConfig, MemoryVfs};

    #[cfg(any(test, feature = "test-utils"))]
    use src_desktop_types::NoopPluginHost;

    use crate::app_core::AppCore;

    #[test]
    fn new_has_no_docs() {
        let core: AppCore<MemoryVfs, NoopPluginHost> =
            AppCore::new(MemoryVfs::default(), NoopPluginHost, AppConfig::default());
        assert_eq!(core.doc_count(), 0);
    }
}
```

- [ ] **Step 1.6: Run to confirm failure**

```
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test -p src-desktop-core new_has_no_docs 2>&1 | tail -5
```

Expected: compile error (AppCore not defined yet).

- [ ] **Step 1.7: Implement `src-desktop-core/src/app_core.rs`**

```rust
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use src_core::{HtmlRenderer, Parser, fnv1a};
use src_editor::{
    EditorState, Highlighter, HighlightContext,
    cursor::Cursor,
};
use src_desktop_types::{
    AppCmd, AppConfig, AppEvent, CursorDisplay, DirEntry, DocId, EditorLine, EditorViewModel,
    FileSystem, FsError, HtmlPatch, ImeEvent, KeyCode, KeyEvent, PaneId, PluginHost,
    PluginWarning, PreeditDraw, VfsPath,
};
use src_plugin_types::{
    PluginFrontMatterField, PluginWarning as PW,
    to_plugin_events,
};

use crate::renderer::PluginAwareHtmlRenderer;

// ── DocumentState ─────────────────────────────────────────────────────────────

pub struct DocumentState {
    pub path:              VfsPath,
    pub editor:            EditorState,
    pub last_rendered_ver: u64,
    pub last_html:         Option<String>,
    pub last_block_hashes: Vec<u64>,
    pub warnings:          Vec<PW>,
    pub fm_config:         Option<AppConfig>,
}

// ── AppCore ───────────────────────────────────────────────────────────────────

pub struct AppCore<Fs: FileSystem, Ph: PluginHost> {
    pub fs:          Fs,
    pub plugin_host: Ph,
    pub config:      AppConfig,
    docs:            BTreeMap<DocId, DocumentState>,
    next_doc_id:     u64,
}

impl<Fs: FileSystem, Ph: PluginHost> AppCore<Fs, Ph> {

    pub fn new(fs: Fs, plugin_host: Ph, config: AppConfig) -> Self {
        Self { fs, plugin_host, config, docs: BTreeMap::new(), next_doc_id: 1 }
    }

    pub fn doc_count(&self) -> usize { self.docs.len() }

    // ── Helper ────────────────────────────────────────────────────────────────

    fn alloc_doc_id(&mut self) -> DocId {
        let id = DocId(self.next_doc_id);
        self.next_doc_id += 1;
        id
    }

    fn make_view_model(doc_id: DocId, doc: &mut DocumentState) -> EditorViewModel {
        let buf_str = doc.editor.buffer.as_str();
        let total_lines = doc.editor.buffer.line_count() as u32;

        let mut ctx = HighlightContext::Normal;
        let mut visible_lines: Vec<EditorLine> = Vec::new();

        for (line_no, line) in buf_str.lines().enumerate() {
            // Compute next context before consuming current ctx
            let next_ctx = update_ctx(line, ctx);
            let spans = Highlighter::highlight_line(line, ctx);
            ctx = next_ctx;
            visible_lines.push(src_desktop_types::EditorLine {
                line_no: line_no as u32,
                spans,
            });
        }
        // Empty buffer has no lines(); add one empty line
        if visible_lines.is_empty() {
            visible_lines.push(src_desktop_types::EditorLine { line_no: 0, spans: alloc::vec![] });
        }

        let cursor_display = CursorDisplay {
            line:       doc.editor.cursor.line as u32,
            visual_col: doc.editor.cursor.preferred_visual_col,
            blink:      true,
        };

        let preedit = doc.editor.ime.composing.as_ref().map(|p| PreeditDraw {
            text:            p.text.clone(),
            underline_range: p.cursor,
        });

        EditorViewModel {
            doc_id,
            visible_lines,
            total_lines: total_lines.max(1),
            cursor:    cursor_display,
            selection: None,
            preedit,
            scroll:    doc.editor.scroll,
            dirty:     doc.editor.version > 0,
        }
    }

    // ── Document management ───────────────────────────────────────────────────

    pub fn open_bytes(&mut self, path: VfsPath, content: Vec<u8>) -> (DocId, EditorViewModel) {
        let text = String::from_utf8(content).unwrap_or_default();
        let editor = EditorState::from_str(&text);
        let doc_id = self.alloc_doc_id();
        let mut doc = DocumentState {
            path,
            editor,
            last_rendered_ver: u64::MAX,
            last_html:         None,
            last_block_hashes: Vec::new(),
            warnings:          Vec::new(),
            fm_config:         None,
        };
        let vm = Self::make_view_model(doc_id, &mut doc);
        self.docs.insert(doc_id, doc);
        (doc_id, vm)
    }

    pub fn open_file(&mut self, path: &VfsPath) -> Result<(DocId, EditorViewModel), FsError> {
        let bytes = self.fs.read(path)?;
        Ok(self.open_bytes(path.clone(), bytes))
    }

    pub fn close_doc(&mut self, doc_id: DocId) {
        self.docs.remove(&doc_id);
    }

    pub fn save_doc(&mut self, doc_id: DocId) -> Result<(), FsError> {
        let doc = self.docs.get(&doc_id).ok_or(FsError::NotFound(VfsPath::from("")))?;
        let content = doc.editor.buffer.as_str();
        let path = doc.path.clone();
        self.fs.write(&path, content.as_bytes())
    }

    // ── Edit operations ───────────────────────────────────────────────────────

    pub fn apply_key(&mut self, doc_id: DocId, key: &KeyEvent) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        let editor = &mut doc.editor;

        match &key.key {
            KeyCode::Char(c) if !key.mods.ctrl && !key.mods.alt => {
                let mut s = String::new();
                s.push(*c);
                editor.insert_at_cursor(&s);
            }
            KeyCode::Backspace => { editor.backspace(); }
            KeyCode::Enter => { editor.insert_at_cursor("\n"); }
            KeyCode::Tab if !key.mods.ctrl => { editor.insert_at_cursor("    "); }
            KeyCode::ArrowRight => { editor.cursor.move_right(&mut editor.buffer); }
            KeyCode::ArrowLeft  => { editor.cursor.move_left(&mut editor.buffer); }
            KeyCode::ArrowDown  => { editor.cursor.move_down(&mut editor.buffer); }
            KeyCode::ArrowUp    => { editor.cursor.move_up(&mut editor.buffer); }
            KeyCode::Home       => { editor.cursor.move_line_start(&mut editor.buffer); }
            KeyCode::End        => { editor.cursor.move_line_end(&mut editor.buffer); }
            _ => return None,
        }

        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn apply_ime(&mut self, doc_id: DocId, ime: ImeEvent) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        doc.editor.ime.apply(ime, &mut doc.editor.buffer, &mut doc.editor.cursor);
        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn undo(&mut self, doc_id: DocId) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        doc.editor.undo.undo(&mut doc.editor.buffer, &mut doc.editor.cursor);
        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn redo(&mut self, doc_id: DocId) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        doc.editor.undo.redo(&mut doc.editor.buffer, &mut doc.editor.cursor);
        Some(Self::make_view_model(doc_id, doc))
    }

    // ── Front matter config override ──────────────────────────────────────────

    /// Placeholder: extract lint/plugin config from front matter fields.
    /// Full parsing left to future iteration; currently clears fm_config.
    pub fn apply_front_matter(&mut self, doc_id: DocId, _fields: &[PluginFrontMatterField]) {
        if let Some(doc) = self.docs.get_mut(&doc_id) {
            doc.fm_config = None;
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render_full(&mut self, doc_id: DocId)
        -> Option<(String, Vec<u64>, Vec<PW>)>
    {
        // Phase 1: extract data (release docs borrow before using plugin_host)
        let (content, source, effective_lint) = {
            let doc = self.docs.get(&doc_id)?;
            let content = doc.editor.buffer.as_str();
            let source  = doc.path.as_str().to_owned();
            let lint    = doc.fm_config.as_ref()
                .map(|c| c.lint.clone())
                .unwrap_or_else(|| self.config.lint.clone());
            (content, source, lint)
        };

        // Phase 2: parse
        let mut parser = Parser::new_with_source(&content, &source);
        let events: Vec<src_core::Event<'_>> = (&mut parser).collect();
        let raw_warnings = core::mem::take(&mut parser.warnings);

        // Filter warnings by lint config
        let filtered: Vec<PW> = raw_warnings.iter()
            .filter(|w| effective_lint.is_enabled(w.code))
            .map(|w| PW { code: w.code.into(), message: w.message.clone(), line: w.line, col: w.col })
            .collect();

        // Plugin lint
        let plugin_events = to_plugin_events(&events);
        let mut extra = self.plugin_host.run_lint_rule(&source, &content, &filtered, &plugin_events);
        let mut all_warnings = filtered;
        all_warnings.append(&mut extra);

        // Render HTML
        let mut html = String::new();
        {
            let mut renderer = PluginAwareHtmlRenderer::new(&mut self.plugin_host);
            renderer.render(&events, &mut html, &source, &content);
        }

        let block_hashes = alloc::vec![fnv1a(html.as_bytes())];

        // Phase 3: update doc state
        let doc = self.docs.get_mut(&doc_id)?;
        doc.last_html         = Some(html.clone());
        doc.last_block_hashes = block_hashes.clone();
        doc.last_rendered_ver = doc.editor.version as u64;
        doc.warnings          = all_warnings.clone();

        Some((html, block_hashes, all_warnings))
    }

    pub fn render_diff(&mut self, doc_id: DocId)
        -> Option<(Vec<HtmlPatch>, Vec<PW>)>
    {
        // Simplified: always full render, return as single patch
        let (html, hashes, warnings) = self.render_full(doc_id)?;
        let hash = hashes.into_iter().next().unwrap_or(0);
        Some((alloc::vec![HtmlPatch { block_id: hash, html }], warnings))
    }

    // ── FS helpers ────────────────────────────────────────────────────────────

    pub fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        let entries = self.fs.list_dir(path)?;
        Ok(entries)
    }

    pub fn create_file(&mut self, path: &VfsPath) -> Result<(), FsError> {
        self.fs.write(path, &[])
    }

    pub fn delete_file(&mut self, path: &VfsPath) -> Result<(), FsError> {
        self.fs.delete(path)
    }

    pub fn rename_file(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        self.fs.rename(from, to)
    }

    pub fn apply_config(&mut self, config: AppConfig) {
        self.config = config;
    }

    // ── execute_cmds ──────────────────────────────────────────────────────────

    pub fn execute_cmds(&mut self, cmds: Vec<AppCmd>)
        -> (Vec<AppEvent>, Vec<AppCmd>)
    {
        let mut result_events: Vec<AppEvent> = Vec::new();
        let mut shell_cmds:    Vec<AppCmd>   = Vec::new();

        for cmd in cmds {
            match cmd {
                AppCmd::WriteFile { path, content } => {
                    // Find doc by path; use doc's buffer if content is empty
                    let doc_content = if content.is_empty() {
                        self.docs.values()
                            .find(|d| d.path == path)
                            .map(|d| d.editor.buffer.as_str().into_bytes())
                            .unwrap_or(content)
                    } else {
                        content
                    };
                    match self.fs.write(&path, &doc_content) {
                        Ok(()) => result_events.push(AppEvent::FileSaved { path }),
                        Err(e) => result_events.push(AppEvent::FileError {
                            path, error: alloc::format!("{e}"),
                        }),
                    }
                }
                AppCmd::ListDir { path } => {
                    match self.fs.list_dir(&path) {
                        Ok(entries) => result_events.push(AppEvent::DirLoaded { path, entries }),
                        Err(e) => result_events.push(AppEvent::FileError {
                            path, error: alloc::format!("{e}"),
                        }),
                    }
                }
                AppCmd::RunRender { pane_id, doc_id } => {
                    match self.render_full(doc_id) {
                        Some((html, _hashes, warnings)) => {
                            result_events.push(AppEvent::RenderComplete { pane_id, html, warnings });
                        }
                        None => {}
                    }
                }
                AppCmd::RunLint { doc_id } => {
                    // Extract data first (phase 1)
                    let (content, source, effective_lint) = match self.docs.get(&doc_id) {
                        None => continue,
                        Some(doc) => {
                            let content = doc.editor.buffer.as_str();
                            let source  = doc.path.as_str().to_owned();
                            let lint    = doc.fm_config.as_ref()
                                .map(|c| c.lint.clone())
                                .unwrap_or_else(|| self.config.lint.clone());
                            (content, source, lint)
                        }
                    };
                    let mut parser = Parser::new_with_source(&content, &source);
                    let events: Vec<src_core::Event<'_>> = (&mut parser).collect();
                    let raw_warnings = core::mem::take(&mut parser.warnings);
                    let mut filtered: Vec<PW> = raw_warnings.iter()
                        .filter(|w| effective_lint.is_enabled(w.code))
                        .map(|w| PW { code: w.code.into(), message: w.message.clone(), line: w.line, col: w.col })
                        .collect();
                    let plugin_events = to_plugin_events(&events);
                    let mut extra = self.plugin_host.run_lint_rule(&source, &content, &filtered, &plugin_events);
                    filtered.append(&mut extra);
                    if let Some(doc) = self.docs.get_mut(&doc_id) {
                        doc.warnings = filtered.clone();
                    }
                    result_events.push(AppEvent::LintComplete { doc_id, warnings: filtered });
                }
                other => { shell_cmds.push(other); }
            }
        }

        (result_events, shell_cmds)
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn update_ctx(line: &str, ctx: HighlightContext) -> HighlightContext {
    match ctx {
        HighlightContext::Normal => {
            if line.starts_with("```") { HighlightContext::InCodeBlock { lang: "text" } }
            else if line.trim_start().starts_with("$$") { HighlightContext::InMathBlock }
            else { HighlightContext::Normal }
        }
        HighlightContext::InCodeBlock { .. } => {
            if line.trim_start().starts_with("```") { HighlightContext::Normal }
            else { ctx }
        }
        HighlightContext::InMathBlock => {
            if line.trim() == "$$" { HighlightContext::Normal }
            else { ctx }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::vec;
    use src_desktop_types::{AppConfig, MemoryVfs, VfsPath};
    use src_desktop_types::NoopPluginHost;
    use super::AppCore;

    fn make_core() -> AppCore<MemoryVfs, NoopPluginHost> {
        AppCore::new(MemoryVfs::default(), NoopPluginHost, AppConfig::default())
    }

    #[test]
    fn new_has_no_docs() {
        assert_eq!(make_core().doc_count(), 0);
    }

    #[test]
    fn open_bytes_returns_view_model() {
        let mut core = make_core();
        let (doc_id, vm) = core.open_bytes(VfsPath::from("a.n.md"), b"hello".to_vec());
        assert_eq!(core.doc_count(), 1);
        assert_eq!(vm.doc_id, doc_id);
    }

    #[test]
    fn close_doc_removes_it() {
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("a.n.md"), b"hi".to_vec());
        core.close_doc(doc_id);
        assert_eq!(core.doc_count(), 0);
    }

    #[test]
    fn open_file_reads_from_fs() {
        let mut vfs = MemoryVfs::default();
        vfs.write(&VfsPath::from("doc.n.md"), b"# Hello").unwrap();
        let mut core = AppCore::new(vfs, NoopPluginHost, AppConfig::default());
        let result = core.open_file(&VfsPath::from("doc.n.md"));
        assert!(result.is_ok());
        let (_, vm) = result.unwrap();
        assert!(vm.total_lines >= 1);
    }

    #[test]
    fn save_doc_writes_to_fs() {
        let mut core = make_core();
        let path = VfsPath::from("out.n.md");
        let (doc_id, _) = core.open_bytes(path.clone(), b"content".to_vec());
        assert!(core.save_doc(doc_id).is_ok());
        let bytes = core.fs.read(&path).unwrap();
        assert_eq!(bytes, b"content");
    }

    #[test]
    fn apply_key_char_updates_vm() {
        use src_desktop_types::{KeyCode, KeyEvent, Modifiers};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"".to_vec());
        let key = KeyEvent { key: KeyCode::Char('A'), mods: Modifiers::default(), text: None };
        let vm = core.apply_key(doc_id, &key);
        assert!(vm.is_some());
        assert!(vm.unwrap().dirty);
    }

    #[test]
    fn undo_redo_roundtrip() {
        use src_desktop_types::{KeyCode, KeyEvent, Modifiers};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"".to_vec());
        let key = KeyEvent { key: KeyCode::Char('X'), mods: Modifiers::default(), text: None };
        core.apply_key(doc_id, &key);
        // undo → empty buffer
        core.undo(doc_id);
        let vm_after_undo = core.apply_key(doc_id, &KeyEvent {
            key: KeyCode::End, mods: Modifiers::default(), text: None,
        });
        // redo → X back
        core.redo(doc_id);
    }

    #[test]
    fn render_full_returns_html() {
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"# Title".to_vec());
        let result = core.render_full(doc_id);
        assert!(result.is_some());
        let (html, _, _) = result.unwrap();
        assert!(html.contains("Title"));
    }

    #[test]
    fn execute_cmds_run_render_emits_render_complete() {
        use src_desktop_types::{AppCmd, AppEvent, PaneId};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"hi".to_vec());
        let cmds = vec![AppCmd::RunRender { pane_id: PaneId(1), doc_id }];
        let (events, shell) = core.execute_cmds(cmds);
        assert!(events.iter().any(|e| matches!(e, AppEvent::RenderComplete { .. })));
        assert!(shell.is_empty());
    }

    #[test]
    fn execute_cmds_passes_unknown_to_shell() {
        use src_desktop_types::{AppCmd, VfsPath};
        let mut core = make_core();
        let cmds = vec![AppCmd::ReadFile { path: VfsPath::from("x.n.md") }];
        let (events, shell) = core.execute_cmds(cmds);
        assert!(events.is_empty());
        assert_eq!(shell.len(), 1);
    }
}
```

- [ ] **Step 1.8: Run all tests**

```
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test -p src-desktop-core 2>&1 | grep "test result"
```

Expected: `test result: ok. 9 passed`

- [ ] **Step 1.9: Commit**

```bash
git -C /mnt/d/project/gloss/.worktrees/desktop-core add \
    ../../Cargo.toml \
    src-desktop-core/Cargo.toml \
    src-desktop-core/src/lib.rs \
    src-desktop-core/src/app_core.rs \
    src-desktop-core/src/renderer.rs
# If HighlightContext needed Clone, also add src-editor/src/highlighter.rs
git -C /mnt/d/project/gloss/.worktrees/desktop-core commit -m "feat(src-desktop-core): AppCore struct, document management, edit ops, rendering, execute_cmds"
```

---

## Task 2: `PluginAwareHtmlRenderer` — full implementation

**Files:**
- Modify: `src-desktop-core/src/renderer.rs`

**Reference:** Read `/mnt/d/project/gloss/src-plugin/src/renderer.rs` before implementing. The new version replaces `GlossPluginHost` with the `Ph: PluginHost` type parameter but the event interception logic is identical. Also read `src-plugin/src/host.rs` to understand how `render_card_output` is implemented there.

- [ ] **Step 2.1: Write failing test**

Add to `src-desktop-core/src/app_core.rs` tests (or a separate test module in renderer.rs):

```rust
#[test]
fn renderer_intercepts_code_block_via_noop() {
    use src_desktop_types::{AppConfig, MemoryVfs, NoopPluginHost, VfsPath};
    use crate::AppCore;
    // NoopPluginHost returns None for all hooks → fallback HTML used
    let mut core: AppCore<MemoryVfs, NoopPluginHost> =
        AppCore::new(MemoryVfs::default(), NoopPluginHost, AppConfig::default());
    let md = b"```rust\nfn main() {}\n```";
    let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), md.to_vec());
    let result = core.render_full(doc_id);
    assert!(result.is_some());
    let (html, _, _) = result.unwrap();
    // fallback renders a <pre><code> block
    assert!(html.contains("main"));
}
```

Run: `cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test -p src-desktop-core renderer_intercepts 2>&1 | tail -5`

Expected: test passes already (stub renderer falls through to empty, so `main` won't be in html) — actually this should fail. Good.

- [ ] **Step 2.2: Implement `src-desktop-core/src/renderer.rs`**

Read `/mnt/d/project/gloss/src-plugin/src/renderer.rs` for the interception pattern. Implement the full renderer:

```rust
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use src_core::{Event, HtmlRenderer, Tag, escape_html};
use src_desktop_types::PluginHost;
use src_plugin_types::{
    CardLinkOutput, PluginFrontMatterField,
};

pub struct PluginAwareHtmlRenderer<'a, Ph: PluginHost> {
    pub host: &'a mut Ph,
}

impl<'a, Ph: PluginHost> PluginAwareHtmlRenderer<'a, Ph> {
    pub fn new(host: &'a mut Ph) -> Self { Self { host } }

    pub fn render<'ev>(
        &mut self,
        events: &[Event<'ev>],
        out: &mut String,
        source: &str,
        _markdown: &str,
    ) {
        let mut renderer = HtmlRenderer::new(false);
        let mut i = 0;

        while i < events.len() {
            match &events[i] {
                // ── FrontMatter hook ─────────────────────────────────────
                Event::FrontMatter(fields) => {
                    let pfields: Vec<PluginFrontMatterField> = fields.iter().map(|f| {
                        PluginFrontMatterField { key: f.key.into(), raw: f.raw.into() }
                    }).collect();
                    if let Some(html) = self.host.run_front_matter(&pfields, source) {
                        out.push_str(&html);
                    } else {
                        // fallback: let HtmlRenderer handle it
                        // Event::FrontMatter doesn't clone, so we skip — default renders nothing visible
                    }
                    i += 1;
                }

                // ── CardLink hook ─────────────────────────────────────────
                Event::CardLink(url) => {
                    let url_s = url.to_string();
                    if let Some(card_out) = self.host.run_card_link(&url_s) {
                        out.push_str(&render_card_output(&url_s, card_out));
                    } else {
                        // fallback: render as plain link
                        out.push_str(&alloc::format!(
                            "<a href=\"{}\" class=\"nm-card-link\">{}</a>",
                            escape_html(&url_s), escape_html(&url_s)
                        ));
                    }
                    i += 1;
                }

                // ── CodeBlock hook ────────────────────────────────────────
                Event::Start(Tag::CodeBlock(lang, filename)) => {
                    let lang_s     = lang.to_string();
                    let filename_s = filename.to_string();
                    let mut code_text = String::new();
                    i += 1;
                    while i < events.len() {
                        match &events[i] {
                            Event::Text(t)                    => { code_text.push_str(t); i += 1; }
                            Event::End(Tag::CodeBlock(..)) => { i += 1; break; }
                            _                                 => { i += 1; }
                        }
                    }
                    if let Some(html) = self.host.run_code_highlight(&lang_s, &code_text, &filename_s) {
                        out.push_str(&html);
                    } else {
                        // fallback: plain <pre><code>
                        out.push_str("<pre><code");
                        if !lang_s.is_empty() {
                            out.push_str(&alloc::format!(" class=\"language-{}\"", escape_html(&lang_s)));
                        }
                        out.push('>');
                        out.push_str(&escape_html(&code_text));
                        out.push_str("</code></pre>\n");
                    }
                }

                // ── Default: pass to HtmlRenderer ─────────────────────────
                _ => {
                    // HtmlRenderer::feed takes Event by value.
                    // We must reconstruct events from the slice element.
                    // For non-intercepted events, use the renderer's feed() by
                    // re-parsing a minimal representation.
                    // Since Event<'_> is a borrowed type without Clone in src-core,
                    // we use a workaround: render the text content directly for Text events,
                    // and use a separate renderer instance for structural events.
                    //
                    // NOTE: The proper solution is to have src-core Event derive Clone,
                    // or expose an event replay API. For now, we feed what we can.
                    // This is sufficient for correctness in Plan 4 scope.
                    //
                    // Implementation: use push_html on a sub-parser for the remaining events
                    // is not possible here. Instead, handle the most common cases manually
                    // and delegate the rest to a raw feed.
                    //
                    // Since we can't clone Event, collect contiguous non-intercepted events
                    // and render them by feeding each one individually.
                    // We need src-core to export Event with Clone, or use unsafe.
                    // For now: re-render the whole document and extract what we need
                    // is not practical. Use the simplest correct approach:
                    // push the event index range to a secondary renderer.
                    //
                    // SIMPLEST CORRECT APPROACH: collect all non-intercepted events into
                    // a new parser invocation over the same markdown text. Since we have
                    // access to the original markdown in the `_markdown` param, re-parse it.
                    // This is O(n²) but correct.
                    //
                    // For Plan 4, implement this by re-parsing in render() for the fallback path.
                    // The subagent should look at src-plugin/src/renderer.rs to see how this
                    // is handled there (it likely uses a different approach).
                    renderer.feed_cloned(&events[i], out);
                    i += 1;
                }
            }
        }
        renderer.finish(out, 0);
    }
}

fn render_card_output(url: &str, out: CardLinkOutput) -> String {
    if let Some(html) = out.html { return html; }
    let title = out.title.as_deref().unwrap_or(url);
    let desc  = out.description.as_deref().unwrap_or("");
    alloc::format!(
        "<div class=\"nm-card\"><a href=\"{url}\">{title}</a><p>{desc}</p></div>",
        url   = escape_html(url),
        title = escape_html(title),
        desc  = escape_html(desc),
    )
}
```

**IMPORTANT:** The `renderer.feed_cloned(...)` call above won't compile because `HtmlRenderer` doesn't have `feed_cloned`. The correct implementation requires one of:
a) `Event<'a>` to derive `Clone` in src-core (preferred — check src-core/src/parser.rs to see if Clone is already derived)
b) Or use `push_html` on a sub-slice

**The subagent must:**
1. Check if `Event` in `src-core/src/parser.rs` derives `Clone`
2. If not, add `#[derive(Clone)]` to `Event`, `Tag`, and `FrontMatterField` in `src-core/src/parser.rs`
3. Replace `renderer.feed_cloned(...)` with `renderer.feed(events[i].clone(), out)`
4. Run `cargo test --workspace` to ensure no regressions in src-core

- [ ] **Step 2.3: Run renderer test**

```
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test -p src-desktop-core renderer_intercepts 2>&1 | tail -5
```

Expected: `test result: ok. 1 passed`

- [ ] **Step 2.4: Run full suite**

```
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test -p src-desktop-core 2>&1 | grep "test result"
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo test --workspace 2>&1 | grep "FAILED" | head -5
```

Expected: all tests pass, no regressions.

- [ ] **Step 2.5: Verify wasm32**

```
cd /mnt/d/project/gloss/.worktrees/desktop-core && cargo build -p src-desktop-core --target wasm32-unknown-unknown 2>&1 | grep "^error"
```

Expected: 0 errors.

- [ ] **Step 2.6: Commit**

```bash
git -C /mnt/d/project/gloss/.worktrees/desktop-core add \
    src-desktop-core/src/renderer.rs \
    src-core/src/parser.rs
git -C /mnt/d/project/gloss/.worktrees/desktop-core commit -m "feat(src-desktop-core): PluginAwareHtmlRenderer with hook interception"
```
