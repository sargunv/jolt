//! Shared lossless syntax tree infrastructure for Jolt.

mod comment;
mod event;
mod kind;
mod language;
mod parser;
mod syntax_tree;

mod red;

pub use comment::{Comment, CommentKind, Comments, trivia_has_blank_line};
pub use event::{CompletedMarker, Event, Marker};
pub use kind::RawSyntaxKind;
pub use language::Language;
pub use parser::{
    CursorCheckpoint, LanguageLexer, LexedToken, ParseEvents, Parser, TokenBuffer, TokenCursor,
};
pub use red::{
    SyntaxElement, SyntaxNode, SyntaxToken, source_gap_is_trivia, source_slice_is_whitespace,
    tokens_between,
};
pub use syntax_tree::{
    BuildSyntaxTreeError, SyntaxTokenData, SyntaxTree, SyntaxTrivia, TriviaKind, build_syntax_tree,
};
