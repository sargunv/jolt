//! Shared formatter document IR and renderer for Jolt.

mod document;
mod render;
#[cfg(test)]
mod tests;
mod validation;
mod width;

pub use document::{
    Doc, FillEntry, FlatLine, GroupId, align, break_, break_parent, concat, dedent, dedent_by,
    empty_line, fill, fill_entry, flat_text, flat_text_with_width, force_group, force_group_id,
    group, group_id, hard_line, hard_line_without_break_parent, if_break, if_group_breaks, indent,
    indent_by, indent_if_break, join, line, line_suffix, line_suffix_boundary, literal_text,
    literal_text_with_line_widths, literal_text_with_width, nil, soft_line, text, text_with_width,
};
pub use render::{
    IndentStyle, LineEnding, RenderError, RenderOptions, RenderStats, Rendered, render,
};
pub use width::TextWidth;
