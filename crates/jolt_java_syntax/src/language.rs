use jolt_diagnostics::DiagnosticCodeId;
use jolt_syntax::{Language, LexedToken, RawSyntaxKind};

use crate::JavaSyntaxKind;
use crate::lexer::JavaLexer;
use crate::parser::JavaParseDiagnosticCode;

/// Java language binding for the shared syntax tree infrastructure.
pub enum JavaLanguage {}

impl Language for JavaLanguage {
    type Kind = JavaSyntaxKind;

    type Lexer<'source> = JavaLexer<'source>;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        JavaSyntaxKind::from_raw(raw).expect("raw Java syntax kind must be valid")
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        kind.to_raw()
    }

    fn eof_kind() -> Self::Kind {
        JavaSyntaxKind::Eof
    }

    fn error_node_kind() -> Self::Kind {
        JavaSyntaxKind::ErrorNode
    }

    fn expected_diagnostic_code() -> DiagnosticCodeId {
        JavaParseDiagnosticCode::ExpectedSyntax.id()
    }

    fn unexpected_diagnostic_code() -> DiagnosticCodeId {
        JavaParseDiagnosticCode::UnexpectedSyntax.id()
    }

    fn split_token(token: &LexedToken<Self>) -> Option<&'static [Self::Kind]>
    where
        Self: Sized,
    {
        match token.kind {
            JavaSyntaxKind::GtEq => Some(&[JavaSyntaxKind::Gt, JavaSyntaxKind::Assign]),
            JavaSyntaxKind::RShift => Some(&[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt]),
            JavaSyntaxKind::UnsignedRShift => {
                Some(&[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt, JavaSyntaxKind::Gt])
            }
            JavaSyntaxKind::RShiftEq => Some(&[
                JavaSyntaxKind::Gt,
                JavaSyntaxKind::Gt,
                JavaSyntaxKind::Assign,
            ]),
            JavaSyntaxKind::UnsignedRShiftEq => Some(&[
                JavaSyntaxKind::Gt,
                JavaSyntaxKind::Gt,
                JavaSyntaxKind::Gt,
                JavaSyntaxKind::Assign,
            ]),
            _ => None,
        }
    }
}
