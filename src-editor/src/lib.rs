#![no_std]
extern crate alloc;

pub mod gap_buffer;
pub mod cursor;
pub mod ime;
pub mod undo;
pub mod highlighter;
pub mod editor_state;

pub use gap_buffer::GapBuffer;
pub use cursor::Cursor;
pub use ime::{ImeState, Preedit};
pub use undo::{UndoHistory, EditOp};
pub use highlighter::{Highlighter, HighlightContext};
pub use editor_state::EditorState;
