use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use src_plugin_types::{CardLinkOutput, PluginWarning, PluginEvent, PluginFrontMatterField};
use crate::path::{VfsPath, DirEntry, FsError};

pub trait FileSystem {
    fn read(&self,       path: &VfsPath) -> Result<Vec<u8>, FsError>;
    fn write(&mut self,  path: &VfsPath, data: &[u8]) -> Result<(), FsError>;
    fn list_dir(&self,   path: &VfsPath) -> Result<Vec<DirEntry>, FsError>;
    fn exists(&self,     path: &VfsPath) -> bool;
    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError>;
    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError>;
    /// Rename/move a file. Only files are supported; renaming directories returns NotFound.
    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError>;
    fn is_dir(&self,     path: &VfsPath) -> bool;
}

/// Plugin host abstraction. AppCore calls src_plugin_types::to_plugin_events()
/// before calling run_lint_rule — the trait always receives already-converted slices.
pub trait PluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str) -> Option<String>;
    fn run_card_link(&mut self, url: &str) -> Option<CardLinkOutput>;
    fn run_lint_rule(&mut self, src: &str, md: &str,
        existing: &[PluginWarning], events: &[PluginEvent]) -> Vec<PluginWarning>;
    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str) -> Option<String>;
}

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// IME event source. Shell polls this and converts to AppEvent::Ime.
/// In the no_std types layer so Tauri/WASM/test shells share one interface.
pub trait ImeSource {
    fn poll_event(&mut self) -> Option<ImeEvent>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImeEvent {
    Start,
    Update { preedit: String, cursor: Option<(usize, usize)> },
    Commit { text: String },
    Cancel,
}
