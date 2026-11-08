//! Assignments, conditionals, unary operators, casts and parentheses.

use jolt_fmt::{Doc, cat, group};
use jolt_java_syntax::{
    AssignmentExpression, CastExpression, ConditionalExpression, JavaSyntaxKind as K,
    ParenthesizedExpression, UnaryExpression,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::assignment::assignment_like;
use crate::helpers::lists::{enclosed, parenthesized};

pub(crate) fn assignment_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: AssignmentExpression<'a>,
) -> Doc {
    let left = f.fmt_opt(node.left());
    let value = node.right().map(AstNode::syntax);
    let right = f.fmt_opt(node.right());
    assignment_like(f, left, node.operator(), value, right)
}

/// Flat if it fits; otherwise `?` and `:` each start a line one indent deeper
/// than the condition. A conditional directly inside another always breaks.
pub(crate) fn conditional_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConditionalExpression<'a>,
) -> Doc {
    let nested = node
        .syntax()
        .parent()
        .is_some_and(|parent| K::from_raw(parent.kind()) == K::ConditionalExpression);
    let force = if nested { f.break_parent() } else { Doc::NIL };
    let condition = f.fmt_opt(node.condition());
    let mut tail = Vec::with_capacity(8);
    // A branch missing with its operator (recovery) gets no line.
    for (operator, operand) in [
        (node.question(), node.then_expression()),
        (node.colon(), node.else_expression()),
    ] {
        let operator = f.token_opt(operator);
        let operand = f.fmt_opt(operand);
        if !operator.is_nil() || !operand.is_nil() {
            tail.extend([f.line(), operator, f.space(), operand]);
        }
    }
    let tail = f.concat(tail);
    let tail = f.indent(tail);
    group![f; force, condition, tail]
}

/// The operator attached to its operand (the printer separates the two where
/// they would lex as one token, as in `- -x`).
pub(crate) fn unary_expression<'a>(f: &mut JavaFormatter<'a>, node: UnaryExpression<'a>) -> Doc {
    let operator = f.token_opt(node.operator());
    let operand = f.fmt_opt(node.operand());
    cat![f; operator, operand]
}

/// `(Type) value`, the value hanging after the type (see
/// `DocBuilder::hang`).
pub(crate) fn cast_expression<'a>(f: &mut JavaFormatter<'a>, node: CastExpression<'a>) -> Doc {
    let ty = f.fmt_opt(node.r#type());
    let parens = (node.open_paren(), node.close_paren());
    let parenthesized = enclosed(f, node.syntax(), parens, ty, false);
    let expression = f.fmt_opt(node.expression());
    let line = f.line();
    let operand = f.hang(line, expression);
    cat![f; parenthesized, operand]
}

/// `(`, the expression one indent deeper, `)`: breaks like a call.
pub(crate) fn parenthesized_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ParenthesizedExpression<'a>,
) -> Doc {
    let expression = f.fmt_opt(node.expression());
    parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        expression,
        node.close_paren(),
    )
}
