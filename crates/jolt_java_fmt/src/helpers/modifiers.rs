use jolt_fmt_ir::{Doc, concat, hard_line, join, space};
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxToken, ModifierEntry};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_after_relocated_leading_comments,
    token_has_comments,
};

pub(crate) fn modifier_prefix_from_docs<'source>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
) -> Doc<'source> {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    modifier_prefix_from_modifier_docs(
        annotation_docs,
        modifier_entries
            .into_iter()
            .map(|entry| format_modifier_entry(&entry, LeadingComments::Suppress)),
    )
}

pub(crate) fn modifier_prefix_from_token_docs<'source>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_tokens: Vec<JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);
    modifier_prefix_from_modifier_docs(
        annotation_docs,
        modifier_tokens
            .into_iter()
            .map(|token| format_modifier_token(&token, LeadingComments::Preserve)),
    )
}

fn modifier_prefix_from_modifier_docs<'source>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for annotation in annotation_docs {
        docs.push(annotation);
        docs.push(hard_line());
    }
    let mut modifier_docs = modifier_docs.into_iter().peekable();
    if modifier_docs.peek().is_some() {
        docs.push(join(&space(), modifier_docs));
        docs.push(space());
    }

    concat(docs)
}

pub(crate) fn inline_modifier_prefix_from_docs<'source>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_tokens: Vec<JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);
    let has_docs = !annotation_docs.is_empty() || !modifier_tokens.is_empty();
    if has_docs {
        concat([
            join(
                &space(),
                annotation_docs.into_iter().chain(
                    modifier_tokens
                        .into_iter()
                        .map(|token| format_modifier_token(&token, LeadingComments::Preserve)),
                ),
            ),
            space(),
        ])
    } else {
        jolt_fmt_ir::nil()
    }
}

fn sorted_modifier_tokens(mut tokens: Vec<JavaSyntaxToken<'_>>) -> Vec<JavaSyntaxToken<'_>> {
    sort_modifier_runs(&mut tokens, token_has_comments, |run| {
        run.sort_by_key(|token| modifier_order(token.kind()));
    });
    tokens
}

fn sorted_modifier_entries(mut entries: Vec<ModifierEntry<'_>>) -> Vec<ModifierEntry<'_>> {
    sort_modifier_runs(
        &mut entries,
        |entry| entry.tokens().any(token_has_comments),
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

fn modifier_entry_order(entry: &ModifierEntry<'_>) -> u8 {
    if modifier_entry_text_matches(entry, &["sealed"]) {
        11
    } else if modifier_entry_text_matches(entry, &["non", "-", "sealed"]) {
        12
    } else {
        entry
            .tokens()
            .next()
            .map_or(u8::MAX, |token| modifier_order(token.kind()))
    }
}

fn modifier_entry_text_matches(entry: &ModifierEntry<'_>, pieces: &[&str]) -> bool {
    let mut tokens = entry.tokens();
    for piece in pieces {
        let Some(token) = tokens.next() else {
            return false;
        };
        if token.text() != *piece {
            return false;
        }
    }
    tokens.next().is_none()
}

fn format_modifier_entry<'source>(
    entry: &ModifierEntry<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    concat(
        entry
            .tokens()
            .map(|token| format_modifier_token(token, leading_comments)),
    )
}

fn format_modifier_token<'source>(
    token: &JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
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
