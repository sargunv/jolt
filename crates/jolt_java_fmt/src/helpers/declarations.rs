use jolt_fmt_ir::{Doc, concat, group, line, text};

use crate::helpers::blocks::braced_body_tail;

pub(crate) fn declaration_with_body(prefix: Doc, header: Doc, body: Option<Doc>) -> Doc {
    concat([
        prefix,
        group(concat([header, line(), text("{")])),
        braced_body_tail(body),
    ])
}

pub(crate) fn declaration_without_body(prefix: Doc, header: Doc) -> Doc {
    concat([prefix, group(header), text(";")])
}
