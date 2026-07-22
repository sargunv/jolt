use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_syntax::SynthesisClaim;

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    claim: SynthesisClaim<'source>,
) -> Doc<'source> {
    doc.synthesized_source(claim)
}
