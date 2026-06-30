use jolt_fmt_ir::{Doc, concat, fill, fill_entry, group, indent_by, join, line, soft_line, text};

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(concat([text(","), line()]), items))
}

pub(crate) fn delimited_comma_list_flat(
    open: &'static str,
    close: &'static str,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    group(concat([text(open), join(text(", "), items), text(close)]))
}

pub(crate) fn delimited_comma_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let last = items.pop().expect("non-empty items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    group(concat([
        text(open),
        indent_by(
            indent_levels,
            concat([soft_line(), fill(entries, concat([last, text(close)]))]),
        ),
    ]))
}

pub(crate) fn delimited_comma_list_one_per_line(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let last = items.pop().expect("non-empty items checked above");
    let mut body = items
        .into_iter()
        .flat_map(|item| [item, text(","), line()])
        .collect::<Vec<_>>();
    body.push(last);
    body.push(text(close));

    group(concat([
        text(open),
        indent_by(indent_levels, concat([soft_line(), concat(body)])),
    ]))
}

pub(crate) fn keyword_prefixed_comma_list(
    keyword: &'static str,
    continuation_indent_levels: u16,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return text(keyword);
    };

    group(concat([
        text(keyword),
        text(" "),
        first,
        indent_by(
            continuation_indent_levels,
            concat(items.map(|item| concat([text(","), line(), item]))),
        ),
    ]))
}
