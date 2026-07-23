use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{Annotation, ModifierList, ParameterModifierList, PartitionedModifierItem};

use crate::helpers::comments::{
    comment_forces_line, format_construct_leading_comments, trailing_comments_force_line,
};
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
    let Some(authorization) = modifiers.canonical_reorder_claim() else {
        return format_modifier_items_in_source_order(modifiers.partitioned_items(), true, doc);
    };
    let first_is_annotation = matches!(
        modifiers.partitioned_items().next(),
        Some(Ok(PartitionedModifierItem::DeclarationAnnotation(_)
            | PartitionedModifierItem::TypeUseAnnotation(_)))
    );
    let leading = format_construct_leading_comments(doc, modifiers.first_token().as_ref());
    let parts = partition_modifier_items(&modifiers, doc);
    let annotations = parts
        .declaration_annotations
        .into_iter()
        .chain(parts.type_use_annotations);
    let annotations = format_declaration_annotations(annotations, first_is_annotation, doc);
    let modifiers = modifier_prefix_from_docs(doc, parts.entries, !first_is_annotation);
    let formatted = doc_concat!(doc, [leading, annotations, modifiers]);
    doc.reordered_source(formatted, authorization)
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
    let Some(authorization) = modifiers.canonical_reorder_claim() else {
        return TypedModifierPrefix {
            declaration_prefix: format_modifier_items_in_source_order(
                modifiers.partitioned_items(),
                true,
                doc,
            ),
            type_use_prefix: Doc::nil(),
        };
    };
    let parts = partition_modifier_items(&modifiers, doc);
    let formatted = format_typed_modifier_prefix_from_split_parts(
        parts.declaration_annotations,
        parts.type_use_annotations,
        parts.entries,
        doc,
    );
    authorize_typed_prefix(&formatted, modifiers.first_token(), authorization, doc)
}

pub(crate) fn format_typed_parameter_modifier_prefix<'source>(
    modifiers: &ParameterModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let Some(authorization) = modifiers.canonical_reorder_claim() else {
        return TypedModifierPrefix {
            declaration_prefix: format_modifier_items_in_source_order(
                modifiers.partitioned_items(),
                false,
                doc,
            ),
            type_use_prefix: Doc::nil(),
        };
    };
    let parts = partition_parameter_modifier_items(modifiers, doc);
    let formatted = format_typed_modifier_prefix_from_split_parts(
        parts.declaration_annotations,
        parts.type_use_annotations,
        parts.entries,
        doc,
    );
    authorize_typed_prefix(&formatted, modifiers.first_token(), authorization, doc)
}

pub(crate) fn format_inline_typed_parameter_modifier_prefix<'source>(
    modifiers: &ParameterModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let Some(authorization) = modifiers.canonical_reorder_claim() else {
        return TypedModifierPrefix {
            declaration_prefix: format_modifier_items_in_source_order(
                modifiers.partitioned_items(),
                false,
                doc,
            ),
            type_use_prefix: Doc::nil(),
        };
    };
    let parts = partition_parameter_modifier_items(modifiers, doc);
    let type_use_is_first = parts.declaration_annotations.is_empty() && parts.entries.is_empty();
    let (type_use_forces_line, type_use_needs_line) =
        last_annotation_line_state(&parts.type_use_annotations);
    let (terminal_forces_line, terminal_needs_line) = if let Some(entry) = parts.entries.last() {
        let token = match entry {
            ModifierEntry::Token(token) | ModifierEntry::Sealed(token) => Some(*token),
            ModifierEntry::NonSealed(modifier) => modifier.last_token(),
            ModifierEntry::Malformed(_) => None,
        };
        let forces = token.is_some_and(|token| {
            token
                .trailing_comments()
                .any(|comment| comment_forces_line(&comment))
        });
        (forces, forces)
    } else {
        last_annotation_line_state(&parts.declaration_annotations)
    };
    let declaration_annotations =
        format_inline_annotations(parts.declaration_annotations, true, doc);
    let declaration_prefix = inline_modifier_prefix_from_docs(
        doc,
        [declaration_annotations],
        parts.entries,
        true,
        terminal_forces_line,
        terminal_needs_line,
    );
    let type_use_prefix = if parts.type_use_annotations.is_empty() {
        Doc::nil()
    } else {
        let annotations =
            format_inline_annotations(parts.type_use_annotations, type_use_is_first, doc);
        inline_modifier_prefix_from_docs(
            doc,
            [annotations],
            Vec::new(),
            false,
            type_use_forces_line,
            type_use_needs_line,
        )
    };
    authorize_typed_prefix(
        &TypedModifierPrefix {
            declaration_prefix,
            type_use_prefix,
        },
        modifiers.first_token(),
        authorization,
        doc,
    )
}

fn authorize_typed_prefix<'source>(
    prefix: &TypedModifierPrefix<'source>,
    first: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    authorization: jolt_java_syntax::ReorderClaim<'source>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    let declaration_prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, first.as_ref()),
            prefix.declaration_prefix,
        ]
    );
    TypedModifierPrefix {
        declaration_prefix: doc.reordered_source(declaration_prefix, authorization),
        type_use_prefix: prefix.type_use_prefix,
    }
}

struct PartitionedModifiers<'source> {
    declaration_annotations: Vec<Annotation<'source>>,
    type_use_annotations: Vec<Annotation<'source>>,
    entries: Vec<ModifierEntry<'source>>,
}

fn format_modifier_items_in_source_order<'source>(
    items: impl IntoIterator<
        Item = Result<PartitionedModifierItem<'source>, jolt_java_syntax::JavaSyntaxInvariantError>,
    >,
    annotations_break: bool,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = items
        .into_iter()
        .map(|item| match item {
            Ok(
                PartitionedModifierItem::DeclarationAnnotation(annotation)
                | PartitionedModifierItem::TypeUseAnnotation(annotation),
            ) => {
                let forces_line = last_annotation_line_state(std::slice::from_ref(&annotation)).0;
                (format_annotation(&annotation, doc), true, true, forces_line)
            }
            Ok(PartitionedModifierItem::Token(token)) => (
                inline_modifier_prefix_from_docs(
                    doc,
                    std::iter::empty(),
                    vec![ModifierEntry::Token(token)],
                    false,
                    true,
                    false,
                ),
                true,
                false,
                trailing_comments_force_line(&token),
            ),
            Ok(PartitionedModifierItem::Sealed(token)) => (
                inline_modifier_prefix_from_docs(
                    doc,
                    std::iter::empty(),
                    vec![ModifierEntry::Sealed(token)],
                    false,
                    true,
                    false,
                ),
                true,
                false,
                trailing_comments_force_line(&token),
            ),
            Ok(PartitionedModifierItem::NonSealed(non_sealed)) => {
                let forces_line = non_sealed
                    .last_token()
                    .is_some_and(|token| trailing_comments_force_line(&token));
                (
                    inline_modifier_prefix_from_docs(
                        doc,
                        std::iter::empty(),
                        vec![ModifierEntry::NonSealed(non_sealed)],
                        false,
                        true,
                        false,
                    ),
                    true,
                    false,
                    forces_line,
                )
            }
            Ok(PartitionedModifierItem::Bogus(bogus)) => {
                (format_malformed(&bogus, doc), false, false, false)
            }
            Ok(PartitionedModifierItem::Malformed(malformed)) => {
                (format_malformed(&malformed, doc), false, false, false)
            }
            Ok(PartitionedModifierItem::Missing(missing)) => (
                crate::helpers::recovery::format_missing(&missing, doc),
                false,
                false,
                false,
            ),
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                (Doc::nil(), false, false, false)
            }
        })
        .collect::<Vec<_>>();
    doc.concat_list(|docs| {
        let mut previous_structured = false;
        let mut previous_annotation = false;
        let mut previous_forces_line = false;
        for (item, structured, annotation, forces_line) in items {
            if previous_structured && (structured || previous_forces_line) {
                let separator =
                    if previous_forces_line || (annotations_break && previous_annotation) {
                        docs.hard_line()
                    } else {
                        docs.space()
                    };
                docs.push(separator);
            }
            docs.push(item);
            previous_structured = structured;
            previous_annotation = annotation;
            previous_forces_line = forces_line;
        }
        if previous_structured {
            let separator = if previous_forces_line || (annotations_break && previous_annotation) {
                docs.hard_line()
            } else {
                docs.space()
            };
            docs.push(separator);
        }
    })
}

fn partition_parameter_modifier_items<'source>(
    modifiers: &ParameterModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> PartitionedModifiers<'source> {
    partition_items(modifiers.partitioned_items(), doc)
}

fn partition_modifier_items<'source>(
    modifiers: &ModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> PartitionedModifiers<'source> {
    partition_items(modifiers.partitioned_items(), doc)
}

fn partition_items<'source>(
    items: impl IntoIterator<
        Item = Result<PartitionedModifierItem<'source>, jolt_java_syntax::JavaSyntaxInvariantError>,
    >,
    doc: &mut DocBuilder<'source>,
) -> PartitionedModifiers<'source> {
    let mut result = PartitionedModifiers {
        declaration_annotations: Vec::new(),
        type_use_annotations: Vec::new(),
        entries: Vec::new(),
    };
    for item in items {
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
            Ok(PartitionedModifierItem::Bogus(bogus)) => {
                result
                    .entries
                    .push(ModifierEntry::Malformed(format_malformed(&bogus, doc)));
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
        let (terminal_forces_line, terminal_needs_line) =
            last_annotation_line_state(&type_use_annotations);
        let type_use_annotations = format_inline_annotations(type_use_annotations, false, doc);
        inline_modifier_prefix_from_docs(
            doc,
            [type_use_annotations],
            Vec::new(),
            false,
            terminal_forces_line,
            terminal_needs_line,
        )
    };

    TypedModifierPrefix {
        declaration_prefix,
        type_use_prefix,
    }
}

fn last_annotation_token<'source>(
    annotations: &[Annotation<'source>],
) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
    annotations.last().and_then(Annotation::last_token)
}

fn last_annotation_line_state(annotations: &[Annotation<'_>]) -> (bool, bool) {
    let Some(token) = last_annotation_token(annotations) else {
        return (false, false);
    };
    let forces = token
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    (
        forces,
        forces && token.kind() != jolt_java_syntax::JavaSyntaxKind::RParen,
    )
}

pub(crate) fn format_modifier_prefix_from_parts<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut annotations = annotations.into_iter().peekable();
    let modifiers_own_leading = annotations.peek().is_none();
    let annotations = format_declaration_annotations(annotations, !modifiers_own_leading, doc);
    let modifiers = modifier_prefix_from_docs(doc, modifier_entries, modifiers_own_leading);
    doc_concat!(doc, [annotations, modifiers])
}

fn format_inline_annotations<'source>(
    annotations: Vec<Annotation<'source>>,
    suppress_first_leading: bool,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for (index, annotation) in annotations.into_iter().enumerate() {
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            let annotation = if suppress_first_leading && index == 0 {
                format_annotation_without_leading_comments(&annotation, docs)
            } else {
                format_annotation(&annotation, docs)
            };
            docs.push(annotation);
        }
    })
}

fn format_declaration_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    suppress_first_leading: bool,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for (index, annotation) in annotations.into_iter().enumerate() {
            let annotation = if suppress_first_leading && index == 0 {
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
