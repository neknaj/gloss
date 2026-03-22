#![no_std]

extern crate alloc;

pub mod parser;
pub mod html;

pub use parser::{Parser, Event, Tag};
pub use html::push_html;

#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::html::push_html;
    use alloc::string::String;

    #[test]
    fn test_parser() {
        let markdown = "- User list\n- Second list\n\n1. Number list\n\nCode `hello`\nBreak\\nLine\nEscaped \\{ test \\} \\\\";
        let parser = Parser::new(&markdown);
        let mut html_output = String::new();
        push_html(&mut html_output, parser);
        assert!(html_output.contains("<ul>"), "Missing ul");
        assert!(html_output.contains("<ol>"), "Missing ol");
        assert!(html_output.contains("<code>"), "Missing code");
        assert!(html_output.contains("<br>\n"), "Missing br");
        assert!(html_output.contains("{ test }"), "Missing escape");
    }
}
