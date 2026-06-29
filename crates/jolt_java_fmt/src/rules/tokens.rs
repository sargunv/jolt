use super::{Doc, JavaSyntaxToken, text};

pub(super) fn format_token(token: &JavaSyntaxToken) -> Doc {
    text(token.text())
}
