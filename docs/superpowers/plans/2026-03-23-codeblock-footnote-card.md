# Code Block Header / Footnote / Card Link Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three new syntax extensions to the Gloss parser: (1) `rust:filename` info string on fenced code blocks renders a language+filename header, (2) `[^id]` / `[^id]: content` footnote syntax, (3) `@[card](URL)` block-level card link.

**Architecture:** All changes are in `src-core`. The parser (`parser.rs`) gains new `Tag`/`Event` variants and extended parsing logic; the renderer (`html.rs`) gains matching HTML emitters. CSS for new classes is added to both the CLI embedded stylesheet (`src-cli/src/main.rs`) and the web playground stylesheet (`web-playground/src/style.css`). Footnotes require a two-pass approach: pre-scan collects definitions, then `parse_blocks`/`parse_inline` emit numbered refs, and a footnote section is appended after all blocks.

**Tech Stack:** Rust `#![no_std]` + `alloc`, `wasm-bindgen` (web), Trunk (bundler). Tests via `cargo test --test integration`.

---

## File Map

| File | Changes |
|------|---------|
| `src-core/src/parser.rs` | New `Tag` variants, new `Event` variants, `parse_blocks` extended, `parse_inline` signature extended, footnote pre-scan & section emission |
| `src-core/src/html.rs` | New `match` arms for all new events/tags |
| `src-core/tests/integration.rs` | New tests for all three features; update `test_code_fence` |
| `src-cli/src/main.rs` | New CSS rules in embedded stylesheet |
| `web-playground/src/style.css` | New CSS rules |

---

## Task 1: Code Block — parser changes

**Files:**
- Modify: `src-core/src/parser.rs`

### New Tag variant

`Tag::CodeBlock(&'a str)` → `Tag::CodeBlock(&'a str, &'a str)` where fields are `(lang, filename)`. `filename` is `""` when not specified.

### Parsing change (around line 156)

Replace:
```rust
let lang = tline[3..].trim();
events.push(Event::Start(Tag::CodeBlock(lang)));
// ...
events.push(Event::End(Tag::CodeBlock(lang)));
```
With:
```rust
let info = tline[3..].trim();
let (lang, filename) = if let Some(colon) = info.find(':') {
    (&info[..colon], &info[colon + 1..])
} else {
    (info, "")
};
events.push(Event::Start(Tag::CodeBlock(lang, filename)));
// ...
events.push(Event::End(Tag::CodeBlock(lang, filename)));
```

- [ ] Apply the above change to `parser.rs`
- [ ] Fix all compile errors from the `Tag::CodeBlock` arity change (both in `parser.rs` and `html.rs`)
- [ ] Run `cargo build` — must compile cleanly

---

## Task 2: Code Block — HTML rendering + tests

**Files:**
- Modify: `src-core/src/html.rs`
- Modify: `src-core/tests/integration.rs`
- Modify: `src-cli/src/main.rs`
- Modify: `web-playground/src/style.css`

### HTML rendering

Replace the existing `CodeBlock` match arms with:

```rust
Event::Start(Tag::CodeBlock(lang, filename)) => {
    let has_header = !lang.is_empty() || !filename.is_empty();
    if has_header {
        out.push_str("<div class=\"nm-code-container\">");
        out.push_str("<div class=\"nm-code-header\">");
        if !lang.is_empty() {
            out.push_str(&format!(
                "<span class=\"nm-badge-main\">{}</span>",
                escape_html(lang)
            ));
        }
        if !filename.is_empty() {
            out.push_str(&format!(
                "<span class=\"nm-badge-flag\">{}</span>",
                escape_html(filename)
            ));
        }
        out.push_str("</div><div class=\"nm-code-content\">");
    }
    let cls = if lang.is_empty() {
        String::new()
    } else {
        format!(" language-{}", escape_html(lang))
    };
    out.push_str(&format!("<pre class=\"nm-code\"><code class=\"{}\">", cls));
}
Event::End(Tag::CodeBlock(lang, filename)) => {
    let has_header = !lang.is_empty() || !filename.is_empty();
    out.push_str("</code></pre>");
    if has_header {
        out.push_str("</div></div>");
    }
    out.push('\n');
}
```

### CSS — add to CLI embedded stylesheet (`src-cli/src/main.rs`)

Add after the existing `.nm-code-inline` rule (keeping existing `.nm-code-container` / `.nm-code-header` / `.nm-code-content` / `.nm-code` rules which are already present):

```css
.nm-badge-main { display: inline-block; padding: 2px 8px; border-radius: 6px; background: #7aa2f7; color: #1a202e; font-size: 11px; font-weight: bold; letter-spacing: .05em; }
.nm-badge-flag { display: inline-block; padding: 2px 8px; border-radius: 6px; border: 1px solid var(--border); background: rgba(0,0,0,0.2); color: var(--muted); font-size: 11px; }
```

(These already exist in `web-playground/src/style.css` — no change needed there.)

### Integration tests — update `test_code_fence`, add new tests

```rust
#[test]
fn test_code_fence_no_info() {
    // No lang, no filename → plain pre/code, no container
    let md = "```\nplain\n```";
    assert_eq!(
        render(md),
        "<pre class=\"nm-code\"><code class=\"\">plain\n</code></pre>"
    );
}

#[test]
fn test_code_fence_lang_only() {
    let md = "```rust\nfn f() {}\n```";
    assert_eq!(
        render(md),
        "<div class=\"nm-code-container\"><div class=\"nm-code-header\"><span class=\"nm-badge-main\">rust</span></div><div class=\"nm-code-content\"><pre class=\"nm-code\"><code class=\" language-rust\">fn f() {}\n</code></pre></div></div>"
    );
}

#[test]
fn test_code_fence_lang_and_filename() {
    let md = "```rust:src/main.rs\nfn f() {}\n```";
    assert_eq!(
        render(md),
        "<div class=\"nm-code-container\"><div class=\"nm-code-header\"><span class=\"nm-badge-main\">rust</span><span class=\"nm-badge-flag\">src/main.rs</span></div><div class=\"nm-code-content\"><pre class=\"nm-code\"><code class=\" language-rust\">fn f() {}\n</code></pre></div></div>"
    );
}
```

Remove or update the old `test_code_fence` test (it tested the old plain output for `rust` lang).

- [ ] Write the three new tests — run `cargo test --test integration test_code_fence` → FAIL (expected, old test still present)
- [ ] Apply HTML rendering changes to `html.rs`
- [ ] Add CSS to `src-cli/src/main.rs`
- [ ] Remove old `test_code_fence`, ensure new tests pass: `cargo test --test integration`
- [ ] Commit: `git add src-core/src/parser.rs src-core/src/html.rs src-core/tests/integration.rs src-cli/src/main.rs && git commit -m "feat(parser): code block lang:filename header"`

---

## Task 3: Card Link — parser + HTML + CSS

**Files:**
- Modify: `src-core/src/parser.rs`
- Modify: `src-core/src/html.rs`
- Modify: `src-core/tests/integration.rs`
- Modify: `src-cli/src/main.rs`
- Modify: `web-playground/src/style.css`

### New Event variant

Add to `Event<'a>`:
```rust
CardLink(&'a str),   // URL
```

### Parsing — block level in `parse_blocks`

Add before the paragraph collector (before the `let mut para = Vec::new();` line). Also add `|| tline.starts_with("@[")` to the paragraph break condition list.

```rust
// Card link block: @[card](URL)
if tline.starts_with("@[") {
    // Parse type name between [ and ]
    if let Some(bracket_end) = tline[2..].find(']') {
        let type_name = &tline[2..2 + bracket_end];
        let after_bracket = &tline[2 + bracket_end + 1..];
        if type_name == "card" {
            if after_bracket.starts_with('(') && after_bracket.ends_with(')') {
                let url = &after_bracket[1..after_bracket.len() - 1];
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    warnings.push(format!(
                        "Card link URL '{}' should start with http:// or https://",
                        url
                    ));
                }
                events.push(Event::CardLink(url));
            } else {
                warnings.push(format!(
                    "Malformed @[card] syntax near '{}': expected @[card](URL).",
                    &tline[..tline.len().min(40)]
                ));
            }
        } else {
            warnings.push(format!(
                "Unknown embed type '{}' in '@[{}]': only 'card' is supported.",
                type_name, type_name
            ));
        }
    }
    i += 1;
    continue;
}
```

Also add to the paragraph break guard:
```rust
|| tline.starts_with("@[")
```

### HTML rendering

Add to `push_html` match:
```rust
Event::CardLink(url) => {
    out.push_str(&format!(
        "<a href=\"{url}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">{url}</span></a>\n",
        url = escape_html(url)
    ));
}
```

### CSS — add to both `src-cli/src/main.rs` and `web-playground/src/style.css`

```css
.nm-card-link { display: block; border: 1px solid var(--border); border-radius: 10px; padding: 12px 16px; margin: 16px 0; background: var(--card); color: var(--fg); text-decoration: none; transition: border-color 0.15s; }
.nm-card-link:hover { border-color: var(--accent); }
.nm-card-url { display: block; font-size: 0.85em; color: var(--muted); word-break: break-all; }
```

### Integration tests

```rust
#[test]
fn test_card_link() {
    assert_eq!(
        render("@[card](https://example.com)"),
        "<a href=\"https://example.com\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">https://example.com</span></a>"
    );
}

#[test]
fn test_card_link_warn_non_http() {
    let parser = Parser::new("@[card](ftp://example.com)");
    assert!(parser.warnings.iter().any(|w| w.contains("http")));
}

#[test]
fn test_card_link_warn_unknown_type() {
    let parser = Parser::new("@[embed](https://example.com)");
    assert!(parser.warnings.iter().any(|w| w.contains("embed")));
}
```

- [ ] Write the three tests → `cargo test --test integration test_card` → FAIL
- [ ] Apply parser changes (new Event variant, block detection, paragraph guard)
- [ ] Apply html.rs changes
- [ ] Add CSS to both stylesheets
- [ ] `cargo test --test integration` → new tests pass
- [ ] Commit: `git add src-core/src/parser.rs src-core/src/html.rs src-core/tests/integration.rs src-cli/src/main.rs web-playground/src/style.css && git commit -m "feat(parser): @[card](URL) block card link"`

---

## Task 4: Footnotes — pre-scan helper

**Files:**
- Modify: `src-core/src/parser.rs`

Add a free function to pre-scan footnote definitions. Footnote definition lines have the form `[^id]: content` at the start of a line (no leading whitespace required beyond what `trim_start` covers, but definitions are typically at block level).

```rust
/// Collect all footnote definitions from the document lines.
/// Returns (id, content) pairs in document order.
/// Lines of the form `[^id]: content` are definitions.
fn collect_fn_defs<'a>(lines: &[&'a str]) -> Vec<(&'a str, &'a str)> {
    let mut defs: Vec<(&'a str, &'a str)> = Vec::new();
    for &line in lines {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("[^") {
            if let Some(colon_idx) = rest.find("]: ") {
                let id = &rest[..colon_idx];
                // Validate id: must be non-empty, no whitespace
                if !id.is_empty() && !id.contains(' ') {
                    let content = &rest[colon_idx + 3..];
                    // Only add if not duplicate
                    if !defs.iter().any(|(did, _)| *did == id) {
                        defs.push((id, content));
                    }
                }
            }
        }
    }
    defs
}
```

- [ ] Add `collect_fn_defs` to `parser.rs`
- [ ] `cargo build` → compiles (function is unused yet, may warn; add `#[allow(dead_code)]` temporarily)

---

## Task 5: Footnotes — extend `parse_inline` and `parse_blocks` signatures

**Files:**
- Modify: `src-core/src/parser.rs`

Add two parameters to `parse_inline`:

```rust
fn parse_inline<'a>(
    mut text: &'a str,
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<String>,
    in_ruby: bool,
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &mut Vec<&'a str>,
)
```

Add same two parameters to `parse_blocks`:

```rust
fn parse_blocks<'a>(
    lines: &[&'a str],
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<String>,
    root: bool,
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &mut Vec<&'a str>,
)
```

Update all call sites — every call to `parse_inline` inside `parse_blocks` and inside `parse_inline` itself must pass through `fn_defs` and `fn_refs`. The blockquote recursive call to `parse_blocks` likewise passes them through.

Update `Parser::new` to orchestrate:
```rust
pub fn new(text: &'a str) -> Self {
    let lines: Vec<&str> = text.lines().collect();
    let mut events = Vec::new();
    let mut warnings = Vec::new();
    let fn_defs = collect_fn_defs(&lines);
    let mut fn_refs: Vec<&str> = Vec::new();
    parse_blocks(&lines, &mut events, &mut warnings, true, &fn_defs, &mut fn_refs);
    // Footnote section emitted after all blocks (Task 7)
    Parser { events: events.into_iter(), warnings }
}
```

- [ ] Apply the signature changes to both functions and all call sites
- [ ] `cargo build` → must compile with no errors
- [ ] `cargo test --test integration` → existing passing tests still pass

---

## Task 6: Footnotes — new Tag/Event variants + HTML rendering

**Files:**
- Modify: `src-core/src/parser.rs`
- Modify: `src-core/src/html.rs`
- Modify: `src-cli/src/main.rs`
- Modify: `web-playground/src/style.css`

### New variants

In `Event<'a>`:
```rust
FootnoteRef(u32),   // inline superscript: footnote number
```

In `Tag<'a>`:
```rust
FootnoteSection,
FootnoteItem(u32),  // footnote number
```

### HTML rendering — add to `push_html` match

```rust
Event::FootnoteRef(n) => {
    out.push_str(&format!(
        "<sup class=\"nm-fn-ref\"><a href=\"#fn-{n}\" id=\"fnref-{n}\">{n}</a></sup>"
    ));
}
Event::Start(Tag::FootnoteSection) => {
    out.push_str("<section class=\"nm-footnotes\"><ol>\n");
}
Event::End(Tag::FootnoteSection) => {
    out.push_str("</ol></section>\n");
}
Event::Start(Tag::FootnoteItem(n)) => {
    out.push_str(&format!("<li id=\"fn-{n}\">"));
}
Event::End(Tag::FootnoteItem(n)) => {
    out.push_str(&format!(" <a href=\"#fnref-{n}\" class=\"nm-fn-back\">↩</a></li>\n"));
}
```

### CSS — add to both stylesheets

```css
.nm-fn-ref { font-size: 0.75em; vertical-align: super; line-height: 0; }
.nm-fn-ref a { color: var(--accent); text-decoration: none; }
.nm-footnotes { margin-top: 32px; border-top: 1px solid var(--border); padding-top: 12px; font-size: 0.9em; color: var(--muted); }
.nm-footnotes ol { padding-left: 20px; }
.nm-footnotes li { margin: 4px 0; }
.nm-fn-back { color: var(--muted); text-decoration: none; margin-left: 4px; }
```

- [ ] Add new `Event`/`Tag` variants
- [ ] Add HTML rendering arms to `html.rs`
- [ ] Add CSS to both stylesheets
- [ ] `cargo build` → compiles (new variants unused yet)

---

## Task 7: Footnotes — inline reference parsing + definition skipping

**Files:**
- Modify: `src-core/src/parser.rs`

### Skip definition lines in `parse_blocks`

Add near the top of the `while i < lines.len()` loop (before the code block check):

```rust
// Skip footnote definition lines (they are rendered in the footnote section)
if tline.starts_with("[^") && tline.contains("]: ") {
    i += 1;
    continue;
}
```

Also add `|| (tline.starts_with("[^") && tline.contains("]: "))` to the paragraph break guard list.

### Inline reference detection in `parse_inline`

Add **before** the generic `[` handler (so `[^` is caught first):

```rust
// Footnote reference: [^id]
if text.starts_with("[^") {
    if let Some(bracket_end) = text[2..].find(']') {
        let id = &text[2..2 + bracket_end];
        if !id.is_empty() && !id.contains(' ') {
            // Look up definition
            if fn_defs.iter().any(|(did, _)| *did == id) {
                // Assign number: position in fn_refs + 1, or add if new
                let num = if let Some(pos) = fn_refs.iter().position(|r| *r == id) {
                    (pos + 1) as u32
                } else {
                    fn_refs.push(id);
                    fn_refs.len() as u32
                };
                events.push(Event::FootnoteRef(num));
            } else {
                warnings.push(format!(
                    "Footnote reference '[^{}]' has no matching definition.",
                    id
                ));
                // Render as plain text
                events.push(Event::Text(&text[..2 + bracket_end + 1]));
            }
            text = &text[2 + bracket_end + 1..];
            continue;
        }
    }
}
```

Also add `'@'` and `'^'` are not needed in `next_special` — `[^` is caught by the existing `'['` entry in `next_special`.

### Emit footnote section at end of `Parser::new`

```rust
// After parse_blocks:
emit_fn_section(&fn_defs, &fn_refs, &mut events, &mut warnings);
```

Add function:
```rust
fn emit_fn_section<'a>(
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &[&'a str],
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<String>,
) {
    // Warn about definitions that were never referenced
    for (id, _) in fn_defs {
        if !fn_refs.contains(id) {
            warnings.push(format!(
                "Footnote '[^{}]' is defined but never referenced.",
                id
            ));
        }
    }
    if fn_refs.is_empty() {
        return;
    }
    events.push(Event::Start(Tag::FootnoteSection));
    // Emit items in reference order
    for (idx, &id) in fn_refs.iter().enumerate() {
        let num = (idx + 1) as u32;
        if let Some(&(_, content)) = fn_defs.iter().find(|(did, _)| *did == id) {
            events.push(Event::Start(Tag::FootnoteItem(num)));
            // Parse content inline (no nested footnotes)
            let mut nested_refs: Vec<&'a str> = Vec::new();
            parse_inline(content, events, warnings, false, fn_defs, &mut nested_refs);
            events.push(Event::End(Tag::FootnoteItem(num)));
        }
    }
    events.push(Event::End(Tag::FootnoteSection));
}
```

- [ ] Add definition-skip logic to `parse_blocks`
- [ ] Add `[^id]` handler to `parse_inline` (before generic `[` handler)
- [ ] Add `emit_fn_section` function and call it in `Parser::new`
- [ ] `cargo build` → compiles

---

## Task 8: Footnotes — integration tests + final validation

**Files:**
- Modify: `src-core/tests/integration.rs`

```rust
fn render_with_warnings(md: &str) -> (String, Vec<String>) {
    let parser = Parser::new(md);
    let warnings = parser.warnings.clone();
    let mut out = String::new();
    push_html(&mut out, parser);
    (out.trim().to_string(), warnings)
}

#[test]
fn test_footnote_basic() {
    let md = "Hello[^1] world.\n\n[^1]: A footnote.";
    let (html, _) = render_with_warnings(md);
    assert!(html.contains("<sup class=\"nm-fn-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup>"));
    assert!(html.contains("<section class=\"nm-footnotes\">"));
    assert!(html.contains("<li id=\"fn-1\">A footnote."));
    assert!(html.contains("href=\"#fnref-1\""));
}

#[test]
fn test_footnote_multiple() {
    let md = "First[^a] and second[^b].\n\n[^a]: Note A.\n[^b]: Note B.";
    let (html, _) = render_with_warnings(md);
    assert!(html.contains("id=\"fn-1\""));
    assert!(html.contains("id=\"fn-2\""));
    assert!(html.contains("Note A."));
    assert!(html.contains("Note B."));
}

#[test]
fn test_footnote_definition_not_rendered_inline() {
    // Definition line should not appear as paragraph text
    let md = "Text.\n\n[^1]: A note.";
    let (html, _) = render_with_warnings(md);
    assert!(!html.contains("<p>[^1]"));
}

#[test]
fn test_footnote_warn_undefined_ref() {
    let (_, warnings) = render_with_warnings("Text[^x].");
    assert!(warnings.iter().any(|w| w.contains("[^x]")));
}

#[test]
fn test_footnote_warn_unused_def() {
    let (_, warnings) = render_with_warnings("Text.\n\n[^1]: Unused note.");
    assert!(warnings.iter().any(|w| w.contains("[^1]") && w.contains("never referenced")));
}
```

- [ ] Write all five tests → `cargo test --test integration test_footnote` → FAIL
- [ ] Fix any issues in the implementation until all new tests pass
- [ ] `cargo test --test integration` → all tests pass (or pre-existing failures unchanged)
- [ ] Commit: `git add -p && git commit -m "feat(parser): footnote syntax [^id] / [^id]: content"`

---

## Task 9: Sample file update + final build check

**Files:**
- Modify: `web-playground/sample.n.md`

Add demo sections for each new feature:

```markdown
## コードブロック（ファイル名[付/つ]き）

\`\`\`rust:src/main.rs
fn main() {
    println!("Hello, world!");
}
\`\`\`

## カード[型/がた]リンク

@[card](https://www.rust-lang.org)

## [脚注/きゃくちゅう]

Gloss[記法/きほう]は[複数/ふくすう]の[言語/げんご]での[並記/へいき]を[可能/かのう]にします[^1]。

[^1]: [詳細/しょうさい]は[公式/こうしき]ドキュメントを[参照/さんしょう]してください。
```

- [ ] Add demo sections to `web-playground/sample.n.md`
- [ ] Run `trunk build` (if Trunk available) or `cargo build` to confirm no regressions
- [ ] `cargo test` — all tests pass
- [ ] Final commit: `git add web-playground/sample.n.md && git commit -m "docs(sample): add code block header, card link, and footnote examples"`
