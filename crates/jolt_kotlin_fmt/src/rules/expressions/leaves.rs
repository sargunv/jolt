use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{
    Expression, KotlinSyntaxToken, LiteralExpression, NameExpression, StringTemplateExpression,
    SuperExpression, ThisExpression,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::rules::types::format_type_argument_list;

use super::format_expression;

pub(super) fn format_literal_expression<'source>(
    expression: &LiteralExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    expression
        .literal_token()
        .map_or_else(jolt_fmt_ir::nil, |token| {
            format_token(&token, leading, TrailingTrivia::Preserve)
        })
}

pub(super) fn format_name_expression<'source>(
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    expression
        .name_token()
        .map_or_else(jolt_fmt_ir::nil, |token| {
            format_token(&token, leading, TrailingTrivia::Preserve)
        })
}

pub(super) fn format_this_expression<'source>(
    expression: &ThisExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(this) = expression.this_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(&this, leading, TrailingTrivia::Preserve),
        format_label_suffix(expression.at_token(), expression.label_token()),
    ])
}

pub(super) fn format_super_expression<'source>(
    expression: &SuperExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(super_token) = expression.super_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(&super_token, leading, TrailingTrivia::Preserve),
        expression
            .type_argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
            }),
        format_label_suffix(expression.at_token(), expression.label_token()),
    ])
}

fn format_label_suffix<'source>(
    at: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
    label: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        at.map_or_else(jolt_fmt_ir::nil, |at| {
            format_token(&at, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            format_token(&label, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
    ])
}

pub(super) fn format_string_template_expression<'source>(
    expression: &StringTemplateExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let mut long_entries = expression
        .parts()
        .filter_map(|part| {
            Some(LongTemplateEntry {
                start: part.long_entry_start()?,
                end: part.long_entry_end()?,
                expression: part.expression()?,
            })
        })
        .collect::<Vec<_>>();
    long_entries.sort_by_key(|entry| entry.start.token_text_range().start());

    let mut docs = Vec::new();
    let mut skip_until = None;
    let first_token = expression
        .first_token()
        .map(|token| token.token_text_range());
    for token in expression.token_iter() {
        let token_leading = if Some(token.token_text_range()) == first_token {
            leading
        } else {
            LeadingTrivia::Preserve
        };
        if skip_until.is_some_and(|end| token.token_text_range().start().get() < end) {
            continue;
        }
        skip_until = None;

        if let Some(entry) = long_entries
            .iter()
            .find(|entry| entry.start.token_text_range() == token.token_text_range())
        {
            docs.push(format_token(
                &entry.start,
                token_leading,
                TrailingTrivia::Preserve,
            ));
            docs.push(format_expression(&entry.expression));
            docs.push(format_token(
                &entry.end,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::Preserve,
            ));
            skip_until = Some(entry.end.token_text_range().end().get());
        } else {
            docs.push(format_token(
                &token,
                token_leading,
                TrailingTrivia::Preserve,
            ));
        }
    }

    concat(docs)
}

struct LongTemplateEntry<'source> {
    start: KotlinSyntaxToken<'source>,
    end: KotlinSyntaxToken<'source>,
    expression: Expression<'source>,
}
