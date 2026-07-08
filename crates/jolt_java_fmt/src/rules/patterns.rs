use jolt_fmt_ir::{Doc, concat};
use jolt_java_syntax::{ComponentPattern, MatchAllPattern, Pattern, RecordPattern, TypePattern};

use crate::context::JavaFormatter;
use crate::helpers::comments::{LeadingTrivia, format_token_sequence, format_token_with_comments};
use crate::helpers::lists::{CommaListItem, parenthesized_list, recovered_comma_list_items};
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_pattern<'source>(
    pattern: &Pattern<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern, formatter),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern, formatter),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern, formatter),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern),
    }
}

fn format_type_pattern<'source>(
    pattern: &TypePattern<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    pattern
        .variable()
        .map_or_else(jolt_fmt_ir::nil, |variable| {
            format_local_variable_declaration(&variable, formatter)
        })
}

fn format_record_pattern<'source>(
    pattern: &RecordPattern<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        pattern
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_record_pattern_components(pattern, formatter),
    ])
}

fn format_record_pattern_components<'source>(
    pattern: &RecordPattern<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = pattern.open_paren();
    let close = pattern.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        record_pattern_items(pattern, formatter),
    )
}

fn record_pattern_items<'source, 'fmt>(
    pattern: &'fmt RecordPattern<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(pattern.entries_with_recovered(), |entry| CommaListItem {
        doc: format_component_pattern(&entry.component, formatter),
        comma: entry.comma,
    })
}

fn format_component_pattern<'source>(
    pattern: &ComponentPattern<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    pattern.pattern().map_or_else(
        || format_token_sequence(pattern.token_iter(), LeadingTrivia::Preserve),
        |pattern| format_pattern(&pattern, formatter),
    )
}

fn format_match_all_pattern<'source>(pattern: &MatchAllPattern<'source>) -> Doc<'source> {
    pattern
        .underscore()
        .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token))
}
