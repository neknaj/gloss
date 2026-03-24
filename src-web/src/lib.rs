use wasm_bindgen::prelude::*;
use src_core::{Parser, push_html, push_html_with_ids, fnv1a, split_source_blocks};

// ── JSON helpers ──────────────────────────────────────────────────────────────

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _    => out.push(c),
        }
    }
    out.push('"');
    out
}

fn warnings_to_json(warnings: &[src_core::Warning]) -> String {
    let items: Vec<String> = warnings.iter().map(|w| {
        format!(
            "{{\"code\":{},\"message\":{},\"source\":{},\"line\":{},\"col\":{}}}",
            json_str(w.code),
            json_str(&w.message),
            json_str(&w.source),
            w.line,
            w.col,
        )
    }).collect();
    format!("[{}]", items.join(","))
}

// ── Public WASM API ───────────────────────────────────────────────────────────

/// Render Gloss Markdown to a plain HTML fragment (no block IDs).
/// Suitable for server-side / static use.
#[wasm_bindgen]
pub fn render_markdown(input: &str) -> String {
    let parser = Parser::new(input);
    let mut out = String::new();
    push_html(&mut out, parser);
    out
}

/// Render and return a JSON string `{"html":"...","warnings":[...]}`.
///
/// The `html` value contains `data-bid` attributes on block-level elements,
/// enabling the JS side to do morphdom-style differential DOM updates.
///
/// Each warning: `{code, message, source, line, col}`.
#[wasm_bindgen]
pub fn render_with_warnings(input: &str, source: &str) -> String {
    let parser = Parser::new_with_source(input, source);
    let warnings_json = warnings_to_json(&parser.warnings);
    let mut html = String::new();
    push_html_with_ids(&mut html, parser);
    format!("{{\"html\":{},\"warnings\":{}}}", json_str(&html), warnings_json)
}

/// Return a JSON array of FNV-1a hex hashes, one per source block.
/// JS can compare successive calls to detect which blocks changed.
#[wasm_bindgen]
pub fn source_block_hashes(input: &str) -> String {
    let blocks = split_source_blocks(input);
    let hashes: Vec<String> = blocks.iter()
        .map(|b| format!("\"{}\"", format!("{:x}", fnv1a(b))))
        .collect();
    format!("[{}]", hashes.join(","))
}

// ── WASM entry point ─────────────────────────────────────────────────────────

/// Called automatically when the WASM module loads.
/// Sets up the panic hook and fires a `wasm-ready` event so JS can start.
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // Signal JS that WASM is ready; JS handles all DOM wiring from here.
    let window = web_sys::window().expect("no global `window` exists");
    let event = web_sys::Event::new("wasm-ready")?;
    window.dispatch_event(&event)?;

    Ok(())
}
