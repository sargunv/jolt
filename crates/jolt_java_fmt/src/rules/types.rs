use jolt_fmt_ir::{Doc, concat, group, hard_line, line, text};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType, JavaSyntaxToken,
    NameSyntax, PrimitiveType, Type, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter,
    TypeParameterList, UnionType, UnionTypeEntry, VoidType, WildcardBound, WildcardType,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::{
    comment_forces_line, format_leading_comments, format_token_text, format_token_with_comments,
    format_trailing_comments, format_trailing_comments_before_line_break,
};
use crate::helpers::lists::{CommaListItem, angle_bracket_list};
use crate::rules::annotations::format_annotation;

pub(crate) fn format_type(ty: &Type, formatter: &JavaFormatter<'_>) -> Doc {
    format_type_with_leading_comments(ty, LeadingComments::Preserve, formatter)
}

pub(crate) fn format_type_without_leading_comments(
    ty: &Type,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_with_leading_comments(ty, LeadingComments::SuppressFirstToken, formatter)
}

fn format_type_with_leading_comments(
    ty: &Type,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match ty {
        Type::PrimitiveType(ty) => format_primitive_type(ty, leading_comments, formatter),
        Type::VoidType(ty) => format_void_type_with_leading_comments(ty, leading_comments),
        Type::ClassType(ty) => format_class_type(ty, leading_comments, formatter),
        Type::ArrayType(ty) => concat([
            ty.element_type().map_or_else(jolt_fmt_ir::nil, |element| {
                format_type_with_leading_comments(&element, leading_comments, formatter)
            }),
            ty.dimensions().map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        ]),
        Type::IntersectionType(ty) => format_intersection_type(ty, formatter),
        Type::UnionType(ty) => format_union_type(ty, formatter),
        Type::WildcardType(ty) => format_wildcard_type(ty, formatter),
    }
}

#[derive(Clone, Copy)]
enum LeadingComments {
    Preserve,
    SuppressFirstToken,
}

pub(crate) fn format_type_parameter_list(
    parameters: Option<TypeParameterList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    parameters.map_or_else(jolt_fmt_ir::nil, |parameters| {
        let open = parameters.open_angle();
        let close = parameters.close_angle();
        angle_bracket_list(
            open.as_ref(),
            close.as_ref(),
            parameters
                .entries()
                .map(|entry| CommaListItem {
                    doc: format_type_parameter(&entry.parameter, formatter),
                    comma: entry.comma,
                })
                .collect(),
        )
    })
}

pub(crate) fn format_type_argument_list(
    arguments: &TypeArgumentList,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = arguments.open_angle();
    let close = arguments.close_angle();
    angle_bracket_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_type_argument(&entry.argument, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(crate) fn format_array_dimensions(
    dimensions: &ArrayDimensions,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat(
        dimensions
            .dimensions()
            .map(|dimension| format_array_dimension(&dimension, formatter)),
    )
}

fn format_primitive_type(
    ty: &PrimitiveType,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_inline_annotations(ty.annotations().collect(), formatter),
        ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
            concat([
                match leading_comments {
                    LeadingComments::Preserve => format_leading_comments(&keyword),
                    LeadingComments::SuppressFirstToken => jolt_fmt_ir::nil(),
                },
                format_token_text(keyword.text()),
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
            format_token_text(keyword.text()),
            format_trailing_comments(&keyword),
        ])
    })
}

fn format_class_type(
    ty: &ClassType,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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
                formatter,
            )
        }),
    ))
}

fn format_class_type_segment(
    segment: jolt_java_syntax::ClassTypeSegment,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_inline_annotations(segment.annotations, formatter),
        format_type_name(&segment.name, leading_comments, formatter),
        segment
            .type_arguments
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
    ])
}

fn format_type_name(
    name: &NameSyntax,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    jolt_fmt_ir::join(
        text("."),
        name.segments_with_annotations()
            .enumerate()
            .map(|(index, segment)| {
                concat([
                    format_inline_annotations(segment.annotations, formatter),
                    if index == 0 {
                        match leading_comments {
                            LeadingComments::Preserve => {
                                format_token_with_comments(&segment.identifier)
                            }
                            LeadingComments::SuppressFirstToken => concat([
                                format_token_text(segment.identifier.text()),
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

fn format_intersection_type(ty: &IntersectionType, formatter: &JavaFormatter<'_>) -> Doc {
    format_intersection_entries(ty.entries().collect(), formatter)
}

fn format_union_type(ty: &UnionType, formatter: &JavaFormatter<'_>) -> Doc {
    format_union_entries(ty.entries().collect(), formatter)
}

fn format_type_parameter(parameter: &TypeParameter, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        format_inline_annotations(parameter.annotations().collect(), formatter),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
        parameter.bounds().map_or_else(jolt_fmt_ir::nil, |bounds| {
            concat([text(" extends "), format_type_bounds(&bounds, formatter)])
        }),
    ])
}

fn format_type_bounds(bounds: &TypeBoundList, formatter: &JavaFormatter<'_>) -> Doc {
    format_intersection_entries(bounds.entries().collect(), formatter)
}

fn format_intersection_entries(
    entries: Vec<jolt_java_syntax::IntersectionTypeEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_operator_entries(
        entries
            .into_iter()
            .map(|entry| (entry.ty, entry.separator))
            .collect(),
        "&",
        formatter,
    )
}

fn format_union_entries(entries: Vec<UnionTypeEntry>, formatter: &JavaFormatter<'_>) -> Doc {
    format_type_operator_entries(
        entries
            .into_iter()
            .map(|entry| (entry.ty, entry.separator))
            .collect(),
        "|",
        formatter,
    )
}

fn format_type_operator_entries(
    entries: Vec<(Type, Option<JavaSyntaxToken>)>,
    fallback_operator: &'static str,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, (ty, separator)) in entries.into_iter().enumerate() {
        docs.push(format_type(&ty, formatter));
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
                    format_token_text(separator.text()),
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

fn format_type_argument(argument: &TypeArgument, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        format_inline_annotations(argument.annotations().collect(), formatter),
        argument
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
    ])
}

fn format_wildcard_type(ty: &WildcardType, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        text("?"),
        ty.bound_clause().map_or_else(jolt_fmt_ir::nil, |bound| {
            let (keyword, bound) = match bound {
                WildcardBound::Extends(bound) => ("extends", bound),
                WildcardBound::Super(bound) => ("super", bound),
            };
            concat([
                text(" "),
                text(keyword),
                text(" "),
                format_type(&bound, formatter),
            ])
        }),
    ])
}

fn format_array_dimension(dimension: &ArrayDimension, formatter: &JavaFormatter<'_>) -> Doc {
    let annotations = dimension.annotations().collect::<Vec<_>>();
    if annotations.is_empty() {
        return text("[]");
    }

    concat([
        text(" "),
        format_inline_annotations(annotations, formatter),
        text("[]"),
    ])
}

pub(crate) fn format_inline_annotations(
    annotations: Vec<Annotation>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if annotations.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        jolt_fmt_ir::join(
            text(" "),
            annotations
                .into_iter()
                .map(|annotation| format_annotation(&annotation, formatter)),
        ),
        text(" "),
    ])
}
