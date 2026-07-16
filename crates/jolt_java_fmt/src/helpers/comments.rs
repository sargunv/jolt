use std::borrow::Cow;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaComment, JavaCommentKind, JavaSyntaxToken, RemovalClaim};

use crate::helpers::formatter_ignore::is_formatter_control_marker;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeadingTrivia {
    Preserve,
    SuppressAlreadyHandled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrailingTrivia {
    Preserve,
    BeforeLineBreak,
    BeforeSoftLine,
    BeforeSpaceIfComments,
    RelocatedToEnclosingContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InlineLeadingTrivia {
    AfterPreviousToken,
    BeforeToken,
}

pub(crate) fn token_has_comments(token: &JavaSyntaxToken<'_>) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

pub(crate) fn token_iter_has_comments<'source>(
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
) -> bool {
    tokens.into_iter().any(|token| token_has_comments(&token))
}

pub(crate) fn comments_from_tokens<'source>(
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
) -> impl Iterator<Item = JavaComment<'source>> {
    tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
}

pub(crate) fn has_removed_comments<'source>(
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> bool {
    comments
        .into_iter()
        .any(|comment| !is_formatter_control_marker(comment.text()))
}

pub(crate) fn format_construct_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_leading_comment_list(
        doc,
        token
            .into_iter()
            .flat_map(JavaSyntaxToken::leading_comments),
    )
}

pub(crate) fn format_leading_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
    })
}

pub(crate) fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Option<Doc<'source>> {
    let mut has_claims = false;
    let mut has_output = false;
    let docs = doc.concat_list(|docs| {
        for comment in comments {
            if is_formatter_control_marker(comment.text()) {
                let claim = docs.source_trivia(comment.source_pieces(), |_| Doc::nil());
                docs.push(claim);
                has_claims = true;
                continue;
            }
            if has_output {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            has_claims = true;
            has_output = true;
        }
    });

    has_claims.then_some(docs)
}

/// Removes a source token only when syntax issued the exact claim.
///
/// A denied claim is expected for malformed syntax and preserves the original
/// token and trivia instead of treating recovery as a formatter invariant.
pub(crate) fn format_token_removal<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    claim: Option<RemovalClaim<'source>>,
) -> (Doc<'source>, bool) {
    let Some(claim) = claim else {
        return (format_token_with_comments(doc, token), false);
    };
    let removed = doc.removed_source(claim);
    let comments =
        format_removed_comments(doc, comments_from_tokens([*token])).unwrap_or_else(Doc::nil);
    (doc.concat([removed, comments]), true)
}

pub(crate) fn format_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in token.leading_comments() {
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
    })
}

pub(crate) fn format_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in token.trailing_comments() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

pub(crate) fn format_trailing_comments_before_line_break<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let mut comments = token.trailing_comments().peekable();
    doc.concat_list(|docs| {
        while let Some(comment) = comments.next() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comments.peek().is_some() && comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

pub(crate) fn format_inline_trailing_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment = format_comment(docs, &comment);
            docs.push(comment);
        }
    })
}

pub(crate) fn format_separator_with_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    unforced_break: Doc<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_token(
                doc,
                token,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if trailing_comments_force_line(token) {
                doc.hard_line()
            } else {
                unforced_break
            },
        ]
    )
}

pub(crate) fn format_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
        }
    })
}

pub(crate) fn has_delimiter_dangling_comments<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> bool {
    open.is_some_and(|token| !token.trailing_comments().is_empty())
        || close.is_some_and(|token| !token.leading_comments().is_empty())
}

pub(crate) fn delimiter_dangling_comments<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> impl Iterator<Item = JavaComment<'source>> {
    open.into_iter()
        .flat_map(JavaSyntaxToken::trailing_comments)
        .chain(
            close
                .into_iter()
                .flat_map(JavaSyntaxToken::leading_comments),
        )
}

pub(crate) fn trailing_comments_force_line(token: &JavaSyntaxToken<'_>) -> bool {
    token
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment))
}

pub(crate) fn format_token_with_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

pub(crate) fn format_token_after_relocated_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token(doc, token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

pub(crate) fn format_token_before_relocated_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

pub(crate) fn format_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let token_text = doc.source_token(token);
    format_token_doc(doc, token, token_text, leading, trailing)
}

pub(crate) fn format_token_doc<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    token_doc: Doc<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            match leading {
                LeadingTrivia::Preserve => format_leading_comments(doc, token),
                LeadingTrivia::SuppressAlreadyHandled => Doc::nil(),
            },
            token_doc,
            match trailing {
                TrailingTrivia::Preserve => format_trailing_comments(doc, token),
                TrailingTrivia::BeforeLineBreak => {
                    format_trailing_comments_before_line_break(doc, token)
                }
                TrailingTrivia::BeforeSoftLine => doc_concat!(
                    doc,
                    [
                        format_trailing_comments_before_line_break(doc, token),
                        if trailing_comments_force_line(token) {
                            doc.hard_line()
                        } else {
                            doc.soft_line()
                        },
                    ]
                ),
                TrailingTrivia::BeforeSpaceIfComments => {
                    if token.trailing_comments().is_empty() {
                        Doc::nil()
                    } else {
                        doc_concat!(
                            doc,
                            [
                                format_trailing_comments_before_line_break(doc, token),
                                if trailing_comments_force_line(token) {
                                    doc.hard_line()
                                } else {
                                    doc.space()
                                },
                            ]
                        )
                    }
                }
                TrailingTrivia::RelocatedToEnclosingContext => Doc::nil(),
            },
        ]
    )
}

pub(crate) fn format_token_with_inline_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
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
            InlineLeadingTrivia::AfterPreviousToken => doc_concat!(doc, [space, comments]),
            InlineLeadingTrivia::BeforeToken => doc_concat!(doc, [comments, space]),
        }
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc_concat!(doc, [leading, token])
}

pub(crate) fn format_token_after_construct_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    construct_first_token: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if construct_first_token == Some(token) {
        format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
    } else {
        format_token_with_comments(doc, token)
    }
}

pub(crate) fn format_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &JavaComment<'source>,
) -> Doc<'source> {
    doc.source_trivia(comment.source_pieces(), |doc| match comment.kind() {
        JavaCommentKind::Line => format_line_comment(doc, comment.text()),
        JavaCommentKind::Block if is_star_block_comment(comment.text()) => {
            format_star_block_comment(doc, comment.text())
        }
        JavaCommentKind::Block => format_block_comment(doc, comment.text()),
        JavaCommentKind::Doc => format_star_block_comment(doc, comment.text()),
    })
}

pub(crate) fn comment_forces_line(comment: &JavaComment<'_>) -> bool {
    comment.kind() == JavaCommentKind::Line || comment.text().contains(['\n', '\r'])
}

pub(crate) fn comment_is_star_block(comment: &JavaComment<'_>) -> bool {
    comment.kind() == JavaCommentKind::Doc || is_star_block_comment(comment.text())
}

fn format_comment_lines<'source>(
    doc: &mut DocBuilder<'source>,
    lines: impl IntoIterator<Item = impl Into<Cow<'source, str>>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for line in lines {
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let line = docs.text(line);
            docs.push(line);
        }
    })
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
    format_comment_lines(doc, preserved_comment_lines(comment))
}

fn format_star_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    let content = strip_block_comment_delimiters(comment);
    doc.concat_list(|docs| {
        let open = docs.literal_text("/**");
        docs.push(open);

        let mut has_content = false;
        let mut pending_blank_lines = 0;
        for line in content.lines().map(normalize_star_block_body_line) {
            if line.is_empty() {
                if has_content {
                    pending_blank_lines += 1;
                }
                continue;
            }

            has_content = true;
            for _ in 0..pending_blank_lines {
                let line = docs.literal_text(" *");
                let hard_line = docs.hard_line();
                docs.push(hard_line);
                docs.push(line);
            }
            pending_blank_lines = 0;
            let prefix = docs.literal_text(" * ");
            let line = docs.text(line);
            let line = doc_concat!(docs, [prefix, line]);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
            docs.push(line);
        }

        let close = docs.literal_text(" */");
        let hard_line = docs.hard_line();
        docs.push(hard_line);
        docs.push(close);
    })
}

fn preserved_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    comment.trim().lines().map(str::trim)
}

fn is_star_block_comment(comment: &str) -> bool {
    let trimmed = comment.trim_start();
    if trimmed.starts_with("/**") {
        return true;
    }

    let Some(body) = trimmed.strip_prefix("/*") else {
        return false;
    };

    body.lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_start().starts_with('*'))
}

fn strip_block_comment_delimiters(comment: &str) -> &str {
    comment
        .trim()
        .strip_prefix("/**")
        .or_else(|| comment.trim().strip_prefix("/*"))
        .unwrap_or(comment.trim())
        .strip_suffix("*/")
        .unwrap_or_else(|| {
            comment
                .trim()
                .strip_prefix("/**")
                .or_else(|| comment.trim().strip_prefix("/*"))
                .unwrap_or(comment.trim())
        })
}

fn normalize_star_block_body_line(line: &str) -> &str {
    line.trim_start()
        .strip_prefix('*')
        .map_or_else(|| line.trim(), str::trim_start)
}
