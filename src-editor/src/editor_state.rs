use src_desktop_types::{Selection, ScrollOffset};
use crate::gap_buffer::GapBuffer;
use crate::cursor::Cursor;
use crate::ime::ImeState;
use crate::undo::UndoHistory;

pub struct EditorState {
    pub buffer:    GapBuffer,
    pub cursor:    Cursor,
    pub selection: Option<Selection>,
    pub ime:       ImeState,
    pub undo:      UndoHistory,
    pub scroll:    ScrollOffset,
    pub version:   u32,
}

impl EditorState {
    pub fn new() -> Self {
        EditorState {
            buffer:    GapBuffer::new(),
            cursor:    Cursor::new(),
            selection: None,
            ime:       ImeState::new(),
            undo:      UndoHistory::new(),
            scroll:    ScrollOffset::default(),
            version:   0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let mut state = Self::new();
        state.buffer = GapBuffer::from_str(s);
        state
    }

    /// Insert text at the cursor, record in undo, bump version.
    pub fn insert_at_cursor(&mut self, text: &str) {
        let pos = self.cursor.byte_pos;
        let len = text.len();
        self.buffer.insert(pos, text);
        self.undo.record(
            crate::undo::EditOp::Insert { byte_pos: pos, text: text.into() },
            pos, pos + len,
        );
        self.cursor.byte_pos = pos + len;
        self.cursor.sync_line_col(&mut self.buffer);
        self.version += 1;
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor.byte_pos == 0 { return; }
        let end = self.cursor.byte_pos;
        let buf_str = self.buffer.as_str(); // owned String
        let start = prev_char_boundary(&buf_str, end);
        // buf_str is owned, so no borrow conflict here
        let deleted = self.buffer.delete(start..end);
        self.undo.record(
            crate::undo::EditOp::Delete { byte_pos: start, deleted },
            end, start,
        );
        self.cursor.byte_pos = start;
        self.cursor.sync_line_col(&mut self.buffer);
        self.version += 1;
    }
}

impl Default for EditorState {
    fn default() -> Self { Self::new() }
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos - 1;
    while p > 0 && !s.is_char_boundary(p) { p -= 1; }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn insert_at_cursor_updates_buffer() {
        let mut state = EditorState::from_str("hello");
        state.cursor.byte_pos = 5;
        state.insert_at_cursor(" world");
        assert_eq!(state.buffer.as_str(), "hello world");
    }
    #[test] fn insert_bumps_version() {
        let mut state = EditorState::new();
        assert_eq!(state.version, 0);
        state.insert_at_cursor("x");
        assert_eq!(state.version, 1);
    }
    #[test] fn backspace_removes_char() {
        let mut state = EditorState::from_str("hello");
        state.cursor.byte_pos = 5;
        state.backspace();
        assert_eq!(state.buffer.as_str(), "hell");
    }
    #[test] fn backspace_at_start_is_noop() {
        let mut state = EditorState::from_str("hello");
        state.backspace();
        assert_eq!(state.buffer.as_str(), "hello");
    }
    #[test] fn undo_reverts_insert() {
        let mut state = EditorState::from_str("hello");
        state.cursor.byte_pos = 5;
        state.insert_at_cursor(" world");
        state.undo.undo(&mut state.buffer);
        assert_eq!(state.buffer.as_str(), "hello");
    }
}
