use jolt_diagnostics::DiagnosticCodeId;
use jolt_syntax::{Language, LexedToken, RawSyntaxKind};

use crate::KotlinSyntaxKind;
use crate::lexer::KotlinLexer;
use crate::parser::KotlinParseDiagnosticCode;

/// Kotlin language binding for the shared syntax tree infrastructure.
pub enum KotlinLanguage {}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct KotlinNormalizationAuthority(pub(crate) ());

pub(crate) const NORMALIZATION_AUTHORITY: KotlinNormalizationAuthority =
    KotlinNormalizationAuthority(());

impl Language for KotlinLanguage {
    type Kind = KotlinSyntaxKind;

    type Lexer<'source> = KotlinLexer<'source>;
    type NormalizationAuthority = KotlinNormalizationAuthority;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        KotlinSyntaxKind::from_raw(raw).expect("raw Kotlin syntax kind must be valid")
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        kind.to_raw()
    }

    fn eof_kind() -> Self::Kind {
        KotlinSyntaxKind::Eof
    }

    fn error_node_kind() -> Self::Kind {
        KotlinSyntaxKind::ErrorNode
    }

    fn expected_diagnostic_code() -> DiagnosticCodeId {
        KotlinParseDiagnosticCode::ExpectedSyntax.id()
    }

    fn unexpected_diagnostic_code() -> DiagnosticCodeId {
        KotlinParseDiagnosticCode::UnexpectedSyntax.id()
    }

    fn split_token(token: &LexedToken<Self>) -> Option<&'static [Self::Kind]>
    where
        Self: Sized,
    {
        match token.kind {
            KotlinSyntaxKind::SafeAccess => {
                Some(&[KotlinSyntaxKind::Question, KotlinSyntaxKind::Dot])
            }
            _ => None,
        }
    }
}
