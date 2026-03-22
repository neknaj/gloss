use wasm_bindgen::prelude::*;
use src_core::{Parser, push_html};
use web_sys::{HtmlTextAreaElement, HtmlElement};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    let editor = document.get_element_by_id("editor").expect("no editor found").dyn_into::<HtmlTextAreaElement>()?;
    let preview = document.get_element_by_id("preview").expect("no preview found").dyn_into::<HtmlElement>()?;
    
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
    
    // Initial content
    editor.set_value("# Hello Gloss!\n\nThis is a test of `[漢字/かんじ]` and `{Gloss/Test}`.");
    
    // Execute one initial render
    let markdown = editor.value();
    let parser = Parser::new(&markdown);
    let mut init_html = String::new();
    push_html(&mut init_html, parser);
    preview.set_inner_html(&init_html);
    
    render.forget();
    
    Ok(())
}

