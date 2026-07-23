use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxToken, NonSealedModifier};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_after_relocated_leading_comments,
    token_has_comments, trailing_comments_force_line,
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

    fn trailing_comments_force_line(&self) -> bool {
        match self {
            Self::Token(token) | Self::Sealed(token) => trailing_comments_force_line(token),
            Self::NonSealed(modifier) => modifier
                .last_token()
                .is_some_and(|token| trailing_comments_force_line(&token)),
            Self::Malformed(_) => false,
        }
    }
}

pub(crate) fn modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    suppress_first_entry_leading: bool,
) -> Doc<'source> {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    doc.concat_list(|docs| {
        let mut previous_is_structured = false;
        let mut previous_forces_line = false;
        for (index, entry) in modifier_entries.into_iter().enumerate() {
            let entry_is_structured = entry.is_structured();
            let entry_forces_line = entry.trailing_comments_force_line();
            if !docs.is_empty()
                && previous_is_structured
                && (entry_is_structured || previous_forces_line)
            {
                let separator = if previous_forces_line {
                    docs.hard_line()
                } else {
                    docs.space()
                };
                docs.push(separator);
            }
            let leading = if suppress_first_entry_leading && index == 0 {
                LeadingComments::Suppress
            } else {
                LeadingComments::Preserve
            };
            let entry = format_modifier_entry(docs, &entry, leading);
            docs.push(entry);
            previous_is_structured = entry_is_structured;
            previous_forces_line = entry_forces_line;
        }
        if previous_is_structured {
            let separator = if previous_forces_line {
                docs.hard_line()
            } else {
                docs.space()
            };
            docs.push(separator);
        }
    })
}

pub(crate) fn inline_modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    annotation_docs: impl IntoIterator<Item = Doc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    suppress_first_entry_leading: bool,
    terminal_forces_line: bool,
    append_terminal_line: bool,
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
        let mut previous_forces_line = false;
        for (index, entry) in modifier_entries.into_iter().enumerate() {
            let entry_is_structured = entry.is_structured();
            let entry_forces_line = entry.trailing_comments_force_line();
            if !docs.is_empty()
                && previous_is_structured
                && (entry_is_structured || previous_forces_line)
            {
                let separator = if previous_forces_line {
                    docs.hard_line()
                } else {
                    docs.space()
                };
                docs.push(separator);
            }
            let leading = if suppress_first_entry_leading && index == 0 && docs.is_empty() {
                LeadingComments::Suppress
            } else {
                LeadingComments::Preserve
            };
            let entry = format_modifier_entry(docs, &entry, leading);
            docs.push(entry);
            previous_is_structured = entry_is_structured;
            previous_forces_line = entry_forces_line;
        }
        has_docs = !docs.is_empty();
        ends_with_structured = previous_is_structured;
    });
    if !has_docs {
        return Doc::nil();
    }
    if append_terminal_line {
        let line = doc.hard_line();
        doc_concat!(doc, [docs, line])
    } else if terminal_forces_line {
        docs
    } else if ends_with_structured {
        let space = doc.space();
        doc_concat!(doc, [docs, space])
    } else {
        docs
    }
}

fn sorted_modifier_entries(mut entries: Vec<ModifierEntry<'_>>) -> Vec<ModifierEntry<'_>> {
    let is_barrier = |entry: &ModifierEntry<'_>| match entry {
        ModifierEntry::Token(token) | ModifierEntry::Sealed(token) => token_has_comments(token),
        ModifierEntry::NonSealed(non_sealed) => non_sealed
            .token_iter()
            .any(|token| token_has_comments(&token)),
        ModifierEntry::Malformed(_) => true,
    };
    let mut run_start = None;
    for index in 0..entries.len() {
        if is_barrier(&entries[index]) {
            if let Some(start) = run_start.take() {
                sort_modifier_run(&mut entries[start..index]);
            }
        } else if run_start.is_none() {
            run_start = Some(index);
        }
    }
    if let Some(start) = run_start {
        sort_modifier_run(&mut entries[start..]);
    }
    entries
}

fn sort_modifier_run(run: &mut [ModifierEntry<'_>]) {
    if !run
        .windows(2)
        .all(|pair| modifier_entry_order(&pair[0]) <= modifier_entry_order(&pair[1]))
    {
        run.sort_by_key(modifier_entry_order);
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
                format_modifier_token(docs, &token, LeadingComments::Preserve)
            });
            docs.push(minus);
            let sealed = format_required_field(non_sealed.sealed_keyword(), docs, |token, docs| {
                format_modifier_token(docs, &token, LeadingComments::Preserve)
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
