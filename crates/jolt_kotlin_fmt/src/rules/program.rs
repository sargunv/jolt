use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinFamily, KotlinFile, KotlinFileItem, KotlinMalformedSyntax, KotlinMissingSyntax,
    KotlinRoleElement, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxView, PackageHeader,
    StatementSyntax, boundary_separator_removal_claim,
};
use jolt_syntax::tokens_have_blank_line_between;

use crate::helpers::blocks::{BodyItemSeparator, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_removed_separator,
    format_terminator_list, format_token, token_has_comments,
};
use crate::helpers::recovery::{
    KotlinFormatListPart, format_malformed, format_missing, format_optional_field,
    format_required_field, resolve_list_part,
};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_file_item;
use crate::rules::imports::format_import_list;
use crate::rules::names::format_qualified_name;
use crate::rules::statements::format_statement_syntax_with_leading;
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_run_doc,
};

pub(crate) fn format_file<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (annotations, annotations_visible) = format_file_annotations(doc, file);
    let mut entries = Vec::new();

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
    let contents = match (annotations_visible, contents) {
        (true, Some(contents)) => {
            let line = doc.empty_line();
            doc.concat([annotations, line, contents])
        }
        (_, None) => annotations,
        (false, Some(contents)) => doc.concat([annotations, contents]),
    };
    let line = doc.hard_line();
    doc.concat([contents, eof, line])
}

fn collect_items<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
    entries: &mut Vec<FileEntry<'source>>,
) {
    let items = match file.items() {
        KotlinSyntaxField::Present(items) => items,
        KotlinSyntaxField::Malformed(malformed) => {
            entries.push(FileEntry::Malformed(malformed));
            return;
        }
        KotlinSyntaxField::Missing(missing) => {
            entries.push(FileEntry::Missing(missing));
            return;
        }
    };
    let mut preceding_item = None;
    for part in items.parts() {
        match part {
            KotlinSyntaxListPart::Item(KotlinRoleElement::Node(node)) => {
                if let Some(item) = KotlinFileItem::cast(node) {
                    preceding_item = Some(item);
                    entries.push(FileEntry::Item(item));
                } else {
                    preceding_item = None;
                    doc.block_on_invariant("invalid Kotlin file item node");
                }
            }
            KotlinSyntaxListPart::Item(KotlinRoleElement::Token(token))
            | KotlinSyntaxListPart::Separator(token) => {
                let separator = format_removed_separator(
                    doc,
                    &token,
                    preceding_item
                        .as_ref()
                        .and_then(|owner| boundary_separator_removal_claim(owner, token)),
                    false,
                );
                entries.push(FileEntry::Separator(
                    separator,
                    FormatterIgnoreItemRange::between(&token, &token),
                    token_has_comments(&token),
                ));
            }
            KotlinSyntaxListPart::Malformed(malformed) => {
                preceding_item = None;
                entries.push(FileEntry::Malformed(malformed));
            }
            KotlinSyntaxListPart::Missing(missing) => {
                preceding_item = None;
                entries.push(FileEntry::Missing(missing));
            }
        }
    }
}

enum FileEntry<'source> {
    Item(KotlinFileItem<'source>),
    Malformed(KotlinMalformedSyntax<'source>),
    Missing(KotlinMissingSyntax<'source>),
    Separator(Doc<'source>, FormatterIgnoreItemRange, bool),
}

impl FileEntry<'_> {
    fn ignore_range(&self) -> Option<FormatterIgnoreItemRange> {
        match self {
            Self::Item(item) => {
                let first = item.first_token()?;
                let last = item.last_token()?;
                Some(FormatterIgnoreItemRange::between(&first, &last))
            }
            Self::Malformed(malformed) => {
                let syntax = malformed.syntax_node()?;
                Some(FormatterIgnoreItemRange::between(
                    &syntax.first_token()?,
                    &syntax.last_token()?,
                ))
            }
            Self::Missing(_) => None,
            Self::Separator(_, range, _) => Some(*range),
        }
    }
}

fn format_entries_with_ignored<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
    entries: Vec<FileEntry<'source>>,
) -> (Option<Doc<'source>>, Vec<Range<usize>>) {
    let container = file.text_range();
    let runs = doc.formatter_ignore_runs(container, entries.iter().map(FileEntry::ignore_range));
    if runs.is_empty() {
        return (
            format_entry_segment(doc, entries).map(|section| section.doc),
            Vec::new(),
        );
    }
    let ignored_eof_comments = runs
        .iter()
        .filter_map(FormatterIgnoreRun::claimed_on_marker_range)
        .collect();

    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut entries = entries.into_iter().map(Some).collect::<Vec<_>>();
    for_each_formatter_ignore_splice(entries.len(), &runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            push_entry_segment(doc, &mut sections, &mut segment);
            sections.push(FileSection::ignored(formatter_ignore_run_doc(run, doc)));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(entry) = entries[index].take() {
                segment.push(entry);
            }
        }
    });
    push_entry_segment(doc, &mut sections, &mut segment);
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
    let mut body = Vec::new();
    for entry in entries {
        match entry {
            FileEntry::Item(KotlinFileItem::ImportDirectiveList(imports)) => {
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(format_import_list(doc, &imports)));
            }
            FileEntry::Item(KotlinFileItem::PackageHeader(package)) => {
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(format_package_header(doc, &package)));
            }
            FileEntry::Item(item) => {
                body.push(item);
            }
            FileEntry::Malformed(malformed) => {
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(format_malformed(&malformed, doc)));
            }
            FileEntry::Missing(missing) => {
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection::visible(format_missing(&missing, doc)));
            }
            FileEntry::Separator(separator, _, visible) => {
                flush_body(doc, &mut body, &mut sections);
                sections.push(FileSection {
                    doc: separator,
                    visible,
                    ignored: false,
                });
            }
        }
    }
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
) -> (Doc<'source>, bool) {
    let annotations = match file.annotations() {
        KotlinSyntaxField::Present(annotations) => annotations,
        KotlinSyntaxField::Malformed(malformed) => {
            let visible = malformed.first_token().is_some();
            return (format_malformed(&malformed, doc), visible);
        }
        KotlinSyntaxField::Missing(missing) => return (format_missing(&missing, doc), false),
    };
    let mut formatted = Vec::new();
    let mut invisible = Vec::new();
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
            KotlinFormatListPart::Recovery(recovery) => {
                if recovery.is_visible() {
                    formatted.push(recovery.doc());
                } else {
                    invisible.push(recovery.doc());
                }
            }
        }
    }
    let invisible = doc.concat(invisible);
    if formatted.is_empty() {
        return (invisible, false);
    }
    let formatted = join_hard_lines(doc, formatted);
    (doc.concat([invisible, formatted]), true)
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
            let formatted = format_body_item(body, item);
            body.push(formatted);
            index += 1;
        }
    })
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
    let keyword = format_required_field(package.package_token(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let name = format_required_field(package.name(), doc, |name, doc| {
        let has_token = name.first_token().is_some();
        let name = format_qualified_name(doc, &name);
        if has_token {
            let space = doc.space();
            doc.concat([space, name])
        } else {
            name
        }
    });
    let suffix = format_optional_field(package.suffix(), doc, |suffix, doc| {
        format_malformed(&suffix, doc)
    });
    let terminators = format_required_field(package.terminators(), doc, |terminators, doc| {
        format_terminator_list(doc, &terminators)
    });
    doc.concat([keyword, name, suffix, terminators])
}
