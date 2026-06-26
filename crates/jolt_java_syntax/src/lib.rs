//! Java lexer, parser, and typed syntax wrappers for Jolt.

mod kind;
mod language;
mod lexer;
mod nodes;
mod parser;

pub use jolt_diagnostics::{
    Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
pub use kind::JavaSyntaxKind;
pub use lexer::{
    JavaLexDiagnosticCode, JavaLexer, JavaLexerCheckpoint, JavaTokenSource,
    JavaTokenSourceCheckpoint, LexerDiagnostic, Token, Trivia, TriviaKind,
};
pub use nodes::*;
pub use parser::{JavaParse, JavaParseDiagnosticCode, parse_compilation_unit};
