use jolt_fmt_ir::{Doc, concat, hard_line, join, space};
use jolt_kotlin_syntax::{KotlinSyntaxKind, KotlinSyntaxToken};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token, token_has_comments};

pub(crate) fn modifier_prefix_from_parts<'source>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_tokens: &mut [KotlinSyntaxToken<'source>],
) -> Doc<'source> {
    if annotation_docs.is_empty() && modifier_tokens.is_empty() {
        return jolt_fmt_ir::nil();
    }

    sort_modifier_runs(modifier_tokens);
    let mut docs = Vec::new();
    for annotation in annotation_docs {
        docs.push(annotation);
        docs.push(hard_line());
    }
    if !modifier_tokens.is_empty() {
        docs.push(join(
            &space(),
            modifier_tokens.iter().map(format_modifier_token),
        ));
        docs.push(space());
    }

    concat(docs)
}

fn sort_modifier_runs(tokens: &mut [KotlinSyntaxToken<'_>]) {
    let mut run_start = None;

    for index in 0..tokens.len() {
        if token_has_comments(&tokens[index]) {
            if let Some(start) = run_start.take() {
                tokens[start..index].sort_by_key(modifier_order);
            }
        } else if run_start.is_none() {
            run_start = Some(index);
        }
    }

    if let Some(start) = run_start {
        tokens[start..].sort_by_key(modifier_order);
    }
}

fn format_modifier_token<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
}

fn modifier_order(token: &KotlinSyntaxToken<'_>) -> u8 {
    match token.kind() {
        KotlinSyntaxKind::PublicKw => 0,
        KotlinSyntaxKind::ProtectedKw => 1,
        KotlinSyntaxKind::PrivateKw => 2,
        KotlinSyntaxKind::InternalKw => 3,
        KotlinSyntaxKind::ExpectKw => 4,
        KotlinSyntaxKind::ActualKw => 5,
        KotlinSyntaxKind::FinalKw => 6,
        KotlinSyntaxKind::OpenKw => 7,
        KotlinSyntaxKind::AbstractKw => 8,
        KotlinSyntaxKind::SealedKw => 9,
        KotlinSyntaxKind::ConstKw => 10,
        KotlinSyntaxKind::ExternalKw => 11,
        KotlinSyntaxKind::OverrideKw => 12,
        KotlinSyntaxKind::LateinitKw => 13,
        KotlinSyntaxKind::TailrecKw => 14,
        KotlinSyntaxKind::VarargKw => 15,
        KotlinSyntaxKind::SuspendKw => 16,
        KotlinSyntaxKind::InnerKw => 17,
        KotlinSyntaxKind::EnumKw => 18,
        KotlinSyntaxKind::AnnotationKw => 19,
        KotlinSyntaxKind::FunKw => 20,
        KotlinSyntaxKind::CompanionKw => 21,
        KotlinSyntaxKind::InlineKw => 22,
        KotlinSyntaxKind::ValueKw => 23,
        KotlinSyntaxKind::InfixKw => 24,
        KotlinSyntaxKind::OperatorKw => 25,
        KotlinSyntaxKind::DataKw => 26,
        _ => soft_modifier_order(token.text()),
    }
}

fn soft_modifier_order(text: &str) -> u8 {
    match text {
        "public" => 0,
        "protected" => 1,
        "private" => 2,
        "internal" => 3,
        "expect" => 4,
        "actual" => 5,
        "final" => 6,
        "open" => 7,
        "abstract" => 8,
        "sealed" => 9,
        "const" => 10,
        "external" => 11,
        "override" => 12,
        "lateinit" => 13,
        "tailrec" => 14,
        "vararg" => 15,
        "suspend" => 16,
        "inner" => 17,
        "enum" => 18,
        "annotation" => 19,
        "fun" => 20,
        "companion" => 21,
        "inline" => 22,
        "value" => 23,
        "infix" => 24,
        "operator" => 25,
        "data" => 26,
        _ => u8::MAX,
    }
}
