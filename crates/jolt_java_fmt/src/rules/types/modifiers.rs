//! Modifier lists (declaration layout with N2 ordering), parameter modifier
//! lists (declaration and inline layouts), `non-sealed`, and type-use
//! annotation lists.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    AnnotationList, JavaSyntaxKind as K, ModifierElement, ModifierList, ModifierPartition,
    NonSealedModifier, ParameterModifier, ParameterModifierList, is_contextual,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;

/// Declaration annotations one per line, then keywords and type-use
/// annotations separated by spaces, followed by a space. NIL when empty.
///
/// When the list is recovery-free, keywords are reordered (N2), and the
/// annotations move out of the keywords: on a type, constructor or
/// annotation element declaration every annotation goes on its own line
/// before the keywords; on a field or method, type-use annotations (those
/// after a keyword) follow the keywords inline. A keyword carrying a comment,
/// or a bogus modifier, is a barrier: the keywords on each side sort
/// separately.
pub(crate) fn modifier_list<'a>(f: &mut JavaFormatter<'a>, node: ModifierList<'a>) -> Doc {
    if !node.syntax().is_recovery_free() {
        let mut docs = Vec::new();
        for (element, partition) in node.partitioned() {
            docs.push(modifier_element(f, element));
            docs.push(separator(f, partition));
        }
        return f.concat(docs);
    }
    let hoist_all = node
        .syntax()
        .parent()
        .is_some_and(|parent| hoists_annotations(K::from_raw(parent.kind())));
    let mut lines = Vec::new();
    let mut keywords = Vec::new();
    let mut type_use = Vec::new();
    for (element, partition) in node.partitioned() {
        match (element, partition) {
            (ModifierElement::Annotation(_), ModifierPartition::DeclarationAnnotation) => {
                lines.push(element);
            }
            (ModifierElement::Annotation(_), _) if hoist_all => lines.push(element),
            (ModifierElement::Annotation(_), _) => type_use.push(element),
            _ => keywords.push(element),
        }
    }
    sort_keywords(f, &mut keywords);
    let mut docs = Vec::new();
    for element in lines {
        docs.extend([modifier_element(f, element), f.hard_line()]);
    }
    for element in keywords.into_iter().chain(type_use) {
        docs.extend([modifier_element(f, element), f.space()]);
    }
    f.concat(docs)
}

/// Declarations whose annotations all go on their own lines before the
/// keywords, wherever they were written (the reference's choice).
const fn hoists_annotations(parent: K) -> bool {
    matches!(
        parent,
        K::ClassDeclaration
            | K::RecordDeclaration
            | K::EnumDeclaration
            | K::InterfaceDeclaration
            | K::AnnotationInterfaceDeclaration
            | K::ConstructorDeclaration
            | K::CompactConstructorDeclaration
            | K::AnnotationElementDeclaration
    )
}

/// Sorts each run of keywords between barriers into the canonical order
/// (stable, so unknown keywords keep their relative order).
fn sort_keywords<'a>(f: &JavaFormatter<'a>, keywords: &mut [ModifierElement<'a>]) {
    let mut start = 0;
    for index in 0..=keywords.len() {
        if index == keywords.len() || is_barrier(f, keywords[index]) {
            keywords[start..index].sort_by_key(|&element| keyword_order(element));
            start = index + 1;
        }
    }
}

fn is_barrier<'a>(f: &JavaFormatter<'a>, element: ModifierElement<'a>) -> bool {
    match element {
        ModifierElement::Token(token) => {
            let token = SyntaxElement::Token(token);
            f.has_leading_comments(token) || f.has_trailing_comments(token)
        }
        ModifierElement::NonSealedModifier(modifier) => f.has_comments_within(modifier.syntax()),
        ModifierElement::Annotation(_) | ModifierElement::Bogus(_) => true,
    }
}

fn keyword_order(element: ModifierElement<'_>) -> u8 {
    match element {
        ModifierElement::Token(token) => token_order(token),
        ModifierElement::NonSealedModifier(_) => 12,
        ModifierElement::Annotation(_) | ModifierElement::Bogus(_) => u8::MAX,
    }
}

fn token_order(token: SyntaxToken<'_>) -> u8 {
    match K::from_raw(token.kind()) {
        K::PublicKw => 0,
        K::ProtectedKw => 1,
        K::PrivateKw => 2,
        K::AbstractKw => 3,
        K::DefaultKw => 4,
        K::StaticKw => 5,
        K::FinalKw => 6,
        K::TransientKw => 7,
        K::VolatileKw => 8,
        K::SynchronizedKw => 9,
        K::NativeKw => 10,
        K::StrictfpKw => 13,
        _ if is_contextual(token, "sealed") => 11,
        _ => u8::MAX,
    }
}

fn modifier_element<'a>(f: &mut JavaFormatter<'a>, element: ModifierElement<'a>) -> Doc {
    match element {
        ModifierElement::Annotation(annotation) => f.fmt(annotation),
        ModifierElement::Bogus(bogus) => f.fmt(bogus),
        ModifierElement::NonSealedModifier(modifier) => f.fmt(modifier),
        ModifierElement::Token(token) => f.token(token),
    }
}

/// The declaration layout of `final` and annotations (local variables,
/// enhanced-for variables, resources, type patterns), in source order.
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

/// `non-sealed`: three tokens printed without spaces.
pub(crate) fn non_sealed_modifier<'a>(
    f: &mut JavaFormatter<'a>,
    node: NonSealedModifier<'a>,
) -> Doc {
    let non = f.token_opt(node.non_keyword());
    let minus = f.token_opt(node.minus());
    let sealed = f.token_opt(node.sealed_keyword());
    cat![f; non, minus, sealed]
}

/// Type-use annotations inline, each followed by a space; NIL when empty.
pub(crate) fn annotation_list<'a>(f: &mut JavaFormatter<'a>, node: AnnotationList<'a>) -> Doc {
    let mut docs = Vec::new();
    for annotation in annotations(f, Some(node)) {
        docs.extend([annotation, f.space()]);
    }
    f.concat(docs)
}

/// Declaration annotations of a package, module or enum constant, each on
/// its own line, followed by a line break; NIL when empty.
pub(crate) fn annotation_lines<'a>(
    f: &mut JavaFormatter<'a>,
    node: Option<AnnotationList<'a>>,
) -> Doc {
    let mut docs = Vec::new();
    for annotation in annotations(f, node) {
        docs.extend([annotation, f.hard_line()]);
    }
    f.concat(docs)
}

/// Every annotation of the list, bogus items included.
fn annotations<'a>(f: &mut JavaFormatter<'a>, node: Option<AnnotationList<'a>>) -> Vec<Doc> {
    node.into_iter()
        .flat_map(AnnotationList::elements)
        .filter_map(|element| match element {
            ListElement::Item(annotation) => Some(f.fmt(annotation)),
            ListElement::Bogus(bogus) => Some(f.node(bogus)),
            ListElement::Separator(_) | ListElement::Missing => None,
        })
        .collect()
}
