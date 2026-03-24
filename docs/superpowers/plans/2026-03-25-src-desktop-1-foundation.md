# src-desktop Foundation (Plan 1 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `src-plugin-types` to `no_std + alloc` (moving `to_plugin_events` into it), then create the `src-desktop-types` crate — the shared protocol layer (primitive types, traits, MemoryVfs, NoopPluginHost) — that all subsequent plans depend on.

**Architecture:** Two crates are created/modified. `src-plugin-types` gains `#![no_std]` and absorbs `to_plugin_events()` from `src-plugin/convert.rs`. `src-desktop-types` is a new no_std crate holding every shared type (VfsPath, AppEvent, AppCmd, DrawCmd, FileSystem, PluginHost, MemoryVfs, etc.) defined in spec §3. Both are added to the workspace.

**Tech Stack:** Rust, no_std + alloc, serde (alloc mode), serde_json (alloc mode), BTreeMap (no HashMap), src-core (Event type).

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md`

---

## File Map

### Modified files

| File | Change |
|------|--------|
| `Cargo.toml` | Add `src-desktop-types` to workspace members |
| `src-plugin-types/Cargo.toml` | Switch to no_std serde/serde_json; add src-core dep |
| `src-plugin-types/src/lib.rs` | Add `#![no_std]`, `extern crate alloc`, `to_plugin_events()` |
| `src-plugin/src/convert.rs` | Re-export `to_plugin_events` from `src-plugin-types` |

### Created files

| File | Contents |
|------|----------|
| `src-desktop-types/Cargo.toml` | no_std crate, deps: src-core, src-plugin-types, serde, serde_json |
| `src-desktop-types/src/lib.rs` | `#![no_std]` + module declarations |
| `src-desktop-types/src/path.rs` | `VfsPath`, `DirEntry`, `FsError` |
| `src-desktop-types/src/primitives.rs` | `PaneId`, `DocId`, `Rect`, `ScrollOffset`, `KeyEvent`, `MouseEvent`, `Selection`, all DrawCmd helper structs |
| `src-desktop-types/src/config.rs` | `LintRules`, `PluginEntrySpec`, `AppConfig` |
| `src-desktop-types/src/events.rs` | `AppEvent`, `AppCmd`, `SplitDirection`, `PaneKind`, `FocusTarget` |
| `src-desktop-types/src/draw.rs` | `DrawCmd`, `PanelLayout`, `DividerLayout` |
| `src-desktop-types/src/traits.rs` | `FileSystem`, `PluginHost`, `Clipboard`, `ImeSource`, `ImeEvent` |
| `src-desktop-types/src/memory_vfs.rs` | `MemoryVfs`, `VfsNode` |
| `src-desktop-types/src/noop.rs` | `NoopPluginHost` (cfg-gated) |
| `src-desktop-types/src/editor_view.rs` | `EditorViewModel` |

---

## Task 1: Migrate `src-plugin-types` to no_std

`src-plugin-types` currently uses `serde_json = "1"` (std). We add `#![no_std]`, switch to alloc-only serde/serde_json, add `src-core` as a dependency, and move `to_plugin_events()` here.

**Files:**
- Modify: `src-plugin-types/Cargo.toml`
- Modify: `src-plugin-types/src/lib.rs`
- Modify: `src-plugin/src/convert.rs`

- [ ] **Step 1.1: Update `src-plugin-types/Cargo.toml`**

Replace the entire file:

```toml
[package]
name = "src-plugin-types"
version = "0.1.0"
edition = "2021"

[dependencies]
serde     = { version = "1", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1", default-features = false, features = ["alloc"] }
src-core  = { path = "../src-core" }
```

- [ ] **Step 1.2: Add `#![no_std]`, `extern crate alloc`, and `to_plugin_events()` to `src-plugin-types/src/lib.rs`**

The full updated file (all existing types + new function at the bottom):

```rust
#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

use serde::{Deserialize, Serialize};
use src_core::parser::{Event, Tag};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PluginWarning {
    pub code:    String,
    pub message: String,
    pub line:    u32,
    pub col:     u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PluginFrontMatterField {
    pub key: String,
    pub raw: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum PluginEvent {
    Start        { tag: String },
    End          { tag: String },
    Text         { content: String },
    MathInline   { latex: String },
    MathDisplay  { latex: String },
    FrontMatter  { fields: Vec<PluginFrontMatterField> },
    CardLink     { url: String },
    FootnoteRef  { number: u32 },
    SoftBreak,
    HardBreak,
    Rule,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CodeHighlightInput {
    pub lang:     String,
    pub code:     String,
    pub filename: String,
    pub config:   serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CodeHighlightOutput {
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardLinkInput {
    pub url:    String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardLinkOutput {
    pub html: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LintRuleInput {
    pub source:             String,
    pub markdown:           String,
    pub existing_warnings:  Vec<PluginWarning>,  // keep existing field name (used by src-plugin)
    pub events:             Vec<PluginEvent>,
    pub config:             serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LintRuleOutput {
    pub warnings: Vec<PluginWarning>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FrontMatterInput {
    pub fields: Vec<PluginFrontMatterField>,
    pub source: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FrontMatterOutput {
    /// None = use default rendering. Keep field name `html` (used by src-plugin).
    pub html: Option<String>,
}

// ── Conversion: src-core Event → PluginEvent ──────────────────────────────

/// Convert a slice of core `Event`s to `PluginEvent`s (used by AppCore before
/// calling `PluginHost::run_lint_rule`). `BlockId` events are dropped.
pub fn to_plugin_events(events: &[Event<'_>]) -> Vec<PluginEvent> {
    events.iter().filter_map(to_plugin_event).collect()
}

fn to_plugin_event(event: &Event<'_>) -> Option<PluginEvent> {
    Some(match event {
        Event::Text(t)           => PluginEvent::Text { content: t.to_string() },
        Event::SoftBreak         => PluginEvent::SoftBreak,
        Event::HardBreak         => PluginEvent::HardBreak,
        Event::Rule              => PluginEvent::Rule,
        Event::MathInline(m)    => PluginEvent::MathInline { latex: m.to_string() },
        Event::MathDisplay(m)   => PluginEvent::MathDisplay { latex: m.to_string() },
        Event::CardLink(url)     => PluginEvent::CardLink { url: url.to_string() },
        Event::FootnoteRef(n)    => PluginEvent::FootnoteRef { number: *n },
        Event::FrontMatter(flds) => PluginEvent::FrontMatter {
            fields: flds.iter().map(|f| PluginFrontMatterField {
                key: f.key.to_string(),
                raw: f.raw.to_string(),
            }).collect(),
        },
        Event::Start(tag)        => PluginEvent::Start { tag: tag_to_string(tag) },
        Event::End(tag)          => PluginEvent::End   { tag: tag_to_string(tag) },
        Event::BlockId(_)        => return None,
    })
}

fn tag_to_string(tag: &Tag) -> String {
    match tag {
        Tag::Paragraph           => "Paragraph".into(),
        Tag::Heading(n)          => format!("Heading({n})"),
        Tag::Section(n)          => format!("Section({n})"),
        Tag::List(true)          => "OrderedList".into(),
        Tag::List(false)         => "UnorderedList".into(),
        Tag::Item                => "Item".into(),
        Tag::Code                => "Code".into(),
        Tag::CodeBlock(l, f)     => format!("CodeBlock({l},{f})"),
        Tag::Blockquote          => "Blockquote".into(),
        Tag::Table(_)            => "Table".into(),
        Tag::TableHead           => "TableHead".into(),
        Tag::TableRow            => "TableRow".into(),
        Tag::TableCell(_)        => "TableCell".into(),
        Tag::Strong              => "Strong".into(),
        Tag::Emphasis            => "Emphasis".into(),
        Tag::Strikethrough       => "Strikethrough".into(),
        Tag::Link(_)             => "Link".into(),
        Tag::Image(_, _)         => "Image".into(),
        Tag::Ruby(_)             => "Ruby".into(),
        Tag::Anno(_)             => "Anno".into(),
        Tag::AnnoNote            => "AnnoNote".into(),
        Tag::FootnoteSection     => "FootnoteSection".into(),
        Tag::FootnoteItem(n)     => format!("FootnoteItem({n})"),
    }
}
```

- [ ] **Step 1.3: Update `src-plugin/src/convert.rs` to re-export from `src-plugin-types`**

This keeps existing code in `src-plugin` working without changes. Replace the file:

```rust
// Re-export conversion utilities that have moved to src-plugin-types.
// src-plugin internal code that calls to_plugin_events() still works unchanged.
pub use src_plugin_types::to_plugin_events;

use src_plugin_types::{PluginWarning};
use src_core::parser::Warning;

/// Convert core `Warning` slice to plugin `PluginWarning` vec.
pub fn to_plugin_warnings(warnings: &[Warning]) -> Vec<PluginWarning> {
    warnings.iter().map(|w| PluginWarning {
        code:    w.code.to_string(),
        message: w.message.clone(),
        line:    w.line,
        col:     w.col,
    }).collect()
}
```

- [ ] **Step 1.4: Build to check compilation**

```bash
cargo build -p src-plugin-types -p src-plugin
```

Expected: Compiles without errors. If errors appear about `std::` paths, replace with `alloc::` equivalents.

- [ ] **Step 1.5: Run the full test suite**

```bash
cargo test
```

Expected: All existing tests pass. The `to_plugin_events` tests in `src-plugin` (if any) should still pass via the re-export.

- [ ] **Step 1.6: Verify no_std compile target**

```bash
cargo build -p src-plugin-types --target thumbv7m-none-eabi 2>&1 | grep -E "^error" | head -20
```

If `thumbv7m-none-eabi` is not installed, use `--target wasm32-unknown-unknown` instead.
Expected: Zero `error[...]` lines (warnings are fine).

- [ ] **Step 1.7: Commit**

```bash
git add src-plugin-types/Cargo.toml src-plugin-types/src/lib.rs src-plugin/src/convert.rs
git commit -m "feat(plugin-types): migrate to no_std+alloc, absorb to_plugin_events"
```

---

## Task 2: Create the `src-desktop-types` crate

This crate is the no_std protocol layer shared by every logic crate. It contains all types, traits, and utilities defined in spec §3.

**Files:** All files in `src-desktop-types/`.

- [ ] **Step 2.1: Add `src-desktop-types` to the workspace**

Edit `Cargo.toml`, add `"src-desktop-types"` to the `members` array:

```toml
[workspace]
members = [
    "src-core",
    "src-web",
    "src-cli",
    "src-plugin-types",
    "src-plugin",
    "src-desktop-types",
]
resolver = "2"
```

- [ ] **Step 2.2: Create `src-desktop-types/Cargo.toml`**

```toml
[package]
name = "src-desktop-types"
version = "0.1.0"
edition = "2021"

[dependencies]
src-core         = { path = "../src-core" }
src-plugin-types = { path = "../src-plugin-types" }
serde            = { version = "1", default-features = false, features = ["derive", "alloc"] }
serde_json       = { version = "1", default-features = false, features = ["alloc"] }

[features]
default     = []
test-utils  = []
```

- [ ] **Step 2.3: Create `src-desktop-types/src/lib.rs`**

```rust
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

// Convenient re-exports
pub use path::{VfsPath, DirEntry, FsError};
pub use primitives::*;
pub use config::{LintRules, PluginEntrySpec, AppConfig};
pub use traits::{FileSystem, PluginHost, Clipboard, ImeSource, ImeEvent};
pub use events::{AppEvent, AppCmd, SplitDirection, PaneKind, FocusTarget};
pub use draw::{DrawCmd, PanelLayout, DividerLayout};
pub use memory_vfs::MemoryVfs;
pub use editor_view::EditorViewModel;
```

- [ ] **Step 2.4: Create `src-desktop-types/src/path.rs`**

```rust
use alloc::string::{String, ToString};
use alloc::borrow::ToOwned;
use core::fmt;

/// OS-path-independent path type (no_std). Uses `/` as separator internally.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VfsPath(pub String);

impl VfsPath {
    pub fn as_str(&self) -> &str { &self.0 }

    pub fn join(&self, segment: &str) -> Self {
        if self.0.ends_with('/') {
            VfsPath(alloc::format!("{}{}", self.0, segment))
        } else {
            VfsPath(alloc::format!("{}/{}", self.0, segment))
        }
    }

    pub fn parent(&self) -> Option<Self> {
        let s = self.0.trim_end_matches('/');
        let idx = s.rfind('/')?;
        Some(VfsPath(s[..idx].to_owned()))
    }

    pub fn file_name(&self) -> Option<&str> {
        let s = self.0.trim_end_matches('/');
        s.rfind('/').map(|i| &s[i + 1..]).or(if s.is_empty() { None } else { Some(s) })
    }
}

impl From<&str> for VfsPath {
    fn from(s: &str) -> Self { VfsPath(s.to_owned()) }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}

pub struct DirEntry {
    pub name:   String,
    pub path:   VfsPath,
    pub is_dir: bool,
}

#[derive(Debug)]
pub enum FsError {
    NotFound(VfsPath),
    PermissionDenied,
    AlreadyExists(VfsPath),
    Io(String),
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::NotFound(p)     => write!(f, "not found: {}", p.as_str()),
            FsError::PermissionDenied => write!(f, "permission denied"),
            FsError::AlreadyExists(p) => write!(f, "already exists: {}", p.as_str()),
            FsError::Io(msg)         => write!(f, "I/O error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_adds_separator() {
        let p = VfsPath::from("/foo");
        assert_eq!(p.join("bar").as_str(), "/foo/bar");
    }

    #[test]
    fn join_no_double_slash() {
        let p = VfsPath::from("/foo/");
        assert_eq!(p.join("bar").as_str(), "/foo/bar");
    }

    #[test]
    fn parent_returns_prefix() {
        let p = VfsPath::from("/foo/bar/baz.md");
        assert_eq!(p.parent().unwrap().as_str(), "/foo/bar");
    }

    #[test]
    fn file_name_returns_last_segment() {
        let p = VfsPath::from("/foo/bar/baz.md");
        assert_eq!(p.file_name(), Some("baz.md"));
    }

    #[test]
    fn root_has_no_parent() {
        assert!(VfsPath::from("/").parent().is_none());
    }
}
```

- [ ] **Step 2.5: Create `src-desktop-types/src/primitives.rs`**

```rust
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use crate::path::VfsPath;

// ── Identity types ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocId(pub u64);

// ── Geometry ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ScrollOffset { pub x: f32, pub y: f32 }

// ── Keyboard / mouse input ─────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyEvent {
    pub key:  KeyCode,
    pub mods: Modifiers,
    /// Printable character text (not via IME).
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl:  bool,
    pub shift: bool,
    pub alt:   bool,
    pub meta:  bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Enter, Backspace, Delete, Escape, Tab,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
    F(u8),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MouseEvent {
    pub kind:   MouseKind,
    pub x:      f32,
    pub y:      f32,
    pub button: MouseButton,
    pub mods:   Modifiers,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MouseKind {
    Press, Release, Move,
    Scroll { delta_x: f32, delta_y: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MouseButton { Left, Right, Middle }

// ── Editor selection ───────────────────────────────────────────────────────

/// Byte positions into the GapBuffer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Selection { pub anchor: usize, pub active: usize }

// ── DrawCmd display helpers ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorDisplay { pub line: u32, pub visual_col: u32, pub blink: bool }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectionDisplay {
    /// (line_start, col_start, line_end, col_end) in visual columns.
    pub ranges: Vec<(u32, u32, u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreeditDraw {
    pub text:            String,
    pub underline_range: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorDraw { pub x: f32, pub y: f32, pub height: f32 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectionDraw { pub rects: Vec<Rect> }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorLine {
    pub line_no: u32,
    pub spans:   Vec<TextSpan>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text:   String,
    pub color:  u32,   // ARGB
    pub bold:   bool,
    pub italic: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    pub doc_id: DocId,
    pub title:  String,
    pub dirty:  bool,
}

// ── Content data types ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileTreeEntry {
    pub name:     String,
    pub path:     VfsPath,
    pub is_dir:   bool,
    pub depth:    u32,
    pub expanded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id:      String,
    pub path:    VfsPath,
    pub hooks:   Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WarningInfo {
    /// Display adapter. PluginWarning.line (u32) → Some(line).
    pub code:    String,
    pub message: String,
    pub line:    Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DialogKind {
    OpenFile,
    SaveFile { suggested: Option<VfsPath> },
    Confirm  { message: String },
}

// ── HTML diff ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HtmlPatch {
    pub block_id: u64,   // fnv1a block hash
    pub html:     String,
}

// ── Layout ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SplitDirection { Horizontal, Vertical }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaneKind { Editor, Preview, FileTree, PluginManager }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FocusTarget { Pane(PaneId), StatusBar }

// ── Workspace helpers ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocMeta {
    pub path:  VfsPath,
    pub title: String,
    pub dirty: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub doc_id: DocId,
    pub title:  String,
    pub dirty:  bool,
}

// ── Model panel fields ────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct FileTreeState {
    pub root_path: Option<VfsPath>,
    pub entries:   Vec<FileTreeEntry>,
    pub expanded:  Vec<VfsPath>,
}

#[derive(Clone, Debug, Default)]
pub struct PluginManagerState {
    pub plugins: Vec<PluginInfo>,
}

#[derive(Clone, Debug, Default)]
pub struct StatusState {
    pub left:          String,
    pub right:         String,
    pub warning_count: u32,
}
```

- [ ] **Step 2.6: Create `src-desktop-types/src/config.rs`**

```rust
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::path::VfsPath;

/// Map of lint rule code → enabled.
pub struct LintRules(pub BTreeMap<String, bool>);

impl LintRules {
    pub fn is_enabled(&self, code: &str) -> bool {
        self.0.get(code).copied().unwrap_or(true)
    }
}

impl Default for LintRules {
    fn default() -> Self { LintRules(BTreeMap::new()) }
}

/// Plugin configuration entry (no_std).
/// `config` is a raw JSON string — the shell layer converts serde_json::Value → String.
pub struct PluginEntrySpec {
    pub id:     String,
    pub path:   VfsPath,
    pub hooks:  Vec<String>,
    /// JSON string. Parse at the point of use with serde_json::from_str.
    pub config: String,
}

pub struct AppConfig {
    pub lint:    LintRules,
    pub plugins: Vec<PluginEntrySpec>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig { lint: LintRules::default(), plugins: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_rules_default_enabled() {
        let rules = LintRules::default();
        assert!(rules.is_enabled("W001"), "unknown rules default to enabled");
    }

    #[test]
    fn lint_rules_explicit_disable() {
        let mut map = BTreeMap::new();
        map.insert("W001".into(), false);
        let rules = LintRules(map);
        assert!(!rules.is_enabled("W001"));
        assert!(rules.is_enabled("W002"), "unmentioned rules default to enabled");
    }
}
```

- [ ] **Step 2.7: Create `src-desktop-types/src/traits.rs`**

```rust
use alloc::string::String;
use alloc::vec::Vec;
use src_plugin_types::{CardLinkOutput, PluginWarning, PluginEvent, PluginFrontMatterField};
use crate::path::{VfsPath, DirEntry, FsError};

pub trait FileSystem {
    fn read(&self,       path: &VfsPath) -> Result<Vec<u8>, FsError>;
    fn write(&mut self,  path: &VfsPath, data: &[u8]) -> Result<(), FsError>;
    fn list_dir(&self,   path: &VfsPath) -> Result<Vec<DirEntry>, FsError>;
    fn exists(&self,     path: &VfsPath) -> bool;
    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError>;
    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError>;
    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError>;
    fn is_dir(&self,     path: &VfsPath) -> bool;
}

/// Plugin host abstraction. AppCore calls to_plugin_events() before run_lint_rule,
/// so this trait always receives converted PluginEvent slices.
pub trait PluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str)
        -> Option<String>;
    fn run_card_link(&mut self, url: &str)
        -> Option<CardLinkOutput>;
    fn run_lint_rule(&mut self, src: &str, md: &str,
        existing: &[PluginWarning], events: &[PluginEvent])
        -> Vec<PluginWarning>;
    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str)
        -> Option<String>;
}

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// IME event source. AppCore does not hold this trait — the shell polls it
/// and converts results to AppEvent::Ime before entering the dispatch loop.
/// Defined here (in the no_std protocol layer) so WASM and Tauri shells share
/// the same interface, and because ImeEvent lives here.
pub trait ImeSource {
    fn poll_event(&mut self) -> Option<ImeEvent>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImeEvent {
    Start,
    Update { preedit: String, cursor: Option<(usize, usize)> },
    Commit { text: String },
    Cancel,
}
```

- [ ] **Step 2.8: Create `src-desktop-types/src/events.rs`**

```rust
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use src_plugin_types::PluginWarning;
use crate::path::{VfsPath, DirEntry, FsError};
use crate::config::AppConfig;
use crate::traits::ImeEvent;
use crate::primitives::{
    PaneId, DocId, KeyEvent, MouseEvent, SplitDirection, DialogKind, Rect,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Ime(ImeEvent),
    Resize { width: u32, height: u32 },

    // FS completion (returned from AppCmd processing)
    FileLoaded  { path: VfsPath, content: Vec<u8> },
    FileSaved   { path: VfsPath },
    FileError   { path: VfsPath, error: String },  // FsError serialized as String
    DirLoaded   { path: VfsPath, entries: Vec<DirEntry> },

    // Rendering
    RenderComplete { pane_id: PaneId, html: String, warnings: Vec<PluginWarning> },
    LintComplete   { doc_id: DocId,   warnings: Vec<PluginWarning> },

    // Config
    ConfigLoaded(AppConfig),

    // Clipboard
    ClipboardText(String),

    // UI
    TabSelected        { pane_id: PaneId, tab_index: usize },
    PaneSplitRequested { pane_id: PaneId, direction: SplitDirection },

    // Preview
    PreviewLinkClicked { pane_id: PaneId, url: String, new_tab: bool },
    PreviewScrolled    { pane_id: PaneId, offset_y: f32 },

    Quit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppCmd {
    // ── AppCore handles ────────────────────────────────────────────────────
    WriteFile      { path: VfsPath, content: Vec<u8> },
    ListDir        { path: VfsPath },
    RunRender      { pane_id: PaneId, doc_id: DocId },
    RunLint        { doc_id: DocId },               // → LintComplete
    ScheduleRender { pane_id: PaneId, delay_ms: u32 },

    // ── Shell handles ──────────────────────────────────────────────────────
    ReadFile       { path: VfsPath },               // async I/O → FileLoaded
    LoadConfig     { path: VfsPath },               // async I/O → ConfigLoaded
    CopyToClipboard { text: String },
    PasteRequest,                                   // sync in spawn_blocking → ClipboardText
    SetImeCursorArea { rect: Rect },
    ShowOpenFileDialog,
    ShowSaveFileDialog { suggested: Option<VfsPath> },
    OpenUrl        { url: String },
    Quit,
}

// Re-export enums needed by layout/core crates that import from events.rs
pub use crate::primitives::{SplitDirection, PaneKind, FocusTarget, DialogKind};
```

Note: `AppConfig` doesn't implement `Serialize/Deserialize` by default (it's no_std without a derive). Add serde derives to `AppConfig` and `LintRules` in `config.rs`:

Update `config.rs` — add `use serde::{Deserialize, Serialize};` and derive macros:
```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LintRules(pub BTreeMap<String, bool>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginEntrySpec { ... }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig { ... }
```

- [ ] **Step 2.9: Create `src-desktop-types/src/draw.rs`**

```rust
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use crate::path::VfsPath;
use crate::primitives::{
    PaneId, PaneKind, Rect, EditorLine, CursorDraw, SelectionDraw, PreeditDraw,
    ScrollOffset, TabInfo, HtmlPatch, FileTreeEntry, PluginInfo, WarningInfo, DialogKind,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DrawCmd {
    // Layout (absolute coords for shell to position with CSS position:absolute)
    SetLayout {
        panels:   Vec<PanelLayout>,
        dividers: Vec<DividerLayout>,
    },
    SetTabBar {
        pane_id:    PaneId,
        tabs:       Vec<TabInfo>,
        active_tab: usize,
    },

    // Editor pane (Canvas 2D precise rendering)
    EditorFrame {
        pane_id:   PaneId,
        bounds:    Rect,
        lines:     Vec<EditorLine>,
        cursor:    CursorDraw,
        selection: Option<SelectionDraw>,
        preedit:   Option<PreeditDraw>,
        scroll:    ScrollOffset,
    },

    // Preview pane (HTML handed off to shell)
    PreviewMount  { pane_id: PaneId, html: String },
    PreviewPatch  { pane_id: PaneId, patches: Vec<HtmlPatch> },
    PreviewScroll { pane_id: PaneId, offset_y: f32 },

    // Content data (HTML/CSS handles rendering)
    SetFileTree   { entries: Vec<FileTreeEntry>, expanded: Vec<VfsPath> },
    SetPluginList { plugins: Vec<PluginInfo> },
    SetStatusBar  { left: String, right: String, warning_count: u32 },
    SetWarnings   { warnings: Vec<WarningInfo> },

    // IME / overlays
    SetImeCursorArea { rect: Rect },
    ShowDialog       { kind: DialogKind },
    ShowTooltip      { x: f32, y: f32, text: String },
    HideTooltip,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PanelLayout {
    pub pane_id: PaneId,
    pub bounds:  Rect,
    pub kind:    PaneKind,
    pub visible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DividerLayout {
    pub bounds:     Rect,
    pub direction:  crate::primitives::SplitDirection,
    pub draggable:  bool,
}
```

- [ ] **Step 2.10: Create `src-desktop-types/src/memory_vfs.rs`**

```rust
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::path::{VfsPath, DirEntry, FsError};
use crate::traits::FileSystem;

pub enum VfsNode {
    File { name: String, content: Vec<u8> },
    Dir  { name: String, children: BTreeMap<String, VfsNode> },
}

pub struct MemoryVfs {
    root: VfsNode,
}

impl MemoryVfs {
    pub fn new() -> Self {
        MemoryVfs { root: VfsNode::Dir { name: String::new(), children: BTreeMap::new() } }
    }

    /// Iterate all files as (path_str, content) pairs.
    pub fn iter_files(&self) -> impl Iterator<Item = (String, &[u8])> {
        let mut results = Vec::new();
        collect_files(&self.root, &mut String::from("/"), &mut results);
        results.into_iter()
    }

    fn navigate<'a>(root: &'a VfsNode, path: &VfsPath) -> Option<&'a VfsNode> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        let mut cur = root;
        for part in parts {
            match cur {
                VfsNode::Dir { children, .. } => cur = children.get(part)?,
                VfsNode::File { .. } => return None,
            }
        }
        Some(cur)
    }

    fn navigate_mut<'a>(root: &'a mut VfsNode, path: &VfsPath) -> Option<&'a mut VfsNode> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        let mut cur = root;
        for part in parts {
            match cur {
                VfsNode::Dir { children, .. } => cur = children.get_mut(part)?,
                VfsNode::File { .. } => return None,
            }
        }
        Some(cur)
    }
}

fn collect_files<'a>(node: &'a VfsNode, prefix: &mut String, out: &mut Vec<(String, &'a [u8])>) {
    match node {
        VfsNode::File { name, content } => {
            let path = if prefix.ends_with('/') {
                alloc::format!("{}{}", prefix, name)
            } else {
                alloc::format!("{}/{}", prefix, name)
            };
            out.push((path, content));
        }
        VfsNode::Dir { name, children } => {
            let prev_len = prefix.len();
            if !name.is_empty() {
                if !prefix.ends_with('/') { prefix.push('/'); }
                prefix.push_str(name);
            }
            for child in children.values() {
                collect_files(child, prefix, out);
            }
            prefix.truncate(prev_len);
        }
    }
}

impl FileSystem for MemoryVfs {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>, FsError> {
        match Self::navigate(&self.root, path) {
            Some(VfsNode::File { content, .. }) => Ok(content.clone()),
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn write(&mut self, path: &VfsPath, data: &[u8]) -> Result<(), FsError> {
        // Ensure parent dirs exist, then write/create the file.
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err(FsError::Io("empty path".into())); }
        let (dir_parts, file_name) = parts.split_at(parts.len() - 1);
        let file_name = file_name[0].to_string();
        let mut cur = &mut self.root;
        for part in dir_parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.entry(part.to_string()).or_insert_with(|| {
                        VfsNode::Dir { name: part.to_string(), children: BTreeMap::new() }
                    });
                }
                VfsNode::File { .. } => return Err(FsError::Io("parent is a file".into())),
            }
        }
        match cur {
            VfsNode::Dir { children, .. } => {
                children.insert(
                    file_name.clone(),
                    VfsNode::File { name: file_name, content: data.to_vec() },
                );
                Ok(())
            }
            VfsNode::File { .. } => Err(FsError::Io("expected directory".into())),
        }
    }

    fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        match Self::navigate(&self.root, path) {
            Some(VfsNode::Dir { children, .. }) => {
                Ok(children.values().map(|n| match n {
                    VfsNode::File { name, .. } => DirEntry {
                        name: name.clone(),
                        path: path.join(name),
                        is_dir: false,
                    },
                    VfsNode::Dir { name, .. } => DirEntry {
                        name: name.clone(),
                        path: path.join(name),
                        is_dir: true,
                    },
                }).collect())
            }
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn exists(&self, path: &VfsPath) -> bool { Self::navigate(&self.root, path).is_some() }

    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        let mut cur = &mut self.root;
        for part in &parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.entry(part.to_string()).or_insert_with(|| {
                        VfsNode::Dir { name: part.to_string(), children: BTreeMap::new() }
                    });
                }
                VfsNode::File { .. } => return Err(FsError::Io("parent is a file".into())),
            }
        }
        Ok(())
    }

    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err(FsError::Io("cannot delete root".into())); }
        let (dir_parts, name) = parts.split_at(parts.len() - 1);
        let name = name[0];
        let mut cur = &mut self.root;
        for part in dir_parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.get_mut(*part).ok_or_else(|| FsError::NotFound(path.clone()))?;
                }
                VfsNode::File { .. } => return Err(FsError::NotFound(path.clone())),
            }
        }
        match cur {
            VfsNode::Dir { children, .. } => {
                children.remove(name).ok_or(FsError::NotFound(path.clone()))?;
                Ok(())
            }
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        let content = self.read(from)?;
        self.write(to, &content)?;
        self.delete(from)
    }

    fn is_dir(&self, path: &VfsPath) -> bool {
        matches!(Self::navigate(&self.root, path), Some(VfsNode::Dir { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs() -> MemoryVfs { MemoryVfs::new() }

    #[test]
    fn write_and_read_file() {
        let mut vfs = vfs();
        vfs.write(&VfsPath::from("/foo/bar.md"), b"hello").unwrap();
        assert_eq!(vfs.read(&VfsPath::from("/foo/bar.md")).unwrap(), b"hello");
    }

    #[test]
    fn read_missing_file_returns_error() {
        let vfs = vfs();
        assert!(matches!(
            vfs.read(&VfsPath::from("/no/such.md")),
            Err(FsError::NotFound(_))
        ));
    }

    #[test]
    fn list_dir_returns_children() {
        let mut vfs = vfs();
        vfs.write(&VfsPath::from("/dir/a.md"), b"").unwrap();
        vfs.write(&VfsPath::from("/dir/b.md"), b"").unwrap();
        let mut entries = vfs.list_dir(&VfsPath::from("/dir")).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.md");
        assert_eq!(entries[1].name, "b.md");
    }

    #[test]
    fn delete_file() {
        let mut vfs = vfs();
        vfs.write(&VfsPath::from("/x.md"), b"data").unwrap();
        assert!(vfs.exists(&VfsPath::from("/x.md")));
        vfs.delete(&VfsPath::from("/x.md")).unwrap();
        assert!(!vfs.exists(&VfsPath::from("/x.md")));
    }

    #[test]
    fn rename_moves_file() {
        let mut vfs = vfs();
        vfs.write(&VfsPath::from("/old.md"), b"content").unwrap();
        vfs.rename(&VfsPath::from("/old.md"), &VfsPath::from("/new.md")).unwrap();
        assert!(!vfs.exists(&VfsPath::from("/old.md")));
        assert_eq!(vfs.read(&VfsPath::from("/new.md")).unwrap(), b"content");
    }
}
```

- [ ] **Step 2.11: Create `src-desktop-types/src/noop.rs`**

```rust
use alloc::vec::Vec;
use alloc::string::String;
use src_plugin_types::{CardLinkOutput, PluginWarning, PluginEvent, PluginFrontMatterField};
use crate::traits::PluginHost;

/// Stub PluginHost that does nothing. Used in tests and WASM playground.
pub struct NoopPluginHost;

impl PluginHost for NoopPluginHost {
    fn run_code_highlight(&mut self, _lang: &str, _code: &str, _filename: &str)
        -> Option<String> { None }
    fn run_card_link(&mut self, _url: &str)
        -> Option<CardLinkOutput> { None }
    fn run_lint_rule(&mut self, _src: &str, _md: &str,
        _existing: &[PluginWarning], _events: &[PluginEvent])
        -> Vec<PluginWarning> { Vec::new() }
    fn run_front_matter(&mut self, _fields: &[PluginFrontMatterField], _src: &str)
        -> Option<String> { None }
}
```

Note: `noop.rs` is only included when `#[cfg(any(test, feature = "test-utils", target_arch = "wasm32"))]` — this is handled by the conditional `pub mod noop;` in `lib.rs`.

- [ ] **Step 2.12: Create `src-desktop-types/src/editor_view.rs`**

`EditorViewModel` is in `src-desktop-types` so `src-desktop-layout` can store it in `Model` without depending on `src-editor`:

```rust
use alloc::vec::Vec;
use crate::primitives::{DocId, EditorLine, CursorDisplay, SelectionDisplay, PreeditDraw, ScrollOffset};

/// Snapshot of editor state for rendering. Computed by AppCore from EditorState,
/// stored in Model.workspace.editors, read by view().
#[derive(Clone, Debug, Default)]
pub struct EditorViewModel {
    pub doc_id:        DocId,
    pub visible_lines: Vec<EditorLine>,
    pub total_lines:   u32,
    pub cursor:        CursorDisplay,
    pub selection:     Option<SelectionDisplay>,
    pub preedit:       Option<PreeditDraw>,
    pub scroll:        ScrollOffset,
    pub dirty:         bool,
}

impl Default for DocId { fn default() -> Self { DocId(0) } }
impl Default for CursorDisplay {
    fn default() -> Self { CursorDisplay { line: 0, visual_col: 0, blink: true } }
}
```

- [ ] **Step 2.13: Build `src-desktop-types`**

```bash
cargo build -p src-desktop-types
```

Expected: Compiles without errors. Fix any import or missing-derive issues.

- [ ] **Step 2.14: Run `src-desktop-types` tests**

```bash
cargo test -p src-desktop-types
```

Expected: All `path.rs`, `config.rs`, and `memory_vfs.rs` tests pass.

- [ ] **Step 2.15: Run full workspace test to check nothing is broken**

```bash
cargo test
```

Expected: All existing tests pass.

- [ ] **Step 2.16: Verify no_std compile**

```bash
cargo build -p src-desktop-types --target wasm32-unknown-unknown
```

Expected: Compiles without errors (wasm32-unknown-unknown is a no_std target by default).

- [ ] **Step 2.17: Commit**

```bash
git add Cargo.toml src-desktop-types/
git commit -m "feat(desktop-types): create no_std protocol layer crate"
```

---

## Task 3: Verify `src-plugin` still works and fix lint

After the `src-plugin-types` migration, `src-plugin` must still compile and pass tests.

- [ ] **Step 3.1: Build all crates**

```bash
cargo build
```

Expected: All crates compile.

- [ ] **Step 3.2: Run full test suite one final time**

```bash
cargo test 2>&1 | tail -20
```

Expected: No test failures.

- [ ] **Step 3.3: Check for warnings in new code**

```bash
cargo build -p src-plugin-types -p src-desktop-types 2>&1 | grep "^warning"
```

Fix any dead_code, unused_import, or clippy-style warnings.

- [ ] **Step 3.4: Final commit**

```bash
git add -u
git commit -m "fix(foundation): clean up warnings in plugin-types and desktop-types"
```

---

## Definition of Done

- `src-plugin-types` compiles with `#![no_std]` and no `std` feature of serde/serde_json
- `to_plugin_events()` lives in `src-plugin-types` and is accessible there
- `src-plugin/convert.rs` re-exports `to_plugin_events` so existing call sites work
- `src-desktop-types` compiles on `wasm32-unknown-unknown`
- All `MemoryVfs`, `VfsPath`, `LintRules` tests pass
- `cargo test` (all workspace) passes
