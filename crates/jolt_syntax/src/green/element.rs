use jolt_text::TextSize;

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
    pub(crate) fn text_len(&self) -> TextSize {
        match self {
            Self::Node(node) => node.text_len(),
            Self::Token(token) => token.text_len(),
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
