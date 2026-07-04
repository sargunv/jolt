use jolt_fmt_ir::{Doc, concat, group, text};

use crate::helpers::blocks::braced_body_tail;

pub(crate) fn declaration_with_body<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([
        prefix,
        group(header),
        text(" "),
        text("{"),
        braced_body_tail(body),
    ])
}

pub(crate) fn declaration_without_body<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
) -> Doc<'source> {
    concat([prefix, group(header), text(";")])
}
