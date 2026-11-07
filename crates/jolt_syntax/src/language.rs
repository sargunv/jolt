//! Raw kinds and the static facts a language provides about its tree.

/// A language's syntax kind as stored in the tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawSyntaxKind(pub u16);

/// Static facts about one language's tree. Implemented by a unit struct
/// (`JavaLanguage`).
pub trait Language: Copy + Eq + std::fmt::Debug + 'static {
    type Kind: Copy + Eq + std::fmt::Debug;
    const EOF: Self::Kind;
    /// The single bogus node kind, accepted at every category slot and list
    /// item position.
    const BOGUS: Self::Kind;
    /// Bracket pairs that [`crate::Parser::recover_too_deep`] keeps balanced.
    const BRACKETS: &'static [(Self::Kind, Self::Kind)];
    /// Code of the diagnostic [`crate::Parser::expect`] reports.
    const EXPECTED_CODE: &'static str;
    /// Code of the diagnostic [`crate::LoopGuard::check`] reports.
    const UNEXPECTED_CODE: &'static str;
    /// Code of the diagnostic [`crate::Parser::recover_too_deep`] reports.
    const TOO_DEEP_CODE: &'static str;
    fn to_raw(kind: Self::Kind) -> RawSyntaxKind;
    fn from_raw(raw: RawSyntaxKind) -> Self::Kind;
    /// List nodes (variable-length children). Comment placement never
    /// attaches to a list node, only to its items.
    fn is_list(raw: RawSyntaxKind) -> bool;
    fn kind_name(raw: RawSyntaxKind) -> &'static str;
}
