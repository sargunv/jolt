//! Shared comment infrastructure for language syntax trees.
//!
//! Comments are derived from [`SyntaxTrivia`] attached to tokens. Both the Java
//! and Kotlin syntax crates consume these types via type aliases (e.g.
//! `JavaComment<'source> = jolt_syntax::Comment<'source>`), keeping the parse
//! and iteration logic in one place.

use std::slice;

use jolt_text::{TextRange, TextSize};

use crate::{SyntaxTrivia, TriviaKind};

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
    source: &'source str,
    text_range: TextRange,
}

impl<'source> Comment<'source> {
    /// Returns the comment kind.
    #[must_use]
    pub const fn kind(&self) -> CommentKind {
        self.kind
    }

    /// Returns the raw comment text.
    #[must_use]
    pub fn text(&self) -> &'source str {
        &self.source[self.text_range.start().get()..self.text_range.end().get()]
    }

    /// Returns the source range covered by this comment.
    #[must_use]
    pub fn text_range(&self) -> TextRange {
        self.text_range
    }

    pub(crate) const fn new(
        kind: CommentKind,
        source: &'source str,
        text_range: TextRange,
    ) -> Self {
        Self {
            kind,
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
    offset: TextSize,
}

impl<'source> Comments<'source> {
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
            let kind = match trivia.kind() {
                TriviaKind::LineComment | TriviaKind::ShebangComment => CommentKind::Line,
                TriviaKind::BlockComment => CommentKind::Block,
                TriviaKind::DocComment => CommentKind::Doc,
                TriviaKind::Whitespace | TriviaKind::Newline | TriviaKind::Ignored => continue,
            };
            return Some(Comment::new(kind, self.source, text_range));
        }

        None
    }
}

/// Returns true when the supplied trivia contains an intentional blank line
/// (two or more consecutive `Newline` pieces not reset by an intervening
/// comment).
///
/// Whitespace and ignored trivia do not interrupt the run.
#[must_use]
pub(crate) fn trivia_has_blank_line(trivia: &[SyntaxTrivia]) -> bool {
    let mut line_breaks_since_content = 0;
    for trivia in trivia {
        match trivia.kind() {
            TriviaKind::Newline => {
                line_breaks_since_content += 1;
                if line_breaks_since_content >= 2 {
                    return true;
                }
            }
            TriviaKind::Whitespace | TriviaKind::Ignored => {}
            TriviaKind::LineComment
            | TriviaKind::ShebangComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment => {
                line_breaks_since_content = 0;
            }
        }
    }

    false
}
