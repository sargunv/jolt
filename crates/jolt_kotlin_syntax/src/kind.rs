use jolt_syntax::RawSyntaxKind;
use num_enum::{IntoPrimitive, TryFromPrimitive};

macro_rules! define_kotlin_syntax_kind {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        /// A Kotlin token or syntax node kind.
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, Eq, Hash, IntoPrimitive, PartialEq, TryFromPrimitive)]
        #[allow(clippy::enum_variant_names)]
        pub enum KotlinSyntaxKind {
            $($token,)*
            $($kind,)*
            $($bogus,)*
        }
    };
}

kotlin_syntax_schema!(define_kotlin_syntax_kind);

impl KotlinSyntaxKind {
    /// Converts this kind into the raw representation used by shared syntax data.
    #[must_use]
    pub(crate) fn to_raw(self) -> RawSyntaxKind {
        RawSyntaxKind::new(u16::from(self))
    }

    /// Converts a raw kind back into a Kotlin syntax kind.
    #[must_use]
    pub(crate) fn from_raw(raw: RawSyntaxKind) -> Option<Self> {
        Self::try_from(raw.get()).ok()
    }
}
