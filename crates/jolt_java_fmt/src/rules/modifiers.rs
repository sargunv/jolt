use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{Annotation, ModifierList, ParameterModifierList, PartitionedModifierItem};

use crate::helpers::modifiers::{
    ModifierEntry, inline_modifier_prefix_from_docs, modifier_prefix_from_docs,
};
use crate::helpers::recovery::format_malformed;
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

    let entry_capacity = modifiers
        .partitioned_items()
        .filter(|item| {
            !matches!(
                item,
                Ok(PartitionedModifierItem::DeclarationAnnotation(_)
                    | PartitionedModifierItem::TypeUseAnnotation(_))
            )
        })
        .count();
    let mut entries = Vec::with_capacity(entry_capacity);
    let mut annotation_index = 0;
    let annotations = doc.concat_list(|annotations| {
        for item in modifiers.partitioned_items() {
            match item {
                Ok(
                    PartitionedModifierItem::DeclarationAnnotation(annotation)
                    | PartitionedModifierItem::TypeUseAnnotation(annotation),
                ) => {
                    let annotation = if annotation_index == 0 {
                        format_annotation_without_leading_comments(&annotation, annotations)
                    } else {
                        format_annotation(&annotation, annotations)
                    };
                    annotations.push(annotation);
                    let hard_line = annotations.hard_line();
                    annotations.push(hard_line);
                    annotation_index += 1;
                }
                Ok(PartitionedModifierItem::Token(token)) => {
                    entries.push(ModifierEntry::Token(token));
                }
                Ok(PartitionedModifierItem::Sealed(token)) => {
                    entries.push(ModifierEntry::Sealed(token));
                }
                Ok(PartitionedModifierItem::NonSealed(non_sealed)) => {
                    entries.push(ModifierEntry::NonSealed(non_sealed));
                }
                Ok(PartitionedModifierItem::Malformed(malformed)) => {
                    entries.push(ModifierEntry::Malformed(format_malformed(
                        &malformed,
                        annotations,
                    )));
                }
                Ok(PartitionedModifierItem::Missing(missing)) => {
                    entries.push(ModifierEntry::Malformed(
                        crate::helpers::recovery::format_missing(&missing, annotations),
                    ));
                }
                Err(error) => {
                    annotations.block_on_invariant(error.to_string());
                    entries.push(ModifierEntry::Malformed(Doc::nil()));
                }
            }
        }
    });
    let modifiers = modifier_prefix_from_docs(doc, std::iter::empty::<Doc<'source>>(), entries);
    doc_concat!(doc, [annotations, modifiers])
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

    let parts = partition_modifier_items(&modifiers, doc);
    format_typed_modifier_prefix_from_split_parts(
        parts.declaration_annotations,
        parts.type_use_annotations,
        parts.entries,
        doc,
    )
}

pub(crate) fn format_typed_parameter_modifier_prefix<'source>(
    modifiers: &ParameterModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let (declaration_count, type_use_count, entry_count) = modifiers.partitioned_items().fold(
        (0, 0, 0),
        |(declarations, type_uses, entries), item| match item {
            Ok(PartitionedModifierItem::DeclarationAnnotation(_)) => {
                (declarations + 1, type_uses, entries)
            }
            Ok(PartitionedModifierItem::TypeUseAnnotation(_)) => {
                (declarations, type_uses + 1, entries)
            }
            Ok(
                PartitionedModifierItem::Token(_)
                | PartitionedModifierItem::Malformed(_)
                | PartitionedModifierItem::Missing(_),
            ) => (declarations, type_uses, entries + 1),
            Ok(PartitionedModifierItem::Sealed(_) | PartitionedModifierItem::NonSealed(_))
            | Err(_) => (declarations, type_uses, entries),
        },
    );
    let mut declaration_annotations = Vec::with_capacity(declaration_count);
    let mut type_use_annotations = Vec::with_capacity(type_use_count);
    let mut entries = Vec::with_capacity(entry_count);
    for item in modifiers.partitioned_items() {
        match item {
            Ok(PartitionedModifierItem::DeclarationAnnotation(annotation)) => {
                declaration_annotations.push(annotation);
            }
            Ok(PartitionedModifierItem::TypeUseAnnotation(annotation)) => {
                type_use_annotations.push(annotation);
            }
            Ok(PartitionedModifierItem::Token(token)) => {
                entries.push(ModifierEntry::Token(token));
            }
            Ok(PartitionedModifierItem::Malformed(malformed)) => {
                entries.push(ModifierEntry::Malformed(format_malformed(&malformed, doc)));
            }
            Ok(PartitionedModifierItem::Missing(missing)) => {
                entries.push(ModifierEntry::Malformed(
                    crate::helpers::recovery::format_missing(&missing, doc),
                ));
            }
            Ok(PartitionedModifierItem::Sealed(_) | PartitionedModifierItem::NonSealed(_)) => {
                doc.block_on_invariant("parameter modifier had a declaration-only role");
            }
            Err(error) => {
                doc.block_on_invariant(error.to_string());
            }
        }
    }
    format_typed_modifier_prefix_from_split_parts(
        declaration_annotations,
        type_use_annotations,
        entries,
        doc,
    )
}

struct PartitionedModifiers<'source> {
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    entries: Vec<ModifierEntry<'source>>,
}

fn partition_modifier_items<'source>(
    modifiers: &ModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> PartitionedModifiers<'source> {
    let (declaration_count, type_use_count, entry_count) = modifiers.partitioned_items().fold(
        (0, 0, 0),
        |(declarations, type_uses, entries), item| match item {
            Ok(PartitionedModifierItem::DeclarationAnnotation(_)) => {
                (declarations + 1, type_uses, entries)
            }
            Ok(PartitionedModifierItem::TypeUseAnnotation(_)) => {
                (declarations, type_uses + 1, entries)
            }
            _ => (declarations, type_uses, entries + 1),
        },
    );
    let mut result = PartitionedModifiers {
        declaration_annotations: Vec::with_capacity(declaration_count),
        type_use_annotations: Vec::with_capacity(type_use_count),
        entries: Vec::with_capacity(entry_count),
    };
    for item in modifiers.partitioned_items() {
        match item {
            Ok(PartitionedModifierItem::DeclarationAnnotation(annotation)) => {
                result.declaration_annotations.push(annotation);
            }
            Ok(PartitionedModifierItem::TypeUseAnnotation(annotation)) => {
                result.type_use_annotations.push(annotation);
            }
            Ok(PartitionedModifierItem::Token(token)) => {
                result.entries.push(ModifierEntry::Token(token));
            }
            Ok(PartitionedModifierItem::Sealed(token)) => {
                result.entries.push(ModifierEntry::Sealed(token));
            }
            Ok(PartitionedModifierItem::NonSealed(non_sealed)) => {
                result.entries.push(ModifierEntry::NonSealed(non_sealed));
            }
            Ok(PartitionedModifierItem::Malformed(malformed)) => {
                result
                    .entries
                    .push(ModifierEntry::Malformed(format_malformed(&malformed, doc)));
            }
            Ok(PartitionedModifierItem::Missing(missing)) => {
                result.entries.push(ModifierEntry::Malformed(
                    crate::helpers::recovery::format_missing(&missing, doc),
                ));
            }
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                result.entries.push(ModifierEntry::Malformed(Doc::nil()));
            }
        }
    }
    result
}

fn format_typed_modifier_prefix_from_split_parts<'source>(
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let declaration_prefix =
        format_modifier_prefix_from_parts(declaration_annotations, modifier_entries, doc);
    let type_use_prefix = if type_use_annotations.is_empty() {
        Doc::nil()
    } else {
        let type_use_annotations = format_inline_annotations(type_use_annotations, doc);
        inline_modifier_prefix_from_docs(doc, [type_use_annotations], Vec::new())
    };

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
    let annotations = format_declaration_annotations(annotations, doc);
    let modifiers =
        modifier_prefix_from_docs(doc, std::iter::empty::<Doc<'source>>(), modifier_entries);
    doc_concat!(doc, [annotations, modifiers])
}

fn format_inline_annotations<'source>(
    annotations: Vec<Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for annotation in annotations {
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            let annotation = format_annotation(&annotation, docs);
            docs.push(annotation);
        }
    })
}

fn format_declaration_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for (index, annotation) in annotations.into_iter().enumerate() {
            let annotation = if index == 0 {
                format_annotation_without_leading_comments(&annotation, docs)
            } else {
                format_annotation(&annotation, docs)
            };
            docs.push(annotation);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
    })
}
