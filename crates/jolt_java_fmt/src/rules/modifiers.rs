use jolt_fmt_ir::Doc;
use jolt_java_syntax::{Annotation, JavaSyntaxToken, ModifierList};

use crate::context::JavaFormatter;
use crate::helpers::modifiers::{inline_modifier_prefix_from_docs, modifier_prefix_from_docs};
use crate::rules::annotations::format_annotation;

pub(crate) struct TypedModifierPrefix {
    pub(crate) declaration_prefix: Doc,
    pub(crate) type_use_prefix: Doc,
}

pub(crate) fn format_modifier_prefix(
    modifiers: Option<ModifierList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    format_modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_tokens().collect(),
        formatter,
    )
}

pub(crate) fn format_typed_modifier_prefix(
    modifiers: Option<ModifierList>,
    formatter: &JavaFormatter<'_>,
) -> TypedModifierPrefix {
    let Some(modifiers) = modifiers else {
        return TypedModifierPrefix {
            declaration_prefix: jolt_fmt_ir::nil(),
            type_use_prefix: jolt_fmt_ir::nil(),
        };
    };

    format_typed_modifier_prefix_from_split_parts(
        modifiers.declaration_annotations().collect(),
        modifiers.type_use_annotations_after_modifiers().collect(),
        modifiers.modifier_tokens().collect(),
        formatter,
    )
}

pub(crate) fn format_typed_modifier_prefix_from_split_parts(
    declaration_annotations: Vec<Annotation>,
    type_use_annotations: Vec<Annotation>,
    modifier_tokens: Vec<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> TypedModifierPrefix {
    TypedModifierPrefix {
        declaration_prefix: format_modifier_prefix_from_parts(
            declaration_annotations,
            modifier_tokens,
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

pub(crate) fn format_modifier_prefix_from_parts(
    annotations: Vec<Annotation>,
    modifier_tokens: Vec<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    modifier_prefix_from_docs(
        annotations
            .into_iter()
            .map(|annotation| format_annotation(&annotation, formatter))
            .collect(),
        modifier_tokens,
    )
}
