use crate::RawSyntaxKind;

/// A language binding for shared syntax tree infrastructure.
pub trait Language: 'static {
    /// The language-specific syntax kind enum.
    type Kind: Copy + Eq;

    /// Converts a raw kind stored in shared syntax data to a language kind.
    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind;

    /// Converts a language kind to the raw representation used by shared syntax data.
    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind;
}
