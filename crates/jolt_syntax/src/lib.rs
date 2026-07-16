//! Shared lossless syntax tree infrastructure for Jolt.

mod comment;
mod conservation;
mod event;
mod kind;
mod language;
mod normalization;
mod parser;
#[doc(hidden)]
pub mod schema;
mod syntax_tree;

mod red;

pub use comment::{Comment, CommentKind, Comments};
pub use conservation::{
    ConservationError, SourceIdentity, SourceTokenId, SourceTriviaId, SourceTriviaPiece,
    SourceTriviaSide, SyntaxConservationTracker, SyntaxVerbatimCore,
};
pub use event::{CompletedMarker, Event, Marker, NodeAnchor};
pub use kind::RawSyntaxKind;
pub use language::Language;
pub use normalization::{
    NormalizedToken, RemovalClaim, RemovalReason, ReorderClaim, ReorderReason, ReplacementClaim,
    SynthesisClaim,
};
pub use parser::{
    CursorCheckpoint, DiagnosticMarker, LanguageLexer, LexedToken, ParseEvents, Parser,
    TokenBuffer, TokenCursor, UnresolvedDiagnosticOwner,
};
pub use red::{
    SyntaxElement, SyntaxNode, SyntaxSlot, SyntaxToken, represented_range_is_trivia,
    tokens_between, tokens_have_blank_line_between,
};
#[cfg(feature = "bench")]
pub use syntax_tree::SyntaxTreeMetrics;
pub use syntax_tree::{
    BuildSyntaxTreeError, SyntaxDiagnosticOwner, SyntaxNodeId, SyntaxTokenData, SyntaxTree,
    SyntaxTrivia, TriviaKind,
};
#[doc(hidden)]
pub use syntax_tree::{
    FactoryNode, FactorySlot, ParsedChildren, SyntaxFactory, SyntaxTreeSink,
    build_syntax_tree_with_factory, build_syntax_tree_with_factory_and_diagnostic_owners,
};
