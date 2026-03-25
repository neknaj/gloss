#![no_std]
extern crate alloc;

pub mod app_core;
pub mod renderer;

pub use app_core::{AppCore, DocumentState};
pub use renderer::PluginAwareHtmlRenderer;
