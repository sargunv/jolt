use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, join, line, space};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType,
    IntersectionTypeEntry, JavaSyntaxToken, NameSyntax, PrimitiveType, RecoveredSeparatedListEntry,
    Type, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter, TypeParameterList,
    UnionType, UnionTypeEntry, VoidType, WildcardType,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line, format_token,
    format_token_after_relocated_leading_comments, format_token_sequence,
    format_token_with_comments, format_token_with_inline_leading_comments,
};
use crate::helpers::lists::{CommaListItem, angle_bracket_list, recovered_comma_list_items};
use crate::rules::annotations::format_annotation;

pub(crate) fn format_type<'source>(
    ty: &Type<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_with_leading_comments(ty, LeadingComments::Preserve, formatter)
}

pub(crate) fn format_type_without_leading_comments<'source>(
    ty: &Type<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_with_leading_comments(ty, LeadingComments::SuppressFirstToken, formatter)
}

fn format_type_with_leading_comments<'source>(
    ty: &Type<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(crate) fn format_type_parameter_list<'source>(
    parameters: Option<TypeParameterList<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    parameters.map_or_else(jolt_fmt_ir::nil, |parameters| {
        let open = parameters.open_angle();
        let close = parameters.close_angle();
        let items = type_parameter_list_items(&parameters, formatter);
        angle_bracket_list(open.as_ref(), close.as_ref(), items)
    })
}

pub(crate) fn format_type_argument_list<'source>(
    arguments: &TypeArgumentList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = arguments.open_angle();
    let close = arguments.close_angle();
    let items = type_argument_list_items(arguments, formatter);
    angle_bracket_list(open.as_ref(), close.as_ref(), items)
}

fn type_parameter_list_items<'source, 'fmt>(
    parameters: &'fmt TypeParameterList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(parameters.entries_with_recovered(), |entry| CommaListItem {
        doc: format_type_parameter(&entry.parameter, formatter),
        comma: entry.comma,
    })
}

fn type_argument_list_items<'source, 'fmt>(
    arguments: &'fmt TypeArgumentList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(arguments.entries_with_recovered(), |entry| CommaListItem {
        doc: format_type_argument(&entry.argument, formatter),
        comma: entry.comma,
    })
}

pub(crate) fn format_array_dimensions<'source>(
    dimensions: &ArrayDimensions<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat(
        dimensions
            .dimensions()
            .map(|dimension| format_array_dimension(&dimension, formatter)),
    )
}

fn format_primitive_type<'source>(
    ty: &PrimitiveType<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_inline_annotations(ty.annotations(), formatter),
        ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
            format_type_head_token(&keyword, leading_comments)
        }),
    ])
}

pub(crate) fn format_void_type<'source>(ty: &VoidType<'source>) -> Doc<'source> {
    format_void_type_with_leading_comments(ty, LeadingComments::Preserve)
}

fn format_void_type_with_leading_comments<'source>(
    ty: &VoidType<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    ty.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
        format_type_head_token(&keyword, leading_comments)
    })
}

fn format_type_head_token<'source>(
    token: &JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    match leading_comments {
        LeadingComments::Preserve => {
            format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }
        LeadingComments::SuppressFirstToken => {
            format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
        }
    }
}

fn format_class_type<'source>(
    ty: &ClassType<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for (index, segment) in ty.segments().enumerate() {
        if index > 0 {
            docs.push(
                segment
                    .dot_before
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
            );
        }
        docs.push(format_class_type_segment(
            segment,
            if index == 0 {
                leading_comments
            } else {
                LeadingComments::Preserve
            },
            formatter,
        ));
    }
    group(concat(docs))
}

fn format_class_type_segment<'source>(
    segment: jolt_java_syntax::ClassTypeSegment<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_type_name<'source>(
    name: &NameSyntax<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for (index, segment) in name.segments_with_annotations().enumerate() {
        if index > 0 {
            docs.push(
                segment
                    .dot_before
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
            );
        }
        docs.push(format_inline_annotations(segment.annotations, formatter));
        docs.push(if index == 0 {
            format_type_head_token(&segment.identifier, leading_comments)
        } else {
            format_token_with_comments(&segment.identifier)
        });
    }
    concat(docs)
}

fn format_intersection_type<'source>(
    ty: &IntersectionType<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_intersection_entries(ty, formatter)
}

fn format_union_type<'source>(
    ty: &UnionType<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_union_entries(ty, formatter)
}

fn format_type_parameter<'source>(
    parameter: &TypeParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_inline_annotations(parameter.annotations(), formatter),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        parameter.bounds().map_or_else(jolt_fmt_ir::nil, |bounds| {
            concat([
                space(),
                bounds
                    .extends_token()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                space(),
                format_type_bounds(&bounds, formatter),
            ])
        }),
    ])
}

fn format_type_bounds<'source>(
    bounds: &TypeBoundList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_operator_entries_doc(
        type_operator_parts(bounds.entries_with_recovered()),
        formatter,
    )
}

fn format_intersection_entries<'source>(
    ty: &IntersectionType<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_operator_entries(type_operator_parts(ty.entries_with_recovered()), formatter)
}

fn format_union_entries<'source>(
    ty: &UnionType<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_operator_entries(type_operator_parts(ty.entries_with_recovered()), formatter)
}

fn format_type_operator_entries<'source>(
    entries: impl IntoIterator<Item = TypeOperatorPart<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(format_type_operator_entries_doc(entries, formatter))
}

fn format_type_operator_entries_doc<'source>(
    entries: impl IntoIterator<Item = TypeOperatorPart<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let Some(first) = entries.next() else {
        return jolt_fmt_ir::nil();
    };

    let (first, mut previous_separator) = format_type_operator_first_part(first, formatter);
    let mut rest = Vec::new();
    for part in entries {
        match part {
            TypeOperatorPart::Type { ty, separator } => {
                rest.push(format_type_operator_continuation(
                    previous_separator.as_ref(),
                    format_type(&ty, formatter),
                ));
                previous_separator = separator;
            }
            TypeOperatorPart::Recovered(doc) => {
                rest.push(format_type_operator_continuation(
                    previous_separator.as_ref(),
                    doc,
                ));
                previous_separator = None;
            }
        }
    }
    if previous_separator.is_some() {
        rest.push(format_type_operator_continuation(
            previous_separator.as_ref(),
            jolt_fmt_ir::nil(),
        ));
    }

    concat([first, indent(concat(rest))])
}

enum TypeOperatorPart<'source> {
    Type {
        ty: Type<'source>,
        separator: Option<JavaSyntaxToken<'source>>,
    },
    Recovered(Doc<'source>),
}

trait TypeOperatorEntry<'source> {
    fn ty(&self) -> Type<'source>;
    fn separator(&self) -> Option<JavaSyntaxToken<'source>>;
}

impl<'source> TypeOperatorEntry<'source> for IntersectionTypeEntry<'source> {
    fn ty(&self) -> Type<'source> {
        self.ty
    }

    fn separator(&self) -> Option<JavaSyntaxToken<'source>> {
        self.separator
    }
}

impl<'source> TypeOperatorEntry<'source> for UnionTypeEntry<'source> {
    fn ty(&self) -> Type<'source> {
        self.ty
    }

    fn separator(&self) -> Option<JavaSyntaxToken<'source>> {
        self.separator
    }
}

fn type_operator_parts<'source, Entry>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
) -> impl Iterator<Item = TypeOperatorPart<'source>>
where
    Entry: TypeOperatorEntry<'source>,
{
    entries.into_iter().map(|entry| match entry {
        RecoveredSeparatedListEntry::Entry(entry) => TypeOperatorPart::Type {
            ty: entry.ty(),
            separator: entry.separator(),
        },
        RecoveredSeparatedListEntry::Token(token) => TypeOperatorPart::Recovered(format_token(
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )),
        RecoveredSeparatedListEntry::Error(error) => TypeOperatorPart::Recovered(
            format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
        ),
        RecoveredSeparatedListEntry::Node(node) => TypeOperatorPart::Recovered(
            format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
        ),
    })
}

fn format_type_operator_first_part<'source>(
    part: TypeOperatorPart<'source>,
    formatter: &JavaFormatter<'_>,
) -> (Doc<'source>, Option<JavaSyntaxToken<'source>>) {
    match part {
        TypeOperatorPart::Type { ty, separator } => (format_type(&ty, formatter), separator),
        TypeOperatorPart::Recovered(doc) => (doc, None),
    }
}

fn type_operator_separator_forces_line(separator: Option<&JavaSyntaxToken<'_>>) -> bool {
    separator.is_some_and(|separator| {
        separator
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}

fn format_type_operator_continuation<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
    operand: Doc<'source>,
) -> Doc<'source> {
    let separator_doc = format_type_operator_separator_before_operand(separator);

    if type_operator_separator_forces_line(separator) {
        concat([separator_doc, indent(concat([hard_line(), operand]))])
    } else {
        concat([separator_doc, operand])
    }
}

fn format_type_operator_separator_before_operand<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(separator) = separator {
        format_type_operator_separator(separator)
    } else {
        line()
    }
}

fn format_type_operator_separator<'source>(separator: &JavaSyntaxToken<'source>) -> Doc<'source> {
    concat([line(), {
        let forces_line = separator
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment));
        concat([
            format_token(
                separator,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if forces_line {
                jolt_fmt_ir::nil()
            } else {
                space()
            },
        ])
    }])
}

fn format_type_argument<'source>(
    argument: &TypeArgument<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let has_annotations = argument.annotations().next().is_some();
    if argument.ty().is_none() && !has_annotations {
        return format_token_sequence(argument.token_iter(), LeadingTrivia::Preserve);
    }

    concat([
        format_inline_annotations(argument.annotations(), formatter),
        argument
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
    ])
}

fn format_wildcard_type<'source>(
    ty: &WildcardType<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        ty.question_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        ty.bound_keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
            concat([
                space(),
                format_token_with_comments(&keyword),
                ty.bound().map_or_else(jolt_fmt_ir::nil, |bound| {
                    concat([space(), format_type(&bound, formatter)])
                }),
            ])
        }),
    ])
}

fn format_array_dimension<'source>(
    dimension: &ArrayDimension<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut annotations = dimension.annotations().peekable();
    if annotations.peek().is_none() {
        return concat([
            dimension
                .open_bracket()
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
            dimension
                .close_bracket()
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, format_array_dimension_close_bracket),
        ]);
    }

    concat([
        space(),
        format_inline_annotations(annotations, formatter),
        dimension
            .open_bracket()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        dimension
            .close_bracket()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_array_dimension_close_bracket),
    ])
}

fn format_array_dimension_close_bracket<'source>(close: &JavaSyntaxToken<'source>) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        close,
        InlineLeadingTrivia::AfterPreviousToken,
        TrailingTrivia::Preserve,
    )
}

pub(crate) fn format_inline_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut annotations = annotations.into_iter().peekable();
    if annotations.peek().is_none() {
        return jolt_fmt_ir::nil();
    }

    concat([
        join(
            &space(),
            annotations.map(|annotation| format_annotation(&annotation, formatter)),
        ),
        space(),
    ])
}
