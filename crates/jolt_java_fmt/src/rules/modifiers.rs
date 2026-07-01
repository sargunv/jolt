use jolt_fmt_ir::Doc;
use jolt_java_syntax::{Annotation, JavaSyntaxToken, ModifierList};

use crate::helpers::modifiers::modifier_prefix_from_docs;
use crate::rules::annotations::format_annotation;

pub(crate) fn format_modifier_prefix(modifiers: Option<ModifierList>) -> Doc {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    format_modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_tokens().collect(),
    )
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
