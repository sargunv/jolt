//! Shared formatter document IR and renderer for Jolt.

mod comment_text;
mod comments;
mod document;
pub mod formatter_ignore;
mod lists;
mod options;
mod recovery;
mod render;
mod root;
mod source_fragment;
mod token_trivia;
mod width;

pub use comment_text::{
    StarBlockOpener, format_comment_lines, format_star_block_comment,
    is_empty_single_line_block_comment, is_star_block_comment, preserved_block_comment_lines,
    preserved_comment_lines,
};
pub use comments::{
    InlineLeadingTrivia, comment_forces_line, comment_is_star_block, format_comment,
    format_dangling_comments, format_delimiter_dangling_comments, format_ignored_trivia,
    format_inline_trailing_comment_list, format_leading_comment_list, format_leading_comments,
    format_removed_comments, format_separator_with_comments, format_token,
    format_token_after_relocated_leading_comments, format_token_body,
    format_token_with_inline_leading_comments, format_trailing_comment,
    format_trailing_comments_before_line_break, has_delimiter_dangling_comments,
    token_has_comments, trailing_comments_force_line,
};
#[cfg(feature = "bench")]
pub use document::DocArenaMetrics;
pub use document::{ConcatBuilder, Doc, DocBuilder};
pub use lists::{
    BodyItemSeparator, CommaListItem, attach_comma_separator, comma_list, comma_list_parts,
};
pub use options::{FormatOptions, FormatSinkResult};
pub use recovery::{
    FormatDelimiter, FormatField, FormatListPart, LayoutDoc, assemble_malformed_fragment,
    format_malformed, format_malformed_core, format_missing, format_optional_field,
    format_required_field, resolve_list_part, resolve_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
pub use render::{RenderControl, RenderError, RenderSink};
#[doc(hidden)]
pub use root::{FormatRootMetrics, format_root_to_sink};
pub use source_fragment::{ExceptionalSeparator, LexicalAtom, LexicalAtomKind, LexicalSafety};
pub use token_trivia::{LeadingTrivia, TrailingTrivia, format_token_doc};
