use std::fmt;

/// A language-specific syntax kind stored in shared syntax data.
///
/// The shared tree infrastructure treats this as an opaque value. Language
/// crates map their own syntax kind enums to and from this raw representation.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RawSyntaxKind(u16);

impl RawSyntaxKind {
    /// Creates a raw syntax kind from its numeric representation.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the numeric representation of this syntax kind.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for RawSyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawSyntaxKind({})", self.0)
    }
}
