use super::{
    CastExpression, Doc, InlineLeadingTrivia, InstanceofExpression, JavaSyntaxToken, LeadingTrivia,
    TrailingTrivia, format_expression, format_pattern, format_token, format_token_with_comments,
    format_token_with_inline_leading_comments, format_type, trailing_comments_force_line,
};
use crate::helpers::recovery::{JavaFormatField, format_required_field, resolve_required_field};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{Pattern, Type};

pub(super) fn format_cast_expression<'source>(
    expression: &CastExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (open_paren, open_recovery) = match resolve_required_field(expression.open_paren(), doc) {
        JavaFormatField::Present(token) => (Some(token), Doc::nil()),
        JavaFormatField::Malformed(recovery) => (None, recovery),
    };
    let (close_paren, close_recovery) = match resolve_required_field(expression.close_paren(), doc)
    {
        JavaFormatField::Present(token) => (Some(token), Doc::nil()),
        JavaFormatField::Malformed(recovery) => (None, recovery),
    };
    let ty = format_required_field(expression.r#type(), doc, |ty, doc| format_type(&ty, doc));
    let expression_doc = format_required_field(expression.expression(), doc, |expression, doc| {
        format_expression(&expression, doc)
    });

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                open_recovery,
                format_cast_open_paren(open_paren.as_ref(), doc),
                ty,
                format_cast_close_paren(close_paren.as_ref(), doc),
                close_recovery,
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
    let expression_doc = format_required_field(expression.expression(), doc, |expression, doc| {
        format_expression(&expression, doc)
    });
    let operator = format_required_field(expression.instanceof_keyword(), doc, |token, doc| {
        format_instanceof_operator(&token, doc)
    });
    let rhs = format_required_field(expression.target(), doc, |target, doc| {
        if let Some(ty) = target.cast_family::<Type<'source>>() {
            format_type(&ty, doc)
        } else if let Some(pattern) = target.cast_family::<Pattern<'source>>() {
            format_pattern(&pattern, doc)
        } else {
            doc.block_on_invariant("instanceof target was neither a type nor a pattern");
            Doc::nil()
        }
    });

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
