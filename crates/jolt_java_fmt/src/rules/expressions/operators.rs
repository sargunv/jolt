//! Assignments, conditionals, unary operators, casts and parentheses.

use jolt_fmt::{Doc, cat, group, indent};
use jolt_java_syntax::{
    AssignmentExpression, CastExpression, ConditionalExpression, JavaSyntaxKind as K,
    ParenthesizedExpression, UnaryExpression,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::assignment::assignment_like;
use crate::helpers::lists::enclosed;
use crate::helpers::tokens::needs_space;

pub(crate) fn assignment_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: AssignmentExpression<'a>,
) -> Doc {
    let left = f.fmt_opt(node.left());
    let right = f.fmt_opt(node.right());
    assignment_like(f, left, node.operator(), right)
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
    let question = f.token_opt(node.question());
    let then_expression = f.fmt_opt(node.then_expression());
    let colon = f.token_opt(node.colon());
    let else_expression = f.fmt_opt(node.else_expression());
    let tail = indent![
        f;
        f.line(),
        question,
        f.space(),
        then_expression,
        f.line(),
        colon,
        f.space(),
        else_expression,
    ];
    group![f; force, condition, tail]
}

/// The operator attached to its operand, with a space only where the two
/// would otherwise lex differently (`- -x`).
pub(crate) fn unary_expression<'a>(f: &mut JavaFormatter<'a>, node: UnaryExpression<'a>) -> Doc {
    let space = match (node.operator(), node.operand()) {
        (Some(operator), Some(operand))
            if operand
                .syntax()
                .first_token()
                .is_some_and(|first| needs_space(operator, first)) =>
        {
            f.space()
        }
        _ => Doc::NIL,
    };
    let operator = f.token_opt(node.operator());
    let operand = f.fmt_opt(node.operand());
    cat![f; operator, space, operand]
}

/// `(Type) value`; when it does not fit, the value continues on the next
/// line one indent deeper.
pub(crate) fn cast_expression<'a>(f: &mut JavaFormatter<'a>, node: CastExpression<'a>) -> Doc {
    let ty = f.fmt_opt(node.r#type());
    let parens = (node.open_paren(), node.close_paren());
    let parenthesized = enclosed(f, node.syntax(), parens, ty, false);
    let expression = f.fmt_opt(node.expression());
    let operand = indent![f; f.line(), expression];
    group![f; parenthesized, operand]
}

/// `(`, the expression one indent deeper, `)`: breaks like a call.
pub(crate) fn parenthesized_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ParenthesizedExpression<'a>,
) -> Doc {
    let open = f.token_opt(node.open_paren());
    let expression = f.fmt_opt(node.expression());
    let dangling = f.dangling(node.syntax());
    let inner = if dangling.is_nil() {
        expression
    } else {
        cat![f; expression, f.hard_line(), dangling]
    };
    let inner = indent![f; f.soft_line(), inner];
    let close = f.token_opt(node.close_paren());
    group![f; open, inner, f.soft_line(), close]
}
