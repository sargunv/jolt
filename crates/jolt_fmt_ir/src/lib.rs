//! Shared formatter document IR and renderer for Jolt.

mod document;
pub mod formatter_ignore;
mod options;
mod render;
mod source_fragment;
mod width;

#[cfg(feature = "bench")]
pub use document::DocArenaMetrics;
pub use document::{ConcatBuilder, Doc, DocArena, DocBuilder, DocId};
pub use jolt_syntax::{
    NormalizedToken, RemovalClaim, RemovalReason, ReorderClaim, ReplacementClaim, SynthesisClaim,
};
pub use options::{FormatOptions, FormatSinkResult};
pub use render::{
    IndentStyle, RenderControl, RenderError, RenderOptions, RenderOutcome, RenderSink,
    SourceRenderOutcome, render_source_to, render_to,
};
pub use source_fragment::{
    ExceptionalFragment, ExceptionalSeparator, FragmentBoundary, LexicalAtom, LexicalAtomKind,
    LexicalSafety,
};
pub use width::TextWidth;
