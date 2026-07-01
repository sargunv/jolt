use jolt_fmt_ir::{Doc, concat, group, text};
use jolt_java_syntax::{
    ArgumentList, ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer,
    AssignmentExpression, BinaryExpression, CastExpression, ConditionalExpression, DimExpression,
    Expression, FieldAccessExpression, InstanceofExpression, LambdaExpression, LambdaParameter,
    MethodInvocationExpression, ObjectCreationExpression, ParenthesizedExpression,
    PostfixExpression, SwitchExpression, TypeArgumentList, UnaryExpression,
    VariableInitializerValue,
};

use crate::helpers::chains::member_chain;
use crate::helpers::comments::{format_token_sequence, tokens_have_comments};
use crate::helpers::lists::{angle_bracket_list, braced_initializer_list, parenthesized_list};
use crate::helpers::operators::{assignment_expression, binary_chain, ternary_expression};
use crate::rules::statements::{format_block, format_switch_block};

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
        | Expression::MethodReferenceExpression(_) => format_token_sequence(&expression.tokens()),
        Expression::SwitchExpression(expression) => format_switch_expression(expression),
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
    assignment_expression(
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left)),
        expression
            .operator()
            .map_or_else(String::new, |operator| operator.text().to_owned()),
        expression
            .right()
            .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
    )
}

fn format_conditional_expression(expression: &ConditionalExpression) -> Doc {
    ternary_expression(
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    )
}

fn format_binary_expression(expression: &BinaryExpression) -> Doc {
    let (first, rest) = flatten_binary_expression(expression);
    binary_chain(format_expression(&first), rest)
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
    if let Some(chain) = collect_member_chain(&Expression::from(expression.clone())) {
        return format_member_chain(chain);
    }

    group(concat([
        format_method_invocation_callee(expression),
        format_argument_list(expression.arguments()),
    ]))
}

fn format_field_access_expression(expression: &FieldAccessExpression) -> Doc {
    if let Some(chain) = collect_member_chain(&Expression::from(expression.clone())) {
        return format_member_chain(chain);
    }

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
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
        format_argument_list(expression.arguments()),
        expression.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([text(" "), format_token_sequence(&body.tokens())])
        }),
    ]))
}

fn format_array_creation_expression(expression: &ArrayCreationExpression) -> Doc {
    group(concat([
        text("new "),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
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

    braced_initializer_list(values)
}

pub(crate) fn format_variable_initializer_value(value: VariableInitializerValue) -> Doc {
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
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
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
                        format_token_sequence(&pattern.tokens())
                    })
            },
            |ty| format_token_sequence(&ty.tokens()),
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
                    format_type_argument_list(&arguments)
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
    let tokens = arguments.tokens();
    if tokens_have_comments(&tokens) {
        return format_token_sequence(&tokens);
    }
    let arguments = arguments
        .arguments()
        .map(|argument| format_expression(&argument))
        .collect::<Vec<_>>();

    parenthesized_list(arguments)
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
                .map(|parameter| format_lambda_parameter(&parameter)),
        ),
        text(")"),
    ])
}

fn format_switch_expression(expression: &SwitchExpression) -> Doc {
    concat([
        text("switch ("),
        expression
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| format_expression(&selector)),
        text(") "),
        expression
            .block()
            .map_or_else(|| text("{}"), |block| format_switch_block(&block)),
    ])
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter) -> bool {
    parameter.ty().is_none() && parameter.var_token().is_none() && !parameter.is_variable_arity()
}

fn format_lambda_parameter(parameter: &LambdaParameter) -> Doc {
    let tokens = parameter.tokens();
    if tokens_have_comments(&tokens) {
        return format_token_sequence(&tokens);
    }

    let ty = parameter.ty();
    let var_token = parameter.var_token();
    let has_prefix = ty.is_some() || var_token.is_some();
    let ty = ty.map_or_else(
        || var_token.map_or_else(jolt_fmt_ir::nil, |token| text(token.text().to_owned())),
        |ty| format_token_sequence(&ty.tokens()),
    );
    let name = parameter
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned()));

    if !has_prefix {
        return name;
    }

    concat([
        ty,
        if parameter.is_variable_arity() {
            text("... ")
        } else {
            text(" ")
        },
        name,
    ])
}

struct MemberChainParts {
    root: Expression,
    suffixes: Vec<Doc>,
}

fn collect_member_chain(expression: &Expression) -> Option<MemberChainParts> {
    match expression {
        Expression::MethodInvocationExpression(invocation) => {
            let name = invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            let suffix = concat([
                text("."),
                invocation
                    .type_arguments()
                    .map_or_else(jolt_fmt_ir::nil, |arguments| {
                        format_type_argument_list(&arguments)
                    }),
                text(name.text().to_owned()),
                format_argument_list(invocation.arguments()),
            ]);
            Some(append_chain_suffix(qualifier, suffix))
        }
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            let name = access.field_name()?;
            Some(append_chain_suffix(
                receiver,
                concat([text("."), text(name.text().to_owned())]),
            ))
        }
        _ => None,
    }
}

fn append_chain_suffix(receiver: Expression, suffix: Doc) -> MemberChainParts {
    if let Some(mut chain) = collect_member_chain(&receiver) {
        chain.suffixes.push(suffix);
        return chain;
    }

    MemberChainParts {
        root: receiver,
        suffixes: vec![suffix],
    }
}

fn format_member_chain(chain: MemberChainParts) -> Doc {
    let keep_first_suffix_with_root = is_simple_member_chain_root(&chain.root);
    member_chain(
        format_expression(&chain.root),
        chain.suffixes,
        keep_first_suffix_with_root,
    )
}

const fn is_simple_member_chain_root(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_)
            | Expression::ThisExpression(_)
            | Expression::SuperExpression(_)
            | Expression::ClassLiteralExpression(_)
    )
}

fn flatten_binary_expression(expression: &BinaryExpression) -> (Expression, Vec<(String, Doc)>) {
    let operator = expression
        .operator()
        .map_or_else(String::new, |operator| operator.text().to_owned());
    if !is_flattenable_binary_operator(&operator) {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            vec![(
                operator,
                expression
                    .right()
                    .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
            )],
        );
    }

    let mut operands = Vec::new();
    collect_binary_operands(
        &Expression::from(expression.clone()),
        &operator,
        &mut operands,
    );
    let mut operands = operands.into_iter();
    let first = operands
        .next()
        .unwrap_or_else(|| Expression::from(expression.clone()));
    let rest = operands
        .map(|operand| (operator.clone(), format_expression(&operand)))
        .collect();

    (first, rest)
}

fn collect_binary_operands(
    expression: &Expression,
    operator: &str,
    operands: &mut Vec<Expression>,
) {
    if let Expression::BinaryExpression(binary) = expression
        && binary
            .operator()
            .is_some_and(|token| token.text() == operator)
    {
        if let Some(left) = binary.left() {
            collect_binary_operands(&left, operator, operands);
        }
        if let Some(right) = binary.right() {
            collect_binary_operands(&right, operator, operands);
        }
        return;
    }

    operands.push(expression.clone());
}

const fn is_flattenable_binary_operator(operator: &str) -> bool {
    matches!(operator.as_bytes(), b"&&" | b"||")
}

fn format_type_argument_list(arguments: &TypeArgumentList) -> Doc {
    let tokens = arguments.tokens();
    if tokens_have_comments(&tokens) {
        return format_token_sequence(&tokens);
    }

    let arguments = arguments
        .arguments()
        .map(|argument| format_token_sequence(&argument.tokens()))
        .collect::<Vec<_>>();

    angle_bracket_list(arguments)
}
