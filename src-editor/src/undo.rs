use alloc::vec::Vec;

pub enum EditOp {
    Insert,
    Delete,
}

pub struct UndoHistory {
    pub ops: Vec<EditOp>,
}

impl UndoHistory {
    pub fn new() -> Self {
        UndoHistory { ops: Vec::new() }
    }
}

impl Default for UndoHistory {
    fn default() -> Self { Self::new() }
}
