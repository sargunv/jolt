use jolt_syntax::{Language, RawSyntaxKind};

use crate::KotlinSyntaxKind;

/// Kotlin language binding for the shared syntax tree infrastructure.
pub(crate) enum KotlinLanguage {}

impl Language for KotlinLanguage {
    type Kind = KotlinSyntaxKind;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        KotlinSyntaxKind::from_raw(raw).expect("raw Kotlin syntax kind must be valid")
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        kind.to_raw()
    }
}
