use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, text};

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

    pub(crate) fn without_blank_line_before(self) -> Self {
        Self {
            starts_after_blank_line: false,
            ..self
        }
    }
}

pub(crate) fn braced_body(body: Option<Doc>) -> Doc {
    concat([text("{"), braced_body_tail(body)])
}

pub(crate) fn braced_body_tail(body: Option<Doc>) -> Doc {
    concat([
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
    join_docs(docs, &hard_line())
}

pub(crate) fn join_empty_lines(docs: Vec<Doc>) -> Doc {
    join_docs(docs, &empty_line())
}

fn join_docs(docs: Vec<Doc>, separator: &Doc) -> Doc {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(separator.clone());
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
