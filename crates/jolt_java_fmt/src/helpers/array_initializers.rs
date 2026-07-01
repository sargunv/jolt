use jolt_fmt_ir::TextWidth;
use jolt_fmt_ir::{
    Doc, FlatLine, LevelBreakMode, break_level, break_level_with_indent, concat, fill, fill_entry,
    flat_text, group, hard_line_without_break_parent, indent, indent_by, level_break, line,
    soft_line, text, text_with_width,
};
use jolt_java_syntax::VariableInitializerValue;

use crate::analyzers::array_initializers::{
    TabularEntry, has_only_short_entries, row_opens_without_extra_indent, tabular_entries,
    tabular_layout_for_entries,
};
use crate::context::JavaFormatContext;
use crate::helpers::lists::{
    FormattedList, comment_braced_comma_list, tabular_braced_comma_list_with_before_close_comments,
};
use crate::policy::JavaFormatPolicy;

pub(crate) fn braced_initializer_block(
    list: FormattedList,
    layout: InitializerLayout,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    if list.has_structural_comments() {
        if let InitializerLayout::Tabular {
            cols,
            rows,
            rows_nested,
        } = layout
            && let Some(doc) = tabular_braced_comma_list_with_before_close_comments(
                list.clone(),
                cols,
                rows,
                rows_nested,
                has_trailing_comma,
                policy,
            )
        {
            return doc;
        }
        return comment_braced_comma_list(list, has_trailing_comma);
    }

    let docs = list.into_docs();
    if docs.is_empty() {
        return if has_trailing_comma {
            text("{,}")
        } else {
            text("{}")
        };
    }
    match layout {
        InitializerLayout::Tabular {
            cols,
            rows,
            rows_nested,
        } => tabular_braced_block(docs, cols, rows, rows_nested, has_trailing_comma, policy),
        InitializerLayout::Fill {
            short_items,
            tight_fit,
        } => {
            if has_trailing_comma {
                forced_filled_braced_block(docs)
            } else if short_items {
                filled_braced_block(docs, tight_fit)
            } else {
                unified_break_braced_block(docs, tight_fit)
            }
        }
    }
}

pub(crate) enum InitializerLayout {
    Tabular {
        cols: usize,
        rows: Vec<Vec<usize>>,
        rows_nested: Vec<bool>,
    },
    Fill {
        short_items: bool,
        tight_fit: bool,
    },
}

pub(crate) fn expression_initializer_layout(
    values: &[VariableInitializerValue],
    context: &JavaFormatContext<'_>,
    policy: JavaFormatPolicy,
) -> InitializerLayout {
    let entries = tabular_entries(values);
    initializer_layout_for_entries(&entries, context, policy, FillTightFit::ScalarRows)
}

pub(crate) fn annotation_initializer_layout(
    entries: &[TabularEntry],
    context: &JavaFormatContext<'_>,
    policy: JavaFormatPolicy,
) -> InitializerLayout {
    initializer_layout_for_entries(entries, context, policy, FillTightFit::NonNestedRows)
}

enum FillTightFit {
    ScalarRows,
    NonNestedRows,
}

fn initializer_layout_for_entries(
    entries: &[TabularEntry],
    context: &JavaFormatContext<'_>,
    policy: JavaFormatPolicy,
    tight_fit: FillTightFit,
) -> InitializerLayout {
    if let Some(tabular) = tabular_layout_for_entries(entries, context) {
        let rows_nested = tabular
            .rows
            .iter()
            .map(|row| row_opens_without_extra_indent(entries, row, tabular.cols))
            .collect();
        return InitializerLayout::Tabular {
            cols: tabular.cols,
            rows: tabular.rows,
            rows_nested,
        };
    }

    InitializerLayout::Fill {
        short_items: has_only_short_entries(entries, policy),
        tight_fit: entries.len() >= policy.array_initializer_tight_fit_min_items()
            && match tight_fit {
                FillTightFit::ScalarRows => entries
                    .iter()
                    .all(|entry| entry.kind.is_scalar_initializer() && entry.row_weight == 1),
                FillTightFit::NonNestedRows => entries.iter().all(|entry| !entry.is_nested_array),
            },
    }
}

fn tabular_braced_block(
    docs: Vec<Doc>,
    cols: usize,
    rows: Vec<Vec<usize>>,
    rows_nested: Vec<bool>,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    if docs.is_empty() {
        return text("{}");
    }

    let row_count = rows.len();
    let mut body = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            body.push(hard_line_without_break_parent());
        }
        let nested = rows_nested.get(row_index).copied().unwrap_or(false);
        let is_last_row = row_index + 1 == row_count;
        let row_items = row
            .iter()
            .map(|&index| docs[index].clone())
            .collect::<Vec<_>>();
        body.push(tabular_row(
            row_items,
            cols,
            nested,
            is_last_row,
            has_trailing_comma,
            policy,
        ));
    }

    concat([
        text("{"),
        indent(concat([hard_line_without_break_parent(), concat(body)])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}

fn tabular_row(
    mut items: Vec<Doc>,
    cols: usize,
    nested_row: bool,
    is_last_row: bool,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    if items.is_empty() {
        return text("");
    }

    let row_indent = if cols == 1 || nested_row {
        0
    } else {
        policy.continuation_indent_levels()
    };

    if items.len() == 1 {
        let item = items.into_iter().next().expect("one item");
        let item = if is_last_row && !has_trailing_comma {
            item
        } else {
            concat([item, text(",")])
        };
        return if row_indent == 0 {
            item
        } else {
            indent_by(row_indent, item)
        };
    }

    let last = items.pop().expect("multiple items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));
    let last_doc = if is_last_row && !has_trailing_comma {
        last
    } else {
        concat([last, text(",")])
    };
    let body = fill(entries, last_doc);

    if row_indent == 0 {
        group(body)
    } else {
        group(indent_by(row_indent, body))
    }
}

fn forced_filled_braced_block(docs: Vec<Doc>) -> Doc {
    if docs.is_empty() {
        return text("{,}");
    }

    let last = docs.last().cloned().expect("non-empty docs checked above");
    let entries = docs
        .iter()
        .take(docs.len() - 1)
        .cloned()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    concat([
        text("{"),
        indent(concat([
            hard_line_without_break_parent(),
            fill(entries, concat([last, text(",")])),
        ])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}

fn filled_braced_block(docs: Vec<Doc>, tight_fit: bool) -> Doc {
    if docs.is_empty() {
        return text("{}");
    }

    let last = docs.last().cloned().expect("non-empty docs checked above");
    let entries = docs
        .iter()
        .take(docs.len() - 1)
        .cloned()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    // Dense scalar initializers in GJF behave as if exact-width rows still break.
    // Reserve one invisible column so exact fits choose the next row.
    let fit_guard = if tight_fit {
        text_with_width("", TextWidth::new(1))
    } else {
        text("")
    };

    group(concat([
        text("{"),
        indent(concat([soft_line(), fit_guard, fill(entries, last)])),
        soft_line(),
        text("}"),
    ]))
}

/// Long non-tabular items: inline comma layout when the level fits, else one per line.
fn unified_break_braced_block(docs: Vec<Doc>, tight_fit: bool) -> Doc {
    if docs.is_empty() {
        return text("{}");
    }

    let fit_guard = if tight_fit {
        text_with_width("", TextWidth::new(1))
    } else {
        text("")
    };
    let list = braced_list_comma_level(docs);

    break_level_with_indent(
        1,
        [text("{"), concat([fit_guard, list]), text("}")],
        [
            level_break(LevelBreakMode::Unified, FlatLine::Empty, 0),
            level_break(LevelBreakMode::Unified, FlatLine::Empty, 0),
        ],
    )
    .expect("valid braced initializer break level")
}

fn braced_list_comma_level(mut docs: Vec<Doc>) -> Doc {
    let last = docs.pop().expect("non-empty docs checked above");
    if docs.is_empty() {
        return last;
    }

    let comma_items = docs
        .into_iter()
        .map(|item| concat([item, text(",")]))
        .collect::<Vec<_>>();
    let breaks = vec![level_break(LevelBreakMode::Unified, flat_text(" "), 0); comma_items.len()];
    break_level(comma_items.into_iter().chain(std::iter::once(last)), breaks)
        .expect("valid braced initializer comma level")
}
