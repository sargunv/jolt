use super::{
    Doc, JavaFormatter, SwitchExpression, concat, format_expression, format_switch_block,
    format_token_with_comments,
};
use jolt_fmt_ir::space;

pub(super) fn format_switch_expression<'source>(
    expression: &SwitchExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        expression
            .keyword()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), space()])
            }),
        expression
            .open_paren()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        expression
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| {
                format_expression(&selector, formatter)
            }),
        expression
            .close_paren()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), space()])
            }),
        expression.block().map_or_else(jolt_fmt_ir::nil, |block| {
            format_switch_block(&block, formatter)
        }),
    ])
}
