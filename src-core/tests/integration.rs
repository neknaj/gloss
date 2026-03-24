use src_core::parser::{Parser, Warning};
use src_core::html::push_html;

fn render_with_warnings(md: &str) -> (String, Vec<Warning>) {
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

fn has_code(warnings: &[Warning], code: &str) -> bool {
    warnings.iter().any(|w| w.code == code)
}

#[test]
fn test_ruby() {
    assert_eq!(
        render("[漢字/かんじ]です"),
        "<p><ruby class=\"nm-ruby\"><rb>漢字</rb><rt>かんじ</rt></ruby>です</p>"
    );
}

#[test]
fn test_anno() {
    assert_eq!(
        render("{用語/gloss}です"),
        "<p><ruby class=\"nm-anno\"><rb>用語</rb><rt><span class=\"nm-anno-note\">gloss</span></rt></ruby>です</p>"
    );
}

#[test]
fn test_anno_multi() {
    assert_eq!(
        render("{Word/gloss/extra}です"),
        "<p><ruby class=\"nm-anno\"><rb>Word</rb><rt><span class=\"nm-anno-note\">gloss</span><span class=\"nm-anno-note\">extra</span></rt></ruby>です</p>"
    );
}

#[test]
fn test_section_nesting() {
    // `---` closes level-2, emits HR in parent (level-1); `;;;` closes level-1.
    let md = "# H1\n## H2\n\ntext\n\n---\n\n;;;";
    let expected = r#"<section class="nm-sec level-1">
<h1>H1</h1>
<section class="nm-sec level-2">
<h2>H2</h2>
<p>text</p>
</section>
<hr/>
</section>"#;
    assert_eq!(render(md), expected.trim());
}

#[test]
fn test_inline_formats() {
    assert_eq!(
        render("**strong** *em* ~~strike~~"),
        "<p><strong>strong</strong> <em>em</em> <del>strike</del></p>"
    );
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
    let html = render(md);
    assert!(html.contains("<div class=\"nm-table-wrap\""), "missing table wrap: {html}");
    assert!(html.contains("<table class=\"nm-table\">"), "missing table: {html}");
    assert!(html.contains("<th style=\"text-align:left\">A</th>"), "missing header: {html}");
    assert!(html.contains("<td style=\"text-align:right\">2</td>"), "missing cell: {html}");
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
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("@[card](ftp://example.com)");
    assert!(has_code(&warnings, codes::CARD_NON_HTTP), "expected CARD_NON_HTTP: {:?}", warnings);
}

#[test]
fn test_card_link_warn_unknown_type() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("@[embed](https://example.com)");
    assert!(has_code(&warnings, codes::CARD_UNKNOWN_TYPE), "expected CARD_UNKNOWN_TYPE: {:?}", warnings);
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
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("Text[^x].");
    assert!(has_code(&warnings, codes::FOOTNOTE_UNDEFINED_REF), "expected warn: {:?}", warnings);
    assert!(warnings.iter().any(|w| w.message.contains("[^x]")), "expected [^x] in msg: {:?}", warnings);
}

#[test]
fn test_footnote_warn_unused_def() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("Text.\n\n[^1]: Unused note.");
    assert!(
        has_code(&warnings, codes::FOOTNOTE_UNUSED_DEF) &&
        warnings.iter().any(|w| w.code == codes::FOOTNOTE_UNUSED_DEF && w.message.contains("[^1]")),
        "expected FOOTNOTE_UNUSED_DEF with [^1]: {:?}", warnings
    );
}

#[test]
fn test_ruby_katakana_hiragana_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("[インド/いんど]");
    assert!(
        has_code(&warnings, codes::RUBY_KATAKANA_HIRAGANA),
        "expected katakana-hiragana ruby warning: {:?}", warnings
    );
    let w = warnings.iter().find(|w| w.code == codes::RUBY_KATAKANA_HIRAGANA).unwrap();
    assert_eq!(w.line, 1);
    assert_eq!(w.col, 1);
}

#[test]
fn test_ruby_kanji_katakana_no_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("[自由エネルギー/じゆうえねるぎー]");
    assert!(
        !has_code(&warnings, codes::RUBY_KATAKANA_HIRAGANA),
        "should not warn for kanji+katakana base: {:?}", warnings
    );
}

#[test]
fn test_ruby_empty_base_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("[/reading]");
    assert!(has_code(&warnings, codes::RUBY_EMPTY_BASE), "expected RUBY_EMPTY_BASE: {:?}", warnings);
}

#[test]
fn test_ruby_empty_reading_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("[base/]");
    assert!(has_code(&warnings, codes::RUBY_EMPTY_READING), "expected RUBY_EMPTY_READING: {:?}", warnings);
}

#[test]
fn test_ruby_self_referential_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("[same/same]");
    assert!(has_code(&warnings, codes::RUBY_SELF_REFERENTIAL), "expected RUBY_SELF_REFERENTIAL: {:?}", warnings);
}

#[test]
fn test_anno_empty_base_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("{/note}");
    assert!(has_code(&warnings, codes::ANNO_EMPTY_BASE), "expected ANNO_EMPTY_BASE: {:?}", warnings);
}

#[test]
fn test_anno_looks_like_ruby_warn() {
    use src_core::parser::codes;
    let (_, warnings) = render_with_warnings("{漢字/かんじ}");
    assert!(has_code(&warnings, codes::ANNO_LOOKS_LIKE_RUBY), "expected ANNO_LOOKS_LIKE_RUBY: {:?}", warnings);
}

#[test]
fn test_blockquote() {
    assert_eq!(
        render("> quote\n> line 2"),
        "<blockquote class=\"nm-blockquote\">\n<p>quote<br/>\nline 2</p>\n</blockquote>"
    );
}

#[test]
fn test_warning_positions() {
    // Warnings at known positions
    let md = "normal line\n[インド/いんど]";
    let (_, warnings) = render_with_warnings(md);
    let w = warnings.iter().find(|w| w.code == src_core::parser::codes::RUBY_KATAKANA_HIRAGANA).unwrap();
    assert_eq!(w.line, 2, "warning should be on line 2");
    assert_eq!(w.col, 1, "warning should be at col 1");
}

#[test]
fn test_warning_source_label() {
    let parser = Parser::new_with_source("[インド/いんど]", "test.n.md");
    let w = parser.warnings.iter()
        .find(|w| w.code == src_core::parser::codes::RUBY_KATAKANA_HIRAGANA)
        .expect("expected warning");
    assert_eq!(w.source, "test.n.md");
}

#[test]
fn test_split_source_blocks() {
    use src_core::split_source_blocks;
    let input = "para1\n\npara2\n\npara3";
    let blocks = split_source_blocks(input);
    assert_eq!(blocks.len(), 3, "expected 3 blocks, got: {:?}", blocks);
}

#[test]
fn test_split_source_blocks_code_fence() {
    use src_core::split_source_blocks;
    let input = "intro\n\n```\nline1\n\nline2\n```\n\noutro";
    let blocks = split_source_blocks(input);
    // intro, code fence (atomic), outro
    assert_eq!(blocks.len(), 3, "code fence must be one block: {:?}", blocks);
    assert!(blocks[1].contains("```"), "middle block should be the code fence: {:?}", blocks);
}
