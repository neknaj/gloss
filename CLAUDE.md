# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

**Gloss** is a Rust library and toolchain for a custom Markdown dialect. It extends standard Markdown with five named features:

| Feature | Syntax | Description |
|---------|--------|-------------|
| **Ruby** | `[漢字/かんじ]` | Phonetic annotation *above* text — furigana, pinyin, bopomofo, transliteration, etc. |
| **Anno** | `{用語/term}` / `{w/a/b}` | Semantic annotation *below* text — translation, grammar (case/POS), classification, interlinear gloss |
| **Nest** | `---` / `;;;` | Explicit section-close markers; `---` closes and draws `<hr>`, `;;;` closes silently. Both maintain `<section class="nm-sec level-N">` hierarchy |
| **Math** | `$…$` / `$$…$$` | Inline / display math via KaTeX; `[/]` `{/}` inside are not parsed as Ruby/Anno |
| **Lint** | — | Parser warnings: malformed Ruby/Anno brackets, lone `$`, katakana base + hiragana reading, kanji without ruby, undefined/unused footnotes, non-HTTP card links |

Input files conventionally use the `.n.md` extension.

## Workspace Structure

This is a Cargo workspace with three crates:

- **`src-core`** — `#![no_std]` parser and HTML generator; the core library
  - `parser.rs` — tokenizes input into `Event` stream (`Start/End/Text/MathInline/MathDisplay/SoftBreak/HardBreak/Rule`)
  - `html.rs` — walks events and emits HTML strings via `push_html()`
  - `tests/integration.rs` — unit tests using `render()` helper
- **`src-web`** — WASM build using `wasm-bindgen`; exposes `render_markdown()` to JS and wires up live preview via DOM events
- **`src-cli`** — binary that reads a `.n.md` file and emits a standalone HTML page

The web playground (`web-playground/`) is a separate TypeScript project bundled by **Trunk** (a Rust/WASM bundler). It uses the WASM output from `src-web`.

## Commands

### Build & Run

```sh
# Build all Rust crates
cargo build

# Run the CLI converter
cargo run -p src-cli -- input.n.md [output.html]

# Run Rust tests
cargo test

# Run only integration tests for src-core
cargo test --test integration

# Run a single test by name
cargo test --test integration test_ruby

# Build WASM + serve the web playground (requires trunk)
trunk serve

# Build TypeScript in web-playground
cd web-playground && npm run build:ts
```

### Key Behaviors

- **Lint**: `Parser::warnings: Vec<String>` collects all lint messages during parsing. CLI prints them to stderr in yellow; web playground shows them in an amber warning box above the preview.
- **Math**: the HTML generator emits both native MathML (via `latex2mathml`) and a hidden `.math-tex` span; the web playground uses KaTeX to re-render from the hidden span.
- `src-core` is `#![no_std]` — only `alloc` is available. Do not add `std` dependencies.

## Test Architecture

Integration tests live in `src-core/tests/integration.rs`. Each test calls `render(markdown_str)` and asserts exact HTML output. The golden `.html` files in `tests/testcases/` are separate snapshot references used for manual comparison, not run automatically.

## Syntax Reference

The notation summary from the README is the authoritative source. `doc/spec.md` contains the formal AST specification for the parser. The `Tag` enum in `parser.rs` directly maps to the AST node types defined there.

## Versioning Policy

All crates follow **[Semantic Versioning](https://semver.org/)** in `0.x.y` pre-1.0 form. Breaking changes are allowed until v1.0.

### Release Tags

GitHub releases use component-prefixed tags so CLI and Desktop can release independently:

| Component | Tag format | Example |
|-----------|------------|---------|
| CLI tool | `cli/vX.Y.Z` | `cli/v0.1.0` |
| Desktop app | `desktop/vX.Y.Z` | `desktop/v0.1.0` |
| Core library | `core/vX.Y.Z` | `core/v0.1.0` |
| All-in-one (v1.0+) | `vX.Y.Z` | `v1.0.0` |

### Version Bump Rules (pre-1.0)

- **patch** (`Z`): bug fixes only
- **minor** (`Y`): new features, reworks, breaking changes OK
- **major** stays `0` until production-ready

Crate versions in `Cargo.toml` are bumped per-crate at release time. Keep all crate versions in sync (they all live at the same `x.y.z`) even if not all are being released.

### Release Workflow

```sh
# 1. Bump version in relevant Cargo.toml(s) if needed, commit
git commit -m "chore: bump cli to vX.Y.Z"

# 2. Tag
git tag cli/vX.Y.Z

# 3. Push branch + tag
git push && git push origin cli/vX.Y.Z

# 4. Create GitHub release
gh release create cli/vX.Y.Z --title "CLI vX.Y.Z" --notes "..."
```
