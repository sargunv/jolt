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
pub use lexer::{JavaLexer, Token, Trivia, TriviaKind};
pub use nodes::*;
pub use parser::{JavaParse, parse_compilation_unit};
