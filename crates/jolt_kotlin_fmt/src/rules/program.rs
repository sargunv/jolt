use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, space};
use jolt_kotlin_syntax::{KotlinFile, KotlinFileItem, PackageHeader, StatementSyntax};

use crate::context::KotlinFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
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
    _formatter: &mut KotlinFormatter<'_>,
) -> Doc<'source> {
    concat([format_file_contents(file), hard_line()])
}

fn format_file_contents<'source>(file: &KotlinFile<'source>) -> Doc<'source> {
    let items = file.items().collect::<Vec<_>>();
    if items.is_empty() {
        return format_file_annotations(file).unwrap_or_else(jolt_fmt_ir::nil);
    }

    let ignored_ranges = formatter_ignore_ranges(
        file.source_text(),
        file.text_range().start().get(),
        file.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        let item_ranges = items
            .iter()
            .map(|item| file_item_token_range(item, file.text_range().start().get()))
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        if !ignored_runs.is_empty() {
            return format_file_contents_with_ignored(file, items, &ignored_runs);
        }
    }

    let mut sections = Vec::new();
    if let Some(annotations) = format_file_annotations(file) {
        sections.push(annotations);
    }
    push_file_item_sections(file.source_text(), items, &mut sections);

    join_empty_lines(sections)
}

fn format_file_contents_with_ignored<'source>(
    file: &KotlinFile<'source>,
    items: Vec<KotlinFileItem<'source>>,
    ignored_runs: &[FormatterIgnoreRun<'source>],
) -> Doc<'source> {
    let source = file.source_text();
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    if let Some(annotations) = format_file_annotations(file) {
        sections.push(FileSection {
            doc: annotations,
            hard_line_after: false,
        });
    }

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_file_item_segment(source, &mut sections, &mut segment);
            let run = &ignored_runs[ignored_index];
            sections.push(FileSection {
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

        segment.push(item);
    }

    push_file_item_segment(source, &mut sections, &mut segment);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(FileSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_file_sections(sections)
}

fn format_file_annotations<'source>(file: &KotlinFile<'source>) -> Option<Doc<'source>> {
    let annotations = file.annotations().collect::<Vec<_>>();
    (!annotations.is_empty()).then(|| {
        join_hard_lines(
            annotations
                .iter()
                .map(|annotation| format_annotation(annotation)),
        )
    })
}

fn push_file_item_sections<'source>(
    source: &'source str,
    items: Vec<KotlinFileItem<'source>>,
    sections: &mut Vec<Doc<'source>>,
) {
    let mut package = None;
    let mut imports = None;
    let mut body_items = Vec::new();

    for item in items {
        match item {
            KotlinFileItem::PackageHeader(header) => package = Some(header),
            KotlinFileItem::ImportList(list) => {
                imports = format_imports(list.directives().collect());
            }
            item => body_items.push(item),
        }
    }

    if let Some(package) = package {
        sections.push(format_package_header(&package));
    }
    if let Some(imports) = imports {
        sections.push(imports);
    }
    let body_sections = format_source_body_sections(source, body_items);
    if !body_sections.is_empty() {
        sections.push(join_empty_lines(body_sections));
    }
}

fn push_file_item_segment<'source>(
    source: &'source str,
    sections: &mut Vec<FileSection<'source>>,
    segment: &mut Vec<KotlinFileItem<'source>>,
) {
    if segment.is_empty() {
        return;
    }

    let mut docs = Vec::new();
    push_file_item_sections(source, std::mem::take(segment), &mut docs);
    if !docs.is_empty() {
        sections.push(FileSection {
            doc: join_empty_lines(docs),
            hard_line_after: false,
        });
    }
}

fn join_file_sections(sections: Vec<FileSection<'_>>) -> Doc<'_> {
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

struct FileSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn format_source_body_sections<'source>(
    source: &'source str,
    items: Vec<KotlinFileItem<'source>>,
) -> Vec<Doc<'source>> {
    source_item_groups(source, items)
        .into_iter()
        .map(|group| format_source_item_group(source, &group))
        .collect()
}

fn source_item_groups<'source>(
    source: &str,
    items: Vec<KotlinFileItem<'source>>,
) -> Vec<SourceItemGroup<'source>> {
    let mut groups = Vec::new();
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
    source: &str,
    previous: &KotlinFileItem<'_>,
    current: &KotlinFileItem<'_>,
    previous_end: usize,
    current_start: usize,
) -> bool {
    !has_blank_line_between(source, previous_end, current_start)
        && (is_statement_item(previous)
            || is_statement_item(current)
            || is_fun_interface_pair(previous, current))
}

fn is_statement_item(item: &KotlinFileItem<'_>) -> bool {
    matches!(item, KotlinFileItem::Statement(_))
}

fn is_fun_interface_pair(previous: &KotlinFileItem<'_>, current: &KotlinFileItem<'_>) -> bool {
    matches!(
        (previous, current),
        (
            KotlinFileItem::FunctionDeclaration(function),
            KotlinFileItem::InterfaceDeclaration(_)
        ) if function.is_fun_interface_header()
    )
}

fn format_source_item_group<'source>(
    _source: &'source str,
    group: &SourceItemGroup<'source>,
) -> Doc<'source> {
    if let [item] = group.items.as_slice() {
        return format_body_item(item);
    }
    if let [
        KotlinFileItem::FunctionDeclaration(function),
        KotlinFileItem::InterfaceDeclaration(interface),
    ] = group.items.as_slice()
        && let Some(doc) = format_fun_interface_file_items(function, interface)
    {
        return doc;
    }

    join_hard_lines(group.items.iter().map(format_body_item))
}

fn format_body_item<'source>(item: &KotlinFileItem<'source>) -> Doc<'source> {
    match item {
        KotlinFileItem::Statement(statement) => {
            format_statement_syntax(&StatementSyntax::Statement(*statement))
        }
        _ => format_file_item(item),
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

fn file_item_token_range(item: &KotlinFileItem<'_>, file_start: usize) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &item.first_token()?,
        &item.last_token()?,
        file_start,
    ))
}

fn has_blank_line_between(source: &str, left_end: usize, right_start: usize) -> bool {
    source[left_end..right_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .take(2)
        .count()
        >= 2
}

fn format_package_header<'source>(package: &PackageHeader<'source>) -> Doc<'source> {
    concat([
        package
            .package_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([
                    format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                    space(),
                ])
            }),
        package
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_qualified_name(&name)),
    ])
}
