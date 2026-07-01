use jolt_fmt_ir::{Doc, concat, force_group, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType, JavaComment,
    JavaSyntaxToken, NameSyntax, PrimitiveType, Type, TypeArgument, TypeArgumentList,
    TypeArgumentListEntry, TypeBoundList, TypeParameter, TypeParameterList, TypeParameterListEntry,
    UnionType, VoidType, WildcardBound, WildcardType,
};

use crate::helpers::comments::{
    comment_forces_line, format_comment, format_leading_comments, format_token_with_comments,
    format_trailing_comments, format_trailing_comments_before_line_break,
};
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
        let entries = parameters.entries().collect::<Vec<_>>();
        if entries.is_empty() {
            return format_empty_type_parameter_list(&parameters);
        }

        group(concat([
            format_type_parameter_list_open(&parameters),
            indent(concat([
                format_open_type_parameter_list_spacing(&parameters),
                format_type_parameter_list_entries(entries),
            ])),
            format_type_parameter_list_close_with_spacing(&parameters),
        ]))
    })
}

pub(crate) fn format_type_argument_list(arguments: &TypeArgumentList) -> Doc {
    let entries = arguments.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return format_empty_type_argument_list(arguments);
    }

    group(concat([
        format_type_argument_list_open(arguments),
        indent(concat([
            format_open_type_argument_list_spacing(arguments),
            format_type_argument_list_entries(entries),
        ])),
        format_type_argument_list_close_with_spacing(arguments),
    ]))
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
    group(jolt_fmt_ir::join(
        text(" & "),
        ty.types().map(|ty| format_type(&ty)),
    ))
}

fn format_union_type(ty: &UnionType) -> Doc {
    group(jolt_fmt_ir::join(
        text(" | "),
        ty.types().map(|ty| format_type(&ty)),
    ))
}

fn format_empty_type_parameter_list(parameters: &TypeParameterList) -> Doc {
    if !type_parameter_list_has_dangling_comments(parameters) {
        return concat([
            format_type_parameter_list_open(parameters),
            format_type_parameter_list_close_delimiter(parameters),
        ]);
    }

    force_group(concat([
        format_type_parameter_list_open(parameters),
        indent(concat([
            hard_line(),
            format_type_parameter_list_dangling_comments(parameters),
        ])),
        hard_line(),
        format_type_parameter_list_close_delimiter_without_leading(parameters),
    ]))
}

fn type_parameter_list_has_dangling_comments(parameters: &TypeParameterList) -> bool {
    parameters
        .open_angle()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || parameters
            .close_angle()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_type_parameter_list_open(parameters: &TypeParameterList) -> Doc {
    parameters.open_angle().map_or_else(
        || text("<"),
        |open| concat([format_leading_comments(&open), text("<")]),
    )
}

fn format_open_type_parameter_list_spacing(parameters: &TypeParameterList) -> Doc {
    let Some(open) = parameters.open_angle() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            soft_line()
        },
    ])
}

fn format_type_parameter_list_entries(entries: Vec<TypeParameterListEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_type_parameter(&entry.parameter));
        if let Some(comma) = entry.comma {
            docs.push(format_angle_list_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_type_parameter_list_close_with_spacing(parameters: &TypeParameterList) -> Doc {
    let close_has_leading_comments = parameters
        .close_angle()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_type_parameter_list_close_delimiter(parameters),
    ])
}

fn format_type_parameter_list_close_delimiter(parameters: &TypeParameterList) -> Doc {
    let close = parameters.close_angle();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text(">"),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(&close)
                } else {
                    jolt_fmt_ir::nil()
                },
                text(">"),
                format_trailing_comments(&close),
            ])
        },
    )
}

fn format_type_parameter_list_close_delimiter_without_leading(
    parameters: &TypeParameterList,
) -> Doc {
    parameters.close_angle().map_or_else(
        || text(">"),
        |close| concat([text(">"), format_trailing_comments(&close)]),
    )
}

fn format_type_parameter_list_dangling_comments(parameters: &TypeParameterList) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = parameters.open_angle() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = parameters.close_angle() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

fn format_empty_type_argument_list(arguments: &TypeArgumentList) -> Doc {
    if !type_argument_list_has_dangling_comments(arguments) {
        return concat([
            format_type_argument_list_open(arguments),
            format_type_argument_list_close_delimiter(arguments),
        ]);
    }

    force_group(concat([
        format_type_argument_list_open(arguments),
        indent(concat([
            hard_line(),
            format_type_argument_list_dangling_comments(arguments),
        ])),
        hard_line(),
        format_type_argument_list_close_delimiter_without_leading(arguments),
    ]))
}

fn type_argument_list_has_dangling_comments(arguments: &TypeArgumentList) -> bool {
    arguments
        .open_angle()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || arguments
            .close_angle()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_type_argument_list_open(arguments: &TypeArgumentList) -> Doc {
    arguments.open_angle().map_or_else(
        || text("<"),
        |open| concat([format_leading_comments(&open), text("<")]),
    )
}

fn format_open_type_argument_list_spacing(arguments: &TypeArgumentList) -> Doc {
    let Some(open) = arguments.open_angle() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            soft_line()
        },
    ])
}

fn format_type_argument_list_entries(entries: Vec<TypeArgumentListEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_type_argument(&entry.argument));
        if let Some(comma) = entry.comma {
            docs.push(format_angle_list_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_type_argument_list_close_with_spacing(arguments: &TypeArgumentList) -> Doc {
    let close_has_leading_comments = arguments
        .close_angle()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_type_argument_list_close_delimiter(arguments),
    ])
}

fn format_type_argument_list_close_delimiter(arguments: &TypeArgumentList) -> Doc {
    let close = arguments.close_angle();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text(">"),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(&close)
                } else {
                    jolt_fmt_ir::nil()
                },
                text(">"),
                format_trailing_comments(&close),
            ])
        },
    )
}

fn format_type_argument_list_close_delimiter_without_leading(arguments: &TypeArgumentList) -> Doc {
    arguments.close_angle().map_or_else(
        || text(">"),
        |close| concat([text(">"), format_trailing_comments(&close)]),
    )
}

fn format_type_argument_list_dangling_comments(arguments: &TypeArgumentList) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = arguments.open_angle() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = arguments.close_angle() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

fn format_angle_list_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn push_dangling_comments(docs: &mut Vec<Doc>, comments: Vec<JavaComment>) {
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
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
    group(jolt_fmt_ir::join(
        text(" & "),
        bounds.bounds().map(|bound| format_type(&bound)),
    ))
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

fn format_inline_annotations(annotations: Vec<Annotation>) -> Doc {
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
