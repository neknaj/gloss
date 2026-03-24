use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use crate::path::VfsPath;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocId(pub u64);

impl Default for DocId { fn default() -> Self { DocId(0) } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ScrollOffset { pub x: f32, pub y: f32 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyEvent {
    pub key:  KeyCode,
    pub mods: Modifiers,
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool, pub shift: bool, pub alt: bool, pub meta: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Enter, Backspace, Delete, Escape, Tab,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
    F(u8),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MouseEvent {
    pub kind: MouseKind, pub x: f32, pub y: f32,
    pub button: MouseButton, pub mods: Modifiers,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MouseKind {
    Press, Release, Move,
    Scroll { delta_x: f32, delta_y: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MouseButton { Left, Right, Middle }

/// Byte positions into GapBuffer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Selection { pub anchor: usize, pub active: usize }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorDisplay { pub line: u32, pub visual_col: u32, pub blink: bool }

impl Default for CursorDisplay {
    fn default() -> Self { CursorDisplay { line: 0, visual_col: 0, blink: true } }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectionDisplay {
    pub ranges: Vec<(u32, u32, u32, u32)>, // (line_start, col_start, line_end, col_end)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreeditDraw {
    pub text: String,
    pub underline_range: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorDraw { pub x: f32, pub y: f32, pub height: f32 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectionDraw { pub rects: Vec<Rect> }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorLine { pub line_no: u32, pub spans: Vec<TextSpan> }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text: String, pub color: u32, pub bold: bool, pub italic: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabInfo { pub doc_id: DocId, pub title: String, pub dirty: bool }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileTreeEntry {
    pub name: String, pub path: VfsPath,
    pub is_dir: bool, pub depth: u32, pub expanded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String, pub path: VfsPath, pub hooks: Vec<String>, pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WarningInfo {
    pub code: String, pub message: String,
    pub line: Option<u32>, // display adapter from PluginWarning.line (u32)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DialogKind {
    OpenFile,
    SaveFile { suggested: Option<VfsPath> },
    Confirm  { message: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HtmlPatch { pub block_id: u64, pub html: String }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SplitDirection { Horizontal, Vertical }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaneKind { Editor, Preview, FileTree, PluginManager }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FocusTarget { Pane(PaneId), StatusBar }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocMeta { pub path: VfsPath, pub title: String, pub dirty: bool }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tab { pub doc_id: DocId, pub title: String, pub dirty: bool }

#[derive(Clone, Debug, Default)]
pub struct FileTreeState {
    pub root_path: Option<VfsPath>,
    pub entries: Vec<FileTreeEntry>,
    pub expanded: Vec<VfsPath>,
}

#[derive(Clone, Debug, Default)]
pub struct PluginManagerState { pub plugins: Vec<PluginInfo> }

#[derive(Clone, Debug, Default)]
pub struct StatusState {
    pub left: String, pub right: String, pub warning_count: u32,
}
