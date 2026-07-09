use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxToken, ValueParameter, ValueParameterList};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, parenthesized_list, recovered_comma_list_items};
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;

pub(crate) fn format_value_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ValueParameterList<'source>,
) -> Doc<'source> {
    let items = value_parameter_list_items(doc, list);
    let open = list.open_paren();
    let close = list.close_paren();

    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn value_parameter_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ValueParameterList<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(
        doc,
        list.parameter_entries_with_recovered(),
        |doc, entry| CommaListItem {
            doc: format_value_parameter(doc, &entry.parameter),
            comma: entry.comma,
        },
    )
}

fn format_value_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &ValueParameter<'source>,
) -> Doc<'source> {
    let modifiers = if let Some(modifiers) = parameter.modifiers() {
        let annotations = modifiers
            .annotations()
            .map(|annotation| format_annotation(doc, &annotation))
            .collect::<Vec<_>>();
        modifier_prefix_from_parts(doc, annotations, modifiers.modifier_tokens())
    } else {
        doc.nil()
    };
    let val = if let Some(token) = parameter.val_token() {
        let keyword = format_parameter_keyword(doc, &token);
        let space = doc.space();
        doc.concat([keyword, space])
    } else {
        doc.nil()
    };
    let var = if let Some(token) = parameter.var_token() {
        let keyword = format_parameter_keyword(doc, &token);
        let space = doc.space();
        doc.concat([keyword, space])
    } else {
        doc.nil()
    };
    let name = if let Some(name) = parameter.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let colon = if let Some(colon) = parameter.colon() {
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let space = doc.space();
        doc.concat([colon, space])
    } else {
        doc.nil()
    };
    let ty = if let Some(ty) = parameter.ty() {
        format_type_reference(doc, &ty)
    } else {
        doc.nil()
    };
    let assign = if let Some(assign) = parameter.assign_token() {
        let before = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let after = doc.space();
        let expression = if let Some(expression) = parameter.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
        doc.concat([before, assign, after, expression])
    } else {
        doc.nil()
    };

    doc.concat([modifiers, val, var, name, colon, ty, assign])
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
