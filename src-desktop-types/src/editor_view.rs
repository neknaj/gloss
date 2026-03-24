use alloc::vec::Vec;
use crate::primitives::{DocId, EditorLine, CursorDisplay, SelectionDisplay, PreeditDraw, ScrollOffset};

/// Snapshot of editor display state. Computed from EditorState by AppCore,
/// stored in Model.workspace.editors, read by view().
#[derive(Clone, Debug, Default)]
pub struct EditorViewModel {
    pub doc_id:        DocId,
    pub visible_lines: Vec<EditorLine>,
    pub total_lines:   u32,
    pub cursor:        CursorDisplay,
    pub selection:     Option<SelectionDisplay>,
    pub preedit:       Option<PreeditDraw>,
    pub scroll:        ScrollOffset,
    pub dirty:         bool,
}
