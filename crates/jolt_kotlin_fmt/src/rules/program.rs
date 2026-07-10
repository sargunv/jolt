use std::ops::Range;

use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinFile, KotlinFileItem, PackageHeader, StatementSyntax};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRun, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::{format_file_item, format_fun_interface_file_items};
use crate::rules::imports::format_imports;
use crate::rules::names::format_qualified_name;
use crate::rules::statements::format_statement_syntax;

pub(crate) fn format_file<'source>(
    file: &KotlinFile<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let contents = format_file_contents(doc, file);
    let recovered = format_token_sequence(doc, file.recovered_tokens(), LeadingTrivia::Preserve);
    let hard_line = doc.hard_line();
    doc.concat([contents, recovered, hard_line])
}

fn format_file_contents<'source>(
    doc: &mut DocBuilder<'source>,
    file: &KotlinFile<'source>,
) -> Doc<'source> {
    let items = file.items().collect::<Vec<_>>();
    if items.is_empty() {
        return format_file_annotations(doc, file).unwrap_or_else(|| doc.nil());
    }

    let ignored_ranges = formatter_ignore_ranges(
        file.source_text(),
        file.text_range().start().get(),
        file.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        let item_ranges = items
            .iter()
            .map(|item| file_item_token_range(doc, item, file.text_range().start().get()))
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        if !ignored_runs.is_empty() {
            return format_file_contents_with_ignored(doc, file, items, &ignored_runs);
        }
    }

    doc.concat_list(|sections| {
        if let Some(annotations) = format_file_annotations(sections, file) {
            sections.push(annotations);
        }
        if let Some(item_sections) = format_file_item_sections(sections, file.source_text(), items)
        {
            if !sections.is_empty() {
                let empty_line = sections.empty_line();
                sections.push(empty_line);
            }
            sections.push(item_sections);
        }
    })
}

fn format_file_contents_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    file: &KotlinFile<'source>,
    items: Vec<KotlinFileItem<'source>>,
    ignored_runs: &[FormatterIgnoreRun<'source>],
) -> Doc<'source> {
    let source = file.source_text();
    let mut sections = Vec::with_capacity(items.len().saturating_add(ignored_runs.len()));
    let mut segment = Vec::with_capacity(items.len());
    let mut ignored_index = 0;
    let mut skip_index = 0;

    if let Some(annotations) = format_file_annotations(doc, file) {
        sections.push(FileSection {
            doc: annotations,
            hard_line_after: false,
        });
    }

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_file_item_segment(doc, source, &mut sections, &mut segment);
            let run = &ignored_runs[ignored_index];
            sections.push(FileSection {
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

        segment.push(item);
    }

    push_file_item_segment(doc, source, &mut sections, &mut segment);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(FileSection {
            doc: formatter_ignore_run_doc(run, doc),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_file_sections(doc, sections)
}

fn format_file_annotations<'source>(
    doc: &mut DocBuilder<'source>,
    file: &KotlinFile<'source>,
) -> Option<Doc<'source>> {
    let annotations = file
        .annotations()
        .map(|annotation| format_annotation(doc, &annotation))
        .collect::<Vec<_>>();
    (!annotations.is_empty()).then(|| join_hard_lines(doc, annotations))
}

fn format_file_item_sections<'source>(
    doc: &mut DocBuilder<'source>,
    source: &'source str,
    items: Vec<KotlinFileItem<'source>>,
) -> Option<Doc<'source>> {
    let mut package = None;
    let mut imports = None;
    let mut body_items = Vec::with_capacity(items.len());

    for item in items {
        match item {
            KotlinFileItem::PackageHeader(header) => package = Some(header),
            KotlinFileItem::ImportList(list) => {
                imports = format_imports(doc, list.directives().collect());
            }
            item => body_items.push(item),
        }
    }

    let mut is_empty = true;
    let sections = doc.concat_list(|sections| {
        if let Some(package) = package {
            let package = format_package_header(sections, &package);
            push_file_item_section(sections, package);
        }
        if let Some(imports) = imports {
            push_file_item_section(sections, imports);
        }
        if let Some(body_sections) = format_source_body_sections(sections, source, body_items) {
            push_file_item_section(sections, body_sections);
        }
        is_empty = sections.is_empty();
    });

    (!is_empty).then_some(sections)
}

fn push_file_item_section<'source>(
    sections: &mut ConcatBuilder<'_, 'source>,
    section: Doc<'source>,
) {
    if !sections.is_empty() {
        let empty_line = sections.empty_line();
        sections.push(empty_line);
    }
    sections.push(section);
}

fn push_file_item_segment<'source>(
    doc: &mut DocBuilder<'source>,
    source: &'source str,
    sections: &mut Vec<FileSection<'source>>,
    segment: &mut Vec<KotlinFileItem<'source>>,
) {
    if segment.is_empty() {
        return;
    }

    if let Some(doc) = format_file_item_sections(doc, source, std::mem::take(segment)) {
        sections.push(FileSection {
            doc,
            hard_line_after: false,
        });
    }
}

fn join_file_sections<'source>(
    doc: &mut DocBuilder<'source>,
    sections: Vec<FileSection<'source>>,
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

struct FileSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn format_source_body_sections<'source>(
    doc: &mut DocBuilder<'source>,
    source: &'source str,
    items: Vec<KotlinFileItem<'source>>,
) -> Option<Doc<'source>> {
    let groups = source_item_groups(doc, source, items);
    let mut is_empty = true;
    let sections = doc.concat_list(|sections| {
        for group in groups {
            if !sections.is_empty() {
                let empty_line = sections.empty_line();
                sections.push(empty_line);
            }
            let group = format_source_item_group(sections, source, &group);
            sections.push(group);
        }
        is_empty = sections.is_empty();
    });
    (!is_empty).then_some(sections)
}

fn source_item_groups<'source>(
    doc: &mut DocBuilder<'source>,
    source: &str,
    items: Vec<KotlinFileItem<'source>>,
) -> Vec<SourceItemGroup<'source>> {
    let mut groups = Vec::with_capacity(items.len());
    let mut current: Option<SourceItemGroup<'source>> = None;

    for (item, range) in items
        .into_iter()
        .filter_map(|item| SourceItemRange::new(&item).map(|range| (item, range)))
    {
        let Some(current_group) = current.as_mut() else {
            current = Some(SourceItemGroup::new(item, range));
            continue;
        };

        if current_group.items.last().is_some_and(|previous| {
            should_continue_source_group(
                doc,
                source,
                previous,
                &item,
                current_group.range.token_end,
                range.token_start,
            )
        }) {
            current_group.push(item, range);
            continue;
        }

        groups.push(std::mem::replace(
            current_group,
            SourceItemGroup::new(item, range),
        ));
    }

    if let Some(group) = current {
        groups.push(group);
    }

    groups
}

fn should_continue_source_group(
    doc: &mut DocBuilder<'_>,
    source: &str,
    previous: &KotlinFileItem<'_>,
    current: &KotlinFileItem<'_>,
    previous_end: usize,
    current_start: usize,
) -> bool {
    !has_blank_line_between(doc, source, previous_end, current_start)
        && (is_statement_item(doc, previous)
            || is_statement_item(doc, current)
            || is_fun_interface_pair(doc, previous, current))
}

fn is_statement_item(_doc: &mut DocBuilder<'_>, item: &KotlinFileItem<'_>) -> bool {
    matches!(item, KotlinFileItem::Statement(_))
}

fn is_fun_interface_pair(
    _doc: &mut DocBuilder<'_>,
    previous: &KotlinFileItem<'_>,
    current: &KotlinFileItem<'_>,
) -> bool {
    matches!(
        (previous, current),
        (
            KotlinFileItem::FunctionDeclaration(function),
            KotlinFileItem::InterfaceDeclaration(_)
        ) if function.is_fun_interface_header()
    )
}

fn format_source_item_group<'source>(
    doc: &mut DocBuilder<'source>,
    _source: &'source str,
    group: &SourceItemGroup<'source>,
) -> Doc<'source> {
    if let [item] = group.items.as_slice() {
        return format_body_item(doc, item);
    }
    if let [
        KotlinFileItem::FunctionDeclaration(function),
        KotlinFileItem::InterfaceDeclaration(interface),
    ] = group.items.as_slice()
        && let Some(doc) = format_fun_interface_file_items(doc, function, interface)
    {
        return doc;
    }

    let items = group
        .items
        .iter()
        .map(|item| format_body_item(doc, item))
        .collect::<Vec<_>>();
    join_hard_lines(doc, items)
}

fn format_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &KotlinFileItem<'source>,
) -> Doc<'source> {
    match item {
        KotlinFileItem::Statement(statement) => {
            format_statement_syntax(doc, &StatementSyntax::Statement(*statement))
        }
        _ => format_file_item(doc, item),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceItemGroup<'source> {
    range: SourceItemRange,
    items: Vec<KotlinFileItem<'source>>,
}

impl<'source> SourceItemGroup<'source> {
    fn new(item: KotlinFileItem<'source>, range: SourceItemRange) -> Self {
        Self {
            range,
            items: vec![item],
        }
    }

    fn push(&mut self, item: KotlinFileItem<'source>, range: SourceItemRange) {
        self.range.section_end = range.section_end;
        self.range.token_end = range.token_end;
        self.items.push(item);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceItemRange {
    section_start: usize,
    section_end: usize,
    token_start: usize,
    token_end: usize,
}

impl SourceItemRange {
    fn new(item: &KotlinFileItem<'_>) -> Option<Self> {
        Some(Self {
            section_start: item.text_range().start().get(),
            section_end: item.text_range().end().get(),
            token_start: item.first_token()?.token_text_range().start().get(),
            token_end: item.last_token()?.token_text_range().end().get(),
        })
    }
}

fn file_item_token_range(
    _doc: &mut DocBuilder<'_>,
    item: &KotlinFileItem<'_>,
    file_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &item.first_token()?,
        &item.last_token()?,
        file_start,
    ))
}

fn has_blank_line_between(
    _doc: &mut DocBuilder<'_>,
    source: &str,
    left_end: usize,
    right_start: usize,
) -> bool {
    source[left_end..right_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .take(2)
        .count()
        >= 2
}

fn format_package_header<'source>(
    doc: &mut DocBuilder<'source>,
    package: &PackageHeader<'source>,
) -> Doc<'source> {
    let package_token = if let Some(token) = package.package_token() {
        let token = format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let space = doc.space();
        doc.concat([token, space])
    } else {
        doc.nil()
    };
    let name = if let Some(name) = package.name() {
        format_qualified_name(doc, &name)
    } else {
        doc.nil()
    };
    doc.concat([package_token, name])
}
