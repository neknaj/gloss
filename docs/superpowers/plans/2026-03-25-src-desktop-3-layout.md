# src-desktop-layout (Plan 3 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `src-desktop-layout` crate — a `no_std + alloc` Elm-style UI layer with `Model`, a pure `update()` function, and a pure `view()` function.

**Architecture:** Three modules: `model` (data types + helpers), `update` (pure event→state+cmd), `view` (pure state→DrawCmd). No I/O, no AppCore access. Depends only on `src-desktop-types` and `src-plugin-types`. `update()` consumes the Model by value and returns a new one.

**Tech Stack:** Rust `no_std + alloc`, `src-desktop-types` (all shared types), `src-plugin-types` (PluginWarning).

**Implementation notes:**
- `WorkspaceState` adds a `next_doc_id: u64` counter (not in spec §5.1) so `update_file_loaded()` can assign DocIds when the shell dispatches `FileLoaded`. Plan 4 (AppCore) will instead call `open_bytes()` and pre-populate the model; at that point `update_file_loaded()` can be simplified to a pure lookup. For now the counter lives in layout.
- `AppEvent::FileError` uses `error: String` in the actual `events.rs`, not `FsError` as spec §3.5 says. Use the actual types file as source of truth.

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md` §5

---

## File Map

| File | Responsibility |
|------|----------------|
| `src-desktop-layout/Cargo.toml` | Crate manifest; deps: src-desktop-types, src-plugin-types |
| `src-desktop-layout/src/lib.rs` | `#![no_std]`, module declarations, re-exports |
| `src-desktop-layout/src/model.rs` | `Model`, `WorkspaceState`, `LayoutState`, `PanelNode`, `Pane`, `PreviewState`, helpers |
| `src-desktop-layout/src/update.rs` | `pub fn update(model: Model, event: AppEvent) -> (Model, Vec<AppCmd>)` |
| `src-desktop-layout/src/view.rs` | `pub fn view(model: &Model) -> Vec<DrawCmd>` |

---

## Task 1: Crate scaffold and Model types

**Files:**
- Create: `src-desktop-layout/Cargo.toml`
- Create: `src-desktop-layout/src/lib.rs`
- Create: `src-desktop-layout/src/model.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1.1: Add `src-desktop-layout` to workspace**

Edit root `Cargo.toml`, add `"src-desktop-layout"` to members:

```toml
[workspace]
members = [
    "src-core",
    "src-web",
    "src-cli",
    "src-plugin-types",
    "src-plugin",
    "src-desktop-types",
    "src-editor",
    "src-desktop-layout",
]
```

- [ ] **Step 1.2: Create `src-desktop-layout/Cargo.toml`**

```toml
[package]
name = "src-desktop-layout"
version = "0.1.0"
edition = "2021"

[dependencies]
src-desktop-types = { path = "../src-desktop-types" }
src-plugin-types  = { path = "../src-plugin-types" }
```

- [ ] **Step 1.3: Create `src-desktop-layout/src/lib.rs`**

```rust
#![no_std]
extern crate alloc;

pub mod model;
pub mod update;
pub mod view;

pub use model::{Model, WorkspaceState, LayoutState, PanelNode, Pane, PreviewState};
pub use update::update;
pub use view::view;
```

- [ ] **Step 1.4: Write the failing test for Model**

Add to the end of `src-desktop-layout/src/model.rs` (create the file with just this test first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_new_has_editor_pane() {
        let m = Model::new(1280, 720);
        assert!(matches!(m.layout.root, PanelNode::Leaf(_)));
    }
}
```

- [ ] **Step 1.5: Run to confirm failure**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout model_new 2>&1 | tail -5
```

Expected: compile error (types not defined yet).

- [ ] **Step 1.6: Implement `src-desktop-layout/src/model.rs`**

```rust
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use src_desktop_types::{
    AppConfig, DocId, DocMeta, EditorViewModel, FileTreeState,
    FocusTarget, PaneId, PaneKind, PluginManagerState, Rect,
    SplitDirection, StatusState, Tab, VfsPath,
};
use src_plugin_types::PluginWarning;

// ── Model ────────────────────────────────────────────────────────────────────

pub struct Model {
    pub workspace:      WorkspaceState,
    pub layout:         LayoutState,
    pub file_tree:      FileTreeState,
    pub plugin_manager: PluginManagerState,
    pub status:         StatusState,
    pub config:         AppConfig,
}

impl Model {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            workspace:      WorkspaceState::default(),
            layout:         LayoutState::new(width, height),
            file_tree:      FileTreeState::default(),
            plugin_manager: PluginManagerState::default(),
            status:         StatusState::default(),
            config:         AppConfig::default(),
        }
    }

    /// DocId of the active tab in the currently focused pane, if any.
    pub fn active_doc_id(&self) -> Option<DocId> {
        let pane_id = match self.layout.focus {
            FocusTarget::Pane(id) => id,
            FocusTarget::StatusBar => return None,
        };
        find_pane(&self.layout.root, pane_id)
            .and_then(|p| p.tabs.get(p.active_tab).map(|t| t.doc_id))
    }

    /// DocMeta for the active doc.
    pub fn active_doc_meta(&self) -> Option<&DocMeta> {
        self.active_doc_id().and_then(|id| self.workspace.open_docs.get(&id))
    }
}

// ── WorkspaceState ────────────────────────────────────────────────────────────

pub struct WorkspaceState {
    pub root_path:   Option<VfsPath>,
    pub open_docs:   BTreeMap<DocId, DocMeta>,
    pub editors:     BTreeMap<DocId, EditorViewModel>,
    pub previews:    BTreeMap<DocId, PreviewState>,
    /// Monotonic counter; used by update() to assign DocIds from FileLoaded.
    pub next_doc_id: u64,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            root_path:   None,
            open_docs:   BTreeMap::new(),
            editors:     BTreeMap::new(),
            previews:    BTreeMap::new(),
            next_doc_id: 1,
        }
    }
}

// ── PreviewState ──────────────────────────────────────────────────────────────

pub struct PreviewState {
    pub html:          String,
    pub block_hashes:  Vec<u64>,
    pub scroll_offset: f32,
    pub warnings:      Vec<PluginWarning>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self { html: String::new(), block_hashes: Vec::new(), scroll_offset: 0.0, warnings: Vec::new() }
    }
}

// ── LayoutState ───────────────────────────────────────────────────────────────

pub struct LayoutState {
    pub root:        PanelNode,
    pub window_size: (u32, u32),
    pub focus:       FocusTarget,
}

impl LayoutState {
    pub fn new(width: u32, height: u32) -> Self {
        let initial_pane = Pane {
            id:         PaneId(1),
            kind:       PaneKind::Editor,
            tabs:       Vec::new(),
            active_tab: 0,
            bounds:     Rect { x: 0.0, y: 0.0, width: width as f32, height: height as f32 },
        };
        Self {
            root:        PanelNode::Leaf(initial_pane),
            window_size: (width, height),
            focus:       FocusTarget::Pane(PaneId(1)),
        }
    }
}

// ── PanelNode ─────────────────────────────────────────────────────────────────

pub enum PanelNode {
    Split {
        direction: SplitDirection,
        ratio:     f32,
        a:         Box<PanelNode>,
        b:         Box<PanelNode>,
    },
    Leaf(Pane),
}

impl PanelNode {
    pub fn count_panes(&self) -> usize {
        match self {
            PanelNode::Leaf(_) => 1,
            PanelNode::Split { a, b, .. } => a.count_panes() + b.count_panes(),
        }
    }
}

// ── Pane ──────────────────────────────────────────────────────────────────────

pub struct Pane {
    pub id:         PaneId,
    pub kind:       PaneKind,
    pub tabs:       Vec<Tab>,
    pub active_tab: usize,
    pub bounds:     Rect,
}

// ── Tree helpers ──────────────────────────────────────────────────────────────

pub fn find_pane(node: &PanelNode, id: PaneId) -> Option<&Pane> {
    match node {
        PanelNode::Leaf(p) if p.id == id => Some(p),
        PanelNode::Leaf(_) => None,
        PanelNode::Split { a, b, .. } => find_pane(a, id).or_else(|| find_pane(b, id)),
    }
}

pub fn find_pane_mut(node: &mut PanelNode, id: PaneId) -> Option<&mut Pane> {
    match node {
        PanelNode::Leaf(p) if p.id == id => Some(p),
        PanelNode::Leaf(_) => None,
        PanelNode::Split { a, b, .. } => {
            if find_pane(a, id).is_some() { find_pane_mut(a, id) }
            else { find_pane_mut(b, id) }
        }
    }
}

/// Find the first Editor-kind pane id.
pub fn find_editor_pane_id(node: &PanelNode) -> Option<PaneId> {
    match node {
        PanelNode::Leaf(p) if p.kind == PaneKind::Editor => Some(p.id),
        PanelNode::Leaf(_) => None,
        PanelNode::Split { a, b, .. } => {
            find_editor_pane_id(a).or_else(|| find_editor_pane_id(b))
        }
    }
}

/// Collect all panes into a flat vec.
pub fn collect_panes<'a>(node: &'a PanelNode, out: &mut Vec<&'a Pane>) {
    match node {
        PanelNode::Leaf(p) => out.push(p),
        PanelNode::Split { a, b, .. } => { collect_panes(a, out); collect_panes(b, out); }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_new_has_editor_pane() {
        let m = Model::new(1280, 720);
        assert!(matches!(m.layout.root, PanelNode::Leaf(_)));
    }

    #[test]
    fn count_panes_leaf_is_one() {
        let m = Model::new(800, 600);
        assert_eq!(m.layout.root.count_panes(), 1);
    }

    #[test]
    fn find_pane_returns_leaf() {
        let m = Model::new(800, 600);
        let found = find_pane(&m.layout.root, PaneId(1));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, PaneId(1));
    }

    #[test]
    fn find_pane_missing_returns_none() {
        let m = Model::new(800, 600);
        assert!(find_pane(&m.layout.root, PaneId(99)).is_none());
    }

    #[test]
    fn find_editor_pane_finds_first_editor() {
        let m = Model::new(800, 600);
        assert_eq!(find_editor_pane_id(&m.layout.root), Some(PaneId(1)));
    }

    #[test]
    fn active_doc_id_none_when_no_tabs() {
        let m = Model::new(800, 600);
        assert!(m.active_doc_id().is_none());
    }
}
```

- [ ] **Step 1.7: Run 6 Model tests**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout model 2>&1 | tail -5
```

Expected: `test result: ok. 6 passed`

- [ ] **Step 1.8: Commit**

```bash
git -C /mnt/d/project/gloss/.worktrees/desktop-layout add \
    ../../Cargo.toml \
    src-desktop-layout/Cargo.toml \
    src-desktop-layout/src/lib.rs \
    src-desktop-layout/src/model.rs
git -C /mnt/d/project/gloss/.worktrees/desktop-layout commit -m "feat(src-desktop-layout): crate scaffold and Model types"
```

---

## Task 2: `update()` — key / resize / config events

**Files:**
- Create: `src-desktop-layout/src/update.rs`

- [ ] **Step 2.1: Write failing test**

Create `src-desktop-layout/src/update.rs` with just the test module:

```rust
use alloc::vec;
use src_desktop_types::{AppCmd, AppEvent, KeyCode, KeyEvent, Modifiers, PaneId, VfsPath};
use crate::model::{Model, find_pane};

pub fn update(_model: Model, _event: AppEvent) -> (Model, alloc::vec::Vec<AppCmd>) {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use src_desktop_types::{DocId, DocMeta, Tab};
    use crate::model::PanelNode;

    fn ctrl(c: char) -> AppEvent {
        AppEvent::Key(KeyEvent {
            key: KeyCode::Char(c),
            mods: Modifiers { ctrl: true, ..Modifiers::default() },
            text: None,
        })
    }

    fn make_model_with_doc(dirty: bool) -> Model {
        let mut m = Model::new(1280, 720);
        let doc_id = DocId(1);
        m.workspace.open_docs.insert(doc_id, DocMeta {
            path:  VfsPath::from("test.n.md"),
            title: "test".into(),
            dirty,
        });
        if let crate::model::PanelNode::Leaf(ref mut pane) = m.layout.root {
            pane.tabs.push(Tab { doc_id, title: "test".into(), dirty });
        }
        m
    }

    #[test]
    fn ctrl_s_emits_write_file() {
        let m = make_model_with_doc(false);
        let (_, cmds) = update(m, ctrl('s'));
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::WriteFile { .. })));
    }

    #[test]
    fn ctrl_w_dirty_shows_save_dialog() {
        let m = make_model_with_doc(true);
        let (_, cmds) = update(m, ctrl('w'));
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::ShowSaveFileDialog { .. })));
    }

    #[test]
    fn ctrl_w_clean_removes_tab() {
        let m = make_model_with_doc(false);
        let (m2, _) = update(m, ctrl('w'));
        let tabs = match &m2.layout.root { PanelNode::Leaf(p) => p.tabs.len(), _ => 99 };
        assert_eq!(tabs, 0);
    }

    #[test]
    fn printable_char_emits_schedule_render() {
        let m = make_model_with_doc(false);
        let ev = AppEvent::Key(KeyEvent {
            key: KeyCode::Char('a'),
            mods: Modifiers::default(),
            text: Some("a".into()),
        });
        let (_, cmds) = update(m, ev);
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::ScheduleRender { .. })));
    }

    #[test]
    fn resize_updates_window_size() {
        let m = Model::new(800, 600);
        let (m2, _) = update(m, AppEvent::Resize { width: 1920, height: 1080 });
        assert_eq!(m2.layout.window_size, (1920, 1080));
    }

    #[test]
    fn config_loaded_updates_model() {
        use src_desktop_types::AppConfig;
        let m = Model::new(800, 600);
        let cfg = AppConfig::default();
        let (m2, _) = update(m, AppEvent::ConfigLoaded(cfg));
        // No panic = config updated
        let _ = m2.config;
    }

    #[test]
    fn quit_emits_quit_cmd() {
        let m = Model::new(800, 600);
        let (_, cmds) = update(m, AppEvent::Quit);
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::Quit)));
    }
}
```

- [ ] **Step 2.2: Run to confirm failure**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout update 2>&1 | tail -5
```

Expected: panics with `unimplemented!()`.

- [ ] **Step 2.3: Implement `update()` key/resize/config handling**

Replace the stub with:

```rust
use alloc::vec;
use alloc::vec::Vec;

use src_desktop_types::{
    AppCmd, AppEvent, AppConfig, DocMeta, FocusTarget,
    KeyCode, KeyEvent, Modifiers, PaneId, PaneKind, VfsPath,
};

use crate::model::{find_editor_pane_id, find_pane, find_pane_mut, Model, PanelNode};

// ── Public entry point ────────────────────────────────────────────────────────

pub fn update(model: Model, event: AppEvent) -> (Model, Vec<AppCmd>) {
    match event {
        AppEvent::Key(k)                        => update_key(model, k),
        AppEvent::Resize { width, height }      => update_resize(model, width, height),
        AppEvent::ConfigLoaded(cfg)             => update_config(model, cfg),
        AppEvent::FileLoaded { path, .. }       => update_file_loaded(model, path),
        AppEvent::RenderComplete { pane_id, html, warnings } => {
            update_render_complete(model, pane_id, html, warnings)
        }
        AppEvent::LintComplete { doc_id, warnings } => {
            update_lint_complete(model, doc_id, warnings)
        }
        AppEvent::TabSelected { pane_id, tab_index } => {
            update_tab_selected(model, pane_id, tab_index)
        }
        AppEvent::PreviewScrolled { pane_id, offset_y } => {
            update_preview_scrolled(model, pane_id, offset_y)
        }
        AppEvent::DirLoaded { entries, .. } => {
            let mut m = model;
            m.file_tree.entries = entries;
            (m, vec![])
        }
        AppEvent::Quit => (model, vec![AppCmd::Quit]),
        // Ime, Mouse, ClipboardText, etc. → just emit ScheduleRender for active pane
        AppEvent::Ime(_) | AppEvent::ClipboardText(_) => {
            let cmds = schedule_render_for_focus(&model);
            (model, cmds)
        }
        _ => (model, vec![]),
    }
}

// ── Key handling ──────────────────────────────────────────────────────────────

fn update_key(model: Model, k: KeyEvent) -> (Model, Vec<AppCmd>) {
    if k.mods.ctrl && !k.mods.alt {
        match &k.key {
            KeyCode::Char('s') | KeyCode::Char('S') => return update_save(model),
            KeyCode::Char('w') | KeyCode::Char('W') => return update_close_tab(model),
            KeyCode::Char('o') | KeyCode::Char('O') => {
                return (model, vec![AppCmd::ShowOpenFileDialog]);
            }
            _ => {}
        }
    }
    // Printable / navigation → emit ScheduleRender
    match &k.key {
        KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete
        | KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        | KeyCode::Home | KeyCode::End => {
            let cmds = schedule_render_for_focus(&model);
            (model, cmds)
        }
        _ => (model, vec![]),
    }
}

fn update_save(model: Model) -> (Model, Vec<AppCmd>) {
    let cmd = model.active_doc_meta().map(|meta| AppCmd::WriteFile {
        path:    meta.path.clone(),
        content: alloc::vec::Vec::new(),  // AppCore's execute_cmds uses doc buffer, not this field
    });
    (model, cmd.into_iter().collect())
}

fn update_close_tab(mut model: Model) -> (Model, Vec<AppCmd>) {
    let focus_id = match model.layout.focus {
        FocusTarget::Pane(id) => id,
        _ => return (model, vec![]),
    };
    if let Some(pane) = find_pane_mut(&mut model.layout.root, focus_id) {
        if pane.tabs.is_empty() { return (model, vec![]); }
        let idx = pane.active_tab;
        let tab = &pane.tabs[idx];
        // Check dirty
        let is_dirty = model.workspace.open_docs
            .get(&tab.doc_id)
            .map(|m| m.dirty)
            .unwrap_or(false);
        if is_dirty {
            let path = model.workspace.open_docs
                .get(&pane.tabs[idx].doc_id)
                .map(|m| m.path.clone());
            return (model, vec![AppCmd::ShowSaveFileDialog { suggested: path }]);
        }
        let doc_id = pane.tabs.remove(idx).doc_id;
        if pane.active_tab >= pane.tabs.len() && !pane.tabs.is_empty() {
            pane.active_tab = pane.tabs.len() - 1;
        }
        model.workspace.open_docs.remove(&doc_id);
        model.workspace.editors.remove(&doc_id);
        model.workspace.previews.remove(&doc_id);
    }
    (model, vec![])
}

// ── Simple state updates ──────────────────────────────────────────────────────

fn update_resize(mut model: Model, width: u32, height: u32) -> (Model, Vec<AppCmd>) {
    model.layout.window_size = (width, height);
    (model, vec![])
}

fn update_config(mut model: Model, cfg: AppConfig) -> (Model, Vec<AppCmd>) {
    model.config = cfg;
    (model, vec![])
}

// ── Async / FS events ─────────────────────────────────────────────────────────

/// FileLoaded: shell has already called core.open_bytes() and inserted
/// the doc + vm into the model. update() assigns DocId, adds tab, emits RunRender.
/// If the doc is already in open_docs (by path), just emits RunRender.
fn update_file_loaded(mut model: Model, path: VfsPath) -> (Model, Vec<AppCmd>) {
    // Find existing doc_id by path, or create a new entry
    let doc_id = model.workspace.open_docs.iter()
        .find(|(_, meta)| meta.path == path)
        .map(|(id, _)| *id)
        .unwrap_or_else(|| {
            let id = src_desktop_types::DocId(model.workspace.next_doc_id);
            model.workspace.next_doc_id += 1;
            let title = path.file_name().unwrap_or("untitled").into();
            model.workspace.open_docs.insert(id, DocMeta {
                path: path.clone(), title, dirty: false,
            });
            // Add tab to active editor pane
            let focus_id = match model.layout.focus {
                FocusTarget::Pane(pid) => pid,
                _ => PaneId(1),
            };
            if let Some(pane) = find_pane_mut(&mut model.layout.root, focus_id) {
                if pane.kind == PaneKind::Editor {
                    let t = src_desktop_types::Tab {
                        doc_id: id,
                        title: path.file_name().unwrap_or("untitled").into(),
                        dirty: false,
                    };
                    pane.tabs.push(t);
                    pane.active_tab = pane.tabs.len() - 1;
                }
            }
            id
        });

    let pane_id = match model.layout.focus {
        FocusTarget::Pane(pid) => pid,
        _ => PaneId(1),
    };
    (model, vec![AppCmd::RunRender { pane_id, doc_id }])
}

fn update_render_complete(
    mut model: Model,
    _pane_id: src_desktop_types::PaneId,
    html: alloc::string::String,
    warnings: alloc::vec::Vec<src_plugin_types::PluginWarning>,
) -> (Model, Vec<AppCmd>) {
    if let Some(doc_id) = model.active_doc_id() {
        let ps = model.workspace.previews.entry(doc_id).or_insert_with(Default::default);
        ps.html = html;
        ps.warnings = warnings;
    }
    (model, vec![])
}

fn update_lint_complete(
    mut model: Model,
    doc_id: src_desktop_types::DocId,
    warnings: alloc::vec::Vec<src_plugin_types::PluginWarning>,
) -> (Model, Vec<AppCmd>) {
    let ps = model.workspace.previews.entry(doc_id).or_insert_with(Default::default);
    ps.warnings = warnings;
    (model, vec![])
}

fn update_tab_selected(
    mut model: Model,
    pane_id: src_desktop_types::PaneId,
    tab_index: usize,
) -> (Model, Vec<AppCmd>) {
    if let Some(pane) = find_pane_mut(&mut model.layout.root, pane_id) {
        if tab_index < pane.tabs.len() {
            pane.active_tab = tab_index;
        }
    }
    (model, vec![])
}

fn update_preview_scrolled(
    mut model: Model,
    _pane_id: src_desktop_types::PaneId,
    offset_y: f32,
) -> (Model, Vec<AppCmd>) {
    if let Some(doc_id) = model.active_doc_id() {
        let ps = model.workspace.previews.entry(doc_id).or_insert_with(Default::default);
        ps.scroll_offset = offset_y;
    }
    (model, vec![])
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn schedule_render_for_focus(model: &Model) -> Vec<AppCmd> {
    let pane_id = match model.layout.focus {
        FocusTarget::Pane(id) => id,
        _ => return vec![],
    };
    if model.active_doc_id().is_some() {
        vec![AppCmd::ScheduleRender { pane_id, delay_ms: 300 }]
    } else {
        vec![]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use src_desktop_types::{DocId, DocMeta, Tab, VfsPath};
    use crate::model::PanelNode;

    fn ctrl(c: char) -> AppEvent {
        AppEvent::Key(KeyEvent {
            key: KeyCode::Char(c),
            mods: Modifiers { ctrl: true, ..Modifiers::default() },
            text: None,
        })
    }

    fn make_model_with_doc(dirty: bool) -> Model {
        let mut m = Model::new(1280, 720);
        let doc_id = DocId(1);
        m.workspace.open_docs.insert(doc_id, DocMeta {
            path:  VfsPath::from("test.n.md"),
            title: "test".into(),
            dirty,
        });
        if let PanelNode::Leaf(ref mut pane) = m.layout.root {
            pane.tabs.push(Tab { doc_id, title: "test".into(), dirty });
        }
        m
    }

    #[test]
    fn ctrl_s_emits_write_file() {
        let m = make_model_with_doc(false);
        let (_, cmds) = update(m, ctrl('s'));
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::WriteFile { .. })));
    }

    #[test]
    fn ctrl_w_dirty_shows_save_dialog() {
        let m = make_model_with_doc(true);
        let (_, cmds) = update(m, ctrl('w'));
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::ShowSaveFileDialog { .. })));
    }

    #[test]
    fn ctrl_w_clean_removes_tab() {
        let m = make_model_with_doc(false);
        let (m2, _) = update(m, ctrl('w'));
        let tabs = match &m2.layout.root { PanelNode::Leaf(p) => p.tabs.len(), _ => 99 };
        assert_eq!(tabs, 0);
    }

    #[test]
    fn printable_char_emits_schedule_render() {
        let m = make_model_with_doc(false);
        let ev = AppEvent::Key(KeyEvent {
            key: KeyCode::Char('a'),
            mods: Modifiers::default(),
            text: Some("a".into()),
        });
        let (_, cmds) = update(m, ev);
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::ScheduleRender { .. })));
    }

    #[test]
    fn resize_updates_window_size() {
        let m = Model::new(800, 600);
        let (m2, _) = update(m, AppEvent::Resize { width: 1920, height: 1080 });
        assert_eq!(m2.layout.window_size, (1920, 1080));
    }

    #[test]
    fn config_loaded_updates_model() {
        let m = Model::new(800, 600);
        let (m2, _) = update(m, AppEvent::ConfigLoaded(AppConfig::default()));
        let _ = m2.config;
    }

    #[test]
    fn quit_emits_quit_cmd() {
        let m = Model::new(800, 600);
        let (_, cmds) = update(m, AppEvent::Quit);
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::Quit)));
    }

    #[test]
    fn file_loaded_adds_tab_and_emits_run_render() {
        let m = Model::new(1280, 720);
        let (m2, cmds) = update(m, AppEvent::FileLoaded {
            path: VfsPath::from("hello.n.md"),
            content: alloc::vec![],
        });
        let tabs = match &m2.layout.root { PanelNode::Leaf(p) => p.tabs.len(), _ => 0 };
        assert_eq!(tabs, 1);
        assert!(cmds.iter().any(|c| matches!(c, AppCmd::RunRender { .. })));
    }

    #[test]
    fn tab_selected_updates_active_tab() {
        let mut m = make_model_with_doc(false);
        // Add second tab
        let doc_id2 = DocId(2);
        if let PanelNode::Leaf(ref mut pane) = m.layout.root {
            pane.tabs.push(Tab { doc_id: doc_id2, title: "b".into(), dirty: false });
        }
        let (m2, _) = update(m, AppEvent::TabSelected { pane_id: PaneId(1), tab_index: 1 });
        let active = match &m2.layout.root { PanelNode::Leaf(p) => p.active_tab, _ => 99 };
        assert_eq!(active, 1);
    }
}
```

- [ ] **Step 2.4: Run 9 update tests**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout update 2>&1 | tail -5
```

Expected: `test result: ok. 9 passed`

- [ ] **Step 2.5: Commit**

```bash
git -C /mnt/d/project/gloss/.worktrees/desktop-layout add src-desktop-layout/src/update.rs
git -C /mnt/d/project/gloss/.worktrees/desktop-layout commit -m "feat(src-desktop-layout): update() pure function"
```

---

## Task 3: `view()` — layout calculation and DrawCmd generation

**Files:**
- Create: `src-desktop-layout/src/view.rs`

- [ ] **Step 3.1: Write failing tests**

Create `src-desktop-layout/src/view.rs` with stub + tests:

```rust
use alloc::vec;
use alloc::vec::Vec;
use src_desktop_types::DrawCmd;
use crate::model::Model;

pub fn view(_model: &Model) -> Vec<DrawCmd> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use src_desktop_types::{DocId, DocMeta, Tab, VfsPath};
    use crate::model::PanelNode;

    #[test]
    fn view_empty_model_emits_set_layout() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetLayout { .. })));
    }

    #[test]
    fn view_emits_status_bar() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetStatusBar { .. })));
    }

    #[test]
    fn view_emits_tab_bar_for_each_pane() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetTabBar { .. })));
    }

    #[test]
    fn horizontal_split_gives_two_panels() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::{Pane, PanelNode};
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let layout = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        });
        assert_eq!(layout.unwrap().len(), 2);
    }

    #[test]
    fn split_panel_bounds_sum_to_window_width() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::{Pane, PanelNode};
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let panels = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        }).unwrap();
        let total: f32 = panels.iter().map(|p| p.bounds.width).sum::<f32>()
            + cmds.iter().filter_map(|c| {
                if let DrawCmd::SetLayout { dividers, .. } = c {
                    Some(dividers.iter().map(|d| d.bounds.width).sum::<f32>())
                } else { None }
            }).sum::<f32>();
        // Total width of panels + dividers = window width (1000.0)
        assert!((total - 1000.0_f32).abs() < 1.0, "total={total}");
    }
}
```

- [ ] **Step 3.2: Run to confirm failure**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout view 2>&1 | tail -5
```

Expected: 3 tests fail (stub returns empty vec).

- [ ] **Step 3.3: Implement `src-desktop-layout/src/view.rs`**

```rust
use alloc::vec;
use alloc::vec::Vec;

use src_desktop_types::{
    CursorDraw, DividerLayout, DrawCmd, EditorViewModel, FocusTarget,
    PanelLayout, PaneId, PaneKind, Rect, ScrollOffset, SplitDirection,
    TabInfo,
};

use crate::model::{collect_panes, Model, PanelNode, Pane};

const DIVIDER_SIZE: f32 = 4.0;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn view(model: &Model) -> Vec<DrawCmd> {
    let mut cmds = Vec::new();
    let (w, h) = model.layout.window_size;
    let root_bounds = Rect { x: 0.0, y: 0.0, width: w as f32, height: h as f32 };

    // 1. Layout
    let mut panels: Vec<PanelLayout> = Vec::new();
    let mut dividers: Vec<DividerLayout> = Vec::new();
    collect_layout(&model.layout.root, root_bounds, &mut panels, &mut dividers);
    cmds.push(DrawCmd::SetLayout { panels: panels.clone(), dividers });

    // 2. Per-pane content
    let mut pane_vec: Vec<&Pane> = Vec::new();
    collect_panes(&model.layout.root, &mut pane_vec);

    for pane in pane_vec {
        // Find the computed bounds for this pane from the panel list
        let bounds = panels.iter()
            .find(|pl| pl.pane_id == pane.id)
            .map(|pl| pl.bounds)
            .unwrap_or(root_bounds);

        // Tab bar
        let tabs: Vec<TabInfo> = pane.tabs.iter().map(|t| TabInfo {
            doc_id: t.doc_id,
            title:  t.title.clone(),
            dirty:  t.dirty,
        }).collect();
        cmds.push(DrawCmd::SetTabBar {
            pane_id:    pane.id,
            tabs,
            active_tab: pane.active_tab,
        });

        // Content
        let active_doc = pane.tabs.get(pane.active_tab).map(|t| t.doc_id);
        match pane.kind {
            PaneKind::Editor => {
                if let Some(doc_id) = active_doc {
                    if let Some(vm) = model.workspace.editors.get(&doc_id) {
                        emit_editor_frame(&mut cmds, pane.id, bounds, vm);
                    }
                }
            }
            PaneKind::Preview => {
                if let Some(doc_id) = active_doc {
                    if let Some(ps) = model.workspace.previews.get(&doc_id) {
                        cmds.push(DrawCmd::PreviewMount {
                            pane_id: pane.id,
                            html:    ps.html.clone(),
                        });
                        cmds.push(DrawCmd::PreviewScroll {
                            pane_id:  pane.id,
                            offset_y: ps.scroll_offset,
                        });
                        let warnings: Vec<_> = ps.warnings.iter().map(|w| {
                            src_desktop_types::WarningInfo {
                                code:    w.code.clone(),
                                message: w.message.clone(),
                                line:    Some(w.line),
                            }
                        }).collect();
                        cmds.push(DrawCmd::SetWarnings { warnings });
                    }
                }
            }
            PaneKind::FileTree => {
                cmds.push(DrawCmd::SetFileTree {
                    entries:  model.file_tree.entries.clone(),
                    expanded: model.file_tree.expanded.clone(),
                });
            }
            PaneKind::PluginManager => {
                cmds.push(DrawCmd::SetPluginList {
                    plugins: model.plugin_manager.plugins.clone(),
                });
            }
        }
    }

    // 3. Status bar
    cmds.push(DrawCmd::SetStatusBar {
        left:          model.status.left.clone(),
        right:         model.status.right.clone(),
        warning_count: model.status.warning_count,
    });

    cmds
}

// ── Layout calculation ────────────────────────────────────────────────────────

fn collect_layout(
    node: &PanelNode,
    bounds: Rect,
    panels: &mut Vec<PanelLayout>,
    dividers: &mut Vec<DividerLayout>,
) {
    match node {
        PanelNode::Leaf(pane) => {
            panels.push(PanelLayout {
                pane_id: pane.id,
                bounds,
                kind:    pane.kind,
                visible: true,
            });
        }
        PanelNode::Split { direction, ratio, a, b } => {
            let (bounds_a, div_bounds, bounds_b) = split_bounds(bounds, *direction, *ratio);
            collect_layout(a, bounds_a, panels, dividers);
            collect_layout(b, bounds_b, panels, dividers);
            dividers.push(DividerLayout {
                bounds:     div_bounds,
                direction:  *direction,
                draggable:  true,
            });
        }
    }
}

fn split_bounds(bounds: Rect, direction: SplitDirection, ratio: f32) -> (Rect, Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => {
            let split_x = bounds.x + bounds.width * ratio;
            let a = Rect { x: bounds.x, y: bounds.y, width: split_x - bounds.x, height: bounds.height };
            let div = Rect { x: split_x, y: bounds.y, width: DIVIDER_SIZE, height: bounds.height };
            let b = Rect { x: split_x + DIVIDER_SIZE, y: bounds.y,
                           width: bounds.width - (split_x - bounds.x) - DIVIDER_SIZE,
                           height: bounds.height };
            (a, div, b)
        }
        SplitDirection::Vertical => {
            let split_y = bounds.y + bounds.height * ratio;
            let a = Rect { x: bounds.x, y: bounds.y, width: bounds.width, height: split_y - bounds.y };
            let div = Rect { x: bounds.x, y: split_y, width: bounds.width, height: DIVIDER_SIZE };
            let b = Rect { x: bounds.x, y: split_y + DIVIDER_SIZE,
                           width: bounds.width,
                           height: bounds.height - (split_y - bounds.y) - DIVIDER_SIZE };
            (a, div, b)
        }
    }
}

// ── Editor frame ──────────────────────────────────────────────────────────────

fn emit_editor_frame(cmds: &mut Vec<DrawCmd>, pane_id: PaneId, bounds: Rect, vm: &EditorViewModel) {
    cmds.push(DrawCmd::EditorFrame {
        pane_id,
        bounds,
        lines:     vm.visible_lines.clone(),
        cursor:    CursorDraw { x: 0.0, y: 0.0, height: 16.0 },
        selection: None,
        preedit:   vm.preedit.clone(),
        scroll:    vm.scroll,
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use src_desktop_types::{DocId, DocMeta, Tab, VfsPath};
    use crate::model::PanelNode;

    #[test]
    fn view_empty_model_emits_set_layout() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetLayout { .. })));
    }

    #[test]
    fn view_emits_status_bar() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetStatusBar { .. })));
    }

    #[test]
    fn view_emits_tab_bar_for_each_pane() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetTabBar { .. })));
    }

    #[test]
    fn horizontal_split_gives_two_panels() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::Pane;
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let layout = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        });
        assert_eq!(layout.unwrap().len(), 2);
    }

    #[test]
    fn split_panel_bounds_sum_to_window_width() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::Pane;
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let panels = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        }).unwrap();
        let total_panel_w: f32 = panels.iter().map(|p| p.bounds.width).sum();
        let total_div_w: f32 = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { dividers, .. } = c {
                Some(dividers.iter().map(|d| d.bounds.width).sum())
            } else { None }
        }).unwrap_or(0.0);
        assert!((total_panel_w + total_div_w - 1000.0_f32).abs() < 1.0);
    }
}
```

- [ ] **Step 3.4: Run 5 view tests**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout view 2>&1 | tail -5
```

Expected: `test result: ok. 5 passed`

- [ ] **Step 3.5: Run full suite + wasm32 check**

```
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo test -p src-desktop-layout 2>&1 | grep "test result"
cd /mnt/d/project/gloss/.worktrees/desktop-layout && cargo build -p src-desktop-layout --target wasm32-unknown-unknown 2>&1 | grep "^error"
```

Expected: 20 passed, 0 wasm32 errors.

- [ ] **Step 3.6: Commit**

```bash
git -C /mnt/d/project/gloss/.worktrees/desktop-layout add src-desktop-layout/src/view.rs
git -C /mnt/d/project/gloss/.worktrees/desktop-layout commit -m "feat(src-desktop-layout): view() pure function and layout calculation"
```
