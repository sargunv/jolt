//! Object and array creation, dimension expressions, array initializers.

use jolt_fmt::{Doc, cat, group};
use jolt_java_syntax::{
    ArrayCreationExpression, ArrayInitializer, DimExpression, DimExpressionList,
    ObjectCreationExpression, VariableInitializerList,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited, enclosed, joined};
use crate::helpers::{spaced, spaced_node};

/// `new <T> Type(args) body`, without the qualifier and its `.`.
pub(super) fn object_creation_tail<'a>(
    f: &mut JavaFormatter<'a>,
    node: ObjectCreationExpression<'a>,
) -> Doc {
    let new = f.token_opt(node.new_keyword());
    let type_arguments = f.fmt_opt(node.constructor_type_arguments());
    let ty = f.fmt_opt(node.r#type());
    let arguments = f.fmt_opt(node.arguments());
    let body = spaced_node(f, node.body());
    // The two spaces collapse into one without type arguments.
    cat![f; new, f.space(), type_arguments, f.space(), ty, arguments, body]
}

/// `new int[size][]`, `new String[] { … }`.
pub(crate) fn array_creation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ArrayCreationExpression<'a>,
) -> Doc {
    let new = f.token_opt(node.new_keyword());
    let ty = f.fmt_opt(node.r#type());
    let dimension_expressions = node
        .dimension_expressions()
        .into_iter()
        .flat_map(DimExpressionList::elements);
    let dimension_expressions = joined(f, dimension_expressions, Doc::NIL);
    let dimensions = f.fmt_opt(node.dimensions());
    let initializer = spaced_node(f, node.initializer());
    group![f; new, f.space(), ty, dimension_expressions, dimensions, initializer]
}

/// `[expression]`, annotated as ` @A [expression]`.
pub(crate) fn dim_expression<'a>(f: &mut JavaFormatter<'a>, node: DimExpression<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let annotations = spaced(f, annotations);
    let expression = f.fmt_opt(node.expression());
    let brackets = (node.open_bracket(), node.close_bracket());
    let brackets = enclosed(f, node.syntax(), brackets, expression, true);
    let brackets = f.group(brackets);
    cat![f; annotations, brackets]
}

/// `{ a, b }` when flat, `{}` when empty; broken, one element per line with
/// a trailing comma (N8).
pub(crate) fn array_initializer<'a>(f: &mut JavaFormatter<'a>, node: ArrayInitializer<'a>) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_brace(),
        node.values()
            .into_iter()
            .flat_map(VariableInitializerList::elements),
        node.close_brace(),
        ListLayout::Initializer,
    )
}
