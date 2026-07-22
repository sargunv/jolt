use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxToken, ReplacementClaim, SynthesisClaim};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token_doc};

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    claim: SynthesisClaim<'source>,
) -> Doc<'source> {
    doc.synthesized_source(claim)
}

pub(crate) fn format_token_with_normalized_text<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    claim: ReplacementClaim<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    // Source-backed normalized token: the source token provides trivia;
    // formatter policy provides the printed spelling.
    let token_doc = doc.replaced_source(claim);
    format_token_doc(doc, token, token_doc, leading, trailing)
}
