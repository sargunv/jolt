use super::{Doc, JavaSyntaxToken, text};
use jolt_fmt_ir::literal_text;

pub(super) fn format_token(token: &JavaSyntaxToken) -> Doc {
    text(token.text())
}

pub(super) fn format_multiline_token(token: &JavaSyntaxToken) -> Doc {
    literal_text(token.text())
}
