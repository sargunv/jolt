use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{Annotation, ModifierEntry, ModifierList};

use crate::helpers::modifiers::{inline_modifier_prefix_from_docs, modifier_prefix_from_docs};
use crate::rules::annotations::{format_annotation, format_annotation_without_leading_comments};

pub(crate) struct TypedModifierPrefix<'source> {
    pub(crate) declaration_prefix: Doc<'source>,
    pub(crate) type_use_prefix: Doc<'source>,
}

pub(crate) fn format_modifier_prefix<'source>(
    modifiers: Option<ModifierList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(modifiers) = modifiers else {
        return Doc::nil();
    };

    format_modifier_prefix_from_parts(
        modifiers.annotations(),
        modifiers.modifier_entries().collect(),
        doc,
    )
}

pub(crate) fn format_typed_modifier_prefix<'source>(
    modifiers: Option<ModifierList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let Some(modifiers) = modifiers else {
        return TypedModifierPrefix {
            declaration_prefix: Doc::nil(),
            type_use_prefix: Doc::nil(),
        };
    };

    format_typed_modifier_prefix_from_split_parts(
        modifiers.declaration_annotations().collect(),
        modifiers.type_use_annotations_after_modifiers().collect(),
        modifiers.modifier_entries().collect(),
        doc,
    )
}

fn format_typed_modifier_prefix_from_split_parts<'source>(
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let declaration_prefix =
        format_modifier_prefix_from_parts(declaration_annotations, modifier_entries, doc);
    let type_use_annotations = format_inline_annotations(type_use_annotations, doc);
    let type_use_prefix = inline_modifier_prefix_from_docs(doc, [type_use_annotations], Vec::new());

    TypedModifierPrefix {
        declaration_prefix,
        type_use_prefix,
    }
}

pub(crate) fn format_typed_modifier_prefix_from_split_entries<'source>(
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let declaration_annotations = format_declaration_annotations(
        declaration_annotations,
        FirstAnnotationLeading::Preserve,
        doc,
    );
    let declaration_modifiers =
        modifier_prefix_from_docs(doc, std::iter::empty::<Doc<'source>>(), modifier_entries);
    let declaration_prefix = doc_concat!(doc, [declaration_annotations, declaration_modifiers]);
    let type_use_annotations = format_inline_annotations(type_use_annotations, doc);
    let type_use_prefix = inline_modifier_prefix_from_docs(doc, [type_use_annotations], Vec::new());

    TypedModifierPrefix {
        declaration_prefix,
        type_use_prefix,
    }
}

pub(crate) fn format_modifier_prefix_from_parts<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations =
        format_declaration_annotations(annotations, FirstAnnotationLeading::Suppress, doc);
    let modifiers =
        modifier_prefix_from_docs(doc, std::iter::empty::<Doc<'source>>(), modifier_entries);
    doc_concat!(doc, [annotations, modifiers])
}

fn format_inline_annotations<'source>(
    annotations: Vec<Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for annotation in annotations {
        if !docs.is_empty() {
            docs.push(doc.space(), doc);
        }
        let annotation = format_annotation(&annotation, doc);
        docs.push(annotation, doc);
    }
    docs.finish(doc)
}

fn format_declaration_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    first_leading: FirstAnnotationLeading,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for (index, annotation) in annotations.into_iter().enumerate() {
        let annotation = if index == 0 && first_leading == FirstAnnotationLeading::Suppress {
            format_annotation_without_leading_comments(&annotation, doc)
        } else {
            format_annotation(&annotation, doc)
        };
        docs.push(annotation, doc);
        docs.push(doc.hard_line(), doc);
    }
    docs.finish(doc)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FirstAnnotationLeading {
    Preserve,
    Suppress,
}
