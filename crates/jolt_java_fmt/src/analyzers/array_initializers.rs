use jolt_diagnostics::TextRange;
use jolt_java_syntax::{
    AnnotationElementValue, Expression, JavaSyntaxKind, VariableInitializerValue,
};
use std::collections::HashMap;

use crate::context::JavaFormatContext;

pub(crate) struct TabularLayout {
    pub cols: usize,
    pub rows: Vec<Vec<usize>>,
}

#[derive(Clone, Copy)]
pub(crate) struct TabularEntry {
    pub range: TextRange,
    pub kind: ParallelKind,
    pub row_weight: usize,
    pub is_nested_array: bool,
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

pub(crate) fn annotation_array_tabular_entry(value: &AnnotationElementValue) -> TabularEntry {
    TabularEntry {
        range: value
            .code_text_range()
            .expect("parser-clean annotation array value should have a source range"),
        kind: annotation_value_parallel_kind(value),
        row_weight: annotation_value_row_weight(value),
        is_nested_array: value.array_initializer().is_some(),
    }
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
    value.code_text_range().map_or(usize::MAX, |range| {
        range.end().get().saturating_sub(range.start().get())
    })
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
        let kind = entries[row[column]].kind;
        if kind != ParallelKind::Other {
            *counts.entry(kind).or_default() += 1;
        }
    }
    counts.values().any(|count| *count >= at_least)
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) enum ParallelKind {
    Syntax(JavaSyntaxKind),
    Annotation,
    Other,
}

impl ParallelKind {
    pub(crate) fn is_scalar_initializer(self) -> bool {
        matches!(
            self,
            Self::Syntax(
                JavaSyntaxKind::IntegerLiteral
                    | JavaSyntaxKind::FloatingPointLiteral
                    | JavaSyntaxKind::BooleanLiteral
                    | JavaSyntaxKind::CharacterLiteral
                    | JavaSyntaxKind::StringLiteral
                    | JavaSyntaxKind::TextBlockLiteral
                    | JavaSyntaxKind::NullLiteral
                    | JavaSyntaxKind::NameExpression
            )
        )
    }
}

fn parallel_kind_for_initializer(value: &VariableInitializerValue) -> ParallelKind {
    match value {
        VariableInitializerValue::LiteralExpression(literal) => literal
            .token()
            .map_or(ParallelKind::Other, |token| syntax_kind(token.kind())),
        VariableInitializerValue::AssignmentExpression(assignment) => assignment
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        VariableInitializerValue::ArrayInitializer(_)
        | VariableInitializerValue::ArrayCreationExpression(_) => {
            syntax_kind(JavaSyntaxKind::ArrayInitializer)
        }
        VariableInitializerValue::BinaryExpression(binary) => binary
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        VariableInitializerValue::UnaryExpression(unary) => {
            unary.operand().map_or(ParallelKind::Other, |operand| {
                immediate_expression_kind(&operand)
            })
        }
        VariableInitializerValue::PostfixExpression(postfix) => {
            postfix.operand().map_or(ParallelKind::Other, |operand| {
                immediate_expression_kind(&operand)
            })
        }
        _ => syntax_kind(value.kind()),
    }
}

pub(crate) fn expression_parallel_kind(expression: &Expression) -> ParallelKind {
    match expression {
        Expression::LiteralExpression(literal) => literal
            .token()
            .map_or(ParallelKind::Other, |token| syntax_kind(token.kind())),
        Expression::AssignmentExpression(assignment) => assignment
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        Expression::ArrayCreationExpression(_) => syntax_kind(JavaSyntaxKind::ArrayInitializer),
        Expression::BinaryExpression(binary) => binary
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        Expression::UnaryExpression(unary) => {
            unary.operand().map_or(ParallelKind::Other, |operand| {
                immediate_expression_kind(&operand)
            })
        }
        Expression::PostfixExpression(postfix) => {
            postfix.operand().map_or(ParallelKind::Other, |operand| {
                immediate_expression_kind(&operand)
            })
        }
        _ => syntax_kind(expression.kind()),
    }
}

pub(crate) fn annotation_value_parallel_kind(value: &AnnotationElementValue) -> ParallelKind {
    if value.array_initializer().is_some() {
        return syntax_kind(JavaSyntaxKind::ArrayInitializer);
    }
    if value.annotation().is_some() {
        return ParallelKind::Annotation;
    }
    if let Some(expression) = value.expression() {
        return expression_parallel_kind(&expression);
    }
    ParallelKind::Other
}

fn immediate_expression_kind(expression: &Expression) -> ParallelKind {
    match expression {
        Expression::LiteralExpression(literal) => literal
            .token()
            .map_or(ParallelKind::Other, |token| syntax_kind(token.kind())),
        Expression::AssignmentExpression(assignment) => assignment
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        Expression::ArrayCreationExpression(_) => syntax_kind(JavaSyntaxKind::ArrayInitializer),
        Expression::BinaryExpression(binary) => binary
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        Expression::UnaryExpression(unary) => unary
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        Expression::PostfixExpression(postfix) => postfix
            .operator()
            .map_or(ParallelKind::Other, |operator| syntax_kind(operator.kind())),
        _ => syntax_kind(expression.kind()),
    }
}

fn syntax_kind(kind: JavaSyntaxKind) -> ParallelKind {
    ParallelKind::Syntax(kind)
}

fn annotation_value_row_weight(value: &AnnotationElementValue) -> usize {
    if let Some(initializer) = value.array_initializer() {
        return initializer
            .values()
            .map(|value| annotation_value_row_weight(&value))
            .sum();
    }
    1
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
