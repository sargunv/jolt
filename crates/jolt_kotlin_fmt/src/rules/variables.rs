use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxToken, ValueParameter, ValueParameterList};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, parenthesized_list, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, format_optional_field, format_or_verbatim,
    format_required_field, resolve_required_delimiter, resolve_required_field,
};
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::types::{format_modifier_sequence, format_type_reference};

pub(crate) fn format_value_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ValueParameterList<'source>,
) -> Doc<'source> {
    format_or_verbatim(list, doc, |doc| {
        let open = resolve_required_delimiter(list.open_paren(), doc);
        let close = resolve_required_delimiter(list.close_paren(), doc);
        let items = match resolve_required_field(list.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, parameter| CommaListItem {
                    doc: format_value_parameter(doc, &parameter),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
                doc: recovery,
                comma: None,
            }],
        };
        format_parenthesized_delimiters(doc, &open, &close, items)
    })
}

fn format_value_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &ValueParameter<'source>,
) -> Doc<'source> {
    format_or_verbatim(parameter, doc, |doc| {
        let modifiers = format_required_field(parameter.modifiers(), doc, |modifiers, doc| {
            format_modifier_sequence(doc, &modifiers)
        });
        let property_keyword =
            format_optional_field(parameter.property_keyword(), doc, |role, doc| {
                let keyword = format_parameter_keyword(doc, &role);
                let space = doc.space();
                doc.concat([keyword, space])
            });
        let name =
            format_required_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
        let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
            let colon = format_token(
                doc,
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let space = doc.space();
            doc.concat([colon, space])
        });
        let ty = format_optional_field(parameter.r#type(), doc, |ty, doc| {
            format_type_reference(doc, &ty)
        });
        let assign = format_optional_field(parameter.assign(), doc, |assign, doc| {
            let before = doc.space();
            let assign = format_token(
                doc,
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let after = doc.space();
            doc.concat([before, assign, after])
        });
        let default = format_optional_field(parameter.default(), doc, |expression, doc| {
            format_expression(doc, &expression)
        });
        doc.concat([
            modifiers,
            property_keyword,
            name,
            colon,
            ty,
            assign,
            default,
        ])
    })
}

fn format_parameter_keyword<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

fn delimiter_recovery<'source>(delimiter: &KotlinFormatDelimiter<'source>) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(recovery) => *recovery,
    }
}

fn format_parenthesized_delimiters<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    close: &KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let open_recovery = delimiter_recovery(open);
    let close_recovery = delimiter_recovery(close);
    let list = parenthesized_list(doc, open.source(), close.source(), items);
    doc.concat([open_recovery, list, close_recovery])
}
