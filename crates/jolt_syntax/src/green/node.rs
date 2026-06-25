use std::sync::Arc;

use jolt_text::TextSize;

use crate::RawSyntaxKind;

use super::GreenElement;

/// An immutable parentless green node.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GreenNode(Arc<GreenNodeData>);

#[derive(Debug, Eq, Hash, PartialEq)]
struct GreenNodeData {
    kind: RawSyntaxKind,
    text_len: TextSize,
    children: Box<[GreenElement]>,
}

impl GreenNode {
    /// Creates a green node from already-built child elements.
    #[must_use]
    pub fn new(kind: RawSyntaxKind, children: impl IntoIterator<Item = GreenElement>) -> Self {
        let children = children.into_iter().collect::<Box<[_]>>();
        let text_len = children_text_len(&children);

        Self(Arc::new(GreenNodeData {
            kind,
            text_len,
            children,
        }))
    }

    /// Returns the node kind.
    #[must_use]
    pub fn kind(&self) -> RawSyntaxKind {
        self.0.kind
    }

    /// Returns the byte length covered by this node.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        self.0.text_len
    }

    /// Returns this node's direct children.
    #[must_use]
    pub fn children(&self) -> &[GreenElement] {
        &self.0.children
    }

    /// Returns true if both handles point at the same green node allocation.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

fn children_text_len(children: &[GreenElement]) -> TextSize {
    TextSize::new(children.iter().map(|child| child.text_len().get()).sum())
}
