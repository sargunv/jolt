//! Lambda expressions and their parameters.

use jolt_fmt::{Doc, cat, group, indent};
use jolt_java_syntax::{
    LambdaExpression, LambdaModifier, LambdaModifierList, LambdaParameter, LambdaParameterList,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited, joined};

/// `parameters -> body`. Parameters that do not fit break as a list.
pub(crate) fn lambda_expression<'a>(f: &mut JavaFormatter<'a>, node: LambdaExpression<'a>) -> Doc {
    let parameters = match single_untyped_parameter(f, node) {
        Some(parameter) => {
            // N6: `(x) ->` prints `x ->`.
            let open = removed(f, node.open_paren());
            let parameter = f.fmt(parameter);
            let close = removed(f, node.close_paren());
            cat![f; open, parameter, close]
        }
        None if node.open_paren().is_none() && node.close_paren().is_none() => {
            let parameters = node
                .parameters()
                .into_iter()
                .flat_map(LambdaParameterList::elements);
            joined(f, parameters, |_| Doc::NIL)
        }
        None if is_empty(f, node) => {
            // The reference breaks even an empty `()` when the line does not
            // fit: `(` / `) -> body`.
            let open = f.token_opt(node.open_paren());
            let inner = indent![f; f.soft_line()];
            let close = f.token_opt(node.close_paren());
            group![f; open, inner, f.soft_line(), close]
        }
        None => delimited(
            f,
            node.syntax(),
            node.open_paren(),
            node.parameters()
                .into_iter()
                .flat_map(LambdaParameterList::elements),
            node.close_paren(),
            ListLayout {
                padded: false,
                trailing_comma: TrailingComma::Never,
            },
        ),
    };
    let arrow = f.token_opt(node.arrow());
    let body = f.fmt_opt(node.body());
    cat![f; parameters, f.space(), arrow, f.space(), body]
}

/// `()` with nothing inside, not even comments.
fn is_empty<'a>(f: &JavaFormatter<'a>, node: LambdaExpression<'a>) -> bool {
    !f.has_dangling_comments(node.syntax())
        && node
            .parameters()
            .into_iter()
            .flat_map(LambdaParameterList::elements)
            .all(|element| matches!(element, ListElement::Missing))
}

fn removed<'a>(f: &mut JavaFormatter<'a>, token: Option<SyntaxToken<'a>>) -> Doc {
    token.map_or(Doc::NIL, |token| f.removed(SyntaxElement::Token(token)))
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
        && [open, close].into_iter().all(|token| {
            let token = SyntaxElement::Token(token);
            !f.has_leading_comments(token) && !f.has_trailing_comments(token)
        });
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
        docs.push(match modifier {
            LambdaModifier::Annotation(annotation) => f.fmt(annotation),
            LambdaModifier::Bogus(bogus) => f.fmt(bogus),
            LambdaModifier::Token(token) => f.token(token),
        });
        docs.push(f.space());
    }
    let ty = node.r#type();
    docs.push(f.fmt_opt(ty));
    if let Some(annotations) = node.varargs_annotations()
        && annotations.elements().next().is_some()
    {
        docs.push(f.space());
        docs.push(f.fmt(annotations));
    }
    let ellipsis = node.ellipsis();
    docs.push(f.token_opt(ellipsis));
    if ty.is_some() || ellipsis.is_some() {
        docs.push(f.space());
    }
    docs.push(f.token_opt(node.name()));
    docs.push(f.fmt_opt(node.dimensions()));
    f.concat(docs)
}
