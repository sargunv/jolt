use std::{fmt, marker::PhantomData};

use jolt_text::{TextRange, TextSize};

use crate::{
    Language, RawSyntaxKind, SyntaxConservationTracker, SyntaxVerbatimCore,
    syntax_tree::{NodeId, SyntaxNodeId, SyntaxTree, TokenId, TreeElement, TreeSlot},
};

use super::{SyntaxElement, SyntaxSlot, SyntaxToken};

impl<'tree, L: Language> SyntaxSlot<'tree, L> {
    fn first_token(self) -> Option<SyntaxToken<'tree, L>> {
        match self {
            Self::Node(node) => node.first_token(),
            Self::Token(token) => Some(token),
            Self::Empty => None,
        }
    }

    fn last_token(self) -> Option<SyntaxToken<'tree, L>> {
        match self {
            Self::Node(node) => node.last_token(),
            Self::Token(token) => Some(token),
            Self::Empty => None,
        }
    }
}

/// A parent-aware borrowed cursor over a syntax tree node.
pub struct SyntaxNode<'tree, L: Language> {
    source: &'tree str,
    tree: &'tree SyntaxTree,
    id: NodeId,
    language: PhantomData<L>,
}

impl<'tree, L: Language> SyntaxNode<'tree, L> {
    /// Creates the red root for a syntax tree.
    #[must_use]
    pub fn new_root(source: &'tree str, tree: &'tree SyntaxTree) -> Self {
        Self {
            source,
            tree,
            id: tree.root(),
            language: PhantomData,
        }
    }

    pub(crate) const fn new_child(source: &'tree str, tree: &'tree SyntaxTree, id: NodeId) -> Self {
        Self {
            source,
            tree,
            id,
            language: PhantomData,
        }
    }

    /// Returns the source text backing this syntax tree.
    #[must_use]
    pub const fn source(&self) -> &'tree str {
        self.source
    }

    /// Returns this node's stable identity within its parse-owned tree.
    #[must_use]
    pub fn id(&self) -> SyntaxNodeId {
        SyntaxNodeId(self.id.index_u32())
    }

    /// Creates root-level dense source conservation accounting for this tree.
    ///
    /// Optimized builds return a zero-sized no-op tracker.
    #[must_use]
    pub fn conservation_tracker(&self) -> SyntaxConservationTracker<'tree> {
        SyntaxConservationTracker::new(self)
    }

    /// Returns this node's exact syntax-owned malformed-verbatim core when it
    /// is the language's current parser error node.
    #[must_use]
    pub fn malformed_verbatim_core(&self) -> Option<SyntaxVerbatimCore<'tree, L>> {
        self.is_directly_malformed()
            .then(|| SyntaxVerbatimCore::new(*self))
    }

    /// Returns the syntax-owned zero-width core for an empty grammar slot.
    #[must_use]
    pub fn missing_verbatim_core(&self, slot: usize) -> Option<SyntaxVerbatimCore<'tree, L>> {
        matches!(self.slot_at(slot), Some(SyntaxSlot::Empty)).then(|| {
            let previous = (0..slot)
                .rev()
                .find_map(|index| self.slot_at(index)?.last_token());
            let next =
                (slot + 1..self.slot_count()).find_map(|index| self.slot_at(index)?.first_token());
            let offset = next.map_or_else(
                || {
                    previous.map_or_else(
                        || self.text_range().start(),
                        |token| token.token_text_range().end(),
                    )
                },
                |token| token.token_text_range().start(),
            );
            SyntaxVerbatimCore::empty(
                *self,
                TextRange::empty(offset),
                previous.map(|token| token.source_id().id),
                next.map(|token| token.source_id().id),
            )
        })
    }

    /// Returns whether the parser/syntax factory assigned malformed ownership
    /// directly to this node.
    #[must_use]
    pub fn is_directly_malformed(&self) -> bool {
        self.tree.is_directly_malformed(self.id)
    }

    /// Returns whether this subtree contains no malformed node or missing slot.
    #[must_use]
    pub fn is_recovery_free(&self) -> bool {
        self.tree.is_recovery_free(self.id)
    }

    pub(crate) const fn tree(&self) -> &'tree SyntaxTree {
        self.tree
    }

    /// Returns the language-specific kind for this node.
    #[must_use]
    pub fn kind(&self) -> L::Kind {
        L::kind_from_raw(self.raw_kind())
    }

    /// Returns the raw kind for this node.
    #[must_use]
    pub(crate) fn raw_kind(&self) -> RawSyntaxKind {
        self.tree.node(self.id).kind
    }

    /// Returns this node's parent.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.tree
            .parent(self.id)
            .map(|id| Self::new_child(self.source, self.tree, id))
    }

    /// Returns this node's index among its parent's children.
    #[must_use]
    pub fn index(&self) -> usize {
        self.tree.index(self.id) as usize
    }

    /// Returns the byte offset where this node starts.
    #[must_use]
    pub(crate) fn offset(&self) -> TextSize {
        self.tree.node_offset(self.id)
    }

    /// Returns the byte length covered by this node.
    #[must_use]
    pub(crate) fn text_len(&self) -> TextSize {
        self.tree.node_text_len(self.id)
    }

    /// Returns the full source range covered by this node.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns this node's child nodes and tokens.
    pub fn children_with_tokens(
        &self,
    ) -> impl Iterator<Item = SyntaxElement<'tree, L>> + use<'tree, L> {
        let source = self.source;
        let tree = self.tree;
        self.tree.children(self.id).map(move |child| match child {
            TreeElement::Node(id) => SyntaxElement::Node(Self::new_child(source, tree, id)),
            TreeElement::Token(id) => SyntaxElement::Token(SyntaxToken::new(source, tree, id)),
        })
    }

    /// Returns the number of physical grammar slots stored for this node.
    #[doc(hidden)]
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.tree.slot_count(self.id)
    }

    /// Returns a physical grammar slot, preserving empty positions.
    #[doc(hidden)]
    #[must_use]
    pub fn slot_at(&self, index: usize) -> Option<SyntaxSlot<'tree, L>> {
        self.tree.slot_at(self.id, index).map(|slot| match slot {
            TreeSlot::Node(id) => SyntaxSlot::Node(Self::new_child(self.source, self.tree, id)),
            TreeSlot::Token(id) => SyntaxSlot::Token(SyntaxToken::new(self.source, self.tree, id)),
            TreeSlot::Empty => SyntaxSlot::Empty,
        })
    }

    /// Returns this node's child nodes.
    pub fn children(&self) -> impl Iterator<Item = Self> + use<'tree, L> {
        self.tree.children(self.id).filter_map(|child| match child {
            TreeElement::Node(id) => Some(Self::new_child(self.source, self.tree, id)),
            TreeElement::Token(_) => None,
        })
    }

    /// Returns this node's child tokens.
    pub fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken<'tree, L>> + use<'tree, L> {
        self.tree.children(self.id).filter_map(|child| match child {
            TreeElement::Node(_) => None,
            TreeElement::Token(id) => Some(SyntaxToken::new(self.source, self.tree, id)),
        })
    }

    /// Returns the first token contained by this node.
    #[must_use]
    pub fn first_token(&self) -> Option<SyntaxToken<'tree, L>> {
        let tokens = self.tree.token_range(self.id);
        (tokens.start < tokens.end)
            .then(|| SyntaxToken::new(self.source, self.tree, TokenId::new(tokens.start)))
    }

    /// Returns the last token contained by this node.
    #[must_use]
    pub fn last_token(&self) -> Option<SyntaxToken<'tree, L>> {
        let tokens = self.tree.token_range(self.id);
        (tokens.start < tokens.end)
            .then(|| SyntaxToken::new(self.source, self.tree, TokenId::new(tokens.end - 1)))
    }

    /// Returns every token contained by this node in source order.
    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken<'tree, L>> + use<'tree, L> {
        Tokens::new(*self)
    }

    /// Returns the next sibling node or token.
    #[must_use]
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<'tree, L>> {
        self.parent()?
            .child_element_at(self.index().saturating_add(1))
    }

    /// Returns the previous sibling node or token.
    #[must_use]
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<'tree, L>> {
        self.parent()?
            .child_element_at(self.index().checked_sub(1)?)
    }

    pub(super) fn child_element_at(&self, index: usize) -> Option<SyntaxElement<'tree, L>> {
        self.tree
            .child_at(self.id, index)
            .map(|child| self.child_element(child))
    }

    fn child_element(&self, child: TreeElement) -> SyntaxElement<'tree, L> {
        match child {
            TreeElement::Node(id) => {
                SyntaxElement::Node(Self::new_child(self.source, self.tree, id))
            }
            TreeElement::Token(id) => {
                SyntaxElement::Token(SyntaxToken::new(self.source, self.tree, id))
            }
        }
    }
}

impl<L: Language> Clone for SyntaxNode<'_, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: Language> Copy for SyntaxNode<'_, L> {}

impl<L: Language> PartialEq for SyntaxNode<'_, L> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.id == other.id
    }
}

impl<L: Language> Eq for SyntaxNode<'_, L> {}

impl<L> fmt::Debug for SyntaxNode<'_, L>
where
    L: Language,
    L::Kind: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_node(f, self, 0)
    }
}

fn fmt_node<L>(f: &mut fmt::Formatter<'_>, node: &SyntaxNode<'_, L>, indent: usize) -> fmt::Result
where
    L: Language,
    L::Kind: fmt::Debug,
{
    fmt_indent(f, indent)?;
    write!(f, "{:?}", node.kind())?;

    for child in node.children_with_tokens() {
        writeln!(f)?;
        match child {
            SyntaxElement::Node(node) => fmt_node(f, &node, indent + 1)?,
            SyntaxElement::Token(token) => {
                fmt_indent(f, indent + 1)?;
                write!(f, "{token:?}")?;
            }
        }
    }

    Ok(())
}

fn fmt_indent(f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        f.write_str("  ")?;
    }

    Ok(())
}

struct Tokens<'tree, L: Language> {
    source: &'tree str,
    tree: &'tree SyntaxTree,
    range: std::ops::Range<usize>,
    language: PhantomData<L>,
}

impl<'tree, L: Language> Tokens<'tree, L> {
    fn new(root: SyntaxNode<'tree, L>) -> Self {
        Self {
            source: root.source,
            tree: root.tree,
            range: root.tree.token_range(root.id),
            language: PhantomData,
        }
    }
}

impl<'tree, L: Language> Iterator for Tokens<'tree, L> {
    type Item = SyntaxToken<'tree, L>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = TokenId::new(self.range.next()?);
        Some(SyntaxToken::new(self.source, self.tree, id))
    }
}
