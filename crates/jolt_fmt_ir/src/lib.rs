//! Shared formatter document IR and renderer for Jolt.

mod document;
pub mod formatter_ignore;
mod options;
mod render;
mod width;

pub use document::{Doc, DocArena, DocBuilder, DocId, DocList};
pub use options::{FormatOptions, FormatSinkResult};
pub use render::{
    IndentStyle, RenderControl, RenderError, RenderOptions, RenderOutcome, RenderSink, render_to,
};
pub use width::TextWidth;
