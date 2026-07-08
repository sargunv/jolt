use jolt_fmt_ir::{Doc, concat, hard_line, join, space};
use jolt_kotlin_syntax::KotlinSyntaxToken;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};

pub(crate) fn modifier_prefix_from_parts<'source, Modifiers>(
    annotation_docs: Vec<Doc<'source>>,
    modifier_tokens: Modifiers,
) -> Doc<'source>
where
    Modifiers: IntoIterator<Item = KotlinSyntaxToken<'source>>,
{
    let mut modifier_docs = modifier_tokens
        .into_iter()
        .map(|token| format_modifier_token(&token))
        .peekable();
    if annotation_docs.is_empty() && modifier_docs.peek().is_none() {
        return jolt_fmt_ir::nil();
    }

    let mut docs = Vec::new();
    for annotation in annotation_docs {
        docs.push(annotation);
        docs.push(hard_line());
    }
    if modifier_docs.peek().is_some() {
        docs.push(join(&space(), modifier_docs));
        docs.push(space());
    }

    concat(docs)
}

fn format_modifier_token<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
}
