# src-editor Core (Plan 2 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `src-editor` crate — a `no_std + alloc` text editor core with GapBuffer, CJK-aware cursor movement, Japanese IME state machine, undo/redo history, syntax highlighter, and the `EditorState` aggregate.

**Architecture:** Five self-contained modules (gap_buffer, cursor, ime, undo, highlighter) plus an `editor_state` aggregate and a crate root. Each module depends only on `src-desktop-types` (for shared primitives like `ScrollOffset`, `Selection`, `TextSpan`, `ImeEvent`). No std, no regex — state machine highlighter only.

**Tech Stack:** Rust `no_std + alloc`, `src-desktop-types` (shared types), `src-core` not required.

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md` §4

---

## File Map

| File | Responsibility |
|------|----------------|
| `src-editor/Cargo.toml` | Crate manifest; deps: src-desktop-types |
| `src-editor/src/lib.rs` | `#![no_std]`, module declarations, re-exports |
| `src-editor/src/gap_buffer.rs` | `GapBuffer` — UTF-8 byte buffer with gap |
| `src-editor/src/cursor.rs` | `Cursor` — byte/line/col position + CJK movement |
| `src-editor/src/ime.rs` | `ImeState`, `Preedit` — composition session manager |
| `src-editor/src/undo.rs` | `UndoHistory`, `UndoGroup`, `EditOp` |
| `src-editor/src/highlighter.rs` | `Highlighter`, `HighlightContext` — state machine |
| `src-editor/src/editor_state.rs` | `EditorState` aggregate |

---

## Task 1: Crate scaffold and `GapBuffer`

GapBuffer is the core data structure. Everything else builds on it. Implement and test it first.

**Files:**
- Create: `src-editor/Cargo.toml`
- Create: `src-editor/src/lib.rs`
- Create: `src-editor/src/gap_buffer.rs`

- [ ] **Step 1.1: Add `src-editor` to workspace**

Edit `Cargo.toml` (workspace root), add `"src-editor"` to members:

```toml
[workspace]
members = [
    "src-core",
    "src-web",
    "src-cli",
    "src-plugin-types",
    "src-plugin",
    "src-desktop-types",
    "src-editor",
]
resolver = "2"
```

- [ ] **Step 1.2: Create `src-editor/Cargo.toml`**

```toml
[package]
name = "src-editor"
version = "0.1.0"
edition = "2021"

[dependencies]
src-desktop-types = { path = "../src-desktop-types" }
```

- [ ] **Step 1.3: Create `src-editor/src/lib.rs`**

```rust
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
```

- [ ] **Step 1.4: Write failing tests for GapBuffer**

Create `src-editor/src/gap_buffer.rs` with just the struct and tests (no impl yet):

```rust
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ops::Range;

pub struct GapBuffer {
    buf:       Vec<u8>,
    gap_start: usize,
    gap_end:   usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> GapBuffer { GapBuffer::from_str(s) }

    #[test] fn empty_buffer_has_zero_bytes() {
        assert_eq!(buf("").len_bytes(), 0);
    }
    #[test] fn insert_at_start() {
        let mut b = buf("world");
        b.insert(0, "hello ");
        assert_eq!(b.as_str(), "hello world");
    }
    #[test] fn insert_in_middle() {
        let mut b = buf("helo");
        b.insert(2, "ll");
        assert_eq!(b.as_str(), "helo");  // will fail — expected "hello" after fix
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
}
```

- [ ] **Step 1.5: Run tests to verify they fail**

```bash
cargo test -p src-editor 2>&1 | grep -E "FAILED|error\[" | head -10
```

Expected: compile error (methods not defined yet).

- [ ] **Step 1.6: Implement GapBuffer**

Replace the file with the full implementation:

```rust
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
        b.insert(2, "ll");
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
```

- [ ] **Step 1.7: Run GapBuffer tests**

```bash
cargo test -p src-editor gap_buffer 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 16 passed`

- [ ] **Step 1.8: Commit**

```bash
git add Cargo.toml src-editor/
git commit -m "feat(editor): add GapBuffer with gap management and line utilities"
```

---

## Task 2: Cursor (CJK-aware movement)

Cursor tracks byte position, line, and column. All moves go through GapBuffer queries.

**Files:**
- Create: `src-editor/src/cursor.rs`

- [ ] **Step 2.1: Write failing tests**

Add file `src-editor/src/cursor.rs` with struct and tests (impl stubs):

```rust
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
```

- [ ] **Step 2.2: Run to confirm failure**

```bash
cargo test -p src-editor cursor 2>&1 | grep -E "FAILED|error\[" | head -5
```

Expected: compile errors (methods not defined).

- [ ] **Step 2.3: Implement Cursor**

Replace file contents with full implementation:

```rust
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
        let mut pos = self.byte_pos;
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
        let mut buf = buf("日本");
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
        let mut cur = Cursor { byte_pos: 3, line: 0, col_byte: 3, preferred_visual_col: 3 };
        cur.move_line_start(&mut buf);
        assert_eq!(cur.byte_pos, 0);
    }
    #[test] fn move_word_right_stops_at_boundary() {
        let mut buf = buf("hello world");
        let mut cur = Cursor::new();
        cur.move_word_right(&mut buf);
        assert_eq!(cur.byte_pos, 5);
    }
    #[test] fn visual_col_ascii() {
        let mut buf = buf("hello");
        let cur = Cursor { byte_pos: 3, line: 0, col_byte: 3, preferred_visual_col: 0 };
        assert_eq!(cur.visual_col(&mut buf), 3);
    }
    #[test] fn visual_col_after_cjk() {
        let mut buf = buf("日x");
        let cur = Cursor { byte_pos: 3, line: 0, col_byte: 3, preferred_visual_col: 0 };
        assert_eq!(cur.visual_col(&mut buf), 2);
    }
}
```

- [ ] **Step 2.4: Run cursor tests**

```bash
cargo test -p src-editor cursor 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 10 passed`

- [ ] **Step 2.5: Commit**

```bash
git add src-editor/src/cursor.rs
git commit -m "feat(editor): add CJK-aware Cursor with word movement"
```

---

## Task 3: IME state machine

`ImeState` manages an active composition session without touching the buffer until commit.

**Files:**
- Create: `src-editor/src/ime.rs`

- [ ] **Step 3.1: Write failing tests first**

Create `src-editor/src/ime.rs`:

```rust
use alloc::string::String;
use src_desktop_types::ImeEvent;
use crate::gap_buffer::GapBuffer;
use crate::cursor::Cursor;

pub struct Preedit {
    pub text:            String,
    pub cursor:          Option<(usize, usize)>,
    pub insert_byte_pos: usize,
}

pub struct ImeState {
    pub composing: Option<Preedit>,
}

impl ImeState {
    pub fn new() -> Self { ImeState { composing: None } }
    pub fn is_composing(&self) -> bool { self.composing.is_some() }
}

impl Default for ImeState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (GapBuffer, Cursor, ImeState) {
        (GapBuffer::from_str("hello"), Cursor::new(), ImeState::new())
    }

    #[test] fn start_records_position() {
        let (mut buf, mut cur, mut ime) = setup();
        cur.byte_pos = 3;
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        assert!(ime.is_composing());
        assert_eq!(ime.composing.as_ref().unwrap().insert_byte_pos, 3);
    }
    #[test] fn update_sets_preedit_text() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Update { preedit: "にほ".into(), cursor: None }, &mut buf, &mut cur);
        assert_eq!(ime.composing.as_ref().unwrap().text, "にほ");
    }
    #[test] fn commit_inserts_into_buffer() {
        let (mut buf, mut cur, mut ime) = setup();
        cur.byte_pos = 5; // end of "hello"
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Commit { text: "語".into() }, &mut buf, &mut cur);
        assert!(!ime.is_composing());
        assert_eq!(buf.as_str(), "hello語");
    }
    #[test] fn cancel_clears_composing() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Cancel, &mut buf, &mut cur);
        assert!(!ime.is_composing());
    }
    #[test] fn commit_moves_cursor_to_end() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Commit { text: "ab".into() }, &mut buf, &mut cur);
        assert_eq!(cur.byte_pos, 2); // "ab" = 2 bytes
    }
}
```

- [ ] **Step 3.2: Run to confirm failure**

```bash
cargo test -p src-editor ime 2>&1 | grep -E "FAILED|error\[" | head -5
```

Expected: compile error (`apply` not defined).

- [ ] **Step 3.3: Implement `ImeState::apply`**

Add the method to `ImeState`:

```rust
impl ImeState {
    pub fn new() -> Self { ImeState { composing: None } }
    pub fn is_composing(&self) -> bool { self.composing.is_some() }

    pub fn apply(&mut self, event: ImeEvent, buf: &mut GapBuffer, cursor: &mut Cursor) {
        match event {
            ImeEvent::Start => {
                self.composing = Some(Preedit {
                    text: String::new(),
                    cursor: None,
                    insert_byte_pos: cursor.byte_pos,
                });
            }
            ImeEvent::Update { preedit, cursor: ime_cursor } => {
                if let Some(p) = &mut self.composing {
                    p.text   = preedit;
                    p.cursor = ime_cursor;
                }
            }
            ImeEvent::Commit { text } => {
                if let Some(p) = self.composing.take() {
                    let pos = p.insert_byte_pos;
                    let len = text.len();
                    buf.insert(pos, &text);
                    cursor.byte_pos = pos + len;
                    cursor.sync_line_col(buf);
                }
            }
            ImeEvent::Cancel => {
                self.composing = None;
            }
        }
    }
}
```

- [ ] **Step 3.4: Run IME tests**

```bash
cargo test -p src-editor ime 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 5 passed`

- [ ] **Step 3.5: Commit**

```bash
git add src-editor/src/ime.rs
git commit -m "feat(editor): add ImeState composition session manager"
```

---

## Task 4: Undo/Redo history

`UndoHistory` records operations in groups; undo/redo replays them in reverse.

**Files:**
- Create: `src-editor/src/undo.rs`

- [ ] **Step 4.1: Create `src-editor/src/undo.rs` with tests**

```rust
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
```

- [ ] **Step 4.2: Run undo tests**

```bash
cargo test -p src-editor undo 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 6 passed`

- [ ] **Step 4.3: Commit**

```bash
git add src-editor/src/undo.rs
git commit -m "feat(editor): add UndoHistory with group undo/redo"
```

---

## Task 5: Syntax Highlighter

State machine highlighter, no regex. Recognises Gloss Markdown syntax: headings, ruby, anno, math, code blocks, inline code.

**Files:**
- Create: `src-editor/src/highlighter.rs`

- [ ] **Step 5.1: Create `src-editor/src/highlighter.rs` with tests**

```rust
use alloc::vec::Vec;
use alloc::string::String;
use src_desktop_types::{TextSpan};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighlightContext {
    Normal,
    InCodeBlock { lang: &'static str },
    InMathBlock,
}

// ARGB color constants
const COLOR_DEFAULT:  u32 = 0xFF_D4D4D4;
const COLOR_HEADING:  u32 = 0xFF_569CD6;
const COLOR_RUBY:     u32 = 0xFF_CE9178;
const COLOR_ANNO:     u32 = 0xFF_9CDCFE;
const COLOR_MATH:     u32 = 0xFF_C586C0;
const COLOR_CODE:     u32 = 0xFF_4EC9B0;
const COLOR_KEYWORD:  u32 = 0xFF_569CD6;
const COLOR_COMMENT:  u32 = 0xFF_6A9955;

pub struct Highlighter;

impl Highlighter {
    /// Highlight one line. Returns a Vec of TextSpan covering the whole line.
    pub fn highlight_line(line: &str, ctx: HighlightContext) -> Vec<TextSpan> {
        match ctx {
            HighlightContext::InCodeBlock { .. } => highlight_code_line(line),
            HighlightContext::InMathBlock  => vec![span(line, COLOR_MATH, false, false)],
            HighlightContext::Normal       => highlight_normal_line(line),
        }
    }
}

fn highlight_normal_line(line: &str) -> Vec<TextSpan> {
    // Heading: starts with one or more `#`
    if line.starts_with('#') {
        let hashes = line.chars().take_while(|&c| c == '#').count();
        let rest = &line[hashes..];
        return vec![
            span(&line[..hashes], COLOR_HEADING, true, false),
            span(rest, COLOR_HEADING, true, false),
        ];
    }
    // Fallback: scan for inline constructs
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut text_start = 0usize;

    macro_rules! flush {
        ($end:expr) => {
            if text_start < $end {
                spans.push(span(&line[text_start..$end], COLOR_DEFAULT, false, false));
            }
        };
    }

    while let Some((i, c)) = chars.next() {
        match c {
            '[' => {
                // Ruby: [base/ruby] — scan forward for matching ]
                flush!(i);
                let rest = &line[i..];
                if let Some(end) = rest.find(']') {
                    let _inner = &rest[1..end];
                    spans.push(span(&rest[..end + 1], COLOR_RUBY, false, false));
                    // Skip consumed chars
                    let consumed = end + 1;
                    for _ in 0..rest[1..consumed].chars().count() { chars.next(); }
                    text_start = i + consumed;
                } else {
                    text_start = i;
                }
            }
            '{' => {
                // Anno: {word/anno}
                flush!(i);
                let rest = &line[i..];
                if let Some(end) = rest.find('}') {
                    spans.push(span(&rest[..end + 1], COLOR_ANNO, false, false));
                    let consumed = end + 1;
                    for _ in 0..rest[1..consumed].chars().count() { chars.next(); }
                    text_start = i + consumed;
                } else {
                    text_start = i;
                }
            }
            '$' => {
                // Inline math: $...$
                flush!(i);
                let rest = &line[i + 1..];
                if let Some(end) = rest.find('$') {
                    let full_len = 1 + end + 1;
                    spans.push(span(&line[i..i + full_len], COLOR_MATH, false, true));
                    for _ in 0..end { chars.next(); }
                    chars.next(); // closing $
                    text_start = i + full_len;
                } else {
                    text_start = i;
                }
            }
            '`' => {
                // Inline code: `...`
                flush!(i);
                let rest = &line[i + 1..];
                if let Some(end) = rest.find('`') {
                    let full_len = 1 + end + 1;
                    spans.push(span(&line[i..i + full_len], COLOR_CODE, false, false));
                    for _ in 0..end { chars.next(); }
                    chars.next();
                    text_start = i + full_len;
                } else {
                    text_start = i;
                }
            }
            _ => {}
        }
    }
    flush!(line.len());
    if spans.is_empty() {
        spans.push(span(line, COLOR_DEFAULT, false, false));
    }
    spans
}

fn highlight_code_line(line: &str) -> Vec<TextSpan> {
    // Minimal: comments start with // or #
    if line.trim_start().starts_with("//") || line.trim_start().starts_with('#') {
        return vec![span(line, COLOR_COMMENT, false, true)];
    }
    vec![span(line, COLOR_DEFAULT, false, false)]
}

fn span(text: &str, color: u32, bold: bool, italic: bool) -> TextSpan {
    TextSpan { text: String::from(text), color, bold, italic }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn plain_line_is_default_color() {
        let spans = Highlighter::highlight_line("hello world", HighlightContext::Normal);
        assert!(spans.iter().all(|s| s.color == COLOR_DEFAULT));
    }
    #[test] fn heading_gets_heading_color() {
        let spans = Highlighter::highlight_line("## Title", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_HEADING));
    }
    #[test] fn ruby_bracket_colored() {
        let spans = Highlighter::highlight_line("[漢字/かんじ]", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_RUBY));
    }
    #[test] fn anno_brace_colored() {
        let spans = Highlighter::highlight_line("{word/gloss}", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_ANNO));
    }
    #[test] fn inline_math_colored() {
        let spans = Highlighter::highlight_line("result is $x^2$", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_MATH));
    }
    #[test] fn inline_code_colored() {
        let spans = Highlighter::highlight_line("use `cargo test`", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_CODE));
    }
    #[test] fn math_block_whole_line() {
        let spans = Highlighter::highlight_line("x^2 + y^2", HighlightContext::InMathBlock);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].color, COLOR_MATH);
    }
    #[test] fn code_block_comment_italic() {
        let spans = Highlighter::highlight_line("// comment", HighlightContext::InCodeBlock { lang: "rust" });
        assert!(spans.iter().any(|s| s.italic));
    }
    #[test] fn spans_cover_full_line() {
        let line = "hello [漢字/かんじ] world";
        let spans = Highlighter::highlight_line(line, HighlightContext::Normal);
        let total: usize = spans.iter().map(|s| s.text.len()).sum();
        assert_eq!(total, line.len());
    }
}
```

- [ ] **Step 5.2: Run highlighter tests**

```bash
cargo test -p src-editor highlighter 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 9 passed`

- [ ] **Step 5.3: Commit**

```bash
git add src-editor/src/highlighter.rs
git commit -m "feat(editor): add state-machine syntax highlighter for Gloss Markdown"
```

---

## Task 6: EditorState aggregate and final wiring

`EditorState` composes GapBuffer, Cursor, ImeState, UndoHistory, and Selection into one struct. Also validate that `src-editor` compiles for `wasm32-unknown-unknown`.

**Files:**
- Create: `src-editor/src/editor_state.rs`

- [ ] **Step 6.1: Create `src-editor/src/editor_state.rs`**

```rust
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
    pub version:   u64,
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
        let buf_str = self.buffer.as_str();
        let start = prev_char_boundary(&buf_str, end);
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
```

- [ ] **Step 6.2: Run editor_state tests**

```bash
cargo test -p src-editor editor_state 2>&1 | grep -E "test result|FAILED"
```

Expected: `test result: ok. 5 passed`

- [ ] **Step 6.3: Run full src-editor test suite**

```bash
cargo test -p src-editor 2>&1 | grep -E "test result|FAILED"
```

Expected: all tests pass (gap_buffer: 16, cursor: 10, ime: 5, undo: 6, highlighter: 9, editor_state: 5 = ~51 tests)

- [ ] **Step 6.4: Verify no_std compliance**

```bash
cargo build -p src-editor --target wasm32-unknown-unknown
```

Expected: success.

- [ ] **Step 6.5: Run full workspace test suite**

```bash
cargo test 2>&1 | grep -E "test result|FAILED"
```

Expected: all existing tests still pass.

- [ ] **Step 6.6: Commit**

```bash
git add src-editor/src/editor_state.rs
git commit -m "feat(editor): add EditorState aggregate, complete src-editor crate"
```

---

## Definition of Done

- `cargo test -p src-editor` passes (~51 tests across 6 modules)
- `cargo build -p src-editor --target wasm32-unknown-unknown` succeeds
- `cargo test` (full workspace) passes
- All modules are `#![no_std]` — no `use std::` anywhere in `src-editor/`
