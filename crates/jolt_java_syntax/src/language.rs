use jolt_syntax::{Language, RawSyntaxKind};

use crate::JavaSyntaxKind;

/// Java language binding for the shared syntax tree infrastructure.
pub(crate) enum JavaLanguage {}

impl Language for JavaLanguage {
    type Kind = JavaSyntaxKind;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        JavaSyntaxKind::from_raw(raw).expect("raw Java syntax kind must be valid")
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        kind.to_raw()
    }
}
