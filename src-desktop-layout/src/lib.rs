#![no_std]
extern crate alloc;

pub mod model;
pub mod update;
pub mod view;

pub use model::{Model, WorkspaceState, LayoutState, PanelNode, Pane, PreviewState};
pub use update::update;
pub use view::view;
