use std::borrow::Cow;

use jolt_fmt_ir::{
    Doc, concat, empty_line, hard_line, join, literal_text as literal, soft_line, space, text,
};
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
    items: impl IntoIterator<Item = Item>,
    mut has_leading_comments: impl FnMut(&Item) -> bool,
    mut format_run: impl FnMut(Vec<Item>) -> Doc<'source>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut current_run = Vec::new();

    for item in items {
        if has_leading_comments(&item) && !current_run.is_empty() {
            push_leading_comment_run(&mut docs, std::mem::take(&mut current_run), &mut format_run);
        }
        current_run.push(item);
    }
    if !current_run.is_empty() {
        push_leading_comment_run(&mut docs, current_run, &mut format_run);
    }

    concat(docs)
}

fn push_leading_comment_run<'source, Item>(
    docs: &mut Vec<Doc<'source>>,
    run: Vec<Item>,
    format_run: &mut impl FnMut(Vec<Item>) -> Doc<'source>,
) {
    if !docs.is_empty() {
        docs.push(empty_line());
    }
    docs.push(format_run(run));
}

pub(crate) fn format_leading_comments<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in token.leading_comments() {
        docs.push(format_comment(&comment));
        docs.push(hard_line());
    }
    concat(docs)
}

pub(crate) fn format_trailing_comments<'source>(
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in token.trailing_comments() {
        docs.push(space());
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

pub(crate) fn format_trailing_comments_before_line_break<'source>(
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let mut comments = token.trailing_comments().peekable();
    let mut docs = Vec::new();

    while let Some(comment) = comments.next() {
        docs.push(space());
        docs.push(format_comment(&comment));
        if comments.peek().is_some() && comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }

    concat(docs)
}

pub(crate) fn format_dangling_comments<'source>(
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

pub(crate) fn comments_from_tokens<'source>(
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> impl Iterator<Item = KotlinComment<'source>> {
    tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
}

pub(crate) fn has_removed_comments<'source>(
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> bool {
    comments
        .into_iter()
        .any(|comment| !is_formatter_control_marker(comment.text()))
}

pub(crate) fn format_removed_comments<'source>(
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Option<Doc<'source>> {
    let mut docs = Vec::new();
    for comment in comments {
        if is_formatter_control_marker(comment.text()) {
            continue;
        }
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }

    (!docs.is_empty()).then(|| concat(docs))
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
    token: &KotlinSyntaxToken<'source>,
    unforced_break: Doc<'source>,
) -> Doc<'source> {
    concat([
        format_token(
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        if trailing_comments_force_line(token) {
            hard_line()
        } else {
            unforced_break
        },
    ])
}

pub(crate) fn format_token_after_relocated_leading_comments<'source>(
    token: &KotlinSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token(token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

pub(crate) fn format_token<'source>(
    token: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    concat([
        match leading {
            LeadingTrivia::Preserve => format_leading_comments(token),
            LeadingTrivia::SuppressAlreadyHandled => jolt_fmt_ir::nil(),
        },
        format_token_text(token.text()),
        match trailing {
            TrailingTrivia::Preserve => format_trailing_comments(token),
            TrailingTrivia::BeforeLineBreak => format_trailing_comments_before_line_break(token),
            TrailingTrivia::BeforeSoftLine => concat([
                format_trailing_comments_before_line_break(token),
                if trailing_comments_force_line(token) {
                    hard_line()
                } else {
                    soft_line()
                },
            ]),
            TrailingTrivia::BeforeSpaceIfComments => {
                if token.trailing_comments().is_empty() {
                    jolt_fmt_ir::nil()
                } else {
                    concat([
                        format_trailing_comments_before_line_break(token),
                        if trailing_comments_force_line(token) {
                            hard_line()
                        } else {
                            space()
                        },
                    ])
                }
            }
            TrailingTrivia::RelocatedToEnclosingContext => jolt_fmt_ir::nil(),
        },
    ])
}

pub(crate) fn format_token_sequence<'source>(
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut previous = None;

    for (index, token) in tokens.into_iter().enumerate() {
        if let Some(previous) = previous {
            docs.push(format_token_gap(&previous, &token));
        }
        docs.push(format_token(
            &token,
            if index == 0 {
                leading
            } else {
                LeadingTrivia::Preserve
            },
            TrailingTrivia::Preserve,
        ));
        previous = Some(token);
    }

    concat(docs)
}

pub(crate) fn format_token_gap<'source>(
    left: &KotlinSyntaxToken<'source>,
    right: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let start = left.token_text_range().end().get();
    let end = right.token_text_range().start().get();
    if start >= end || trailing_comments_force_line(left) {
        return jolt_fmt_ir::nil();
    }

    let gap = &left.source()[start..end];
    if gap.contains(['\n', '\r']) {
        hard_line()
    } else if gap.chars().any(char::is_whitespace) {
        space()
    } else {
        jolt_fmt_ir::nil()
    }
}

pub(crate) fn format_token_with_inline_leading_comments<'source>(
    token: &KotlinSyntaxToken<'source>,
    placement: InlineLeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = token.leading_comments();
    concat([
        if leading.is_empty() {
            jolt_fmt_ir::nil()
        } else {
            let comments = join(&space(), leading.map(|comment| format_comment(&comment)));
            match placement {
                InlineLeadingTrivia::BeforeToken => concat([comments, space()]),
            }
        },
        format_token_after_relocated_leading_comments(token, trailing),
    ])
}

pub(crate) fn format_comment<'source>(comment: &KotlinComment<'source>) -> Doc<'source> {
    match comment.kind() {
        KotlinCommentKind::Line | KotlinCommentKind::Block => {
            format_comment_lines(preserved_comment_lines(comment.text()))
        }
        KotlinCommentKind::Doc => format_star_block_comment(comment.text()),
    }
}

pub(crate) fn format_token_text(token_text: &str) -> Doc<'_> {
    if token_text.contains(['\n', '\r']) {
        literal(token_text)
    } else {
        text(token_text)
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
    lines: impl IntoIterator<Item = impl Into<Cow<'source, str>>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for line in lines {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(text(line));
    }
    concat(docs)
}

fn format_star_block_comment(comment: &str) -> Doc<'_> {
    let content = strip_block_comment_delimiters(comment);
    let mut docs = vec![literal("/**")];

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
            push_comment_line(&mut docs, literal(" *"));
        }
        pending_blank_lines = 0;
        push_comment_line(&mut docs, concat([literal(" * "), text(line)]));
    }

    push_comment_line(&mut docs, literal(" */"));
    concat(docs)
}

fn push_comment_line<'source>(docs: &mut Vec<Doc<'source>>, line: Doc<'source>) {
    if !docs.is_empty() {
        docs.push(hard_line());
    }
    docs.push(line);
}

fn preserved_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    comment.trim().lines().map(str::trim)
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
