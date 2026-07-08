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
        ) if function.token_iter().count() == 1 && function.fun_token().is_some()
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

#[cfg(test)]
mod tests {
    use jolt_fmt_ir::{
        IndentStyle, RenderControl, RenderOptions, RenderSink, TextWidth, concat, hard_line,
        render_to,
    };
    use jolt_kotlin_syntax::parse_kotlin_file;

    use super::format_file_contents;

    #[test]
    fn normalizes_package_import_spacing_and_import_order() {
        let source = "package   com.example\nimport z.Z\nimport a.A\nclass Demo\n";

        assert_eq!(
            format(source),
            "package com.example\n\nimport a.A\nimport z.Z\n\nclass Demo\n"
        );
    }

    #[test]
    fn preserves_file_annotation_preamble() {
        let source = "@file:Suppress(\"unused\")\n\npackage sample\n\nclass Demo\n";

        assert_eq!(
            format(source),
            "@file:Suppress(\"unused\")\n\npackage sample\n\nclass Demo\n"
        );
    }

    #[test]
    fn recovered_lambda_missing_close_preserves_body_tokens() {
        let formatted = format("fun demo() { run { value(1)\n");

        assert!(formatted.contains("run"));
        assert!(formatted.contains("value"));
        assert!(formatted.contains('1'));
    }

    #[test]
    fn recovered_block_missing_close_preserves_statement_tokens() {
        let formatted = format("fun demo() { val answer = 42\n");

        assert!(formatted.contains("val"));
        assert!(formatted.contains("answer"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn recovered_argument_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("fun demo() { call(/* before */,\n/* value */ value)\n}\n");

        assert!(formatted.contains("/* before */"));
        assert!(formatted.contains(','));
        assert!(formatted.contains("/* value */"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn recovered_value_parameter_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("fun demo(first: String, /* orphan */ , second: Int) {}\n");

        assert!(formatted.contains("first"));
        assert!(formatted.contains("String"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("second"));
        assert!(formatted.contains("Int"));
    }

    #[test]
    fn recovered_type_argument_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("fun demo() { call<First, /* orphan */ , Second>() }\n");

        assert!(formatted.contains("First"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("Second"));
    }

    #[test]
    fn recovered_call_without_callee_formats_structured_suffixes() {
        let formatted = format("fun demo() { <String>(value) }\n");

        assert!(formatted.contains("String"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn recovered_when_missing_close_preserves_entries() {
        let formatted = format("fun demo(value: Int) { when (value) { 1 -> one()\n");

        assert!(formatted.contains("when"));
        assert!(formatted.contains('1'));
        assert!(formatted.contains("one"));
    }

    #[test]
    fn recovered_for_partial_header_preserves_body() {
        let formatted =
            format("fun demo(items: List<String>) { for (item in items print(item) }\n");

        assert!(formatted.contains("for"));
        assert!(formatted.contains("item"));
        assert!(formatted.contains("items"));
        assert!(formatted.contains("print"));
    }

    #[test]
    fn recovered_else_without_branch_preserves_else_token() {
        let formatted = format("fun demo(flag: Boolean) { if (flag) yes() else }\n");

        assert!(formatted.contains("else"));
    }

    #[test]
    fn recovered_class_body_preserves_orphan_tokens_and_comments() {
        let source = "class Demo { fun ok() {}\n/* orphan */ +\nval value = 1 }\n";
        let formatted = format(source);

        assert!(formatted.contains("ok"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains('+'));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn recovered_block_with_formatter_ignore_preserves_orphan_tokens() {
        let source = concat!(
            "fun demo() {\n",
            "  val before = 1\n",
            "  // @formatter:off\n",
            "  val raw=1+2\n",
            "  // @formatter:on\n",
            "  /* block */ val after = value\n",
            "}\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("val raw=1+2"), "{formatted}");
        assert!(formatted.contains("/* block */"), "{formatted}");
        assert!(formatted.contains("after"), "{formatted}");
        assert!(formatted.contains("value"), "{formatted}");
    }

    #[test]
    fn recovered_class_body_with_formatter_ignore_preserves_orphan_tokens() {
        let source = concat!(
            "class Demo {\n",
            "  fun before() {}\n",
            "  // @formatter:off\n",
            "  fun raw(){ }\n",
            "  // @formatter:on\n",
            "  /* member */ +\n",
            "}\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("fun raw(){ }"));
        assert!(formatted.contains("/* member */"));
        assert!(formatted.contains('+'));
    }

    #[test]
    fn recovered_navigation_without_receiver_preserves_selector_tokens() {
        let formatted = format("fun demo() { .next }\n");

        assert!(formatted.contains('.'));
        assert!(formatted.contains("next"));
    }

    #[test]
    fn recovered_assignment_without_left_preserves_operator_and_right() {
        let formatted = format("fun demo() { = value }\n");

        assert!(formatted.contains('='));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn recovered_collection_and_index_lists_preserve_orphan_tokens_and_comments() {
        let formatted = format(
            "fun demo(values: List<Int>) { [/* collection */ , value]\nvalues[/* index */ , 0] }\n",
        );

        assert!(formatted.contains("/* collection */"));
        assert!(formatted.contains("value"));
        assert!(formatted.contains("/* index */"));
        assert!(formatted.contains('0'));
    }

    #[test]
    fn recovered_type_parameter_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("class Box<First, /* orphan */ , Second>\n");

        assert!(formatted.contains("First"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("Second"));
    }

    #[test]
    fn recovered_when_condition_list_preserves_orphan_tokens_and_comments() {
        let source = "fun demo(x: Int) { when (x) { 1, /* orphan */ , 2 -> hit() } }\n";
        let formatted = format(source);

        assert!(formatted.contains('1'));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains('2'));
        assert!(formatted.contains("hit"));
    }

    #[test]
    fn recovered_context_function_type_preserves_parameter_commas() {
        let source = "typealias Handler = context(First, /* orphan */ , Second) () -> Unit\n";
        let formatted = format(source);

        assert!(formatted.contains("context"));
        assert!(formatted.contains("First"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("Second"));
        assert!(formatted.contains("Unit"));
    }

    #[test]
    fn recovered_lambda_parameter_list_preserves_orphan_separators() {
        let formatted = format("fun demo() { { first, /* orphan */ , second -> first } }\n");

        assert!(formatted.contains("first"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("second"));
        assert!(formatted.contains("->"));
    }

    #[test]
    fn explicit_backing_field_formats_available_represented_pieces() {
        let formatted = format("val value: Int field = /* backing */ compute()\n");

        assert!(formatted.contains("value"));
        assert!(formatted.contains("field"));
        assert!(formatted.contains('='));
        assert!(formatted.contains("/* backing */"));
        assert!(formatted.contains("compute"));
    }

    #[test]
    fn import_suffixes_use_structured_alias_and_star_tokens() {
        let formatted = format("import sample.deep.*\nimport sample.Name as Alias\n");

        assert!(formatted.contains("sample.deep"));
        assert!(formatted.contains('*'));
        assert!(formatted.contains("sample.Name"));
        assert!(formatted.contains("as"));
        assert!(formatted.contains("Alias"));
    }

    #[test]
    fn recovered_declaration_tails_preserve_dangling_tokens() {
        let formatted = format("fun value() =\nval answer =\nval delegated by\n");

        assert!(formatted.contains("fun value()"), "{formatted}");
        assert!(formatted.contains('='), "{formatted}");
        assert!(formatted.contains("answer"), "{formatted}");
        assert!(formatted.contains("by"), "{formatted}");
    }

    #[test]
    fn recovered_control_flow_missing_children_preserves_available_parts() {
        let formatted = format("fun demo(flag: Boolean) { label@\ndo { work() }\nwhen (flag)\n}\n");

        assert!(formatted.contains("label"));
        assert!(formatted.contains('@'));
        assert!(formatted.contains("do"));
        assert!(formatted.contains("work"));
        assert!(formatted.contains("when"));
        assert!(formatted.contains("flag"));
    }

    #[test]
    fn recovered_lambda_branch_preserves_body_orphan_tokens() {
        let formatted =
            format("fun demo(flag: Boolean) { if (flag) { run(/* branch */ value)\nnext() } }\n");

        assert!(formatted.contains("/* branch */"), "{formatted}");
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains("next"), "{formatted}");
    }

    #[test]
    fn recovered_annotation_argument_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("@Anno(/* orphan */ , value)\nclass Demo\n");

        assert!(formatted.contains("Anno"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn recovered_type_constraints_preserve_orphan_tokens_and_comments() {
        let formatted = format("fun <T> demo() where T : Any, /* orphan */ , T : Closeable {}\n");

        assert!(formatted.contains("T : Any"), "{formatted}");
        assert!(formatted.contains("/* orphan */"), "{formatted}");
        assert!(formatted.contains("Closeable"), "{formatted}");
    }

    #[test]
    fn recovered_destructuring_preserves_orphan_tokens_and_comments() {
        let formatted = format("val (first, /* orphan */ , second) = pair\n");

        assert!(formatted.contains("first"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("second"));
    }

    #[test]
    fn recovered_object_delegation_preserves_orphan_tokens_and_comments() {
        let formatted = format("val value = object : First, /* orphan */ , Second {}\n");

        assert!(formatted.contains("First"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains("Second"));
    }

    fn format(source: &str) -> String {
        let parse = parse_kotlin_file(source);
        let file = parse.syntax().expect("test input should parse");
        let doc = concat([format_file_contents(&file), hard_line()]);
        let mut sink = StringDocSink::default();
        render_to(
            &doc,
            RenderOptions {
                line_width: TextWidth::from(80),
                indent_width: 4,
                indent_style: IndentStyle::Space,
            },
            &mut sink,
        )
        .expect("test doc should render");
        sink.output
    }

    #[derive(Default)]
    struct StringDocSink {
        output: String,
    }

    impl RenderSink for &mut StringDocSink {
        fn write_str(&mut self, text: &str) -> RenderControl {
            self.output.push_str(text);
            RenderControl::Continue
        }
    }
}
