use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression, JavaFormatter,
    PostfixExpression, UnaryExpression, concat, format_expression, format_token_with_comments,
};
use crate::helpers::comments::token_has_comments;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};
use jolt_fmt_ir::space;
use jolt_fmt_ir::{force_group, group, indent, line};
use jolt_java_syntax::{ExpressionParentRole, JavaOperator};

pub(super) fn format_assignment_expression<'source>(
    expression: &AssignmentExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(super) fn format_conditional_expression<'source>(
    expression: &ConditionalExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    ternary_expression(
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        expression
            .question_token()
            .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        expression
            .colon_token()
            .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        should_force_conditional_break(expression),
    )
}

pub(super) fn format_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(operator) = expression.operator() else {
        return format_binary_expression_without_operator(expression, formatter);
    };

    if expression.left().is_none() {
        return binary_chain(
            jolt_fmt_ir::nil(),
            vec![(
                format_operator_with_comments(&operator),
                expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                    format_expression(&right, formatter)
                }),
            )],
        );
    }

    let parent_operator = operator.text();
    let (first, rest) = flatten_binary_expression(expression, formatter);
    let first = format_binary_operand(&first, parent_operator, formatter);
    binary_chain(first, rest)
}

fn format_binary_expression_without_operator<'source>(
    expression: &BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let left = expression.left();
    let right = expression.right();
    if left.is_none() && right.is_none() {
        return format_token_sequence(expression.token_iter(), LeadingTrivia::Preserve);
    }

    concat([
        left.map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left, formatter)),
        right.map_or_else(jolt_fmt_ir::nil, |right| {
            format_expression(&right, formatter)
        }),
    ])
}

pub(super) fn format_unary_expression<'source>(
    expression: &UnaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(super) fn format_postfix_expression<'source>(
    expression: &PostfixExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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
    expression: &BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> (Expression<'source>, Vec<(Doc<'source>, Doc<'source>)>) {
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

fn format_binary_operand<'source>(
    expression: &Expression<'source>,
    parent_operator: &str,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let doc = format_expression(expression, formatter);
    if should_parenthesize_binary_operand(expression, parent_operator) {
        jolt_fmt_ir::group(concat([
            // Intentional synthesized token: readability parentheses preserve
            // the parsed precedence while making mixed binary precedence clear.
            inserted_syntax_token("(", FormatterInsertedToken::PrecedenceParenthesis),
            jolt_fmt_ir::indent(concat([jolt_fmt_ir::soft_line(), doc])),
            jolt_fmt_ir::soft_line(),
            // Intentional synthesized token: closes the formatter-owned
            // readability parenthesis above.
            inserted_syntax_token(")", FormatterInsertedToken::PrecedenceParenthesis),
        ]))
    } else {
        doc
    }
}

fn unflattened_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    formatter: &JavaFormatter<'_>,
    operator: &JavaOperator<'source>,
) -> (Expression<'source>, Vec<(Doc<'source>, Doc<'source>)>) {
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

fn format_operator_with_comments<'source>(operator: &JavaOperator<'source>) -> Doc<'source> {
    if let Some(token) = operator.as_single_token() {
        return format_token_with_comments(token);
    }

    let mut tokens = operator.tokens().enumerate().peekable();
    concat(std::iter::from_fn(|| {
        let (index, token) = tokens.next()?;
        let is_first = index == 0;
        let is_last = tokens.peek().is_none();
        Some(format_token(
            &token,
            if is_first {
                LeadingTrivia::Preserve
            } else {
                LeadingTrivia::SuppressAlreadyHandled
            },
            if is_last {
                TrailingTrivia::Preserve
            } else {
                TrailingTrivia::RelocatedToEnclosingContext
            },
        ))
    }))
}

fn assignment_expression<'source>(
    left: Doc<'source>,
    operator: Doc<'source>,
    right: Doc<'source>,
) -> Doc<'source> {
    group(concat([left, space(), operator, assignment_rhs(right)]))
}

fn assignment_rhs(right: Doc<'_>) -> Doc<'_> {
    indent(concat([line(), right]))
}

fn binary_chain<'source>(
    first: Doc<'source>,
    rest: Vec<(Doc<'source>, Doc<'source>)>,
) -> Doc<'source> {
    if rest.is_empty() {
        return first;
    }

    group(concat([
        first,
        indent(concat(rest.into_iter().map(|(operator, operand)| {
            concat([line(), operator, space(), operand])
        }))),
    ]))
}

fn ternary_expression<'source>(
    condition: Doc<'source>,
    question: Doc<'source>,
    consequence: Doc<'source>,
    colon: Doc<'source>,
    alternative: Doc<'source>,
    force_break: bool,
) -> Doc<'source> {
    let doc = concat([
        condition,
        indent(concat([
            line(),
            question,
            space(),
            consequence,
            line(),
            colon,
            space(),
            alternative,
        ])),
    ]);

    if force_break {
        force_group(doc)
    } else {
        group(doc)
    }
}

fn should_force_conditional_break(expression: &ConditionalExpression<'_>) -> bool {
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
