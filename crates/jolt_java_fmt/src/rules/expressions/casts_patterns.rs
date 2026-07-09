use super::{
    CastExpression, Doc, InlineLeadingTrivia, InstanceofExpression, JavaSyntaxToken, LeadingTrivia,
    TrailingTrivia, format_expression, format_pattern, format_token, format_token_with_comments,
    format_token_with_inline_leading_comments, format_type, trailing_comments_force_line,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_cast_expression<'source>(
    expression: &CastExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open_paren = expression.open_paren();
    let close_paren = expression.close_paren();
    let ty = expression
        .ty()
        .map_or_else(Doc::nil, |ty| format_type(&ty, doc));
    let expression_doc = expression
        .expression()
        .map_or_else(Doc::nil, |expression| format_expression(&expression, doc));

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_cast_open_paren(open_paren.as_ref(), doc),
                ty,
                format_cast_close_paren(close_paren.as_ref(), doc),
                if close_paren
                    .as_ref()
                    .is_some_and(trailing_comments_force_line)
                {
                    Doc::nil()
                } else {
                    doc.space()
                },
                expression_doc,
            ]
        ),
    )
}

fn format_cast_open_paren<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    open.map_or_else(Doc::nil, |open| {
        format_token_with_inline_leading_comments(
            doc,
            open,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSpaceIfComments,
        )
    })
}

fn format_cast_close_paren<'source>(
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
                Doc::nil()
            },
            close.map_or_else(Doc::nil, |close| format_token_with_comments(doc, close)),
        ]
    )
}

pub(super) fn format_instanceof_expression<'source>(
    expression: &InstanceofExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let expression_doc = expression
        .expression()
        .map_or_else(Doc::nil, |expression| format_expression(&expression, doc));
    let operator = expression
        .instanceof_token()
        .map_or_else(Doc::nil, |token| format_instanceof_operator(&token, doc));
    let rhs = match expression.ty() {
        Some(ty) => format_type(&ty, doc),
        None => expression
            .pattern()
            .map_or_else(Doc::nil, |pattern| format_pattern(&pattern, doc)),
    };

    doc_concat!(doc, [expression_doc, doc.space(), operator, rhs])
}

fn format_instanceof_operator<'source>(
    operator: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_token(
                doc,
                operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if trailing_comments_force_line(operator) {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}
