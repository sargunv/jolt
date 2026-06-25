//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod kind;
mod language;
mod lexer;
mod parser;

pub use kind::JavaSyntaxKind;
pub use language::JavaLanguage;
pub use lexer::{
    JavaLexer, JavaLexerCheckpoint, JavaTokenSource, JavaTokenSourceCheckpoint, LexerDiagnostic,
    LexerDiagnosticKind, Token, Trivia, TriviaKind,
};
pub use parser::{
    JavaParse, JavaSyntaxElement, JavaSyntaxNode, JavaSyntaxToken, parse_compilation_unit,
};
