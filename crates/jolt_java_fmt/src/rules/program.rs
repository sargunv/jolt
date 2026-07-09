use jolt_fmt_ir::space;
use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line};
use jolt_java_syntax::{CompilationUnit, CompilationUnitItem, JavaSyntaxKind, PackageDeclaration};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, comments_from_tokens, format_comment, format_removed_comments,
    format_token_sequence, format_token_with_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs, token_range_between,
};
use crate::rules::annotations::format_annotation;
use crate::rules::comments::format_comment_only_compilation_unit;
use crate::rules::declarations::{format_method_declaration, format_type_declaration};
use crate::rules::imports::format_imports;
use crate::rules::modules::format_module_declaration;
use crate::rules::names::format_name;
use crate::rules::variables::format_field_declaration;

pub(crate) fn format_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    formatter: &mut JavaFormatter<'_>,
) -> Doc<'source> {
    let items = unit.items_with_recovered().collect::<Vec<_>>();
    let contents = if items.is_empty() || items.iter().all(is_recovered_eof_token) {
        format_comment_only_compilation_unit(unit)
    } else {
        let ignored_ranges = formatter_ignore_ranges(
            unit.source_text(),
            unit.text_range().start().get(),
            unit.token_iter(),
        );
        if ignored_ranges.is_empty() {
            return concat([
                format_compilation_unit_item_entries(items, formatter)
                    .unwrap_or_else(jolt_fmt_ir::nil),
                hard_line(),
            ]);
        }
        let item_ranges = items
            .iter()
            .map(recovered_compilation_unit_item_token_range)
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        format_compilation_unit_item_entries_with_ignored(items, &ignored_runs, formatter)
    };

    concat([contents, hard_line()])
}

fn is_recovered_eof_token(
    item: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, CompilationUnitItem<'_>>,
) -> bool {
    matches!(
        item,
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token)
            if token.kind() == JavaSyntaxKind::Eof
    )
}

fn format_compilation_unit_item_entries<'source>(
    items: Vec<
        jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    >,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::with_capacity(items.len());
    let mut segment = Vec::with_capacity(items.len());
    for item in items {
        match item {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => segment.push(item),
            recovered => {
                push_compilation_unit_segment(&mut sections, &mut segment, formatter);
                push_compilation_unit_recovered_section(&mut sections, recovered);
            }
        }
    }
    push_compilation_unit_segment(&mut sections, &mut segment, formatter);

    (!sections.is_empty()).then(|| join_program_sections(sections))
}

fn format_compilation_unit_items<'source>(
    items: Vec<CompilationUnitItem<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::with_capacity(4);
    let mut package = None;
    let mut imports = Vec::with_capacity(items.len());
    let mut module = None;
    let mut declarations = Vec::with_capacity(items.len());
    let mut pending_removed_comments = None;

    for item in items {
        match item {
            CompilationUnitItem::Package(declaration) => package = Some(declaration),
            CompilationUnitItem::Import(declaration) => imports.push(declaration),
            CompilationUnitItem::Module(declaration) => module = Some(declaration),
            CompilationUnitItem::Type(declaration) => {
                push_declaration_doc(
                    &mut declarations,
                    &mut pending_removed_comments,
                    format_type_declaration(&declaration, formatter),
                );
            }
            CompilationUnitItem::Field(declaration) => {
                push_declaration_doc(
                    &mut declarations,
                    &mut pending_removed_comments,
                    format_field_declaration(&declaration, formatter),
                );
            }
            CompilationUnitItem::Method(declaration) => {
                push_declaration_doc(
                    &mut declarations,
                    &mut pending_removed_comments,
                    format_method_declaration(&declaration, formatter),
                );
            }
            CompilationUnitItem::EmptyDeclaration(declaration) => {
                if let Some(comments) =
                    format_removed_comments(comments_from_tokens(declaration.token_iter()))
                {
                    append_pending_removed_comments(&mut pending_removed_comments, comments);
                }
            }
        }
    }

    if let Some(comments) = pending_removed_comments {
        declarations.push(comments);
    }

    if let Some(package) = package {
        sections.push(format_package_declaration(&package, formatter));
    }

    let imports = format_imports(imports);
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = module {
        sections.push(format_module_declaration(&module));
    }

    if !declarations.is_empty() {
        sections.push(join_empty_lines(declarations));
    }

    (!sections.is_empty()).then(|| join_empty_lines(sections))
}

fn push_declaration_doc<'source>(
    declarations: &mut Vec<Doc<'source>>,
    pending_removed_comments: &mut Option<Doc<'source>>,
    declaration: Doc<'source>,
) {
    let declaration = if let Some(comments) = pending_removed_comments.take() {
        concat([comments, hard_line(), declaration])
    } else {
        declaration
    };
    declarations.push(declaration);
}

fn append_pending_removed_comments<'source>(
    pending_removed_comments: &mut Option<Doc<'source>>,
    comments: Doc<'source>,
) {
    *pending_removed_comments = Some(if let Some(pending) = pending_removed_comments.take() {
        concat([pending, hard_line(), comments])
    } else {
        comments
    });
}

fn format_compilation_unit_item_entries_with_ignored<'source>(
    items: Vec<
        jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    >,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut sections = Vec::with_capacity(items.len().saturating_add(ignored_runs.len()));
    let mut segment = Vec::with_capacity(items.len());
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_compilation_unit_segment(&mut sections, &mut segment, formatter);
            let run = &ignored_runs[ignored_index];
            sections.push(ProgramSection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= item_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(item_index) {
            continue;
        }

        match item {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => segment.push(item),
            recovered => {
                push_compilation_unit_segment(&mut sections, &mut segment, formatter);
                push_compilation_unit_recovered_section(&mut sections, recovered);
            }
        }
    }

    push_compilation_unit_segment(&mut sections, &mut segment, formatter);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ProgramSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_program_sections(sections)
}

fn push_compilation_unit_segment<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    segment: &mut Vec<CompilationUnitItem<'source>>,
    formatter: &JavaFormatter<'_>,
) {
    if segment.is_empty() {
        return;
    }
    let items = std::mem::take(segment);
    if let Some(doc) = format_compilation_unit_items(items, formatter) {
        sections.push(ProgramSection {
            doc,
            hard_line_after: false,
        });
    }
}

fn push_compilation_unit_recovered_section<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    item: jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
) {
    let is_eof = is_recovered_eof_token(&item);
    let Some(doc) = format_recovered_compilation_unit_item(item) else {
        return;
    };
    if is_eof && let Some(previous) = sections.last_mut() {
        previous.hard_line_after = true;
    }
    sections.push(ProgramSection {
        doc,
        hard_line_after: false,
    });
}

fn join_program_sections(sections: Vec<ProgramSection<'_>>) -> Doc<'_> {
    let mut joined = Vec::with_capacity(sections.len().saturating_mul(2).saturating_sub(1));
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

struct ProgramSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn compilation_unit_item_token_range(item: &CompilationUnitItem<'_>) -> Option<Range<usize>> {
    Some(token_range_between(
        &item.first_token()?,
        &item.last_token()?,
    ))
}

fn recovered_compilation_unit_item_token_range(
    item: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, CompilationUnitItem<'_>>,
) -> Option<Range<usize>> {
    match item {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => {
            compilation_unit_item_token_range(item)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(token_range_between(token, token))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(token_range_between(
            &error.first_token()?,
            &error.last_token()?,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(token_range_between(
            &node.first_token()?,
            &node.last_token()?,
        )),
    }
}

fn format_recovered_compilation_unit_item<'source>(
    item: jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
) -> Option<Doc<'source>> {
    match item {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(_) => None,
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            if token.kind() == JavaSyntaxKind::Eof {
                Some(join_hard_lines(
                    token
                        .leading_comments()
                        .chain(token.trailing_comments())
                        .map(|comment| format_comment(&comment)),
                ))
            } else {
                Some(format_token_sequence(
                    std::iter::once(token),
                    LeadingTrivia::Preserve,
                ))
            }
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(format_token_sequence(
            error.token_iter(),
            LeadingTrivia::Preserve,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(format_token_sequence(
            node.token_iter(),
            LeadingTrivia::Preserve,
        )),
    }
}

fn format_package_declaration<'source>(
    package: &PackageDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut annotations = package
        .annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .peekable();
    let declaration = concat([
        package
            .package_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(&token), space()])
            }),
        package
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        package
            .semicolon()
            .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
    ]);

    if annotations.peek().is_none() {
        declaration
    } else {
        concat([join_hard_lines(annotations), hard_line(), declaration])
    }
}
