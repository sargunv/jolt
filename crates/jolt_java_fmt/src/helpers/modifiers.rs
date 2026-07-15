use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxToken, NonSealedModifier};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_after_relocated_leading_comments,
    token_has_comments,
};
use crate::helpers::recovery::format_required_field;

#[derive(Clone, Copy)]
pub(crate) enum ModifierEntry<'source> {
    Token(JavaSyntaxToken<'source>),
    Sealed(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
    Malformed(Doc<'source>),
}

impl ModifierEntry<'_> {
    fn is_structured(&self) -> bool {
        !matches!(self, Self::Malformed(_))
    }
}

pub(crate) fn modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    annotation_docs: impl IntoIterator<Item = Doc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
) -> Doc<'source> {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    doc.concat_list(|docs| {
        for annotation in annotation_docs {
            docs.push(annotation);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
        let mut has_modifiers = false;
        let mut last_modifier_is_structured = false;
        let modifiers = docs.concat_list(|modifiers| {
            let mut previous_is_structured = false;
            for entry in modifier_entries {
                let entry_is_structured = entry.is_structured();
                if !modifiers.is_empty() && previous_is_structured && entry_is_structured {
                    let space = modifiers.space();
                    modifiers.push(space);
                }
                let entry = format_modifier_entry(modifiers, &entry, LeadingComments::Suppress);
                modifiers.push(entry);
                previous_is_structured = entry_is_structured;
            }
            has_modifiers = !modifiers.is_empty();
            last_modifier_is_structured = previous_is_structured;
        });
        if has_modifiers {
            docs.push(modifiers);
            if last_modifier_is_structured {
                let space = docs.space();
                docs.push(space);
            }
        }
    })
}

pub(crate) fn inline_modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    annotation_docs: impl IntoIterator<Item = Doc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
) -> Doc<'source> {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    let mut has_docs = false;
    let mut ends_with_structured = false;
    let docs = doc.concat_list(|docs| {
        for annotation in annotation_docs {
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            docs.push(annotation);
        }
        let mut previous_is_structured = !docs.is_empty();
        for entry in modifier_entries {
            let entry_is_structured = entry.is_structured();
            if !docs.is_empty() && previous_is_structured && entry_is_structured {
                let space = docs.space();
                docs.push(space);
            }
            let entry = format_modifier_entry(docs, &entry, LeadingComments::Preserve);
            docs.push(entry);
            previous_is_structured = entry_is_structured;
        }
        has_docs = !docs.is_empty();
        ends_with_structured = previous_is_structured;
    });
    if !has_docs {
        return Doc::nil();
    }
    if ends_with_structured {
        let space = doc.space();
        doc_concat!(doc, [docs, space])
    } else {
        docs
    }
}

fn sorted_modifier_entries(mut entries: Vec<ModifierEntry<'_>>) -> Vec<ModifierEntry<'_>> {
    sort_modifier_runs(
        &mut entries,
        |entry| match entry {
            ModifierEntry::Token(token) | ModifierEntry::Sealed(token) => token_has_comments(token),
            ModifierEntry::NonSealed(non_sealed) => non_sealed
                .token_iter()
                .any(|token| token_has_comments(&token)),
            ModifierEntry::Malformed(_) => true,
        },
        |run| {
            if !run
                .windows(2)
                .all(|pair| modifier_entry_order(&pair[0]) <= modifier_entry_order(&pair[1]))
            {
                run.sort_by_key(modifier_entry_order);
            }
        },
    );
    entries
}

/// Stably orders the comment-free runs among `m` modifier entries.
///
/// The runs partition the input, so their stable comparison sorts perform
/// O(m log m) comparisons in total and use O(m) auxiliary storage. Each key is
/// a constant-size grammar-order integer; there is no layout search or retry.
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
    match entry {
        ModifierEntry::Token(token) => modifier_order(token.kind()),
        ModifierEntry::Sealed(_) => 11,
        ModifierEntry::NonSealed(_) => 12,
        ModifierEntry::Malformed(_) => u8::MAX,
    }
}

fn format_modifier_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &ModifierEntry<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    match entry {
        ModifierEntry::Token(token) | ModifierEntry::Sealed(token) => {
            format_modifier_token(doc, token, leading_comments)
        }
        ModifierEntry::Malformed(malformed) => *malformed,
        ModifierEntry::NonSealed(non_sealed) => doc.concat_list(|docs| {
            let non = format_required_field(non_sealed.non_keyword(), docs, |token, docs| {
                format_modifier_token(docs, &token, leading_comments)
            });
            docs.push(non);
            let minus = format_required_field(non_sealed.minus(), docs, |token, docs| {
                format_modifier_token(docs, &token, leading_comments)
            });
            docs.push(minus);
            let sealed = format_required_field(non_sealed.sealed_keyword(), docs, |token, docs| {
                format_modifier_token(docs, &token, leading_comments)
            });
            docs.push(sealed);
        }),
    }
}

fn format_modifier_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    match leading_comments {
        LeadingComments::Preserve => format_token(
            doc,
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        LeadingComments::Suppress => format_token_after_relocated_leading_comments(
            doc,
            token,
            TrailingTrivia::BeforeLineBreak,
        ),
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
