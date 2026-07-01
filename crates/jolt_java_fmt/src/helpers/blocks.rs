use jolt_fmt_ir::{Doc, concat, hard_line, text};

pub(crate) struct BodyItem {
    doc: Doc,
    starts_after_blank_line: bool,
}

impl BodyItem {
    pub(crate) fn new(doc: Doc, starts_after_blank_line: bool) -> Self {
        Self {
            doc,
            starts_after_blank_line,
        }
    }
}

pub(crate) fn braced_block(items: Vec<Doc>) -> Doc {
    braced_body((!items.is_empty()).then(|| join_hard_lines(items)))
}

pub(crate) fn braced_body_items(items: Vec<BodyItem>) -> Doc {
    braced_body((!items.is_empty()).then(|| join_body_items(items)))
}

pub(crate) fn braced_body(body: Option<Doc>) -> Doc {
    concat([
        text("{"),
        body.map_or_else(hard_line, |body| {
            concat([
                jolt_fmt_ir::indent(concat([hard_line(), body])),
                hard_line(),
            ])
        }),
        text("}"),
    ])
}

pub(crate) fn empty_block() -> Doc {
    braced_body(None)
}

pub(crate) fn join_hard_lines(docs: Vec<Doc>) -> Doc {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(hard_line());
        }
        joined.push(doc);
    }
    concat(joined)
}

pub(crate) fn join_body_items(items: Vec<BodyItem>) -> Doc {
    let mut joined = Vec::new();
    for item in items {
        if !joined.is_empty() {
            joined.push(if item.starts_after_blank_line {
                jolt_fmt_ir::empty_line()
            } else {
                hard_line()
            });
        }
        joined.push(item.doc);
    }
    concat(joined)
}
