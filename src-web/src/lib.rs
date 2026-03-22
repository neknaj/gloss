use wasm_bindgen::prelude::*;
use src_core::{Parser, Event, push_html};

#[wasm_bindgen]
pub fn parse_to_html(input: &str) -> String {
    let parser = Parser::new(input);
    let mut html_output = String::new();
    push_html(&mut html_output, parser);
    html_output
}
