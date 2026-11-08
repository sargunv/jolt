//! The value after an operator: `left = right`, and a switch rule's
//! `-> value;`.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::JavaSyntaxKind as K;
use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxToken};

use crate::JavaFormatter;

/// `left space`, then `op` and `right`, the formatted `value` node, as
/// `operator_value` lays them out.
pub(crate) fn assignment_like<'a>(
    f: &mut JavaFormatter<'a>,
    left: Doc,
    op: Option<SyntaxToken<'a>>,
    value: Option<SyntaxNode<'a>>,
    right: Doc,
) -> Doc {
    let right = operator_value(f, op, value, right);
    cat![f; left, f.space(), right]
}

/// `op`, then a space and `doc`, the formatted `value` node with what follows
/// it on its line. The value hangs (see `DocBuilder::hang`): it moves to the
/// next line one indent deeper if it fits there, and otherwise starts on the
/// operator's line. An operator chain or a conditional that doesn't fit
/// always starts the next line, so that its operands start one level deeper
/// than the operator's line. So does every value after a trailing comment of
/// `op` that ends its line (`x = // c`), as after a keyword (see
/// `after_token`).
pub(crate) fn operator_value<'a>(
    f: &mut JavaFormatter<'a>,
    op: Option<SyntaxToken<'a>>,
    value: Option<SyntaxNode<'a>>,
    doc: Doc,
) -> Doc {
    let op_breaks = op.is_some_and(|op| f.trailing_comments_break(SyntaxElement::Token(op)));
    let chain = value.is_some_and(|value| {
        matches!(
            K::from_raw(value.kind()),
            K::BinaryExpression | K::InstanceofExpression | K::ConditionalExpression
        )
    });
    let op = f.token_opt(op);
    let value = if op_breaks || chain {
        let value = indent![f; f.line(), doc];
        f.group(value)
    } else {
        let line = f.line();
        f.hang(line, doc)
    };
    cat![f; op, value]
}
