# Gloss Plugin System Design

**Date:** 2026-03-24
**Status:** Draft v3
**Scope:** CLI (`src-cli`) + shared plugin infrastructure; Tauri (`src-desktop`) future use

---

## 1. Goals

- Allow users to customize code block rendering, card link display, lint rules, and front matter HTML without modifying `src-core`.
- Keep `src-core` `#![no_std]` and dependency-free (one additive refactor required — see §6.3).
- Plugins are WebAssembly modules loaded via [Extism](https://extism.org/), providing language-agnostic extensibility and sandboxing.
- Simple customization (lint ON/OFF) via `gloss.toml` config file without requiring a WASM plugin.
- Tauri (`src-desktop`) can reuse the plugin infrastructure by depending on `src-plugin` directly.

---

## 2. Crate Structure

```
gloss/ (workspace)
├── src-core           #![no_std] — parser + HTML generator; one additive refactor (§6.3)
├── src-plugin-types   new — serde-compatible shared types (no Extism dep)
├── src-plugin         new — Extism host, gloss.toml loader, hook runner
├── src-cli            extended — depends on src-plugin
└── src-web            unchanged
```

`src-plugin-types` has no Extism dependency so plugin authors can depend on it from their PDK crate without pulling in wasmtime.

---

## 3. Configuration

### 3.1 `gloss.toml` (project-wide)

Searched for in the current working directory when `src-cli` runs. Missing file → all built-in defaults apply, no error.

```toml
[lint]
# All existing lint rule codes — set to false to disable.
# Default for every key is true.
kanji-no-ruby          = true
ruby-kana-base         = true
ruby-kanji-reading     = true
ruby-katakana-hiragana = true
ruby-empty-base        = true
ruby-empty-reading     = true
ruby-self-referential  = true
ruby-malformed         = true
anno-looks-like-ruby   = true
anno-empty-base        = true
anno-malformed         = true
math-unclosed-inline   = true
math-unclosed-display  = true
footnote-undefined-ref = true
footnote-unused-def    = true
card-non-http          = true
card-malformed         = true
card-unknown-type      = true

[[plugins]]
id    = "shiki"
path  = "plugins/shiki.wasm"
hooks = ["code-highlight"]
[plugins.config]
theme = "tokyo-night"

[[plugins]]
id    = "ogp-card"
path  = "plugins/ogp-card.wasm"
hooks = ["card-link"]
[plugins.config]
timeout_ms = 3000
cache_dir  = ".gloss-cache"

[[plugins]]
id    = "my-lint"
path  = "plugins/my-lint.wasm"
hooks = ["lint-rule"]

[[plugins]]
id    = "my-frontmatter"
path  = "plugins/my-frontmatter.wasm"
hooks = ["front-matter"]
```

### 3.2 Front Matter Override (per-file)

Front matter `plugins` key overrides `gloss.toml` for that file only.

```yaml
---
date: "2024-09-20"
tags: ["Rust"]
plugins:
  lint:
    kanji-no-ruby: false
  list:
    - id: "shiki"
      hooks: ["code-highlight"]
---
```

**Semantics:**
- `plugins.lint` keys are merged with `gloss.toml`'s `[lint]` (front matter wins on conflict).
- `plugins.list`, when present, **replaces** the global plugin list entirely for this file. Omitting `plugins.list` inherits the global list unchanged.
- Per-plugin `config` stanzas cannot be overridden from front matter; they are always taken from `gloss.toml`. This keeps front matter lightweight and avoids deep-merge complexity.

**Config precedence:** front matter > gloss.toml > built-in defaults.

---

## 4. Plugin Hooks

All communication between host and plugin is JSON over Extism's memory model. Each hook function has a defined name the WASM module must export.

### 4.1 `code-highlight`

**Purpose:** Render a fenced code block. Enables syntax highlighting, SVG rendering, Mermaid diagrams, etc.

**WASM export name:** `code_highlight`

```json
// Input
{
  "lang":     "rust",
  "code":     "fn main() { println!(\"hello\"); }",
  "filename": "main.rs",
  "config":   { "theme": "tokyo-night" }
}

// Output
// html: null  → fall back to src-core's default rendering for this block
// html: "..."  → use verbatim (plugin owns the full <pre>/<code> structure)
{ "html": "<pre class=\"shiki\">...</pre>" }
```

**`lang` is `""` when the fenced code block has no language specifier.** The host still calls registered `code-highlight` plugins; plugins that cannot handle an empty lang should return `{ "html": null }`.

**Fallback:** If no plugin is registered, all return `null`, or the WASM call fails (error displayed per §5.5), `src-core`'s default HTML is used.
**Multiple plugins:** First plugin returning non-null `html` wins; remaining plugins for this hook are skipped for that block.

### 4.2 `card-link`

**Purpose:** Enrich a card link with metadata (OGP, etc.) and/or fully custom HTML.

**WASM export name:** `card_link`

```json
// Input
{
  "url":    "https://example.com/article",
  "config": { "timeout_ms": 3000, "cache_dir": ".gloss-cache" }
}

// Output
{
  "title":       "Example Article",
  "description": "A short description.",
  "image_url":   "https://example.com/og.png",
  "html":        null
}
```

**Rendering logic (in priority order):**
1. If `html` is non-null → use it verbatim.
2. Else if any of `title` / `description` / `image_url` is non-null → render available fields into the default `nm-card-link` template; missing fields are omitted without error.
3. Else (all fields null) → use the existing plain URL card fallback.

**Fallback on error:** WASM call failure → error displayed per §5.5 → plain URL card rendered.
**Multiple plugins:** First non-null result wins (same first-wins rule as code-highlight).

### 4.3 `lint-rule`

**Purpose:** Add custom lint warnings beyond `src-core`'s built-in rules.

**WASM export name:** `lint_rule`

```json
// Input
{
  "source":   "playground.n.md",
  "markdown": "全文テキスト...",
  "existing_warnings": [
    { "code": "kanji-no-ruby", "message": "...", "line": 3, "col": 5 }
  ],
  "events": [
    { "type": "Start",    "data": { "tag": "Paragraph" } },
    { "type": "Text",     "data": { "content": "hello" } },
    { "type": "CardLink", "data": { "url": "https://example.com" } },
    { "type": "FootnoteRef", "data": { "number": 1 } }
  ]
}

// Output
{
  "warnings": [
    { "code": "my-rule", "message": "説明", "line": 1, "col": 1 }
  ]
}
```

**All** registered `lint-rule` plugins run; their warnings are merged with built-in warnings after built-in lint filtering (`[lint]` config) has been applied.

**Error handling:** See §5.5. Failures are displayed and the plugin's warnings are dropped; remaining plugins continue to run.

### 4.4 `front-matter`

**Purpose:** Custom rendering of the front matter metadata block.

**WASM export name:** `front_matter`

```json
// Input
{
  "fields": [
    { "key": "date", "raw": "\"2024-09-20\"" },
    { "key": "tags", "raw": "[\"Rust\", \"Gloss\"]" },
    { "key": "type", "raw": "\"Article\"" }
  ],
  "source": "my-post.n.md",
  "config": {}
}

// Output
// html: null  → fall back to built-in nm-frontmatter block
// html: "..."  → use verbatim
{ "html": "<div class=\"my-fm\">...</div>" }
```

**Fallback:** `null` output or error (displayed per §5.5) → existing `nm-frontmatter` rendering in `html.rs`.
**Multiple plugins:** First non-null `html` wins.

---

## 5. Data Types (`src-plugin-types`)

```rust
// All structs derive Serialize + Deserialize (and Clone, Debug).
// No Extism dependency.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginWarning {
    pub code: String,       // copied from Warning::code (&'static str → String)
    pub message: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginFrontMatterField {
    pub key: String,
    pub raw: String,
}

/// Mirror of src-core's Event enum — lifetime-erased, serde-compatible.
/// BlockId is intentionally excluded (internal renderer detail, not meaningful to plugins).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum PluginEvent {
    Start       { tag: String },
    End         { tag: String },
    Text        { content: String },
    MathInline  { latex: String },
    MathDisplay { latex: String },
    FrontMatter { fields: Vec<PluginFrontMatterField> },
    CardLink    { url: String },
    FootnoteRef { number: u32 },
    SoftBreak,
    HardBreak,
    Rule,
}

// ── Hook I/O structs ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightInput {
    pub lang: String,      // "" when no language specifier
    pub code: String,      // raw (unescaped) source text
    pub filename: String,  // "" when no filename label
    pub config: serde_json::Value,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightOutput {
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
    pub html: Option<String>,
}
```

---

## 5.5 Error Policy

All plugin errors — whether from configuration, loading, or hook execution — follow a single rule:

> **Display the error, then fall back to default behaviour. Never abort rendering.**

This makes problems debuggable without breaking the document output.

### Error categories and display format

| Category | When | Display | Fallback |
|---|---|---|---|
| Config parse error | `gloss.toml` has invalid TOML | `[gloss-plugin] config error: <detail>` to stderr | Use built-in defaults, load no plugins |
| Plugin load error | WASM file not found, invalid WASM, memory limit exceeded | `[gloss-plugin:ID] load failed: <detail>` to stderr | Skip that plugin for entire run |
| Hook call error | WASM trap, timeout, non-UTF-8 output, JSON decode failure | `[gloss-plugin:ID] <hook> failed: <detail>` to stderr | Treat as `null` output → fallback rendering |
| Config key unknown | `[lint]` contains unrecognised rule code | `[gloss-plugin] unknown lint rule: <key>` to stderr | Ignore unknown key, continue |

**Key behaviours:**
- Config errors and load errors are reported once at startup, before any rendering begins.
- Hook call errors are reported per-invocation (once per failing block/element).
- All error lines are prefixed `[gloss-plugin]` for easy filtering.
- The `host_log` host function (available to plugin WASM) uses the same format: `[plugin:ID] <msg>`.
- In future Tauri (`src-desktop`) use, these messages are surfaced in the GUI's warning panel rather than stderr.

### Updated lint-rule isolation (replaces §4.3 "Error isolation" paragraph)

If a `lint-rule` plugin call fails, its warnings are dropped **and** a `[gloss-plugin:ID] lint_rule failed: <reason>` line is written to stderr. Processing continues with remaining plugins. This replaces the previous "silently dropped" wording.

---

## 6. Host Implementation (`src-plugin`)

### 6.1 Key responsibilities

1. Parse `gloss.toml` (using `toml` crate); return `GlossConfig::default()` if file absent.
2. Load each WASM plugin via Extism `Plugin::new()`.
3. Security: WASI filesystem/network access disabled by default; memory limit 16 MB per plugin.
4. Expose host function `host_log(msg: String)` so plugins can write debug lines to stderr.
5. Run hooks in registration order; apply first-wins or merge semantics per hook type.

### 6.2 Integration flow in `src-cli`

```
1.  Load gloss.toml → GlossConfig
2.  Load plugins → GlossPluginHost
3.  Parse .n.md with Parser::new_with_source() → event Vec + built-in warnings
4.  Convert src-core Warning → PluginWarning (Warning::code: &'static str → String::from)
5.  Apply [lint] disabled-rule filter to built-in PluginWarnings
6.  Convert event Vec → Vec<PluginEvent> (drop BlockId; map remainder)
7.  Run all lint-rule plugins → merge additional PluginWarnings
8.  Render HTML via PluginAwareRenderer (see §6.3), which calls:
       • front-matter hook for FrontMatter events
       • code-highlight hook for CodeBlock events
       • card-link hook for CardLink events
       • src-core HtmlRenderer for all other events
9.  Print PluginWarnings to stderr; write HTML to output file
```

### 6.3 `PluginAwareRenderer` and the `HtmlRenderer` refactor

`push_html_inner` in `src-core/src/html.rs` is currently a single monolithic function with all state as local variables. Intercepting individual events from outside is not possible without duplicating that logic.

**Required change to `src-core`:** Refactor `push_html_inner` into a public `HtmlRenderer` struct with a `feed(&mut self, event: Event, out: &mut String)` method. All existing state (`in_thead`, `in_anno`, `pending_bid`, `pending_fm`, etc.) becomes struct fields. The existing `push_html` and `push_html_with_ids` functions become thin wrappers that construct an `HtmlRenderer` and call `feed` in a loop — **no behaviour change, no public API break**.

This is an additive change to `src-core`. `#![no_std]` is preserved.

`PluginAwareRenderer` (in `src-plugin`) then:
- Creates an `HtmlRenderer` instance.
- Iterates the event Vec.
- On `FrontMatter`: calls the front-matter hook; if non-null, writes the HTML directly; if null, calls `renderer.feed(FrontMatter(..))` for default rendering.
- On `Start(CodeBlock)` … `End(CodeBlock)`: collects the inner `Text` events to reconstruct the code string, calls the code-highlight hook; if non-null, writes the HTML directly and skips `renderer.feed` for those events; if null, replays the collected events through `renderer.feed`.
- On `CardLink`: calls the card-link hook; renders per §4.2 logic; skips `renderer.feed`.
- All other events: passed directly to `renderer.feed`.

---

## 7. Testing Strategy

### Unit tests (`src-plugin`)
- Config parsing: valid `gloss.toml`; missing file returns defaults without error; invalid TOML returns descriptive error.
- Lint filter: warnings with disabled codes are removed; enabled codes are kept.
- Front matter override merging: per-file `lint` keys merge with global; per-file `list` replaces global list; absent `list` inherits global.
- `Warning` → `PluginWarning` conversion round-trips correctly.

### Integration tests (`src-plugin`, `src-cli`)
- Minimal fixture WASM plugins compiled from Rust test helpers (one per hook).
- `code-highlight`: plugin returns HTML → appears in output; plugin returns null → default used; lang="" still calls plugin.
- `card-link`: fields-only output → default template with available fields; html output → verbatim; all-null → plain URL fallback.
- `lint-rule`: plugin adds warnings → merged in output; plugin panics → dropped + host stderr message; multiple plugins → all run and merge.
- `front-matter`: plugin returns HTML → replaces nm-frontmatter; plugin returns null → default rendered.

### Negative / error tests (all follow §5.5 error policy)
- Invalid `gloss.toml` TOML → `[gloss-plugin] config error:` to stderr; rendering proceeds with built-in defaults.
- Unknown `[lint]` rule key → `[gloss-plugin] unknown lint rule:` to stderr; key ignored; other rules applied.
- Plugin WASM file not found → `[gloss-plugin:ID] load failed:` to stderr at startup; plugin skipped for entire run.
- Plugin exports wrong function name → treated as null output; fallback rendering used; no error (hook not implemented is valid).
- Plugin WASM trap / panic → `[gloss-plugin:ID] <hook> failed:` to stderr; fallback rendering used.
- Plugin returns non-UTF-8 bytes → same as trap above.
- Plugin returns syntactically valid JSON but missing required fields → treated as null output; fallback used.
- All errors: rendering output is still produced (never abort on plugin failure).

### Snapshot tests
- Extend `src-core/tests/html/` if `HtmlRenderer` refactor changes any output.
- Add `src-cli/tests/` for end-to-end rendering with plugin fixtures; BLESS workflow same as `src-core`.

---

## 8. Future: `src-desktop` (Tauri)

`src-desktop` depends on `src-plugin` the same way `src-cli` does. `GlossPluginHost` and `PluginAwareRenderer` are instantiated in Tauri's async command handlers. Plugins are loaded from the user's project directory. No architectural changes needed to `src-plugin`.

---

## 9. Out of Scope

- `src-web` (WASM-on-WASM is impractical).
- `event-filter` hook (not needed for stated use cases; can be added later).
- Plugin marketplace or remote plugin loading.
- Plugin versioning / compatibility checks (deferred).
- Overriding per-plugin `config` from front matter (deferred; use `gloss.toml` for config).
