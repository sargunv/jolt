use jolt_java_syntax::TriviaKind;

use crate::context::{JavaCommentTrivia, JavaFormatContext};
use crate::policy::JavaFormatPolicy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommentPlacement {
    OwnLine,
    InlineBlock,
    TrailingLine,
}

pub(crate) fn rewrite_comment_lines(
    context: &JavaFormatContext<'_>,
    comment: &JavaCommentTrivia,
    placement: CommentPlacement,
) -> Vec<String> {
    let raw = context.raw_text(comment);
    if let Some(formatted) = reformat_parameter_comment(raw) {
        return vec![formatted];
    }

    let column0 = match placement {
        CommentPlacement::OwnLine => 0,
        CommentPlacement::InlineBlock | CommentPlacement::TrailingLine => {
            context.source_column_at(comment.trivia.range.start().get())
        }
    };

    match comment.trivia.kind {
        TriviaKind::LineComment => rewrite_line_comment_lines(raw, column0, context.policy()),
        TriviaKind::BlockComment | TriviaKind::JavadocComment => {
            rewrite_block_comment_lines(raw, column0, placement)
        }
        TriviaKind::Ignored => vec![raw.to_owned()],
        TriviaKind::Whitespace | TriviaKind::Newline => {
            unreachable!("comment buckets only store formatter-accounted comment trivia")
        }
    }
}

fn rewrite_block_comment_lines(
    raw: &str,
    column0: usize,
    placement: CommentPlacement,
) -> Vec<String> {
    let lines = raw_comment_lines(raw)
        .into_iter()
        .map(strip_trailing_whitespace)
        .collect::<Vec<_>>();
    if matches!(placement, CommentPlacement::InlineBlock) && !javadoc_shaped(&lines) {
        return lines;
    }
    if javadoc_shaped(&lines) {
        return indent_javadoc_lines(lines, column0);
    }

    preserve_comment_indentation(lines, column0)
}

fn rewrite_line_comment_lines(raw: &str, column0: usize, policy: JavaFormatPolicy) -> Vec<String> {
    let lines = raw_comment_lines(raw)
        .into_iter()
        .map(|line| line.trim().to_owned())
        .collect::<Vec<_>>();
    indent_line_comments(wrap_line_comments(lines, column0, policy), column0)
}

fn reformat_parameter_comment(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("/*") || !trimmed.ends_with("*/") {
        return None;
    }

    let inner = trimmed.strip_prefix("/*")?.strip_suffix("*/")?.trim();
    let eq_idx = inner.rfind('=')?;
    let name = inner[..eq_idx].trim();
    if !inner[eq_idx + 1..].trim().is_empty() {
        return None;
    }
    parameter_comment_name(name)?;

    Some(format!("/* {name}= */"))
}

fn parameter_comment_name(body: &str) -> Option<&str> {
    if body.ends_with("...") {
        let prefix = body.strip_suffix("...")?;
        if is_java_identifier(prefix) {
            return Some(body);
        }
        return None;
    }

    is_java_identifier(body).then_some(body)
}

fn is_java_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    if !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        return false;
    }
    true
}

fn wrap_line_comments(lines: Vec<String>, column0: usize, policy: JavaFormatPolicy) -> Vec<String> {
    let max_width = policy.max_line_length();
    let mut result = Vec::new();
    for mut line in lines {
        line = normalize_line_comment_prefix(&line);
        if line.starts_with("// MOE:") {
            result.push(line);
            continue;
        }
        while line.len() + column0 > max_width {
            let mut idx = max_width.saturating_sub(column0);
            while idx > 2
                && !line
                    .as_bytes()
                    .get(idx)
                    .is_some_and(u8::is_ascii_whitespace)
            {
                idx -= 1;
            }
            if idx <= 2 {
                break;
            }
            result.push(line[..idx].to_owned());
            line = format!("//{}", &line[idx..]);
        }
        result.push(line);
    }
    result
}

fn normalize_line_comment_prefix(line: &str) -> String {
    if line.starts_with("//noinspection") || line.starts_with("//$NON-NLS-") {
        return line.to_owned();
    }

    let bytes = line.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'/' || bytes[1] != b'/' {
        return line.to_owned();
    }

    let mut slash_count = 0;
    for byte in bytes {
        if *byte == b'/' {
            slash_count += 1;
        } else {
            break;
        }
    }
    if slash_count >= bytes.len() {
        return line.to_owned();
    }
    if bytes[slash_count].is_ascii_whitespace() || bytes[slash_count] == b'/' {
        return line.to_owned();
    }

    format!("{} {}", "/".repeat(slash_count), &line[slash_count..])
}

fn indent_line_comments(lines: Vec<String>, column0: usize) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut result = vec![lines[0].trim().to_owned()];
    let indent = " ".repeat(column0);
    for line in lines.into_iter().skip(1) {
        result.push(format!("{indent}{}", line.trim()));
    }
    result
}

fn strip_trailing_whitespace(line: &str) -> String {
    line.trim_end().to_owned()
}

fn javadoc_shaped(lines: &[String]) -> bool {
    let Some(first) = lines.first() else {
        return false;
    };
    let first = first.trim();
    if first.starts_with("/**") {
        return true;
    }
    if !first.starts_with("/*") {
        return false;
    }

    lines[1..].iter().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty() || trimmed.starts_with('*')
    })
}

fn indent_javadoc_lines(lines: Vec<String>, column0: usize) -> Vec<String> {
    let Some(first) = lines.first() else {
        return lines;
    };

    let mut result = vec![first.trim().to_owned()];
    let indent = " ".repeat(column0 + 1);
    for line in lines.into_iter().skip(1) {
        let trimmed = line.trim();
        let mut formatted = indent.clone();
        if !trimmed.starts_with('*') {
            formatted.push_str("* ");
        }
        formatted.push_str(trimmed);
        result.push(formatted);
    }
    result
}

fn preserve_comment_indentation(lines: Vec<String>, column0: usize) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let start_col = lines[1..]
        .iter()
        .filter_map(|line| line.find(|ch: char| !ch.is_whitespace()))
        .min()
        .unwrap_or(0);
    let column_prefix = " ".repeat(column0);
    let mut result = vec![lines[0].clone()];
    for line in lines.into_iter().skip(1) {
        if line.trim().is_empty() {
            result.push(String::new());
            continue;
        }
        let mut formatted = column_prefix.clone();
        if line.len() >= start_col {
            formatted.push_str(&line[start_col..]);
        } else {
            formatted.push_str(&line);
        }
        result.push(formatted);
    }
    result
}

fn raw_comment_lines(raw: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = raw.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        let end = match ch {
            '\r' => {
                let mut end = index + ch.len_utf8();
                if let Some((next_index, '\n')) = chars.peek().copied() {
                    chars.next();
                    end = next_index + '\n'.len_utf8();
                }
                end
            }
            '\n' | '\u{2028}' | '\u{2029}' => index + ch.len_utf8(),
            _ => continue,
        };

        lines.push(&raw[start..index]);
        start = end;
    }

    lines.push(&raw[start..]);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_comment_rewrites_to_spaced_form() {
        assert_eq!(
            reformat_parameter_comment("/*a=*/"),
            Some("/* a= */".to_owned())
        );
        assert_eq!(
            reformat_parameter_comment("/*xs...=*/"),
            Some("/* xs...= */".to_owned())
        );
        assert_eq!(reformat_parameter_comment("/* b */"), None);
    }

    #[test]
    fn line_comment_prefix_normalizes_missing_space() {
        assert_eq!(normalize_line_comment_prefix("//foo"), "// foo");
        assert_eq!(
            normalize_line_comment_prefix("//noinspection X"),
            "//noinspection X"
        );
    }
}
