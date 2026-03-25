use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
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
