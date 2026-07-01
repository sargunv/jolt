use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};
use jolt_java_syntax::{JavaComment, JavaCommentKind, JavaSyntaxKind, JavaSyntaxToken, TriviaKind};

use crate::helpers::formatter_ignore::is_formatter_control_marker;

pub(crate) fn format_token_sequence(tokens: &[JavaSyntaxToken]) -> Doc {
    let mut docs = Vec::new();
    let mut previous_kind = None;
    let mut after_forced_break = false;

    for token in tokens {
        for comment in token.leading_comments() {
            if !docs.is_empty() && !after_forced_break {
                docs.push(hard_line());
            }
            docs.push(format_comment(&comment));
            docs.push(hard_line());
            previous_kind = None;
            after_forced_break = true;
        }

        if !after_forced_break
            && previous_kind.is_some_and(|previous| needs_space(previous, token.kind()))
        {
            docs.push(text(" "));
        }

        docs.push(format_token_text(token.text()));
        after_forced_break = false;

        let mut forced_break_after_token = false;
        for comment in token.trailing_comments() {
            docs.push(text(" "));
            docs.push(format_comment(&comment));
            if comment_forces_line(&comment) {
                docs.push(hard_line());
                forced_break_after_token = true;
            }
        }

        if forced_break_after_token {
            previous_kind = None;
            after_forced_break = true;
        } else {
            previous_kind = Some(token.kind());
        }
    }

    concat(docs)
}

pub(crate) fn tokens_have_comments(tokens: &[JavaSyntaxToken]) -> bool {
    tokens.iter().any(token_has_comments)
}

pub(crate) fn token_has_comments(token: &JavaSyntaxToken) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

pub(crate) fn format_construct_leading_comments(tokens: &[JavaSyntaxToken]) -> Doc {
    tokens
        .first()
        .map_or_else(jolt_fmt_ir::nil, format_leading_comments)
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
    concat([
        format_leading_comments(token),
        format_token_text(token.text()),
        format_trailing_comments(token),
    ])
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
    trimmed.starts_with("/**") || trimmed.starts_with("/*\n") || trimmed.starts_with("/*\r")
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

fn needs_space(previous: JavaSyntaxKind, current: JavaSyntaxKind) -> bool {
    if suppresses_space_before(current) || suppresses_space_after(previous) {
        return false;
    }

    if current == JavaSyntaxKind::LParen
        && matches!(
            previous,
            JavaSyntaxKind::Identifier
                | JavaSyntaxKind::ThisKw
                | JavaSyntaxKind::SuperKw
                | JavaSyntaxKind::Gt
                | JavaSyntaxKind::RBracket
        )
    {
        return false;
    }

    if current == JavaSyntaxKind::LBracket {
        return false;
    }

    if previous == JavaSyntaxKind::Comma || previous == JavaSyntaxKind::Semicolon {
        return true;
    }

    true
}

const fn suppresses_space_before(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::RParen
            | JavaSyntaxKind::RBracket
            | JavaSyntaxKind::RBrace
            | JavaSyntaxKind::Comma
            | JavaSyntaxKind::Semicolon
            | JavaSyntaxKind::Dot
            | JavaSyntaxKind::DoubleColon
            | JavaSyntaxKind::Colon
            | JavaSyntaxKind::Ellipsis
            | JavaSyntaxKind::Lt
            | JavaSyntaxKind::Gt
    )
}

const fn suppresses_space_after(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::LParen
            | JavaSyntaxKind::LBracket
            | JavaSyntaxKind::LBrace
            | JavaSyntaxKind::Dot
            | JavaSyntaxKind::DoubleColon
            | JavaSyntaxKind::At
            | JavaSyntaxKind::Lt
    )
}
