use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder, DocList};
use jolt_java_syntax::{CompilationUnit, CompilationUnitItem, JavaSyntaxKind, PackageDeclaration};

use crate::helpers::blocks::join_empty_lines;
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
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = unit.items_with_recovered().collect::<Vec<_>>();
    let contents = if items.is_empty() || items.iter().all(is_recovered_eof_token) {
        format_comment_only_compilation_unit(unit, doc)
    } else {
        let ignored_ranges = formatter_ignore_ranges(
            unit.source_text(),
            unit.text_range().start().get(),
            unit.token_iter(),
        );
        if ignored_ranges.is_empty() {
            return doc_concat!(
                doc,
                [
                    format_compilation_unit_item_entries(items, doc).unwrap_or_else(Doc::nil),
                    doc.hard_line(),
                ]
            );
        }
        let item_ranges = items
            .iter()
            .map(recovered_compilation_unit_item_token_range)
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        format_compilation_unit_item_entries_with_ignored(items, &ignored_runs, doc)
    };

    doc_concat!(doc, [contents, doc.hard_line()])
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
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::with_capacity(items.len());
    let mut segment = Vec::with_capacity(items.len());
    for item in items {
        match item {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => segment.push(item),
            recovered => {
                push_compilation_unit_segment(&mut sections, &mut segment, doc);
                push_compilation_unit_recovered_section(&mut sections, recovered, doc);
            }
        }
    }
    push_compilation_unit_segment(&mut sections, &mut segment, doc);

    (!sections.is_empty()).then(|| join_program_sections(sections, doc))
}

fn format_compilation_unit_items<'source>(
    items: Vec<CompilationUnitItem<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::with_capacity(4);
    let mut package = None;
    let mut imports = Vec::with_capacity(items.len());
    let mut module = None;
    let mut declarations = doc.list();
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
                    format_type_declaration(&declaration, doc),
                    doc,
                );
            }
            CompilationUnitItem::Field(declaration) => {
                push_declaration_doc(
                    &mut declarations,
                    &mut pending_removed_comments,
                    format_field_declaration(&declaration, doc),
                    doc,
                );
            }
            CompilationUnitItem::Method(declaration) => {
                push_declaration_doc(
                    &mut declarations,
                    &mut pending_removed_comments,
                    format_method_declaration(&declaration, doc),
                    doc,
                );
            }
            CompilationUnitItem::EmptyDeclaration(declaration) => {
                if let Some(comments) =
                    format_removed_comments(doc, comments_from_tokens(declaration.token_iter()))
                {
                    append_pending_removed_comments(&mut pending_removed_comments, comments, doc);
                }
            }
        }
    }

    if let Some(comments) = pending_removed_comments {
        if !declarations.is_empty() {
            declarations.push(doc.empty_line(), doc);
        }
        declarations.push(comments, doc);
    }

    if let Some(package) = package {
        sections.push(format_package_declaration(&package, doc));
    }

    let imports = format_imports(imports, doc);
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = module {
        sections.push(format_module_declaration(&module, doc));
    }

    if !declarations.is_empty() {
        sections.push(declarations.finish(doc));
    }

    (!sections.is_empty()).then(|| join_empty_lines(doc, sections))
}

fn push_declaration_doc<'source>(
    declarations: &mut DocList<'source>,
    pending_removed_comments: &mut Option<Doc<'source>>,
    declaration: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) {
    let declaration = if let Some(comments) = pending_removed_comments.take() {
        doc_concat!(doc, [comments, doc.hard_line(), declaration])
    } else {
        declaration
    };
    if !declarations.is_empty() {
        declarations.push(doc.empty_line(), doc);
    }
    declarations.push(declaration, doc);
}

fn append_pending_removed_comments<'source>(
    pending_removed_comments: &mut Option<Doc<'source>>,
    comments: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) {
    *pending_removed_comments = Some(if let Some(pending) = pending_removed_comments.take() {
        doc_concat!(doc, [pending, doc.hard_line(), comments])
    } else {
        comments
    });
}

fn format_compilation_unit_item_entries_with_ignored<'source>(
    items: Vec<
        jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    >,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::with_capacity(items.len().saturating_add(ignored_runs.len()));
    let mut segment = Vec::with_capacity(items.len());
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_compilation_unit_segment(&mut sections, &mut segment, doc);
            let run = &ignored_runs[ignored_index];
            sections.push(ProgramSection {
                doc: formatter_ignore_run_doc(run, doc),
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
                push_compilation_unit_segment(&mut sections, &mut segment, doc);
                push_compilation_unit_recovered_section(&mut sections, recovered, doc);
            }
        }
    }

    push_compilation_unit_segment(&mut sections, &mut segment, doc);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ProgramSection {
            doc: formatter_ignore_run_doc(run, doc),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_program_sections(sections, doc)
}

fn push_compilation_unit_segment<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    segment: &mut Vec<CompilationUnitItem<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    if segment.is_empty() {
        return;
    }
    let items = std::mem::take(segment);
    if let Some(doc) = format_compilation_unit_items(items, doc) {
        sections.push(ProgramSection {
            doc,
            hard_line_after: false,
        });
    }
}

fn push_compilation_unit_recovered_section<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    item: jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    let is_eof = is_recovered_eof_token(&item);
    let Some(item_doc) = format_recovered_compilation_unit_item(item, doc) else {
        return;
    };
    if is_eof && let Some(previous) = sections.last_mut() {
        previous.hard_line_after = true;
    }
    sections.push(ProgramSection {
        doc: item_doc,
        hard_line_after: false,
    });
}

fn join_program_sections<'source>(
    sections: Vec<ProgramSection<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut joined = doc.list();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            let separator = if previous_hard_line_after {
                doc.hard_line()
            } else {
                doc.empty_line()
            };
            joined.push(separator, doc);
        }
        joined.push(section.doc, doc);
        previous_hard_line_after = section.hard_line_after;
    }
    joined.finish(doc)
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
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    match item {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(_) => None,
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            if token.kind() == JavaSyntaxKind::Eof {
                let mut comments = doc.list();
                for comment in token.leading_comments().chain(token.trailing_comments()) {
                    if !comments.is_empty() {
                        let hard_line = doc.hard_line();
                        comments.push(hard_line, doc);
                    }
                    let comment = format_comment(doc, &comment);
                    comments.push(comment, doc);
                }
                Some(comments.finish(doc))
            } else {
                Some(format_token_sequence(
                    doc,
                    std::iter::once(token),
                    LeadingTrivia::Preserve,
                ))
            }
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(format_token_sequence(
            doc,
            error.token_iter(),
            LeadingTrivia::Preserve,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(format_token_sequence(
            doc,
            node.token_iter(),
            LeadingTrivia::Preserve,
        )),
    }
}

fn format_package_declaration<'source>(
    package: &PackageDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut annotations = doc.list();
    for annotation in package.annotations() {
        if !annotations.is_empty() {
            let hard_line = doc.hard_line();
            annotations.push(hard_line, doc);
        }
        let annotation = format_annotation(&annotation, doc);
        annotations.push(annotation, doc);
    }
    let has_annotations = !annotations.is_empty();

    let package_token = match package.package_token() {
        Some(token) => {
            let token = format_token_with_comments(doc, &token);
            let space = doc.space();
            doc_concat!(doc, [token, space])
        }
        None => Doc::nil(),
    };
    let name = match package.name() {
        Some(name) => format_name(&name, doc),
        None => Doc::nil(),
    };
    let semicolon = match package.semicolon() {
        Some(token) => format_token_with_comments(doc, &token),
        None => Doc::nil(),
    };
    let declaration = doc_concat!(doc, [package_token, name, semicolon]);

    if has_annotations {
        let annotations = annotations.finish(doc);
        let hard_line = doc.hard_line();
        doc_concat!(doc, [annotations, hard_line, declaration])
    } else {
        declaration
    }
}
