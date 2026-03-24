use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ops::Range;

/// UTF-8 gap buffer. The gap is a contiguous hole in `buf` between
/// `gap_start` and `gap_end`. Text before the gap and after it together
/// form the document contents.
pub struct GapBuffer {
    buf:       Vec<u8>,
    gap_start: usize,
    gap_end:   usize,
}

const INITIAL_GAP: usize = 64;

impl GapBuffer {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(INITIAL_GAP);
        buf.resize(INITIAL_GAP, 0);
        GapBuffer { buf, gap_start: 0, gap_end: INITIAL_GAP }
    }

    pub fn from_str(s: &str) -> Self {
        let mut gb = Self::new();
        gb.insert(0, s);
        gb
    }

    // ── Gap management ─────────────────────────────────────────────────────

    fn gap_len(&self) -> usize { self.gap_end - self.gap_start }

    /// Move the gap so `gap_start == pos`.
    fn move_gap(&mut self, pos: usize) {
        if pos == self.gap_start { return; }
        if pos < self.gap_start {
            let count = self.gap_start - pos;
            self.buf.copy_within(pos..self.gap_start, self.gap_end - count);
            self.gap_start = pos;
            self.gap_end -= count;
        } else {
            let count = pos - self.gap_start;
            let src_start = self.gap_end;
            self.buf.copy_within(src_start..src_start + count, self.gap_start);
            self.gap_start += count;
            self.gap_end += count;
        }
    }

    /// Ensure gap has at least `needed` bytes.
    fn ensure_gap(&mut self, needed: usize) {
        if self.gap_len() >= needed { return; }
        let extra = needed + INITIAL_GAP;
        let new_gap_end = self.gap_end + extra;
        // Shift content after gap forward.
        let tail_len = self.buf.len() - self.gap_end;
        let new_len = self.buf.len() + extra;
        self.buf.resize(new_len, 0);
        self.buf.copy_within(self.gap_end..self.gap_end + tail_len, new_gap_end);
        self.gap_end = new_gap_end;
    }

    // ── Content length ─────────────────────────────────────────────────────

    pub fn len_bytes(&self) -> usize { self.buf.len() - self.gap_len() }

    pub fn len_chars(&self) -> usize {
        // Count UTF-8 leading bytes (not continuation bytes 0x80..0xBF)
        let count_leading = |slice: &[u8]| slice.iter().filter(|&&b| b < 0x80 || b >= 0xC0).count();
        count_leading(&self.buf[..self.gap_start]) + count_leading(&self.buf[self.gap_end..])
    }

    pub fn is_empty(&self) -> bool { self.len_bytes() == 0 }

    // ── Public API ─────────────────────────────────────────────────────────

    pub fn insert(&mut self, byte_pos: usize, text: &str) {
        let bytes = text.as_bytes();
        self.ensure_gap(bytes.len());
        self.move_gap(byte_pos);
        self.buf[self.gap_start..self.gap_start + bytes.len()].copy_from_slice(bytes);
        self.gap_start += bytes.len();
    }

    pub fn delete(&mut self, byte_range: Range<usize>) -> String {
        let start = byte_range.start;
        let end   = byte_range.end;
        self.move_gap(start);
        let deleted = core::str::from_utf8(&self.buf[self.gap_end..self.gap_end + (end - start)])
            .unwrap_or("")
            .to_string();
        self.gap_end += end - start;
        deleted
    }

    /// Move the gap so the range is contiguous, then return a &str into it.
    /// Takes `&mut self` because it moves the gap.
    pub fn slice(&mut self, byte_range: Range<usize>) -> &str {
        // Move gap out of the requested range.
        if self.gap_start > byte_range.start && self.gap_start < byte_range.end {
            // Gap overlaps — move it past the end.
            self.move_gap(byte_range.end);
        }
        let (lo, hi) = self.logical_to_raw(byte_range.start, byte_range.end);
        core::str::from_utf8(&self.buf[lo..hi]).unwrap_or("")
    }

    /// Collect the full buffer as an owned String.
    pub fn as_str(&self) -> String {
        let mut s = String::with_capacity(self.len_bytes());
        s.push_str(core::str::from_utf8(&self.buf[..self.gap_start]).unwrap_or(""));
        s.push_str(core::str::from_utf8(&self.buf[self.gap_end..]).unwrap_or(""));
        s
    }

    // ── Line utilities ─────────────────────────────────────────────────────

    pub fn line_count(&self) -> usize {
        let count_nl = |s: &[u8]| s.iter().filter(|&&b| b == b'\n').count();
        1 + count_nl(&self.buf[..self.gap_start]) + count_nl(&self.buf[self.gap_end..])
    }

    /// Byte offset of the start of `line` (0-indexed).
    pub fn line_to_byte(&self, line: usize) -> usize {
        if line == 0 { return 0; }
        let mut seen = 0usize;
        let mut pos  = 0usize;
        for chunk in [&self.buf[..self.gap_start], &self.buf[self.gap_end..]] {
            for (i, &b) in chunk.iter().enumerate() {
                if b == b'\n' {
                    seen += 1;
                    if seen == line { return pos + i + 1; }
                }
            }
            pos += chunk.len();
        }
        self.len_bytes()
    }

    /// Which line (0-indexed) does `byte_pos` fall on?
    pub fn byte_to_line(&self, byte_pos: usize) -> usize {
        let mut nl = 0usize;
        let mut seen = 0usize;
        for chunk in [&self.buf[..self.gap_start], &self.buf[self.gap_end..]] {
            for &b in chunk {
                if seen >= byte_pos { return nl; }
                if b == b'\n' { nl += 1; }
                seen += 1;
            }
        }
        nl
    }

    // ── CJK visual width ──────────────────────────────────────────────────

    pub fn char_visual_width(c: char) -> u32 {
        // East Asian Wide / Fullwidth → 2 columns; everything else → 1.
        match c as u32 {
            0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33BF |
            0x33FF..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0xA4CF |
            0xA960..=0xA97F | 0xAC00..=0xD7FF | 0xF900..=0xFAFF |
            0xFE10..=0xFE1F | 0xFE30..=0xFE4F | 0xFF01..=0xFF60 |
            0xFFE0..=0xFFE6 | 0x1B000..=0x1B0FF | 0x1F004..=0x1F0CF |
            0x1F200..=0x1F2FF | 0x20000..=0x2FFFD | 0x30000..=0x3FFFD => 2,
            _ => 1,
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    /// Convert logical byte range to raw buffer indices (skipping the gap).
    fn logical_to_raw(&self, start: usize, end: usize) -> (usize, usize) {
        let raw_start = if start < self.gap_start { start } else { start + self.gap_len() };
        let raw_end   = if end   <= self.gap_start { end   } else { end   + self.gap_len() };
        (raw_start, raw_end)
    }
}

impl Default for GapBuffer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> GapBuffer { GapBuffer::from_str(s) }

    #[test] fn empty_buffer_has_zero_bytes() { assert_eq!(buf("").len_bytes(), 0); }
    #[test] fn insert_at_start() {
        let mut b = buf("world");
        b.insert(0, "hello ");
        assert_eq!(b.as_str(), "hello world");
    }
    #[test] fn insert_in_middle() {
        let mut b = buf("helo");
        b.insert(3, "l");
        assert_eq!(b.as_str(), "hello");
    }
    #[test] fn delete_range() {
        let mut b = buf("hello world");
        let deleted = b.delete(5..6);
        assert_eq!(deleted, " ");
        assert_eq!(b.as_str(), "helloworld");
    }
    #[test] fn line_count_empty() { assert_eq!(buf("").line_count(), 1); }
    #[test] fn line_count_two_lines() { assert_eq!(buf("a\nb").line_count(), 2); }
    #[test] fn line_to_byte_first_line() { assert_eq!(buf("hello\nworld").line_to_byte(0), 0); }
    #[test] fn line_to_byte_second_line() { assert_eq!(buf("hello\nworld").line_to_byte(1), 6); }
    #[test] fn byte_to_line() { assert_eq!(buf("hello\nworld").byte_to_line(7), 1); }
    #[test] fn slice_returns_substring() {
        let mut b = buf("hello world");
        assert_eq!(b.slice(6..11), "world");
    }
    #[test] fn len_chars_ascii() { assert_eq!(buf("abc").len_chars(), 3); }
    #[test] fn len_chars_cjk() { assert_eq!(buf("日本語").len_chars(), 3); }
    #[test] fn char_visual_width_ascii() { assert_eq!(GapBuffer::char_visual_width('a'), 1); }
    #[test] fn char_visual_width_cjk() { assert_eq!(GapBuffer::char_visual_width('日'), 2); }
    #[test] fn multiple_inserts_consistent() {
        let mut b = GapBuffer::new();
        b.insert(0, "c");
        b.insert(0, "a");
        b.insert(1, "b");
        assert_eq!(b.as_str(), "abc");
    }
    #[test] fn delete_and_reinsert() {
        let mut b = buf("hello world");
        b.delete(5..11);
        b.insert(5, " rust");
        assert_eq!(b.as_str(), "hello rust");
    }
}
