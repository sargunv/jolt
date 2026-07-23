use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, ImportDeclaration, JavaMalformedSyntax,
    JavaMissingSyntax, JavaSyntaxListPart, JavaSyntaxToken, JavaSyntaxView, PackageDeclaration,
};

use crate::helpers::comments::{
    comments_from_tokens, format_comment, format_ignored_trivia, format_token_removal,
    format_token_with_comments, has_removed_comments, trailing_comments_force_line,
};
use crate::helpers::recovery::{
    JavaFormatField, format_malformed, format_missing, format_required_field,
    resolve_required_field,
};
use crate::rules::annotations::format_required_annotation_lines;
use crate::rules::declarations::{format_method_declaration, format_type_declaration};
use crate::rules::imports::format_imports;
use crate::rules::modules::format_module_declaration;
use crate::rules::names::format_name;
use crate::rules::variables::format_field_declaration;
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_run_doc,
    formatter_ignore_runs_claim_boundary_comment,
};

pub(crate) fn format_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = Vec::new();
    match unit.items() {
        jolt_java_syntax::JavaSyntaxField::Present(items) => {
            let parts = items.parts();
            entries.reserve(parts.size_hint().0);
            for part in parts {
                match part {
                    JavaSyntaxListPart::Item(item) => entries.push(ProgramEntry::Item(item)),
                    JavaSyntaxListPart::Separator(token) => {
                        entries.push(ProgramEntry::Token(token));
                    }
                    JavaSyntaxListPart::Malformed(malformed) => {
                        entries.push(ProgramEntry::Malformed(malformed));
                    }
                    JavaSyntaxListPart::Missing(missing) => {
                        entries.push(ProgramEntry::Missing(missing));
                    }
                }
            }
        }
        jolt_java_syntax::JavaSyntaxField::Malformed(malformed) => {
            entries.push(ProgramEntry::Malformed(malformed));
        }
        jolt_java_syntax::JavaSyntaxField::Missing(missing) => {
            entries.push(ProgramEntry::Missing(missing));
        }
    }

    let container = unit.text_range();
    let runs = doc.formatter_ignore_runs(container, entries.iter().map(ProgramEntry::ignore_range));
    let contents = if runs.is_empty() {
        format_program_entries(entries, doc)
    } else {
        format_program_entries_with_ignored(entries, &runs, doc)
    };
    let has_source_contents = unit.token_iter().any(|token| !token.text().is_empty());
    let last_source_token_forces_line = unit
        .token_iter()
        .filter(|token| !token.text().is_empty())
        .last()
        .is_some_and(|token| trailing_comments_force_line(&token));
    let eof = format_required_field(unit.eof(), doc, |token, doc| {
        let comments = doc.concat_list(|comments| {
            let mut emitted_comment = false;
            for comment in token.leading_comments().chain(token.trailing_comments()) {
                if formatter_ignore_runs_claim_boundary_comment(&runs, &comment) {
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
        });
        let line = if last_source_token_forces_line {
            Doc::nil()
        } else if unit.is_recovery_free() || token.has_leading_line_break() {
            doc.hard_line()
        } else {
            Doc::nil()
        };
        let ignored = format_ignored_trivia(doc, &token);
        doc.concat([comments, line, ignored])
    });
    doc.concat([contents, eof])
}

enum ProgramEntry<'source> {
    Item(CompilationUnitItem<'source>),
    Token(JavaSyntaxToken<'source>),
    Malformed(JavaMalformedSyntax<'source>),
    Missing(JavaMissingSyntax<'source>),
}

impl ProgramEntry<'_> {
    fn ignore_range(&self) -> Option<FormatterIgnoreItemRange> {
        match self {
            Self::Item(item) => {
                let first = item.first_token()?;
                let last = item.last_token()?;
                Some(FormatterIgnoreItemRange::between(&first, &last))
            }
            Self::Token(token) => Some(FormatterIgnoreItemRange::between(token, token)),
            Self::Malformed(item) => ignore_range(item),
            Self::Missing(_) => None,
        }
    }
}

fn ignore_range<'source>(view: &impl JavaSyntaxView<'source>) -> Option<FormatterIgnoreItemRange> {
    let syntax = view.syntax_node()?;
    Some(FormatterIgnoreItemRange::between(
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
            ProgramEntry::Item(CompilationUnitItem::ImportDeclaration(import)) => {
                imports.push(import);
                continue;
            }
            ProgramEntry::Item(CompilationUnitItem::EmptyDeclaration(empty))
                if !has_removed_comments(comments_from_tokens(empty.token_iter())) =>
            {
                // A commentless top-level semicolon is canonically removed, so
                // it is transparent to the surrounding import sorting run.
                // Keep its removed-source claim without making layout depend
                // on a token that produces no output.
                sections.push((
                    format_program_item(CompilationUnitItem::EmptyDeclaration(empty), doc),
                    false,
                    false,
                ));
                continue;
            }
            ProgramEntry::Item(CompilationUnitItem::BogusCompilationUnitItem(bogus)) => {
                flush_imports(&mut imports, &mut sections, doc);
                let recovery = format_malformed(&bogus, doc);
                append_program_recovery(&mut sections, recovery, doc);
                continue;
            }
            ProgramEntry::Item(item) => {
                flush_imports(&mut imports, &mut sections, doc);
                let visible = program_item_is_visible(&item);
                let comment_only = program_item_is_comment_only(&item);
                (format_program_item(item, doc), visible, comment_only)
            }
            ProgramEntry::Token(token) => {
                flush_imports(&mut imports, &mut sections, doc);
                let recovery = format_token_with_comments(doc, &token);
                append_program_recovery(&mut sections, recovery, doc);
                continue;
            }
            ProgramEntry::Malformed(malformed) => {
                flush_imports(&mut imports, &mut sections, doc);
                let recovery = format_malformed(&malformed, doc);
                append_program_recovery(&mut sections, recovery, doc);
                continue;
            }
            ProgramEntry::Missing(missing) => {
                flush_imports(&mut imports, &mut sections, doc);
                (format_missing(&missing, doc), false, false)
            }
        };
        sections.push(section);
    }
    flush_imports(&mut imports, &mut sections, doc);
    join_program_sections(doc, sections)
}

fn append_program_recovery<'source>(
    sections: &mut Vec<(Doc<'source>, bool, bool)>,
    recovery: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) {
    if let Some((previous, visible, comment_only)) = sections.last_mut() {
        *previous = doc.concat([*previous, recovery]);
        *visible = true;
        *comment_only = false;
    } else {
        sections.push((recovery, true, false));
    }
}

fn program_item_is_visible(item: &CompilationUnitItem<'_>) -> bool {
    match item {
        CompilationUnitItem::EmptyDeclaration(empty) => {
            has_removed_comments(comments_from_tokens(empty.token_iter()))
        }
        _ => true,
    }
}

fn program_item_is_comment_only(item: &CompilationUnitItem<'_>) -> bool {
    matches!(item, CompilationUnitItem::EmptyDeclaration(empty) if has_removed_comments(comments_from_tokens(empty.token_iter())))
}

fn program_entries_are_visible(entries: &[ProgramEntry<'_>]) -> bool {
    entries.iter().any(|entry| match entry {
        ProgramEntry::Item(item) => program_item_is_visible(item),
        ProgramEntry::Token(_) | ProgramEntry::Malformed(_) => true,
        ProgramEntry::Missing(_) => false,
    })
}

fn flush_imports<'source>(
    imports: &mut Vec<ImportDeclaration<'source>>,
    sections: &mut Vec<(Doc<'source>, bool, bool)>,
    doc: &mut DocBuilder<'source>,
) {
    if let Some(formatted) = format_imports(imports, doc) {
        sections.push((formatted, true, false));
    }
    imports.clear();
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
    let mut entries = entries.into_iter().map(Some).collect::<Vec<_>>();
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            if !retained.is_empty() {
                let visible = program_entries_are_visible(&retained);
                sections.push((
                    format_program_entries(std::mem::take(&mut retained), doc),
                    visible,
                    false,
                ));
            }
            sections.push((formatter_ignore_run_doc(run, doc), true, true));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(entry) = entries[index].take() {
                retained.push(entry);
            }
        }
    });
    if !retained.is_empty() {
        let visible = program_entries_are_visible(&retained);
        sections.push((format_program_entries(retained, doc), visible, false));
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

fn format_program_item<'source>(
    item: CompilationUnitItem<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match item {
        CompilationUnitItem::PackageDeclaration(package) => {
            format_package_declaration(&package, doc)
        }
        CompilationUnitItem::ImportDeclaration(import) => {
            doc.block_on_invariant("import bypassed its compilation-unit sorting run");
            crate::rules::imports::format_imports(&[import], doc).unwrap_or_else(Doc::nil)
        }
        CompilationUnitItem::ModuleDeclaration(module) => format_module_declaration(&module, doc),
        CompilationUnitItem::ClassDeclaration(declaration) => {
            format_type_declaration(&declaration.into(), doc)
        }
        CompilationUnitItem::RecordDeclaration(declaration) => {
            format_type_declaration(&declaration.into(), doc)
        }
        CompilationUnitItem::EnumDeclaration(declaration) => {
            format_type_declaration(&declaration.into(), doc)
        }
        CompilationUnitItem::InterfaceDeclaration(declaration) => {
            format_type_declaration(&declaration.into(), doc)
        }
        CompilationUnitItem::AnnotationInterfaceDeclaration(declaration) => {
            format_type_declaration(&declaration.into(), doc)
        }
        CompilationUnitItem::FieldDeclaration(declaration) => {
            format_field_declaration(&declaration, doc)
        }
        CompilationUnitItem::MethodDeclaration(declaration) => {
            format_method_declaration(&declaration, doc)
        }
        CompilationUnitItem::EmptyDeclaration(declaration) => {
            match resolve_required_field(declaration.semicolon(), doc) {
                JavaFormatField::Present(semicolon) => {
                    let (normalized, _) = format_token_removal(
                        doc,
                        &semicolon,
                        declaration.separator_removal_claim(),
                    );
                    normalized
                }
                JavaFormatField::Malformed(recovery) => recovery,
            }
        }
        CompilationUnitItem::BogusCompilationUnitItem(bogus) => format_malformed(&bogus, doc),
    }
}

fn format_package_declaration<'source>(
    package: &PackageDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let (annotations, annotations_visible) =
            format_required_annotation_lines(package.annotations(), doc);
        let keyword = format_required_field(package.package_keyword(), doc, |token, doc| {
            doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
        });
        let name = format_required_field(package.name(), doc, |name, doc| format_name(&name, doc));
        let semicolon = format_required_field(package.semicolon(), doc, |token, doc| {
            format_token_with_comments(doc, &token)
        });
        let declaration = doc_concat!(doc, [keyword, name, semicolon]);
        if annotations_visible {
            doc_concat!(doc, [annotations, doc.hard_line(), declaration])
        } else {
            doc_concat!(doc, [annotations, declaration])
        }
    }
}
