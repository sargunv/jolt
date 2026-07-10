use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::KotlinSyntaxToken;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};

pub(crate) fn modifier_prefix_from_parts<'source, Modifiers>(
    doc: &mut DocBuilder<'source>,
    annotation_docs: impl IntoIterator<Item = Doc<'source>>,
    modifier_tokens: Modifiers,
) -> Doc<'source>
where
    Modifiers: IntoIterator<Item = KotlinSyntaxToken<'source>>,
{
    let mut annotation_docs = annotation_docs.into_iter().peekable();
    let modifier_docs = modifier_tokens
        .into_iter()
        .map(|token| format_modifier_token(doc, &token))
        .collect::<Vec<_>>();
    if annotation_docs.peek().is_none() && modifier_docs.is_empty() {
        return doc.nil();
    }

    doc.concat_list(|docs| {
        for annotation in annotation_docs {
            docs.push(annotation);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
        if !modifier_docs.is_empty() {
            let space = docs.space();
            let modifiers = docs.join(space, modifier_docs);
            docs.push(modifiers);
            let space = docs.space();
            docs.push(space);
        }
    })
}

fn format_modifier_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}
