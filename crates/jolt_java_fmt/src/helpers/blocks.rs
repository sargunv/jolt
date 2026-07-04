use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, join, text};

pub(crate) struct BodyItem<'source> {
    doc: Doc<'source>,
    starts_after_blank_line: bool,
}

impl<'source> BodyItem<'source> {
    pub(crate) fn new(doc: Doc<'source>, starts_after_blank_line: bool) -> Self {
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

pub(crate) fn braced_body(body: Option<Doc<'_>>) -> Doc<'_> {
    concat([text("{"), braced_body_tail(body)])
}

pub(crate) fn braced_body_tail(body: Option<Doc<'_>>) -> Doc<'_> {
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

pub(crate) fn empty_block<'source>() -> Doc<'source> {
    braced_body(None)
}

pub(crate) fn join_hard_lines<'source>(
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    join(&hard_line(), docs)
}

pub(crate) fn join_empty_lines<'source>(
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    join(&empty_line(), docs)
}

pub(crate) fn join_body_items(items: Vec<BodyItem<'_>>) -> Doc<'_> {
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
