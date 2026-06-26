use std::fmt;

use jolt_text::{TextRange, TextSize};

use crate::{Language, RawSyntaxKind};

use super::{SyntaxNode, SyntaxToken};

/// A parent-aware syntax node or token.
pub enum SyntaxElement<L: Language> {
    /// A syntax node.
    Node(SyntaxNode<L>),
    /// A syntax token.
    Token(SyntaxToken<L>),
}

impl<L: Language> SyntaxElement<L> {
    /// Returns the language-specific kind for this element.
    #[must_use]
    pub fn kind(&self) -> L::Kind {
        L::kind_from_raw(self.raw_kind())
    }

    /// Returns the raw kind for this element.
    #[must_use]
    pub fn raw_kind(&self) -> RawSyntaxKind {
        match self {
            Self::Node(node) => node.raw_kind(),
            Self::Token(token) => token.raw_kind(),
        }
    }

    /// Returns this element's parent node.
    #[must_use]
    pub fn parent(&self) -> Option<SyntaxNode<L>> {
        match self {
            Self::Node(node) => node.parent(),
            Self::Token(token) => Some(token.parent()),
        }
    }

    /// Returns this element's index among its parent's green children.
    #[must_use]
    pub fn index(&self) -> usize {
        match self {
            Self::Node(node) => node.index(),
            Self::Token(token) => token.index(),
        }
    }

    /// Returns the byte offset where this element starts.
    #[must_use]
    pub fn offset(&self) -> TextSize {
        match self {
            Self::Node(node) => node.offset(),
            Self::Token(token) => token.offset(),
        }
    }

    /// Returns the byte length covered by this element.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        match self {
            Self::Node(node) => node.text_len(),
            Self::Token(token) => token.text_len(),
        }
    }

    /// Returns the full source range covered by this element.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        match self {
            Self::Node(node) => node.text_range(),
            Self::Token(token) => token.text_range(),
        }
    }

    /// Returns this element as a node, if it is one.
    #[must_use]
    pub fn into_node(self) -> Option<SyntaxNode<L>> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }

    /// Returns this element as a token, if it is one.
    #[must_use]
    pub fn into_token(self) -> Option<SyntaxToken<L>> {
        match self {
            Self::Node(_) => None,
            Self::Token(token) => Some(token),
        }
    }
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
