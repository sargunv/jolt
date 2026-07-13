use std::{fmt, marker::PhantomData};

use jolt_text::{TextRange, TextSize};

use crate::{
    Comments, Language, RawSyntaxKind, SourceTokenId, SourceTriviaPiece, SourceTriviaSide,
    SyntaxTrivia,
    comment::{trivia_has_blank_line, trivia_iter_has_blank_line},
    conservation::source_trivia_pieces,
    syntax_tree::{SyntaxTree, TokenId},
};

/// A parent-aware borrowed cursor over a syntax tree token.
pub struct SyntaxToken<'tree, L: Language> {
    source: &'tree str,
    tree: &'tree SyntaxTree,
    id: TokenId,
    language: PhantomData<L>,
}

/// Returns true when the represented trivia between two adjacent tokens
/// contains an intentional blank line.
#[must_use]
pub fn tokens_have_blank_line_between<L: Language>(
    left: &SyntaxToken<'_, L>,
    right: &SyntaxToken<'_, L>,
) -> bool {
    trivia_iter_has_blank_line(left.trailing().iter().chain(right.leading()))
}

impl<'tree, L: Language> SyntaxToken<'tree, L> {
    pub(crate) const fn new(source: &'tree str, tree: &'tree SyntaxTree, id: TokenId) -> Self {
        Self {
            source,
            tree,
            id,
            language: PhantomData,
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
        self.tree.token(self.id).kind
    }

    /// Returns the source text backing this syntax tree.
    #[must_use]
    pub const fn source(&self) -> &'tree str {
        self.source
    }

    /// Returns this token's parse-local, tree-branded source identity.
    #[must_use]
    pub const fn source_id(&self) -> SourceTokenId<'tree> {
        SourceTokenId {
            tree: self.tree,
            id: self.id,
        }
    }

    /// Returns the byte offset where this token starts, including leading trivia.
    #[must_use]
    pub fn offset(&self) -> TextSize {
        self.tree.token(self.id).offset
    }

    /// Returns the token text without attached trivia.
    #[must_use]
    pub fn text(&self) -> &'tree str {
        let range = self.token_text_range();
        &self.source[range.start().get()..range.end().get()]
    }

    /// Returns the byte length covered by this token, including attached trivia.
    #[must_use]
    pub(crate) fn text_len(&self) -> TextSize {
        self.tree.token(self.id).text_len
    }

    /// Returns the full source range covered by this token, including attached trivia.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.offset(), self.offset() + self.text_len())
    }

    /// Returns the source range covered by the token text without attached trivia.
    #[must_use]
    pub fn token_text_range(&self) -> TextRange {
        self.tree.token(self.id).token_text_range
    }

    /// Returns trivia attached before this token.
    #[must_use]
    pub fn leading(&self) -> &'tree [SyntaxTrivia] {
        self.tree.trivia(&self.tree.token(self.id).leading)
    }

    /// Returns trivia attached after this token.
    #[must_use]
    pub fn trailing(&self) -> &'tree [SyntaxTrivia] {
        self.tree.trivia(&self.tree.token(self.id).trailing)
    }

    /// Returns leading trivia paired with exact parse-local identities.
    #[must_use]
    pub fn leading_trivia_with_ids(
        &self,
    ) -> impl ExactSizeIterator<Item = SourceTriviaPiece<'tree>> + use<'tree, L> {
        source_trivia_pieces(
            self.source_id(),
            SourceTriviaSide::Leading,
            self.tree.token(self.id).leading(),
        )
    }

    /// Returns trailing trivia paired with exact parse-local identities.
    #[must_use]
    pub fn trailing_trivia_with_ids(
        &self,
    ) -> impl ExactSizeIterator<Item = SourceTriviaPiece<'tree>> + use<'tree, L> {
        source_trivia_pieces(
            self.source_id(),
            SourceTriviaSide::Trailing,
            self.tree.token(self.id).trailing(),
        )
    }

    /// Returns comments attached before this token.
    #[must_use]
    pub fn leading_comments(&self) -> Comments<'tree> {
        Comments::new(self.source, self.leading(), self.offset())
    }

    /// Returns comments attached after this token.
    #[must_use]
    pub fn trailing_comments(&self) -> Comments<'tree> {
        Comments::new(self.source, self.trailing(), self.token_text_range().end())
    }

    /// Returns true when the token's leading trivia contains an intentional
    /// blank line (two or more consecutive newlines not separated by a comment).
    #[must_use]
    pub fn has_leading_blank_line(&self) -> bool {
        trivia_has_blank_line(self.leading())
    }
}

impl<L: Language> Clone for SyntaxToken<'_, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: Language> Copy for SyntaxToken<'_, L> {}

impl<L: Language> PartialEq for SyntaxToken<'_, L> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.id == other.id
    }
}

impl<L: Language> Eq for SyntaxToken<'_, L> {}

impl<L> fmt::Debug for SyntaxToken<'_, L>
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
            fmt_trivia(f, self.source, self.offset(), self.leading())?;
        }

        if !self.trailing().is_empty() {
            f.write_str(" trailing=")?;
            fmt_trivia(
                f,
                self.source,
                self.token_text_range().end(),
                self.trailing(),
            )?;
        }

        Ok(())
    }
}

fn fmt_trivia(
    f: &mut fmt::Formatter<'_>,
    source: &str,
    start: TextSize,
    trivia: &[SyntaxTrivia],
) -> fmt::Result {
    f.write_str("[")?;
    let mut offset = start;

    for (index, piece) in trivia.iter().enumerate() {
        if index > 0 {
            f.write_str(", ")?;
        }

        let range = TextRange::new(offset, offset + piece.text_len());
        offset = range.end();
        write!(
            f,
            "{:?} {:?}",
            piece.kind(),
            &source[range.start().get()..range.end().get()]
        )?;
    }

    f.write_str("]")
}
