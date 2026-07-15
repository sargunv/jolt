use jolt_fmt_ir::{Doc, DocBuilder, ReplacementClaim, SynthesisClaim};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token_doc};
use crate::helpers::lexical_safety::JavaLexicalSafety;

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    claim: SynthesisClaim<'source>,
) -> Doc<'source> {
    let fragment = doc.synthesized_source(claim);
    doc.resolve_exceptional(fragment, None, None, &mut JavaLexicalSafety)
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
    let fragment = doc.replaced_source(claim);
    let token_doc = doc.resolve_exceptional(fragment, None, None, &mut JavaLexicalSafety);
    format_token_doc(doc, token, token_doc, leading, trailing)
}
