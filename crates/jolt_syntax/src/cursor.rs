//! Borrowed cursors over a [`SyntaxTree`].

use std::fmt;
use std::hash::{Hash, Hasher};

use crate::tree::{HAS_RECOVERY, MALFORMED, NodeData, Slot, TokenData};
use crate::{NodeId, RawSyntaxKind, SyntaxTree, TextRange, TextSize, TokenId, Trivia, TriviaKind};

/// A tree reference whose equality is identity.
#[derive(Clone, Copy)]
struct TreeRef<'a>(&'a SyntaxTree);

impl PartialEq for TreeRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for TreeRef<'_> {}

impl Hash for TreeRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxNode<'a> {
    tree: TreeRef<'a>,
    source: &'a str,
    id: NodeId,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxToken<'a> {
    tree: TreeRef<'a>,
    source: &'a str,
    id: TokenId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxElement<'a> {
    Node(SyntaxNode<'a>),
    Token(SyntaxToken<'a>),
}

impl<'a> SyntaxElement<'a> {
    #[must_use]
    pub fn kind(self) -> RawSyntaxKind {
        match self {
            Self::Node(node) => node.kind(),
            Self::Token(token) => token.kind(),
        }
    }

    #[must_use]
    pub fn first_token(self) -> Option<SyntaxToken<'a>> {
        match self {
            Self::Node(node) => node.first_token(),
            Self::Token(token) => Some(token),
        }
    }

    #[must_use]
    pub fn last_token(self) -> Option<SyntaxToken<'a>> {
        match self {
            Self::Node(node) => node.last_token(),
            Self::Token(token) => Some(token),
        }
    }

    #[must_use]
    pub fn text_range(self) -> TextRange {
        match self {
            Self::Node(node) => node.text_range(),
            Self::Token(token) => token.text_range(),
        }
    }

    #[must_use]
    pub const fn as_node(self) -> Option<SyntaxNode<'a>> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }

    #[must_use]
    pub const fn as_token(self) -> Option<SyntaxToken<'a>> {
        match self {
            Self::Node(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl<'a> SyntaxNode<'a> {
    /// The root of `tree`, whose text is `source`.
    #[must_use]
    pub fn root(tree: &'a SyntaxTree, source: &'a str) -> Self {
        Self {
            tree: TreeRef(tree),
            source,
            id: tree.root,
        }
    }

    fn data(self) -> &'a NodeData {
        self.tree.0.node(self.id)
    }

    fn node(self, id: NodeId) -> Self {
        Self { id, ..self }
    }

    fn token(self, id: TokenId) -> SyntaxToken<'a> {
        SyntaxToken {
            tree: self.tree,
            source: self.source,
            id,
        }
    }

    fn element(self, slot: Slot) -> Option<SyntaxElement<'a>> {
        match slot {
            Slot::Node(id) => Some(SyntaxElement::Node(self.node(id))),
            Slot::Token(id) => Some(SyntaxElement::Token(self.token(id))),
            Slot::Empty => None,
        }
    }

    #[must_use]
    pub fn kind(self) -> RawSyntaxKind {
        self.data().kind
    }

    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }

    /// The tree this node belongs to.
    #[must_use]
    pub const fn tree(self) -> &'a SyntaxTree {
        self.tree.0
    }

    #[must_use]
    pub fn parent(self) -> Option<Self> {
        let parent = self.data().parent;
        (parent != u32::MAX).then(|| self.node(NodeId(parent)))
    }

    /// This node's slot index in its parent.
    #[must_use]
    pub fn index(self) -> usize {
        self.data().index as usize
    }

    /// Typed-accessor slot read. Returns `None` for an empty slot, and for
    /// every slot of a malformed node (whose children are raw).
    #[must_use]
    pub fn slot(self, index: usize) -> Option<SyntaxElement<'a>> {
        if self.is_malformed() {
            return None;
        }
        self.element(self.tree.0.node_slots(self.id).get(index)?.unpack())
    }

    /// Every slot in order, `None` for empty ones. Empty for a malformed node.
    pub fn slots(self) -> impl Iterator<Item = Option<SyntaxElement<'a>>> + 'a {
        let slots = if self.is_malformed() {
            &[]
        } else {
            self.tree.0.node_slots(self.id)
        };
        slots.iter().map(move |slot| self.element(slot.unpack()))
    }

    /// Raw non-empty children in order, malformed or not.
    pub fn children(self) -> impl Iterator<Item = SyntaxElement<'a>> + 'a {
        self.tree
            .0
            .node_slots(self.id)
            .iter()
            .filter_map(move |slot| self.element(slot.unpack()))
    }

    #[must_use]
    pub fn first_token(self) -> Option<SyntaxToken<'a>> {
        let [start, end] = self.data().tokens;
        (start < end).then(|| self.token(TokenId(start)))
    }

    #[must_use]
    pub fn last_token(self) -> Option<SyntaxToken<'a>> {
        let [start, end] = self.data().tokens;
        (start < end).then(|| self.token(TokenId(end - 1)))
    }

    pub fn tokens(self) -> impl Iterator<Item = SyntaxToken<'a>> + 'a {
        let [start, end] = self.data().tokens;
        (start..end).map(move |id| self.token(TokenId(id)))
    }

    /// From the first token's text start to the last token's text end (outer
    /// trivia excluded). An empty node gives an empty range at the next
    /// token's text start.
    #[must_use]
    pub fn text_range(self) -> TextRange {
        let [start, end] = self.data().tokens;
        let tokens = &self.tree.0.tokens;
        if start == end {
            let offset = tokens.get(start as usize).map_or_else(
                || {
                    tokens
                        .last()
                        .map_or(TextSize::default(), |token| token.range().end())
                },
                |token| token.range().start(),
            );
            return TextRange::empty(offset);
        }
        TextRange::new(
            tokens[start as usize].range().start(),
            tokens[end as usize - 1].range().end(),
        )
    }

    #[must_use]
    pub fn is_malformed(self) -> bool {
        self.data().flags & MALFORMED != 0
    }

    #[must_use]
    pub fn is_recovery_free(self) -> bool {
        self.data().flags & HAS_RECOVERY == 0
    }

    #[must_use]
    pub const fn source(self) -> &'a str {
        self.source
    }

    /// Iterative preorder walk, including this node.
    pub fn descendants(self) -> impl Iterator<Item = SyntaxElement<'a>> + 'a {
        let mut stack = vec![SyntaxElement::Node(self)];
        std::iter::from_fn(move || {
            let element = stack.pop()?;
            if let SyntaxElement::Node(node) = element {
                let slots = node.tree.0.node_slots(node.id);
                stack.extend(
                    slots
                        .iter()
                        .rev()
                        .filter_map(|slot| node.element(slot.unpack())),
                );
            }
            Some(element)
        })
    }
}

impl<'a> SyntaxToken<'a> {
    fn data(self) -> &'a TokenData {
        self.tree.0.token(self.id)
    }

    #[must_use]
    pub fn kind(self) -> RawSyntaxKind {
        self.data().kind()
    }

    #[must_use]
    pub const fn id(self) -> TokenId {
        self.id
    }

    #[must_use]
    pub fn parent(self) -> SyntaxNode<'a> {
        SyntaxNode {
            tree: self.tree,
            source: self.source,
            id: NodeId(self.data().parent),
        }
    }

    #[must_use]
    pub fn text(self) -> &'a str {
        let range = self.text_range();
        &self.source[range.start().get()..range.end().get()]
    }

    #[must_use]
    pub fn text_range(self) -> TextRange {
        self.data().range()
    }

    pub fn leading(self) -> impl Iterator<Item = TriviaPiece<'a>> + 'a {
        let data = self.data();
        TriviaPieces {
            trivia: &self.tree.0.trivia[data.leading()],
            source: self.source,
            offset: data.full_start(),
        }
    }

    pub fn trailing(self) -> impl Iterator<Item = TriviaPiece<'a>> + 'a {
        let data = self.data();
        TriviaPieces {
            trivia: &self.tree.0.trivia[data.trailing()],
            source: self.source,
            offset: data.range().end(),
        }
    }

    #[must_use]
    pub fn prev(self) -> Option<Self> {
        let id = self.id.0.checked_sub(1)?;
        Some(Self {
            id: TokenId(id),
            ..self
        })
    }

    #[must_use]
    pub fn next(self) -> Option<Self> {
        let id = self.id.0 + 1;
        ((id as usize) < self.tree.0.tokens.len()).then_some(Self {
            id: TokenId(id),
            ..self
        })
    }
}

impl fmt::Debug for SyntaxNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}",
            (self.tree.0.kind_name)(self.kind()),
            self.text_range()
        )
    }
}

impl fmt::Debug for SyntaxToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}",
            (self.tree.0.kind_name)(self.kind()),
            self.text_range()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriviaPiece<'a> {
    pub kind: TriviaKind,
    pub text: &'a str,
    pub range: TextRange,
}

struct TriviaPieces<'a> {
    trivia: &'a [Trivia],
    source: &'a str,
    offset: TextSize,
}

impl<'a> Iterator for TriviaPieces<'a> {
    type Item = TriviaPiece<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (first, rest) = self.trivia.split_first()?;
        self.trivia = rest;
        let start = self.offset;
        self.offset = start + TextSize::from(first.len);
        Some(TriviaPiece {
            kind: first.kind,
            text: &self.source[start.get()..self.offset.get()],
            range: TextRange::new(start, self.offset),
        })
    }
}

/// A typed view of a node.
pub trait AstNode<'a>: Copy {
    fn can_cast(kind: RawSyntaxKind) -> bool;
    fn cast(node: SyntaxNode<'a>) -> Option<Self>;
    fn syntax(self) -> SyntaxNode<'a>;
}

/// One slot of a separated list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListElement<'a, T> {
    Item(T),
    Separator(SyntaxToken<'a>),
    /// A missing item or separator.
    Missing,
    /// A `Bogus` node at an item position whose type cannot hold it.
    Bogus(SyntaxNode<'a>),
}

impl<'a, T> ListElement<'a, T> {
    /// Classifies slot `index` (items at even, separators at odd indices).
    #[doc(hidden)]
    pub fn new(
        index: usize,
        element: Option<SyntaxElement<'a>>,
        cast: impl Fn(SyntaxElement<'a>) -> Option<T>,
    ) -> Self {
        match element {
            None => Self::Missing,
            Some(SyntaxElement::Token(token)) if index % 2 == 1 => Self::Separator(token),
            Some(element) => match (cast(element), element) {
                (Some(item), _) => Self::Item(item),
                (None, SyntaxElement::Node(node)) => Self::Bogus(node),
                (None, SyntaxElement::Token(_)) => Self::Missing,
            },
        }
    }
}
