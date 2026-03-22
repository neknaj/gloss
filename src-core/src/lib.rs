#![no_std]

extern crate alloc;

pub mod parser;
pub mod html;

pub use parser::{Parser, Event, Tag};
pub use html::push_html;
