//! Binary operator chains.

use jolt_fmt::Doc;
use jolt_java_syntax::{BinaryExpression, Expression, JavaSyntaxKind as K, binary_precedence};
use jolt_syntax::{AstNode, SyntaxToken};

use crate::JavaFormatter;

fn operator_kind(node: BinaryExpression<'_>) -> Option<K> {
    node.operator().map(|token| K::from_raw(token.kind()))
}

/// Whether a left operand with operator `child` joins the chain of `parent`:
/// same precedence, except that shifts and mixed or `%` multiplicative
/// operators do not flatten.
fn flattens(parent: K, child: K) -> bool {
    let shift = |kind| matches!(kind, K::LShift | K::RShift | K::UnsignedRShift);
    let multiplicative = |kind| matches!(kind, K::Star | K::Slash | K::Percent);
    if binary_precedence(parent).is_none() || binary_precedence(parent) != binary_precedence(child)
    {
        return false;
    }
    if shift(parent) && shift(child) {
        return false;
    }
    if multiplicative(parent) && multiplicative(child) {
        return parent == child && parent != K::Percent;
    }
    true
}

/// A left-nested chain of one precedence level, walked iteratively: flat if
/// it fits, else broken before every operator one indent deeper.
pub(crate) fn binary_expression<'a>(f: &mut JavaFormatter<'a>, node: BinaryExpression<'a>) -> Doc {
    // Spine from the outermost node inward.
    let mut spine = vec![node];
    let mut head = node.left();
    while let (Some(Expression::BinaryExpression(left)), Some(parent)) = (
        head,
        operator_kind(*spine.last().expect("spine is not empty")),
    ) {
        if left.syntax().is_malformed()
            || !operator_kind(left).is_some_and(|child| flattens(parent, child))
        {
            break;
        }
        spine.push(left);
        head = left.left();
    }
    let mut leading = Vec::new();
    for &interior in &spine[1..] {
        leading.push(f.leading_comments(interior.syntax()));
    }
    let readability = operator_kind(node).is_some_and(|kind| {
        matches!(
            kind,
            K::Amp | K::Bar | K::Caret | K::LShift | K::RShift | K::UnsignedRShift
        )
    });
    let head = operand(f, head, readability);
    let mut rest = Vec::with_capacity(spine.len() * 4);
    for (depth, &binary) in spine.iter().enumerate().rev() {
        let operator: Option<SyntaxToken<'a>> = binary.operator();
        rest.push(f.line());
        rest.push(f.token_opt(operator));
        rest.push(f.space());
        rest.push(operand(f, binary.right(), readability));
        if depth > 0 {
            rest.push(f.trailing_comments(binary.syntax()));
        }
    }
    let rest = f.concat(rest);
    let rest = f.indent(rest);
    let leading = f.concat(leading);
    let chain = jolt_fmt::cat![f; leading, head, rest];
    f.group(chain)
}

/// An operand, wrapped in synthesized readability parentheses (N7) when it is
/// itself a binary expression under a bitwise or shift operator.
fn operand<'a>(
    f: &mut JavaFormatter<'a>,
    operand: Option<Expression<'a>>,
    readability: bool,
) -> Doc {
    let wrap = readability && matches!(operand, Some(Expression::BinaryExpression(_)));
    let doc = f.fmt_opt(operand);
    if !wrap {
        return doc;
    }
    let inner = jolt_fmt::indent![f; f.soft_line(), doc];
    let (open, close) = (f.text("("), f.text(")"));
    jolt_fmt::group![f; open, inner, f.soft_line(), close]
}
