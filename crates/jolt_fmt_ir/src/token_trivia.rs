//! Shared token trivia placement for language formatters.
//!
//! Leading/trailing placement modes and the Doc assembly around a token body
//! are language-agnostic. Each language crate still owns how comments are
//! formatted and when trailing comments force a hard line.
//!
//! Comment documents are built lazily via closures so suppressed placement
//! modes do not claim trivia that an enclosing construct already owns.

use crate::{Doc, DocBuilder};

/// Whether leading comments on a token are emitted with the token or already
/// handled by an enclosing construct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeadingTrivia {
    Preserve,
    SuppressAlreadyHandled,
}

/// How trailing comments on a token are placed relative to layout breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrailingTrivia {
    Preserve,
    BeforeLineBreak,
    BeforeSoftLine,
    BeforeSpaceIfComments,
    RelocatedToEnclosingContext,
}

/// Assembles leading trivia + token body + trailing trivia.
///
/// Closures run only for the placement arms that need them, so suppressed
/// modes never claim comment source.
#[allow(clippy::too_many_arguments)]
pub fn format_token_doc<'source>(
    doc: &mut DocBuilder<'source>,
    token_doc: Doc<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
    mut leading_comments: impl FnMut(&mut DocBuilder<'source>) -> Doc<'source>,
    mut trailing_preserve: impl FnMut(&mut DocBuilder<'source>) -> Doc<'source>,
    mut trailing_before_break: impl FnMut(&mut DocBuilder<'source>) -> Doc<'source>,
    force_line: bool,
    has_trailing_comments: bool,
) -> Doc<'source> {
    let leading = match leading {
        LeadingTrivia::Preserve => leading_comments(doc),
        LeadingTrivia::SuppressAlreadyHandled => Doc::nil(),
    };
    let trailing = match trailing {
        TrailingTrivia::Preserve => trailing_preserve(doc),
        TrailingTrivia::BeforeLineBreak => trailing_before_break(doc),
        TrailingTrivia::BeforeSoftLine => {
            let comments = trailing_before_break(doc);
            let line = if force_line {
                doc.hard_line()
            } else {
                doc.soft_line()
            };
            doc.concat([comments, line])
        }
        TrailingTrivia::BeforeSpaceIfComments => {
            if has_trailing_comments {
                let comments = trailing_before_break(doc);
                let space = if force_line {
                    doc.hard_line()
                } else {
                    doc.space()
                };
                doc.concat([comments, space])
            } else {
                Doc::nil()
            }
        }
        TrailingTrivia::RelocatedToEnclosingContext => Doc::nil(),
    };
    doc.concat([leading, token_doc, trailing])
}
