use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression, JavaFormatter,
    PostfixExpression, UnaryExpression, concat, format_expression, format_token_with_comments,
    text,
};
use crate::helpers::comments::{comment_forces_line, format_comment, token_has_comments};
use jolt_fmt_ir::{force_group, group, indent, line};
use jolt_java_syntax::{ExpressionParentRole, JavaOperator};

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

fn flatten_binary_expression<'source>(
    expression: &'source BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> (Expression<'source>, Vec<(Doc, Doc)>) {
    let Some(operator) = expression.operator() else {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(*expression)),
            expression
                .right()
                .map(|right| (jolt_fmt_ir::nil(), format_expression(&right, formatter)))
                .into_iter()
                .collect(),
        );
    };
    let root = Expression::from(*expression);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(root, &mut operands, &mut operators);
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

fn unflattened_binary_expression<'source>(
    expression: &'source BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
    operator: &JavaOperator<'source>,
) -> (Expression<'source>, Vec<(Doc, Doc)>) {
    (
        expression
            .left()
            .unwrap_or_else(|| Expression::from(*expression)),
        vec![(
            format_operator_with_comments(operator),
            expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                format_expression(&right, formatter)
            }),
        )],
    )
}

fn collect_binary_chain<'source>(
    expression: Expression<'source>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression);
        return;
    };

    if let Some(left) = binary.left() {
        collect_binary_left(left, operator.text(), operands, operators);
    }
    operators.push(operator);
    if let Some(right) = binary.right() {
        operands.push(right);
    }
}

fn collect_binary_left<'source>(
    expression: Expression<'source>,
    parent_operator: &str,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression);
        return;
    };

    if !should_flatten_binary(parent_operator, operator.text()) {
        operands.push(expression);
        return;
    }

    collect_binary_chain(Expression::from(binary), operands, operators);
}

fn binary_for_chain(expression: Expression<'_>) -> Option<BinaryExpression<'_>> {
    match expression {
        Expression::BinaryExpression(binary) => Some(binary),
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

fn assignment_expression(left: Doc, operator: Doc, right: Doc) -> Doc {
    group(concat([left, text(" "), operator, assignment_rhs(right)]))
}

fn assignment_rhs(right: Doc) -> Doc {
    indent(concat([line(), right]))
}

fn binary_chain(first: Doc, rest: Vec<(Doc, Doc)>) -> Doc {
    if rest.is_empty() {
        return first;
    }

    group(concat([
        first,
        indent(concat(rest.into_iter().map(|(operator, operand)| {
            concat([line(), operator, text(" "), operand])
        }))),
    ]))
}

fn ternary_expression(
    condition: Doc,
    question: Doc,
    consequence: Doc,
    colon: Doc,
    alternative: Doc,
    force_break: bool,
) -> Doc {
    let doc = concat([
        condition,
        indent(concat([
            line(),
            question,
            text(" "),
            consequence,
            line(),
            colon,
            text(" "),
            alternative,
        ])),
    ]);

    if force_break {
        force_group(doc)
    } else {
        group(doc)
    }
}

fn should_force_conditional_break(expression: &ConditionalExpression) -> bool {
    matches!(
        Expression::from(*expression).parent_role(),
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
