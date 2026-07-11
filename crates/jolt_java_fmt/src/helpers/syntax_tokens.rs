use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token_doc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FormatterInsertedToken {
    BlockBrace,
    PrecedenceParenthesis,
    TrailingComma,
    EnumSeparator,
}

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    spelling: &'static str,
    _reason: FormatterInsertedToken,
) -> Doc<'source> {
    doc.text(spelling)
}

pub(crate) fn format_token_with_normalized_text<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    spelling: &'static str,
    reason: FormatterInsertedToken,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    // Source-backed normalized token: the source token provides trivia;
    // formatter policy provides the printed spelling.
    let token_doc = inserted_syntax_token(doc, spelling, reason);
    format_token_doc(doc, token, token_doc, leading, trailing)
}
