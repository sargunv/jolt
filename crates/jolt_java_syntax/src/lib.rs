//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod lexer;

pub use lexer::{
    JavaLexer, JavaLexerCheckpoint, JavaSyntaxKind, JavaTokenSource, JavaTokenSourceCheckpoint,
    LexerDiagnostic, LexerDiagnosticKind, Token, Trivia, TriviaKind,
};
