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
    text(context.raw_text(comment))
}
