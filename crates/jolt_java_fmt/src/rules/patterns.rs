use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{ComponentPattern, MatchAllPattern, Pattern, RecordPattern, TypePattern};

use crate::helpers::comments::{LeadingTrivia, format_token_sequence, format_token_with_comments};
use crate::helpers::lists::{CommaListItem, parenthesized_list, recovered_comma_list_items};
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
    }
}

fn format_type_pattern<'source>(
    pattern: &TypePattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match pattern.variable() {
        Some(variable) => format_local_variable_declaration(&variable, doc),
        None => Doc::nil(),
    }
}

fn format_record_pattern<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let ty = match pattern.ty() {
        Some(ty) => format_type(&ty, doc),
        None => Doc::nil(),
    };
    let components = format_record_pattern_components(pattern, doc);
    doc_concat!(doc, [ty, components])
}

fn format_record_pattern_components<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = pattern.open_paren();
    let close = pattern.close_paren();
    let items = record_pattern_items(pattern, doc);
    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn record_pattern_items<'source, 'fmt>(
    pattern: &'fmt RecordPattern<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, pattern.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_component_pattern(&entry.component, doc),
            comma: entry.comma,
        }
    })
}

fn format_component_pattern<'source>(
    pattern: &ComponentPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match pattern.pattern() {
        Some(pattern) => format_pattern(&pattern, doc),
        None => format_token_sequence(doc, pattern.token_iter(), LeadingTrivia::Preserve),
    }
}

fn format_match_all_pattern<'source>(
    pattern: &MatchAllPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match pattern.underscore() {
        Some(token) => format_token_with_comments(doc, &token),
        None => Doc::nil(),
    }
}
