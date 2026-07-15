use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{ComponentPattern, MatchAllPattern, Pattern, RecordPattern, TypePattern};

use crate::helpers::comments::format_token_with_comments;
use crate::helpers::lists::{parenthesized_list, syntax_comma_list_items};
use crate::helpers::recovery::{
    JavaFormatField, format_malformed, format_required_field, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_pattern<'source>(
    pattern: &Pattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern, doc),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern, doc),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern, doc),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern, doc),
        Pattern::BogusPattern(pattern) => format_malformed(pattern, doc),
    }
}

fn format_type_pattern<'source>(
    pattern: &TypePattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(pattern.declaration(), doc, |declaration, doc| {
        format_local_variable_declaration(&declaration, doc)
    })
}

fn format_record_pattern<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let ty = format_required_field(pattern.r#type(), doc, |ty, doc| {
        format_type(&ty.into(), doc)
    });
    let components = format_record_pattern_components(pattern, doc);
    doc_concat!(doc, [ty, components])
}

fn format_record_pattern_components<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(pattern.open_paren(), doc);
    let close = resolve_required_delimiter(pattern.close_paren(), doc);
    let items = match resolve_required_field(pattern.components(), doc) {
        JavaFormatField::Present(components) => {
            syntax_comma_list_items(doc, components.parts(), |component, doc| {
                format_component_pattern(&component, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![crate::helpers::lists::CommaListItem {
            doc: recovery,
            comma: None,
        }],
    };
    parenthesized_list(doc, open, close, items)
}

fn format_component_pattern<'source>(
    pattern: &ComponentPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(pattern.pattern(), doc, |value, doc| {
        let Some(pattern) = value.cast_family::<Pattern<'source>>() else {
            doc.block_on_invariant("component pattern role was not a pattern");
            return Doc::nil();
        };
        format_pattern(&pattern, doc)
    })
}

fn format_match_all_pattern<'source>(
    pattern: &MatchAllPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(pattern.underscore(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    })
}
