//! Shared lossless syntax tree infrastructure for Jolt.

mod comment;
mod event;
mod kind;
mod language;
mod parser;
mod syntax_tree;

mod red;

pub use comment::{Comment, CommentKind, Comments};
pub use event::{CompletedMarker, Event, Marker};
pub use kind::RawSyntaxKind;
pub use language::Language;
pub use parser::{
    CursorCheckpoint, LanguageLexer, LexedToken, ParseEvents, Parser, TokenBuffer, TokenCursor,
};
pub use red::{
    SyntaxElement, SyntaxNode, SyntaxToken, represented_range_is_trivia, tokens_between,
    tokens_have_blank_line_between,
};
#[cfg(feature = "bench")]
pub use syntax_tree::SyntaxTreeMetrics;
pub use syntax_tree::{
    BuildSyntaxTreeError, SyntaxTokenData, SyntaxTree, SyntaxTrivia, TriviaKind, build_syntax_tree,
};
