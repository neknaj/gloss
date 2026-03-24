#![no_std]

extern crate alloc;

pub mod parser;
pub mod html;

pub use parser::{Parser, Event, Tag, Warning, FrontMatterField, fnv1a, split_source_blocks};
pub use html::{push_html, push_html_with_ids, HtmlRenderer, escape_html};

#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::html::push_html;
    use alloc::string::String;

    #[test]
    fn test_parser() {
        let markdown = "> block\n\n| H1 | H2 |\n|:---|---:|\n| **bold** | *italic* ~~strike~~ |\n\n[link](url) ![img](url)";
        let parser = Parser::new(markdown);
        let mut html_output = String::new();
        push_html(&mut html_output, parser);

        assert!(html_output.contains("<blockquote class=\"nm-blockquote\">"), "Missing blockquote");
        assert!(html_output.contains("<table class=\"nm-table\">"), "Missing table");
        assert!(html_output.contains("style=\"text-align:left\""), "Missing td align left");
        assert!(html_output.contains("style=\"text-align:right\""), "Missing td align right");
        assert!(html_output.contains("<strong>bold</strong>"), "Missing strong");
        assert!(html_output.contains("<em>italic</em>"), "Missing em");
        assert!(html_output.contains("<del>strike</del>"), "Missing strike");
        assert!(html_output.contains("<a href=\"url\">link</a>"), "Missing link");
        assert!(html_output.contains("<img src=\"url\" alt=\"img\" class=\"nm-image\"/>"), "Missing image");
    }
}
