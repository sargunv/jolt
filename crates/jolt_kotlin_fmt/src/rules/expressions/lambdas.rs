use jolt_fmt_ir::{Doc, concat, group, hard_line, if_break, indent, space};
use jolt_kotlin_syntax::{
    BlockItem, DestructuringDeclaration, LambdaExpression, LambdaParameter, LambdaParameterList,
    RecoveredSeparatedListEntry,
};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_separator_with_comments, format_token,
    format_token_sequence, token_has_comments,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, recovered_comma_list_items,
};
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
    let mut entries =
        recovered_comma_list_items(parameter_list.parameter_entries_with_recovered(), |entry| {
            CommaListItem {
                doc: format_lambda_parameter(&entry.parameter),
                comma: entry.comma,
            }
        })
        .into_iter()
        .peekable();
    let (lower, _) = entries.size_hint();
    let mut docs = Vec::with_capacity(lower.saturating_mul(2).saturating_add(1));

    while let Some(entry) = entries.next() {
        docs.push(entry.doc);
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, space()));
        } else if entries.peek().is_some() {
            docs.push(space());
        }
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
        recovered_comma_list_items(declaration.entries_with_recovered(), |entry| {
            CommaListItem {
                doc: entry
                    .entry
                    .name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
                comma: entry.comma,
            }
        }),
    )
}

pub(super) fn lambda_body_docs<'source>(
    lambda: &LambdaExpression<'source>,
    items: &[BlockItem<'source>],
) -> Vec<Doc<'source>> {
    let mut docs = Vec::with_capacity(items.len());
    let mut recovered_docs = Vec::new();

    for entry in lambda.body_items_with_recovered() {
        match entry {
            RecoveredSeparatedListEntry::Entry(item) => {
                push_recovered_lambda_docs(&mut docs, &mut recovered_docs);
                docs.push(format_block_item(&item));
            }
            RecoveredSeparatedListEntry::Token(token) => recovered_docs.push(
                format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve),
            ),
            RecoveredSeparatedListEntry::Error(error) => recovered_docs.push(
                format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
            ),
            RecoveredSeparatedListEntry::Node(node) => recovered_docs.push(format_token_sequence(
                node.token_iter(),
                LeadingTrivia::Preserve,
            )),
        }
    }

    push_recovered_lambda_docs(&mut docs, &mut recovered_docs);

    if docs.is_empty() {
        return items.iter().map(format_block_item).collect();
    }

    docs
}

fn push_recovered_lambda_docs<'source>(
    docs: &mut Vec<Doc<'source>>,
    recovered_docs: &mut Vec<Doc<'source>>,
) {
    if recovered_docs.is_empty() {
        return;
    }

    docs.push(concat(std::mem::take(recovered_docs)));
}
