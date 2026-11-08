//! Lambda expressions and their parameters.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    LambdaExpression, LambdaModifierList, LambdaParameter, LambdaParameterList,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited, joined};
use crate::helpers::spaced;

/// `parameters -> body`. Parameters that do not fit break as a list.
pub(crate) fn lambda_expression<'a>(f: &mut JavaFormatter<'a>, node: LambdaExpression<'a>) -> Doc {
    let parameters = match single_untyped_parameter(f, node) {
        // N6: `(x) ->` prints `x ->`.
        Some(parameter) => f.fmt(parameter),
        None if node.open_paren().is_none() && node.close_paren().is_none() => {
            let parameters = node
                .parameters()
                .into_iter()
                .flat_map(LambdaParameterList::elements);
            joined(f, parameters, Doc::NIL)
        }
        None => delimited(
            f,
            node.syntax(),
            node.open_paren(),
            node.parameters()
                .into_iter()
                .flat_map(LambdaParameterList::elements),
            node.close_paren(),
            ListLayout::Plain,
        ),
    };
    let arrow = f.token_opt(node.arrow());
    let body = f.fmt_opt(node.body());
    cat![f; parameters, f.space(), arrow, f.space(), body]
}

/// The parameter of `(x) ->` when N6 removes its parentheses: one parameter
/// without modifiers, type or dimensions, and no comments on or inside the
/// parentheses.
fn single_untyped_parameter<'a>(
    f: &JavaFormatter<'a>,
    node: LambdaExpression<'a>,
) -> Option<LambdaParameter<'a>> {
    let (open, close) = (node.open_paren()?, node.close_paren()?);
    if !node.syntax().is_recovery_free() || f.has_dangling_comments(node.syntax()) {
        return None;
    }
    let mut elements = node.parameters()?.elements();
    let (Some(ListElement::Item(parameter)), None) = (elements.next(), elements.next()) else {
        return None;
    };
    let untyped = parameter
        .modifiers()
        .is_none_or(|modifiers| modifiers.syntax().children().next().is_none())
        && parameter.r#type().is_none()
        && parameter.ellipsis().is_none()
        && parameter.dimensions().is_none();
    let quiet = !f.has_comments_within(parameter.syntax())
        && [open, close]
            .into_iter()
            .all(|token| !f.has_comments(SyntaxElement::Token(token)));
    (untyped && quiet).then_some(parameter)
}

/// `final @A Type... name[]`; modifiers and annotations stay inline.
pub(crate) fn lambda_parameter<'a>(f: &mut JavaFormatter<'a>, node: LambdaParameter<'a>) -> Doc {
    let mut docs = Vec::new();
    for modifier in node
        .modifiers()
        .into_iter()
        .flat_map(LambdaModifierList::items)
    {
        docs.extend([f.element(modifier.element()), f.space()]);
    }
    let ty = node.r#type();
    docs.push(f.fmt_opt(ty));
    let annotations = f.fmt_opt(node.varargs_annotations());
    docs.push(spaced(f, annotations));
    let ellipsis = node.ellipsis();
    docs.push(f.token_opt(ellipsis));
    if ty.is_some() || ellipsis.is_some() {
        docs.push(f.space());
    }
    docs.push(f.token_opt(node.name()));
    docs.push(f.fmt_opt(node.dimensions()));
    f.concat(docs)
}
