use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitDeclaration, EmptyDeclaration, FieldDeclaration,
    ImportDeclaration, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxView, MethodDeclaration,
    ModuleDeclaration, PackageDeclaration, TypeDeclaration,
};

use crate::helpers::comments::{
    comments_from_tokens, format_comment, format_removed_comments, format_token_with_comments,
    has_removed_comments,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRun, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    token_range_between,
};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_malformed, format_or_verbatim,
    format_required_field, resolve_list_part,
};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::{format_method_declaration, format_type_declaration};
use crate::rules::imports::format_imports;
use crate::rules::modules::format_module_declaration;
use crate::rules::names::format_name;
use crate::rules::variables::format_field_declaration;

pub(crate) fn format_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(unit, doc, |doc| format_valid_compilation_unit(unit, doc))
}

fn format_valid_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = Vec::new();

    match crate::helpers::recovery::resolve_optional_field(unit.package(), doc) {
        JavaFormatField::Present(Some(package)) => entries.push(ProgramEntry::Package(package)),
        JavaFormatField::Present(None) => {}
        JavaFormatField::Malformed(malformed) => entries.push(ProgramEntry::Raw(malformed, None)),
    }
    collect_imports(unit.imports(), &mut entries, doc);
    match crate::helpers::recovery::resolve_optional_field(unit.module(), doc) {
        JavaFormatField::Present(Some(module)) => entries.push(ProgramEntry::Module(module)),
        JavaFormatField::Present(None) => {}
        JavaFormatField::Malformed(malformed) => entries.push(ProgramEntry::Raw(malformed, None)),
    }
    collect_declarations(unit.declarations(), &mut entries, doc);

    let ignored_ranges = formatter_ignore_ranges(
        unit.source_text(),
        unit.text_range().start().get(),
        unit.token_iter(),
    );
    let (contents, ignored_eof_comments) = if ignored_ranges.is_empty() {
        (format_program_entries(entries, doc), Vec::new())
    } else {
        let item_ranges = entries
            .iter()
            .map(ProgramEntry::token_range)
            .collect::<Vec<_>>();
        let runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        let base = unit.text_range().start().get();
        let ignored_eof_comments = runs
            .iter()
            .filter(|run| run.include_on_marker)
            .map(|run| {
                let start = base + run.range.interior.start;
                start..start + run.range.raw_text_with_on.len()
            })
            .collect();
        (
            format_program_entries_with_ignored(entries, &runs, doc),
            ignored_eof_comments,
        )
    };
    let has_source_contents = unit.token_iter().any(|token| !token.text().is_empty());
    let eof = format_required_field(unit.eof(), doc, |token, doc| {
        doc.concat_list(|comments| {
            let mut emitted_comment = false;
            for comment in token.leading_comments().chain(token.trailing_comments()) {
                let range = comment.text_range();
                let range = range.start().get()..range.end().get();
                if ignored_eof_comments.iter().any(|ignored: &Range<usize>| {
                    ignored.start <= range.start && range.end <= ignored.end
                }) {
                    continue;
                }
                if emitted_comment || has_source_contents {
                    let hard_line = comments.hard_line();
                    comments.push(hard_line);
                }
                let comment = format_comment(comments, &comment);
                comments.push(comment);
                emitted_comment = true;
            }
        })
    });
    doc_concat!(doc, [contents, eof, doc.hard_line()])
}

fn collect_imports<'source>(
    field: Result<
        JavaSyntaxField<'source, jolt_java_syntax::ImportDeclarationList<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    entries: &mut Vec<ProgramEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    match crate::helpers::recovery::resolve_required_field(field, doc) {
        JavaFormatField::Present(list) => {
            let parts = list.parts();
            let (lower, _) = parts.size_hint();
            if lower != 0 {
                entries.reserve(lower);
            }
            for part in parts {
                match part {
                    Ok(JavaSyntaxListPart::Item(import)) => {
                        entries.push(ProgramEntry::Import(import));
                    }
                    Ok(JavaSyntaxListPart::Separator(separator)) => {
                        entries.push(ProgramEntry::Raw(
                            crate::helpers::comments::format_token_with_comments(doc, &separator),
                            Some(token_range_between(&separator, &separator)),
                        ));
                    }
                    Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                        let range = token_range(&malformed);
                        entries.push(ProgramEntry::Raw(format_malformed(&malformed, doc), range));
                    }
                    Ok(JavaSyntaxListPart::Missing(missing)) => entries.push(ProgramEntry::Raw(
                        crate::helpers::recovery::format_missing(&missing, doc),
                        None,
                    )),
                    Err(error) => doc.block_on_invariant(error.to_string()),
                }
            }
        }
        JavaFormatField::Malformed(malformed) => entries.push(ProgramEntry::Raw(malformed, None)),
    }
}

fn collect_declarations<'source>(
    field: Result<
        JavaSyntaxField<'source, jolt_java_syntax::CompilationUnitDeclarationList<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    entries: &mut Vec<ProgramEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    match crate::helpers::recovery::resolve_required_field(field, doc) {
        JavaFormatField::Present(list) => {
            let parts = list.parts();
            let (lower, _) = parts.size_hint();
            if lower != 0 {
                entries.reserve(lower);
            }
            for part in parts {
                match part {
                    Ok(JavaSyntaxListPart::Item(item)) => {
                        entries.push(ProgramEntry::Declaration(item));
                    }
                    Ok(JavaSyntaxListPart::Separator(separator)) => {
                        entries.push(ProgramEntry::Raw(
                            crate::helpers::comments::format_token_with_comments(doc, &separator),
                            Some(token_range_between(&separator, &separator)),
                        ));
                    }
                    Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                        let range = token_range(&malformed);
                        entries.push(ProgramEntry::Raw(format_malformed(&malformed, doc), range));
                    }
                    Ok(JavaSyntaxListPart::Missing(missing)) => entries.push(ProgramEntry::Raw(
                        crate::helpers::recovery::format_missing(&missing, doc),
                        None,
                    )),
                    Err(error) => doc.block_on_invariant(error.to_string()),
                }
            }
        }
        JavaFormatField::Malformed(malformed) => entries.push(ProgramEntry::Raw(malformed, None)),
    }
}

enum ProgramEntry<'source> {
    Package(PackageDeclaration<'source>),
    Import(ImportDeclaration<'source>),
    Module(ModuleDeclaration<'source>),
    Declaration(CompilationUnitDeclaration<'source>),
    Raw(Doc<'source>, Option<Range<usize>>),
}

impl ProgramEntry<'_> {
    fn token_range(&self) -> Option<Range<usize>> {
        match self {
            Self::Package(item) => token_range(item),
            Self::Import(item) => token_range(item),
            Self::Module(item) => token_range(item),
            Self::Declaration(item) => {
                let first = item.first_token()?;
                let last = item.last_token()?;
                Some(token_range_between(&first, &last))
            }
            Self::Raw(_, range) => range.clone(),
        }
    }
}

fn token_range<'source>(view: &impl JavaSyntaxView<'source>) -> Option<Range<usize>> {
    let syntax = view.syntax_node()?;
    Some(token_range_between(
        &syntax.first_token()?,
        &syntax.last_token()?,
    ))
}

fn format_program_entries<'source>(
    entries: Vec<ProgramEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::with_capacity(entries.len());
    let mut imports = Vec::new();
    for entry in entries {
        let section = match entry {
            ProgramEntry::Import(import) => {
                imports.push(import);
                continue;
            }
            ProgramEntry::Package(package) => {
                flush_imports(&mut imports, &mut sections, doc);
                (format_package_declaration(&package, doc), true, false)
            }
            ProgramEntry::Module(module) => {
                flush_imports(&mut imports, &mut sections, doc);
                (format_module_declaration(&module, doc), true, false)
            }
            ProgramEntry::Declaration(declaration) => {
                flush_imports(&mut imports, &mut sections, doc);
                let visible = declaration_is_visible(declaration);
                let comment_only =
                    declaration
                        .cast_node::<EmptyDeclaration<'_>>()
                        .is_some_and(|empty| {
                            has_removed_comments(comments_from_tokens(empty.token_iter()))
                        });
                (
                    format_compilation_unit_declaration(declaration, doc),
                    visible,
                    comment_only,
                )
            }
            ProgramEntry::Raw(raw, _) => {
                flush_imports(&mut imports, &mut sections, doc);
                (raw, true, false)
            }
        };
        sections.push(section);
    }
    flush_imports(&mut imports, &mut sections, doc);
    join_program_sections(doc, sections)
}

fn declaration_is_visible(declaration: CompilationUnitDeclaration<'_>) -> bool {
    declaration
        .cast_node::<EmptyDeclaration<'_>>()
        .is_none_or(|empty| has_removed_comments(comments_from_tokens(empty.token_iter())))
}

fn program_entries_are_visible(entries: &[ProgramEntry<'_>]) -> bool {
    entries.iter().any(|entry| match entry {
        ProgramEntry::Declaration(declaration) => declaration_is_visible(*declaration),
        ProgramEntry::Package(_)
        | ProgramEntry::Import(_)
        | ProgramEntry::Module(_)
        | ProgramEntry::Raw(_, _) => true,
    })
}

fn flush_imports<'source>(
    imports: &mut Vec<ImportDeclaration<'source>>,
    sections: &mut Vec<(Doc<'source>, bool, bool)>,
    doc: &mut DocBuilder<'source>,
) {
    if let Some(imports) = format_imports(std::mem::take(imports), doc) {
        sections.push((imports, true, false));
    }
}

fn join_program_sections<'source>(
    doc: &mut DocBuilder<'source>,
    sections: Vec<(Doc<'source>, bool, bool)>,
) -> Doc<'source> {
    let mut saw_visible = false;
    let mut previous_was_comment = false;
    doc.concat_list(|joined| {
        for (section, visible, comment_only) in sections {
            if visible && saw_visible {
                let separator = if previous_was_comment {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section);
            saw_visible |= visible;
            if visible {
                previous_was_comment = comment_only;
            }
        }
    })
}

fn format_program_entries_with_ignored<'source>(
    entries: Vec<ProgramEntry<'source>>,
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections: Vec<(Doc<'source>, bool, bool)> = Vec::new();
    let mut retained = Vec::new();
    let mut run_index = 0;
    let mut skip_index = 0;
    for (index, entry) in entries.into_iter().enumerate() {
        while runs
            .get(run_index)
            .is_some_and(|run| run.insert_index == index)
        {
            if !retained.is_empty() {
                let visible = program_entries_are_visible(&retained);
                sections.push((
                    format_program_entries(std::mem::take(&mut retained), doc),
                    visible,
                    false,
                ));
            }
            sections.push((formatter_ignore_run_doc(&runs[run_index], doc), true, true));
            run_index += 1;
        }
        while runs
            .get(skip_index)
            .is_some_and(|run| run.skip_end <= index)
        {
            skip_index += 1;
        }
        if runs.get(skip_index).is_some_and(|run| run.skips(index)) {
            continue;
        }
        retained.push(entry);
    }
    if !retained.is_empty() {
        let visible = program_entries_are_visible(&retained);
        sections.push((format_program_entries(retained, doc), visible, false));
    }
    while let Some(run) = runs.get(run_index) {
        sections.push((formatter_ignore_run_doc(run, doc), true, true));
        run_index += 1;
    }
    let mut saw_visible = false;
    let mut previous_was_ignored = false;
    doc.concat_list(|joined| {
        for (section, visible, ignored) in sections {
            if visible && saw_visible {
                let separator = if previous_was_ignored {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section);
            saw_visible |= visible;
            previous_was_ignored = ignored;
        }
    })
}

fn format_compilation_unit_declaration<'source>(
    declaration: CompilationUnitDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(declaration) = declaration.cast_family::<TypeDeclaration<'source>>() {
        return format_type_declaration(&declaration, doc);
    }
    if let Some(declaration) = declaration.cast_node::<FieldDeclaration<'source>>() {
        return format_field_declaration(&declaration, doc);
    }
    if let Some(declaration) = declaration.cast_node::<MethodDeclaration<'source>>() {
        return format_method_declaration(&declaration, doc);
    }
    if let Some(declaration) = declaration.cast_node::<EmptyDeclaration<'source>>() {
        return match crate::helpers::recovery::resolve_required_field(declaration.semicolon(), doc)
        {
            JavaFormatField::Present(_) => {
                let removed = declaration
                    .separator_removal_claim()
                    .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
                let comments =
                    format_removed_comments(doc, comments_from_tokens(declaration.token_iter()))
                        .unwrap_or_else(Doc::nil);
                doc_concat!(doc, [removed, comments])
            }
            JavaFormatField::Malformed(recovery) => recovery,
        };
    }
    doc.block_on_invariant("compilation-unit declaration contradicted its declared role");
    Doc::nil()
}

fn format_package_declaration<'source>(
    package: &PackageDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(package, doc, |doc| {
        let annotations = format_required_field(package.annotations(), doc, |list, doc| {
            doc.concat_list(|docs| {
                for part in list.parts() {
                    match resolve_list_part(part, docs) {
                        JavaFormatListPart::Item(annotation) => {
                            if !docs.is_empty() {
                                let line = docs.hard_line();
                                docs.push(line);
                            }
                            let annotation = format_annotation(&annotation, docs);
                            docs.push(annotation);
                        }
                        JavaFormatListPart::Separator(separator) => {
                            let separator = format_token_with_comments(docs, &separator);
                            docs.push(separator);
                        }
                        JavaFormatListPart::Malformed(malformed) => docs.push(malformed),
                    }
                }
            })
        });
        let keyword = format_required_field(package.package_keyword(), doc, |token, doc| {
            doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
        });
        let name = format_required_field(package.name(), doc, |name, doc| format_name(&name, doc));
        let semicolon = format_required_field(package.semicolon(), doc, |token, doc| {
            format_token_with_comments(doc, &token)
        });
        let declaration = doc_concat!(doc, [keyword, name, semicolon]);
        if annotations == Doc::nil() {
            declaration
        } else {
            doc_concat!(doc, [annotations, doc.hard_line(), declaration])
        }
    })
}
