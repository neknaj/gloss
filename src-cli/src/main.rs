use std::env;
use std::fs;
use std::process;
use src_core::parser::Parser;
use src_core::html::push_html;

const HTML_HEAD: &str = r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>Gloss Markdown Preview</title>
<style>
:root {
  --bg: #0b0f19;
  --fg: #e6edf3;
  --muted: #aab6c3;
  --card: #121a2a;
  --border: #23304a;
  --code: #0f1626;
  --accent: #7aa2f7;
}
html, body {
  background: var(--bg);
  color: var(--fg);
  font-family: system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial;
  line-height: 1.65;
  margin: 0;
  height: 100vh;
}
main { max-width: 980px; margin: 24px auto; padding: 0 16px; }
a { color: var(--accent); }
hr { border: none; border-top: 1px solid var(--border); margin: 24px 0; }
.nm-sec { padding: 0.5em; padding-left: 2em; margin: 1em; border-left: 3px solid var(--border); border-radius: 1em; }
h1, h2, h3, h4, h5, h6 { margin: 18px 0 10px; }
p { margin: 10px 0; }
ul, ol { margin: 10px 0 10px 22px; }
strong { font-weight: 700; }
em { font-style: italic; }
del { text-decoration: line-through; color: var(--muted); }
.nm-image { max-width: 100%; border-radius: 8px; }
.nm-blockquote { border-left: 4px solid var(--border); margin: 12px 0; padding: 8px 16px; background: rgba(255,255,255,0.02); border-radius: 0 8px 8px 0; color: var(--muted); }
.nm-blockquote p { margin: 4px 0; }
.nm-table-wrap { overflow-x: auto; margin: 16px 0; }
.nm-table { border-collapse: collapse; width: 100%; font-size: 14px; }
.nm-table th, .nm-table td { border: 1px solid var(--border); padding: 6px 12px; text-align: left; }
.nm-table thead tr { background: rgba(122,162,247,0.12); }
.nm-table tbody tr:nth-child(even) { background: rgba(255,255,255,0.02); }
.nm-table th { font-weight: 600; color: var(--accent); }
.nm-code-container { border: 1px solid var(--border); border-radius: 12px; background: var(--card); margin: 24px 0; overflow: hidden; }
.nm-code-header { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: rgba(255,255,255,0.03); border-bottom: 1px solid var(--border); flex-wrap: wrap; }
.nm-code-content { position: relative; }
.nm-code { background: var(--code); padding: 12px; overflow: auto; margin: 0; border: none; border-radius: 0; }
.nm-code code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 13px; white-space: pre; }
.nm-code-inline { background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.10); border-radius: 8px; padding: 1px 6px; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 0.9em; }
.nm-badge-main { display: inline-block; padding: 2px 8px; border-radius: 6px; background: #7aa2f7; color: #1a202e; font-size: 11px; font-weight: bold; letter-spacing: .05em; }
.nm-badge-flag { display: inline-block; padding: 2px 8px; border-radius: 6px; border: 1px solid var(--border); background: rgba(0,0,0,0.2); color: var(--muted); font-size: 11px; }
.nm-card-link { display: block; border: 1px solid var(--border); border-radius: 10px; padding: 12px 16px; margin: 16px 0; background: var(--card); color: var(--fg); text-decoration: none; transition: border-color 0.15s; }
.nm-card-link:hover { border-color: var(--accent); }
.nm-card-url { display: block; font-size: 0.85em; color: var(--muted); word-break: break-all; }
.nm-fn-ref { font-size: 0.75em; vertical-align: super; line-height: 0; }
.nm-fn-ref a { color: var(--accent); text-decoration: none; }
.nm-footnotes { margin-top: 32px; border-top: 1px solid var(--border); padding-top: 12px; font-size: 0.9em; color: var(--muted); }
.nm-footnotes ol { padding-left: 20px; }
.nm-footnotes li { margin: 4px 0; }
.nm-fn-back { color: var(--muted); text-decoration: none; margin-left: 4px; }

/* Front matter */
.nm-frontmatter { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 14px; padding: 7px 14px; margin: 4px 0 20px; border-left: 3px solid var(--accent); background: rgba(122,162,247,0.06); border-radius: 0 8px 8px 0; font-size: 0.84em; }
.nm-fm-meta { display: flex; flex-wrap: wrap; gap: 4px 12px; align-items: center; }
.nm-fm-field { display: inline-flex; align-items: center; gap: 4px; }
.nm-fm-key { color: var(--muted); font-size: 0.9em; }
.nm-fm-key::after { content: ":"; }
.nm-fm-val { color: var(--fg); font-weight: 500; }
.nm-fm-tags { display: flex; flex-wrap: wrap; gap: 4px; }
.nm-fm-tag { background: rgba(122,162,247,0.14); border: 1px solid rgba(122,162,247,0.35); color: var(--accent); border-radius: 12px; padding: 1px 9px; font-size: 0.82em; }

/* Unified Ruby and Anno Styles */
ruby rt {
  font-size: 0.65em;
  color: var(--muted);
  opacity: 0.9;
  line-height: 1;
}
.nm-ruby { ruby-position: over; }
.nm-anno { ruby-position: under; }
.nm-anno rt { font-size: 0.65em; }
.nm-anno-note { display: inline; }
.nm-anno-note + .nm-anno-note::before { content: ' / '; opacity: 0.6; }

.math-inline { color: var(--muted); }
.math-display {
  display: block;
  padding: 8px 10px;
  margin: 8px 0;
  background: rgba(255,255,255,0.03);
  border: 1px dashed var(--border);
  border-radius: 10px;
  overflow-x: auto;
}
</style>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js"></script>
<script>
document.addEventListener("DOMContentLoaded", function() {
  document.querySelectorAll('.math-inline, .math-display').forEach(function(el) {
    var texSpan = el.querySelector('.math-tex');
    if (texSpan) {
      katex.render(texSpan.textContent, el, {
        displayMode: el.classList.contains('math-display'),
        throwOnError: false
      });
    }
  });
});
</script>
</head>
<body>
<main>
"#;

const HTML_TAIL: &str = r#"
</main>
</body>
</html>
"#;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.n.md> [output.html]", args[0]);
        process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = if args.len() >= 3 {
        args[2].clone()
    } else {
        let mut p = input_path.to_string();
        if p.ends_with(".n.md") {
            p = p.replace(".n.md", ".html");
        } else if p.ends_with(".md") {
            p = p.replace(".md", ".html");
        } else {
            p.push_str(".html");
        }
        p
    };

    let text = match fs::read_to_string(input_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error reading file {}: {}", input_path, e);
            process::exit(1);
        }
    };

    let source = input_path.to_string();
    let parser = Parser::new_with_source(&text, &source);

    if !parser.warnings.is_empty() {
        for w in &parser.warnings {
            eprintln!("\x1b[33m[{}:{}:{}] {} — {}\x1b[0m",
                w.source, w.line, w.col, w.code, w.message);
        }
    }

    let mut html_body = String::new();
    push_html(&mut html_body, parser);

    let final_html = format!("{}{}{}", HTML_HEAD, html_body, HTML_TAIL);

    if let Err(e) = fs::write(&output_path, final_html) {
        eprintln!("Error writing output file {}: {}", output_path, e);
        process::exit(1);
    }

    println!("Successfully compiled {} -> {}", input_path, output_path);
}
