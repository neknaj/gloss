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
        let markdown = "# Hello Gloss!\n\nThis is a test of `[漢字/かんじ]` and `{Gloss/Test}`.";
        let parser = Parser::new(&markdown);
        let mut html_output = String::new();
        push_html(&mut html_output, parser);
        assert!(!html_output.is_empty(), "HTML OUTPUT IS EMPTY!");
    }
}
