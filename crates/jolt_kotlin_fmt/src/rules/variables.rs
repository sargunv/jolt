use jolt_fmt_ir::{Doc, concat, space};
use jolt_kotlin_syntax::{KotlinSyntaxToken, ValueParameter, ValueParameterList};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, parenthesized_list, recovered_comma_list_items};
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;

pub(crate) fn format_value_parameter_list<'source>(
    list: &ValueParameterList<'source>,
) -> Doc<'source> {
    let items = value_parameter_list_items(list);
    let open = list.open_paren();
    let close = list.close_paren();

    parenthesized_list(open.as_ref(), close.as_ref(), items)
}

fn value_parameter_list_items<'source>(
    list: &ValueParameterList<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(list.parameter_entries_with_recovered(), |entry| {
        CommaListItem {
            doc: format_value_parameter(&entry.parameter),
            comma: entry.comma,
        }
    })
}

fn format_value_parameter<'source>(parameter: &ValueParameter<'source>) -> Doc<'source> {
    concat([
        parameter
            .modifiers()
            .map_or_else(jolt_fmt_ir::nil, |modifiers| {
                modifier_prefix_from_parts(
                    modifiers
                        .annotations()
                        .map(|annotation| format_annotation(&annotation))
                        .collect(),
                    modifiers.modifier_tokens(),
                )
            }),
        parameter
            .val_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_parameter_keyword(&token), space()])
            }),
        parameter
            .var_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_parameter_keyword(&token), space()])
            }),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        parameter.colon().map_or_else(jolt_fmt_ir::nil, |colon| {
            concat([
                format_token(&colon, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                space(),
            ])
        }),
        parameter
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type_reference(&ty)),
        parameter
            .assign_token()
            .map_or_else(jolt_fmt_ir::nil, |assign| {
                concat([
                    space(),
                    format_token(&assign, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                    space(),
                    parameter
                        .expression()
                        .map_or_else(jolt_fmt_ir::nil, |expression| {
                            format_expression(&expression)
                        }),
                ])
            }),
    ])
}

fn format_parameter_keyword<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}
