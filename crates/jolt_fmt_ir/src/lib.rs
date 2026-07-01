//! Shared formatter document IR and renderer for Jolt.

mod document;
mod render;
#[cfg(test)]
mod tests;
mod validation;
mod width;

pub use document::{
    BreakMarkerId, Doc, FillEntry, FlatLine, GroupFit, GroupId, LevelBreak, LevelBreakMode, align,
    best_fitting, break_, break_level, break_level_with_indent, break_parent, concat, empty_line,
    fill, fill_entry, flat_text, flat_text_with_width, force_group, force_group_id, group,
    group_id, group_with_fit, hard_line, hard_line_without_break_parent, if_break, if_group_breaks,
    indent, indent_by, indent_if_break, join, level_break, level_break_with_prefix, line,
    line_suffix, line_suffix_boundary, literal_text, literal_text_with_line_widths,
    literal_text_with_width, marked_break, nil, soft_line, text, text_with_width,
};
pub use render::{
    IndentStyle, LineEnding, RenderError, RenderOptions, RenderStats, Rendered, render,
};
pub use width::TextWidth;
