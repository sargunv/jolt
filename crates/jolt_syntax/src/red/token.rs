use jolt_text::{TextRange, TextSize};

use crate::{GreenToken, GreenTrivia, Language, RawSyntaxKind};

use super::{SyntaxElement, SyntaxNode};

/// A parent-aware cursor over a green token.
pub struct SyntaxToken<L: Language> {
    green: GreenToken,
    parent: SyntaxNode<L>,
    offset: TextSize,
    index: usize,
}

impl<L: Language> SyntaxToken<L> {
    pub(super) const fn new(
        green: GreenToken,
        parent: SyntaxNode<L>,
        offset: TextSize,
        index: usize,
    ) -> Self {
        Self {
            green,
            parent,
            offset,
            index,
        }
    }

    /// Returns the raw green token backing this red token.
    #[must_use]
    pub const fn green(&self) -> &GreenToken {
        &self.green
    }

    /// Returns the language-specific kind for this token.
    #[must_use]
    pub fn kind(&self) -> L::Kind {
        L::kind_from_raw(self.raw_kind())
    }

    /// Returns the raw kind for this token.
    #[must_use]
    pub fn raw_kind(&self) -> RawSyntaxKind {
        self.green().kind()
    }

    /// Returns this token's parent node.
    #[must_use]
    pub fn parent(&self) -> SyntaxNode<L> {
        self.parent.clone()
    }

    /// Returns this token's index among its parent's green children.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the byte offset where this token starts, including leading trivia.
    #[must_use]
    pub const fn offset(&self) -> TextSize {
        self.offset
    }

    /// Returns the token text without attached trivia.
    #[must_use]
    pub fn text(&self) -> &str {
        self.green().text()
    }

    /// Returns the byte length covered by this token, including attached trivia.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        self.green().text_len()
    }

    /// Returns the full source range covered by this token, including attached trivia.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns the source range covered by the token text without attached trivia.
    #[must_use]
    pub fn token_text_range(&self) -> TextRange {
        let start = self.offset() + trivia_text_len(self.leading());

        TextRange::new(start, start + self.green().token_text_len())
    }

    /// Returns trivia attached before this token.
    #[must_use]
    pub fn leading(&self) -> &[GreenTrivia] {
        self.green().leading()
    }

    /// Returns trivia attached after this token.
    #[must_use]
    pub fn trailing(&self) -> &[GreenTrivia] {
        self.green().trailing()
    }

    /// Returns the next sibling node.
    #[must_use]
    pub fn next_sibling(&self) -> Option<SyntaxNode<L>> {
        self.parent()
            .children_with_tokens()
            .skip(self.index().saturating_add(1))
            .find_map(SyntaxElement::into_node)
    }

    /// Returns the next sibling node or token.
    #[must_use]
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()
            .children_with_tokens()
            .nth(self.index().saturating_add(1))
    }

    /// Returns the previous sibling node.
    #[must_use]
    pub fn prev_sibling(&self) -> Option<SyntaxNode<L>> {
        self.parent()
            .children_with_tokens()
            .take(self.index())
            .filter_map(SyntaxElement::into_node)
            .last()
    }

    /// Returns the previous sibling node or token.
    #[must_use]
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<L>> {
        self.parent()
            .children_with_tokens()
            .nth(self.index().checked_sub(1)?)
    }
}

impl<L: Language> Clone for SyntaxToken<L> {
    fn clone(&self) -> Self {
        Self {
            green: self.green.clone(),
            parent: self.parent.clone(),
            offset: self.offset,
            index: self.index,
        }
    }
}

impl<L: Language> PartialEq for SyntaxToken<L> {
    fn eq(&self, other: &Self) -> bool {
        self.offset() == other.offset() && self.green().ptr_eq(other.green())
    }
}

impl<L: Language> Eq for SyntaxToken<L> {}

fn trivia_text_len(trivia: &[GreenTrivia]) -> TextSize {
    TextSize::new(trivia.iter().map(|piece| piece.text_len().get()).sum())
}
