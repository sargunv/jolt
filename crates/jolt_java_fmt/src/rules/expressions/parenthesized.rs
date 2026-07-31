use super::{
    Doc, InlineLeadingTrivia, LeadingTrivia, ParenthesizedExpression, TrailingTrivia,
    comment_forces_line, format_expression, format_token,
    format_token_with_inline_leading_comments, format_trailing_comments_before_line_break,
};
use crate::helpers::comments::format_line_start_construct;
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
                            // The inner expression begins a line of its own
                            // whenever a leading comment forces the group to
                            // break, matching the open paren's trailing run.
                            format_required_field(expression.expression(), doc, |inner, doc| {
                                format_line_start_construct(doc, inner.first_token(), |doc| {
                                    format_expression(&inner, doc)
                                })
                            }),
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
    doc_concat!(
        doc,
        [
            // The close paren is glued to the expression before it, so its
            // leading comments take the previous token's trailing form.
            doc.soft_line(),
            close.map_or_else(Doc::nil, |close| {
                format_token_with_inline_leading_comments(
                    doc,
                    close,
                    InlineLeadingTrivia::AfterPreviousToken,
                    TrailingTrivia::Preserve,
                )
            }),
        ]
    )
}
