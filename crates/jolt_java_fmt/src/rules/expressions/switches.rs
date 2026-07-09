use super::{
    Doc, SwitchExpression, format_expression, format_switch_block, format_token_with_comments,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_switch_expression<'source>(
    expression: &SwitchExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            expression
                .keyword()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [format_token_with_comments(doc, token), doc.space()]
                ),),
            expression
                .open_paren()
                .as_ref()
                .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
            expression
                .selector()
                .map_or_else(Doc::nil, |selector| format_expression(&selector, doc),),
            expression
                .close_paren()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [format_token_with_comments(doc, token), doc.space()]
                ),),
            expression
                .block()
                .map_or_else(Doc::nil, |block| format_switch_block(&block, doc)),
        ]
    )
}
