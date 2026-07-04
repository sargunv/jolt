use std::fmt;

use crate::Language;

use super::{SyntaxNode, SyntaxToken};

/// A parent-aware syntax node or token.
pub enum SyntaxElement<L: Language> {
    /// A syntax node.
    Node(SyntaxNode<L>),
    /// A syntax token.
    Token(SyntaxToken<L>),
}

impl<L: Language> Clone for SyntaxElement<L> {
    fn clone(&self) -> Self {
        match self {
            Self::Node(node) => Self::Node(node.clone()),
            Self::Token(token) => Self::Token(token.clone()),
        }
    }
}

impl<L> fmt::Debug for SyntaxElement<L>
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

impl<L: Language> From<SyntaxNode<L>> for SyntaxElement<L> {
    fn from(node: SyntaxNode<L>) -> Self {
        Self::Node(node)
    }
}

impl<L: Language> From<SyntaxToken<L>> for SyntaxElement<L> {
    fn from(token: SyntaxToken<L>) -> Self {
        Self::Token(token)
    }
}
