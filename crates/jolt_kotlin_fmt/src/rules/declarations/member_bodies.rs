use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ClassBody, ClassMemberDeclaration, ClassMemberDeclarationEntry, Declaration, KotlinSyntaxToken,
    RecoveredSeparatedListEntry,
};

use crate::helpers::blocks::source_braced_body;
use crate::helpers::comments::{
    LeadingTrivia, comments_from_tokens, format_removed_comments, format_token_sequence,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};

use super::{
    format_declaration, format_enum_entry_with_separator, format_function_declaration,
    format_initializer_block, format_property_declaration, format_secondary_constructor,
    format_type_alias_declaration,
};

pub(super) fn format_class_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: Option<ClassBody<'source>>,
) -> Doc<'source> {
    let Some(body) = body else {
        return doc.nil();
    };
    let body_doc = format_class_body_contents(doc, &body);
    let space = doc.space();
    let body = source_braced_body(
        doc,
        body.open_brace().as_ref(),
        body.close_brace().as_ref(),
        body_doc,
    );
    doc.concat([space, body])
}

fn format_class_body_contents<'source>(
    doc: &mut DocBuilder<'source>,
    body: &ClassBody<'source>,
) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_class_body_contents_with_ignored(doc, body, &ignored_ranges);
    }

    let sections = class_body_sections_with_recovered_entries(doc, body);

    (!sections.is_empty()).then(|| join_class_body_sections(doc, sections))
}

fn format_class_body_contents_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    body: &ClassBody<'source>,
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Option<Doc<'source>> {
    let body_start = body.text_range().start().get();
    let entries = body
        .member_declaration_entries_with_recovered()
        .collect::<Vec<_>>();
    let entry_ranges = entries
        .iter()
        .map(|entry| recovered_class_member_token_range(doc, entry, body_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &entry_ranges);
    if ignored_runs.is_empty() {
        let sections = class_body_sections_from_recovered_entries(doc, entries);
        return (!sections.is_empty()).then(|| join_class_body_sections(doc, sections));
    }

    let mut sections = Vec::with_capacity(entries.len().saturating_add(ignored_runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    let mut previous_member_had_trailing_comments = false;
    for (entry_index, entry) in entries.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == entry_index
        {
            let run = &ignored_runs[ignored_index];
            sections.push(ClassBodySection {
                doc: formatter_ignore_run_doc(run, doc),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= entry_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(entry_index) {
            if let RecoveredSeparatedListEntry::Entry(member) = &entry {
                previous_member_had_trailing_comments = member
                    .comma
                    .map_or_else(|| member.member.last_token(), Some)
                    .is_some_and(|token| !token.trailing_comments().is_empty());
            }
            continue;
        }

        push_class_body_recovered_entry(
            doc,
            &mut sections,
            entry,
            &mut previous_member_had_trailing_comments,
        );
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ClassBodySection {
            doc: formatter_ignore_run_doc(run, doc),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    (!sections.is_empty()).then(|| join_class_body_sections(doc, sections))
}

fn class_body_sections_with_recovered_entries<'source>(
    doc: &mut DocBuilder<'source>,
    body: &ClassBody<'source>,
) -> Vec<ClassBodySection<'source>> {
    class_body_sections_from_recovered_entries(
        doc,
        body.member_declaration_entries_with_recovered(),
    )
}

fn class_body_sections_from_recovered_entries<'source>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<
        Item = RecoveredSeparatedListEntry<'source, ClassMemberDeclarationEntry<'source>>,
    >,
) -> Vec<ClassBodySection<'source>> {
    let entries = entries.into_iter();
    let (lower, _) = entries.size_hint();
    let mut sections = Vec::with_capacity(lower);
    let mut previous_member_had_trailing_comments = false;

    for entry in entries {
        push_class_body_recovered_entry(
            doc,
            &mut sections,
            entry,
            &mut previous_member_had_trailing_comments,
        );
    }

    sections
}

fn push_class_body_recovered_entry<'source>(
    doc: &mut DocBuilder<'source>,
    sections: &mut Vec<ClassBodySection<'source>>,
    entry: RecoveredSeparatedListEntry<'source, ClassMemberDeclarationEntry<'source>>,
    previous_member_had_trailing_comments: &mut bool,
) {
    match entry {
        RecoveredSeparatedListEntry::Entry(member) => {
            *previous_member_had_trailing_comments = member
                .comma
                .map_or_else(|| member.member.last_token(), Some)
                .is_some_and(|token| !token.trailing_comments().is_empty());
            sections.push(ClassBodySection {
                doc: format_class_member_entry(doc, &member),
                hard_line_after: false,
            });
        }
        RecoveredSeparatedListEntry::Token(token) => {
            let recovered =
                format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve);
            push_recovered_class_body_doc(
                doc,
                sections,
                recovered,
                *previous_member_had_trailing_comments,
            );
        }
        RecoveredSeparatedListEntry::Error(error) => {
            let recovered = format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve);
            push_recovered_class_body_doc(
                doc,
                sections,
                recovered,
                *previous_member_had_trailing_comments,
            );
        }
        RecoveredSeparatedListEntry::Node(node) => {
            let recovered = format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve);
            push_recovered_class_body_doc(
                doc,
                sections,
                recovered,
                *previous_member_had_trailing_comments,
            );
        }
    }
}

fn push_recovered_class_body_doc<'source>(
    doc: &mut DocBuilder<'source>,
    sections: &mut Vec<ClassBodySection<'source>>,
    recovered_doc: Doc<'source>,
    previous_member_had_trailing_comments: bool,
) {
    if previous_member_had_trailing_comments {
        let line = doc.hard_line();
        sections.push(ClassBodySection {
            doc: doc.concat([line, recovered_doc]),
            hard_line_after: false,
        });
    } else if let Some(previous) = sections.last_mut() {
        previous.doc = doc.concat([
            std::mem::replace(&mut previous.doc, doc.nil()),
            recovered_doc,
        ]);
    } else {
        sections.push(ClassBodySection {
            doc: recovered_doc,
            hard_line_after: false,
        });
    }
}

fn format_class_member_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &ClassMemberDeclarationEntry<'source>,
) -> Doc<'source> {
    format_class_member_declaration(doc, &entry.member, entry.comma)
}

fn format_class_member_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    member: &ClassMemberDeclaration<'source>,
    comma: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(declaration) = member.declaration() {
        return match declaration {
            Declaration::EnumEntry(entry) => format_enum_entry_with_separator(doc, &entry, comma),
            Declaration::FunctionDeclaration(declaration) => {
                format_function_declaration(doc, &declaration)
            }
            Declaration::PropertyDeclaration(declaration) => {
                format_property_declaration(doc, &declaration)
            }
            Declaration::TypeAliasDeclaration(declaration) => {
                format_type_alias_declaration(doc, &declaration)
            }
            Declaration::SecondaryConstructor(constructor) => {
                format_secondary_constructor(doc, &constructor)
            }
            Declaration::InitializerBlock(block) => format_initializer_block(doc, &block),
            _ => format_declaration(doc, &declaration),
        };
    }

    if let Some(statement) = member.statement() {
        return crate::rules::statements::format_statement_syntax_with_leading(doc, &statement);
    }

    let comments = comments_from_tokens(member.token_iter());
    if let Some(comments) = format_removed_comments(doc, comments) {
        return comments;
    }

    format_token_sequence(doc, member.token_iter(), LeadingTrivia::Preserve)
}

fn class_member_token_range(
    _doc: &mut DocBuilder<'_>,
    entry: &ClassMemberDeclarationEntry<'_>,
    body_start: usize,
) -> Option<std::ops::Range<usize>> {
    let last_token = entry.comma.or_else(|| entry.member.last_token())?;
    Some(relative_token_range_between(
        &entry.member.first_token()?,
        &last_token,
        body_start,
    ))
}

fn recovered_class_member_token_range(
    doc: &mut DocBuilder<'_>,
    entry: &RecoveredSeparatedListEntry<'_, ClassMemberDeclarationEntry<'_>>,
    body_start: usize,
) -> Option<std::ops::Range<usize>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => {
            class_member_token_range(doc, entry, body_start)
        }
        RecoveredSeparatedListEntry::Token(token) => {
            let range = token.token_text_range();
            Some(range.start().get() - body_start..range.end().get() - body_start)
        }
        RecoveredSeparatedListEntry::Error(error) => Some(relative_token_range_between(
            &error.first_token()?,
            &error.last_token()?,
            body_start,
        )),
        RecoveredSeparatedListEntry::Node(node) => Some(relative_token_range_between(
            &node.first_token()?,
            &node.last_token()?,
            body_start,
        )),
    }
}

fn join_class_body_sections<'source>(
    doc: &mut DocBuilder<'source>,
    sections: Vec<ClassBodySection<'source>>,
) -> Doc<'source> {
    let mut previous_hard_line_after = false;
    doc.concat_list(|joined| {
        for section in sections {
            if !joined.is_empty() {
                let separator = if previous_hard_line_after {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section.doc);
            previous_hard_line_after = section.hard_line_after;
        }
    })
}

struct ClassBodySection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}
