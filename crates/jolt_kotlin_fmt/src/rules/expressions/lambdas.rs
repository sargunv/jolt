use jolt_fmt_ir::{Doc, concat, group, hard_line, if_break, indent, space};
use jolt_kotlin_syntax::{
    BlockItem, DestructuringDeclaration, KotlinSyntaxKind, KotlinSyntaxToken, LambdaExpression,
    LambdaParameter, LambdaParameterList,
};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence, token_has_comments,
};
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list};
use crate::helpers::source::source_gap_is_trivia;
use crate::rules::names::format_name;
use crate::rules::statements::format_block_item;
use crate::rules::types::format_type_reference;

pub(super) fn format_lambda_expression<'source>(
    lambda: &LambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(labeled) = format_labeled_lambda_expression(lambda, leading) {
        return labeled;
    }

    let Some(open) = lambda.open_brace() else {
        return jolt_fmt_ir::nil();
    };
    let close = lambda.close_brace();

    let items = lambda.body_items().collect::<Vec<_>>();

    let open = format_token(&open, leading, TrailingTrivia::Preserve);
    let close = close.map_or_else(jolt_fmt_ir::nil, |close| {
        format_token(
            &close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    });

    let parameter_prefix = lambda
        .parameter_list()
        .and_then(|parameters| format_lambda_parameter_prefix(&parameters));
    let item_docs = lambda_body_docs(lambda, &items);
    if item_docs.is_empty() {
        let empty_body_parameters = parameter_prefix.map_or_else(jolt_fmt_ir::nil, |prefix| {
            concat([space(), prefix, space()])
        });
        return concat([open, empty_body_parameters, close]);
    }

    let body_doc_count = item_docs.len();
    let body = join_hard_lines(item_docs);

    let block = concat([
        open.clone(),
        parameter_prefix
            .clone()
            .map_or_else(jolt_fmt_ir::nil, |prefix| concat([space(), prefix])),
        indent(concat([hard_line(), body.clone()])),
        hard_line(),
        close.clone(),
    ]);

    if items.len() == 1 && body_doc_count == 1 {
        let inline = concat([
            open.clone(),
            space(),
            parameter_prefix.map_or_else(jolt_fmt_ir::nil, |prefix| concat([prefix, space()])),
            body,
            space(),
            close.clone(),
        ]);
        return group(if_break(block, inline));
    }

    block
}

fn format_labeled_lambda_expression<'source>(
    lambda: &LambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let inner = lambda.inner_lambda()?;
    let label = lambda.label_token()?;
    let at = lambda.at_token()?;
    let at_has_comments = token_has_comments(&at);

    Some(concat([
        format_token(&label, leading, TrailingTrivia::RelocatedToEnclosingContext),
        format_token(&at, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        if at_has_comments {
            space()
        } else {
            jolt_fmt_ir::nil()
        },
        format_lambda_expression(&inner, LeadingTrivia::SuppressAlreadyHandled),
    ]))
}

fn format_lambda_parameter_prefix<'source>(
    parameter_list: &LambdaParameterList<'source>,
) -> Option<Doc<'source>> {
    let arrow = parameter_list.arrow_token()?;
    let parameters = parameter_list.parameters().collect::<Vec<_>>();
    let commas = comma_tokens_after_parameters(parameter_list, &parameters);
    let mut docs = Vec::new();

    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            if let Some(comma) = commas.get(index - 1).copied().flatten() {
                docs.push(format_token(
                    &comma,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ));
            }
            docs.push(space());
        }
        docs.push(format_lambda_parameter(parameter));
    }

    if !docs.is_empty() {
        docs.push(space());
    }
    docs.push(format_token(
        &arrow,
        LeadingTrivia::SuppressAlreadyHandled,
        TrailingTrivia::RelocatedToEnclosingContext,
    ));

    Some(concat(docs))
}

fn format_lambda_parameter<'source>(parameter: &LambdaParameter<'source>) -> Doc<'source> {
    concat([
        parameter.destructuring_declaration().map_or_else(
            || {
                parameter
                    .name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name))
            },
            |declaration| format_destructuring_declaration(&declaration),
        ),
        parameter.colon().map_or_else(jolt_fmt_ir::nil, |colon| {
            concat([
                format_token(
                    &colon,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ),
                space(),
            ])
        }),
        parameter
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type_reference(&ty)),
    ])
}

fn format_destructuring_declaration<'source>(
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    compact_parenthesized_list(
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        declaration
            .entries_with_commas()
            .map(|entry| CommaListItem {
                doc: entry
                    .entry
                    .name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn comma_tokens_after_parameters<'source>(
    list: &LambdaParameterList<'source>,
    parameters: &[LambdaParameter<'source>],
) -> Vec<Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>> {
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

fn lambda_body_docs<'source>(
    lambda: &LambdaExpression<'source>,
    items: &[BlockItem<'source>],
) -> Vec<Doc<'source>> {
    let Some(open) = lambda.open_brace() else {
        return items.iter().map(format_block_item).collect();
    };
    let body_end = lambda.close_brace().map_or_else(
        || lambda.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    let tokens = lambda.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut docs = Vec::new();
    let mut covered_until = lambda
        .parameter_list()
        .and_then(|parameters| parameters.arrow_token())
        .map_or_else(
            || open.token_text_range().end().get(),
            |arrow| arrow.token_text_range().end().get(),
        );

    for item in items {
        push_uncovered_lambda_tokens(
            &mut docs,
            lambda,
            &tokens,
            &mut token_cursor,
            covered_until,
            item.text_range().start().get(),
        );
        docs.push(format_block_item(item));
        covered_until = item.text_range().end().get();
    }

    push_uncovered_lambda_tokens(
        &mut docs,
        lambda,
        &tokens,
        &mut token_cursor,
        covered_until,
        body_end,
    );
    docs
}

fn push_uncovered_lambda_tokens<'source>(
    docs: &mut Vec<Doc<'source>>,
    lambda: &LambdaExpression<'source>,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) {
    if source_gap_is_trivia(
        lambda.source_text(),
        lambda.text_range().start().get(),
        tokens.iter().copied(),
        start,
        end,
    ) {
        return;
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
        return;
    }

    docs.push(format_token_sequence(gap_tokens, LeadingTrivia::Preserve));
}
