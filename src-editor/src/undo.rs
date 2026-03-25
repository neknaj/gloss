use alloc::string::String;
use alloc::vec::Vec;
use crate::gap_buffer::GapBuffer;

#[derive(Clone)]
pub enum EditOp {
    Insert { byte_pos: usize, text: String },
    Delete { byte_pos: usize, deleted: String },
}

struct UndoGroup {
    ops:           Vec<EditOp>,
    cursor_before: usize,
    cursor_after:  usize,
}

pub struct UndoHistory {
    undo_stack: Vec<UndoGroup>,
    redo_stack: Vec<UndoGroup>,
    group_open: bool,
    pending:    Vec<EditOp>,
    cursor_at_group_start: usize,
}

impl UndoHistory {
    pub fn new() -> Self {
        UndoHistory {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            group_open: false,
            pending:    Vec::new(),
            cursor_at_group_start: 0,
        }
    }

    pub fn begin_group(&mut self, cursor_byte_pos: usize) {
        if !self.group_open {
            self.group_open = true;
            self.cursor_at_group_start = cursor_byte_pos;
            self.pending.clear();
        }
    }

    pub fn end_group(&mut self, cursor_byte_pos: usize) {
        if self.group_open {
            self.group_open = false;
            if !self.pending.is_empty() {
                self.undo_stack.push(UndoGroup {
                    ops:           self.pending.drain(..).collect(),
                    cursor_before: self.cursor_at_group_start,
                    cursor_after:  cursor_byte_pos,
                });
                self.redo_stack.clear();
            }
        }
    }

    /// Record a single operation (must be called inside begin_group/end_group,
    /// or as a standalone atomic if no group is open).
    pub fn record(&mut self, op: EditOp, cursor_before: usize, cursor_after: usize) {
        if self.group_open {
            self.pending.push(op);
        } else {
            self.undo_stack.push(UndoGroup {
                ops: alloc::vec![op],
                cursor_before,
                cursor_after,
            });
            self.redo_stack.clear();
        }
    }

    /// Undo the last group. Returns the cursor position to restore to.
    pub fn undo(&mut self, buf: &mut GapBuffer) -> Option<usize> {
        let group = self.undo_stack.pop()?;
        for op in group.ops.iter().rev() {
            match op {
                EditOp::Insert { byte_pos, text } => {
                    buf.delete(*byte_pos..*byte_pos + text.len());
                }
                EditOp::Delete { byte_pos, deleted } => {
                    buf.insert(*byte_pos, deleted);
                }
            }
        }
        let cursor = group.cursor_before;
        self.redo_stack.push(group);
        Some(cursor)
    }

    /// Redo the last undone group. Returns cursor position after.
    pub fn redo(&mut self, buf: &mut GapBuffer) -> Option<usize> {
        let group = self.redo_stack.pop()?;
        for op in &group.ops {
            match op {
                EditOp::Insert { byte_pos, text } => {
                    buf.insert(*byte_pos, text);
                }
                EditOp::Delete { byte_pos, deleted } => {
                    buf.delete(*byte_pos..*byte_pos + deleted.len());
                }
            }
        }
        let cursor = group.cursor_after;
        self.undo_stack.push(group);
        Some(cursor)
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
}

impl Default for UndoHistory {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn undo_single_insert() {
        let mut buf = GapBuffer::from_str("hello");
        let mut h = UndoHistory::new();
        buf.insert(5, " world");
        h.record(EditOp::Insert { byte_pos: 5, text: " world".into() }, 5, 11);
        h.undo(&mut buf);
        assert_eq!(buf.as_str(), "hello");
    }
    #[test] fn redo_after_undo() {
        let mut buf = GapBuffer::from_str("hello");
        let mut h = UndoHistory::new();
        buf.insert(5, " world");
        h.record(EditOp::Insert { byte_pos: 5, text: " world".into() }, 5, 11);
        h.undo(&mut buf);
        h.redo(&mut buf);
        assert_eq!(buf.as_str(), "hello world");
    }
    #[test] fn undo_delete_restores_text() {
        let mut buf = GapBuffer::from_str("hello world");
        let mut h = UndoHistory::new();
        buf.delete(5..11);
        h.record(EditOp::Delete { byte_pos: 5, deleted: " world".into() }, 11, 5);
        h.undo(&mut buf);
        assert_eq!(buf.as_str(), "hello world");
    }
    #[test] fn undo_group_replays_in_reverse() {
        let mut buf = GapBuffer::from_str("hello");
        let mut h = UndoHistory::new();
        h.begin_group(5);
        buf.insert(5, " world");
        h.record(EditOp::Insert { byte_pos: 5, text: " world".into() }, 5, 11);
        buf.insert(11, "!");
        h.record(EditOp::Insert { byte_pos: 11, text: "!".into() }, 11, 12);
        h.end_group(12);
        h.undo(&mut buf);
        assert_eq!(buf.as_str(), "hello");
    }
    #[test] fn redo_cleared_after_new_edit() {
        let mut buf = GapBuffer::from_str("hello");
        let mut h = UndoHistory::new();
        buf.insert(5, " world");
        h.record(EditOp::Insert { byte_pos: 5, text: " world".into() }, 5, 11);
        h.undo(&mut buf);
        assert!(h.can_redo());
        buf.insert(5, "!");
        h.record(EditOp::Insert { byte_pos: 5, text: "!".into() }, 5, 6);
        assert!(!h.can_redo());
    }
    #[test] fn undo_returns_cursor_before() {
        let mut buf = GapBuffer::from_str("hello");
        let mut h = UndoHistory::new();
        buf.insert(5, " world");
        h.record(EditOp::Insert { byte_pos: 5, text: " world".into() }, 5, 11);
        let pos = h.undo(&mut buf).unwrap();
        assert_eq!(pos, 5);
    }
}
