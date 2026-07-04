use std::{fmt, marker::PhantomData};

use jolt_text::{TextRange, TextSize};

use crate::{
    Language, RawSyntaxKind,
    syntax_tree::{NodeId, SyntaxTree, TreeElement},
};

use super::{SyntaxElement, SyntaxToken};

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
            .node(self.id)
            .parent
            .map(|id| Self::new_child(self.source, self.tree, id))
    }

    /// Returns this node's index among its parent's children.
    #[must_use]
    pub fn index(&self) -> usize {
        self.tree.node(self.id).index
    }

    /// Returns the byte offset where this node starts.
    #[must_use]
    pub(crate) fn offset(&self) -> TextSize {
        self.tree.node(self.id).offset
    }

    /// Returns the byte length covered by this node.
    #[must_use]
    pub(crate) fn text_len(&self) -> TextSize {
        self.tree.node(self.id).text_len
    }

    /// Returns the full source range covered by this node.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns this node's child nodes and tokens.
    pub fn children_with_tokens(&self) -> impl Iterator<Item = SyntaxElement<'tree, L>> + '_ {
        self.tree
            .children(self.id)
            .iter()
            .copied()
            .map(|child| self.child_element(child))
    }

    /// Returns this node's child nodes.
    pub fn children(&self) -> impl Iterator<Item = Self> + '_ {
        self.tree
            .children(self.id)
            .iter()
            .copied()
            .filter_map(|child| match child {
                TreeElement::Node(id) => Some(Self::new_child(self.source, self.tree, id)),
                TreeElement::Token(_) => None,
            })
    }

    /// Returns this node's child tokens.
    pub fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken<'tree, L>> + '_ {
        self.tree
            .children(self.id)
            .iter()
            .copied()
            .filter_map(|child| match child {
                TreeElement::Node(_) => None,
                TreeElement::Token(id) => Some(SyntaxToken::new(self.source, self.tree, id)),
            })
    }

    /// Returns the first token contained by this node.
    #[must_use]
    pub fn first_token(&self) -> Option<SyntaxToken<'tree, L>> {
        for child in self.tree.children(self.id).iter().copied() {
            match child {
                TreeElement::Node(id) => {
                    let node = Self::new_child(self.source, self.tree, id);
                    if let Some(token) = node.first_token() {
                        return Some(token);
                    }
                }
                TreeElement::Token(id) => {
                    return Some(SyntaxToken::new(self.source, self.tree, id));
                }
            }
        }

        None
    }

    /// Returns the last token contained by this node.
    #[must_use]
    pub fn last_token(&self) -> Option<SyntaxToken<'tree, L>> {
        for child in self.tree.children(self.id).iter().copied().rev() {
            match child {
                TreeElement::Node(id) => {
                    let node = Self::new_child(self.source, self.tree, id);
                    if let Some(token) = node.last_token() {
                        return Some(token);
                    }
                }
                TreeElement::Token(id) => {
                    return Some(SyntaxToken::new(self.source, self.tree, id));
                }
            }
        }

        None
    }

    /// Returns this node's descendant nodes in preorder, excluding this node.
    pub fn descendants(&self) -> impl Iterator<Item = Self> + use<'tree, L> {
        Descendants::new(*self)
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
            .children(self.id)
            .get(index)
            .copied()
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

struct Descendants<'tree, L: Language> {
    source: &'tree str,
    tree: &'tree SyntaxTree,
    stack: Vec<NodeId>,
    language: PhantomData<L>,
}

impl<'tree, L: Language> Descendants<'tree, L> {
    fn new(root: SyntaxNode<'tree, L>) -> Self {
        let mut stack = Vec::new();
        stack.extend(
            root.tree
                .children(root.id)
                .iter()
                .rev()
                .filter_map(|child| {
                    if let TreeElement::Node(id) = child {
                        Some(*id)
                    } else {
                        None
                    }
                }),
        );

        Self {
            source: root.source,
            tree: root.tree,
            stack,
            language: PhantomData,
        }
    }
}

impl<'tree, L: Language> Iterator for Descendants<'tree, L> {
    type Item = SyntaxNode<'tree, L>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        self.stack
            .extend(self.tree.children(id).iter().rev().filter_map(|child| {
                if let TreeElement::Node(id) = child {
                    Some(*id)
                } else {
                    None
                }
            }));

        Some(SyntaxNode::new_child(self.source, self.tree, id))
    }
}

struct Tokens<'tree, L: Language> {
    source: &'tree str,
    tree: &'tree SyntaxTree,
    stack: Vec<TreeElement>,
    language: PhantomData<L>,
}

impl<'tree, L: Language> Tokens<'tree, L> {
    fn new(root: SyntaxNode<'tree, L>) -> Self {
        let mut stack = Vec::new();
        stack.extend(root.tree.children(root.id).iter().rev().copied());

        Self {
            source: root.source,
            tree: root.tree,
            stack,
            language: PhantomData,
        }
    }
}

impl<'tree, L: Language> Iterator for Tokens<'tree, L> {
    type Item = SyntaxToken<'tree, L>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(element) = self.stack.pop() {
            match element {
                TreeElement::Node(id) => {
                    self.stack
                        .extend(self.tree.children(id).iter().copied().rev());
                }
                TreeElement::Token(id) => {
                    return Some(SyntaxToken::new(self.source, self.tree, id));
                }
            }
        }

        None
    }
}
