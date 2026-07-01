use jolt_fmt_ir::{Doc, concat, text};
use jolt_java_syntax::{ComponentPattern, MatchAllPattern, Pattern, RecordPattern, TypePattern};

use crate::helpers::comments::format_token_text;
use crate::helpers::lists::{CommaListItem, parenthesized_list};
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_pattern(pattern: &Pattern) -> Doc {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern),
    }
}

fn format_type_pattern(pattern: &TypePattern) -> Doc {
    pattern
        .variable()
        .map_or_else(jolt_fmt_ir::nil, |variable| {
            format_local_variable_declaration(&variable)
        })
}

fn format_record_pattern(pattern: &RecordPattern) -> Doc {
    concat([
        pattern
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        format_record_pattern_components(pattern),
    ])
}

fn format_record_pattern_components(pattern: &RecordPattern) -> Doc {
    let open = pattern.open_paren();
    let close = pattern.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        pattern
            .entries()
            .map(|entry| CommaListItem {
                doc: format_component_pattern(&entry.component),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_component_pattern(pattern: &ComponentPattern) -> Doc {
    pattern
        .pattern()
        .map_or_else(jolt_fmt_ir::nil, |pattern| format_pattern(&pattern))
}

fn format_match_all_pattern(pattern: &MatchAllPattern) -> Doc {
    pattern
        .underscore()
        .map_or_else(|| text("_"), |token| format_token_text(token.text()))
}
