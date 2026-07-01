use jolt_fmt_ir::{Doc, concat, group, literal_text, text};
use jolt_java_syntax::{
    ArgumentList, ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer,
    AssignmentExpression, BinaryExpression, CastExpression, ConditionalExpression, DimExpression,
    Expression, FieldAccessExpression, InstanceofExpression, LambdaExpression, LambdaParameter,
    MethodInvocationExpression, ObjectCreationExpression, ParenthesizedExpression,
    PostfixExpression, UnaryExpression, VariableInitializerValue,
};

use crate::rules::statements::format_block;

pub(crate) fn format_expression(expression: &Expression) -> Doc {
    match expression {
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(expression)
        }
        Expression::AssignmentExpression(expression) => format_assignment_expression(expression),
        Expression::ConditionalExpression(expression) => format_conditional_expression(expression),
        Expression::BinaryExpression(expression) => format_binary_expression(expression),
        Expression::UnaryExpression(expression) => format_unary_expression(expression),
        Expression::PostfixExpression(expression) => format_postfix_expression(expression),
        Expression::LambdaExpression(expression) => format_lambda_expression(expression),
        Expression::LiteralExpression(_)
        | Expression::NameExpression(_)
        | Expression::ThisExpression(_)
        | Expression::SuperExpression(_)
        | Expression::ClassLiteralExpression(_)
        | Expression::MethodReferenceExpression(_)
        | Expression::SwitchExpression(_) => source_doc(&expression.source_text()),
        Expression::ArrayCreationExpression(expression) => {
            format_array_creation_expression(expression)
        }
        Expression::InstanceofExpression(expression) => format_instanceof_expression(expression),
        Expression::CastExpression(expression) => format_cast_expression(expression),
        Expression::FieldAccessExpression(expression) => format_field_access_expression(expression),
        Expression::ArrayAccessExpression(expression) => format_array_access_expression(expression),
        Expression::MethodInvocationExpression(expression) => {
            format_method_invocation_expression(expression)
        }
        Expression::ObjectCreationExpression(expression) => {
            format_object_creation_expression(expression)
        }
    }
}

pub(crate) fn expression_source_text(expression: &Expression) -> String {
    expression.source_text().trim().to_owned()
}

fn format_parenthesized_expression(expression: &ParenthesizedExpression) -> Doc {
    concat([
        text("("),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(")"),
    ])
}

fn format_assignment_expression(expression: &AssignmentExpression) -> Doc {
    group(concat([
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left)),
        text(" "),
        text(
            expression
                .operator()
                .map_or_else(String::new, |operator| operator.text().to_owned()),
        ),
        text(" "),
        expression
            .right()
            .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
    ]))
}

fn format_conditional_expression(expression: &ConditionalExpression) -> Doc {
    group(concat([
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        text(" ? "),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(" : "),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    ]))
}

fn format_binary_expression(expression: &BinaryExpression) -> Doc {
    group(concat([
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left)),
        text(" "),
        text(
            expression
                .operator()
                .map_or_else(String::new, |operator| operator.text().to_owned()),
        ),
        text(" "),
        expression
            .right()
            .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
    ]))
}

fn format_unary_expression(expression: &UnaryExpression) -> Doc {
    concat([
        text(
            expression
                .operator()
                .map_or_else(String::new, |operator| operator.text().to_owned()),
        ),
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| format_expression(&operand)),
    ])
}

fn format_postfix_expression(expression: &PostfixExpression) -> Doc {
    concat([
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| format_expression(&operand)),
        text(
            expression
                .operator()
                .map_or_else(String::new, |operator| operator.text().to_owned()),
        ),
    ])
}

fn format_method_invocation_expression(expression: &MethodInvocationExpression) -> Doc {
    group(concat([
        format_method_invocation_callee(expression),
        format_argument_list(expression.arguments()),
    ]))
}

fn format_field_access_expression(expression: &FieldAccessExpression) -> Doc {
    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| format_expression(&receiver)),
        text("."),
        text(
            expression
                .field_name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        ),
    ]))
}

fn format_array_access_expression(expression: &ArrayAccessExpression) -> Doc {
    group(concat([
        expression
            .array()
            .map_or_else(jolt_fmt_ir::nil, |array| format_expression(&array)),
        text("["),
        expression
            .index()
            .map_or_else(jolt_fmt_ir::nil, |index| format_expression(&index)),
        text("]"),
    ]))
}

fn format_object_creation_expression(expression: &ObjectCreationExpression) -> Doc {
    group(concat([
        expression
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([format_expression(&qualifier), text(".")])
            }),
        text("new "),
        expression.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            text(ty.source_text().trim().to_owned())
        }),
        format_argument_list(expression.arguments()),
        expression.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([text(" "), source_doc(&body.source_text())])
        }),
    ]))
}

fn format_array_creation_expression(expression: &ArrayCreationExpression) -> Doc {
    group(concat([
        text("new "),
        expression.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            text(ty.source_text().trim().to_owned())
        }),
        concat(
            expression
                .dimensions()
                .map(|dimension| format_dim_expression(&dimension)),
        ),
        expression
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                concat([text(" "), format_array_initializer(&initializer)])
            }),
    ]))
}

fn format_dim_expression(dimension: &DimExpression) -> Doc {
    concat([
        text("["),
        dimension
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text("]"),
    ])
}

fn format_array_initializer(initializer: &ArrayInitializer) -> Doc {
    let values = initializer
        .values()
        .map(format_variable_initializer_value)
        .collect::<Vec<_>>();

    if values.is_empty() {
        return text("{}");
    }

    concat([text("{"), jolt_fmt_ir::join(text(", "), values), text("}")])
}

fn format_variable_initializer_value(value: VariableInitializerValue) -> Doc {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::NameExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ThisExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::SuperExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ParenthesizedExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ClassLiteralExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::FieldAccessExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayAccessExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::MethodInvocationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::MethodReferenceExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ObjectCreationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayCreationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::AssignmentExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ConditionalExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::InstanceofExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::BinaryExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::UnaryExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::PostfixExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::CastExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::LambdaExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::SwitchExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(&initializer)
        }
    }
}

fn format_cast_expression(expression: &CastExpression) -> Doc {
    concat([
        text("("),
        expression.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            text(ty.source_text().trim().to_owned())
        }),
        text(") "),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    ])
}

fn format_instanceof_expression(expression: &InstanceofExpression) -> Doc {
    concat([
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(" instanceof "),
        expression.ty().map_or_else(
            || {
                expression
                    .pattern()
                    .map_or_else(jolt_fmt_ir::nil, |pattern| {
                        text(pattern.source_text().trim().to_owned())
                    })
            },
            |ty| text(ty.source_text().trim().to_owned()),
        ),
    ])
}

fn format_method_invocation_callee(expression: &MethodInvocationExpression) -> Doc {
    if let Some(name) = expression.direct_method_name() {
        return concat([
            expression
                .qualifier()
                .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                    concat([format_expression(&qualifier), text(".")])
                }),
            expression
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    text(arguments.source_text().trim().to_owned())
                }),
            text(name.text().to_owned()),
        ]);
    }

    expression
        .simple_name_expression()
        .map_or_else(jolt_fmt_ir::nil, |name| format_expression(&name))
}

fn format_argument_list(arguments: Option<ArgumentList>) -> Doc {
    let Some(arguments) = arguments else {
        return text("()");
    };
    let arguments = arguments
        .arguments()
        .map(|argument| format_expression(&argument))
        .collect::<Vec<_>>();

    if arguments.is_empty() {
        return text("()");
    }

    concat([
        text("("),
        jolt_fmt_ir::join(text(", "), arguments),
        text(")"),
    ])
}

fn format_lambda_expression(expression: &LambdaExpression) -> Doc {
    concat([
        format_lambda_parameters(expression),
        text(" -> "),
        expression.expression_body().map_or_else(
            || {
                expression
                    .block_body()
                    .map_or_else(jolt_fmt_ir::nil, |block| format_block(&block))
            },
            |body| format_expression(&body),
        ),
    ])
}

fn format_lambda_parameters(expression: &LambdaExpression) -> Doc {
    if let Some(parameter) = expression.concise_parameter()
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        return text(
            parameter
                .name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        );
    }

    let parameters = expression
        .parameters()
        .map(|parameters| parameters.parameters().collect::<Vec<_>>())
        .unwrap_or_default();

    if let [parameter] = parameters.as_slice()
        && is_simple_untyped_lambda_parameter(parameter)
    {
        return text(
            parameter
                .name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        );
    }

    concat([
        text("("),
        jolt_fmt_ir::join(
            text(", "),
            parameters
                .into_iter()
                .map(|parameter| text(parameter.source_text().trim().to_owned())),
        ),
        text(")"),
    ])
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter) -> bool {
    parameter.ty().is_none()
        && parameter
            .name()
            .is_some_and(|name| parameter.source_text().trim() == name.text())
}

fn source_doc(source: &str) -> Doc {
    literal_text(source.trim().to_owned())
}
