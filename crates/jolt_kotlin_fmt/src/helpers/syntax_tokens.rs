use jolt_fmt_ir::{Doc, DocBuilder, SynthesisClaim};

use super::lexical_safety::KotlinLexicalSafety;

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    claim: SynthesisClaim<'source>,
) -> Doc<'source> {
    let fragment = doc.synthesized_source(claim);
    doc.resolve_exceptional(fragment, None, None, &mut KotlinLexicalSafety)
}
