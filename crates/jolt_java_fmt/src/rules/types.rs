use jolt_fmt_ir::{Doc, concat, group, hard_line, line, text};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType, JavaSyntaxToken,
    NameSyntax, PrimitiveType, Type, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter,
    TypeParameterList, UnionType, UnionTypeEntry, VoidType, WildcardBound, WildcardType,
};

use crate::helpers::comments::{
    comment_forces_line, format_leading_comments, format_token_with_comments,
    format_trailing_comments, format_trailing_comments_before_line_break,
};
use crate::helpers::lists::{CommaListItem, angle_bracket_list};
use crate::rules::annotations::format_annotation;

pub(crate) fn format_type(ty: &Type) -> Doc {
    format_type_with_leading_comments(ty, LeadingComments::Preserve)
}

pub(crate) fn format_type_without_leading_comments(ty: &Type) -> Doc {
    format_type_with_leading_comments(ty, LeadingComments::SuppressFirstToken)
}

fn format_type_with_leading_comments(ty: &Type, leading_comments: LeadingComments) -> Doc {
    match ty {
        Type::PrimitiveType(ty) => format_primitive_type(ty, leading_comments),
        Type::VoidType(ty) => format_void_type_with_leading_comments(ty, leading_comments),
        Type::ClassType(ty) => format_class_type(ty, leading_comments),
        Type::ArrayType(ty) => concat([
            ty.element_type().map_or_else(jolt_fmt_ir::nil, |element| {
                format_type_with_leading_comments(&element, leading_comments)
            }),
            ty.dimensions().map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        ]),
        Type::IntersectionType(ty) => format_intersection_type(ty),
        Type::UnionType(ty) => format_union_type(ty),
        Type::WildcardType(ty) => format_wildcard_type(ty),
    }
}

#[derive(Clone, Copy)]
enum LeadingComments {
    Preserve,
    SuppressFirstToken,
}

pub(crate) fn format_type_parameter_list(parameters: Option<TypeParameterList>) -> Doc {
    parameters.map_or_else(jolt_fmt_ir::nil, |parameters| {
        let open = parameters.open_angle();
        let close = parameters.close_angle();
        angle_bracket_list(
            open.as_ref(),
            close.as_ref(),
            parameters
                .entries()
                .map(|entry| CommaListItem {
                    doc: format_type_parameter(&entry.parameter),
                    comma: entry.comma,
                })
                .collect(),
        )
    })
}

pub(crate) fn format_type_argument_list(arguments: &TypeArgumentList) -> Doc {
    let open = arguments.open_angle();
    let close = arguments.close_angle();
    angle_bracket_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_type_argument(&entry.argument),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(crate) fn format_array_dimensions(dimensions: &ArrayDimensions) -> Doc {
    concat(
        dimensions
            .dimensions()
            .map(|dimension| format_array_dimension(&dimension)),
    )
}

fn format_primitive_type(ty: &PrimitiveType, leading_comments: LeadingComments) -> Doc {
    concat([
        format_inline_annotations(ty.annotations().collect()),
        ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
            concat([
                match leading_comments {
                    LeadingComments::Preserve => format_leading_comments(&keyword),
                    LeadingComments::SuppressFirstToken => jolt_fmt_ir::nil(),
                },
                text(keyword.text().to_owned()),
                format_trailing_comments(&keyword),
            ])
        }),
    ])
}

pub(crate) fn format_void_type(ty: &VoidType) -> Doc {
    format_void_type_with_leading_comments(ty, LeadingComments::Preserve)
}

fn format_void_type_with_leading_comments(ty: &VoidType, leading_comments: LeadingComments) -> Doc {
    ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
        concat([
            match leading_comments {
                LeadingComments::Preserve => format_leading_comments(&keyword),
                LeadingComments::SuppressFirstToken => jolt_fmt_ir::nil(),
            },
            text(keyword.text().to_owned()),
            format_trailing_comments(&keyword),
        ])
    })
}

fn format_class_type(ty: &ClassType, leading_comments: LeadingComments) -> Doc {
    group(jolt_fmt_ir::join(
        text("."),
        ty.segments().enumerate().map(|(index, segment)| {
            format_class_type_segment(
                segment,
                if index == 0 {
                    leading_comments
                } else {
                    LeadingComments::Preserve
                },
            )
        }),
    ))
}

fn format_class_type_segment(
    segment: jolt_java_syntax::ClassTypeSegment,
    leading_comments: LeadingComments,
) -> Doc {
    concat([
        format_inline_annotations(segment.annotations),
        format_type_name(&segment.name, leading_comments),
        segment
            .type_arguments
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
            }),
    ])
}

fn format_type_name(name: &NameSyntax, leading_comments: LeadingComments) -> Doc {
    jolt_fmt_ir::join(
        text("."),
        name.segments_with_annotations()
            .enumerate()
            .map(|(index, segment)| {
                concat([
                    format_inline_annotations(segment.annotations),
                    if index == 0 {
                        match leading_comments {
                            LeadingComments::Preserve => {
                                format_token_with_comments(&segment.identifier)
                            }
                            LeadingComments::SuppressFirstToken => concat([
                                text(segment.identifier.text().to_owned()),
                                format_trailing_comments(&segment.identifier),
                            ]),
                        }
                    } else {
                        format_token_with_comments(&segment.identifier)
                    },
                ])
            }),
    )
}

fn format_intersection_type(ty: &IntersectionType) -> Doc {
    format_intersection_entries(ty.entries().collect())
}

fn format_union_type(ty: &UnionType) -> Doc {
    format_union_entries(ty.entries().collect())
}

fn format_type_parameter(parameter: &TypeParameter) -> Doc {
    concat([
        format_inline_annotations(parameter.annotations().collect()),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        parameter.bounds().map_or_else(jolt_fmt_ir::nil, |bounds| {
            concat([text(" extends "), format_type_bounds(&bounds)])
        }),
    ])
}

fn format_type_bounds(bounds: &TypeBoundList) -> Doc {
    format_intersection_entries(bounds.entries().collect())
}

fn format_intersection_entries(entries: Vec<jolt_java_syntax::IntersectionTypeEntry>) -> Doc {
    format_type_operator_entries(
        entries
            .into_iter()
            .map(|entry| (entry.ty, entry.separator))
            .collect(),
        "&",
    )
}

fn format_union_entries(entries: Vec<UnionTypeEntry>) -> Doc {
    format_type_operator_entries(
        entries
            .into_iter()
            .map(|entry| (entry.ty, entry.separator))
            .collect(),
        "|",
    )
}

fn format_type_operator_entries(
    entries: Vec<(Type, Option<JavaSyntaxToken>)>,
    fallback_operator: &'static str,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, (ty, separator)) in entries.into_iter().enumerate() {
        docs.push(format_type(&ty));
        if let Some(separator) = separator {
            docs.push(format_type_operator_separator(
                Some(&separator),
                fallback_operator,
            ));
        } else if index + 1 < entries_len {
            docs.push(format_type_operator_separator(None, fallback_operator));
        }
    }

    group(concat(docs))
}

fn format_type_operator_separator(
    separator: Option<&JavaSyntaxToken>,
    fallback_operator: &'static str,
) -> Doc {
    concat([
        line(),
        separator.map_or_else(
            || concat([text(fallback_operator), text(" ")]),
            |separator| {
                concat([
                    format_leading_comments(separator),
                    text(separator.text().to_owned()),
                    format_trailing_comments_before_line_break(separator),
                    if separator
                        .trailing_comments()
                        .iter()
                        .any(comment_forces_line)
                    {
                        hard_line()
                    } else {
                        text(" ")
                    },
                ])
            },
        ),
    ])
}

fn format_type_argument(argument: &TypeArgument) -> Doc {
    concat([
        format_inline_annotations(argument.annotations().collect()),
        argument
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
    ])
}

fn format_wildcard_type(ty: &WildcardType) -> Doc {
    concat([
        text("?"),
        ty.bound_clause().map_or_else(jolt_fmt_ir::nil, |bound| {
            let (keyword, bound) = match bound {
                WildcardBound::Extends(bound) => ("extends", bound),
                WildcardBound::Super(bound) => ("super", bound),
            };
            concat([text(" "), text(keyword), text(" "), format_type(&bound)])
        }),
    ])
}

fn format_array_dimension(dimension: &ArrayDimension) -> Doc {
    let annotations = dimension.annotations().collect::<Vec<_>>();
    if annotations.is_empty() {
        return text("[]");
    }

    concat([
        text(" "),
        format_inline_annotations(annotations),
        text("[]"),
    ])
}

pub(crate) fn format_inline_annotations(annotations: Vec<Annotation>) -> Doc {
    if annotations.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        jolt_fmt_ir::join(
            text(" "),
            annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation)),
        ),
        text(" "),
    ])
}
