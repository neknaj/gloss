# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

A Rust library and toolchain for a custom Markdown dialect called **Gloss** that adds ruby annotation and multilingual gloss notation to standard Markdown:

- `[漢字/かんじ]` — ruby (furigana/phonetic annotation above text)
- `{用語/gloss}` or `{word/lang1/lang2}` — gloss (interlinear annotation below text)
- `$...$` / `$$...$$` — math (KaTeX; brackets inside are not parsed as ruby/gloss)
- Input files conventionally use `.n.md` extension

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

- The parser emits warnings (stored in `Parser::warnings`) for malformed syntax (unclosed brackets, lone `$` signs, etc.). CLI prints them to stderr in yellow; web playground shows them in an amber warning box above the preview.
- Math output: the HTML generator emits both native MathML (via `latex2mathml`) and a hidden `.math-tex` span; the web playground uses KaTeX to re-render from the hidden span.
- `src-core` is `#![no_std]` — only `alloc` is available. Do not add `std` dependencies.

## Test Architecture

Integration tests live in `src-core/tests/integration.rs`. Each test calls `render(markdown_str)` and asserts exact HTML output. The golden `.html` files in `tests/testcases/` are separate snapshot references used for manual comparison, not run automatically.

## Syntax Reference

The notation summary from the README is the authoritative source. `doc/spec.md` contains the formal AST specification for the parser. The `Tag` enum in `parser.rs` directly maps to the AST node types defined there.
