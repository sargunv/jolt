use jolt_fmt_ir::{Doc, DocBuilder, LayoutDoc};
use jolt_java_syntax::{JavaSyntaxField, JavaSyntaxKind, JavaSyntaxToken, NonSealedModifier};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line, format_token,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    token_has_comments, trailing_comments_force_line,
};
use crate::helpers::recovery::format_required_field;

#[derive(Clone, Copy)]
pub(crate) enum ModifierEntry<'source> {
    Token(JavaSyntaxToken<'source>),
    Sealed(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
    Malformed(LayoutDoc<'source>),
}

impl ModifierEntry<'_> {
    fn is_structured(&self) -> bool {
        !matches!(self, Self::Malformed(..))
    }

    pub(crate) fn is_visible(&self) -> bool {
        match self {
            Self::Token(_) | Self::Sealed(_) => true,
            Self::NonSealed(modifier) => modifier.first_token().is_some(),
            Self::Malformed(layout) => layout.is_visible(),
        }
    }

    pub(crate) fn trailing_comments_force_line(&self) -> bool {
        match self {
            Self::Token(token) | Self::Sealed(token) => trailing_comments_force_line(token),
            Self::NonSealed(modifier) => modifier
                .last_token()
                .is_some_and(|token| trailing_comments_force_line(&token)),
            Self::Malformed(..) => false,
        }
    }

    fn leading_comments_force_line(&self) -> bool {
        match self {
            Self::Token(token) | Self::Sealed(token) => token
                .leading_comments()
                .any(|comment| comment_forces_line(&comment)),
            Self::NonSealed(modifier) => modifier.first_token().is_some_and(|token| {
                token
                    .leading_comments()
                    .any(|comment| comment_forces_line(&comment))
            }),
            Self::Malformed(..) => false,
        }
    }
}

#[derive(Clone, Copy)]
enum ModifierTerminal {
    Prefix,
    Inline {
        forces_line: bool,
        append_line: bool,
    },
}

pub(crate) fn modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    suppress_first_entry_leading: bool,
) -> Doc<'source> {
    modifier_docs(
        doc,
        None,
        modifier_entries,
        suppress_first_entry_leading,
        ModifierTerminal::Prefix,
    )
}

pub(crate) fn inline_modifier_prefix_from_docs<'source>(
    doc: &mut DocBuilder<'source>,
    annotations: Option<LayoutDoc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    suppress_first_entry_leading: bool,
    terminal_forces_line: bool,
    append_terminal_line: bool,
) -> Doc<'source> {
    modifier_docs(
        doc,
        annotations,
        modifier_entries,
        suppress_first_entry_leading,
        ModifierTerminal::Inline {
            forces_line: terminal_forces_line,
            append_line: append_terminal_line,
        },
    )
}

fn modifier_docs<'source>(
    doc: &mut DocBuilder<'source>,
    annotations: Option<LayoutDoc<'source>>,
    modifier_entries: Vec<ModifierEntry<'source>>,
    suppress_first_entry_leading: bool,
    terminal: ModifierTerminal,
) -> Doc<'source> {
    let modifier_entries = sorted_modifier_entries(modifier_entries);
    let mut visible = annotations.is_some_and(LayoutDoc::is_visible);
    let mut previous_is_structured = visible;
    let mut previous_forces_line = false;
    doc.concat_list(|docs| {
        if let Some(annotations) = annotations {
            docs.push(annotations.doc());
        }
        for entry in modifier_entries {
            let entry_is_structured = entry.is_structured();
            let entry_is_visible = entry.is_visible();
            let entry_forces_line = entry.trailing_comments_force_line();
            let entry_leading_forces_line = entry.leading_comments_force_line();
            if entry_is_visible
                && visible
                && previous_is_structured
                && (entry_is_structured || previous_forces_line)
            {
                let separator = if previous_forces_line || entry_leading_forces_line {
                    docs.hard_line()
                } else {
                    docs.space()
                };
                docs.push(separator);
            }
            let leading = if suppress_first_entry_leading && !visible {
                LeadingComments::Suppress
            } else {
                LeadingComments::Preserve
            };
            let entry = format_modifier_entry(docs, &entry, leading);
            docs.push(entry);
            if entry_is_visible {
                visible = true;
                previous_is_structured = entry_is_structured;
                previous_forces_line = entry_forces_line;
            }
        }
        if !visible {
            return;
        }
        // Boundaries, not plain breaks: the modifier the prefix ends with may
        // already have ended its own line for a trailing comment.
        let separator = match terminal {
            ModifierTerminal::Prefix if previous_is_structured => Some(if previous_forces_line {
                docs.hard_line_boundary()
            } else {
                docs.space()
            }),
            ModifierTerminal::Inline {
                append_line: true, ..
            } => Some(docs.hard_line_boundary()),
            ModifierTerminal::Inline {
                forces_line: false,
                append_line: false,
            } if previous_is_structured => Some(docs.space()),
            ModifierTerminal::Prefix | ModifierTerminal::Inline { .. } => None,
        };
        if let Some(separator) = separator {
            docs.push(separator);
        }
    })
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
        ModifierEntry::Malformed(layout) => layout.doc(),
        ModifierEntry::NonSealed(non_sealed) => doc.concat_list(|docs| {
            // `non-sealed` is three tokens with trivia allowed between them. A
            // comment in either crevice can be read as the previous token's
            // trailing trivia or the next token's leading trivia depending only
            // on which line the source put it on, and both readings have to
            // render the same or the two readings flip-flop across passes. So
            // the separator covers both sides, and an internal token's leading
            // comments stay inline rather than taking a line of their own.
            let non = non_sealed.non_keyword();
            let minus = non_sealed.minus();
            let sealed = non_sealed.sealed_keyword();
            let non_separator = non_sealed_separator(docs, &non, &minus);
            let minus_separator = non_sealed_separator(docs, &minus, &sealed);

            let formatted_non = format_required_field(non, docs, |token, docs| {
                format_modifier_token(docs, &token, leading_comments)
            });
            docs.push(formatted_non);
            docs.push(non_separator);
            let formatted_minus = format_required_field(minus, docs, |token, docs| {
                format_non_sealed_internal_token(docs, &token)
            });
            docs.push(formatted_minus);
            docs.push(minus_separator);
            let formatted_sealed = format_required_field(sealed, docs, |token, docs| {
                format_non_sealed_internal_token(docs, &token)
            });
            docs.push(formatted_sealed);
        }),
    }
}

/// Separates two tokens inside `non-sealed` when a comment sits between them,
/// whichever token owns it.
///
/// A line comment must end its line before the next token, or that token is
/// swallowed into the comment. Any other comment there still needs whitespace so
/// the tokens stay separate. With no comment on either side the tokens are
/// adjacent, spelling `non-sealed`.
fn non_sealed_separator<'source>(
    doc: &mut DocBuilder<'source>,
    before: &JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    after: &JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let trailing = match before.as_ref() {
        JavaSyntaxField::Present(token) => {
            if trailing_comments_force_line(token) {
                return doc.hard_line();
            }
            !token.trailing_comments().is_empty()
        }
        _ => false,
    };
    let leading = matches!(after.as_ref(), JavaSyntaxField::Present(token) if !token.leading_comments().is_empty());
    if trailing || leading {
        doc.space()
    } else {
        Doc::nil()
    }
}

/// Formats `non-sealed`'s `-` or `sealed`, keeping any leading comments inline.
///
/// The separator ahead of the token already supplies the space, and only a line
/// comment ends the line, so the reparse reads the comment back as the previous
/// token's trailing trivia and renders it identically.
fn format_non_sealed_internal_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        doc,
        token,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::BeforeLineBreak,
    )
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
