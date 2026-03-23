use wasm_bindgen::prelude::*;
use src_core::{Parser, push_html};
use web_sys::{HtmlTextAreaElement, HtmlElement};

/// Render gloss markdown to HTML body fragment (callable from JS)
#[wasm_bindgen]
pub fn render_markdown(input: &str) -> String {
    let parser = Parser::new(input);
    let mut out = String::new();
    push_html(&mut out, parser);
    out
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let editor = document
        .get_element_by_id("editor")
        .expect("no editor found")
        .dyn_into::<HtmlTextAreaElement>()?;
    let preview = document
        .get_element_by_id("preview")
        .expect("no preview found")
        .dyn_into::<HtmlElement>()?;

    // Wire the input event listener for live rendering
    let render_editor = editor.clone();
    let render_preview = preview.clone();
    let render = Closure::<dyn FnMut()>::new(move || {
        let markdown = render_editor.value();
        let parser = Parser::new(&markdown);
        let mut html_output = String::new();
        push_html(&mut html_output, parser);
        render_preview.set_inner_html(&html_output);
    });

    editor.add_event_listener_with_callback("input", render.as_ref().unchecked_ref())?;

    // Dispatch a custom event so JS knows WASM is ready (JS will then fetch sample and set value)
    let event = web_sys::Event::new("wasm-ready")?;
    window.dispatch_event(&event)?;

    render.forget();

    Ok(())
}
