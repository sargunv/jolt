use std::{fmt, marker::PhantomData, rc::Rc};

use jolt_text::{TextRange, TextSize};

use crate::{GreenElement, GreenNode, Language, RawSyntaxKind};

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
    pub fn raw_kind(&self) -> RawSyntaxKind {
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
    pub fn offset(&self) -> TextSize {
        self.data.offset
    }

    /// Returns the byte length covered by this node.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        self.green().text_len()
    }

    /// Returns the full source range covered by this node.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns this node's child nodes and tokens.
    pub fn children_with_tokens(&self) -> impl Iterator<Item = SyntaxElement<L>> + '_ {
        let parent = self.clone();
        let mut offset = self.offset();

        self.green()
            .children()
            .iter()
            .enumerate()
            .map(move |(index, child)| {
                let child_offset = offset;
                offset += child.text_len();

                match child {
                    GreenElement::Node(green) => SyntaxElement::Node(Self::new_child(
                        green.clone(),
                        parent.clone(),
                        child_offset,
                        index,
                    )),
                    GreenElement::Token(green) => SyntaxElement::Token(SyntaxToken::new(
                        green.clone(),
                        parent.clone(),
                        child_offset,
                        index,
                    )),
                }
            })
    }

    /// Returns this node's child nodes.
    pub fn children(&self) -> impl Iterator<Item = Self> + '_ {
        self.children_with_tokens()
            .filter_map(SyntaxElement::into_node)
    }

    /// Returns the first token contained by this node.
    #[must_use]
    pub fn first_token(&self) -> Option<SyntaxToken<L>> {
        self.children_with_tokens()
            .find_map(|element| match element {
                SyntaxElement::Node(node) => node.first_token(),
                SyntaxElement::Token(token) => Some(token),
            })
    }

    /// Returns the last token contained by this node.
    #[must_use]
    pub fn last_token(&self) -> Option<SyntaxToken<L>> {
        let mut last = None;

        for element in self.children_with_tokens() {
            let token = match element {
                SyntaxElement::Node(node) => node.last_token(),
                SyntaxElement::Token(token) => Some(token),
            };

            if token.is_some() {
                last = token;
            }
        }

        last
    }

    /// Returns this node's descendant nodes in preorder, excluding this node.
    pub fn descendants(&self) -> impl Iterator<Item = Self> {
        Descendants::new(self)
    }

    /// Returns the next sibling node.
    #[must_use]
    pub fn next_sibling(&self) -> Option<Self> {
        self.parent()?
            .children_with_tokens()
            .skip(self.index().saturating_add(1))
            .find_map(SyntaxElement::into_node)
    }

    /// Returns the next sibling node or token.
    #[must_use]
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()?
            .children_with_tokens()
            .nth(self.index().saturating_add(1))
    }

    /// Returns the previous sibling node.
    #[must_use]
    pub fn prev_sibling(&self) -> Option<Self> {
        self.parent()?
            .children_with_tokens()
            .take(self.index())
            .filter_map(SyntaxElement::into_node)
            .last()
    }

    /// Returns the previous sibling node or token.
    #[must_use]
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()?
            .children_with_tokens()
            .nth(self.index().checked_sub(1)?)
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
