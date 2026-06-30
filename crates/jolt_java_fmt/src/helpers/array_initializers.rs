use jolt_fmt_ir::TextWidth;
use jolt_fmt_ir::{
    Doc, best_fitting, concat, fill, fill_entry, group, hard_line_without_break_parent, indent,
    indent_by, line, soft_line, text, text_with_width,
};

use crate::helpers::lists::{FormattedList, comment_braced_comma_list};
use crate::policy::JavaFormatPolicy;

pub(crate) fn braced_initializer_block(
    list: FormattedList,
    layout: InitializerLayout,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    if list.has_structural_comments() {
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
                best_fitting(
                    filled_braced_block(docs.clone(), tight_fit),
                    [one_per_line_braced_block(docs)],
                )
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

fn one_per_line_braced_block(docs: Vec<Doc>) -> Doc {
    if docs.is_empty() {
        return text("{}");
    }

    let last = docs.last().cloned().expect("non-empty docs checked above");
    let mut body = docs
        .iter()
        .take(docs.len() - 1)
        .cloned()
        .flat_map(|item| [item, text(","), hard_line_without_break_parent()])
        .collect::<Vec<_>>();
    body.push(last);

    concat([
        text("{"),
        indent(concat([hard_line_without_break_parent(), concat(body)])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}
