use src_core::parser::Parser;
use src_core::html::push_html;

fn render_with_warnings(md: &str) -> (String, Vec<String>) {
    let parser = Parser::new(md);
    let warnings = parser.warnings.clone();
    let mut out = String::new();
    push_html(&mut out, parser);
    (out.trim().to_string(), warnings)
}

fn render(md: &str) -> String {
    let parser = Parser::new(md);
    let mut out = String::new();
    push_html(&mut out, parser);
    out.trim().to_string()
}

#[test]
fn test_ruby() {
    assert_eq!(
        render("ここは[漢字/かんじ]です"),
        "<p>ここは<ruby class=\"nm-ruby\"><rb>漢字</rb><rt>かんじ</rt></ruby>です</p>"
    );
}

#[test]
fn test_gloss() {
    assert_eq!(
        render("これは{用語/gloss}です"),
        "<p>これは<ruby class=\"nm-gloss\"><rb>用語</rb><rt><span class=\"nm-gloss-note\">gloss</span></rt></ruby>です</p>"
    );
}

#[test]
fn test_gloss_multi() {
    // Currently this will likely fail because our gloss parser may be weak on multi-parts
    // but this is the ideal output we are aiming for in Phase 4.
    assert_eq!(
        render("これは{Word/gloss/extra}です"),
        "<p>これは<ruby class=\"nm-gloss\"><rb>Word</rb><rt><span class=\"nm-gloss-note\">gloss</span><span class=\"nm-gloss-note\">extra</span></rt></ruby>です</p>"
    );
}

#[test]
fn test_section_nesting() {
    let md = "# H1\n## H2\n\ntext\n\n---\n\n;;;";
    let expected = r#"<section class="nm-sec level-1">
<h1>H1</h1>
<section class="nm-sec level-2">
<h2>H2</h2>
<p>text</p>
<hr/>
</section>
</section>"#;
    assert_eq!(render(md), expected.trim());
}

#[test]
fn test_inline_formats() {
    assert_eq!(render("**strong** *em* ~~strike~~"), "<p><strong>strong</strong> <em>em</em> <del>strike</del></p>");
}

#[test]
fn test_code_fence_no_info() {
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

#[test]
fn test_table() {
    let md = "| A | B |\n|:---|---:|\n| 1 | 2 |";
    let expected = r#"<div class="nm-table-wrap"><table class="nm-table">
<thead><tr><th style="text-align:left">A</th><th style="text-align:right">B</th></tr>
</thead>
<tbody>
<tr><td style="text-align:left">1</td><td style="text-align:right">2</td></tr>
</tbody>
</table></div>"#;
    assert_eq!(render(md), expected.trim());
}

#[test]
fn test_card_link() {
    assert_eq!(
        render("@[card](https://example.com)"),
        "<a href=\"https://example.com\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">https://example.com</span></a>"
    );
}

#[test]
fn test_card_link_warn_non_http() {
    let (_, warnings) = render_with_warnings("@[card](ftp://example.com)");
    assert!(warnings.iter().any(|w| w.contains("http")));
}

#[test]
fn test_card_link_warn_unknown_type() {
    let (_, warnings) = render_with_warnings("@[embed](https://example.com)");
    assert!(warnings.iter().any(|w| w.contains("embed")));
}

#[test]
fn test_footnote_basic() {
    let md = "Hello[^1] world.\n\n[^1]: A footnote.";
    let (html, _) = render_with_warnings(md);
    assert!(html.contains("<sup class=\"nm-fn-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup>"), "missing ref: {html}");
    assert!(html.contains("<section class=\"nm-footnotes\">"), "missing section: {html}");
    assert!(html.contains("<li id=\"fn-1\">A footnote."), "missing item: {html}");
    assert!(html.contains("href=\"#fnref-1\""), "missing back link: {html}");
}

#[test]
fn test_footnote_multiple() {
    let md = "First[^a] and second[^b].\n\n[^a]: Note A.\n[^b]: Note B.";
    let (html, _) = render_with_warnings(md);
    assert!(html.contains("id=\"fn-1\""), "missing fn-1: {html}");
    assert!(html.contains("id=\"fn-2\""), "missing fn-2: {html}");
    assert!(html.contains("Note A."), "missing A: {html}");
    assert!(html.contains("Note B."), "missing B: {html}");
}

#[test]
fn test_footnote_definition_not_rendered_inline() {
    let md = "Text.\n\n[^1]: A note.";
    let (html, _) = render_with_warnings(md);
    assert!(!html.contains("<p>[^1]"), "def line leaked into paragraph: {html}");
}

#[test]
fn test_footnote_warn_undefined_ref() {
    let (_, warnings) = render_with_warnings("Text[^x].");
    assert!(warnings.iter().any(|w| w.contains("[^x]")), "expected warn: {warnings:?}");
}

#[test]
fn test_footnote_warn_unused_def() {
    let (_, warnings) = render_with_warnings("Text.\n\n[^1]: Unused note.");
    assert!(
        warnings.iter().any(|w| w.contains("[^1]") && w.contains("never referenced")),
        "expected warn: {warnings:?}"
    );
}

#[test]
fn test_ruby_katakana_hiragana_warn() {
    // Purely katakana base + hiragana reading → warning
    let (_, warnings) = render_with_warnings("[インド/いんど]");
    assert!(
        warnings.iter().any(|w| w.contains("インド") && w.contains("katakana")),
        "expected katakana-hiragana ruby warning: {warnings:?}"
    );
}

#[test]
fn test_ruby_kanji_katakana_no_warn() {
    // Kanji + katakana mixed base → NOT purely katakana → no warning
    let (_, warnings) = render_with_warnings("[自由エネルギー/じゆうえねるぎー]");
    assert!(
        !warnings.iter().any(|w| w.contains("katakana")),
        "should not warn for kanji+katakana base: {warnings:?}"
    );
}

#[test]
fn test_blockquote() {
    assert_eq!(
        render("> quote\n> line 2"),
        "<blockquote class=\"nm-blockquote\">\n<p>quote<br/>\nline 2</p>\n</blockquote>"
    );
}
