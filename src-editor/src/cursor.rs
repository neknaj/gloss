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

    /// Recompute `line` and `col_byte` from `byte_pos` and `buf`.
    pub fn sync_line_col(&mut self, buf: &mut GapBuffer) {
        self.line     = buf.byte_to_line(self.byte_pos);
        let line_start = buf.line_to_byte(self.line);
        self.col_byte = self.byte_pos - line_start;
    }

    /// Visual column of the cursor (each CJK char = 2 columns).
    pub fn visual_col(&self, buf: &mut GapBuffer) -> u32 {
        let line_start = buf.line_to_byte(self.line);
        let slice = buf.slice(line_start..self.byte_pos);
        slice.chars().map(GapBuffer::char_visual_width).sum()
    }

    pub fn move_right(&mut self, buf: &mut GapBuffer) {
        if self.byte_pos >= buf.len_bytes() { return; }
        // Skip over a full UTF-8 char.
        let ch_len = char_len_at(buf, self.byte_pos);
        self.byte_pos += ch_len;
        self.sync_line_col(buf);
        self.preferred_visual_col = self.visual_col(buf);
    }

    pub fn move_left(&mut self, buf: &mut GapBuffer) {
        if self.byte_pos == 0 { return; }
        self.byte_pos = prev_char_boundary(buf, self.byte_pos);
        self.sync_line_col(buf);
        self.preferred_visual_col = self.visual_col(buf);
    }

    pub fn move_down(&mut self, buf: &mut GapBuffer) {
        let next_line = self.line + 1;
        if next_line >= buf.line_count() { return; }
        let next_start = buf.line_to_byte(next_line);
        let next_end   = if next_line + 1 < buf.line_count() {
            buf.line_to_byte(next_line + 1) - 1
        } else {
            buf.len_bytes()
        };
        self.byte_pos = visual_col_to_byte(buf, next_start, next_end, self.preferred_visual_col);
        self.line = next_line;
        self.col_byte = self.byte_pos - next_start;
    }

    pub fn move_up(&mut self, buf: &mut GapBuffer) {
        if self.line == 0 { return; }
        let prev_line  = self.line - 1;
        let prev_start = buf.line_to_byte(prev_line);
        let prev_end   = buf.line_to_byte(self.line) - 1; // before \n
        self.byte_pos = visual_col_to_byte(buf, prev_start, prev_end, self.preferred_visual_col);
        self.line = prev_line;
        self.col_byte = self.byte_pos - prev_start;
    }

    pub fn move_line_start(&mut self, buf: &mut GapBuffer) {
        self.byte_pos = buf.line_to_byte(self.line);
        self.col_byte = 0;
        self.preferred_visual_col = 0;
    }

    pub fn move_line_end(&mut self, buf: &mut GapBuffer) {
        let next_line_start = if self.line + 1 < buf.line_count() {
            buf.line_to_byte(self.line + 1)
        } else {
            buf.len_bytes() + 1 // sentinel
        };
        // Position before the newline (or at end of buffer).
        self.byte_pos = if next_line_start > 0 && next_line_start <= buf.len_bytes() {
            next_line_start - 1
        } else {
            buf.len_bytes()
        };
        self.sync_line_col(buf);
        self.preferred_visual_col = self.visual_col(buf);
    }

    pub fn move_word_right(&mut self, buf: &mut GapBuffer) {
        let len = buf.len_bytes();
        if self.byte_pos >= len { return; }
        // Skip whitespace, then skip word chars of same type.
        let pos = self.byte_pos;
        let s = buf.as_str();
        let chars: alloc::vec::Vec<(usize, char)> = s.char_indices().collect();
        let start_idx = chars.partition_point(|&(i, _)| i < pos);
        if start_idx >= chars.len() { return; }
        let start_kind = char_kind(chars[start_idx].1);
        let mut i = start_idx;
        // Move past chars of the same kind.
        while i < chars.len() && char_kind(chars[i].1) == start_kind {
            i += 1;
        }
        // If we started on whitespace, also skip the word after it.
        if start_kind == CharKind::Space && i < chars.len() {
            let word_kind = char_kind(chars[i].1);
            while i < chars.len() && char_kind(chars[i].1) == word_kind {
                i += 1;
            }
        }
        self.byte_pos = if i < chars.len() { chars[i].0 } else { len };
        self.sync_line_col(buf);
        self.preferred_visual_col = self.visual_col(buf);
    }

    pub fn move_word_left(&mut self, buf: &mut GapBuffer) {
        if self.byte_pos == 0 { return; }
        let s = buf.as_str();
        let chars: alloc::vec::Vec<(usize, char)> = s.char_indices().collect();
        // Find index just before current position.
        let end_idx = chars.partition_point(|&(i, _)| i < self.byte_pos);
        if end_idx == 0 { return; }
        let mut i = end_idx - 1;
        let end_kind = char_kind(chars[i].1);
        while i > 0 && char_kind(chars[i - 1].1) == end_kind { i -= 1; }
        // If we backed over whitespace, back over the word before it too.
        if end_kind == CharKind::Space && i > 0 {
            let word_kind = char_kind(chars[i - 1].1);
            while i > 0 && char_kind(chars[i - 1].1) == word_kind { i -= 1; }
        }
        self.byte_pos = chars[i].0;
        self.sync_line_col(buf);
        self.preferred_visual_col = self.visual_col(buf);
    }
}

impl Default for Cursor {
    fn default() -> Self { Self::new() }
}

// ── Helpers ────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum CharKind { Space, Ascii, Cjk, Other }

fn char_kind(c: char) -> CharKind {
    if c.is_ascii_whitespace() { return CharKind::Space; }
    if c.is_ascii()            { return CharKind::Ascii; }
    let u = c as u32;
    if (0x4E00..=0x9FFF).contains(&u) || (0x3040..=0x30FF).contains(&u)
        || (0xAC00..=0xD7FF).contains(&u) { CharKind::Cjk }
    else { CharKind::Other }
}

/// Byte length of the UTF-8 char starting at `byte_pos` in `buf`.
fn char_len_at(buf: &mut GapBuffer, byte_pos: usize) -> usize {
    let s = buf.as_str();
    s[byte_pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

/// Previous UTF-8 char boundary before `byte_pos`.
fn prev_char_boundary(buf: &mut GapBuffer, byte_pos: usize) -> usize {
    let s = buf.as_str();
    let mut pos = byte_pos - 1;
    while pos > 0 && !s.is_char_boundary(pos) { pos -= 1; }
    pos
}

/// Find the byte offset in [line_start..line_end] that best matches `target_vcol`.
fn visual_col_to_byte(buf: &mut GapBuffer, line_start: usize, line_end: usize, target: u32) -> usize {
    let slice = buf.slice(line_start..line_end);
    let mut vcol = 0u32;
    for (i, c) in slice.char_indices() {
        let w = GapBuffer::char_visual_width(c);
        if vcol + w > target { return line_start + i; }
        vcol += w;
        if vcol == target { return line_start + i + c.len_utf8(); }
    }
    line_start + slice.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn buf(s: &str) -> GapBuffer { GapBuffer::from_str(s) }

    #[test] fn move_right_advances_byte_pos() {
        let mut buf = buf("hello");
        let mut cur = Cursor::new();
        cur.move_right(&mut buf);
        assert_eq!(cur.byte_pos, 1);
    }
    #[test] fn move_left_at_start_is_noop() {
        let mut buf = buf("hello");
        let mut cur = Cursor::new();
        cur.move_left(&mut buf);
        assert_eq!(cur.byte_pos, 0);
    }
    #[test] fn move_right_cjk_skips_multibyte() {
        let mut buf = buf("日本");  // 3 bytes each
        let mut cur = Cursor::new();
        cur.move_right(&mut buf);
        assert_eq!(cur.byte_pos, 3);
    }
    #[test] fn move_down_updates_line() {
        let mut buf = buf("hello\nworld");
        let mut cur = Cursor::new();
        cur.move_down(&mut buf);
        assert_eq!(cur.line, 1);
    }
    #[test] fn move_up_at_first_line_is_noop() {
        let mut buf = buf("hello\nworld");
        let mut cur = Cursor::new();
        cur.move_up(&mut buf);
        assert_eq!(cur.line, 0);
    }
    #[test] fn move_line_end_goes_to_newline_before() {
        let mut buf = buf("hello\nworld");
        let mut cur = Cursor::new();
        cur.move_line_end(&mut buf);
        assert_eq!(cur.byte_pos, 5);
    }
    #[test] fn move_line_start_returns_to_zero() {
        let mut buf = buf("hello");
        let mut cur = Cursor::new();
        cur.byte_pos = 3; cur.col_byte = 3;
        cur.move_line_start(&mut buf);
        assert_eq!(cur.byte_pos, 0);
    }
    #[test] fn move_word_right_stops_at_boundary() {
        let mut buf = buf("hello world");
        let mut cur = Cursor::new();
        cur.move_word_right(&mut buf);
        assert_eq!(cur.byte_pos, 5);  // end of "hello"
    }
    #[test] fn visual_col_ascii() {
        let mut buf = buf("hello");
        let cur = Cursor { byte_pos: 3, line: 0, col_byte: 3, preferred_visual_col: 0 };
        assert_eq!(cur.visual_col(&mut buf), 3);
    }
    #[test] fn visual_col_after_cjk() {
        let mut buf = buf("日x");  // 日 = 3 bytes, width 2
        let cur = Cursor { byte_pos: 3, line: 0, col_byte: 3, preferred_visual_col: 0 };
        assert_eq!(cur.visual_col(&mut buf), 2);
    }
}
