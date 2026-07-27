use jolt_fmt_ir::{BodyItemSeparator, Doc, DocBuilder};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, ImportDeclaration, JavaMalformedSyntax,
    JavaMissingSyntax, JavaSyntaxListPart, JavaSyntaxToken, JavaSyntaxView, PackageDeclaration,
};

use crate::helpers::comments::{
    comments_from_tokens, format_comment, format_ignored_trivia, format_token_removal,
    format_token_with_comments, has_removed_comments,
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
        let sections = format_program_sections(entries, doc);
        join_program_sections(doc, sections)
    } else {
        format_program_entries_with_ignored(entries, &runs, doc)
    };
    let has_source_contents = unit.token_iter().any(|token| !token.text().is_empty());
    let eof = format_required_field(unit.eof(), doc, |token, doc| {
        let comments = doc.concat_list(|comments| {
            let mut emitted_comment = false;
            // The gap in front of the first comment belongs to the token's
            // trivia; every later gap belongs to the comment it follows.
            let mut blank_before = token.has_leading_blank_line();
            for comment in token.leading_comments().chain(token.trailing_comments()) {
                let blank_after = comment.is_followed_by_blank_line();
                if formatter_ignore_runs_claim_boundary_comment(&runs, &comment) {
                    blank_before = blank_after;
                    continue;
                }
                if emitted_comment || has_source_contents {
                    let separator = BodyItemSeparator::between(blank_before).doc(comments);
                    comments.push(separator);
                }
                let comment = format_comment(comments, &comment);
                comments.push(comment);
                blank_before = blank_after;
                emitted_comment = true;
            }
        });
        // A boundary, not a break: the file's last item may already have ended
        // its line, for instance with a trailing line comment. Naming the state
        // the end of the file must reach keeps that from stacking a blank line
        // on top of a line end the item itself emitted.
        let line = if unit.is_recovery_free() || token.has_leading_line_break() {
            doc.hard_line_boundary()
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

/// What a top-level section is, for the purpose of spacing it.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ProgramSpacing {
    /// A declaration. Java spaces top-level declarations apart by policy, so
    /// both sides ask for a blank line.
    Declaration,
    /// A verbatim formatter-ignore run, which owns the layout that follows it.
    Ignored,
    /// A stray semicolon that survives only to carry its comments. It reads as
    /// leading trivia of the section it precedes, so it borrows that section's
    /// leading spacing and contributes none of its own after.
    LeadingTrivia,
}

/// One top-level section, with the spacing that surrounds it.
///
/// `spaced_before` and `spaced_after` say whether the declaration spacing
/// policy applies on that side. Where it does not, `blank_before` decides, so
/// the gap the source wrote is the gap that comes back.
struct ProgramSection<'source> {
    doc: Doc<'source>,
    visible: bool,
    spacing: ProgramSpacing,
    spaced_before: bool,
    blank_before: bool,
}

impl<'source> ProgramSection<'source> {
    fn claim_only(doc: Doc<'source>) -> Self {
        Self {
            doc,
            visible: false,
            spacing: ProgramSpacing::Declaration,
            spaced_before: true,
            blank_before: false,
        }
    }

    fn declaration(doc: Doc<'source>, blank_before: bool) -> Self {
        Self {
            doc,
            visible: true,
            spacing: ProgramSpacing::Declaration,
            spaced_before: true,
            blank_before,
        }
    }

    fn salvaged_comments(doc: Doc<'source>, blank_before: bool) -> Self {
        Self {
            doc,
            visible: true,
            spacing: ProgramSpacing::LeadingTrivia,
            // Resolved against the section this one leads.
            spaced_before: false,
            blank_before,
        }
    }

    fn ignored(doc: Doc<'source>) -> Self {
        Self {
            doc,
            visible: true,
            spacing: ProgramSpacing::Ignored,
            spaced_before: true,
            blank_before: false,
        }
    }

    const fn spaced_after(&self) -> bool {
        matches!(self.spacing, ProgramSpacing::Declaration)
    }
}

impl ProgramEntry<'_> {
    fn starts_after_blank_line(&self) -> bool {
        match self {
            Self::Item(item) => item.starts_after_blank_line(),
            Self::Token(token) => token.has_leading_blank_line(),
            Self::Malformed(item) => item.starts_after_blank_line(),
            Self::Missing(_) => false,
        }
    }

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

fn format_program_sections<'source>(
    entries: Vec<ProgramEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Vec<ProgramSection<'source>> {
    let mut sections = Vec::with_capacity(entries.len());
    let mut imports = Vec::new();
    for entry in entries {
        let blank_before = entry.starts_after_blank_line();
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
                let removed =
                    format_program_item(CompilationUnitItem::EmptyDeclaration(empty), doc);
                sections.push(ProgramSection::claim_only(removed));
                continue;
            }
            ProgramEntry::Item(CompilationUnitItem::BogusCompilationUnitItem(bogus)) => {
                flush_imports(&mut imports, &mut sections, doc);
                let visible = bogus.first_token().is_some();
                let recovery = format_malformed(&bogus, doc);
                append_program_recovery(&mut sections, recovery, visible, blank_before, doc);
                continue;
            }
            ProgramEntry::Item(item) => {
                flush_imports(&mut imports, &mut sections, doc);
                let salvaged = matches!(item, CompilationUnitItem::EmptyDeclaration(_));
                let formatted = format_program_item(item, doc);
                if salvaged {
                    ProgramSection::salvaged_comments(formatted, blank_before)
                } else {
                    ProgramSection::declaration(formatted, blank_before)
                }
            }
            ProgramEntry::Token(token) => {
                flush_imports(&mut imports, &mut sections, doc);
                let recovery = format_token_with_comments(doc, &token);
                append_program_recovery(&mut sections, recovery, true, blank_before, doc);
                continue;
            }
            ProgramEntry::Malformed(malformed) => {
                flush_imports(&mut imports, &mut sections, doc);
                let visible = malformed.first_token().is_some();
                let recovery = format_malformed(&malformed, doc);
                append_program_recovery(&mut sections, recovery, visible, blank_before, doc);
                continue;
            }
            ProgramEntry::Missing(missing) => {
                flush_imports(&mut imports, &mut sections, doc);
                ProgramSection::claim_only(format_missing(&missing, doc))
            }
        };
        sections.push(section);
    }
    flush_imports(&mut imports, &mut sections, doc);
    sections
}

fn append_program_recovery<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    recovery: Doc<'source>,
    visible: bool,
    blank_before: bool,
    doc: &mut DocBuilder<'source>,
) {
    if let Some(previous) = sections.last_mut() {
        previous.doc = doc.concat([previous.doc, recovery]);
        if visible {
            // The section now emits syntax of its own, so it is spaced like a
            // declaration rather than like the trivia carrier it started as.
            previous.visible = true;
            previous.spacing = ProgramSpacing::Declaration;
            previous.spaced_before = true;
        }
    } else {
        sections.push(if visible {
            ProgramSection::declaration(recovery, blank_before)
        } else {
            ProgramSection::claim_only(recovery)
        });
    }
}

fn flush_imports<'source>(
    imports: &mut Vec<ImportDeclaration<'source>>,
    sections: &mut Vec<ProgramSection<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    let blank_before = imports
        .first()
        .is_some_and(ImportDeclaration::starts_after_blank_line);
    if let Some(formatted) = format_imports(imports, doc) {
        sections.push(ProgramSection::declaration(formatted, blank_before));
    }
    imports.clear();
}

fn join_program_sections<'source>(
    doc: &mut DocBuilder<'source>,
    mut sections: Vec<ProgramSection<'source>>,
) -> Doc<'source> {
    resolve_leading_trivia_spacing(&mut sections);
    let mut previous_spaced_after = None;
    doc.concat_list(|joined| {
        for section in sections {
            if let Some(spaced_after) = previous_spaced_after
                && section.visible
            {
                // Two declarations take the policy blank line. Where either
                // side opts out, the source's own gap decides.
                let blank = if spaced_after && section.spaced_before {
                    true
                } else {
                    section.blank_before
                };
                let separator = BodyItemSeparator::between(blank).doc(joined);
                joined.push(separator);
            }
            joined.push(section.doc);
            if section.visible {
                previous_spaced_after = Some(section.spaced_after());
            }
        }
    })
}

/// Gives every trivia-carrying section the leading spacing of the section it
/// leads, so a comment salvaged from a stray semicolon is spaced the way the
/// declaration it will read as leading trivia of is spaced. One that leads
/// nothing keeps the source's own gap.
fn resolve_leading_trivia_spacing(sections: &mut [ProgramSection<'_>]) {
    let mut next_spaced_before = false;
    for section in sections.iter_mut().rev() {
        if !section.visible {
            continue;
        }
        if section.spacing == ProgramSpacing::LeadingTrivia {
            section.spaced_before = next_spaced_before;
        }
        next_spaced_before = section.spaced_before;
    }
}

fn format_program_entries_with_ignored<'source>(
    entries: Vec<ProgramEntry<'source>>,
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut retained = Vec::new();
    let mut entries = entries.into_iter().map(Some).collect::<Vec<_>>();
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            flush_retained_program_sections(&mut retained, &mut sections, doc);
            sections.push(ProgramSection::ignored(formatter_ignore_run_doc(run, doc)));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(entry) = entries[index].take() {
                retained.push(entry);
            }
        }
    });
    flush_retained_program_sections(&mut retained, &mut sections, doc);
    join_program_sections(doc, sections)
}

fn flush_retained_program_sections<'source>(
    retained: &mut Vec<ProgramEntry<'source>>,
    sections: &mut Vec<ProgramSection<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    if !retained.is_empty() {
        sections.extend(format_program_sections(std::mem::take(retained), doc));
    }
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
            doc_concat!(doc, [annotations, doc.hard_line_boundary(), declaration])
        } else {
            doc_concat!(doc, [annotations, declaration])
        }
    }
}
