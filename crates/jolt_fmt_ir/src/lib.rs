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
pub use options::{FormatOptions, FormatSinkResult};
pub use render::{
    IndentStyle, RenderControl, RenderError, RenderOptions, RenderOutcome, RenderSink,
    TrackedRenderOutcome, render_to, render_to_tracked,
};
pub use source_fragment::{
    CompletedRenderProof, ExceptionalFragment, ExceptionalSeparator, FragmentBoundary, LexicalAtom,
    LexicalAtomKind, LexicalSafety, NormalizedToken, RemovalClaim, RemovalReason, RenderProof,
    RenderedSourceFragment, ReplacementClaim, SourceFragmentKind, SynthesisClaim,
};
pub use width::TextWidth;
