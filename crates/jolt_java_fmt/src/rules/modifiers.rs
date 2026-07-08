use jolt_fmt_ir::Doc;
use jolt_java_syntax::{Annotation, ModifierEntry, ModifierList};

use crate::context::JavaFormatter;
use crate::helpers::modifiers::{inline_modifier_prefix_from_docs, modifier_prefix_from_docs};
use crate::rules::annotations::{format_annotation, format_annotation_without_leading_comments};

pub(crate) struct TypedModifierPrefix<'source> {
    pub(crate) declaration_prefix: Doc<'source>,
    pub(crate) type_use_prefix: Doc<'source>,
}

pub(crate) fn format_modifier_prefix<'source>(
    modifiers: Option<ModifierList<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    format_modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_entries().collect(),
        formatter,
    )
}

pub(crate) fn format_typed_modifier_prefix<'source>(
    modifiers: Option<ModifierList<'source>>,
    formatter: &JavaFormatter<'_>,
) -> TypedModifierPrefix<'source> {
    let Some(modifiers) = modifiers else {
        return TypedModifierPrefix {
            declaration_prefix: jolt_fmt_ir::nil(),
            type_use_prefix: jolt_fmt_ir::nil(),
        };
    };

    format_typed_modifier_prefix_from_split_parts(
        modifiers.declaration_annotations().collect(),
        modifiers.type_use_annotations_after_modifiers().collect(),
        modifiers.modifier_entries().collect(),
        formatter,
    )
}

fn format_typed_modifier_prefix_from_split_parts<'source>(
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> TypedModifierPrefix<'source> {
    TypedModifierPrefix {
        declaration_prefix: format_modifier_prefix_from_parts(
            declaration_annotations,
            modifier_entries,
            formatter,
        ),
        type_use_prefix: inline_modifier_prefix_from_docs(
            type_use_annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation, formatter))
                .collect(),
            Vec::new(),
        ),
    }
}

pub(crate) fn format_typed_modifier_prefix_from_split_entries<'source>(
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> TypedModifierPrefix<'source> {
    TypedModifierPrefix {
        declaration_prefix: modifier_prefix_from_docs(
            declaration_annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation, formatter))
                .collect(),
            modifier_entries,
        ),
        type_use_prefix: inline_modifier_prefix_from_docs(
            type_use_annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation, formatter))
                .collect(),
            Vec::new(),
        ),
    }
}

pub(crate) fn format_modifier_prefix_from_parts<'source>(
    annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    modifier_prefix_from_docs(
        annotations
            .into_iter()
            .enumerate()
            .map(|(index, annotation)| {
                if index == 0 {
                    format_annotation_without_leading_comments(&annotation, formatter)
                } else {
                    format_annotation(&annotation, formatter)
                }
            })
            .collect(),
        modifier_entries,
    )
}
