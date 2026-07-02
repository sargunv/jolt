use super::{
    Doc, JavaFormatter, SwitchExpression, concat, format_expression, format_switch_block, text,
};

pub(super) fn format_switch_expression(
    expression: &SwitchExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        text("switch ("),
        expression
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| {
                format_expression(&selector, formatter)
            }),
        text(") "),
        expression.block().map_or_else(
            || text("{}"),
            |block| format_switch_block(&block, formatter),
        ),
    ])
}
