//! File-based snapshot tests.
//!
//! `tests/html/` — `.n.md` + `.html` pairs; expects **zero** warnings.
//! `tests/lint/` — `.n.md` + `.html` + `.json` triples; validates warning codes/lines.
//!
//! Run with `BLESS=1 cargo test --test snapshot` to regenerate golden files.

use src_core::parser::{Parser, Warning};
use src_core::html::push_html;
use std::fs;
use std::path::{Path, PathBuf};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn render(md: &str, source: &str) -> (String, Vec<Warning>) {
    let parser = Parser::new_with_source(md, source);
    let warnings = parser.warnings.clone();
    let mut out = String::new();
    push_html(&mut out, parser);
    (out.trim_end().to_string(), warnings)
}

fn bless() -> bool {
    std::env::var("BLESS").map(|v| v == "1").unwrap_or(false)
}

/// Collect all `.n.md` files in `dir`, sorted.
fn n_md_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read dir {}: {}", dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".n.md"))
            .unwrap_or(false))
        .collect();
    files.sort();
    files
}

fn html_path(p: &Path) -> PathBuf {
    let name = p.file_name().unwrap().to_str().unwrap();
    let stem = name.strip_suffix(".n.md").unwrap();
    p.with_file_name(format!("{}.html", stem))
}

fn json_path(p: &Path) -> PathBuf {
    let name = p.file_name().unwrap().to_str().unwrap();
    let stem = name.strip_suffix(".n.md").unwrap();
    p.with_file_name(format!("{}.json", stem))
}

// ── Minimal JSON serializer / deserializer for warning records ────────────────

#[derive(Debug, PartialEq)]
struct ExpectedWarning {
    code: String,
    line: u32,
    col: u32,
}

/// Hand-rolled parser for `[{"code":"X","line":N,"col":N}, ...]`.
fn parse_expected_warnings(json: &str) -> Vec<ExpectedWarning> {
    let mut out = Vec::new();
    let inner = json.trim().trim_start_matches('[').trim_end_matches(']');
    for obj in inner.split('}') {
        let obj = obj.trim().trim_start_matches(',').trim().trim_start_matches('{');
        if obj.is_empty() { continue; }
        let mut code = String::new();
        let mut line = 0u32;
        let mut col  = 0u32;
        for kv in obj.split(',') {
            let kv = kv.trim();
            if let Some(rest) = kv.strip_prefix("\"code\"") {
                code = rest.trim().trim_start_matches(':').trim().trim_matches('"').to_string();
            } else if let Some(rest) = kv.strip_prefix("\"line\"") {
                line = rest.trim().trim_start_matches(':').trim().trim_matches('"')
                    .parse().unwrap_or(0);
            } else if let Some(rest) = kv.strip_prefix("\"col\"") {
                col = rest.trim().trim_start_matches(':').trim().trim_matches('"')
                    .parse().unwrap_or(0);
            }
        }
        if !code.is_empty() {
            out.push(ExpectedWarning { code, line, col });
        }
    }
    out
}

fn warnings_to_json(warnings: &[Warning]) -> String {
    let items: Vec<String> = warnings.iter().map(|w| {
        format!("  {{\"code\":\"{}\",\"line\":{},\"col\":{}}}", w.code, w.line, w.col)
    }).collect();
    format!("[\n{}\n]\n", items.join(",\n"))
}

// ── tests/html/ ──────────────────────────────────────────────────────────────

#[test]
fn test_html_snapshots() {
    let dir = Path::new("tests/html");
    let mut failures: Vec<String> = Vec::new();

    for md_path in n_md_files(dir) {
        let source = md_path.file_name().unwrap().to_str().unwrap().to_string();
        let md = fs::read_to_string(&md_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", md_path.display(), e));
        let (html, warnings) = render(&md, &source);
        let hp = html_path(&md_path);

        if bless() {
            fs::write(&hp, format!("{}\n", html))
                .unwrap_or_else(|e| panic!("cannot write {}: {}", hp.display(), e));
            if !warnings.is_empty() {
                eprintln!("WARN: {} has unexpected warnings:", source);
                for w in &warnings {
                    eprintln!("  [{}:{}] {} — {}", w.line, w.col, w.code, w.message);
                }
            }
            continue;
        }

        // Expect zero warnings
        if !warnings.is_empty() {
            failures.push(format!(
                "{}: expected zero warnings, got:\n{}",
                source,
                warnings.iter().map(|w| format!("  [{}:{}] {} — {}", w.line, w.col, w.code, w.message))
                    .collect::<Vec<_>>().join("\n")
            ));
        }

        // Compare HTML snapshot
        if hp.exists() {
            let expected = fs::read_to_string(&hp).unwrap();
            if html != expected.trim_end() {
                failures.push(format!(
                    "{}:\n  expected: {:?}\n  actual:   {:?}",
                    source,
                    &expected[..expected.len().min(200)],
                    &html[..html.len().min(200)]
                ));
            }
        } else {
            failures.push(format!(
                "{}: missing golden file {} (run BLESS=1)", source, hp.display()
            ));
        }
    }

    if !failures.is_empty() {
        panic!("HTML snapshot failures:\n\n{}", failures.join("\n\n"));
    }
}

// ── tests/lint/ ──────────────────────────────────────────────────────────────

#[test]
fn test_lint_snapshots() {
    let dir = Path::new("tests/lint");
    let mut failures: Vec<String> = Vec::new();

    for md_path in n_md_files(dir) {
        let source = md_path.file_name().unwrap().to_str().unwrap().to_string();
        let md = fs::read_to_string(&md_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", md_path.display(), e));
        let (html, warnings) = render(&md, &source);
        let hp = html_path(&md_path);
        let jp = json_path(&md_path);

        if bless() {
            fs::write(&hp, format!("{}\n", html)).unwrap();
            fs::write(&jp, warnings_to_json(&warnings)).unwrap();
            if warnings.is_empty() {
                eprintln!("WARN: {} produced no warnings (but is in tests/lint/)", source);
            } else {
                for w in &warnings {
                    eprintln!("  LINT {}: [{}:{}] {} — {}", source, w.line, w.col, w.code, w.message);
                }
            }
            continue;
        }

        // Compare HTML
        if hp.exists() {
            let expected = fs::read_to_string(&hp).unwrap();
            if html != expected.trim_end() {
                failures.push(format!(
                    "{}: HTML mismatch\n  expected: {:?}\n  actual:   {:?}",
                    source,
                    &expected[..expected.len().min(200)],
                    &html[..html.len().min(200)]
                ));
            }
        } else {
            failures.push(format!(
                "{}: missing {}.html (run BLESS=1)", source, source
            ));
        }

        // Compare warnings with JSON
        if jp.exists() {
            let json = fs::read_to_string(&jp).unwrap();
            let expected_warns = parse_expected_warnings(&json);
            let actual: Vec<ExpectedWarning> = warnings.iter().map(|w| ExpectedWarning {
                code: w.code.to_string(),
                line: w.line,
                col:  w.col,
            }).collect();

            for exp in &expected_warns {
                if !actual.iter().any(|a| a.code == exp.code && a.line == exp.line) {
                    failures.push(format!(
                        "{}: missing expected warning code='{}' line={}",
                        source, exp.code, exp.line
                    ));
                }
            }
            if actual.len() != expected_warns.len() {
                failures.push(format!(
                    "{}: expected {} warning(s), got {}:\n{}",
                    source,
                    expected_warns.len(),
                    actual.len(),
                    warnings.iter().map(|w| format!("  [{}:{}] {} — {}", w.line, w.col, w.code, w.message))
                        .collect::<Vec<_>>().join("\n")
                ));
            }
        } else {
            failures.push(format!(
                "{}: missing {}.json (run BLESS=1)", source, source
            ));
        }
    }

    if !failures.is_empty() {
        panic!("Lint snapshot failures:\n\n{}", failures.join("\n\n"));
    }
}
