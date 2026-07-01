use jolt_fmt_ir::Doc;
use jolt_java_syntax::{Annotation, JavaSyntaxToken, ModifierList};

use crate::helpers::modifiers::{inline_modifier_prefix_from_docs, modifier_prefix_from_docs};
use crate::rules::annotations::format_annotation;

pub(crate) struct TypedModifierPrefix {
    pub(crate) declaration_prefix: Doc,
    pub(crate) type_use_prefix: Doc,
}

pub(crate) fn format_modifier_prefix(modifiers: Option<ModifierList>) -> Doc {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    format_modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_tokens().collect(),
    )
}

pub(crate) fn format_typed_modifier_prefix(modifiers: Option<ModifierList>) -> TypedModifierPrefix {
    let Some(modifiers) = modifiers else {
        return TypedModifierPrefix {
            declaration_prefix: jolt_fmt_ir::nil(),
            type_use_prefix: jolt_fmt_ir::nil(),
        };
    };

    format_typed_modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_tokens().collect(),
    )
}

pub(crate) fn format_typed_modifier_prefix_from_parts(
    annotations: Vec<Annotation>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> TypedModifierPrefix {
    let first_modifier_start = modifier_tokens
        .iter()
        .map(|token| token.token_text_range().start())
        .min();
    let mut declaration_annotations = Vec::new();
    let mut type_use_annotations = Vec::new();

    for annotation in annotations {
        if first_modifier_start.is_some_and(|start| annotation.text_range().start() > start) {
            type_use_annotations.push(annotation);
        } else {
            declaration_annotations.push(annotation);
        }
    }

    TypedModifierPrefix {
        declaration_prefix: format_modifier_prefix_from_parts(
            declaration_annotations,
            modifier_tokens,
        ),
        type_use_prefix: inline_modifier_prefix_from_docs(
            type_use_annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation))
                .collect(),
            Vec::new(),
        ),
    }
}

pub(crate) fn format_modifier_prefix_from_parts(
    annotations: Vec<Annotation>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> Doc {
    modifier_prefix_from_docs(
        annotations
            .into_iter()
            .map(|annotation| format_annotation(&annotation))
            .collect(),
        modifier_tokens,
    )
}
