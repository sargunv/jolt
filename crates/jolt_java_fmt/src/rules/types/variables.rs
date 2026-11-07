//! Formal and receiver parameters, and variable declarators.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    FormalParameter, ReceiverParameter, VariableDeclarator, VariableDeclaratorList,
    VariableInitializer,
};
use jolt_syntax::ListElement;

use super::modifiers::inline_modifiers;
use crate::JavaFormatter;
use crate::helpers::assignment::assignment_like;

pub(crate) fn formal_parameter<'a>(f: &mut JavaFormatter<'a>, node: FormalParameter<'a>) -> Doc {
    let modifiers = match node.modifiers() {
        Some(modifiers) => inline_modifiers(f, modifiers),
        None => Doc::NIL,
    };
    let ty = f.fmt_opt(node.r#type());
    let varargs_annotations = f.fmt_opt(node.varargs_annotations());
    let varargs_annotations = if varargs_annotations.is_nil() {
        Doc::NIL
    } else {
        cat![f; f.space(), varargs_annotations]
    };
    let ellipsis = f.token_opt(node.ellipsis());
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; modifiers, ty, varargs_annotations, ellipsis, f.space(), name, dimensions]
}

/// `@A Type Outer.this`.
pub(crate) fn receiver_parameter<'a>(
    f: &mut JavaFormatter<'a>,
    node: ReceiverParameter<'a>,
) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let ty = f.fmt_opt(node.r#type());
    let qualifier = f.token_opt(node.qualifier());
    let dot = f.token_opt(node.dot());
    let this = f.token_opt(node.this_keyword());
    cat![f; annotations, ty, f.space(), qualifier, dot, this]
}

/// Declarators on one line if they fit, otherwise each after the first on
/// its own line one indent deeper.
pub(crate) fn variable_declarator_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: VariableDeclaratorList<'a>,
) -> Doc {
    let mut first = Doc::NIL;
    let mut rest = Vec::new();
    let mut seen_item = false;
    for element in node.elements() {
        let doc = match element {
            ListElement::Item(declarator) => f.fmt(declarator),
            ListElement::Bogus(bogus) => f.node(bogus),
            ListElement::Separator(comma) => {
                let comma = f.token(comma);
                rest.extend([comma, f.line()]);
                continue;
            }
            ListElement::Missing => continue,
        };
        if seen_item {
            rest.push(doc);
        } else {
            first = doc;
            seen_item = true;
        }
    }
    let rest = f.concat(rest);
    let rest = f.indent(rest);
    let list = cat![f; first, rest];
    f.group(list)
}

pub(crate) fn variable_declarator<'a>(
    f: &mut JavaFormatter<'a>,
    node: VariableDeclarator<'a>,
) -> Doc {
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    let left = cat![f; name, dimensions];
    if node.assign().is_none() && node.initializer().is_none() {
        return left;
    }
    let right = f.fmt_opt(node.initializer());
    assignment_like(f, left, node.assign(), right)
}

pub(crate) fn variable_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: VariableInitializer<'a>,
) -> Doc {
    f.fmt_opt(node.value())
}
