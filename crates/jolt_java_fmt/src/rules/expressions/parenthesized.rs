use super::{
    Doc, LeadingTrivia, ParenthesizedExpression, TrailingTrivia, comment_forces_line,
    format_expression, format_token, format_token_with_comments,
    format_trailing_comments_before_line_break,
};
use crate::helpers::recovery::{JavaFormatField, format_required_field, resolve_required_field};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::JavaSyntaxToken;

pub(super) fn format_parenthesized_expression<'source>(
    expression: &ParenthesizedExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (open, open_recovery) = match resolve_required_field(expression.open_paren(), doc) {
        JavaFormatField::Present(token) => (Some(token), Doc::nil()),
        JavaFormatField::Malformed(recovery) => (None, recovery),
    };
    let (close, close_recovery) = match resolve_required_field(expression.close_paren(), doc) {
        JavaFormatField::Present(token) => (Some(token), Doc::nil()),
        JavaFormatField::Malformed(recovery) => (None, recovery),
    };
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                open_recovery,
                format_parenthesized_expression_open(open.as_ref(), doc),
                doc_indent!(
                    doc,
                    doc_concat!(
                        doc,
                        [
                            format_open_parenthesized_expression_spacing(open.as_ref(), doc),
                            format_required_field(
                                expression.expression(),
                                doc,
                                |expression, doc| { format_expression(&expression, doc) }
                            ),
                        ]
                    )
                ),
                format_parenthesized_expression_close_with_spacing(close.as_ref(), doc),
                close_recovery,
            ]
        )
    )
}

fn format_parenthesized_expression_open<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    open.map_or_else(Doc::nil, |open| {
        format_token(
            doc,
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_open_parenthesized_expression_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open else {
        return doc.soft_line();
    };

    if open.trailing_comments().is_empty() {
        return doc.soft_line();
    }

    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, open),
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
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.line()
            } else {
                doc.soft_line()
            },
            close.map_or_else(Doc::nil, |close| format_token_with_comments(doc, close)),
        ]
    )
}
