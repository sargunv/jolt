use jolt_fmt_ir::{Doc, concat, text};
use jolt_java_syntax::{ComponentPattern, MatchAllPattern, Pattern, RecordPattern, TypePattern};

use crate::context::JavaFormatter;
use crate::helpers::comments::format_token_text;
use crate::helpers::lists::{CommaListItem, parenthesized_list};
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_pattern(pattern: &Pattern, formatter: &JavaFormatter<'_>) -> Doc {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern, formatter),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern, formatter),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern, formatter),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern),
    }
}

fn format_type_pattern(pattern: &TypePattern, formatter: &JavaFormatter<'_>) -> Doc {
    pattern
        .variable()
        .map_or_else(jolt_fmt_ir::nil, |variable| {
            format_local_variable_declaration(&variable, formatter)
        })
}

fn format_record_pattern(pattern: &RecordPattern, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        pattern
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_record_pattern_components(pattern, formatter),
    ])
}

fn format_record_pattern_components(pattern: &RecordPattern, formatter: &JavaFormatter<'_>) -> Doc {
    let open = pattern.open_paren();
    let close = pattern.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        pattern
            .entries()
            .map(|entry| CommaListItem {
                doc: format_component_pattern(&entry.component, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_component_pattern(pattern: &ComponentPattern, formatter: &JavaFormatter<'_>) -> Doc {
    pattern.pattern().map_or_else(jolt_fmt_ir::nil, |pattern| {
        format_pattern(&pattern, formatter)
    })
}

fn format_match_all_pattern(pattern: &MatchAllPattern) -> Doc {
    pattern
        .underscore()
        .map_or_else(|| text("_"), |token| format_token_text(token.text()))
}
