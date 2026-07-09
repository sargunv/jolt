use std::borrow::Cow;

use jolt_fmt_ir::{Doc, DocBuilder, DocList};
use jolt_java_syntax::{JavaComment, JavaCommentKind, JavaSyntaxToken};

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

pub(crate) fn format_leading_comment_runs<'source, Item>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = Item>,
    mut has_leading_comments: impl FnMut(&Item) -> bool,
    mut format_run: impl FnMut(Vec<Item>, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    let items = items.into_iter();
    let (lower, _) = items.size_hint();
    let mut docs = doc.list();
    let mut current_run = Vec::with_capacity(lower);

    for item in items {
        if has_leading_comments(&item) && !current_run.is_empty() {
            push_leading_comment_run(
                doc,
                &mut docs,
                std::mem::take(&mut current_run),
                &mut format_run,
            );
        }
        current_run.push(item);
    }
    if !current_run.is_empty() {
        push_leading_comment_run(doc, &mut docs, current_run, &mut format_run);
    }

    docs.finish(doc)
}

fn push_leading_comment_run<'source, Item>(
    doc: &mut DocBuilder<'source>,
    docs: &mut DocList<'source>,
    run: Vec<Item>,
    format_run: &mut impl FnMut(Vec<Item>, &mut DocBuilder<'source>) -> Doc<'source>,
) {
    if !docs.is_empty() {
        docs.push(doc.empty_line(), doc);
    }
    docs.push(format_run(run, doc), doc);
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
    let comments = comments.into_iter();
    let mut docs = doc.list();
    for comment in comments {
        docs.push(format_comment(doc, &comment), doc);
        docs.push(doc.hard_line(), doc);
    }
    docs.finish(doc)
}

pub(crate) fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Option<Doc<'source>> {
    let comments = comments.into_iter();
    let mut docs = doc.list();
    for comment in comments {
        if is_formatter_control_marker(comment.text()) {
            continue;
        }
        if !docs.is_empty() {
            docs.push(doc.hard_line(), doc);
        }
        docs.push(format_comment(doc, &comment), doc);
    }

    (!docs.is_empty()).then(|| docs.finish(doc))
}

pub(crate) fn format_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let comments = token.leading_comments();
    let mut docs = doc.list();
    for comment in comments {
        docs.push(format_comment(doc, &comment), doc);
        docs.push(doc.hard_line(), doc);
    }
    docs.finish(doc)
}

pub(crate) fn format_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let comments = token.trailing_comments();
    let mut docs = doc.list();
    for comment in comments {
        docs.push(doc.space(), doc);
        docs.push(format_comment(doc, &comment), doc);
        if comment_forces_line(&comment) {
            docs.push(doc.hard_line(), doc);
        }
    }
    docs.finish(doc)
}

pub(crate) fn format_trailing_comments_before_line_break<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let mut comments = token.trailing_comments().peekable();
    let mut docs = doc.list();

    while let Some(comment) = comments.next() {
        let space = doc.space();
        docs.push(space, doc);
        let comment_doc = format_comment(doc, &comment);
        docs.push(comment_doc, doc);
        if comments.peek().is_some() && comment_forces_line(&comment) {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
    }

    docs.finish(doc)
}

pub(crate) fn format_inline_trailing_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for comment in comments {
        let space = doc.space();
        docs.push(space, doc);
        let comment = format_comment(doc, &comment);
        docs.push(comment, doc);
    }
    docs.finish(doc)
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
    let comments = comments.into_iter();
    let mut docs = doc.list();
    for comment in comments {
        if !docs.is_empty() {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
        let comment = format_comment(doc, &comment);
        docs.push(comment, doc);
    }
    docs.finish(doc)
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
    doc_concat!(
        doc,
        [
            match leading {
                LeadingTrivia::Preserve => format_leading_comments(doc, token),
                LeadingTrivia::SuppressAlreadyHandled => Doc::nil(),
            },
            format_token_text(doc, token.text()),
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

pub(crate) fn format_token_sequence<'source>(
    doc: &mut DocBuilder<'source>,
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let tokens = tokens.into_iter();
    let mut docs = doc.list();
    let mut previous = None;

    for (index, token) in tokens.enumerate() {
        if let Some(previous) = previous {
            docs.push(format_recovered_token_gap(doc, &previous, &token), doc);
        }
        let formatted = format_token(
            doc,
            &token,
            if index == 0 {
                leading
            } else {
                LeadingTrivia::Preserve
            },
            TrailingTrivia::Preserve,
        );
        docs.push(formatted, doc);
        previous = Some(token);
    }

    docs.finish(doc)
}

fn format_recovered_token_gap<'source>(
    doc: &mut DocBuilder<'source>,
    left: &JavaSyntaxToken<'source>,
    right: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let start = left.token_text_range().end().get();
    let end = right.token_text_range().start().get();
    if start >= end || trailing_comments_force_line(left) {
        return Doc::nil();
    }

    let gap = &left.source()[start..end];
    if gap.contains(['\n', '\r']) {
        doc.hard_line()
    } else if gap.chars().any(char::is_whitespace) {
        doc.space()
    } else {
        Doc::nil()
    }
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
        let mut comments = doc.list();
        for comment in leading {
            if !comments.is_empty() {
                let space = doc.space();
                comments.push(space, doc);
            }
            let comment = format_comment(doc, &comment);
            comments.push(comment, doc);
        }
        let comments = comments.finish(doc);
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
    match comment.kind() {
        JavaCommentKind::Line => format_line_comment(doc, comment.text()),
        JavaCommentKind::Block if is_star_block_comment(comment.text()) => {
            format_star_block_comment(doc, comment.text())
        }
        JavaCommentKind::Block => format_block_comment(doc, comment.text()),
        JavaCommentKind::Doc => format_star_block_comment(doc, comment.text()),
    }
}

pub(crate) fn format_token_text<'source>(
    doc: &mut DocBuilder<'source>,
    token_text: &'source str,
) -> Doc<'source> {
    if token_text.contains(['\n', '\r']) {
        doc.literal_text(token_text)
    } else {
        doc.text(token_text)
    }
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
    let lines = lines.into_iter();
    let mut docs = doc.list();
    for line in lines {
        if !docs.is_empty() {
            docs.push(doc.hard_line(), doc);
        }
        docs.push(doc.text(line), doc);
    }
    docs.finish(doc)
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
    let mut docs = doc.list();
    let open = doc.literal_text("/**");
    docs.push(open, doc);

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
            let line = doc.literal_text(" *");
            push_comment_line(doc, &mut docs, line);
        }
        pending_blank_lines = 0;
        let prefix = doc.literal_text(" * ");
        let line = doc.text(line);
        let line = doc_concat!(doc, [prefix, line]);
        push_comment_line(doc, &mut docs, line);
    }

    let close = doc.literal_text(" */");
    push_comment_line(doc, &mut docs, close);
    docs.finish(doc)
}

fn push_comment_line<'source>(
    doc: &mut DocBuilder<'source>,
    docs: &mut DocList<'source>,
    line: Doc<'source>,
) {
    if !docs.is_empty() {
        let hard_line = doc.hard_line();
        docs.push(hard_line, doc);
    }
    docs.push(line, doc);
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
