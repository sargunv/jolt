use jolt_fmt_ir::{Doc, text};

pub(crate) fn qualified_name(segments: Vec<Doc>) -> Doc {
    jolt_fmt_ir::join(text("."), segments)
}
