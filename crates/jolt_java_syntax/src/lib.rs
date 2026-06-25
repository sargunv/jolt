//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod kind;
mod lexer;
mod parser;

pub use kind::JavaSyntaxKind;
pub use lexer::{
    JavaLexer, JavaLexerCheckpoint, JavaTokenSource, JavaTokenSourceCheckpoint, LexerDiagnostic,
    LexerDiagnosticKind, Token, Trivia, TriviaKind,
};
