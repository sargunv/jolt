//! Modifier lists (declaration layout, without N2 sorting yet) and the inline
//! parameter-modifier layout.

use jolt_fmt::Doc;
use jolt_java_syntax::{
    AnnotationList, ModifierElement, ModifierList, ModifierPartition, ParameterModifier,
    ParameterModifierList,
};

use crate::JavaFormatter;

/// Declaration annotations one per line, then keywords and type-use
/// annotations separated by spaces, followed by a space. NIL when empty.
pub(crate) fn modifier_list<'a>(f: &mut JavaFormatter<'a>, node: ModifierList<'a>) -> Doc {
    let mut docs = Vec::new();
    for (element, partition) in node.partitioned() {
        docs.push(match element {
            ModifierElement::Annotation(annotation) => f.fmt(annotation),
            ModifierElement::Bogus(bogus) => f.fmt(bogus),
            ModifierElement::NonSealedModifier(modifier) => f.fmt(modifier),
            ModifierElement::Token(token) => f.token(token),
        });
        docs.push(separator(f, partition));
    }
    f.concat(docs)
}

/// The declaration layout of `final` and annotations (local variables,
/// enhanced-for variables, resources).
pub(crate) fn parameter_modifier_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: ParameterModifierList<'a>,
) -> Doc {
    let mut docs = Vec::new();
    for (element, partition) in node.partitioned() {
        docs.push(parameter_modifier(f, element));
        docs.push(separator(f, partition));
    }
    f.concat(docs)
}

/// The inline layout: every annotation and keyword on one line, separated by
/// spaces, followed by a space; NIL when empty.
pub(crate) fn inline_modifiers<'a>(
    f: &mut JavaFormatter<'a>,
    node: ParameterModifierList<'a>,
) -> Doc {
    let mut docs = Vec::new();
    for element in node.items() {
        docs.push(parameter_modifier(f, element));
        docs.push(f.space());
    }
    f.concat(docs)
}

fn parameter_modifier<'a>(f: &mut JavaFormatter<'a>, element: ParameterModifier<'a>) -> Doc {
    match element {
        ParameterModifier::Annotation(annotation) => f.fmt(annotation),
        ParameterModifier::Bogus(bogus) => f.fmt(bogus),
        ParameterModifier::Token(token) => f.token(token),
    }
}

fn separator(f: &JavaFormatter<'_>, partition: ModifierPartition) -> Doc {
    match partition {
        ModifierPartition::DeclarationAnnotation => f.hard_line(),
        ModifierPartition::TypeUseAnnotation | ModifierPartition::Modifier => f.space(),
    }
}

/// Type-use annotations inline, each followed by a space; NIL when empty.
pub(crate) fn annotation_list<'a>(f: &mut JavaFormatter<'a>, node: AnnotationList<'a>) -> Doc {
    let mut docs = Vec::new();
    for annotation in node.items() {
        docs.push(f.fmt(annotation));
        docs.push(f.space());
    }
    f.concat(docs)
}
