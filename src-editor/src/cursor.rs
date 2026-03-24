use crate::gap_buffer::GapBuffer;

pub struct Cursor {
    pub byte_pos:             usize,
    pub line:                 usize,
    pub col_byte:             usize,
    pub preferred_visual_col: u32,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor { byte_pos: 0, line: 0, col_byte: 0, preferred_visual_col: 0 }
    }
}

impl Default for Cursor {
    fn default() -> Self { Self::new() }
}
