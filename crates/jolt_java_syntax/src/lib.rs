//! Java lexer, parser, and the syntax kinds and typed nodes generated from the
//! Java schema (`tests/codegen/schema.rs`).

mod generated;
mod lexer;
mod parser;
mod semantics;

use jolt_syntax::{Language, RawSyntaxKind};

pub(crate) use generated::JavaSyntaxFactory;
pub use generated::*;
pub use parser::{JavaParse, parse};
pub use semantics::{ModifierPartition, binary_precedence, is_assignment_operator, is_contextual};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JavaLanguage;

impl Language for JavaLanguage {
    type Kind = JavaSyntaxKind;
    const EOF: JavaSyntaxKind = JavaSyntaxKind::Eof;
    const BOGUS: JavaSyntaxKind = JavaSyntaxKind::Bogus;
    const BRACKETS: &'static [(JavaSyntaxKind, JavaSyntaxKind)] = &[
        (JavaSyntaxKind::LParen, JavaSyntaxKind::RParen),
        (JavaSyntaxKind::LBracket, JavaSyntaxKind::RBracket),
        (JavaSyntaxKind::LBrace, JavaSyntaxKind::RBrace),
    ];
    const EXPECTED_CODE: &'static str = parser::codes::EXPECTED;
    const UNEXPECTED_CODE: &'static str = parser::codes::UNEXPECTED;
    const TOO_DEEP_CODE: &'static str = parser::codes::TOO_DEEP;

    fn to_raw(kind: JavaSyntaxKind) -> RawSyntaxKind {
        kind.to_raw()
    }

    fn from_raw(raw: RawSyntaxKind) -> JavaSyntaxKind {
        JavaSyntaxKind::from_raw(raw)
    }

    fn is_list(raw: RawSyntaxKind) -> bool {
        JavaSyntaxKind::from_raw(raw).is_list()
    }

    fn kind_name(raw: RawSyntaxKind) -> &'static str {
        JavaSyntaxKind::from_raw(raw).name()
    }
}
