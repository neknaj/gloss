use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use crate::path::VfsPath;
use crate::primitives::{
    PaneId, PaneKind, Rect, EditorLine, CursorDraw, SelectionDraw, PreeditDraw,
    ScrollOffset, TabInfo, HtmlPatch, FileTreeEntry, PluginInfo, WarningInfo,
    DialogKind, SplitDirection,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DrawCmd {
    SetLayout { panels: Vec<PanelLayout>, dividers: Vec<DividerLayout> },
    SetTabBar  { pane_id: PaneId, tabs: Vec<TabInfo>, active_tab: usize },

    EditorFrame {
        pane_id:   PaneId,
        bounds:    Rect,
        lines:     Vec<EditorLine>,
        cursor:    CursorDraw,
        selection: Option<SelectionDraw>,
        preedit:   Option<PreeditDraw>,
        scroll:    ScrollOffset,
    },

    PreviewMount  { pane_id: PaneId, html: String },
    PreviewPatch  { pane_id: PaneId, patches: Vec<HtmlPatch> },
    PreviewScroll { pane_id: PaneId, offset_y: f32 },

    SetFileTree   { entries: Vec<FileTreeEntry>, expanded: Vec<VfsPath> },
    SetPluginList { plugins: Vec<PluginInfo> },
    SetStatusBar  { left: String, right: String, warning_count: u32 },
    SetWarnings   { warnings: Vec<WarningInfo> },

    SetImeCursorArea { rect: Rect },
    ShowDialog       { kind: DialogKind },
    ShowTooltip      { x: f32, y: f32, text: String },
    HideTooltip,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PanelLayout {
    pub pane_id: PaneId, pub bounds: Rect, pub kind: PaneKind, pub visible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DividerLayout {
    pub bounds: Rect, pub direction: SplitDirection, pub draggable: bool,
}
