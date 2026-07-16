//! Java lexer, parser, and typed syntax wrappers for Jolt.

#[macro_use]
mod schema;
mod kind;
mod language;
mod lexer;
mod nodes;
mod normalization;
mod parser;
mod shape;

#[cfg(test)]
mod schema_audit;

pub use kind::JavaSyntaxKind;
pub use language::JavaLanguage;
pub use lexer::JavaLexer;
pub use nodes::*;
pub use normalization::{JavaDelimiterRemoval, JavaDelimiterSynthesis, RemovalClaim, ReorderClaim};
pub use parser::{JavaParse, parse_compilation_unit};
