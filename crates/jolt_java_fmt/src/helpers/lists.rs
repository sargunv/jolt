use jolt_fmt_ir::{Doc, concat, group, if_break, indent, line, nil, soft_line, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrailingSeparator {
    Never,
    WhenBroken,
}

pub(crate) fn comma_list(items: Vec<Doc>, trailing: TrailingSeparator) -> Doc {
    if items.is_empty() {
        return nil();
    }

    concat([
        jolt_fmt_ir::join(concat([text(","), line()]), items),
        match trailing {
            TrailingSeparator::Never => nil(),
            TrailingSeparator::WhenBroken => if_break(text(","), nil()),
        },
    ])
}

pub(crate) fn semicolon_list(items: Vec<Doc>) -> Doc {
    jolt_fmt_ir::join(concat([text(";"), line()]), items)
}

pub(crate) fn parenthesized_list(items: Vec<Doc>) -> Doc {
    delimited_comma_list("(", items, ")", TrailingSeparator::Never)
}

pub(crate) fn angle_bracket_list(items: Vec<Doc>) -> Doc {
    delimited_comma_list("<", items, ">", TrailingSeparator::Never)
}

pub(crate) fn braced_initializer_list(items: Vec<Doc>) -> Doc {
    if items.is_empty() {
        return text("{}");
    }

    delimited_comma_list("{", items, "}", TrailingSeparator::WhenBroken)
}

fn delimited_comma_list(
    open: &'static str,
    items: Vec<Doc>,
    close: &'static str,
    trailing: TrailingSeparator,
) -> Doc {
    if items.is_empty() {
        return concat([text(open), text(close)]);
    }

    group(concat([
        text(open),
        indent(concat([soft_line(), comma_list(items, trailing)])),
        soft_line(),
        text(close),
    ]))
}
