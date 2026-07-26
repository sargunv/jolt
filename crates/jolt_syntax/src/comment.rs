//! Shared comment infrastructure for language syntax trees.
//!
//! Comments are derived from [`SyntaxTrivia`] attached to tokens. Both the Java
//! and Kotlin syntax crates consume these types via type aliases (e.g.
//! `JavaComment<'source> = jolt_syntax::Comment<'source>`), keeping the parse
//! and iteration logic in one place.

use std::slice;

use jolt_text::{TextRange, TextSize};

#[cfg(debug_assertions)]
use crate::{SourceTokenId, SourceTriviaId, SourceTriviaSide};
use crate::{SourceTriviaPiece, SyntaxTrivia, TriviaKind};

/// A comment kind exposed from syntax trivia.
///
/// Both Java and Kotlin share the same three comment kinds; the only naming
/// difference is documentation conventions (`Javadoc` vs `KDoc`), which does
/// not affect the layout-level distinction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommentKind {
    /// A `//` line comment (also covers shebang comments).
    Line,
    /// A non-documentation block comment.
    Block,
    /// A documentation block comment (`/** */` Javadoc or `/** */` `KDoc`).
    Doc,
}

/// A comment attached as token trivia in a syntax tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Comment<'source> {
    kind: CommentKind,
    terminated: bool,
    followed_by_blank_line: bool,
    source: &'source str,
    #[cfg(not(debug_assertions))]
    text_range: TextRange,
    #[cfg(debug_assertions)]
    piece: SourceTriviaPiece<'source>,
    #[cfg(debug_assertions)]
    line_terminator: Option<SourceTriviaPiece<'source>>,
}

impl<'source> Comment<'source> {
    /// Returns the comment kind.
    #[must_use]
    pub const fn kind(&self) -> CommentKind {
        self.kind
    }

    /// Returns whether the lexer recognized this comment as terminated.
    #[must_use]
    pub const fn is_terminated(&self) -> bool {
        self.terminated
    }

    /// Returns whether a blank line separates this comment from what follows it
    /// in the same trivia run.
    ///
    /// "What follows" is the next comment in the run, or the token the run is
    /// attached to. The gap after a comment belongs to that comment, so this is
    /// the only place a blank line inside a comment run is represented.
    #[must_use]
    pub const fn is_followed_by_blank_line(&self) -> bool {
        self.followed_by_blank_line
    }

    /// Returns the raw comment text.
    #[must_use]
    pub fn text(&self) -> &'source str {
        let range = self.text_range();
        &self.source[range.start().get()..range.end().get()]
    }

    /// Returns the source range covered by this comment.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        #[cfg(debug_assertions)]
        {
            self.piece.text_range()
        }
        #[cfg(not(debug_assertions))]
        {
            self.text_range
        }
    }

    #[cfg(debug_assertions)]
    #[must_use]
    pub const fn source_piece(&self) -> SourceTriviaPiece<'source> {
        self.piece
    }

    #[cfg(debug_assertions)]
    pub fn source_pieces(&self) -> impl Iterator<Item = SourceTriviaPiece<'source>> + use<'source> {
        [Some(self.piece), self.line_terminator]
            .into_iter()
            .flatten()
    }

    #[cfg(not(debug_assertions))]
    pub fn source_pieces(&self) -> impl Iterator<Item = SourceTriviaPiece<'source>> + use<'source> {
        std::iter::empty()
    }

    #[cfg(debug_assertions)]
    pub(crate) const fn new(
        kind: CommentKind,
        terminated: bool,
        followed_by_blank_line: bool,
        source: &'source str,
        piece: SourceTriviaPiece<'source>,
        line_terminator: Option<SourceTriviaPiece<'source>>,
    ) -> Self {
        Self {
            kind,
            terminated,
            followed_by_blank_line,
            source,
            piece,
            line_terminator,
        }
    }

    #[cfg(not(debug_assertions))]
    pub(crate) const fn new(
        kind: CommentKind,
        terminated: bool,
        followed_by_blank_line: bool,
        source: &'source str,
        text_range: TextRange,
    ) -> Self {
        Self {
            kind,
            terminated,
            followed_by_blank_line,
            source,
            text_range,
        }
    }
}

/// Borrowed comments attached to syntax token trivia.
#[derive(Clone)]
pub struct Comments<'source> {
    source: &'source str,
    trivia: slice::Iter<'source, SyntaxTrivia>,
    #[cfg(debug_assertions)]
    token: SourceTokenId<'source>,
    #[cfg(debug_assertions)]
    side: SourceTriviaSide,
    #[cfg(debug_assertions)]
    ordinal: usize,
    offset: TextSize,
    #[cfg(debug_assertions)]
    following_line_terminator: Option<SourceTriviaPiece<'source>>,
}

impl<'source> Comments<'source> {
    #[cfg(debug_assertions)]
    pub(crate) fn new(
        source: &'source str,
        trivia: &'source [SyntaxTrivia],
        token: SourceTokenId<'source>,
        side: SourceTriviaSide,
        offset: TextSize,
        following_line_terminator: Option<SourceTriviaPiece<'source>>,
    ) -> Self {
        Self {
            source,
            trivia: trivia.iter(),
            token,
            side,
            ordinal: 0,
            offset,
            following_line_terminator,
        }
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn new(
        source: &'source str,
        trivia: &'source [SyntaxTrivia],
        offset: TextSize,
    ) -> Self {
        Self {
            source,
            trivia: trivia.iter(),
            offset,
        }
    }

    /// Returns true when none of the trivia pieces carry a comment.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trivia.as_slice().iter().all(|trivia| {
            !matches!(
                trivia.kind(),
                TriviaKind::LineComment
                    | TriviaKind::ShebangComment
                    | TriviaKind::BlockComment
                    | TriviaKind::DocComment
                    | TriviaKind::UnterminatedBlockComment
                    | TriviaKind::UnterminatedDocComment
            )
        })
    }
}

impl<'source> Iterator for Comments<'source> {
    type Item = Comment<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        for trivia in self.trivia.by_ref() {
            let text_range = TextRange::new(self.offset, self.offset + trivia.text_len());
            self.offset = text_range.end();
            #[cfg(debug_assertions)]
            let id = SourceTriviaId::new(self.token, self.side, self.ordinal);
            #[cfg(debug_assertions)]
            {
                self.ordinal += 1;
            }
            let (kind, terminated) = match trivia.kind() {
                TriviaKind::LineComment | TriviaKind::ShebangComment => (CommentKind::Line, true),
                TriviaKind::BlockComment => (CommentKind::Block, true),
                TriviaKind::DocComment => (CommentKind::Doc, true),
                TriviaKind::UnterminatedBlockComment => (CommentKind::Block, false),
                TriviaKind::UnterminatedDocComment => (CommentKind::Doc, false),
                TriviaKind::Whitespace | TriviaKind::Newline | TriviaKind::Ignored => continue,
            };
            let followed_by_blank_line = trivia_opens_with_blank_line(self.trivia.as_slice());
            #[cfg(debug_assertions)]
            let line_terminator = if matches!(kind, CommentKind::Line) {
                (|| {
                    let trivia = *self.trivia.as_slice().first()?;
                    (trivia.kind() == TriviaKind::Newline).then(|| {
                        let range = TextRange::new(self.offset, self.offset + trivia.text_len());
                        SourceTriviaPiece::new(
                            SourceTriviaId::new(self.token, self.side, self.ordinal),
                            trivia,
                            range,
                        )
                    })
                })()
                .or_else(|| {
                    self.trivia
                        .as_slice()
                        .is_empty()
                        .then(|| self.following_line_terminator.take())
                        .flatten()
                })
            } else {
                None
            };
            #[cfg(debug_assertions)]
            return Some(Comment::new(
                kind,
                terminated,
                followed_by_blank_line,
                self.source,
                SourceTriviaPiece::new(id, *trivia, text_range),
                line_terminator,
            ));
            #[cfg(not(debug_assertions))]
            return Some(Comment::new(
                kind,
                terminated,
                followed_by_blank_line,
                self.source,
                text_range,
            ));
        }

        None
    }
}

/// Returns true when `trivia` opens with an intentional blank line: two or more
/// `Newline` pieces before anything else.
///
/// Whitespace and ignored trivia do not interrupt the run, but a comment ends
/// it. The gap that follows a comment belongs to that comment
/// ([`Comment::is_followed_by_blank_line`]), not to the run in front of it, so
/// scanning past a comment would report one blank line in two places.
#[must_use]
pub(crate) fn trivia_opens_with_blank_line(trivia: &[SyntaxTrivia]) -> bool {
    let mut line_breaks = 0;
    for trivia in trivia {
        match trivia.kind() {
            TriviaKind::Newline => {
                line_breaks += 1;
                if line_breaks >= 2 {
                    return true;
                }
            }
            TriviaKind::Whitespace | TriviaKind::Ignored => {}
            TriviaKind::LineComment
            | TriviaKind::ShebangComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::UnterminatedBlockComment
            | TriviaKind::UnterminatedDocComment => return false,
        }
    }

    false
}
