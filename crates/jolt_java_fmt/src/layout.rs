use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, group, hard_line, indent, join, line, soft_line, text,
};

use crate::helpers::lists as java_lists;

pub(crate) fn space_separated(parts: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(line(), parts))
}

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    java_lists::comma_list(items)
}

pub(crate) fn braced_block(items: impl IntoIterator<Item = Doc>) -> Doc {
    braced_block_with_separator(items, hard_line())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BracedBodyLayout {
    pub leading_blank_line: bool,
    pub trailing_blank_line: bool,
}

pub(crate) fn braced_body(items: Vec<Doc>, separators: Vec<Doc>, layout: BracedBodyLayout) -> Doc {
    let BracedBodyLayout {
        leading_blank_line,
        trailing_blank_line,
    } = layout;

    if items.is_empty() && !leading_blank_line && !trailing_blank_line {
        return concat([text("{"), hard_line(), text("}")]);
    }

    let mut body = Vec::new();
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        body.push(first);
    }
    for (separator, item) in separators.into_iter().zip(items) {
        body.push(separator);
        body.push(item);
    }

    if body.is_empty() {
        return concat([text("{"), hard_line(), text("}")]);
    }

    let mut parts = vec![text("{")];
    if leading_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    parts.push(indent(concat([hard_line(), concat(body)])));
    if trailing_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    parts.push(hard_line());
    parts.push(text("}"));
    concat(parts)
}

pub(crate) fn braced_block_with_separators(
    items: impl IntoIterator<Item = Doc>,
    separators: impl IntoIterator<Item = Doc>,
) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    let separators = separators.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    let mut body = Vec::new();
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        body.push(first);
    }
    for (separator, item) in separators.into_iter().zip(items) {
        body.push(separator);
        body.push(item);
    }

    concat([
        text("{"),
        indent(concat([hard_line(), concat(body)])),
        hard_line(),
        text("}"),
    ])
}

fn braced_block_with_separator(items: impl IntoIterator<Item = Doc>, separator: Doc) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    concat([
        text("{"),
        indent(concat([hard_line(), join(separator, items)])),
        hard_line(),
        text("}"),
    ])
}
