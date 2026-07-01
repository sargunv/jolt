use jolt_fmt_ir::{Doc, concat, group, indent, line, nil, soft_line, text};

pub(crate) fn comma_list(items: Vec<Doc>) -> Doc {
    if items.is_empty() {
        return nil();
    }

    jolt_fmt_ir::join(concat([text(","), line()]), items)
}

pub(crate) fn semicolon_list(items: Vec<Doc>) -> Doc {
    jolt_fmt_ir::join(concat([text(";"), line()]), items)
}

pub(crate) fn parenthesized_list(items: Vec<Doc>) -> Doc {
    delimited_comma_list("(", items, ")")
}

fn delimited_comma_list(open: &'static str, items: Vec<Doc>, close: &'static str) -> Doc {
    if items.is_empty() {
        return concat([text(open), text(close)]);
    }

    group(concat([
        text(open),
        indent(concat([soft_line(), comma_list(items)])),
        soft_line(),
        text(close),
    ]))
}
