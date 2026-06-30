use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity, TextRange};
use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, hard_line, join, line_suffix, line_suffix_boundary, text,
};

use crate::context::{JavaCommentTrivia, JavaFormatContext};
use crate::diagnostics::{FormatResult, JavaFormatDiagnosticCode};
use crate::helpers::comments::{CommentPlacement, rewrite_comment_lines};

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
    Ok(context
        .take_leading_comments(code_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect())
}

pub(crate) fn take_leading_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
    code_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    Ok(context
        .take_leading_comments_in_range(owner_range, code_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect())
}

pub(crate) fn take_dangling_comment_docs(
    context: &mut JavaFormatContext<'_>,
    container_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    Ok(context
        .take_dangling_comments(container_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect())
}

pub(crate) fn take_own_line_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    Ok(context
        .take_own_line_comments_in_range(owner_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect())
}

pub(crate) fn take_inline_leading_block_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_inline_leading_block_comments(code_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_inline_leading_block_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_inline_leading_block_comments_in_range(owner_range, code_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_adjacent_leading_javadoc_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_adjacent_leading_javadoc_comments_in_range(owner_range, code_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect()
}

pub(crate) fn take_inline_trailing_block_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_inline_trailing_block_comments(code_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_trailing_line_comment_docs_in_range_as_own_line(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    boundary: TextRange,
) -> Vec<Doc> {
    context
        .take_trailing_line_comments_in_range(code_range, boundary)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect()
}

pub(crate) fn take_adjacent_trailing_block_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_adjacent_trailing_block_comments(code_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_block_comment_docs_in_range_as_inline(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
) -> Vec<Doc> {
    context
        .take_block_comments_in_range(owner_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_same_line_trailing_block_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    owner_range: TextRange,
) -> Vec<Doc> {
    context
        .take_same_line_trailing_block_comments_in_range(code_range, owner_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_same_line_separator_trailing_block_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    owner_range: TextRange,
) -> Vec<Doc> {
    context
        .take_same_line_separator_trailing_block_comments_in_range(code_range, owner_range)
        .into_iter()
        .map(|comment| format_inline_comment(context, &comment))
        .collect()
}

pub(crate) fn take_separator_leading_javadoc_comment_docs_in_range(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
    code_range: TextRange,
) -> Vec<Doc> {
    context
        .take_separator_leading_javadoc_comments_in_range(owner_range, code_range)
        .into_iter()
        .map(|comment| format_own_line_comment(context, &comment))
        .collect()
}

pub(crate) fn reject_unhandled_comments_before_start(
    context: &JavaFormatContext<'_>,
    boundary: TextRange,
    message: &'static str,
) -> FormatResult<()> {
    if let Some(comment) = context.unhandled_comment_trivia_before_start(boundary) {
        return Err(unhandled_comment_diagnostic(comment, message));
    }

    Ok(())
}

pub(crate) fn reject_unhandled_comments_before_end(
    context: &JavaFormatContext<'_>,
    boundary: TextRange,
    message: &'static str,
) -> FormatResult<()> {
    if let Some(comment) = context.unhandled_comment_trivia_before_end(boundary) {
        return Err(unhandled_comment_diagnostic(comment, message));
    }

    Ok(())
}

pub(crate) fn reject_unhandled_comments_in_range(
    context: &JavaFormatContext<'_>,
    boundary: TextRange,
    message: &'static str,
) -> FormatResult<()> {
    if let Some(comment) = context.unhandled_comment_trivia_in_range(boundary) {
        return Err(unhandled_comment_diagnostic(comment, message));
    }

    Ok(())
}

fn unhandled_comment_diagnostic(comment: &JavaCommentTrivia, message: &'static str) -> Diagnostic {
    Diagnostic {
        code: JavaFormatDiagnosticCode::InternalError.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Formatter,
        message: message.to_owned(),
        range: Some(comment.trivia.range),
    }
}

pub(crate) fn with_leading_and_trailing_comments(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    leading: Vec<Doc>,
    doc: Doc,
) -> FormatResult<Doc> {
    let trailing = context.take_trailing_line_comment(code_range);

    let doc = if let Some(comment) = trailing {
        let raw = context.raw_text(&comment);
        let trailing_doc = format_trailing_line_comment(context, &comment);
        if raw.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
            concat([doc, hard_line(), trailing_doc])
        } else {
            concat([
                doc,
                line_suffix(concat([text(" "), trailing_doc])),
                line_suffix_boundary(),
            ])
        }
    } else {
        doc
    };

    if leading.is_empty() {
        return Ok(doc);
    }

    Ok(concat([join(hard_line(), leading), hard_line(), doc]))
}

pub(crate) fn format_own_line_comment_doc(
    context: &JavaFormatContext<'_>,
    comment: &JavaCommentTrivia,
) -> Doc {
    format_own_line_comment(context, comment)
}

fn format_own_line_comment(context: &JavaFormatContext<'_>, comment: &JavaCommentTrivia) -> Doc {
    comment_lines_to_doc(
        rewrite_comment_lines(context, comment, CommentPlacement::OwnLine),
        CommentPlacement::OwnLine,
    )
}

fn format_inline_comment(context: &JavaFormatContext<'_>, comment: &JavaCommentTrivia) -> Doc {
    let raw = context.raw_text(comment);
    if raw.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        comment_lines_to_doc(
            rewrite_comment_lines(context, comment, CommentPlacement::InlineBlock),
            CommentPlacement::InlineBlock,
        )
    } else {
        let lines = rewrite_comment_lines(context, comment, CommentPlacement::InlineBlock);
        text(lines.into_iter().next().unwrap_or_default())
    }
}

fn format_trailing_line_comment(
    context: &JavaFormatContext<'_>,
    comment: &JavaCommentTrivia,
) -> Doc {
    comment_lines_to_doc(
        rewrite_comment_lines(context, comment, CommentPlacement::TrailingLine),
        CommentPlacement::TrailingLine,
    )
}

fn comment_lines_to_doc(lines: Vec<String>, placement: CommentPlacement) -> Doc {
    let dedent_continuations = matches!(placement, CommentPlacement::InlineBlock);
    let mut parts = Vec::new();
    for (index, line) in lines.into_iter().enumerate() {
        if index > 0 {
            if dedent_continuations || line.is_empty() {
                parts.push(break_(FlatLine::Empty, i16::MIN));
            } else {
                parts.push(hard_line());
            }
        }
        if !line.is_empty() {
            parts.push(text(line));
        }
    }
    if parts.is_empty() {
        return text("");
    }

    concat(parts)
}
