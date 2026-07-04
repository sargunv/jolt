use std::fmt;

use crate::Language;

use super::{SyntaxNode, SyntaxToken};

/// A parent-aware syntax node or token.
pub enum SyntaxElement<'source, L: Language> {
    /// A syntax node.
    Node(SyntaxNode<'source, L>),
    /// A syntax token.
    Token(SyntaxToken<'source, L>),
}

impl<L: Language> Clone for SyntaxElement<'_, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: Language> Copy for SyntaxElement<'_, L> {}

impl<L> fmt::Debug for SyntaxElement<'_, L>
where
    L: Language,
    L::Kind: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(node) => node.fmt(f),
            Self::Token(token) => token.fmt(f),
        }
    }
}
