use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ImportDirective, KotlinFamily, KotlinFile, KotlinFileItem, KotlinRoleElement,
    KotlinSyntaxField, KotlinSyntaxView, PackageHeader, StatementSyntax,
};
use jolt_syntax::tokens_have_blank_line_between;

use crate::helpers::blocks::{BodyItemSeparator, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_removed_separator,
    format_terminator_list, format_token, token_has_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_malformed, format_or_verbatim,
    format_required_field, resolve_list_part, resolve_optional_field, resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::{format_file_item, format_fun_interface_file_items};
use crate::rules::imports::format_imports;
use crate::rules::names::format_qualified_name;
use crate::rules::statements::format_statement_syntax_with_leading;

pub(crate) fn format_file<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(file, doc, |doc| format_valid_file(file, doc))
}

fn format_valid_file<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_file_annotations(doc, file);
    let mut entries = Vec::new();

    match resolve_optional_field(file.package_header(), doc) {
        KotlinFormatField::Present(Some(package)) => entries.push(FileEntry::Package(package)),
        KotlinFormatField::Present(None) => {}
        KotlinFormatField::Malformed(recovery) => entries.push(FileEntry::Raw(recovery, None)),
    }
    collect_imports(file, doc, &mut entries);
    collect_items(file, doc, &mut entries);

    let (contents, ignored_eof_comments) = format_entries_with_ignored(file, doc, entries);
    let has_source_contents = file.token_iter().any(|token| !token.text().is_empty());
    let eof = format_required_field(file.eof(), doc, |token, doc| {
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
    let contents = match (annotations, contents) {
        (Some(annotations), Some(contents)) => {
            let line = doc.empty_line();
            doc.concat([annotations, line, contents])
        }
        (Some(annotations), None) => annotations,
        (None, Some(contents)) => contents,
        (None, None) => Doc::nil(),
    };
    let line = doc.hard_line();
    doc.concat([contents, eof, line])
}

fn collect_imports<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
    entries: &mut Vec<FileEntry<'source>>,
) {
    let directives = match resolve_required_field(file.import_list(), doc) {
        KotlinFormatField::Present(directives) => directives,
        KotlinFormatField::Malformed(recovery) => {
            entries.push(FileEntry::Raw(recovery, None));
            return;
        }
    };
    for part in directives.parts() {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(import) => entries.push(FileEntry::Import(import)),
            KotlinFormatListPart::Separator(separator) => entries.push(FileEntry::Raw(
                format_token(
                    doc,
                    &separator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                Some(token_range(&separator, file.text_range().start().get())),
            )),
            KotlinFormatListPart::Malformed(recovery) => {
                entries.push(FileEntry::Raw(recovery, None));
            }
        }
    }
}

fn collect_items<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
    entries: &mut Vec<FileEntry<'source>>,
) {
    let items = match resolve_required_field(file.items(), doc) {
        KotlinFormatField::Present(items) => items,
        KotlinFormatField::Malformed(recovery) => {
            entries.push(FileEntry::Raw(recovery, None));
            return;
        }
    };
    for part in items.parts() {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(KotlinRoleElement::Node(node)) => {
                if let Some(item) = KotlinFileItem::cast(node) {
                    entries.push(FileEntry::Item(item));
                } else {
                    doc.block_on_invariant("invalid Kotlin file item node");
                }
            }
            KotlinFormatListPart::Item(KotlinRoleElement::Token(token))
            | KotlinFormatListPart::Separator(token) => {
                let separator = format_removed_separator(
                    doc,
                    &token,
                    items.separator_removal_claim(token),
                    false,
                );
                entries.push(FileEntry::Separator(
                    separator,
                    Some(token_range(&token, file.text_range().start().get())),
                    token_has_comments(&token),
                ));
            }
            KotlinFormatListPart::Malformed(recovery) => {
                entries.push(FileEntry::Raw(recovery, None));
            }
        }
    }
}

enum FileEntry<'source> {
    Package(PackageHeader<'source>),
    Import(ImportDirective<'source>),
    Item(KotlinFileItem<'source>),
    Raw(Doc<'source>, Option<Range<usize>>),
    Separator(Doc<'source>, Option<Range<usize>>, bool),
}

impl FileEntry<'_> {
    fn token_range(&self, base: usize) -> Option<Range<usize>> {
        match self {
            Self::Package(item) => view_token_range(item, base),
            Self::Import(item) => view_token_range(item, base),
            Self::Item(item) => {
                let first = item.first_token()?;
                let last = item.last_token()?;
                Some(relative_token_range_between(&first, &last, base))
            }
            Self::Raw(_, range) | Self::Separator(_, range, _) => range.clone(),
        }
    }
}

fn format_entries_with_ignored<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
    entries: Vec<FileEntry<'source>>,
) -> (Option<Doc<'source>>, Vec<Range<usize>>) {
    let base = file.text_range().start().get();
    let ignored = formatter_ignore_ranges(file.source_text(), base, file.token_iter());
    if ignored.is_empty() {
        return (
            format_entry_segment(doc, entries).map(|section| section.doc),
            Vec::new(),
        );
    }
    let ranges = entries
        .iter()
        .map(|entry| entry.token_range(base))
        .collect::<Vec<_>>();
    let runs = formatter_ignore_runs(&ignored, &ranges);
    if runs.is_empty() {
        return (
            format_entry_segment(doc, entries).map(|section| section.doc),
            Vec::new(),
        );
    }
    let ignored_eof_comments = runs
        .iter()
        .filter(|run| run.include_on_marker)
        .map(|run| {
            let start = base + run.range.interior.start;
            start..start + run.range.raw_text_with_on.len()
        })
        .collect();

    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut run_index = 0;
    let mut skip_index = 0;
    for (index, entry) in entries.into_iter().enumerate() {
        while run_index < runs.len() && runs[run_index].insert_index == index {
            push_entry_segment(doc, &mut sections, &mut segment);
            sections.push(FileSection::ignored(formatter_ignore_run_doc(
                &runs[run_index],
                doc,
            )));
            run_index += 1;
        }
        while skip_index < runs.len() && runs[skip_index].skip_end <= index {
            skip_index += 1;
        }
        if skip_index < runs.len() && runs[skip_index].skips(index) {
            continue;
        }
        segment.push(entry);
    }
    push_entry_segment(doc, &mut sections, &mut segment);
    while run_index < runs.len() {
        sections.push(FileSection::ignored(formatter_ignore_run_doc(
            &runs[run_index],
            doc,
        )));
        run_index += 1;
    }
    (
        (!sections.is_empty()).then(|| join_sections(doc, sections)),
        ignored_eof_comments,
    )
}

fn push_entry_segment<'source>(
    doc: &mut DocBuilder<'source>,
    sections: &mut Vec<FileSection<'source>>,
    segment: &mut Vec<FileEntry<'source>>,
) {
    if let Some(section) = format_entry_segment(doc, std::mem::take(segment)) {
        sections.push(section);
    }
}

fn format_entry_segment<'source>(
    doc: &mut DocBuilder<'source>,
    entries: Vec<FileEntry<'source>>,
) -> Option<FileSection<'source>> {
    let mut sections = Vec::new();
    let mut imports = Vec::new();
    let mut body = Vec::new();
    for entry in entries {
        match entry {
            FileEntry::Import(import) => {
                flush_body(doc, &mut body, &mut sections);
                imports.push(import);
            }
            FileEntry::Package(package) => {
                flush_imports(doc, &mut imports, &mut sections);
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(format_package_header(doc, &package)));
            }
            FileEntry::Item(item) => {
                flush_imports(doc, &mut imports, &mut sections);
                body.push(item);
            }
            FileEntry::Raw(raw, _) => {
                flush_imports(doc, &mut imports, &mut sections);
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(raw));
            }
            FileEntry::Separator(separator, _, visible) => {
                flush_imports(doc, &mut imports, &mut sections);
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection {
                    doc: separator,
                    visible,
                    ignored: false,
                });
            }
        }
    }
    flush_imports(doc, &mut imports, &mut sections);
    flush_body(doc, &mut body, &mut sections);
    if sections.is_empty() {
        None
    } else {
        let visible = sections.iter().any(|section| section.visible);
        Some(FileSection {
            doc: join_sections(doc, sections),
            visible,
            ignored: false,
        })
    }
}

fn flush_imports<'source>(
    doc: &mut DocBuilder<'source>,
    imports: &mut Vec<ImportDirective<'source>>,
    sections: &mut Vec<FileSection<'source>>,
) {
    if let Some(imports) = format_imports(doc, std::mem::take(imports)) {
        sections.push(FileSection::visible(imports));
    }
}

fn flush_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: &mut Vec<KotlinFileItem<'source>>,
    sections: &mut Vec<FileSection<'source>>,
) {
    if !body.is_empty() {
        let formatted = format_source_body(doc, body);
        body.clear();
        sections.push(FileSection::visible(formatted));
    }
}

fn join_sections<'source>(
    doc: &mut DocBuilder<'source>,
    sections: Vec<FileSection<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        let mut has_visible_section = false;
        let mut previous_was_ignored = false;
        for section in sections {
            if section.visible && has_visible_section {
                let line = if previous_was_ignored {
                    docs.hard_line()
                } else {
                    docs.empty_line()
                };
                docs.push(line);
            }
            docs.push(section.doc);
            has_visible_section |= section.visible;
            previous_was_ignored = section.ignored;
        }
    })
}

struct FileSection<'source> {
    doc: Doc<'source>,
    visible: bool,
    ignored: bool,
}

impl<'source> FileSection<'source> {
    fn visible(doc: Doc<'source>) -> Self {
        Self {
            doc,
            visible: true,
            ignored: false,
        }
    }

    fn ignored(doc: Doc<'source>) -> Self {
        Self {
            doc,
            visible: true,
            ignored: true,
        }
    }
}

fn format_file_annotations<'source>(
    doc: &mut DocBuilder<'source>,
    file: &KotlinFile<'source>,
) -> Option<Doc<'source>> {
    let KotlinFormatField::Present(annotations) = resolve_required_field(file.annotations(), doc)
    else {
        if let KotlinFormatField::Malformed(recovery) =
            resolve_required_field(file.annotations(), doc)
        {
            return Some(recovery);
        }
        return None;
    };
    let mut formatted = Vec::new();
    for part in annotations.parts() {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(annotation) => {
                formatted.push(format_annotation(doc, &annotation));
            }
            KotlinFormatListPart::Separator(separator) => formatted.push(format_token(
                doc,
                &separator,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )),
            KotlinFormatListPart::Malformed(recovery) => formatted.push(recovery),
        }
    }
    (!formatted.is_empty()).then(|| join_hard_lines(doc, formatted))
}

fn format_source_body<'source>(
    doc: &mut DocBuilder<'source>,
    items: &[KotlinFileItem<'source>],
) -> Doc<'source> {
    doc.concat_list(|body| {
        let mut index = 0;
        while let Some(item) = items.get(index) {
            if index > 0 {
                let previous = &items[index - 1];
                let preserve = is_statement_item(previous) || is_statement_item(item);
                let separator = source_item_separator(previous, item, preserve).doc(body);
                body.push(separator);
            }
            if let (
                KotlinFileItem::FunctionDeclaration(function),
                Some(KotlinFileItem::InterfaceDeclaration(interface)),
            ) = (item, items.get(index + 1))
                && is_fun_interface_header(function)
                && let Some(combined) = format_fun_interface_file_items(body, function, interface)
            {
                body.push(combined);
                index += 2;
                continue;
            }
            let formatted = format_body_item(body, item);
            body.push(formatted);
            index += 1;
        }
    })
}

fn is_fun_interface_header(function: &jolt_kotlin_syntax::FunctionDeclaration<'_>) -> bool {
    function.is_recovery_free()
        && matches!(function.fun_token(), Ok(KotlinSyntaxField::Present(_)))
        && matches!(function.context(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(
            function.type_parameters(),
            Ok(KotlinSyntaxField::Missing(_))
        )
        && matches!(function.name(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.parameters(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.return_colon(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.return_type(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.constraints(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.assign(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.body(), Ok(KotlinSyntaxField::Missing(_)))
}

fn is_statement_item(item: &KotlinFileItem<'_>) -> bool {
    matches!(item, KotlinFileItem::Statement(_))
}

fn format_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &KotlinFileItem<'source>,
) -> Doc<'source> {
    match item {
        KotlinFileItem::Statement(statement) => {
            format_statement_syntax_with_leading(doc, &StatementSyntax::Statement(*statement))
        }
        KotlinFileItem::BogusKotlinFileItem(malformed) => format_malformed(malformed, doc),
        _ => format_file_item(doc, item),
    }
}

fn source_item_separator(
    previous: &KotlinFileItem<'_>,
    current: &KotlinFileItem<'_>,
    preserve_source_blank_line: bool,
) -> BodyItemSeparator {
    let blank = !preserve_source_blank_line || items_have_blank_line_between(previous, current);
    let previous_forces_line = previous.last_token().is_some_and(|token| {
        token
            .trailing_comments()
            .any(|comment| comment.kind() == jolt_kotlin_syntax::KotlinCommentKind::Line)
    });
    match (blank, previous_forces_line) {
        (false, true) => BodyItemSeparator::None,
        (true, true) | (false, false) => BodyItemSeparator::Line,
        (true, false) => BodyItemSeparator::EmptyLine,
    }
}

fn items_have_blank_line_between(left: &KotlinFileItem<'_>, right: &KotlinFileItem<'_>) -> bool {
    left.last_token()
        .zip(right.first_token())
        .is_some_and(|(left, right)| tokens_have_blank_line_between(&left, &right))
}

fn format_package_header<'source>(
    doc: &mut DocBuilder<'source>,
    package: &PackageHeader<'source>,
) -> Doc<'source> {
    format_or_verbatim(package, doc, |doc| {
        let keyword = format_required_field(package.package_token(), doc, |token, doc| {
            let token = format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let space = doc.space();
            doc.concat([token, space])
        });
        let name = format_required_field(package.name(), doc, |name, doc| {
            format_qualified_name(doc, &name)
        });
        let terminators = format_required_field(package.terminators(), doc, |terminators, doc| {
            format_terminator_list(doc, &terminators, true)
        });
        doc.concat([keyword, name, terminators])
    })
}

fn view_token_range<'source>(
    view: &impl KotlinSyntaxView<'source>,
    base: usize,
) -> Option<Range<usize>> {
    let syntax = view.syntax_node()?;
    Some(relative_token_range_between(
        &syntax.first_token()?,
        &syntax.last_token()?,
        base,
    ))
}

fn token_range(token: &jolt_kotlin_syntax::KotlinSyntaxToken<'_>, base: usize) -> Range<usize> {
    relative_token_range_between(token, token, base)
}
