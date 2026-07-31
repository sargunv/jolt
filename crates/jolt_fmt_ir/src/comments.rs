//! Shared comment rendering and placement over borrowed syntax tokens.
//!
//! [`Comment`], [`CommentKind`], and [`SyntaxToken`] are shared syntax types, so
//! comment text rendering, ordering, separator spacing, and forced-line
//! decisions are all language-neutral. [`CommentKind::Doc`] already covers both
//! Javadoc and `KDoc`.
//!
//! Leading-comment placement is line-start-aware. A token an enclosing join
//! registered with [`DocBuilder::with_line_start_leading`] — a body item, a
//! member, the first element behind an open delimiter — keeps its leading
//! comments on lines of their own, which is exactly how the reparse reads them
//! back. Any other token keeps them inline: block comments beside the token,
//! line comments on a line of their own before it, since a line comment can
//! never legally sit before its token on one line. Both forms are fixpoints of
//! format ∘ reparse, which is what makes formatting idempotent.
//!
//! Language crates still own comment placement that is specific to their
//! grammar, such as Java's modifier relocation or Kotlin's semicolon
//! terminators.

use jolt_syntax::{Comment, CommentKind, Language, SyntaxToken, TriviaKind};

use crate::comment_text::{
    StarBlockOpener, format_comment_lines, format_star_block_comment,
    is_empty_single_line_block_comment, is_star_block_comment, preserved_block_comment_lines,
    preserved_comment_lines,
};
use crate::token_trivia::format_token_doc;
use crate::{ConcatBuilder, Doc, DocBuilder, LeadingTrivia, TrailingTrivia};

/// Where inline leading comments sit relative to their token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineLeadingTrivia {
    AfterPreviousToken,
    BeforeToken,
    /// Padded with a space on each side, matching the way a member-access dot
    /// boundary renders the dot's trailing comments.
    BetweenSpaces,
}

/// Emits byte-order-mark trivia exactly once.
///
/// A byte order mark carries no layout meaning but is part of the source.
pub fn format_byte_order_mark<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    format_source_marker(doc, token, TriviaKind::ByteOrderMark)
}

/// Emits Java's lexically ignored final SUB marker exactly once.
pub fn format_trailing_substitute<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    format_source_marker(doc, token, TriviaKind::TrailingSubstitute)
}

fn format_source_marker<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    kind: TriviaKind,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for piece in token
            .ignored_trivia()
            .filter(|piece| piece.trivia().kind() == kind)
        {
            let range = piece.text_range();
            let text = &token.source()[range.start().get()..range.end().get()];
            let exact = docs.source_trivia([piece], |docs| docs.literal_text(text));
            docs.push(exact);
        }
    })
}

/// Renders one comment's own text, claiming its source pieces.
pub fn format_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &Comment<'source>,
) -> Doc<'source> {
    doc.source_trivia(comment.source_pieces(), |doc| {
        if !comment.is_terminated() {
            return doc.literal_text(comment.text());
        }

        if is_empty_single_line_block_comment(comment.text()) {
            return format_block_comment(doc, comment.text());
        }

        match comment.kind() {
            CommentKind::Line => format_line_comment(doc, comment.text()),
            CommentKind::Block if is_star_block_comment(comment.text()) => {
                format_star_block_comment(doc, comment.text(), "/*", StarBlockOpener::Keep)
            }
            CommentKind::Block => format_block_comment(doc, comment.text()),
            CommentKind::Doc => {
                format_star_block_comment(doc, comment.text(), "/**", StarBlockOpener::Reflow)
            }
        }
    })
}

/// Renders one comment that follows code on the same line.
///
/// A single-line star block keeps its own line rather than being reflowed onto
/// three, which would push the trailing code's comment away from it.
pub fn format_trailing_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &Comment<'source>,
) -> Doc<'source> {
    if comment_is_star_block(comment) && !comment.text().contains(['\n', '\r']) {
        doc.source_trivia(comment.source_pieces(), |doc| {
            format_block_comment(doc, comment.text())
        })
    } else {
        format_comment(doc, comment)
    }
}

/// Reports whether a comment uses Javadoc/KDoc star-block layout.
#[must_use]
pub fn comment_is_star_block(comment: &Comment<'_>) -> bool {
    comment.kind() == CommentKind::Doc || is_star_block_comment(comment.text())
}

/// Reports whether a comment ends its line, forcing a hard break after it.
#[must_use]
pub fn comment_forces_line(comment: &Comment<'_>) -> bool {
    comment.kind() == CommentKind::Line || comment.text().contains(['\n', '\r'])
}

/// Reports whether any trailing comment on a token forces a hard line.
#[must_use]
pub fn trailing_comments_force_line<L: Language>(token: &SyntaxToken<'_, L>) -> bool {
    token
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment))
}

/// Reports whether a token carries any comment trivia.
#[must_use]
pub fn token_has_comments<L: Language>(token: &SyntaxToken<'_, L>) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

/// Formats leading comments, each on its own line.
///
/// A blank line the source put after a comment is kept, whether the next line
/// holds another comment of the run or the item the run leads. Any longer run
/// of blank lines collapses to one, and a blank line is never invented where
/// the source had none.
///
/// The run is marked as a comment prefix so that a group it leads still gets to
/// measure its code, rather than being broken by the hard line every comment
/// ends with.
pub fn format_leading_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
) -> Doc<'source> {
    let run = doc.concat_list(|docs| {
        for comment in comments {
            let blank_line_after = comment.is_followed_by_blank_line();
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            let line = if blank_line_after {
                docs.empty_line()
            } else {
                docs.hard_line()
            };
            docs.push(line);
        }
    });
    doc.comment_prefix(run)
}

/// Formats a token's leading comments, each on its own line.
pub fn format_leading_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    format_leading_comment_list(doc, token.leading_comments())
}

/// Formats a token's trailing comments, breaking after any that ends its line.
fn format_trailing_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    let mut comments = token.trailing_comments().peekable();
    doc.concat_list(|docs| {
        while let Some(comment) = comments.next() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_trailing_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = if comments.peek().is_none() {
                    docs.hard_line_suffix()
                } else {
                    docs.hard_line()
                };
                docs.push(hard_line);
            }
        }
    })
}

/// Formats a token's trailing comments where an enclosing layout break follows.
///
/// The final comment does not emit its own hard line, because the enclosing
/// construct already breaks after it.
pub fn format_trailing_comments_before_line_break<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    format_trailing_comment_list_before_line_break(doc, token.trailing_comments())
}

/// Formats a selected trailing-comment run where an enclosing layout break
/// follows.
///
/// This is the list form of [`format_trailing_comments_before_line_break`]. It
/// lets an enclosing construct format only the comments it owns when the
/// parser exposes the same source comment at both sides of a syntax boundary.
pub fn format_trailing_comment_list_before_line_break<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
) -> Doc<'source> {
    let mut comments = comments.into_iter().peekable();
    doc.concat_list(|docs| {
        while let Some(comment) = comments.next() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_trailing_comment(docs, &comment);
            docs.push(comment_doc);
            if comments.peek().is_some() && comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

/// Formats trailing comments inline, without any forced line after them.
pub fn format_inline_trailing_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment = format_trailing_comment(docs, &comment);
            docs.push(comment);
        }
    })
}

/// Formats comments that belong to a construct rather than to a token.
///
/// A blank line the source put between two of these comments is kept, longer
/// runs collapse to one, and a blank line is never invented.
pub fn format_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| push_dangling_comments(docs, comments, false))
}

/// Formats the comments a delimited construct holds between its delimiters.
///
/// The comments come from two trivia runs: the open delimiter's trailing run
/// and the close delimiter's leading run. A gap between the two runs opens the
/// close delimiter's leading trivia, so it is owned by that token rather than
/// by the comment in front of it, and it is the one gap
/// [`Comment::is_followed_by_blank_line`] cannot report.
pub fn format_delimiter_dangling_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    open: Option<&SyntaxToken<'source, L>>,
    close: Option<&SyntaxToken<'source, L>>,
) -> Doc<'source> {
    let blank_line_before_close = close.is_some_and(SyntaxToken::has_leading_blank_line);
    let open = open.copied();
    let close = close.copied();
    doc.concat_list(|docs| {
        push_dangling_comments(
            docs,
            open.iter().flat_map(SyntaxToken::trailing_comments),
            false,
        );
        push_dangling_comments(
            docs,
            close.iter().flat_map(SyntaxToken::leading_comments),
            blank_line_before_close,
        );
    })
}

/// Appends one dangling comment run, each comment on its own line.
///
/// `blank_line_before` is the gap in front of the run; it only reaches the
/// output when something already precedes the run on an earlier line, so a
/// blank line that merely opens a construct is dropped rather than indented
/// into it.
fn push_dangling_comments<'source>(
    docs: &mut ConcatBuilder<'_, 'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
    mut blank_line_before: bool,
) {
    for comment in comments {
        if !docs.is_empty() {
            let line = if blank_line_before {
                docs.empty_line()
            } else {
                docs.hard_line()
            };
            docs.push(line);
        }
        blank_line_before = comment.is_followed_by_blank_line();
        let comment = format_comment(docs, &comment);
        docs.push(comment);
    }
}

/// Formats comments salvaged from a removed token, or `None` when there are
/// none.
///
/// A blank line the source put between two of these comments is kept, longer
/// runs collapse to one, and a blank line is never invented.
pub fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = Comment<'source>>,
) -> Option<Doc<'source>> {
    let mut has_comments = false;
    let mut blank_line_before = false;
    let docs = doc.concat_list(|docs| {
        for comment in comments {
            if has_comments {
                let line = if blank_line_before {
                    docs.empty_line()
                } else {
                    docs.hard_line()
                };
                docs.push(line);
            }
            blank_line_before = comment.is_followed_by_blank_line();
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            has_comments = true;
        }
    });

    has_comments.then_some(docs)
}

/// Reports whether a delimited construct holds comments between its delimiters.
#[must_use]
pub fn has_delimiter_dangling_comments<L: Language>(
    open: Option<&SyntaxToken<'_, L>>,
    close: Option<&SyntaxToken<'_, L>>,
) -> bool {
    open.is_some_and(|token| !token.trailing_comments().is_empty())
        || close.is_some_and(|token| !token.leading_comments().is_empty())
}

/// Formats a list separator, breaking after it when its comments force a line.
///
/// A separator is glued to the item before it, so its leading comments take
/// the previous token's trailing form -- the placement the reparse reads back
/// identically.
pub fn format_separator_with_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    unforced_break: Doc<'source>,
) -> Doc<'source> {
    let token_doc = format_token_with_inline_leading_comments(
        doc,
        token,
        InlineLeadingTrivia::AfterPreviousToken,
        TrailingTrivia::BeforeLineBreak,
    );
    let line = if trailing_comments_force_line(token) {
        doc.hard_line()
    } else {
        unforced_break
    };
    doc.concat([token_doc, line])
}

/// Formats a token with its comment trivia placed per the given modes.
pub fn format_token<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let token_doc = doc.source_token(token);
    format_token_body(doc, token, token_doc, leading, trailing)
}

/// Formats comment trivia around an already-built token body.
///
/// Used where a rule replaces the token's own text, such as a normalized
/// separator.
///
/// Preserved leading comments are placed by line-start knowledge: a token the
/// enclosing join registered with [`DocBuilder::with_line_start_leading`]
/// keeps them on lines of their own, while any other token keeps them inline
/// beside it, the only placement the reparse reads back identically for a
/// token that does not statically begin its line.
pub fn format_token_body<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    token_doc: Doc<'source>,
    leading: LeadingTrivia,
    mut trailing: TrailingTrivia,
) -> Doc<'source> {
    if doc.relocates_trailing_trivia(token) {
        trailing = TrailingTrivia::RelocatedToEnclosingContext;
    }
    let line_start = doc.has_line_start_leading(token);
    format_token_doc(
        doc,
        token_doc,
        leading,
        trailing,
        |doc| {
            if line_start {
                format_leading_comments(doc, token)
            } else {
                format_inline_leading_comments(doc, token)
            }
        },
        |doc| format_trailing_comments(doc, token),
        |doc| format_trailing_comments_before_line_break(doc, token),
        trailing_comments_force_line(token),
        !token.trailing_comments().is_empty(),
    )
}

/// Formats a token whose leading comments an enclosing construct already
/// emitted.
pub fn format_token_after_relocated_leading_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token(doc, token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

/// Formats a token whose leading comments stay inline beside it.
pub fn format_token_with_inline_leading_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    placement: InlineLeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = match placement {
        InlineLeadingTrivia::AfterPreviousToken => {
            format_inline_leading_comments_after_previous(doc, token)
        }
        InlineLeadingTrivia::BeforeToken => format_inline_leading_comments(doc, token),
        InlineLeadingTrivia::BetweenSpaces => {
            format_inline_leading_comments_between_spaces(doc, token)
        }
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc.concat([leading, token])
}

/// Formats a token's leading comments inline before the token.
///
/// Block comments sit beside the token; the reparse reads them as the previous
/// token's same-line trailing trivia and emits them in the same place again. A
/// comment that ends its line takes a line of its own instead, because it can
/// never sit before the token on one line without swallowing the code that
/// follows it.
fn format_inline_leading_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    let Some((comments, forces_line)) = inline_leading_comment_run(doc, token) else {
        return Doc::nil();
    };
    let after_comments = if forces_line {
        doc.hard_line()
    } else {
        doc.space()
    };
    let comments = doc.concat([comments, after_comments]);
    if forces_line {
        doc.comment_prefix(comments)
    } else {
        comments
    }
}

/// Formats a token's leading comments for hoisting before the group the token
/// opens.
///
/// `AfterPreviousToken` prefixes a space, matching the trailing run of a
/// previous token the delimiter is glued to; `BeforeToken` relies on the
/// separator the construct already emitted. The run ends the line when its
/// final comment forces one. A line comment hoisted this way stays out of the
/// group's fit, which is what the reparse's trailing-trivia assignment of the
/// same comments also does.
pub fn format_leading_comments_before_group<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    placement: InlineLeadingTrivia,
) -> Doc<'source> {
    let Some((comments, forces_line)) = inline_leading_comment_run(doc, token) else {
        return Doc::nil();
    };
    let before = match placement {
        InlineLeadingTrivia::AfterPreviousToken | InlineLeadingTrivia::BetweenSpaces => doc.space(),
        InlineLeadingTrivia::BeforeToken => Doc::nil(),
    };
    let after = if forces_line {
        doc.hard_line()
    } else {
        Doc::nil()
    };
    doc.concat([before, comments, after])
}

/// Formats a token's leading comments inline after the previous token.
fn format_inline_leading_comments_after_previous<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    let Some((comments, forces_line)) = inline_leading_comment_run(doc, token) else {
        return Doc::nil();
    };
    let before_comments = doc.space();
    let after_comments = if forces_line {
        doc.hard_line()
    } else {
        Doc::nil()
    };
    let comments = doc.concat([before_comments, comments, after_comments]);
    if forces_line {
        doc.comment_prefix(comments)
    } else {
        comments
    }
}

/// Formats a token's leading comments with a space on each side, or a hard
/// line in place of the trailing space when the final comment ends its line.
fn format_inline_leading_comments_between_spaces<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    let Some((comments, forces_line)) = inline_leading_comment_run(doc, token) else {
        return Doc::nil();
    };
    let before_comments = doc.space();
    let after_comments = if forces_line {
        doc.hard_line()
    } else {
        doc.space()
    };
    doc.concat([before_comments, comments, after_comments])
}

fn inline_leading_comment_run<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Option<(Doc<'source>, bool)> {
    let leading = token.leading_comments();
    if leading.is_empty() {
        return None;
    }
    let mut final_comment_forces_line = false;
    let comments = doc.concat_list(|comments| {
        for comment in leading {
            if !comments.is_empty() {
                let separator = if final_comment_forces_line {
                    comments.hard_line()
                } else {
                    comments.space()
                };
                comments.push(separator);
            }
            final_comment_forces_line = comment_forces_line(&comment);
            // The comments sit beside code, so a single-line star block keeps
            // its line: reflowing it onto three would make the reparse read a
            // line-forcing comment where the source had none.
            let comment = format_trailing_comment(comments, &comment);
            comments.push(comment);
        }
    });
    Some((comments, final_comment_forces_line))
}

fn format_line_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    format_comment_lines(doc, preserved_comment_lines(comment))
}

fn format_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    format_comment_lines(doc, preserved_block_comment_lines(comment))
}
