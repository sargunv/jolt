use jolt_fmt_ir::{Doc, concat, hard_line, text};
use jolt_java_syntax::{Annotation, JavaSyntaxKind, JavaSyntaxToken, ModifierList};

use crate::helpers::comments::format_token_sequence;

pub(crate) fn modifier_prefix(modifiers: Option<ModifierList>) -> Doc {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    modifier_prefix_from_parts(
        modifiers.annotations().collect(),
        modifiers.modifier_tokens().collect(),
    )
}

pub(crate) fn modifier_prefix_from_parts(
    annotations: Vec<Annotation>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> Doc {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);

    let mut docs = Vec::new();
    for annotation in annotations {
        docs.push(format_token_sequence(&annotation.tokens()));
        docs.push(hard_line());
    }
    if !modifier_tokens.is_empty() {
        docs.push(jolt_fmt_ir::join(
            text(" "),
            modifier_tokens
                .into_iter()
                .map(|token| text(token.text().to_owned())),
        ));
        docs.push(text(" "));
    }

    concat(docs)
}

fn sorted_modifier_tokens(mut tokens: Vec<JavaSyntaxToken>) -> Vec<JavaSyntaxToken> {
    tokens.sort_by_key(|token| modifier_order(token.kind()));
    tokens
}

const fn modifier_order(kind: JavaSyntaxKind) -> u8 {
    match kind {
        JavaSyntaxKind::PublicKw => 0,
        JavaSyntaxKind::ProtectedKw => 1,
        JavaSyntaxKind::PrivateKw => 2,
        JavaSyntaxKind::AbstractKw => 3,
        JavaSyntaxKind::DefaultKw => 4,
        JavaSyntaxKind::StaticKw => 5,
        JavaSyntaxKind::FinalKw => 6,
        JavaSyntaxKind::TransientKw => 7,
        JavaSyntaxKind::VolatileKw => 8,
        JavaSyntaxKind::SynchronizedKw => 9,
        JavaSyntaxKind::NativeKw => 10,
        JavaSyntaxKind::StrictfpKw => 13,
        _ => u8::MAX,
    }
}
