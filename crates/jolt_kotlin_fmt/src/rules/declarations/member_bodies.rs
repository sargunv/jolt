use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, space};
use jolt_kotlin_syntax::{
    ClassBody, ClassMemberDeclaration, ClassMemberDeclarationEntry, Declaration, KotlinSyntaxKind,
    KotlinSyntaxToken,
};

use crate::helpers::blocks::{join_empty_lines, source_braced_body};
use crate::helpers::comments::{
    LeadingTrivia, comments_from_tokens, format_dangling_comments, format_removed_comments,
    format_token_sequence, has_removed_comments,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::source::source_gap_is_trivia;

use super::{
    format_declaration, format_enum_entry_with_separator, format_function_declaration,
    format_initializer_block, format_property_declaration, format_secondary_constructor,
    format_type_alias_declaration,
};

pub(super) fn format_class_body(body: Option<ClassBody<'_>>) -> Doc<'_> {
    let Some(body) = body else {
        return jolt_fmt_ir::nil();
    };
    let body_doc = format_class_body_contents(&body);
    concat([
        space(),
        source_braced_body(
            body.open_brace().as_ref(),
            body.close_brace().as_ref(),
            body_doc,
        ),
    ])
}

fn format_class_body_contents<'source>(body: &ClassBody<'source>) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_class_body_contents_with_ignored(body, &ignored_ranges);
    }

    let members = body.member_declaration_entries().collect::<Vec<_>>();
    let sections = class_body_sections_with_recovered_tokens(body, &members);

    (!sections.is_empty()).then(|| join_class_body_sections(sections))
}

fn format_class_body_contents_with_ignored<'source>(
    body: &ClassBody<'source>,
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Option<Doc<'source>> {
    let body_start = body.text_range().start().get();
    let members = body.member_declaration_entries().collect::<Vec<_>>();
    let member_ranges = members
        .iter()
        .map(|entry| class_member_token_range(entry, body_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &member_ranges);
    if ignored_runs.is_empty() {
        let docs = members
            .iter()
            .map(format_class_member_entry)
            .collect::<Vec<_>>();
        return (!docs.is_empty()).then(|| join_empty_lines(docs));
    }

    let mut sections = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (member_index, member) in members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            sections.push(ClassBodySection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= member_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(member_index) {
            continue;
        }

        sections.push(ClassBodySection {
            doc: format_class_member_entry(member),
            hard_line_after: false,
        });
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ClassBodySection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    (!sections.is_empty()).then(|| join_class_body_sections(sections))
}

fn class_body_sections_with_recovered_tokens<'source>(
    body: &ClassBody<'source>,
    members: &[ClassMemberDeclarationEntry<'source>],
) -> Vec<ClassBodySection<'source>> {
    let body_start = body.text_range().start().get();
    let body_end = body.close_brace().map_or_else(
        || body.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    let mut cursor = body.open_brace().map_or_else(
        || body.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let tokens = body.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut sections = Vec::new();

    for member in members {
        push_recovered_class_body_gap(
            &mut sections,
            body.source_text(),
            body_start,
            &tokens,
            &mut token_cursor,
            cursor,
            member.member.text_range().start().get(),
        );
        sections.push(ClassBodySection {
            doc: format_class_member_entry(member),
            hard_line_after: false,
        });
        cursor = member.comma.map_or_else(
            || member.member.text_range().end().get(),
            |comma| comma.token_text_range().end().get(),
        );
    }

    push_recovered_class_body_gap(
        &mut sections,
        body.source_text(),
        body_start,
        &tokens,
        &mut token_cursor,
        cursor,
        body_end,
    );

    sections
}

fn push_recovered_class_body_gap<'source>(
    sections: &mut Vec<ClassBodySection<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return;
    }

    let mut gap_tokens = Vec::new();
    let mut previous_token = None;
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            previous_token = Some(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return;
    }

    if recovered_gap_is_enum_separator(&gap_tokens)
        && let Some(previous) = sections.last_mut()
    {
        previous.doc = concat([
            std::mem::replace(&mut previous.doc, jolt_fmt_ir::nil()),
            format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        ]);
        return;
    }

    let attached_comments = previous_token
        .into_iter()
        .flat_map(|token| token.trailing_comments())
        .collect::<Vec<_>>();
    let token_doc = format_token_sequence(gap_tokens, LeadingTrivia::Preserve);
    let doc = if attached_comments.is_empty() {
        token_doc
    } else {
        concat([
            format_dangling_comments(attached_comments),
            hard_line(),
            token_doc,
        ])
    };

    sections.push(ClassBodySection {
        doc,
        hard_line_after: false,
    });
}

fn recovered_gap_is_enum_separator(tokens: &[KotlinSyntaxToken<'_>]) -> bool {
    tokens.iter().all(|token| {
        matches!(
            token.kind(),
            KotlinSyntaxKind::Semicolon | KotlinSyntaxKind::DoubleSemicolon
        )
    })
}

fn format_class_member_entry<'source>(
    entry: &ClassMemberDeclarationEntry<'source>,
) -> Doc<'source> {
    format_class_member_declaration(&entry.member, entry.comma)
}

fn format_class_member_declaration<'source>(
    member: &ClassMemberDeclaration<'source>,
    comma: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    member.declaration().map_or_else(
        || {
            member.statement().map_or_else(
                || {
                    let comments = comments_from_tokens(member.token_iter()).collect::<Vec<_>>();
                    if has_removed_comments(comments.iter().copied()) {
                        return format_removed_comments(comments).unwrap_or_else(jolt_fmt_ir::nil);
                    }

                    format_token_sequence(member.token_iter(), LeadingTrivia::Preserve)
                },
                |statement| {
                    crate::rules::statements::format_statement_syntax_with_leading(&statement)
                },
            )
        },
        |declaration| match declaration {
            Declaration::EnumEntry(entry) => format_enum_entry_with_separator(&entry, comma),
            Declaration::FunctionDeclaration(declaration) => {
                format_function_declaration(&declaration)
            }
            Declaration::PropertyDeclaration(declaration) => {
                format_property_declaration(&declaration)
            }
            Declaration::TypeAliasDeclaration(declaration) => {
                format_type_alias_declaration(&declaration)
            }
            Declaration::SecondaryConstructor(constructor) => {
                format_secondary_constructor(&constructor)
            }
            Declaration::InitializerBlock(block) => format_initializer_block(&block),
            _ => format_declaration(&declaration),
        },
    )
}

fn class_member_token_range(
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

fn join_class_body_sections(sections: Vec<ClassBodySection<'_>>) -> Doc<'_> {
    let mut joined = Vec::new();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            joined.push(if previous_hard_line_after {
                hard_line()
            } else {
                empty_line()
            });
        }
        joined.push(section.doc);
        previous_hard_line_after = section.hard_line_after;
    }
    concat(joined)
}

struct ClassBodySection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}
