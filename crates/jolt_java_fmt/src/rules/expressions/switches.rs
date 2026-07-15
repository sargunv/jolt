use super::{
    Doc, SwitchExpression, format_expression, format_switch_block, format_token_with_comments,
};
use crate::helpers::recovery::format_required_field;
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_switch_expression<'source>(
    expression: &SwitchExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(expression.switch_keyword(), doc, |token, doc| doc_concat!(
                doc,
                [format_token_with_comments(doc, &token), doc.space()]
            )),
            format_required_field(expression.open_paren(), doc, |token, doc| {
                format_token_with_comments(doc, &token)
            }),
            format_required_field(expression.selector(), doc, |selector, doc| {
                format_expression(&selector, doc)
            }),
            format_required_field(expression.close_paren(), doc, |token, doc| doc_concat!(
                doc,
                [format_token_with_comments(doc, &token), doc.space()]
            )),
            format_required_field(expression.body(), doc, |block, doc| {
                format_switch_block(&block, doc)
            }),
        ]
    )
}
