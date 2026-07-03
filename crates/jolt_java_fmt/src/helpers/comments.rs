use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, soft_line, text};
use jolt_java_syntax::{JavaComment, JavaCommentKind, JavaSyntaxToken, TriviaKind};

use crate::comments::CommentMap;
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

pub(crate) fn tokens_have_comments(tokens: &[JavaSyntaxToken]) -> bool {
    tokens.iter().any(token_has_comments)
}

pub(crate) fn token_has_comments(token: &JavaSyntaxToken) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

pub(crate) fn format_construct_leading_comments(
    comments: &CommentMap,
    tokens: &[JavaSyntaxToken],
) -> Doc {
    format_leading_comment_list(comments.leading_comments_for_tokens(tokens))
}

pub(crate) fn format_leading_comment_list(comments: &[JavaComment]) -> Doc {
    let mut docs = Vec::new();
    for comment in comments {
        docs.push(format_comment(comment));
        docs.push(hard_line());
    }
    concat(docs)
}

pub(crate) fn non_formatter_control_comments(comments: Vec<JavaComment>) -> Vec<JavaComment> {
    comments
        .into_iter()
        .filter(|comment| !is_formatter_control_marker(comment.text()))
        .collect()
}

pub(crate) fn format_removed_token_comments(tokens: &[JavaSyntaxToken]) -> Option<Doc> {
    let comments = tokens
        .iter()
        .flat_map(|token| {
            let mut comments = token.leading_comments();
            comments.extend(token.trailing_comments());
            comments
        })
        .collect();
    let comments = non_formatter_control_comments(comments);

    (!comments.is_empty()).then(|| format_dangling_comments(comments))
}

pub(crate) fn format_leading_comments(token: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in token.leading_comments() {
        docs.push(format_comment(&comment));
        docs.push(hard_line());
    }
    concat(docs)
}

pub(crate) fn format_trailing_comments(token: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in token.trailing_comments() {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

pub(crate) fn format_trailing_comments_before_line_break(token: &JavaSyntaxToken) -> Doc {
    let comments = token.trailing_comments();
    let mut docs = Vec::new();
    let comments_len = comments.len();

    for (index, comment) in comments.into_iter().enumerate() {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if index + 1 < comments_len && comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }

    concat(docs)
}

pub(crate) fn format_inline_trailing_comment_list(comments: &[JavaComment]) -> Doc {
    concat(
        comments
            .iter()
            .map(|comment| concat([text(" "), format_comment(comment)]))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn format_separator_with_comments(token: &JavaSyntaxToken, unforced_break: Doc) -> Doc {
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

pub(crate) fn format_dangling_comments(comments: impl IntoIterator<Item = JavaComment>) -> Doc {
    let mut docs = Vec::new();
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

pub(crate) fn trailing_comments_force_line(token: &JavaSyntaxToken) -> bool {
    token.trailing_comments().iter().any(comment_forces_line)
}

pub(crate) fn format_token_with_comments(token: &JavaSyntaxToken) -> Doc {
    format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
}

pub(crate) fn format_token_after_relocated_leading_comments(
    token: &JavaSyntaxToken,
    trailing: TrailingTrivia,
) -> Doc {
    format_token(token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

pub(crate) fn format_token_before_relocated_trailing_comments(
    token: &JavaSyntaxToken,
    leading: LeadingTrivia,
) -> Doc {
    format_token(token, leading, TrailingTrivia::RelocatedToEnclosingContext)
}

pub(crate) fn format_token(
    token: &JavaSyntaxToken,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc {
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
                            text(" ")
                        },
                    ])
                }
            }
            TrailingTrivia::RelocatedToEnclosingContext => jolt_fmt_ir::nil(),
        },
    ])
}

pub(crate) fn format_token_with_inline_leading_comments(
    token: &JavaSyntaxToken,
    placement: InlineLeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc {
    let leading = token.leading_comments();
    concat([
        if leading.is_empty() {
            jolt_fmt_ir::nil()
        } else {
            let comments = jolt_fmt_ir::join(
                text(" "),
                leading.iter().map(format_comment).collect::<Vec<_>>(),
            );
            match placement {
                InlineLeadingTrivia::AfterPreviousToken => concat([text(" "), comments]),
                InlineLeadingTrivia::BeforeToken => concat([comments, text(" ")]),
            }
        },
        format_token_after_relocated_leading_comments(token, trailing),
    ])
}

pub(crate) fn format_token_after_construct_leading_comments(
    token: &JavaSyntaxToken,
    construct_tokens: &[JavaSyntaxToken],
) -> Doc {
    if construct_tokens.first() == Some(token) {
        format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
    } else {
        format_token_with_comments(token)
    }
}

pub(crate) fn format_token_text_after_trivia_relocated(token: &JavaSyntaxToken) -> Doc {
    format_token_text(token.text())
}

pub(crate) fn format_comment(comment: &JavaComment) -> Doc {
    match comment.kind() {
        JavaCommentKind::Line => format_line_comment(comment.text()),
        JavaCommentKind::Block if is_star_block_comment(comment.text()) => {
            format_star_block_comment(comment.text())
        }
        JavaCommentKind::Block => format_block_comment(comment.text()),
        JavaCommentKind::Doc => format_star_block_comment(comment.text()),
    }
}

pub(crate) fn format_raw_comment(kind: TriviaKind, text: &str) -> Doc {
    match kind {
        TriviaKind::LineComment => format_line_comment(text),
        TriviaKind::BlockComment if is_star_block_comment(text) => format_star_block_comment(text),
        TriviaKind::BlockComment => format_block_comment(text),
        TriviaKind::JavadocComment => format_star_block_comment(text),
        TriviaKind::Whitespace | TriviaKind::Newline | TriviaKind::Ignored => jolt_fmt_ir::nil(),
    }
}

pub(crate) fn format_token_text(token_text: &str) -> Doc {
    if token_text.contains(['\n', '\r']) {
        literal_text(token_text.to_owned())
    } else {
        text(token_text.to_owned())
    }
}

pub(crate) fn comment_forces_line(comment: &JavaComment) -> bool {
    comment.kind() == JavaCommentKind::Line || comment.text().contains(['\n', '\r'])
}

pub(crate) fn comment_is_star_block(comment: &JavaComment) -> bool {
    comment.kind() == JavaCommentKind::Doc || is_star_block_comment(comment.text())
}

pub(crate) fn split_leading_comment_barrier_runs<T>(
    items: Vec<T>,
    mut has_leading_comments: impl FnMut(&T) -> bool,
) -> Vec<Vec<T>> {
    let mut runs = Vec::new();
    let mut current_run = Vec::new();

    for item in items {
        if has_leading_comments(&item) && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        current_run.push(item);
    }
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    runs
}

fn format_comment_lines(lines: Vec<String>) -> Doc {
    let mut docs = Vec::new();
    for line in lines {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(text(line));
    }
    concat(docs)
}

fn format_line_comment(comment: &str) -> Doc {
    format_comment_lines(preserve_comment_lines(comment))
}

fn format_block_comment(comment: &str) -> Doc {
    format_comment_lines(preserve_comment_lines(comment))
}

fn format_star_block_comment(comment: &str) -> Doc {
    format_comment_lines(normalize_star_block_comment(comment))
}

fn preserve_comment_lines(comment: &str) -> Vec<String> {
    comment
        .trim()
        .lines()
        .map(|line| line.trim().to_owned())
        .collect()
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

fn normalize_star_block_comment(comment: &str) -> Vec<String> {
    let content = strip_block_comment_delimiters(comment);
    let body = content
        .lines()
        .map(normalize_star_block_body_line)
        .collect::<Vec<_>>();
    let first_content = body.iter().position(|line| !line.is_empty());
    let last_content = body.iter().rposition(|line| !line.is_empty());

    let mut lines = vec!["/**".to_owned()];
    if let (Some(first), Some(last)) = (first_content, last_content) {
        for line in &body[first..=last] {
            if line.is_empty() {
                lines.push(" *".to_owned());
            } else {
                lines.push(format!(" * {line}"));
            }
        }
    }
    lines.push(" */".to_owned());
    lines
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

fn normalize_star_block_body_line(line: &str) -> String {
    line.trim_start().strip_prefix('*').map_or_else(
        || line.trim().to_owned(),
        |line| line.trim_start().to_owned(),
    )
}
