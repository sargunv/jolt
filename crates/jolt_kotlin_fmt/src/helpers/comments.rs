use std::borrow::Cow;

use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinComment, KotlinCommentKind, KotlinSyntaxToken, TokenGap, token_gap,
};

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
    let mut current_run = Vec::with_capacity(lower);
    doc.concat_list(|docs| {
        for item in items {
            if has_leading_comments(&item) && !current_run.is_empty() {
                push_leading_comment_run(docs, std::mem::take(&mut current_run), &mut format_run);
            }
            current_run.push(item);
        }
        if !current_run.is_empty() {
            push_leading_comment_run(docs, current_run, &mut format_run);
        }
    })
}

fn push_leading_comment_run<'source, Item>(
    docs: &mut ConcatBuilder<'_, 'source>,
    run: Vec<Item>,
    format_run: &mut impl FnMut(&mut DocBuilder<'source>, Vec<Item>) -> Doc<'source>,
) {
    if !docs.is_empty() {
        let empty_line = docs.empty_line();
        docs.push(empty_line);
    }
    let run = format_run(docs, run);
    docs.push(run);
}

pub(crate) fn format_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
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
    token: &KotlinSyntaxToken<'source>,
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
    token: &KotlinSyntaxToken<'source>,
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

pub(crate) fn format_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
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
    let mut is_empty = true;
    let comments = doc.concat_list(|docs| {
        for comment in comments {
            if is_formatter_control_marker(comment.text()) {
                continue;
            }
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
        }
        is_empty = docs.is_empty();
    });
    (!is_empty).then_some(comments)
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
    let mut previous = None;
    doc.concat_list(|docs| {
        for (index, token) in tokens.enumerate() {
            if let Some(previous) = previous {
                let gap = format_token_gap(docs, &previous, &token);
                docs.push(gap);
            }
            let token_doc = format_token(
                docs,
                &token,
                if index == 0 {
                    leading
                } else {
                    LeadingTrivia::Preserve
                },
                TrailingTrivia::Preserve,
            );
            docs.push(token_doc);
            previous = Some(token);
        }
    })
}

pub(crate) fn format_token_gap<'source>(
    doc: &mut DocBuilder<'source>,
    left: &KotlinSyntaxToken<'source>,
    right: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    if trailing_comments_force_line(left) {
        return doc.nil();
    }

    match token_gap(left, right) {
        TokenGap::None => doc.nil(),
        TokenGap::Whitespace => doc.space(),
        TokenGap::Line => doc.hard_line(),
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

fn format_star_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    let content = strip_block_comment_delimiters(doc, comment);
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
                let blank = docs.literal_text(" *");
                push_comment_line(docs, blank);
            }
            pending_blank_lines = 0;
            let prefix = docs.literal_text(" * ");
            let line = docs.text(line);
            let line = docs.concat([prefix, line]);
            push_comment_line(docs, line);
        }

        let close = docs.literal_text(" */");
        push_comment_line(docs, close);
    })
}

fn push_comment_line<'source>(docs: &mut ConcatBuilder<'_, 'source>, line: Doc<'source>) {
    if !docs.is_empty() {
        let hard_line = docs.hard_line();
        docs.push(hard_line);
    }
    docs.push(line);
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
