use jolt_fmt_ir::{Doc, concat, group, text};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType, NameSyntax,
    PrimitiveType, Type, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter,
    TypeParameterList, UnionType, VoidType, WildcardBound, WildcardType,
};

use crate::helpers::comments::{format_leading_comments, format_trailing_comments};
use crate::helpers::lists::angle_bracket_list;
use crate::rules::annotations::format_annotation;

pub(crate) fn format_type(ty: &Type) -> Doc {
    match ty {
        Type::PrimitiveType(ty) => format_primitive_type(ty),
        Type::VoidType(ty) => format_void_type(ty),
        Type::ClassType(ty) => format_class_type(ty),
        Type::ArrayType(ty) => concat([
            ty.element_type()
                .map_or_else(jolt_fmt_ir::nil, |element| format_type(&element)),
            ty.dimensions().map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        ]),
        Type::IntersectionType(ty) => format_intersection_type(ty),
        Type::UnionType(ty) => format_union_type(ty),
        Type::WildcardType(ty) => format_wildcard_type(ty),
    }
}

pub(crate) fn format_type_parameter_list(parameters: Option<TypeParameterList>) -> Doc {
    parameters.map_or_else(jolt_fmt_ir::nil, |parameters| {
        angle_bracket_list(
            parameters
                .parameters()
                .map(|parameter| format_type_parameter(&parameter))
                .collect(),
        )
    })
}

pub(crate) fn format_type_argument_list(arguments: &TypeArgumentList) -> Doc {
    angle_bracket_list(
        arguments
            .arguments()
            .map(|argument| format_type_argument(&argument))
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

fn format_primitive_type(ty: &PrimitiveType) -> Doc {
    concat([
        format_inline_annotations(ty.annotations().collect()),
        ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
            concat([
                format_leading_comments(&keyword),
                text(keyword.text().to_owned()),
                format_trailing_comments(&keyword),
            ])
        }),
    ])
}

pub(crate) fn format_void_type(ty: &VoidType) -> Doc {
    ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
        concat([
            format_leading_comments(&keyword),
            text(keyword.text().to_owned()),
            format_trailing_comments(&keyword),
        ])
    })
}

fn format_class_type(ty: &ClassType) -> Doc {
    group(jolt_fmt_ir::join(
        text("."),
        ty.segments().map(format_class_type_segment),
    ))
}

fn format_class_type_segment(segment: jolt_java_syntax::ClassTypeSegment) -> Doc {
    concat([
        format_inline_annotations(segment.annotations),
        format_type_name(&segment.name),
        segment
            .type_arguments
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
            }),
    ])
}

fn format_type_name(name: &NameSyntax) -> Doc {
    jolt_fmt_ir::join(
        text("."),
        name.segments_with_annotations().map(|segment| {
            concat([
                format_inline_annotations(segment.annotations),
                text(segment.identifier.text().to_owned()),
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
