use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, text};
use jolt_java_syntax::{CompilationUnit, CompilationUnitItem, PackageDeclaration};

use crate::context::{FormatRule, JavaFormatter};
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
};
use crate::rules::annotations::format_annotation;
use crate::rules::comments::format_comment_only_compilation_unit;
use crate::rules::declarations::format_type_declaration;
use crate::rules::imports::format_imports;
use crate::rules::modules::format_module_declaration;
use crate::rules::names::format_name;

pub(crate) struct ProgramRule;

impl FormatRule<CompilationUnit> for ProgramRule {
    fn fmt(&self, unit: &CompilationUnit, formatter: &mut JavaFormatter<'_>) -> Doc {
        format_compilation_unit(unit, formatter)
    }
}

fn format_compilation_unit(unit: &CompilationUnit, formatter: &mut JavaFormatter<'_>) -> Doc {
    let items = unit.items().collect::<Vec<_>>();
    let contents = if items.is_empty() {
        format_comment_only_compilation_unit(unit)
    } else {
        let ignored_ranges = formatter_ignore_ranges(&unit.source_text());
        let item_ranges = items
            .iter()
            .map(compilation_unit_item_token_range)
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        if ignored_runs.is_empty() {
            format_compilation_unit_items(items, formatter).unwrap_or_else(jolt_fmt_ir::nil)
        } else {
            format_compilation_unit_items_with_ignored(items, &ignored_runs, formatter)
        }
    };

    concat([contents, hard_line()])
}

fn format_compilation_unit_items(
    items: Vec<CompilationUnitItem>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let mut sections = Vec::new();
    let mut package = None;
    let mut imports = Vec::new();
    let mut module = None;
    let mut types = Vec::new();

    for item in items {
        match item {
            CompilationUnitItem::Package(declaration) => package = Some(declaration),
            CompilationUnitItem::Import(declaration) => imports.push(declaration),
            CompilationUnitItem::Module(declaration) => module = Some(declaration),
            CompilationUnitItem::Type(declaration) => types.push(declaration),
            CompilationUnitItem::EmptyDeclaration(_) => {}
        }
    }

    if let Some(package) = package {
        sections.push(format_package_declaration(&package));
    }

    let imports = format_imports(imports, formatter);
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = module {
        sections.push(format_module_declaration(&module, formatter));
    }

    let types = types
        .into_iter()
        .map(|declaration| format_type_declaration(&declaration))
        .collect::<Vec<_>>();
    if !types.is_empty() {
        sections.push(join_empty_lines(types));
    }

    (!sections.is_empty()).then(|| join_empty_lines(sections))
}

fn format_compilation_unit_items_with_ignored(
    items: Vec<CompilationUnitItem>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun],
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
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

        segment.push(item);
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

fn push_compilation_unit_segment(
    sections: &mut Vec<ProgramSection>,
    segment: &mut Vec<CompilationUnitItem>,
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

fn join_program_sections(sections: Vec<ProgramSection>) -> Doc {
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

struct ProgramSection {
    doc: Doc,
    hard_line_after: bool,
}

fn compilation_unit_item_token_range(item: &CompilationUnitItem) -> Option<Range<usize>> {
    let tokens = match item {
        CompilationUnitItem::Package(item) => item.tokens(),
        CompilationUnitItem::Import(item) => item.tokens(),
        CompilationUnitItem::Module(item) => item.tokens(),
        CompilationUnitItem::Type(item) => item.tokens(),
        CompilationUnitItem::EmptyDeclaration(item) => item.tokens(),
    };
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(first.token_text_range().start().get()..last.token_text_range().end().get())
}

fn format_package_declaration(package: &PackageDeclaration) -> Doc {
    let annotations = package
        .annotations()
        .map(|annotation| format_annotation(&annotation))
        .collect::<Vec<_>>();
    let declaration = concat([
        text("package "),
        package
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        text(";"),
    ]);

    if annotations.is_empty() {
        declaration
    } else {
        concat([join_hard_lines(annotations), hard_line(), declaration])
    }
}
