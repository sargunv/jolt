//! Shared formatter document IR and renderer for Jolt.

mod comment_text;
mod document;
pub mod formatter_ignore;
mod options;
mod recovery;
mod render;
mod source_fragment;
mod token_trivia;
mod width;

pub use comment_text::{
    format_comment_lines, format_star_block_comment, is_empty_single_line_block_comment,
    is_star_block_comment, normalize_star_block_body_line, preserved_block_comment_lines,
    preserved_comment_lines, strip_block_comment_delimiters, universal_comment_lines,
};
#[cfg(feature = "bench")]
pub use document::DocArenaMetrics;
pub use document::{ConcatBuilder, Doc, DocArena, DocBuilder, DocId};
pub use jolt_syntax::{
    NormalizedToken, RemovalClaim, RemovalReason, ReorderClaim, ReplacementClaim, SynthesisClaim,
};
pub use options::{FormatOptions, FormatSinkResult};
pub use recovery::{
    FormatField, assemble_malformed_fragment, format_optional_field, format_required_field,
};
pub use render::{
    IndentStyle, RenderControl, RenderError, RenderOptions, RenderOutcome, RenderSink,
    SourceRenderOutcome, render_source_to, render_to,
};
pub use source_fragment::{
    ExceptionalFragment, ExceptionalSeparator, FragmentBoundary, LexicalAtom, LexicalAtomKind,
    LexicalSafety,
};
pub use token_trivia::{LeadingTrivia, TrailingTrivia, format_token_doc};
pub use width::TextWidth;
