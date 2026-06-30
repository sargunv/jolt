use jolt_diagnostics::TextRange;
use jolt_java_syntax::VariableInitializerValue;
use std::collections::HashMap;

use crate::context::JavaFormatContext;

pub(crate) struct TabularLayout {
    pub cols: usize,
    pub rows: Vec<Vec<usize>>,
}

pub(crate) struct TabularEntry {
    pub range: TextRange,
    pub kind: ParallelKind,
    pub row_weight: usize,
    pub is_nested_array: bool,
}

pub(crate) fn tabular_layout(
    values: &[VariableInitializerValue],
    context: &JavaFormatContext<'_>,
) -> Option<TabularLayout> {
    let entries = values
        .iter()
        .map(initializer_tabular_entry)
        .collect::<Vec<_>>();
    tabular_layout_for_entries(&entries, context)
}

pub(crate) fn tabular_layout_for_entries(
    entries: &[TabularEntry],
    context: &JavaFormatContext<'_>,
) -> Option<TabularLayout> {
    if entries.is_empty() {
        return None;
    }

    let start0 = start_column(entries.first()?.range, context)?;
    let mut rows = Vec::new();
    let mut index = 0;

    let mut first_row = vec![index];
    index += 1;
    while index < entries.len() && start_column(entries[index].range, context)? > start0 {
        first_row.push(index);
        index += 1;
    }
    if index >= entries.len() {
        return None;
    }
    if row_length(entries, &first_row) <= 1 {
        return None;
    }
    rows.push(first_row);

    while index < entries.len() {
        let start = start_column(entries[index].range, context)?;
        if start != start0 {
            return None;
        }
        let mut row = vec![index];
        index += 1;
        while index < entries.len() && start_column(entries[index].range, context)? > start0 {
            row.push(index);
            index += 1;
        }
        rows.push(row);
    }

    let size0 = rows[0].len();
    if !expressions_are_parallel(entries, &rows, 0, rows.len()) {
        return None;
    }
    for column in 1..size0 {
        if !expressions_are_parallel(entries, &rows, column, rows.len() / 2 + 1) {
            return None;
        }
    }

    if rows.len() == 2 {
        if size0 == rows[1].len() {
            return Some(TabularLayout { cols: size0, rows });
        }
        return None;
    }

    for row in rows.iter().take(rows.len() - 1).skip(1) {
        if row.len() != size0 {
            return None;
        }
    }
    if size0 < rows.last().expect("at least two rows").len() {
        return None;
    }

    Some(TabularLayout { cols: size0, rows })
}

pub(crate) fn has_only_short_items(
    values: &[VariableInitializerValue],
    policy: crate::policy::JavaFormatPolicy,
) -> bool {
    let max_length = policy.argument_list_max_item_length_for_filling();
    values
        .iter()
        .all(|value| value_source_width(value) < max_length)
}

pub(crate) fn has_only_short_entries(
    entries: &[TabularEntry],
    policy: crate::policy::JavaFormatPolicy,
) -> bool {
    let max_length = policy.argument_list_max_item_length_for_filling();
    entries.iter().all(|entry| entry_width(entry) < max_length)
}

pub(crate) fn tabular_entries(values: &[VariableInitializerValue]) -> Vec<TabularEntry> {
    values.iter().map(initializer_tabular_entry).collect()
}

pub(crate) fn row_opens_without_extra_indent(
    entries: &[TabularEntry],
    row: &[usize],
    cols: usize,
) -> bool {
    cols == 1
        || row
            .first()
            .is_some_and(|&index| entries[index].is_nested_array)
}

fn initializer_tabular_entry(value: &VariableInitializerValue) -> TabularEntry {
    TabularEntry {
        range: value
            .code_text_range()
            .expect("parser-clean initializer value should have a source range"),
        kind: parallel_kind_for_initializer(value),
        row_weight: value_row_weight(value),
        is_nested_array: matches!(value, VariableInitializerValue::ArrayInitializer(_)),
    }
}

fn start_column(range: TextRange, context: &JavaFormatContext<'_>) -> Option<usize> {
    Some(context.source_column_at(range.start().get()))
}

fn value_source_width(value: &VariableInitializerValue) -> usize {
    value
        .code_text_range()
        .map_or(usize::MAX, |range| range.end().get().saturating_sub(range.start().get()))
}

fn entry_width(entry: &TabularEntry) -> usize {
    entry
        .range
        .end()
        .get()
        .saturating_sub(entry.range.start().get())
}

fn row_length(entries: &[TabularEntry], indices: &[usize]) -> usize {
    indices.iter().map(|&index| entries[index].row_weight).sum()
}

fn expressions_are_parallel(
    entries: &[TabularEntry],
    rows: &[Vec<usize>],
    column: usize,
    at_least: usize,
) -> bool {
    let mut counts = HashMap::<ParallelKind, usize>::new();
    for row in rows {
        if column >= row.len() {
            continue;
        }
        *counts.entry(entries[row[column]].kind).or_default() += 1;
    }
    counts.values().any(|count| *count >= at_least)
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) enum ParallelKind {
    Literal,
    Name,
    ArrayInitializer,
    ArrayCreation,
    Binary,
    UnaryLiteral,
    Annotation,
    Other,
}

fn parallel_kind_for_initializer(value: &VariableInitializerValue) -> ParallelKind {
    match value {
        VariableInitializerValue::LiteralExpression(_) => ParallelKind::Literal,
        VariableInitializerValue::NameExpression(_) => ParallelKind::Name,
        VariableInitializerValue::ArrayInitializer(_) => ParallelKind::ArrayInitializer,
        VariableInitializerValue::ArrayCreationExpression(_) => ParallelKind::ArrayCreation,
        VariableInitializerValue::BinaryExpression(_) => ParallelKind::Binary,
        VariableInitializerValue::UnaryExpression(unary) => {
            if unary.operand().is_some_and(|operand| {
                matches!(operand, jolt_java_syntax::Expression::LiteralExpression(_))
            }) {
                ParallelKind::UnaryLiteral
            } else {
                ParallelKind::Other
            }
        }
        _ => ParallelKind::Other,
    }
}

fn value_row_weight(value: &VariableInitializerValue) -> usize {
    match value {
        VariableInitializerValue::ArrayInitializer(initializer) => initializer
            .values()
            .map(|value| value_row_weight(&value))
            .sum(),
        VariableInitializerValue::ArrayCreationExpression(_) => 1,
        _ => 1,
    }
}
