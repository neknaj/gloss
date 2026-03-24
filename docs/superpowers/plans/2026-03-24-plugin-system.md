# Plugin System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Extism-based WASM plugin system to the Gloss CLI with four hooks: `code-highlight`, `card-link`, `lint-rule`, and `front-matter`.

**Architecture:** A new `src-plugin` crate owns all Extism host code and a `PluginAwareRenderer` that intercepts events before delegating to `HtmlRenderer`. `src-plugin-types` (no Extism dep) holds shared serde types for plugin PDK authors. `src-core` gains one additive change: `push_html_inner` is refactored into a public `HtmlRenderer` struct so `PluginAwareRenderer` can reuse it without duplicating logic.

**Tech Stack:** Rust, Extism host crate (`extism`), `serde`/`serde_json`, `toml`, `src-core` (unchanged API), WASM plugin bytecode (test fixtures only).

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Add `src-plugin-types`, `src-plugin` workspace members |
| Create | `src-plugin-types/Cargo.toml` | Minimal crate: serde + serde_json only |
| Create | `src-plugin-types/src/lib.rs` | Shared types: `PluginEvent`, `PluginWarning`, hook I/O structs |
| Create | `src-plugin/Cargo.toml` | Depends on extism, toml, serde_json, src-core, src-plugin-types |
| Create | `src-plugin/src/lib.rs` | Re-exports public API |
| Create | `src-plugin/src/config.rs` | `GlossConfig`, `LintConfig`, `PluginEntry`, TOML parsing |
| Create | `src-plugin/src/convert.rs` | `to_plugin_events`, `to_plugin_warnings`, `tag_to_string` |
| Create | `src-plugin/src/host.rs` | `GlossPluginHost` — Extism plugin loading + hook dispatch |
| Create | `src-plugin/src/renderer.rs` | `PluginAwareRenderer` — event-intercepting HTML renderer |
| Modify | `src-core/src/html.rs` | Refactor `push_html_inner` → `pub struct HtmlRenderer` |
| Modify | `src-core/src/lib.rs` | Re-export `HtmlRenderer` from `html` module |
| Modify | `src-cli/Cargo.toml` | Add `src-plugin` dependency |
| Modify | `src-cli/src/main.rs` | Load config, build host, use `PluginAwareRenderer` |
| Create | `src-cli/tests/plugin_config.rs` | Integration tests: config loading, lint merging |
| Create | `src-cli/tests/plugin_renderer.rs` | Integration tests: renderer event interception (no WASM) |

---

### Task 1: Workspace Scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `src-plugin-types/Cargo.toml`
- Create: `src-plugin-types/src/lib.rs` (stub)
- Create: `src-plugin/Cargo.toml`
- Create: `src-plugin/src/lib.rs` (stub)

- [ ] **Step 1: Add workspace members**

Edit `Cargo.toml`:

```toml
[workspace]
members = [
    "src-core",
    "src-web",
    "src-cli",
    "src-plugin-types",
    "src-plugin"
]
resolver = "2"
```

- [ ] **Step 2: Create `src-plugin-types/Cargo.toml`**

```toml
[package]
name = "src-plugin-types"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: Create `src-plugin-types/src/lib.rs` stub**

```rust
// Shared types between plugin host (src-plugin) and plugin PDK authors.
// No Extism dependency — pure serde.
```

- [ ] **Step 4: Create `src-plugin/Cargo.toml`**

```toml
[package]
name = "src-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
src-core = { path = "../src-core" }
src-plugin-types = { path = "../src-plugin-types" }
extism = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
```

- [ ] **Step 5: Create `src-plugin/src/lib.rs` stub**

```rust
pub mod config;
pub mod convert;
pub mod host;
pub mod renderer;
```

- [ ] **Step 6: Create stub files for each module**

Create `src-plugin/src/config.rs`, `src-plugin/src/convert.rs`, `src-plugin/src/host.rs`, `src-plugin/src/renderer.rs` each containing just:

```rust
// TODO
```

- [ ] **Step 7: Verify workspace compiles**

Run: `cargo build`
Expected: All crates compile (stubs produce warnings, not errors)

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src-plugin-types/ src-plugin/
git commit -m "feat: scaffold src-plugin-types and src-plugin workspace crates"
```

---

### Task 2: `HtmlRenderer` Struct Refactor

**Files:**
- Modify: `src-core/src/html.rs`
- Modify: `src-core/src/lib.rs`
- Test: `src-core/tests/integration.rs` (run existing — must still pass)

The goal is to expose `push_html_inner`'s state as a public struct so `PluginAwareRenderer` can reuse it. The public API (`push_html`, `push_html_with_ids`) must not change.

- [ ] **Step 1: Run existing tests to establish baseline**

Run: `cargo test --test integration`
Expected: All pass

- [ ] **Step 2: Define `HtmlRenderer` struct**

In `src-core/src/html.rs`, add after the imports:

```rust
/// Stateful HTML emitter. Feed events one at a time with `feed()`,
/// then call `finish()` after the last event to flush pending front matter.
pub struct HtmlRenderer {
    block_ids: bool,
    in_thead: bool,
    in_anno: bool,
    anno_rb_closed: bool,
    pending_bid: Option<u64>,
    pending_fm: Option<String>,
    fm_emitted: bool,
}

impl HtmlRenderer {
    pub fn new(block_ids: bool) -> Self {
        Self {
            block_ids,
            in_thead: false,
            in_anno: false,
            anno_rb_closed: false,
            pending_bid: None,
            pending_fm: None,
            fm_emitted: false,
        }
    }

    /// Emit HTML for one event into `out`.
    pub fn feed<'a>(&mut self, event: Event<'a>, out: &mut String) {
        // (body: move the match arms from push_html_inner here,
        //  replacing the closure `take_bid` with a method call)
        self.handle_event(event, out);
    }

    /// Call after all events. If no H1 was seen, prepends buffered
    /// front matter to the slice of `out` starting at `start_len`.
    pub fn finish(&mut self, out: &mut String, start_len: usize) {
        if let Some(fm) = self.pending_fm.take() {
            let content = out.split_off(start_len);
            out.push_str(&fm);
            out.push_str(&content);
        }
    }

    fn take_bid(&mut self) -> String {
        if self.block_ids {
            if let Some(id) = self.pending_bid.take() {
                return format!(" data-bid=\"{:x}\"", id);
            }
        } else {
            self.pending_bid.take();
        }
        String::new()
    }

    fn handle_event<'a>(&mut self, event: Event<'a>, out: &mut String) {
        match event {
            // paste all match arms from push_html_inner here,
            // replacing take_bid(&mut pending_bid) → self.take_bid()
            // and all local vars with self.field references
            _ => {}
        }
    }
}
```

- [ ] **Step 3: Migrate all match arms from `push_html_inner` into `handle_event`**

Move all 35+ match arms wholesale. Replace:
- `take_bid(&mut pending_bid)` → `self.take_bid()`
- `in_thead` → `self.in_thead`
- `in_anno` → `self.in_anno`
- `anno_rb_closed` → `self.anno_rb_closed`
- `pending_bid` → `self.pending_bid`
- `pending_fm` → `self.pending_fm`
- `fm_emitted` → `self.fm_emitted`

- [ ] **Step 4: Rewrite `push_html_inner` as a thin wrapper**

```rust
fn push_html_inner<'a>(out: &mut String, iter: Parser<'a>, block_ids: bool) {
    let start_len = out.len();
    let mut renderer = HtmlRenderer::new(block_ids);
    for event in iter {
        renderer.feed(event, out);
    }
    renderer.finish(out, start_len);
}
```

- [ ] **Step 5: Re-export from `src-core/src/lib.rs`**

Find the `pub use` block in `lib.rs` and add:
```rust
pub use html::HtmlRenderer;
```

- [ ] **Step 6: Run tests to confirm refactor is correct**

Run: `cargo test --test integration`
Expected: All pass with identical output

- [ ] **Step 7: Commit**

```bash
git add src-core/src/html.rs src-core/src/lib.rs
git commit -m "refactor(html): expose HtmlRenderer struct for event-by-event rendering"
```

---

### Task 3: `src-plugin-types` Shared Types

**Files:**
- Modify: `src-plugin-types/src/lib.rs`

No tests needed for this task — these are plain data types; correctness is verified via usage in later tasks.

- [ ] **Step 1: Write `PluginWarning` and `PluginFrontMatterField`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginWarning {
    pub code: String,
    pub message: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginFrontMatterField {
    pub key: String,
    pub raw: String,
}
```

- [ ] **Step 2: Write `PluginEvent`**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum PluginEvent {
    Start { tag: String },
    End   { tag: String },
    Text  { content: String },
    MathInline { latex: String },
    MathDisplay { latex: String },
    FrontMatter { fields: Vec<PluginFrontMatterField> },
    CardLink { url: String },
    FootnoteRef { number: u32 },
    SoftBreak,
    HardBreak,
    Rule,
}
```

- [ ] **Step 3: Write hook input/output types**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightInput {
    pub lang: String,
    pub code: String,
    pub filename: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightOutput {
    /// Returning `None` signals "I don't handle this — use default".
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CardLinkInput {
    pub url: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CardLinkOutput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    /// Full HTML override. When Some, title/description/image_url are ignored.
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LintRuleInput {
    pub source: String,
    pub markdown: String,
    pub existing_warnings: Vec<PluginWarning>,
    pub events: Vec<PluginEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LintRuleOutput {
    pub warnings: Vec<PluginWarning>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrontMatterInput {
    pub fields: Vec<PluginFrontMatterField>,
    pub source: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrontMatterOutput {
    /// Returning `None` signals "use default rendering".
    pub html: Option<String>,
}
```

- [ ] **Step 4: Verify crate compiles**

Run: `cargo build -p src-plugin-types`
Expected: Compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add src-plugin-types/src/lib.rs
git commit -m "feat(plugin-types): add shared plugin hook I/O types"
```

---

### Task 4: `GlossConfig` and `LintConfig`

**Files:**
- Modify: `src-plugin/src/config.rs`
- Create: `src-cli/tests/plugin_config.rs`

- [ ] **Step 1: Write the failing test**

Create `src-cli/tests/plugin_config.rs`:

```rust
use src_plugin::config::{GlossConfig, LintConfig};
use std::collections::HashMap;

#[test]
fn default_config_enables_all_lint() {
    let cfg = GlossConfig::default();
    assert!(cfg.lint.is_enabled("kanji-no-ruby"));
    assert!(cfg.lint.is_enabled("anything-unknown"));
}

#[test]
fn lint_config_disables_specific_rule() {
    let mut rules = HashMap::new();
    rules.insert("kanji-no-ruby".to_string(), false);
    let lint = LintConfig { rules };
    assert!(!lint.is_enabled("kanji-no-ruby"));
    assert!(lint.is_enabled("ruby-malformed")); // not in map → enabled
}

#[test]
fn config_from_missing_file_returns_default() {
    let cfg = GlossConfig::from_file("/nonexistent/gloss.toml");
    assert!(cfg.plugins.is_empty());
}

#[test]
fn front_matter_override_merges_lint() {
    use src_core::parser::FrontMatterField;
    let mut cfg = GlossConfig::default();
    cfg.lint.rules.insert("kanji-no-ruby".to_string(), true);

    let fields = vec![
        FrontMatterField { key: "plugins", raw: r#"{"lint":{"kanji-no-ruby":false}}"# },
    ];
    let merged = cfg.with_front_matter_override(&fields);
    assert!(!merged.lint.is_enabled("kanji-no-ruby"));
}

#[test]
fn front_matter_override_replaces_plugin_list_when_list_key_present() {
    use src_core::parser::FrontMatterField;
    let cfg = GlossConfig::default();
    let fields = vec![
        FrontMatterField { key: "plugins", raw: r#"{"list":[]}"# },
    ];
    let merged = cfg.with_front_matter_override(&fields);
    assert!(merged.plugins.is_empty());
}

#[test]
fn config_parse_error_returns_default() {
    // Write a temp invalid TOML file, load it, verify default returned
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(f, "not valid toml [[[").unwrap();
    let cfg = GlossConfig::from_file(f.path().to_str().unwrap());
    assert!(cfg.plugins.is_empty()); // fell back to default
}
```

**Note:** Add `tempfile = "3"` to `[dev-dependencies]` in `src-cli/Cargo.toml` for this test.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test plugin_config -p src-cli 2>&1 | head -20`
Expected: FAIL — `src_plugin::config` not found

- [ ] **Step 3: Add `src-plugin` dep to `src-cli/Cargo.toml`**

```toml
[dependencies]
src-core = { path = "../src-core" }
src-plugin = { path = "../src-plugin" }
```

- [ ] **Step 4: Implement `config.rs`**

```rust
use std::collections::HashMap;
use serde::Deserialize;
use src_core::parser::FrontMatterField;

#[derive(Debug, Clone, Default)]
pub struct LintConfig {
    pub rules: HashMap<String, bool>,
}

impl LintConfig {
    /// Returns `true` unless the rule is explicitly set to `false`.
    pub fn is_enabled(&self, code: &str) -> bool {
        self.rules.get(code).copied().unwrap_or(true)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginEntry {
    pub id: String,
    pub path: String,
    pub hooks: Vec<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct GlossConfig {
    pub lint: LintConfig,
    pub plugins: Vec<PluginEntry>,
}

// ── TOML deserialization types ───────────────────────────────────────────────

#[derive(Deserialize)]
struct TomlRoot {
    #[serde(default)]
    lint: TomlLint,
    #[serde(default)]
    plugins: Vec<TomlPlugin>,
}

#[derive(Deserialize, Default)]
struct TomlLint {
    #[serde(default)]
    rules: HashMap<String, bool>,
}

#[derive(Deserialize)]
struct TomlPlugin {
    id: String,
    path: String,
    #[serde(default)]
    hooks: Vec<String>,
    #[serde(default)]
    config: serde_json::Value,
}

const KNOWN_LINT_CODES: &[&str] = &[
    "kanji-no-ruby", "ruby-kana-base", "ruby-kanji-reading", "ruby-katakana-hiragana",
    "ruby-empty-base", "ruby-empty-reading", "ruby-self-referential", "ruby-malformed",
    "anno-looks-like-ruby", "anno-empty-base", "anno-malformed",
    "math-unclosed-inline", "math-unclosed-display",
    "footnote-undefined-ref", "footnote-unused-def",
    "card-non-http", "card-malformed", "card-unknown-type",
];

impl GlossConfig {
    /// Load from a TOML file. Missing file → default. Parse error → stderr + default.
    /// Unknown lint rule keys are printed to stderr and ignored (§5.5).
    pub fn from_file(path: &str) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return Self::default(), // missing file is normal
        };
        match toml::from_str::<TomlRoot>(&text) {
            Ok(root) => {
                // Validate lint rule keys
                for key in root.lint.rules.keys() {
                    if !KNOWN_LINT_CODES.contains(&key.as_str()) {
                        eprintln!("[gloss-plugin] unknown lint rule: {key}");
                    }
                }
                Self {
                    lint: LintConfig { rules: root.lint.rules },
                    plugins: root.plugins.into_iter().map(|p| PluginEntry {
                        id: p.id,
                        path: p.path,
                        hooks: p.hooks,
                        config: p.config,
                    }).collect(),
                }
            },
            Err(e) => {
                eprintln!("[gloss-plugin] config error: {e}");
                Self::default()
            }
        }
    }

    /// Returns a new config with per-file front matter overrides applied.
    /// The `plugins` front matter key must be inline JSON:
    /// `{"lint":{"rule":false},"list":[...]}`
    pub fn with_front_matter_override(&self, fields: &[FrontMatterField<'_>]) -> Self {
        let mut result = self.clone();

        for field in fields {
            if field.key != "plugins" {
                continue;
            }
            let val: serde_json::Value = match serde_json::from_str(field.raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[gloss-plugin] front matter 'plugins' parse error: {e}");
                    continue;
                }
            };

            // Merge lint rules
            if let Some(lint_obj) = val.get("lint").and_then(|v| v.as_object()) {
                for (k, v) in lint_obj {
                    if let Some(enabled) = v.as_bool() {
                        result.lint.rules.insert(k.clone(), enabled);
                    } else {
                        eprintln!("[gloss-plugin] unknown lint rule value for '{k}': expected bool");
                    }
                }
            }

            // Replace plugin list if `list` key present
            if let Some(list) = val.get("list").and_then(|v| v.as_array()) {
                result.plugins = list.iter().filter_map(|entry| {
                    let id = entry.get("id")?.as_str()?.to_string();
                    let path = entry.get("path")?.as_str()?.to_string();
                    let hooks = entry.get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let config = entry.get("config").cloned().unwrap_or(serde_json::Value::Null);
                    Some(PluginEntry { id, path, hooks, config })
                }).collect();
            }
        }

        result
    }
}
```

- [ ] **Step 5: Re-export from `src-plugin/src/lib.rs`**

```rust
pub mod config;
pub mod convert;
pub mod host;
pub mod renderer;
```

- [ ] **Step 6: Run tests**

Run: `cargo test --test plugin_config -p src-cli`
Expected: All 5 tests pass

- [ ] **Step 7: Commit**

```bash
git add src-plugin/src/config.rs src-cli/tests/plugin_config.rs src-cli/Cargo.toml
git commit -m "feat(plugin): implement GlossConfig and LintConfig with front matter override"
```

---

### Task 5: Event/Warning Conversion Utilities

**Files:**
- Modify: `src-plugin/src/convert.rs`
- Test: add to `src-cli/tests/plugin_config.rs` or create `src-cli/tests/plugin_convert.rs`

- [ ] **Step 1: Write the failing tests**

Create `src-cli/tests/plugin_convert.rs`:

```rust
use src_plugin::convert::{to_plugin_events, to_plugin_warnings, tag_to_string};
use src_plugin_types::PluginEvent;
use src_core::parser::{Event, Tag, Warning};

#[test]
fn converts_text_event() {
    let events = vec![Event::Text("hello")];
    let result = to_plugin_events(&events);
    assert_eq!(result.len(), 1);
    matches!(&result[0], PluginEvent::Text { content } if content == "hello");
}

#[test]
fn skips_block_id_events() {
    let events = vec![
        Event::BlockId(42),
        Event::Text("x"),
    ];
    let result = to_plugin_events(&events);
    assert_eq!(result.len(), 1); // BlockId dropped
}

#[test]
fn converts_warnings() {
    let w = Warning {
        code: "kanji-no-ruby",
        message: "kanji without ruby".to_string(),
        source: "test.n.md".to_string(),
        line: 3,
        col: 5,
    };
    let result = to_plugin_warnings(&[w]);
    assert_eq!(result[0].code, "kanji-no-ruby");
    assert_eq!(result[0].line, 3);
}

#[test]
fn tag_to_string_paragraph() {
    assert_eq!(tag_to_string(&Tag::Paragraph), "Paragraph");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test plugin_convert -p src-cli 2>&1 | head -20`
Expected: FAIL — module not found

- [ ] **Step 3: Implement `convert.rs`**

```rust
use src_core::parser::{Event, Tag, Warning, FrontMatterField};
use src_plugin_types::{PluginEvent, PluginFrontMatterField, PluginWarning};

/// Convert a slice of core events to plugin events.
/// `BlockId` events are dropped (internal renderer state, not meaningful to plugins).
pub fn to_plugin_events<'a>(events: &[Event<'a>]) -> Vec<PluginEvent> {
    events.iter().filter_map(|e| to_plugin_event(e)).collect()
}

fn to_plugin_event<'a>(event: &Event<'a>) -> Option<PluginEvent> {
    Some(match event {
        Event::Text(t)          => PluginEvent::Text { content: t.to_string() },
        Event::SoftBreak        => PluginEvent::SoftBreak,
        Event::HardBreak        => PluginEvent::HardBreak,
        Event::Rule             => PluginEvent::Rule,
        Event::MathInline(m)    => PluginEvent::MathInline { latex: m.to_string() },
        Event::MathDisplay(m)   => PluginEvent::MathDisplay { latex: m.to_string() },
        Event::CardLink(url)    => PluginEvent::CardLink { url: url.to_string() },
        Event::FootnoteRef(n)   => PluginEvent::FootnoteRef { number: *n },
        Event::FrontMatter(flds) => PluginEvent::FrontMatter {
            fields: flds.iter().map(|f| PluginFrontMatterField {
                key: f.key.to_string(),
                raw: f.raw.to_string(),
            }).collect(),
        },
        Event::Start(tag)       => PluginEvent::Start { tag: tag_to_string(tag) },
        Event::End(tag)         => PluginEvent::End   { tag: tag_to_string(tag) },
        Event::BlockId(_)       => return None,  // internal — drop
    })
}

/// Convert core `Warning` slice to plugin `PluginWarning` vec.
pub fn to_plugin_warnings(warnings: &[Warning]) -> Vec<PluginWarning> {
    warnings.iter().map(|w| PluginWarning {
        code: w.code.to_string(),
        message: w.message.clone(),
        line: w.line,
        col: w.col,
    }).collect()
}

/// Produce a human-readable string for a `Tag` (used in `PluginEvent`).
pub fn tag_to_string(tag: &Tag) -> String {
    match tag {
        Tag::Paragraph          => "Paragraph".to_string(),
        Tag::Heading(n)         => format!("Heading({n})"),
        Tag::Section(n)         => format!("Section({n})"),
        Tag::List(true)         => "OrderedList".to_string(),
        Tag::List(false)        => "UnorderedList".to_string(),
        Tag::Item               => "Item".to_string(),
        Tag::Code               => "Code".to_string(),
        Tag::CodeBlock(l, f)    => format!("CodeBlock({l},{f})"),
        Tag::Blockquote         => "Blockquote".to_string(),
        Tag::Table(_)           => "Table".to_string(),
        Tag::TableHead          => "TableHead".to_string(),
        Tag::TableRow           => "TableRow".to_string(),
        Tag::TableCell(_)       => "TableCell".to_string(),
        Tag::Strong             => "Strong".to_string(),
        Tag::Emphasis           => "Emphasis".to_string(),
        Tag::Strikethrough      => "Strikethrough".to_string(),
        Tag::Link(_)            => "Link".to_string(),
        Tag::Image(_, _)        => "Image".to_string(),
        Tag::Ruby(_)            => "Ruby".to_string(),
        Tag::Anno(_)            => "Anno".to_string(),
        Tag::AnnoNote           => "AnnoNote".to_string(),
        Tag::FootnoteSection    => "FootnoteSection".to_string(),
        Tag::FootnoteItem(n)    => format!("FootnoteItem({n})"),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test plugin_convert -p src-cli`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add src-plugin/src/convert.rs src-cli/tests/plugin_convert.rs
git commit -m "feat(plugin): add event/warning conversion utilities"
```

---

### Task 6: `GlossPluginHost` — Extism Loading and Hook Dispatch

**Files:**
- Modify: `src-plugin/src/host.rs`

Note: Full integration tests with real WASM binaries are deferred. This task adds unit tests for error handling paths only (no actual plugin loaded).

- [ ] **Step 1: Write failing unit test for empty host**

Add to `src-plugin/src/host.rs` (inline `#[cfg(test)]` module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_host_returns_none_for_all_hooks() {
        let mut host = GlossPluginHost { plugins: vec![] };
        assert!(host.run_code_highlight("rust", "fn main(){}", "", serde_json::Value::Null).is_none());
        assert!(host.run_card_link("https://example.com", serde_json::Value::Null).is_none());
        assert!(host.run_front_matter(&[], "", serde_json::Value::Null).is_none());
    }

    #[test]
    fn empty_host_lint_returns_empty() {
        let mut host = GlossPluginHost { plugins: vec![] };
        let result = host.run_lint_rule("test.n.md", "# hi", &[], &[]);
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p src-plugin 2>&1 | head -20`
Expected: FAIL — `GlossPluginHost` not defined

- [ ] **Step 3: Implement `host.rs`**

```rust
use extism::{Plugin, Manifest, Wasm};
use src_plugin_types::{
    CodeHighlightInput, CodeHighlightOutput,
    CardLinkInput, CardLinkOutput,
    LintRuleInput, LintRuleOutput,
    FrontMatterInput, FrontMatterOutput,
    PluginWarning, PluginFrontMatterField, PluginEvent,
};
use crate::convert::to_plugin_events;
use src_core::parser::Event;

pub struct LoadedPlugin {
    pub id: String,
    pub hooks: Vec<String>,
    pub config: serde_json::Value,
    pub instance: Plugin,
}

pub struct GlossPluginHost {
    pub plugins: Vec<LoadedPlugin>,
}

impl GlossPluginHost {
    /// Create a new host. Plugins that fail to load print an error and are skipped.
    ///
    /// Security settings per spec §6.1:
    /// - WASI filesystem/network access disabled
    /// - 16 MB memory limit per plugin
    /// - `host_log` host function exposed (plugin writes → stderr)
    pub fn new(entries: &[crate::config::PluginEntry]) -> Self {
        // host_log: plugins call this to write debug messages to stderr
        let host_log = extism::host_fn!("host_log", (msg: String) {
            eprintln!("[plugin] {msg}");
        });

        let mut plugins = Vec::new();
        for entry in entries {
            let wasm = Wasm::file(&entry.path);
            let manifest = Manifest::new([wasm])
                .with_memory_max(16 * 1024 * 1024 / 65536); // 16 MB in Wasm pages (64 KiB each)
            match Plugin::new(manifest, [host_log.clone()], true) {
                Ok(instance) => plugins.push(LoadedPlugin {
                    id: entry.id.clone(),
                    hooks: entry.hooks.clone(),
                    config: entry.config.clone(),
                    instance,
                }),
                Err(e) => {
                    eprintln!("[gloss-plugin:{}] load failed: {e}", entry.id);
                }
            }
        }
        Self { plugins }
    }

    /// `code-highlight` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_code_highlight(
        &mut self,
        lang: &str,
        code: &str,
        filename: &str,
        config: serde_json::Value,
    ) -> Option<String> {
        let input = CodeHighlightInput {
            lang: lang.to_string(),
            code: code.to_string(),
            filename: filename.to_string(),
            config,
        };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "code-highlight") { continue; }
            match p.instance.call::<_, String>("code_highlight", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<CodeHighlightOutput>(&raw) {
                        Ok(out) => if out.html.is_some() { return out.html; }
                        Err(e) => eprintln!("[gloss-plugin:{}] code-highlight parse error: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] code-highlight failed: {e}", p.id),
            }
        }
        None
    }

    /// `card-link` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_card_link(
        &mut self,
        url: &str,
        config: serde_json::Value,
    ) -> Option<CardLinkOutput> {
        let input = CardLinkInput { url: url.to_string(), config };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "card-link") { continue; }
            match p.instance.call::<_, String>("card_link", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<CardLinkOutput>(&raw) {
                        Ok(out) => if out.html.is_some() || out.title.is_some() || out.description.is_some() || out.image_url.is_some() { return Some(out); }
                        Err(e) => eprintln!("[gloss-plugin:{}] card-link parse error: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] card-link failed: {e}", p.id),
            }
        }
        None
    }

    /// `lint-rule` hook — all plugins run, warnings merged.
    pub fn run_lint_rule<'a>(
        &mut self,
        source: &str,
        markdown: &str,
        existing_warnings: &[src_plugin_types::PluginWarning],
        events: &[Event<'a>],
    ) -> Vec<PluginWarning> {
        let plugin_events = to_plugin_events(events);
        let input = LintRuleInput {
            source: source.to_string(),
            markdown: markdown.to_string(),
            existing_warnings: existing_warnings.to_vec(),
            events: plugin_events,
        };
        let json = match serde_json::to_string(&input) {
            Ok(j) => j,
            Err(_) => return vec![],
        };
        let mut all_warnings = Vec::new();
        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "lint-rule") { continue; }
            match p.instance.call::<_, String>("lint_rule", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<LintRuleOutput>(&raw) {
                        Ok(out) => all_warnings.extend(out.warnings),
                        Err(e) => eprintln!("[gloss-plugin:{}] lint-rule parse error: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] lint-rule failed: {e}", p.id),
            }
        }
        all_warnings
    }

    /// `front-matter` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_front_matter(
        &mut self,
        fields: &[PluginFrontMatterField],
        source: &str,
        config: serde_json::Value,
    ) -> Option<String> {
        let input = FrontMatterInput {
            fields: fields.to_vec(),
            source: source.to_string(),
            config,
        };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "front-matter") { continue; }
            match p.instance.call::<_, String>("front_matter", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<FrontMatterOutput>(&raw) {
                        Ok(out) => if out.html.is_some() { return out.html; }
                        Err(e) => eprintln!("[gloss-plugin:{}] front-matter parse error: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] front-matter failed: {e}", p.id),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_host_returns_none_for_all_hooks() {
        let mut host = GlossPluginHost { plugins: vec![] };
        assert!(host.run_code_highlight("rust", "fn main(){}", "", serde_json::Value::Null).is_none());
        assert!(host.run_card_link("https://example.com", serde_json::Value::Null).is_none());
        assert!(host.run_front_matter(&[], "", serde_json::Value::Null).is_none());
    }

    #[test]
    fn empty_host_lint_returns_empty() {
        let mut host = GlossPluginHost { plugins: vec![] };
        let result = host.run_lint_rule("test.n.md", "# hi", &[], &[]);
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p src-plugin`
Expected: Both unit tests pass

- [ ] **Step 5: Commit**

```bash
git add src-plugin/src/host.rs
git commit -m "feat(plugin): implement GlossPluginHost with all four hook dispatchers"
```

---

### Task 7: `PluginAwareRenderer`

**Files:**
- Modify: `src-plugin/src/renderer.rs`
- Create: `src-cli/tests/plugin_renderer.rs`

The renderer collects events into a `Vec<Event<'_>>` (from a `Parser`), then iterates by index, intercepting specific events before delegating to `HtmlRenderer`.

- [ ] **Step 1: Write the failing test**

Create `src-cli/tests/plugin_renderer.rs`:

```rust
use src_core::parser::Parser;
use src_plugin::config::GlossConfig;
use src_plugin::host::GlossPluginHost;
use src_plugin::renderer::PluginAwareRenderer;

fn render_no_plugins(markdown: &str) -> String {
    let parser = Parser::new_with_source(markdown, "test.n.md");
    let events: Vec<_> = parser.collect();
    let cfg = GlossConfig::default();
    let mut host = GlossPluginHost { plugins: vec![] };
    let mut out = String::new();
    let mut renderer = PluginAwareRenderer::new(&mut host, &cfg);
    renderer.render(&events, &mut out, "test.n.md", markdown);
    out
}

#[test]
fn renders_paragraph_without_plugins() {
    let html = render_no_plugins("Hello, world.");
    assert!(html.contains("<p>"), "got: {html}");
    assert!(html.contains("Hello, world."), "got: {html}");
}

#[test]
fn renders_code_block_without_plugins() {
    let html = render_no_plugins("```rust\nfn main() {}\n```");
    assert!(html.contains("nm-code"), "got: {html}");
    assert!(html.contains("fn main()"), "got: {html}");
}

#[test]
fn renders_card_link_without_plugins() {
    let html = render_no_plugins("[[https://example.com]]");
    assert!(html.contains("nm-card-link"), "got: {html}");
    assert!(html.contains("example.com"), "got: {html}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test plugin_renderer -p src-cli 2>&1 | head -20`
Expected: FAIL — module not found

- [ ] **Step 3: Implement `renderer.rs`**

```rust
use src_core::html::HtmlRenderer;
use src_core::parser::{Event, Tag, FrontMatterField};
use src_plugin_types::{PluginFrontMatterField, CardLinkOutput};
use crate::host::GlossPluginHost;
use crate::config::GlossConfig;
use crate::convert::{to_plugin_warnings, to_plugin_events};

pub struct PluginAwareRenderer<'host> {
    host: &'host mut GlossPluginHost,
    cfg: &'host GlossConfig,
}

impl<'host> PluginAwareRenderer<'host> {
    pub fn new(host: &'host mut GlossPluginHost, cfg: &'host GlossConfig) -> Self {
        Self { host, cfg }
    }

    /// Render `events` (collected from a `Parser`) into HTML.
    /// `source` and `markdown` are passed to lint-rule plugins.
    /// Note: front matter config override is applied by the caller (main.rs) before
    /// constructing `PluginAwareRenderer`, so `self.cfg` is already the effective config.
    pub fn render<'a>(
        &mut self,
        events: &[Event<'a>],
        out: &mut String,
        source: &str,
        markdown: &str,
    ) {
        let start_len = out.len();
        let mut renderer = HtmlRenderer::new(false);
        let mut i = 0;

        while i < events.len() {
            match &events[i] {
                // ── front-matter hook ─────────────────────────────────────
                Event::FrontMatter(fields) => {
                    let pfields: Vec<PluginFrontMatterField> = fields.iter().map(|f| {
                        PluginFrontMatterField { key: f.key.to_string(), raw: f.raw.to_string() }
                    }).collect();
                    let result = self.host.run_front_matter(
                        &pfields,
                        source,
                        serde_json::Value::Null,
                    );
                    if let Some(html) = result {
                        out.push_str(&html);
                    } else {
                        renderer.feed(events[i].clone(), out);
                    }
                    i += 1;
                }

                // ── code-highlight hook ───────────────────────────────────
                Event::Start(Tag::CodeBlock(lang, filename)) => {
                    let lang = lang.to_string();
                    let filename = filename.to_string();
                    // Collect everything until End(CodeBlock)
                    let code_start = i;
                    i += 1;
                    let mut code_text = String::new();
                    while i < events.len() {
                        if let Event::End(Tag::CodeBlock(_, _)) = &events[i] {
                            i += 1;
                            break;
                        }
                        if let Event::Text(t) = &events[i] {
                            code_text.push_str(t);
                        }
                        i += 1;
                    }

                    let result = self.host.run_code_highlight(
                        &lang,
                        &code_text,
                        &filename,
                        serde_json::Value::Null,
                    );
                    if let Some(html) = result {
                        out.push_str(&html);
                    } else {
                        // Replay the original events through HtmlRenderer
                        for j in code_start..i {
                            renderer.feed(events[j].clone(), out);
                        }
                    }
                }

                // ── card-link hook ────────────────────────────────────────
                Event::CardLink(url) => {
                    let url = url.to_string();
                    let result = self.host.run_card_link(&url, serde_json::Value::Null);
                    if let Some(card_out) = result {
                        out.push_str(&render_card_output(&url, card_out));
                    } else {
                        renderer.feed(events[i].clone(), out);
                    }
                    i += 1;
                }

                // ── default ───────────────────────────────────────────────
                _ => {
                    renderer.feed(events[i].clone(), out);
                    i += 1;
                }
            }
        }

        renderer.finish(out, start_len);
    }

}

/// Render a `CardLinkOutput` to HTML.
/// Priority: `html` (full override) > `title`/`description`/`image_url` (structured) > plain URL fallback.
fn render_card_output(url: &str, out: CardLinkOutput) -> String {
    use src_core::html::escape_html;

    // Priority 1: full HTML override
    if let Some(html) = out.html {
        return html;
    }

    // Priority 2: structured metadata
    if out.title.is_some() || out.description.is_some() || out.image_url.is_some() {
        let mut s = format!(
            "<a href=\"{url}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\">",
            url = escape_html(url)
        );
        if let Some(img) = out.image_url {
            s.push_str(&format!("<img src=\"{}\" class=\"nm-card-img\" alt=\"\">", escape_html(&img)));
        }
        s.push_str("<span class=\"nm-card-body\">");
        if let Some(title) = out.title {
            s.push_str(&format!("<span class=\"nm-card-title\">{}</span>", escape_html(&title)));
        }
        if let Some(desc) = out.description {
            s.push_str(&format!("<span class=\"nm-card-desc\">{}</span>", escape_html(&desc)));
        }
        s.push_str(&format!(
            "<span class=\"nm-card-url\">{}</span></span></a>\n",
            escape_html(url)
        ));
        return s;
    }

    // Priority 3: plain URL fallback (same as HtmlRenderer default)
    let escaped = escape_html(url);
    format!(
        "<a href=\"{escaped}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">{escaped}</span></a>\n"
    )
}
```

- [ ] **Step 4: Ensure `Event` is `Clone`**

Check `src-core/src/parser.rs` for `#[derive(Clone)]` on `Event` and `Tag`. If missing, add it. (Note: `src-core` is `no_std`; `derive(Clone)` is fine.)

Run: `grep -n "derive.*Clone" src-core/src/parser.rs`
If the output doesn't show `Event` and `Tag` having `Clone`, add `Clone` to their derives.

- [ ] **Step 5: Run tests**

Run: `cargo test --test plugin_renderer -p src-cli`
Expected: All 3 tests pass

- [ ] **Step 6: Also run all integration tests to confirm no regressions**

Run: `cargo test --test integration -p src-core`
Expected: All pass

- [ ] **Step 7: Commit**

```bash
git add src-plugin/src/renderer.rs src-cli/tests/plugin_renderer.rs
git commit -m "feat(plugin): implement PluginAwareRenderer with hook interception"
```

---

### Task 8: `src-cli` Integration

**Files:**
- Modify: `src-cli/src/main.rs`
- Test: add to `src-cli/tests/plugin_renderer.rs` (CLI smoke test)

- [ ] **Step 1: Write a CLI-level integration test**

Add to `src-cli/tests/plugin_renderer.rs`:

```rust
#[test]
fn renderer_applies_lint_config_from_default_config() {
    // With default config (no plugins, all lint enabled), run should produce no crash
    let markdown = "# Hello\n\nWorld.";
    let html = render_no_plugins(markdown);
    assert!(html.contains("<h1>"));
    assert!(html.contains("World."));
}
```

- [ ] **Step 2: Modify `src-cli/src/main.rs` to use plugin system**

Replace the existing `main.rs` body with:

```rust
use std::env;
use std::fs;
use std::process;
use src_core::parser::Parser;
use src_plugin::config::GlossConfig;
use src_plugin::host::GlossPluginHost;
use src_plugin::renderer::PluginAwareRenderer;

// HTML_HEAD and HTML_TAIL constants remain unchanged

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.n.md> [output.html]", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = /* same logic as before */;

    let text = match fs::read_to_string(input_path) {
        Ok(t) => t,
        Err(e) => { eprintln!("Error reading file {input_path}: {e}"); process::exit(1); }
    };

    // Load config from gloss.toml in current dir (missing file → default)
    let cfg = GlossConfig::from_file("gloss.toml");

    // Parse
    let source = input_path.to_string();
    let parser = Parser::new_with_source(&text, &source);

    // Print lint warnings from core parser
    if !parser.warnings.is_empty() {
        for w in &parser.warnings {
            eprintln!("\x1b[33m[{}:{}:{}] {} — {}\x1b[0m",
                w.source, w.line, w.col, w.code, w.message);
        }
    }

    // Apply front matter override to config
    let events: Vec<_> = parser.into_iter().collect();
    // (parser.warnings already printed above; collect events fresh)

    // Re-parse to get events (parser is consumed after .warnings access)
    // NOTE: Parser::new_with_source returns a Parser that IS an iterator.
    // We need warnings AND events. Collect both from a single parse.
    // Restructure: parse → collect events + warnings together.

    let text2 = text.clone();
    let parser2 = Parser::new_with_source(&text2, &source);
    let warnings = parser2.warnings.clone();
    let events: Vec<_> = parser2.collect();

    for w in &warnings {
        eprintln!("\x1b[33m[{}:{}:{}] {} — {}\x1b[0m",
            w.source, w.line, w.col, w.code, w.message);
    }

    // Apply front matter config override
    let fm_fields: Vec<_> = events.iter().filter_map(|e| {
        if let src_core::parser::Event::FrontMatter(fields) = e {
            Some(fields.as_slice())
        } else {
            None
        }
    }).next().unwrap_or(&[]).to_vec();
    let effective_cfg = cfg.with_front_matter_override(&fm_fields);

    // Build plugin host
    let mut host = GlossPluginHost::new(&effective_cfg.plugins);

    // Run lint-rule plugins
    let plugin_warnings = host.run_lint_rule(
        &source,
        &text,
        &src_plugin::convert::to_plugin_warnings(&warnings),
        &events,
    );
    for w in &plugin_warnings {
        eprintln!("\x1b[33m[plugin:{}:{}] {} — {}\x1b[0m",
            w.line, w.col, w.code, w.message);
    }

    // Render
    let mut html_body = String::new();
    let mut renderer = PluginAwareRenderer::new(&mut host, &effective_cfg);
    renderer.render(&events, &mut html_body, &source, &text);

    let final_html = format!("{}{}{}", HTML_HEAD, html_body, HTML_TAIL);

    if let Err(e) = fs::write(&output_path, final_html) {
        eprintln!("Error writing output file {output_path}: {e}");
        process::exit(1);
    }

    println!("Successfully compiled {input_path} -> {output_path}");
}
```

**Important implementation note:** `Parser::new_with_source` stores `warnings` as a field populated eagerly. Check `parser.rs` to confirm whether `warnings` is populated before or during iteration. If it's populated during iteration, you must collect the iterator before accessing warnings. The correct pattern:

```rust
let parser = Parser::new_with_source(&text, &source);
// If warnings are populated during iteration, collect first:
let events: Vec<_> = parser.collect_with_warnings(); // check actual API
// OR:
let mut events = Vec::new();
for event in parser { events.push(event); }
// Then access parser.warnings — but parser is moved.
// Better: Parser::new returns (events, warnings) or Parser has a method.
// Check the actual API in parser.rs before implementing.
```

Read `src-core/src/parser.rs` lines 1-50 to understand the `Parser` struct fields and confirm how to access both warnings and events from one parse.

- [ ] **Step 3: Run the new test**

Run: `cargo test --test plugin_renderer -p src-cli`
Expected: All tests pass including the new one

- [ ] **Step 4: Run a manual smoke test**

Run: `cargo run -p src-cli -- web-playground/sample.n.md /tmp/out.html`
Expected: "Successfully compiled ..." printed, output file created with valid HTML

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All tests in all crates pass

- [ ] **Step 6: Commit**

```bash
git add src-cli/src/main.rs src-cli/tests/plugin_renderer.rs
git commit -m "feat(cli): integrate PluginAwareRenderer and GlossPluginHost into CLI pipeline"
```

---

## Implementation Notes

### Parser API Clarification (Task 8)

Before writing Task 8's `main.rs`, read `src-core/src/parser.rs` to confirm:
- Is `Parser.warnings` a `Vec<Warning>` field populated during `new_with_source`?
- Or is it populated lazily during iteration?

The correct integration depends on this. If warnings are populated at construction (before iteration), the simple pattern works. If lazy, collect events first.

### `no_std` Constraint

`src-core` is `#![no_std]` — only `alloc` is available. The `HtmlRenderer` refactor (Task 2) must not add any `std` imports to `html.rs`. Use `alloc::string::String`, `alloc::vec::Vec`, `alloc::format!` only.

### Error Display Format

All plugin errors must follow `[gloss-plugin(:ID)] <message>` format as specified in the spec §5.5. The `:ID` suffix is included when a specific plugin's ID is known.

The `host_log` host function (exposed to WASM plugins) should log as `[plugin:ID] <msg>`. The plan skeleton shows `[plugin] {msg}` — during implementation, pass the plugin ID into the closure or use a per-plugin registration so the ID is available. This is an Extism API detail; check the Extism v1 docs for how to pass context into host functions.

### `Event` Clone Requirement

`PluginAwareRenderer` needs to replay code block events when a plugin returns `None`. This requires `Event<'a>: Clone`. Verify this is derived in `parser.rs` before Task 7. If not, add `Clone` to `Event` and `Tag` derives — this is a safe, additive change.

### Card Link Partial Output (§4.2)

The `render_card_output` function implements the priority rule from the spec:
1. If `html` is `Some` → use it verbatim
2. Else if any of `title`/`description`/`image_url` are `Some` → build structured card HTML
3. Else → fall back to plain URL anchor (same as default)

This means a plugin can return `CardLinkOutput { title: Some("..."), ..Default::default() }` without providing `html` and get a reasonable card rendering.
