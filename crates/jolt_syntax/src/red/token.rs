use std::fmt;

use jolt_text::{TextRange, TextSize};

use crate::{
    GreenTrivia, Language, RawSyntaxKind,
    green::{GreenElement, GreenToken},
};

use super::SyntaxNode;

/// A parent-aware cursor over a green token.
pub struct SyntaxToken<L: Language> {
    parent: SyntaxNode<L>,
    offset: TextSize,
    index: usize,
}

impl<L: Language> SyntaxToken<L> {
    pub(super) const fn new(parent: SyntaxNode<L>, offset: TextSize, index: usize) -> Self {
        Self {
            parent,
            offset,
            index,
        }
    }

    /// Returns the raw green token backing this red token.
    #[must_use]
    pub(crate) fn green(&self) -> &GreenToken {
        match &self.parent.green().children()[self.index] {
            GreenElement::Token(token) => token,
            GreenElement::Node(_) => unreachable!("syntax token index must point to a token"),
        }
    }

    /// Returns the language-specific kind for this token.
    #[must_use]
    pub fn kind(&self) -> L::Kind {
        L::kind_from_raw(self.raw_kind())
    }

    /// Returns the raw kind for this token.
    #[must_use]
    pub(crate) fn raw_kind(&self) -> RawSyntaxKind {
        self.green().kind()
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
    pub(crate) fn text_len(&self) -> TextSize {
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
}

impl<L: Language> Clone for SyntaxToken<L> {
    fn clone(&self) -> Self {
        Self {
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

impl<L> fmt::Debug for SyntaxToken<L>
where
    L: Language,
    L::Kind: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token_range = self.token_text_range();

        write!(
            f,
            "{:?} {:?} @ {}..{}",
            self.kind(),
            self.text(),
            token_range.start().get(),
            token_range.end().get()
        )?;

        if !self.leading().is_empty() {
            f.write_str(" leading=")?;
            fmt_trivia(f, self.leading())?;
        }

        if !self.trailing().is_empty() {
            f.write_str(" trailing=")?;
            fmt_trivia(f, self.trailing())?;
        }

        Ok(())
    }
}

fn fmt_trivia(f: &mut fmt::Formatter<'_>, trivia: &[GreenTrivia]) -> fmt::Result {
    f.write_str("[")?;

    for (index, piece) in trivia.iter().enumerate() {
        if index > 0 {
            f.write_str(", ")?;
        }

        write!(f, "{:?} {:?}", piece.kind(), piece.text())?;
    }

    f.write_str("]")
}

fn trivia_text_len(trivia: &[GreenTrivia]) -> TextSize {
    TextSize::new(trivia.iter().map(|piece| piece.text_len().get()).sum())
}
