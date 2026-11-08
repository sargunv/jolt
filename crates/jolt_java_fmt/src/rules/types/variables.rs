//! Formal and receiver parameters, and variable declarators.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    AnnotationList, FormalParameter, ParameterModifierList, ReceiverParameter, RecordComponent,
    Type, VariableDeclarator, VariableDeclaratorList, VariableInitializer,
};
use jolt_syntax::{AstNode, ListElement, SyntaxToken};

use super::modifiers::inline_modifiers;
use crate::JavaFormatter;
use crate::helpers::assignment::assignment_like;
use crate::helpers::lists::list_item;
use crate::helpers::spaced;

pub(crate) fn formal_parameter<'a>(f: &mut JavaFormatter<'a>, node: FormalParameter<'a>) -> Doc {
    let dimensions = f.fmt_opt(node.dimensions());
    parameter(
        f,
        node.modifiers(),
        node.r#type(),
        node.varargs_annotations(),
        node.ellipsis(),
        node.name(),
        dimensions,
    )
}

/// One component, laid out like a formal parameter.
pub(crate) fn record_component<'a>(f: &mut JavaFormatter<'a>, node: RecordComponent<'a>) -> Doc {
    parameter(
        f,
        node.modifiers(),
        node.r#type(),
        node.varargs_annotations(),
        node.ellipsis(),
        node.name(),
        Doc::NIL,
    )
}

/// `modifiers Type @A... name dimensions`, modifiers inline. The annotations
/// before `...` print with a space before them.
fn parameter<'a>(
    f: &mut JavaFormatter<'a>,
    modifiers: Option<ParameterModifierList<'a>>,
    ty: Option<Type<'a>>,
    varargs_annotations: Option<AnnotationList<'a>>,
    ellipsis: Option<SyntaxToken<'a>>,
    name: Option<SyntaxToken<'a>>,
    dimensions: Doc,
) -> Doc {
    let modifiers = inline_modifiers(f, modifiers);
    let ty = f.fmt_opt(ty);
    let varargs_annotations = f.fmt_opt(varargs_annotations);
    let varargs_annotations = spaced(f, varargs_annotations);
    let ellipsis = f.token_opt(ellipsis);
    let name = f.token_opt(name);
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
/// its own line one indent deeper. With several declarators the whole list
/// is one indent deeper, so a broken value of the first declarator is
/// indented past the continuation declarators.
pub(crate) fn variable_declarator_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: VariableDeclaratorList<'a>,
) -> Doc {
    let mut docs = Vec::new();
    // Something besides one declarator: a second one, or a comma.
    let mut several = false;
    for element in node
        .elements()
        .filter(|element| !matches!(element, ListElement::Missing))
    {
        several |= !docs.is_empty() || matches!(element, ListElement::Separator(_));
        if let ListElement::Separator(comma) = element {
            docs.extend([f.token(comma), f.line()]);
        } else {
            docs.push(list_item(f, element));
        }
    }
    let list = f.concat(docs);
    if !several {
        return list;
    }
    let list = f.indent(list);
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
    let initializer = node.initializer();
    let value = initializer
        .and_then(VariableInitializer::value)
        .map(AstNode::syntax);
    let right = f.fmt_opt(initializer);
    assignment_like(f, left, node.assign(), value, right)
}

pub(crate) fn variable_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: VariableInitializer<'a>,
) -> Doc {
    f.fmt_opt(node.value())
}
