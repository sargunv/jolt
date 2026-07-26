//! Shared comment rendering and placement over borrowed syntax tokens.
//!
//! [`Comment`], [`CommentKind`], and [`SyntaxToken`] are shared syntax types, so
//! comment text rendering, ordering, separator spacing, and forced-line
//! decisions are all language-neutral. [`CommentKind::Doc`] already covers both
//! Javadoc and `KDoc`.
//!
//! Language crates still own comment placement that is specific to their
//! grammar, such as Java's modifier relocation or Kotlin's semicolon
//! terminators.

use jolt_syntax::{Comment, CommentKind, Language, SyntaxToken};

use crate::comment_text::{
    format_comment_lines, format_star_block_comment, is_empty_single_line_block_comment,
    is_star_block_comment, preserved_block_comment_lines, preserved_comment_lines,
};
use crate::token_trivia::format_token_doc;
use crate::{Doc, DocBuilder, LeadingTrivia, TrailingTrivia};

/// Where inline leading comments sit relative to their token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineLeadingTrivia {
    AfterPreviousToken,
    BeforeToken,
}

/// Emits lexically ignored but source-significant trivia exactly once.
///
/// Ignored trivia carries no layout meaning but is part of the source, so a
/// formatter must emit and claim it verbatim rather than dropping it as
/// whitespace. Java's permitted final SUB and Kotlin's leading byte order mark
/// are both of this shape.
pub fn format_ignored_trivia<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for piece in token.ignored_trivia() {
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
                format_star_block_comment(doc, comment.text(), "/*")
            }
            CommentKind::Block => format_block_comment(doc, comment.text()),
            CommentKind::Doc => format_star_block_comment(doc, comment.text(), "/**"),
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
    doc.concat_list(|docs| {
        for comment in token.trailing_comments() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_trailing_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
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
    let mut comments = token.trailing_comments().peekable();
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
    let mut blank_line_before = false;
    doc.concat_list(|docs| {
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
    })
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

/// Returns the comments a delimited construct holds between its delimiters.
pub fn delimiter_dangling_comments<'source, L: Language>(
    open: Option<&SyntaxToken<'source, L>>,
    close: Option<&SyntaxToken<'source, L>>,
) -> impl Iterator<Item = Comment<'source>> {
    open.into_iter()
        .flat_map(SyntaxToken::trailing_comments)
        .chain(close.into_iter().flat_map(SyntaxToken::leading_comments))
}

/// Formats a list separator, breaking after it when its comments force a line.
pub fn format_separator_with_comments<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    unforced_break: Doc<'source>,
) -> Doc<'source> {
    let token_doc = format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
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
pub fn format_token_body<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    token: &SyntaxToken<'source, L>,
    token_doc: Doc<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token_doc(
        doc,
        token_doc,
        leading,
        trailing,
        |doc| format_leading_comments(doc, token),
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
    let leading = token.leading_comments();
    let leading = if leading.is_empty() {
        Doc::nil()
    } else {
        let comments = doc.concat_list(|comments| {
            for comment in leading {
                if !comments.is_empty() {
                    let space = comments.space();
                    comments.push(space);
                }
                let comment = format_comment(comments, &comment);
                comments.push(comment);
            }
        });
        let space = doc.space();
        match placement {
            InlineLeadingTrivia::AfterPreviousToken => doc.concat([space, comments]),
            InlineLeadingTrivia::BeforeToken => doc.concat([comments, space]),
        }
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc.concat([leading, token])
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
