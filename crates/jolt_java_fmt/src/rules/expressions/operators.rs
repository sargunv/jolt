use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression, JavaFormatter,
    PostfixExpression, UnaryExpression, assignment_expression, binary_chain, concat,
    format_expression, format_token_with_comments, ternary_expression, text,
};
use jolt_java_syntax::JavaSyntaxToken;

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
        return unflattened_binary_expression(expression, formatter, operator);
    }

    let root = Expression::from(expression.clone());
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(&root, operator_text, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(expression, formatter, operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operators
        .into_iter()
        .zip(operands)
        .map(|(operator, operand)| {
            (
                format_token_with_comments(&operator),
                format_expression(&operand, formatter),
            )
        })
        .collect();

    (first, rest)
}

fn unflattened_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
    operator: JavaSyntaxToken,
) -> (Expression, Vec<(Doc, Doc)>) {
    (
        expression
            .left()
            .unwrap_or_else(|| Expression::from(expression.clone())),
        vec![(
            format_token_with_comments(&operator),
            expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                format_expression(&right, formatter)
            }),
        )],
    )
}

fn collect_binary_chain(
    expression: &Expression,
    operator: &str,
    operands: &mut Vec<Expression>,
    operators: &mut Vec<JavaSyntaxToken>,
) {
    if let Expression::BinaryExpression(binary) = expression
        && let Some(binary_operator) = binary.operator()
        && binary_operator.text() == operator
    {
        if let Some(left) = binary.left() {
            collect_binary_chain(&left, operator, operands, operators);
        }
        operators.push(binary_operator);
        if let Some(right) = binary.right() {
            collect_binary_chain(&right, operator, operands, operators);
        }
        return;
    }

    operands.push(expression.clone());
}

const fn is_flattenable_binary_operator(operator: &str) -> bool {
    // Same-operator chains parse left-associatively in Java, so flattening only
    // removes redundant CST nesting while preserving token order and grouping.
    matches!(
        operator.as_bytes(),
        b"||"
            | b"&&"
            | b"|"
            | b"^"
            | b"&"
            | b"=="
            | b"!="
            | b"<"
            | b">"
            | b"<="
            | b">="
            | b"<<"
            | b">>"
            | b">>>"
            | b"+"
            | b"-"
            | b"*"
            | b"/"
            | b"%"
    )
}
