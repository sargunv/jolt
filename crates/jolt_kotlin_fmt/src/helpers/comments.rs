use std::borrow::Cow;

use jolt_fmt_ir::{Doc, DocBuilder, DocList};
use jolt_kotlin_syntax::{KotlinComment, KotlinCommentKind, KotlinSyntaxToken};

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
    BeforeToken,
}

pub(crate) fn format_leading_comment_runs<'source, Item>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = Item>,
    mut has_leading_comments: impl FnMut(&Item) -> bool,
    mut format_run: impl FnMut(&mut DocBuilder<'source>, Vec<Item>) -> Doc<'source>,
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
    format_run: &mut impl FnMut(&mut DocBuilder<'source>, Vec<Item>) -> Doc<'source>,
) {
    if !docs.is_empty() {
        let empty_line = doc.empty_line();
        docs.push(empty_line, doc);
    }
    let run = format_run(doc, run);
    docs.push(run, doc);
}

pub(crate) fn format_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for comment in token.leading_comments() {
        let comment = format_comment(doc, &comment);
        docs.push(comment, doc);
        let hard_line = doc.hard_line();
        docs.push(hard_line, doc);
    }
    docs.finish(doc)
}

pub(crate) fn format_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for comment in token.trailing_comments() {
        let space = doc.space();
        docs.push(space, doc);
        let comment_doc = format_comment(doc, &comment);
        docs.push(comment_doc, doc);
        if comment_forces_line(&comment) {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
    }
    docs.finish(doc)
}

pub(crate) fn format_trailing_comments_before_line_break<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
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

pub(crate) fn format_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
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

pub(crate) fn comments_from_tokens<'source>(
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> impl Iterator<Item = KotlinComment<'source>> {
    tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
}

pub(crate) fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Option<Doc<'source>> {
    let mut docs = doc.list();
    for comment in comments {
        if is_formatter_control_marker(comment.text()) {
            continue;
        }
        if !docs.is_empty() {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
        let comment = format_comment(doc, &comment);
        docs.push(comment, doc);
    }

    (!docs.is_empty()).then(|| docs.finish(doc))
}

pub(crate) fn has_delimiter_dangling_comments<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> bool {
    open.is_some_and(|token| !token.trailing_comments().is_empty())
        || close.is_some_and(|token| !token.leading_comments().is_empty())
}

pub(crate) fn delimiter_dangling_comments<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> impl Iterator<Item = KotlinComment<'source>> {
    open.into_iter()
        .flat_map(KotlinSyntaxToken::trailing_comments)
        .chain(
            close
                .into_iter()
                .flat_map(KotlinSyntaxToken::leading_comments),
        )
}

pub(crate) fn format_separator_with_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
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

pub(crate) fn format_token_after_relocated_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token(doc, token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

pub(crate) fn format_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = match leading {
        LeadingTrivia::Preserve => format_leading_comments(doc, token),
        LeadingTrivia::SuppressAlreadyHandled => doc.nil(),
    };
    let token_doc = format_token_text(doc, token.text());
    let trailing = match trailing {
        TrailingTrivia::Preserve => format_trailing_comments(doc, token),
        TrailingTrivia::BeforeLineBreak => format_trailing_comments_before_line_break(doc, token),
        TrailingTrivia::BeforeSoftLine => {
            let comments = format_trailing_comments_before_line_break(doc, token);
            let line = if trailing_comments_force_line(token) {
                doc.hard_line()
            } else {
                doc.soft_line()
            };
            doc.concat([comments, line])
        }
        TrailingTrivia::BeforeSpaceIfComments => {
            if token.trailing_comments().is_empty() {
                doc.nil()
            } else {
                let comments = format_trailing_comments_before_line_break(doc, token);
                let line = if trailing_comments_force_line(token) {
                    doc.hard_line()
                } else {
                    doc.space()
                };
                doc.concat([comments, line])
            }
        }
        TrailingTrivia::RelocatedToEnclosingContext => doc.nil(),
    };
    doc.concat([leading, token_doc, trailing])
}

pub(crate) fn format_token_sequence<'source>(
    doc: &mut DocBuilder<'source>,
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let tokens = tokens.into_iter();
    let mut docs = doc.list();
    let mut previous = None;

    for (index, token) in tokens.enumerate() {
        if let Some(previous) = previous {
            let gap = format_token_gap(doc, &previous, &token);
            docs.push(gap, doc);
        }
        let token_doc = format_token(
            doc,
            &token,
            if index == 0 {
                leading
            } else {
                LeadingTrivia::Preserve
            },
            TrailingTrivia::Preserve,
        );
        docs.push(token_doc, doc);
        previous = Some(token);
    }

    docs.finish(doc)
}

pub(crate) fn format_token_gap<'source>(
    doc: &mut DocBuilder<'source>,
    left: &KotlinSyntaxToken<'source>,
    right: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let start = left.token_text_range().end().get();
    let end = right.token_text_range().start().get();
    if start >= end || trailing_comments_force_line(left) {
        return doc.nil();
    }

    let gap = &left.source()[start..end];
    if gap.contains(['\n', '\r']) {
        doc.hard_line()
    } else if gap.chars().any(char::is_whitespace) {
        doc.space()
    } else {
        doc.nil()
    }
}

pub(crate) fn format_token_with_inline_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    placement: InlineLeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = token.leading_comments();
    let leading = if leading.is_empty() {
        doc.nil()
    } else {
        let comments = leading
            .map(|comment| format_comment(doc, &comment))
            .collect::<Vec<_>>();
        let space = doc.space();
        let comments = doc.join(space, comments);
        match placement {
            InlineLeadingTrivia::BeforeToken => {
                let space = doc.space();
                doc.concat([comments, space])
            }
        }
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc.concat([leading, token])
}

pub(crate) fn format_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &KotlinComment<'source>,
) -> Doc<'source> {
    match comment.kind() {
        KotlinCommentKind::Line | KotlinCommentKind::Block => {
            format_comment_lines(doc, preserved_comment_lines(comment.text()))
        }
        KotlinCommentKind::Doc => format_star_block_comment(doc, comment.text()),
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

pub(crate) fn comment_forces_line(comment: &KotlinComment<'_>) -> bool {
    comment.kind() == KotlinCommentKind::Line || comment.text().contains(['\n', '\r'])
}

pub(crate) fn trailing_comments_force_line(token: &KotlinSyntaxToken<'_>) -> bool {
    token
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment))
}

pub(crate) fn token_has_comments(token: &KotlinSyntaxToken<'_>) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

fn format_comment_lines<'source>(
    doc: &mut DocBuilder<'source>,
    lines: impl IntoIterator<Item = impl Into<Cow<'source, str>>>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for line in lines {
        if !docs.is_empty() {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
        let line = doc.text(line);
        docs.push(line, doc);
    }
    docs.finish(doc)
}

fn format_star_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    let content = strip_block_comment_delimiters(doc, comment);
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
            let blank = doc.literal_text(" *");
            push_comment_line(doc, &mut docs, blank);
        }
        pending_blank_lines = 0;
        let prefix = doc.literal_text(" * ");
        let line = doc.text(line);
        let line = doc.concat([prefix, line]);
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

fn strip_block_comment_delimiters<'source>(
    _doc: &mut DocBuilder<'_>,
    comment: &'source str,
) -> &'source str {
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
