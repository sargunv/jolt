use jolt_text::TextSize;

use crate::RawSyntaxKind;

use super::{GreenNode, GreenToken};

/// A child element in a green tree.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GreenElement {
    /// A nested green node.
    Node(GreenNode),
    /// A green token.
    Token(GreenToken),
}

impl GreenElement {
    /// Returns the kind stored on this element.
    #[must_use]
    pub fn kind(&self) -> RawSyntaxKind {
        match self {
            Self::Node(node) => node.kind(),
            Self::Token(token) => token.kind(),
        }
    }

    /// Returns this element's full byte length.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        match self {
            Self::Node(node) => node.text_len(),
            Self::Token(token) => token.text_len(),
        }
    }

    /// Returns this element as a node, if it is one.
    #[must_use]
    pub const fn as_node(&self) -> Option<&GreenNode> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }

    /// Returns this element as a token, if it is one.
    #[must_use]
    pub const fn as_token(&self) -> Option<&GreenToken> {
        match self {
            Self::Node(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl From<GreenNode> for GreenElement {
    fn from(node: GreenNode) -> Self {
        Self::Node(node)
    }
}

impl From<GreenToken> for GreenElement {
    fn from(token: GreenToken) -> Self {
        Self::Token(token)
    }
}
