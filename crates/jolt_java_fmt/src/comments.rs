use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{Doc, concat, hard_line, join, line_suffix, line_suffix_boundary, text};

use crate::context::{JavaCommentTrivia, JavaFormatContext};
use crate::diagnostics::{FormatResult, missing_layout};

pub(crate) fn with_attached_comments(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    doc: Doc,
) -> FormatResult<Doc> {
    let leading = take_leading_comment_docs(context, code_range)?;

    with_leading_and_trailing_comments(context, code_range, leading, doc)
}

pub(crate) fn take_leading_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    context
        .take_leading_comments(code_range)
        .map_err(|error| missing_layout(error.message, error.range))
        .map(|comments| {
            comments
                .into_iter()
                .map(|comment| format_own_line_comment(context, &comment))
                .collect()
        })
}

pub(crate) fn take_dangling_comment_docs(
    context: &mut JavaFormatContext<'_>,
    container_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    context
        .take_dangling_comments(container_range)
        .map_err(|error| missing_layout(error.message, error.range))
        .map(|comments| {
            comments
                .into_iter()
                .map(|comment| format_own_line_comment(context, &comment))
                .collect()
        })
}

pub(crate) fn take_inline_leading_block_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_inline_leading_block_comments(code_range)
        .into_iter()
        .map(|comment| text(context.raw_text(&comment)))
        .collect()
}

pub(crate) fn take_inline_trailing_block_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_inline_trailing_block_comments(code_range)
        .into_iter()
        .map(|comment| text(context.raw_text(&comment)))
        .collect()
}

pub(crate) fn reject_unhandled_comments_before_start(
    context: &JavaFormatContext<'_>,
    boundary: TextRange,
    message: &'static str,
) -> FormatResult<()> {
    context
        .reject_unhandled_comments_before_start(boundary, message)
        .map_err(|error| missing_layout(error.message, error.range))
}

pub(crate) fn reject_unhandled_comments_before_end(
    context: &JavaFormatContext<'_>,
    boundary: TextRange,
    message: &'static str,
) -> FormatResult<()> {
    context
        .reject_unhandled_comments_before_end(boundary, message)
        .map_err(|error| missing_layout(error.message, error.range))
}

pub(crate) fn with_leading_and_trailing_comments(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    leading: Vec<Doc>,
    doc: Doc,
) -> FormatResult<Doc> {
    let trailing = context
        .take_trailing_line_comment(code_range)
        .map_err(|error| missing_layout(error.message, error.range))?;

    let doc = if let Some(comment) = trailing {
        concat([
            doc,
            line_suffix(text(format!(" {}", context.raw_text(&comment)))),
            line_suffix_boundary(),
        ])
    } else {
        doc
    };

    if leading.is_empty() {
        return Ok(doc);
    }

    Ok(concat([join(hard_line(), leading), hard_line(), doc]))
}

fn format_own_line_comment(context: &JavaFormatContext<'_>, comment: &JavaCommentTrivia) -> Doc {
    join(
        hard_line(),
        comment_lines(context.raw_text(comment))
            .into_iter()
            .map(text),
    )
}

fn comment_lines(raw: &str) -> Vec<String> {
    let lines = raw_comment_lines(raw);
    if !is_conventional_multiline_star_comment(&lines) {
        return lines.into_iter().map(str::to_owned).collect();
    }

    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                return line.to_owned();
            }

            let trimmed = line.trim_start_matches([' ', '\t']);
            if trimmed.is_empty() {
                String::new()
            } else {
                format!(" {trimmed}")
            }
        })
        .collect()
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

fn is_conventional_multiline_star_comment(lines: &[&str]) -> bool {
    lines.len() > 1
        && lines[1..]
            .iter()
            .filter(|line| !line.trim().is_empty())
            .all(|line| line.trim_start_matches([' ', '\t']).starts_with('*'))
}
