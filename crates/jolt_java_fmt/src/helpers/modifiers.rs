use jolt_fmt_ir::{Doc, concat, hard_line, text};
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxToken, ModifierEntry};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_after_relocated_leading_comments,
    token_has_comments,
};

pub(crate) fn modifier_prefix_from_docs(
    annotation_docs: Vec<Doc>,
    modifier_entries: Vec<ModifierEntry>,
) -> Doc {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    modifier_prefix_from_modifier_docs(
        annotation_docs,
        modifier_entries
            .into_iter()
            .map(|entry| format_modifier_entry(&entry, LeadingComments::Suppress))
            .collect(),
    )
}

pub(crate) fn modifier_prefix_from_token_docs(
    annotation_docs: Vec<Doc>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> Doc {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);
    modifier_prefix_from_modifier_docs(
        annotation_docs,
        modifier_tokens
            .into_iter()
            .map(|token| format_modifier_token(&token, LeadingComments::Preserve))
            .collect(),
    )
}

fn modifier_prefix_from_modifier_docs(annotation_docs: Vec<Doc>, modifier_docs: Vec<Doc>) -> Doc {
    let mut docs = Vec::new();
    for annotation in annotation_docs {
        docs.push(annotation);
        docs.push(hard_line());
    }
    if !modifier_docs.is_empty() {
        docs.push(jolt_fmt_ir::join(text(" "), modifier_docs));
        docs.push(text(" "));
    }

    concat(docs)
}

pub(crate) fn inline_modifier_prefix_from_docs(
    annotation_docs: Vec<Doc>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> Doc {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);
    let mut docs = annotation_docs;
    docs.extend(
        modifier_tokens
            .into_iter()
            .map(|token| format_modifier_token(&token, LeadingComments::Preserve)),
    );

    if docs.is_empty() {
        jolt_fmt_ir::nil()
    } else {
        concat([jolt_fmt_ir::join(text(" "), docs), text(" ")])
    }
}

fn sorted_modifier_tokens(mut tokens: Vec<JavaSyntaxToken>) -> Vec<JavaSyntaxToken> {
    sort_modifier_runs(&mut tokens, token_has_comments, |run| {
        run.sort_by_key(|token| modifier_order(token.kind()));
    });
    tokens
}

fn sorted_modifier_entries(mut entries: Vec<ModifierEntry>) -> Vec<ModifierEntry> {
    sort_modifier_runs(
        &mut entries,
        |entry| entry.tokens.iter().any(token_has_comments),
        |run| run.sort_by_key(modifier_entry_order),
    );
    entries
}

fn sort_modifier_runs<T>(
    items: &mut [T],
    mut is_barrier: impl FnMut(&T) -> bool,
    mut sort_run: impl FnMut(&mut [T]),
) {
    // Comment-bearing modifiers keep their original position; only the
    // comment-free runs between them are reorderable.
    let mut run_start = None;

    for index in 0..items.len() {
        if is_barrier(&items[index]) {
            if let Some(start) = run_start.take() {
                sort_run(&mut items[start..index]);
            }
        } else if run_start.is_none() {
            run_start = Some(index);
        }
    }

    if let Some(start) = run_start {
        sort_run(&mut items[start..]);
    }
}

fn modifier_entry_order(entry: &ModifierEntry) -> u8 {
    match modifier_entry_text(entry).as_str() {
        "sealed" => 11,
        "non-sealed" => 12,
        _ => entry
            .tokens
            .first()
            .map_or(u8::MAX, |token| modifier_order(token.kind())),
    }
}

fn modifier_entry_text(entry: &ModifierEntry) -> String {
    entry.tokens.iter().map(JavaSyntaxToken::text).collect()
}

fn format_modifier_entry(entry: &ModifierEntry, leading_comments: LeadingComments) -> Doc {
    concat(
        entry
            .tokens
            .iter()
            .map(|token| format_modifier_token(token, leading_comments)),
    )
}

fn format_modifier_token(token: &JavaSyntaxToken, leading_comments: LeadingComments) -> Doc {
    match leading_comments {
        LeadingComments::Preserve => format_token(
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        LeadingComments::Suppress => {
            format_token_after_relocated_leading_comments(token, TrailingTrivia::BeforeLineBreak)
        }
    }
}

#[derive(Clone, Copy)]
enum LeadingComments {
    Preserve,
    Suppress,
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
