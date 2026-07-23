use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinSyntaxToken, ValueParameter, ValueParameterList, ValueParameterListEntry,
    ValueParameterName,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, delimited_comma_list, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatField, format_optional_field, format_required_field, join_delimited_recovery,
    resolve_required_delimiter, resolve_required_field,
};
use crate::rules::declarations::format_destructuring_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::types::{
    format_bogus_list_entry, format_modifier_sequence, format_type_reference,
};

pub(crate) fn format_value_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ValueParameterList<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(list.open_paren(), doc);
    let close = resolve_required_delimiter(list.close_paren(), doc);
    let items = match resolve_required_field(list.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            physical_comma_list_items(doc, entries.parts(), |doc, parameter| {
                CommaListItem::visible(match parameter {
                    ValueParameterListEntry::ValueParameter(parameter) => {
                        format_value_parameter(doc, &parameter)
                    }
                    ValueParameterListEntry::BogusValueParameter(bogus) => {
                        format_bogus_list_entry(doc, &bogus)
                    }
                })
            })
        }
        KotlinFormatField::Malformed(recovery) => vec![CommaListItem::visible(recovery)],
    };
    let list = delimited_comma_list(doc, open.source(), close.source(), items);
    join_delimited_recovery(doc, &open, list, &close)
}

fn format_value_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &ValueParameter<'source>,
) -> Doc<'source> {
    let has_name = matches!(
        parameter.name(),
        jolt_kotlin_syntax::KotlinSyntaxField::Present(_)
    );
    let has_colon = matches!(
        parameter.colon(),
        jolt_kotlin_syntax::KotlinSyntaxField::Present(_)
    );
    let has_type = matches!(
        parameter.r#type(),
        jolt_kotlin_syntax::KotlinSyntaxField::Present(ty)
            if ty.first_token().is_some()
    );
    let has_assign = matches!(
        parameter.assign(),
        jolt_kotlin_syntax::KotlinSyntaxField::Present(_)
    );
    let modifiers = format_required_field(parameter.modifiers(), doc, |modifiers, doc| {
        format_modifier_sequence(doc, &modifiers)
    });
    let parameter_keyword =
        format_optional_field(parameter.parameter_keyword(), doc, |role, doc| {
            let keyword = format_parameter_keyword(doc, &role);
            let space = doc.space();
            doc.concat([keyword, space])
        });
    let name = format_required_field(parameter.name(), doc, |name, doc| {
        format_parameter_name(doc, name)
    });
    let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        if has_type {
            let space = doc.space();
            doc.concat([colon, space])
        } else {
            colon
        }
    });
    let ty = format_optional_field(parameter.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    let missing_colon_space = if has_name && !has_colon && has_type {
        doc.space()
    } else {
        Doc::nil()
    };
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
        let expression = format_expression(doc, &expression);
        if has_assign {
            expression
        } else {
            let space = doc.space();
            doc.concat([space, expression])
        }
    });
    doc.concat([
        modifiers,
        parameter_keyword,
        name,
        colon,
        missing_colon_space,
        ty,
        assign,
        default,
    ])
}

fn format_parameter_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: ValueParameterName<'source>,
) -> Doc<'source> {
    match name {
        ValueParameterName::Name(name) => format_name(doc, &name),
        ValueParameterName::DestructuringDeclaration(pattern) => {
            format_destructuring_declaration(doc, &pattern)
        }
        ValueParameterName::BogusValueParameterName(bogus) => {
            crate::helpers::recovery::format_malformed(&bogus, doc)
        }
    }
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
