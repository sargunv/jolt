//! Binary operator and `instanceof` chains.

use jolt_fmt::{Doc, cat, group, indent};
use jolt_java_syntax::{
    BinaryExpression, Expression, InstanceofExpression, JavaSyntaxKind as K, binary_precedence,
};
use jolt_syntax::{AstNode, SyntaxNode};

use crate::JavaFormatter;

pub(crate) fn binary_expression<'a>(f: &mut JavaFormatter<'a>, node: BinaryExpression<'a>) -> Doc {
    chain(f, Operation::Binary(node))
}

pub(crate) fn instanceof_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: InstanceofExpression<'a>,
) -> Doc {
    chain(f, Operation::Instanceof(node))
}

/// A node of the left operand spine.
#[derive(Clone, Copy)]
enum Operation<'a> {
    Binary(BinaryExpression<'a>),
    Instanceof(InstanceofExpression<'a>),
}

impl<'a> Operation<'a> {
    fn syntax(self) -> SyntaxNode<'a> {
        match self {
            Self::Binary(node) => node.syntax(),
            Self::Instanceof(node) => node.syntax(),
        }
    }

    fn left(self) -> Option<Expression<'a>> {
        match self {
            Self::Binary(node) => node.left(),
            Self::Instanceof(node) => node.expression(),
        }
    }
}

fn operation(expression: Expression<'_>) -> Option<Operation<'_>> {
    match expression {
        Expression::BinaryExpression(node) if !node.syntax().is_malformed() => {
            Some(Operation::Binary(node))
        }
        Expression::InstanceofExpression(node) if !node.syntax().is_malformed() => {
            Some(Operation::Instanceof(node))
        }
        _ => None,
    }
}

fn operator_kind(node: BinaryExpression<'_>) -> Option<K> {
    node.operator().map(|token| K::from_raw(token.kind()))
}

/// Whether a left operand with operator `child` joins the chain of `parent`:
/// same precedence, except that shifts and mixed or `%` multiplicative
/// operators do not flatten. A missing operator joins nothing.
fn flattens(parent: Option<K>, child: Option<K>) -> bool {
    let (Some(parent), Some(child)) = (parent, child) else {
        return false;
    };
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

/// N7: operands of bitwise and shift operators that are themselves binary
/// expressions get readability parentheses.
fn needs_readability_parentheses(parent: BinaryExpression<'_>) -> bool {
    parent.syntax().is_recovery_free()
        && operator_kind(parent).is_some_and(|kind| {
            matches!(
                kind,
                K::Amp | K::Bar | K::Caret | K::LShift | K::RShift | K::UnsignedRShift
            )
        })
}

/// N6: a parenthesized left operand of a same-precedence chain whose
/// parentheses carry no comments is transparent.
fn transparent_left<'a>(
    f: &JavaFormatter<'a>,
    parent: Operation<'a>,
    left: Expression<'a>,
) -> Option<BinaryExpression<'a>> {
    let (Operation::Binary(parent), Expression::ParenthesizedExpression(parenthesized)) =
        (parent, left)
    else {
        return None;
    };
    if !parent.syntax().is_recovery_free() || f.has_comments_within(parenthesized.syntax()) {
        return None;
    }
    let Some(Expression::BinaryExpression(inner)) = parenthesized.expression() else {
        return None;
    };
    flattens(operator_kind(parent), operator_kind(inner)).then_some(inner)
}

/// A chain under construction: the first operand and `operator operand`
/// pairs of one flattened precedence level.
struct Run {
    kind: Option<K>,
    first: Doc,
    rest: Vec<Doc>,
}

fn finish(f: &mut JavaFormatter<'_>, run: Run) -> Doc {
    let rest = f.concat(run.rest);
    let rest = f.indent(rest);
    group![f; run.first, rest]
}

/// The left operand spine walked iteratively from the innermost operand
/// outward. Each run of left-nested operators that flatten into one chain is
/// flat if it fits, else broken before every operator one indent deeper; an
/// `instanceof` breaks before its keyword. A run ends where the next operator
/// does not flatten, and becomes the first operand of what follows.
fn chain<'a>(f: &mut JavaFormatter<'a>, outer: Operation<'a>) -> Doc {
    // The spine from the outermost node inward, through the parentheses
    // N6 drops.
    let mut spine = vec![outer];
    let mut head = outer.left();
    while let Some(left) = head {
        let parent = *spine.last().expect("spine is not empty");
        let next = match transparent_left(f, parent, left) {
            Some(inner) => Operation::Binary(inner),
            None => match operation(left) {
                Some(next) => next,
                None => break,
            },
        };
        spine.push(next);
        head = next.left();
    }
    let leading: Vec<Doc> = spine[1..]
        .iter()
        .map(|operation| f.leading_comments(operation.syntax()))
        .collect();
    let head_doc = f.fmt_opt(head);
    let leading = f.concat(leading);
    let mut formatted = cat![f; leading, head_doc];
    // The expression `formatted` stands for, for N7.
    let mut formatted_is_binary = matches!(head, Some(Expression::BinaryExpression(_)));
    let mut run: Option<Run> = None;
    // The trailing comments of the node just finished. They follow its last
    // operand, outside any readability parentheses around the node, so the
    // next pass (which sees a parenthesized expression) prints them alike.
    let mut trailing = Doc::NIL;
    for (depth, &operation) in spine.iter().enumerate().rev() {
        match operation {
            Operation::Binary(node) => {
                let kind = operator_kind(node);
                let readability = needs_readability_parentheses(node);
                let mut current = match run.take() {
                    Some(mut run) if flattens(kind, run.kind) => {
                        run.rest.push(trailing);
                        run
                    }
                    finished => {
                        if let Some(finished) = finished {
                            formatted = finish(f, finished);
                        }
                        let first = if readability && formatted_is_binary {
                            readability_parentheses(f, formatted)
                        } else {
                            formatted
                        };
                        Run {
                            kind,
                            first: cat![f; first, trailing],
                            rest: Vec::new(),
                        }
                    }
                };
                let operator = f.token_opt(node.operator());
                let operand = match node.right() {
                    Some(Expression::BinaryExpression(right))
                        if readability && !right.syntax().is_malformed() =>
                    {
                        let leading = f.leading_comments(right.syntax());
                        let body = chain(f, Operation::Binary(right));
                        let body = readability_parentheses(f, body);
                        let trailing = f.trailing_comments(right.syntax());
                        cat![f; leading, body, trailing]
                    }
                    Some(Expression::BinaryExpression(right)) if readability => {
                        let body = f.fmt(right);
                        readability_parentheses(f, body)
                    }
                    right => f.fmt_opt(right),
                };
                current.kind = kind;
                current
                    .rest
                    .extend([f.line(), operator, f.space(), operand]);
                run = Some(current);
                formatted = Doc::NIL;
                formatted_is_binary = true;
            }
            Operation::Instanceof(node) => {
                if let Some(run) = run.take() {
                    formatted = finish(f, run);
                }
                let keyword = f.token_opt(node.instanceof_keyword());
                let target = f.fmt_opt(node.target());
                let tail = indent![f; f.line(), keyword, f.space(), target];
                formatted = group![f; formatted, trailing, tail];
                formatted_is_binary = false;
            }
        }
        trailing = if depth > 0 {
            f.trailing_comments(operation.syntax())
        } else {
            Doc::NIL
        };
    }
    match run {
        Some(run) => finish(f, run),
        None => formatted,
    }
}

/// Synthesized readability parentheses (N7), which break like a
/// parenthesized expression.
fn readability_parentheses(f: &mut JavaFormatter<'_>, doc: Doc) -> Doc {
    let inner = indent![f; f.soft_line(), doc];
    let (open, close) = (f.text("("), f.text(")"));
    group![f; open, inner, f.soft_line(), close]
}
