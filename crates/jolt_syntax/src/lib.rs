//! Language-agnostic syntax infrastructure: text positions, diagnostics, the
//! lossless arena tree with borrowed cursors, and parser support.

mod cursor;
mod debug_tree;
mod diagnostic;
mod language;
mod parser;
mod text;
mod tree;

pub use cursor::{AstNode, ListElement, SyntaxElement, SyntaxNode, SyntaxToken, TriviaPiece};
pub use debug_tree::debug_tree;
pub use diagnostic::Diagnostic;
pub use language::{Language, RawSyntaxKind};
pub use parser::{
    CompletedMarker, Event, LexedToken, Lexer, Lookahead, LoopGuard, MAX_DEPTH, Marker,
    ParseOutput, Parser, TokenSet,
};
pub use text::{LineCol, LineIndex, TextRange, TextSize};
#[doc(hidden)]
pub use tree::{FactorySlot, NodeSink, ParsedChildren, SyntaxFactory, Trailing};
pub use tree::{NodeId, SyntaxTree, TokenData, TokenId, Trivia, TriviaKind, build_tree};
