//! Object and array creation, dimension expressions, array initializers.

use jolt_fmt::{Doc, cat, group};
use jolt_java_syntax::{
    ArrayCreationExpression, ArrayInitializer, DimExpression, ObjectCreationExpression,
    VariableInitializerList,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement};

use super::operators::enclosed;
use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};

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
    // Children, not items: a bogus item keeps its place.
    let mut dimension_expressions = Vec::new();
    if let Some(list) = node.dimension_expressions() {
        for child in list.syntax().children() {
            dimension_expressions.push(match child {
                SyntaxElement::Node(child) => f.node(child),
                SyntaxElement::Token(token) => f.token(token),
            });
        }
    }
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
        Some(annotations) if annotations.syntax().children().next().is_some() => {
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
/// a trailing comma. A trailing comma in the source forces the broken layout.
/// As in the reference, no comma is synthesized after a last element that
/// carries a trailing comment (`last // c` / `}`).
///
/// REQUEST(shared): this trailing-comma policy belongs in
/// `helpers::lists::delimited` (`TrailingComma::IfBroken`), so annotation
/// array initializers get it too.
pub(crate) fn array_initializer<'a>(f: &mut JavaFormatter<'a>, node: ArrayInitializer<'a>) -> Doc {
    let elements = || {
        node.values()
            .into_iter()
            .flat_map(VariableInitializerList::elements)
    };
    let last_commented = elements()
        .filter(|element| !matches!(element, ListElement::Missing))
        .last()
        .is_some_and(|element| match element {
            ListElement::Item(value) => {
                f.has_trailing_comments(SyntaxElement::Node(value.syntax()))
            }
            ListElement::Bogus(value) => f.has_trailing_comments(SyntaxElement::Node(value)),
            ListElement::Separator(_) | ListElement::Missing => false,
        });
    let trailing_comma = if last_commented {
        TrailingComma::Never
    } else {
        TrailingComma::IfBroken
    };
    delimited(
        f,
        node.syntax(),
        node.open_brace(),
        elements(),
        node.close_brace(),
        ListLayout {
            padded: true,
            trailing_comma,
        },
    )
}
