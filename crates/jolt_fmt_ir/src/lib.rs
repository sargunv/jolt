//! Shared formatter document IR and renderer for Jolt.

mod document;
pub mod formatter_ignore;
mod options;
mod render;
mod validation;
mod width;

pub use document::{
    Doc, concat, empty_line, force_group, group, hard_line, if_break, indent, join, line,
    literal_text, nil, soft_line, space, text,
};
pub use options::{FormatOptions, FormatSinkResult};
pub use render::{
    IndentStyle, RenderControl, RenderError, RenderOptions, RenderOutcome, RenderSink, render_to,
};
pub use width::TextWidth;
