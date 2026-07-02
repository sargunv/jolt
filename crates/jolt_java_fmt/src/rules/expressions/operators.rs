use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression, JavaFormatter,
    PostfixExpression, UnaryExpression, assignment_expression, binary_chain, concat,
    format_expression, format_token_with_comments, ternary_expression, text, token_has_comments,
};

pub(super) fn format_assignment_expression(
    expression: &AssignmentExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    assignment_expression(
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left, formatter)),
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
            format_expression(&right, formatter)
        }),
    )
}

pub(super) fn format_conditional_expression(
    expression: &ConditionalExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    ternary_expression(
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        expression
            .question_token()
            .map_or_else(|| text("?"), |token| format_token_with_comments(&token)),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        expression
            .colon_token()
            .map_or_else(|| text(":"), |token| format_token_with_comments(&token)),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
    )
}

pub(super) fn format_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let (first, rest) = flatten_binary_expression(expression, formatter);
    binary_chain(format_expression(&first, formatter), rest)
}

pub(super) fn format_unary_expression(
    expression: &UnaryExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| {
                format_expression(&operand, formatter)
            }),
    ])
}

pub(super) fn format_postfix_expression(
    expression: &PostfixExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| {
                format_expression(&operand, formatter)
            }),
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
    ])
}

fn flatten_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
) -> (Expression, Vec<(Doc, Doc)>) {
    let Some(operator) = expression.operator() else {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            expression
                .right()
                .map(|right| (jolt_fmt_ir::nil(), format_expression(&right, formatter)))
                .into_iter()
                .collect(),
        );
    };
    let operator_text = operator.text();
    if !is_flattenable_binary_operator(operator_text) {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            vec![(
                format_token_with_comments(&operator),
                expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                    format_expression(&right, formatter)
                }),
            )],
        );
    }

    let mut operands = Vec::new();
    let root = Expression::from(expression.clone());
    if binary_operator_comments_in_tree(&root, operator_text) {
        return (
            expression.left().unwrap_or_else(|| root.clone()),
            expression
                .right()
                .map(|right| {
                    (
                        format_token_with_comments(&operator),
                        format_expression(&right, formatter),
                    )
                })
                .into_iter()
                .collect(),
        );
    }

    collect_binary_operands(&root, operator_text, &mut operands);
    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operands
        .map(|operand| {
            (
                format_token_with_comments(&operator),
                format_expression(&operand, formatter),
            )
        })
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

fn binary_operator_comments_in_tree(expression: &Expression, operator: &str) -> bool {
    if let Expression::BinaryExpression(binary) = expression
        && binary
            .operator()
            .is_some_and(|token| token.text() == operator)
    {
        if binary
            .operator()
            .is_some_and(|token| token_has_comments(&token))
        {
            return true;
        }
        return binary
            .left()
            .is_some_and(|left| binary_operator_comments_in_tree(&left, operator))
            || binary
                .right()
                .is_some_and(|right| binary_operator_comments_in_tree(&right, operator));
    }

    false
}

const fn is_flattenable_binary_operator(operator: &str) -> bool {
    matches!(operator.as_bytes(), b"&&" | b"||")
}
