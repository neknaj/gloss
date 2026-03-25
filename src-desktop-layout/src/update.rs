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
            m.file_tree.entries = entries.into_iter().map(|e| src_desktop_types::FileTreeEntry {
                name: e.name,
                path: e.path,
                is_dir: e.is_dir,
                depth: 0,
                expanded: false,
            }).collect();
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
