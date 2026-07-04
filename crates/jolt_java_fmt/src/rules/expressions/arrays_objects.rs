use super::calls::format_argument_list;
use super::{
    ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer, CommaListItem, DimExpression,
    Doc, InlineLeadingTrivia, JavaFormatter, JavaSyntaxToken, ObjectCreationExpression,
    TrailingTrivia, VariableInitializerValue, braced_comma_list_with_trailing_separator,
    comment_forces_line, concat, format_anonymous_class_body, format_expression,
    format_leading_comments, format_token_with_inline_leading_comments,
    format_trailing_comments_before_line_break, format_type, format_type_argument_list, group,
    hard_line, indent, text, trailing_comments_force_line,
};

pub(super) fn format_array_access_expression<'source>(
    expression: &ArrayAccessExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open_bracket = expression.open_bracket();
    let close_bracket = expression.close_bracket();

    group(concat([
        expression.array().map_or_else(jolt_fmt_ir::nil, |array| {
            format_expression(&array, formatter)
        }),
        format_bracketed_expression(
            open_bracket.as_ref(),
            expression.index().map_or_else(jolt_fmt_ir::nil, |index| {
                format_expression(&index, formatter)
            }),
            close_bracket.as_ref(),
        ),
    ]))
}

pub(super) fn format_object_creation_expression<'source>(
    expression: &ObjectCreationExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        expression
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([format_expression(&qualifier, formatter), text(".")])
            }),
        format_creation_new_keyword(expression.new_token().as_ref()),
        expression
            .constructor_type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                concat([format_type_argument_list(&arguments, formatter), text(" ")])
            }),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_argument_list(expression.arguments(), formatter),
        expression.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([text(" "), format_anonymous_class_body(&body, formatter)])
        }),
    ]))
}

pub(super) fn format_array_creation_expression<'source>(
    expression: &ArrayCreationExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        format_creation_new_keyword(expression.new_token().as_ref()),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        concat(
            expression
                .dimensions()
                .map(|dimension| format_dim_expression(&dimension, formatter)),
        ),
        expression
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                concat([text(" "), format_array_initializer(&initializer, formatter)])
            }),
    ]))
}

fn format_creation_new_keyword<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    keyword.map_or_else(
        || text("new "),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                text("new"),
                format_trailing_comments_before_line_break(keyword),
                if trailing_comments_force_line(keyword) {
                    hard_line()
                } else {
                    text(" ")
                },
            ])
        },
    )
}

fn format_dim_expression<'source>(
    dimension: &DimExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open_bracket = dimension.open_bracket();
    let close_bracket = dimension.close_bracket();

    format_bracketed_expression(
        open_bracket.as_ref(),
        dimension
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        close_bracket.as_ref(),
    )
}

fn format_bracketed_expression<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    expression: Doc<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    group(concat([
        format_open_bracket(open),
        indent(concat([format_open_bracket_spacing(open), expression])),
        format_close_bracket_with_spacing(close),
    ]))
}

fn format_open_bracket<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    open.map_or_else(
        || text("["),
        |open| concat([format_leading_comments(open), text("[")]),
    )
}

fn format_open_bracket_spacing<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if open
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
        {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_close_bracket_with_spacing<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(
        || text("]"),
        |close| {
            format_token_with_inline_leading_comments(
                close,
                InlineLeadingTrivia::AfterPreviousToken,
                TrailingTrivia::Preserve,
            )
        },
    )
}

fn format_array_initializer<'source>(
    initializer: &ArrayInitializer<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    braced_comma_list_with_trailing_separator(
        open.as_ref(),
        close.as_ref(),
        initializer
            .entries()
            .map(|entry| CommaListItem {
                doc: format_variable_initializer_value(entry.value, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(crate) fn format_variable_initializer_value<'source>(
    value: VariableInitializerValue<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::TemplateExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::NameExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ThisExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::SuperExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ParenthesizedExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ClassLiteralExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::FieldAccessExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayAccessExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::MethodInvocationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::MethodReferenceExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ObjectCreationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayCreationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::AssignmentExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ConditionalExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::InstanceofExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::BinaryExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::UnaryExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::PostfixExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::CastExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::LambdaExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::SwitchExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(&initializer, formatter)
        }
    }
}
