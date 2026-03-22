use src_core::parser::Parser;
use src_core::html::push_html;

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
fn test_code_fence() {
    let md = "```rust\nfn main() {}\n```";
    assert_eq!(render(md), "<pre class=\"nm-code\"><code class=\" language-rust\">fn main() {}\n</code></pre>");
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
fn test_blockquote() {
    assert_eq!(
        render("> quote\n> line 2"),
        "<blockquote class=\"nm-blockquote\">\n<p>quote<br/>\nline 2</p>\n</blockquote>"
    );
}
