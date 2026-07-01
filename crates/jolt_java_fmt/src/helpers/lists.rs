use jolt_fmt_ir::{Doc, concat, line, text};

pub(crate) fn semicolon_list(items: Vec<Doc>) -> Doc {
    jolt_fmt_ir::join(concat([text(";"), line()]), items)
}
