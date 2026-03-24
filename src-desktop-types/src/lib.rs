#![no_std]
extern crate alloc;

pub mod path;
pub mod primitives;
pub mod config;
pub mod traits;
pub mod events;
pub mod draw;
pub mod memory_vfs;
pub mod editor_view;

#[cfg(any(test, feature = "test-utils", target_arch = "wasm32"))]
pub mod noop;

// Top-level re-exports
pub use path::{VfsPath, DirEntry, FsError};
pub use primitives::*;
pub use config::{LintRules, PluginEntrySpec, AppConfig};
pub use traits::{FileSystem, PluginHost, Clipboard, ImeSource, ImeEvent};
pub use events::{AppEvent, AppCmd};
pub use draw::{DrawCmd, PanelLayout, DividerLayout};
pub use memory_vfs::MemoryVfs;
pub use editor_view::EditorViewModel;
