use std::{fmt, marker::PhantomData, rc::Rc};

use jolt_text::{TextRange, TextSize};

use crate::{
    Language, RawSyntaxKind,
    green::{GreenElement, GreenNode, GreenToken},
};

use super::{SyntaxElement, SyntaxToken};

/// A parent-aware cursor over a green node.
pub struct SyntaxNode<L: Language> {
    data: Rc<SyntaxNodeData<L>>,
}

struct SyntaxNodeData<L: Language> {
    green: GreenNode,
    parent: Option<SyntaxNode<L>>,
    offset: TextSize,
    index: usize,
    language: PhantomData<L>,
}

impl<L: Language> SyntaxNode<L> {
    /// Creates the red root for a green tree.
    #[must_use]
    pub fn new_root(green: GreenNode) -> Self {
        Self {
            data: Rc::new(SyntaxNodeData {
                green,
                parent: None,
                offset: TextSize::new(0),
                index: 0,
                language: PhantomData,
            }),
        }
    }

    pub(super) fn new_child(
        green: GreenNode,
        parent: Self,
        offset: TextSize,
        index: usize,
    ) -> Self {
        Self {
            data: Rc::new(SyntaxNodeData {
                green,
                parent: Some(parent),
                offset,
                index,
                language: PhantomData,
            }),
        }
    }

    /// Returns the raw green node backing this red node.
    #[must_use]
    pub fn green(&self) -> &GreenNode {
        &self.data.green
    }

    /// Returns the language-specific kind for this node.
    #[must_use]
    pub fn kind(&self) -> L::Kind {
        L::kind_from_raw(self.raw_kind())
    }

    /// Returns the raw kind for this node.
    #[must_use]
    pub(crate) fn raw_kind(&self) -> RawSyntaxKind {
        self.green().kind()
    }

    /// Returns this node's parent.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.data.parent.clone()
    }

    /// Returns this node's index among its parent's green children.
    #[must_use]
    pub fn index(&self) -> usize {
        self.data.index
    }

    /// Returns the byte offset where this node starts.
    #[must_use]
    pub(crate) fn offset(&self) -> TextSize {
        self.data.offset
    }

    /// Returns the byte length covered by this node.
    #[must_use]
    pub(crate) fn text_len(&self) -> TextSize {
        self.green().text_len()
    }

    /// Returns the full source range covered by this node.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns this node's child nodes and tokens.
    pub fn children_with_tokens(&self) -> impl Iterator<Item = SyntaxElement<L>> + '_ {
        self.child_slots()
            .map(|(index, offset, child)| self.child_element(index, offset, child))
    }

    /// Returns this node's child nodes.
    pub fn children(&self) -> impl Iterator<Item = Self> + '_ {
        self.child_slots()
            .filter_map(|(index, offset, child)| match child {
                GreenElement::Node(green) => Some(self.child_node(index, offset, green)),
                GreenElement::Token(_) => None,
            })
    }

    /// Returns this node's child tokens.
    pub fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken<L>> + '_ {
        self.child_slots()
            .filter_map(|(index, offset, child)| match child {
                GreenElement::Node(_) => None,
                GreenElement::Token(green) => Some(self.child_token(index, offset, green)),
            })
    }

    /// Returns the first token contained by this node.
    #[must_use]
    pub fn first_token(&self) -> Option<SyntaxToken<L>> {
        for (index, offset, child) in self.child_slots() {
            match child {
                GreenElement::Node(green) => {
                    let node = self.child_node(index, offset, green);
                    if let Some(token) = node.first_token() {
                        return Some(token);
                    }
                }
                GreenElement::Token(green) => {
                    return Some(self.child_token(index, offset, green));
                }
            }
        }

        None
    }

    /// Returns the last token contained by this node.
    #[must_use]
    pub fn last_token(&self) -> Option<SyntaxToken<L>> {
        for (index, offset, child) in self.child_slots_rev() {
            match child {
                GreenElement::Node(green) => {
                    let node = self.child_node(index, offset, green);
                    if let Some(token) = node.last_token() {
                        return Some(token);
                    }
                }
                GreenElement::Token(green) => {
                    return Some(self.child_token(index, offset, green));
                }
            }
        }

        None
    }

    /// Returns this node's descendant nodes in preorder, excluding this node.
    pub fn descendants(&self) -> impl Iterator<Item = Self> {
        Descendants::new(self)
    }

    /// Returns every token contained by this node in source order.
    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken<L>> {
        Tokens::new(self)
    }

    /// Returns the next sibling node or token.
    #[must_use]
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()?
            .child_element_at(self.index().saturating_add(1))
    }

    /// Returns the previous sibling node or token.
    #[must_use]
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()?
            .child_element_at(self.index().checked_sub(1)?)
    }

    pub(super) fn child_element_at(&self, index: usize) -> Option<SyntaxElement<L>> {
        self.child_slots()
            .find(|(child_index, _, _)| *child_index == index)
            .map(|(index, offset, child)| self.child_element(index, offset, child))
    }

    fn child_slots(&self) -> impl Iterator<Item = (usize, TextSize, &GreenElement)> + '_ {
        let mut offset = self.offset();

        self.green()
            .children()
            .iter()
            .enumerate()
            .map(move |(index, child)| {
                let child_offset = offset;
                offset += child.text_len();
                (index, child_offset, child)
            })
    }

    fn child_slots_rev(&self) -> impl Iterator<Item = (usize, TextSize, &GreenElement)> + '_ {
        let mut offset = self.offset() + self.text_len();

        self.green()
            .children()
            .iter()
            .enumerate()
            .rev()
            .map(move |(index, child)| {
                offset -= child.text_len();
                (index, offset, child)
            })
    }

    fn child_element(
        &self,
        index: usize,
        offset: TextSize,
        child: &GreenElement,
    ) -> SyntaxElement<L> {
        match child {
            GreenElement::Node(green) => SyntaxElement::Node(self.child_node(index, offset, green)),
            GreenElement::Token(green) => {
                SyntaxElement::Token(self.child_token(index, offset, green))
            }
        }
    }

    fn child_node(&self, index: usize, offset: TextSize, green: &GreenNode) -> Self {
        Self::new_child(green.clone(), self.clone(), offset, index)
    }

    fn child_token(&self, index: usize, offset: TextSize, _green: &GreenToken) -> SyntaxToken<L> {
        SyntaxToken::new(self.clone(), offset, index)
    }
}

impl<L: Language> Clone for SyntaxNode<L> {
    fn clone(&self) -> Self {
        Self {
            data: Rc::clone(&self.data),
        }
    }
}

impl<L: Language> PartialEq for SyntaxNode<L> {
    fn eq(&self, other: &Self) -> bool {
        self.offset() == other.offset() && self.green().ptr_eq(other.green())
    }
}

impl<L: Language> Eq for SyntaxNode<L> {}

impl<L> fmt::Debug for SyntaxNode<L>
where
    L: Language,
    L::Kind: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_node(f, self, 0)
    }
}

fn fmt_node<L>(f: &mut fmt::Formatter<'_>, node: &SyntaxNode<L>, indent: usize) -> fmt::Result
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

struct Descendants<L: Language> {
    stack: Vec<SyntaxNode<L>>,
}

impl<L: Language> Descendants<L> {
    fn new(root: &SyntaxNode<L>) -> Self {
        let mut stack = root.children().collect::<Vec<_>>();
        stack.reverse();

        Self { stack }
    }
}

impl<L: Language> Iterator for Descendants<L> {
    type Item = SyntaxNode<L>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let mut children = node.children().collect::<Vec<_>>();
        children.reverse();
        self.stack.extend(children);

        Some(node)
    }
}

struct Tokens<L: Language> {
    stack: Vec<TokenFrame<L>>,
}

struct TokenFrame<L: Language> {
    node: SyntaxNode<L>,
    next_index: usize,
    next_offset: TextSize,
}

impl<L: Language> Tokens<L> {
    fn new(root: &SyntaxNode<L>) -> Self {
        Self {
            stack: vec![TokenFrame {
                node: root.clone(),
                next_index: 0,
                next_offset: root.offset(),
            }],
        }
    }
}

impl<L: Language> Iterator for Tokens<L> {
    type Item = SyntaxToken<L>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let frame = self.stack.last_mut()?;
            if frame.next_index == frame.node.green().children().len() {
                self.stack.pop();
                continue;
            }

            let index = frame.next_index;
            let offset = frame.next_offset;
            let text_len = frame.node.green().children()[index].text_len();
            frame.next_index += 1;
            frame.next_offset += text_len;

            let parent = frame.node.clone();
            match &parent.green().children()[index] {
                GreenElement::Node(green) => {
                    let node = SyntaxNode::new_child(green.clone(), parent, offset, index);
                    self.stack.push(TokenFrame {
                        next_offset: node.offset(),
                        node,
                        next_index: 0,
                    });
                }
                GreenElement::Token(_) => return Some(SyntaxToken::new(parent, offset, index)),
            }
        }
    }
}
