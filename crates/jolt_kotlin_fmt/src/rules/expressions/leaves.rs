use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Expression, KotlinSyntaxToken, LiteralExpression, NameExpression, StringTemplateExpression,
    SuperExpression, ThisExpression,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::rules::types::format_type_argument_list;

use super::format_expression;

pub(super) fn format_literal_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &LiteralExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(token) = expression.literal_token() {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    } else {
        doc.nil()
    }
}

pub(super) fn format_name_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(token) = expression.name_token() {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    } else {
        doc.nil()
    }
}

pub(super) fn format_this_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ThisExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(this) = expression.this_token() else {
        return doc.nil();
    };

    let this = format_token(doc, &this, leading, TrailingTrivia::Preserve);
    let label = format_label_suffix(doc, expression.at_token(), expression.label_token());
    doc.concat([this, label])
}

pub(super) fn format_super_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &SuperExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(super_token) = expression.super_token() else {
        return doc.nil();
    };

    let super_token = format_token(doc, &super_token, leading, TrailingTrivia::Preserve);
    let arguments = if let Some(arguments) = expression.type_argument_list() {
        format_type_argument_list(doc, &arguments)
    } else {
        doc.nil()
    };
    let label = format_label_suffix(doc, expression.at_token(), expression.label_token());
    doc.concat([super_token, arguments, label])
}

fn format_label_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    at: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
    label: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let at = if let Some(at) = at {
        format_token(doc, &at, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    } else {
        doc.nil()
    };
    let label = if let Some(label) = label {
        format_token(
            doc,
            &label,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([at, label])
}

pub(super) fn format_string_template_expression<'source>(
    doc: &mut DocBuilder<'source>,
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

    let tokens = expression.token_iter();
    let mut skip_until = None;
    let first_token = expression
        .first_token()
        .map(|token| token.token_text_range());
    doc.concat_list(|docs| {
        for token in tokens {
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
                let start =
                    format_token(docs, &entry.start, token_leading, TrailingTrivia::Preserve);
                docs.push(start);
                let expression = format_expression(docs, &entry.expression);
                docs.push(expression);
                let end = format_token(
                    docs,
                    &entry.end,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::Preserve,
                );
                docs.push(end);
                skip_until = Some(entry.end.token_text_range().end().get());
            } else {
                let token = format_token(docs, &token, token_leading, TrailingTrivia::Preserve);
                docs.push(token);
            }
        }
    })
}

struct LongTemplateEntry<'source> {
    start: KotlinSyntaxToken<'source>,
    end: KotlinSyntaxToken<'source>,
    expression: Expression<'source>,
}
