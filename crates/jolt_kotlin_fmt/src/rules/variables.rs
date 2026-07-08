use jolt_fmt_ir::{Doc, concat, space};
use jolt_kotlin_syntax::{KotlinSyntaxKind, KotlinSyntaxToken, ValueParameter, ValueParameterList};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::lists::{CommaListItem, parenthesized_list};
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::helpers::source::source_gap_is_trivia;
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;

pub(crate) fn format_value_parameter_list<'source>(
    list: &ValueParameterList<'source>,
) -> Doc<'source> {
    let ValueParameterListItems {
        items,
        has_recovered_tokens,
    } = value_parameter_list_items(list);
    let open = list.open_paren();
    let close = list.close_paren();
    let source_text = list.source_text();

    if !has_recovered_tokens
        && !source_text.contains(['\n', '\r'])
        && source_text.len() <= 48
        && items.last().is_none_or(|item| item.comma.is_none())
    {
        return crate::helpers::lists::compact_parenthesized_list(
            open.as_ref(),
            close.as_ref(),
            items,
        );
    }

    parenthesized_list(open.as_ref(), close.as_ref(), items)
}

struct ValueParameterListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_recovered_tokens: bool,
}

fn value_parameter_list_items<'source>(
    list: &ValueParameterList<'source>,
) -> ValueParameterListItems<'source> {
    let parameters = list.entries().collect::<Vec<_>>();
    let commas = comma_tokens_after_parameters(list, &parameters);
    let source_start = list.text_range().start().get();
    let source = list.source_text();
    let tokens = list.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut covered_until = list.open_paren().map_or_else(
        || list.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let mut items = Vec::new();
    let mut has_recovered_tokens = false;

    for (index, parameter) in parameters.iter().enumerate() {
        has_recovered_tokens |= push_recovered_value_parameter_gap(
            &mut items,
            source,
            source_start,
            &tokens,
            &mut token_cursor,
            covered_until,
            parameter.text_range().start().get(),
        );
        let comma = commas.get(index).copied().flatten();
        items.push(CommaListItem {
            doc: format_value_parameter(parameter),
            comma,
        });
        covered_until = comma.map_or_else(
            || parameter.text_range().end().get(),
            |comma| comma.token_text_range().end().get(),
        );
    }

    let list_end = list.close_paren().map_or_else(
        || list.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    has_recovered_tokens |= push_recovered_value_parameter_gap(
        &mut items,
        source,
        source_start,
        &tokens,
        &mut token_cursor,
        covered_until,
        list_end,
    );

    ValueParameterListItems {
        items,
        has_recovered_tokens,
    }
}

fn push_recovered_value_parameter_gap<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) -> bool {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return false;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return false;
    }

    items.push(CommaListItem {
        doc: format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        comma: None,
    });
    true
}

fn format_value_parameter<'source>(parameter: &ValueParameter<'source>) -> Doc<'source> {
    concat([
        parameter
            .modifiers()
            .map_or_else(jolt_fmt_ir::nil, |modifiers| {
                let mut modifier_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
                modifier_prefix_from_parts(
                    modifiers
                        .annotations()
                        .map(|annotation| format_annotation(&annotation))
                        .collect(),
                    &mut modifier_tokens,
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

fn comma_tokens_after_parameters<'source>(
    list: &ValueParameterList<'source>,
    parameters: &[ValueParameter<'source>],
) -> Vec<Option<KotlinSyntaxToken<'source>>> {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let end = parameter.text_range().end();
            let next_start = parameters.get(index + 1).map_or_else(
                || list.text_range().end(),
                |parameter| parameter.text_range().start(),
            );
            list.token_iter().find(|token| {
                token.kind() == KotlinSyntaxKind::Comma
                    && token.text_range().start() >= end
                    && token.text_range().start() < next_start
            })
        })
        .collect()
}
