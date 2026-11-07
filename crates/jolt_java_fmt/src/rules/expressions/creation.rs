//! Object and array creation, dimension expressions, array initializers.

use jolt_fmt::{Doc, cat, group};
use jolt_java_syntax::{
    ArrayCreationExpression, ArrayInitializer, DimExpression, DimExpressionList,
    ObjectCreationExpression, VariableInitializerList,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::lists::enclosed;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited, list_item};

/// `new <T> Type(args) body`, without the qualifier and its `.`.
pub(super) fn object_creation_tail<'a>(
    f: &mut JavaFormatter<'a>,
    node: ObjectCreationExpression<'a>,
) -> Doc {
    let new = f.token_opt(node.new_keyword());
    let type_arguments = match node.constructor_type_arguments() {
        Some(arguments) => {
            let arguments = f.fmt(arguments);
            cat![f; arguments, f.space()]
        }
        None => Doc::NIL,
    };
    let ty = f.fmt_opt(node.r#type());
    let arguments = f.fmt_opt(node.arguments());
    let body = match node.body() {
        Some(body) => {
            let body = f.fmt(body);
            cat![f; f.space(), body]
        }
        None => Doc::NIL,
    };
    cat![f; new, f.space(), type_arguments, ty, arguments, body]
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
        .flat_map(DimExpressionList::elements)
        .map(|element| list_item(f, element))
        .collect::<Vec<_>>();
    let dimension_expressions = f.concat(dimension_expressions);
    let dimensions = f.fmt_opt(node.dimensions());
    let initializer = match node.initializer() {
        Some(initializer) => {
            let initializer = f.fmt(initializer);
            cat![f; f.space(), initializer]
        }
        None => Doc::NIL,
    };
    group![f; new, f.space(), ty, dimension_expressions, dimensions, initializer]
}

/// `[expression]`, annotated as ` @A [expression]`.
pub(crate) fn dim_expression<'a>(f: &mut JavaFormatter<'a>, node: DimExpression<'a>) -> Doc {
    let annotations = match node.annotations() {
        Some(annotations) if annotations.elements().next().is_some() => {
            let annotations = f.fmt(annotations);
            cat![f; f.space(), annotations]
        }
        annotations => f.fmt_opt(annotations),
    };
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
        ListLayout {
            padded: true,
            trailing_comma: TrailingComma::IfBroken,
        },
    )
}
