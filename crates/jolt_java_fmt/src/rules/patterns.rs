use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    ArrayType, BogusType, ClassType, ComponentPattern, MatchAllPattern, Pattern, PrimitiveType,
    RecordPattern, RecordPatternType, TypePattern, TypePatternType,
};

use crate::helpers::comments::format_token_with_comments;
use crate::helpers::lists::{delimited_comma_list, syntax_comma_list_items};
use crate::helpers::recovery::{
    JavaFormatField, format_malformed, format_optional_field, format_required_field,
    resolve_required_delimiter, resolve_required_field,
};
use crate::rules::modifiers::{TypedModifierPrefix, format_typed_parameter_modifier_prefix};
use crate::rules::types::format_array_dimensions;
use crate::rules::types::format_type;

pub(crate) fn format_pattern<'source>(
    pattern: &Pattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern, doc),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern, doc),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern, doc),
        Pattern::BogusPattern(pattern) => format_malformed(pattern, doc),
    }
}

fn format_type_pattern<'source>(
    pattern: &TypePattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = match resolve_required_field(pattern.modifiers(), doc) {
        JavaFormatField::Present(modifiers) => {
            format_typed_parameter_modifier_prefix(&modifiers, doc)
        }
        JavaFormatField::Malformed(recovery) => TypedModifierPrefix {
            declaration_prefix: recovery,
            type_use_prefix: Doc::nil(),
        },
    };
    let ty = format_required_field(pattern.r#type(), doc, |ty, doc| {
        format_type_pattern_type(ty, doc)
    });
    let name = format_required_field(pattern.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let dimensions = format_optional_field(pattern.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    doc_concat!(
        doc,
        [
            modifiers.declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            doc.space(),
            name,
            dimensions,
        ]
    )
}

fn format_record_pattern<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let ty = format_required_field(pattern.r#type(), doc, |ty, doc| {
        format_record_pattern_type(ty, doc)
    });
    let components = format_record_pattern_components(pattern, doc);
    doc_concat!(doc, [ty, components])
}

fn format_type_pattern_type<'source>(
    ty: TypePatternType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(ty) = ty.cast_node::<ClassType<'source>>() {
        format_type(&ty.into(), doc)
    } else if let Some(ty) = ty.cast_node::<PrimitiveType<'source>>() {
        format_type(&ty.into(), doc)
    } else if let Some(ty) = ty.cast_node::<ArrayType<'source>>() {
        format_type(&ty.into(), doc)
    } else if let Some(ty) = ty.cast_node::<BogusType<'source>>() {
        format_malformed(&ty, doc)
    } else {
        doc.block_on_invariant("invalid type pattern type role");
        Doc::nil()
    }
}

fn format_record_pattern_type<'source>(
    ty: RecordPatternType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(ty) = ty.cast_node::<ClassType<'source>>() {
        format_type(&ty.into(), doc)
    } else if let Some(ty) = ty.cast_node::<BogusType<'source>>() {
        format_malformed(&ty, doc)
    } else {
        doc.block_on_invariant("invalid record pattern type role");
        Doc::nil()
    }
}

fn format_record_pattern_components<'source>(
    pattern: &RecordPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(pattern.open_paren(), doc);
    let close = resolve_required_delimiter(pattern.close_paren(), doc);
    let items = match resolve_required_field(pattern.components(), doc) {
        JavaFormatField::Present(components) => {
            syntax_comma_list_items(doc, components.parts(), |component, doc| {
                format_component_pattern(&component, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![crate::helpers::lists::CommaListItem {
            doc: recovery,
            comma: None,
        }],
    };
    delimited_comma_list(doc, open, close, items)
}

fn format_component_pattern<'source>(
    pattern: &ComponentPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(pattern.pattern(), doc, |value, doc| {
        format_pattern(&value, doc)
    })
}

fn format_match_all_pattern<'source>(
    pattern: &MatchAllPattern<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(pattern.underscore(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    })
}
