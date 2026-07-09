use super::{
    Doc, LeadingTrivia, ParenthesizedExpression, TrailingTrivia, comment_forces_line,
    format_expression, format_token, format_token_with_comments,
    format_trailing_comments_before_line_break,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_parenthesized_expression<'source>(
    expression: &ParenthesizedExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_parenthesized_expression_open(expression, doc),
                doc_indent!(
                    doc,
                    doc_concat!(
                        doc,
                        [
                            format_open_parenthesized_expression_spacing(expression, doc),
                            expression.expression().map_or_else(Doc::nil, |expression| {
                                format_expression(&expression, doc)
                            },),
                        ]
                    )
                ),
                format_parenthesized_expression_close_with_spacing(expression, doc),
            ]
        )
    )
}

fn format_parenthesized_expression_open<'source>(
    expression: &ParenthesizedExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    expression.open_paren().map_or_else(Doc::nil, |open| {
        format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_open_parenthesized_expression_spacing<'source>(
    expression: &ParenthesizedExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = expression.open_paren() else {
        return doc.soft_line();
    };

    if open.trailing_comments().is_empty() {
        return doc.soft_line();
    }

    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, &open),
            if open
                .trailing_comments()
                .any(|comment| comment_forces_line(&comment))
            {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

fn format_parenthesized_expression_close_with_spacing<'source>(
    expression: &ParenthesizedExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close_has_leading_comments = expression
        .close_paren()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.line()
            } else {
                doc.soft_line()
            },
            expression
                .close_paren()
                .map_or_else(Doc::nil, |close| format_token_with_comments(doc, &close),),
        ]
    )
}
