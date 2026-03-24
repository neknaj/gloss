use alloc::string::String;
use alloc::vec::Vec;
use src_plugin_types::PluginWarning;
use crate::path::{VfsPath, DirEntry};
use crate::traits::ImeEvent;
use crate::primitives::{PaneId, DocId, KeyEvent, MouseEvent, SplitDirection, Rect};
use crate::config::AppConfig;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Ime(ImeEvent),
    Resize { width: u32, height: u32 },

    FileLoaded  { path: VfsPath, content: Vec<u8> },
    FileSaved   { path: VfsPath },
    FileError   { path: VfsPath, error: String },
    DirLoaded   { path: VfsPath, entries: Vec<DirEntry> },

    RenderComplete { pane_id: PaneId, html: String, warnings: Vec<PluginWarning> },
    LintComplete   { doc_id: DocId, warnings: Vec<PluginWarning> },

    ConfigLoaded(AppConfig),
    ClipboardText(String),

    TabSelected        { pane_id: PaneId, tab_index: usize },
    PaneSplitRequested { pane_id: PaneId, direction: SplitDirection },

    PreviewLinkClicked { pane_id: PaneId, url: String, new_tab: bool },
    PreviewScrolled    { pane_id: PaneId, offset_y: f32 },

    Quit,
}

#[derive(Clone, Debug)]
pub enum AppCmd {
    // AppCore handles:
    WriteFile      { path: VfsPath, content: Vec<u8> },
    ListDir        { path: VfsPath },
    RunRender      { pane_id: PaneId, doc_id: DocId },
    RunLint        { doc_id: DocId },
    ScheduleRender { pane_id: PaneId, delay_ms: u32 },

    // Shell handles:
    ReadFile       { path: VfsPath },
    LoadConfig     { path: VfsPath },
    CopyToClipboard { text: String },
    PasteRequest,
    SetImeCursorArea { rect: Rect },
    ShowOpenFileDialog,
    ShowSaveFileDialog { suggested: Option<VfsPath> },
    OpenUrl        { url: String },
    Quit,
}
