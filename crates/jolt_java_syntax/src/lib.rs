//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod kind;
mod language;
mod lexer;
mod parser;

pub use jolt_diagnostics::{
    Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
pub use kind::JavaSyntaxKind;
pub use language::JavaLanguage;
pub use lexer::{
    JavaLexDiagnosticCode, JavaLexer, JavaLexerCheckpoint, JavaTokenSource,
    JavaTokenSourceCheckpoint, LexerDiagnostic, Token, Trivia, TriviaKind,
};
pub use parser::{
    JavaParse, JavaParseDiagnosticCode, JavaSyntaxElement, JavaSyntaxNode, JavaSyntaxToken,
    parse_compilation_unit,
};
