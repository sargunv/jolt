//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod lexer;

pub use lexer::{
    JavaSyntaxKind, Lexed, LexerDiagnostic, LexerDiagnosticKind, Token, Trivia, TriviaKind, lex,
};
