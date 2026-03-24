use src_core::parser::Parser;
use src_plugin::config::GlossConfig;
use src_plugin::host::GlossPluginHost;
use src_plugin::renderer::PluginAwareRenderer;

fn render_no_plugins(markdown: &str) -> String {
    let parser = Parser::new_with_source(markdown, "test.n.md");
    let events: Vec<_> = parser.collect();
    let cfg = GlossConfig::default();
    let mut host = GlossPluginHost { plugins: vec![] };
    let mut out = String::new();
    let mut renderer = PluginAwareRenderer::new(&mut host, &cfg);
    renderer.render(&events, &mut out, "test.n.md", markdown);
    out
}

#[test]
fn renders_paragraph_without_plugins() {
    let html = render_no_plugins("Hello, world.");
    assert!(html.contains("<p>"), "got: {html}");
    assert!(html.contains("Hello, world."), "got: {html}");
}

#[test]
fn renders_code_block_without_plugins() {
    use src_core::html::push_html;
    let markdown = "```rust\nfn main() {}\n```";
    let expected = {
        let parser = Parser::new_with_source(markdown, "test.n.md");
        let mut out = String::new();
        push_html(&mut out, parser);
        out
    };
    let html = render_no_plugins(markdown);
    assert_eq!(html, expected, "got: {html}");
}

#[test]
fn renders_card_link_without_plugins() {
    use src_core::html::push_html;
    let markdown = "@[card](https://example.com)";
    let expected = {
        let parser = Parser::new_with_source(markdown, "test.n.md");
        let mut out = String::new();
        push_html(&mut out, parser);
        out
    };
    let html = render_no_plugins(markdown);
    assert_eq!(html, expected, "got: {html}");
}

#[test]
fn renders_heading_and_front_matter_without_plugins() {
    let markdown = "---\ntitle: \"Test\"\n---\n\n# Hello\n";
    let html = render_no_plugins(markdown);
    assert!(html.contains("<h1>"), "got: {html}");
    assert!(html.contains("nm-frontmatter"), "got: {html}");
}

#[test]
fn renderer_with_default_config_produces_same_output_as_push_html() {
    use src_core::html::push_html;
    let markdown = "# Hello\n\nSome *text*.\n\n```rust\nfn main() {}\n```\n";
    let push_html_output = {
        let parser = Parser::new_with_source(markdown, "test.n.md");
        let mut out = String::new();
        push_html(&mut out, parser);
        out
    };
    let plugin_output = render_no_plugins(markdown);
    assert_eq!(push_html_output, plugin_output,
        "PluginAwareRenderer with no plugins should produce identical output to push_html");
}
