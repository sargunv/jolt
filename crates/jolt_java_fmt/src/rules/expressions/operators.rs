use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression, JavaFormatter,
    PostfixExpression, UnaryExpression, assignment_expression, binary_chain, concat,
    format_expression, format_token_with_comments, ternary_expression, text,
};
use crate::helpers::comments::{comment_forces_line, format_comment};
use jolt_java_syntax::{ExpressionParentRole, JavaOperator, JavaSyntaxToken};

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
                format_operator_with_comments(&operator)
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
        should_force_conditional_break(expression),
    )
}

pub(super) fn format_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let parent_operator = expression
        .operator()
        .map(|operator| operator.text().to_owned());
    let (first, rest) = flatten_binary_expression(expression, formatter);
    let first = parent_operator.as_deref().map_or_else(
        || format_expression(&first, formatter),
        |operator| format_binary_operand(&first, operator, formatter),
    );
    binary_chain(first, rest)
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
    let root = Expression::from(expression.clone());
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(&root, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(expression, formatter, &operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operators
        .into_iter()
        .zip(operands)
        .map(|(operator, operand)| {
            (
                format_operator_with_comments(&operator),
                format_binary_operand(&operand, operator.text(), formatter),
            )
        })
        .collect();

    (first, rest)
}

fn format_binary_operand(
    expression: &Expression,
    parent_operator: &str,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let doc = format_expression(expression, formatter);
    if should_parenthesize_binary_operand(expression, parent_operator) {
        jolt_fmt_ir::group(concat([
            text("("),
            jolt_fmt_ir::indent(concat([jolt_fmt_ir::soft_line(), doc])),
            jolt_fmt_ir::soft_line(),
            text(")"),
        ]))
    } else {
        doc
    }
}

fn unflattened_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
    operator: &JavaOperator,
) -> (Expression, Vec<(Doc, Doc)>) {
    (
        expression
            .left()
            .unwrap_or_else(|| Expression::from(expression.clone())),
        vec![(
            format_operator_with_comments(operator),
            expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                format_expression(&right, formatter)
            }),
        )],
    )
}

fn collect_binary_chain(
    expression: &Expression,
    operands: &mut Vec<Expression>,
    operators: &mut Vec<JavaOperator>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression.clone());
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression.clone());
        return;
    };

    if let Some(left) = binary.left() {
        collect_binary_left(&left, operator.text(), operands, operators);
    }
    operators.push(operator);
    if let Some(right) = binary.right() {
        operands.push(right);
    }
}

fn collect_binary_left(
    expression: &Expression,
    parent_operator: &str,
    operands: &mut Vec<Expression>,
    operators: &mut Vec<JavaOperator>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression.clone());
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression.clone());
        return;
    };

    if !should_flatten_binary(parent_operator, operator.text()) {
        operands.push(expression.clone());
        return;
    }

    collect_binary_chain(&Expression::from(binary), operands, operators);
}

fn binary_for_chain(expression: &Expression) -> Option<BinaryExpression> {
    match expression {
        Expression::BinaryExpression(binary) => Some(binary.clone()),
        Expression::ParenthesizedExpression(parenthesized)
            if parenthesized
                .open_paren()
                .is_none_or(|token| !token_has_comments(&token))
                && parenthesized
                    .close_paren()
                    .is_none_or(|token| !token_has_comments(&token)) =>
        {
            match parenthesized.expression() {
                Some(Expression::BinaryExpression(binary)) => Some(binary),
                _ => None,
            }
        }
        _ => None,
    }
}

fn token_has_comments(token: &JavaSyntaxToken) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

fn format_operator_with_comments(operator: &JavaOperator) -> Doc {
    if let Some(token) = operator.as_single_token() {
        return format_token_with_comments(token);
    }

    concat([
        format_operator_leading_comments(operator),
        text(operator.text()),
        format_operator_trailing_comments(operator),
    ])
}

fn format_operator_leading_comments(operator: &JavaOperator) -> Doc {
    let mut docs = Vec::new();
    for comment in operator.leading_comments() {
        docs.push(format_comment(&comment));
        docs.push(jolt_fmt_ir::hard_line());
    }
    concat(docs)
}

fn format_operator_trailing_comments(operator: &JavaOperator) -> Doc {
    let mut docs = Vec::new();
    for comment in operator.trailing_comments() {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(jolt_fmt_ir::hard_line());
        }
    }
    concat(docs)
}

fn should_force_conditional_break(expression: &ConditionalExpression) -> bool {
    matches!(
        Expression::from(expression.clone()).parent_role(),
        Some(
            ExpressionParentRole::ConditionalCondition
                | ExpressionParentRole::ConditionalTrueExpression
                | ExpressionParentRole::ConditionalFalseExpression
        )
    )
}

fn should_flatten_binary(parent_operator: &str, child_operator: &str) -> bool {
    let Some(parent_precedence) = binary_operator_precedence(parent_operator) else {
        return false;
    };
    let Some(child_precedence) = binary_operator_precedence(child_operator) else {
        return false;
    };
    if parent_precedence != child_precedence {
        return false;
    }

    if is_shift_operator(parent_operator) && is_shift_operator(child_operator) {
        return false;
    }

    if is_multiplicative_operator(parent_operator) && is_multiplicative_operator(child_operator) {
        return parent_operator == child_operator
            && parent_operator != "%"
            && child_operator != "%";
    }

    true
}

fn should_parenthesize_binary_operand(expression: &Expression, parent_operator: &str) -> bool {
    if !is_bitwise_or_shift_operator(parent_operator) {
        return false;
    }

    matches!(expression, Expression::BinaryExpression(_))
}

fn binary_operator_precedence(operator: &str) -> Option<u8> {
    Some(match operator {
        "||" => 1,
        "&&" => 2,
        "|" => 3,
        "^" => 4,
        "&" => 5,
        "==" | "!=" => 6,
        "<" | ">" | "<=" | ">=" => 7,
        "<<" | ">>" | ">>>" => 8,
        "+" | "-" => 9,
        "*" | "/" | "%" => 10,
        _ => return None,
    })
}

fn is_shift_operator(operator: &str) -> bool {
    matches!(operator, "<<" | ">>" | ">>>")
}

fn is_bitwise_or_shift_operator(operator: &str) -> bool {
    matches!(operator, "|" | "^" | "&" | "<<" | ">>" | ">>>")
}

fn is_multiplicative_operator(operator: &str) -> bool {
    matches!(operator, "*" | "/" | "%")
}
