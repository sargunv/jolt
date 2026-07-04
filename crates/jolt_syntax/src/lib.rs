//! Shared lossless syntax tree infrastructure for Jolt.

mod event;
mod kind;
mod language;
mod syntax_tree;

mod red;

pub use event::{CompletedMarker, Event, Marker};
pub use kind::RawSyntaxKind;
pub use language::Language;
pub use red::{SyntaxElement, SyntaxNode, SyntaxToken};
pub use syntax_tree::{
    BuildSyntaxTreeError, SyntaxTokenData, SyntaxTree, SyntaxTrivia, TriviaKind, build_syntax_tree,
};
