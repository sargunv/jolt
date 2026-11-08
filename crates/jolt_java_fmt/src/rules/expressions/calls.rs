//! Invocation forms and argument lists. Qualified invocations under a call
//! are formatted by the receiver spine (`spine.rs`).

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    ArgumentList, ExpressionList, QualifiedInvocationName, QualifiedMethodInvocation,
    UnqualifiedInvocationName, UnqualifiedMethodInvocation,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited};

pub(crate) fn unqualified_method_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: UnqualifiedMethodInvocation<'a>,
) -> Doc {
    let type_arguments = f.fmt_opt(node.type_arguments());
    let name = f.element_opt(node.name().map(UnqualifiedInvocationName::element));
    let arguments = f.fmt_opt(node.arguments());
    cat![f; type_arguments, name, arguments]
}

/// `.name(…)` without the receiver: the segment of a member chain.
pub(super) fn qualified_invocation_segment<'a>(
    f: &mut JavaFormatter<'a>,
    node: QualifiedMethodInvocation<'a>,
) -> Doc {
    let dot = f.token_opt(node.dot());
    let type_arguments = f.fmt_opt(node.type_arguments());
    let name = f.element_opt(node.name().map(QualifiedInvocationName::element));
    let arguments = f.fmt_opt(node.arguments());
    cat![f; dot, type_arguments, name, arguments]
}

/// Reached only when no call rule owns the invocation (the receiver spine
/// formats every qualified invocation under a well-formed call).
pub(crate) fn qualified_method_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: QualifiedMethodInvocation<'a>,
) -> Doc {
    let receiver = f.fmt_opt(node.receiver());
    let segment = qualified_invocation_segment(f, node);
    cat![f; receiver, segment]
}

pub(crate) fn argument_list<'a>(f: &mut JavaFormatter<'a>, node: ArgumentList<'a>) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.arguments()
            .into_iter()
            .flat_map(ExpressionList::elements),
        node.close_paren(),
        ListLayout::Arguments,
    )
}
