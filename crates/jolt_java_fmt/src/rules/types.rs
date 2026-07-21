use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, ClassTypeSegmentNode, IntersectionType,
    JavaSyntaxListPart, JavaSyntaxToken, NameSyntax, PrimitiveType, QualifiedNameSegmentNode, Type,
    TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter, TypeParameterList, UnionType,
    VoidType, WildcardType,
};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line, format_token,
    format_token_after_relocated_leading_comments, format_token_with_comments,
    format_token_with_inline_leading_comments,
};
use crate::helpers::lists::{CommaListItem, delimited_comma_list, syntax_comma_list_items};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_malformed, format_optional_field,
    format_required_field, resolve_list_part, resolve_required_delimiter, resolve_required_field,
};
use crate::rules::annotations::format_annotation;

pub(crate) fn format_type<'source>(
    ty: &Type<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_type_with_leading_comments(ty, LeadingComments::Preserve, doc)
}

pub(crate) fn format_type_without_leading_comments<'source>(
    ty: &Type<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_type_with_leading_comments(ty, LeadingComments::SuppressFirstToken, doc)
}

fn format_type_with_leading_comments<'source>(
    ty: &Type<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match ty {
        Type::PrimitiveType(ty) => format_primitive_type(ty, leading_comments, doc),
        Type::VoidType(ty) => format_void_type_with_leading_comments(ty, leading_comments, doc),
        Type::ClassType(ty) => format_class_type(ty, leading_comments, doc),
        Type::ArrayType(ty) => {
            let element = format_required_field(ty.element_type(), doc, |element, doc| {
                let Some(element) = element.cast_family::<Type<'source>>() else {
                    doc.block_on_invariant("array element type was not a type");
                    return Doc::nil();
                };
                format_type_with_leading_comments(&element, leading_comments, doc)
            });
            let dimensions = format_required_field(ty.dimensions(), doc, |dimensions, doc| {
                format_array_dimensions(&dimensions, doc)
            });
            doc_concat!(doc, [element, dimensions])
        }
        Type::IntersectionType(ty) => format_intersection_type(ty, doc),
        Type::UnionType(ty) => format_union_type(ty, doc),
        Type::WildcardType(ty) => format_wildcard_type(ty, doc),
        Type::BogusType(ty) => format_malformed(ty, doc),
    }
}

#[derive(Clone, Copy)]
enum LeadingComments {
    Preserve,
    SuppressFirstToken,
}

pub(crate) fn format_type_parameter_list<'source>(
    parameters: Option<TypeParameterList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match parameters {
        Some(parameters) => {
            let open = resolve_required_delimiter(parameters.open_angle(), doc);
            let close = resolve_required_delimiter(parameters.close_angle(), doc);
            let items = type_parameter_list_items(&parameters, doc);
            delimited_comma_list(doc, open, close, items)
        }
        None => Doc::nil(),
    }
}

pub(crate) fn format_type_argument_list<'source>(
    arguments: &TypeArgumentList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_angle(), doc);
    let close = resolve_required_delimiter(arguments.close_angle(), doc);
    let items = type_argument_list_items(arguments, doc);
    delimited_comma_list(doc, open, close, items)
}

fn type_parameter_list_items<'source, 'fmt>(
    parameters: &'fmt TypeParameterList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    match resolve_required_field(parameters.parameters(), doc) {
        JavaFormatField::Present(parameters) => {
            syntax_comma_list_items(doc, parameters.parts(), |parameter, doc| {
                format_type_parameter(&parameter, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
        }],
    }
}

fn type_argument_list_items<'source, 'fmt>(
    arguments: &'fmt TypeArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    match resolve_required_field(arguments.arguments(), doc) {
        JavaFormatField::Present(arguments) => {
            syntax_comma_list_items(doc, arguments.parts(), |argument, doc| {
                format_type_argument(&argument, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
        }],
    }
}

pub(crate) fn format_array_dimensions<'source>(
    dimensions: &ArrayDimensions<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in dimensions.parts() {
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(dimension) => format_array_dimension(&dimension, docs),
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
        }
    })
}

fn format_primitive_type<'source>(
    ty: &PrimitiveType<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_optional_field(ty.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts(annotations.parts(), doc)
    });
    let keyword = format_required_field(ty.keyword(), doc, |keyword, doc| {
        format_type_head_token(&keyword, leading_comments, doc)
    });
    doc_concat!(doc, [annotations, keyword])
}

pub(crate) fn format_void_type<'source>(
    ty: &VoidType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_void_type_with_leading_comments(ty, LeadingComments::Preserve, doc)
}

fn format_void_type_with_leading_comments<'source>(
    ty: &VoidType<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(ty.void_keyword(), doc, |keyword, doc| {
        format_type_head_token(&keyword, leading_comments, doc)
    })
}

fn format_type_head_token<'source>(
    token: &JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match leading_comments {
        LeadingComments::Preserve => format_token(
            doc,
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        LeadingComments::SuppressFirstToken => {
            format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
        }
    }
}

fn format_class_type<'source>(
    ty: &ClassType<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let contents = format_required_field(ty.segments(), doc, |segments, doc| {
        let mut first = true;
        doc.concat_list(|docs| {
            for part in segments.parts() {
                let part = match resolve_list_part(part, docs) {
                    JavaFormatListPart::Item(segment) => {
                        let comments = if first {
                            leading_comments
                        } else {
                            LeadingComments::Preserve
                        };
                        first = false;
                        format_class_type_segment(&segment, comments, docs)
                    }
                    JavaFormatListPart::Separator(dot) => format_token_with_comments(docs, &dot),
                    JavaFormatListPart::Malformed(recovery) => recovery,
                };
                docs.push(part);
            }
        })
    });
    doc_group!(doc, contents)
}

fn format_class_type_segment<'source>(
    segment: &ClassTypeSegmentNode<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(segment.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts(annotations.parts(), doc)
    });
    let name = format_required_field(segment.name(), doc, |name, doc| {
        format_type_name(&name, leading_comments, doc)
    });
    let type_arguments = format_optional_field(segment.type_arguments(), doc, |arguments, doc| {
        format_type_argument_list(&arguments, doc)
    });
    doc_concat!(doc, [annotations, name, type_arguments])
}

fn format_type_name<'source>(
    name: &NameSyntax<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match name {
        NameSyntax::Name(name) => {
            format_required_field(name.identifier(), doc, |identifier, doc| {
                format_type_head_token(&identifier, leading_comments, doc)
            })
        }
        NameSyntax::QualifiedName(name) => {
            let first = format_required_field(name.first_segment(), doc, |segment, doc| {
                format_qualified_name_segment(&segment, leading_comments, doc)
            });
            let first_dot = format_required_field(name.first_dot(), doc, |dot, doc| {
                format_token_with_comments(doc, &dot)
            });
            let remaining =
                format_required_field(name.remaining_segments(), doc, |segments, doc| {
                    doc.concat_list(|docs| {
                        for part in segments.parts() {
                            let part = match resolve_list_part(part, docs) {
                                JavaFormatListPart::Item(segment) => format_qualified_name_segment(
                                    &segment,
                                    LeadingComments::Preserve,
                                    docs,
                                ),
                                JavaFormatListPart::Separator(dot) => {
                                    format_token_with_comments(docs, &dot)
                                }
                                JavaFormatListPart::Malformed(recovery) => recovery,
                            };
                            docs.push(part);
                        }
                    })
                });
            doc_concat!(doc, [first, first_dot, remaining])
        }
        NameSyntax::BogusName(name) => format_malformed(name, doc),
    }
}

fn format_qualified_name_segment<'source>(
    segment: &QualifiedNameSegmentNode<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(segment.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts(annotations.parts(), doc)
    });
    let identifier = format_required_field(segment.identifier(), doc, |identifier, doc| {
        format_type_head_token(&identifier, leading_comments, doc)
    });
    doc_concat!(doc, [annotations, identifier])
}

fn format_intersection_type<'source>(
    ty: &IntersectionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let entries = format_intersection_entries(ty, doc);
    doc_group!(doc, entries)
}

fn format_union_type<'source>(
    ty: &UnionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_union_entries(ty, doc)
}

fn format_type_parameter<'source>(
    parameter: &TypeParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(parameter.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts(annotations.parts(), doc)
    });
    let name = format_required_field(parameter.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let bounds = format_optional_field(parameter.bounds(), doc, |bounds, doc| {
        let space_before_extends = doc.space();
        let extends = format_required_field(bounds.extends_keyword(), doc, |token, doc| {
            format_token_with_comments(doc, &token)
        });
        let space_after_extends = doc.space();
        let bounds = format_type_bounds(&bounds, doc);
        doc_concat!(
            doc,
            [space_before_extends, extends, space_after_extends, bounds]
        )
    });
    doc_concat!(doc, [annotations, name, bounds])
}

fn format_type_bounds<'source>(
    bounds: &TypeBoundList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(bounds.bounds(), doc, |bound, doc| {
        let Some(bound) = bound.cast_family::<Type<'source>>() else {
            doc.block_on_invariant("type bound was not a type");
            return Doc::nil();
        };
        match bound {
            Type::IntersectionType(intersection) => format_intersection_entries(&intersection, doc),
            _ => format_type(&bound, doc),
        }
    })
}

fn format_intersection_entries<'source>(
    ty: &IntersectionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let first = format_required_field(ty.first_type(), doc, |ty, doc| format_type(&ty, doc));
    let (first_separator, indent_after_separator) =
        format_required_type_operator(ty.first_amp(), doc);
    let remaining = format_required_field(ty.remaining_types(), doc, |types, doc| {
        format_type_operator_list(types.parts(), doc)
    });
    let remaining = if indent_after_separator {
        doc_indent!(doc, doc_concat!(doc, [doc.hard_line(), remaining]))
    } else {
        remaining
    };
    doc_concat!(
        doc,
        [
            first,
            doc_indent!(doc, doc_concat!(doc, [first_separator, remaining]))
        ]
    )
}

fn format_union_entries<'source>(
    ty: &UnionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let first = format_required_field(ty.first_type(), doc, |ty, doc| format_type(&ty, doc));
    let (first_separator, indent_after_separator) =
        format_required_type_operator(ty.first_bar(), doc);
    let remaining = format_required_field(ty.remaining_types(), doc, |types, doc| {
        format_type_operator_list(types.parts(), doc)
    });
    let remaining = if indent_after_separator {
        doc_indent!(doc, doc_concat!(doc, [doc.hard_line(), remaining]))
    } else {
        remaining
    };
    doc_concat!(doc, [first, first_separator, remaining])
}

fn format_type_operator_list<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, Type<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut indent_next = false;
    doc.concat_list(|docs| {
        for part in parts {
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(ty) => {
                    let ty = format_type(&ty, docs);
                    let ty = if indent_next {
                        doc_indent!(docs, doc_concat!(docs, [docs.hard_line(), ty]))
                    } else {
                        ty
                    };
                    indent_next = false;
                    ty
                }
                JavaFormatListPart::Separator(separator) => {
                    indent_next = type_operator_forces_line(&separator);
                    format_type_operator_separator(&separator, docs)
                }
                JavaFormatListPart::Malformed(recovery) => {
                    indent_next = false;
                    recovery
                }
            };
            docs.push(part);
        }
    })
}

fn format_required_type_operator<'source>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(separator) => {
            let forces_line = type_operator_forces_line(&separator);
            (format_type_operator_separator(&separator, doc), forces_line)
        }
        JavaFormatField::Malformed(recovery) => (recovery, false),
    }
}

fn type_operator_forces_line(separator: &JavaSyntaxToken<'_>) -> bool {
    separator
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment))
}

fn format_type_operator_separator<'source>(
    separator: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let line = doc.line();
    let forces_line = type_operator_forces_line(separator);
    let separator = format_token(
        doc,
        separator,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeLineBreak,
    );
    let space = if forces_line { Doc::nil() } else { doc.space() };
    let separator = doc_concat!(doc, [separator, space]);
    doc_concat!(doc, [line, separator])
}

fn format_type_argument<'source>(
    argument: &TypeArgument<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(argument.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts(annotations.parts(), doc)
    });
    let ty = format_required_field(argument.r#type(), doc, |ty, doc| format_type(&ty, doc));

    doc_concat!(doc, [annotations, ty])
}

fn format_wildcard_type<'source>(
    ty: &WildcardType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let question = format_required_field(ty.question(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let bound = format_optional_field(ty.bound_keyword(), doc, |keyword, doc| {
        let before_keyword = doc.space();
        let keyword = format_token_with_comments(doc, &keyword);
        let bound = format_optional_field(ty.bound(), doc, |bound, doc| {
            let space = doc.space();
            let bound = format_type(&bound, doc);
            doc_concat!(doc, [space, bound])
        });
        doc_concat!(doc, [before_keyword, keyword, bound])
    });
    doc_concat!(doc, [question, bound])
}

fn format_array_dimension<'source>(
    dimension: &ArrayDimension<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(dimension.annotations(), doc, |annotations, doc| {
        format_inline_annotation_parts_with_leading(annotations.parts(), doc)
    });
    let open = format_required_field(dimension.open_bracket(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let close = format_required_field(dimension.close_bracket(), doc, |token, doc| {
        format_array_dimension_close_bracket(&token, doc)
    });
    doc_concat!(doc, [annotations, open, close])
}

fn format_array_dimension_close_bracket<'source>(
    close: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        doc,
        close,
        InlineLeadingTrivia::AfterPreviousToken,
        TrailingTrivia::Preserve,
    )
}

fn format_inline_annotation_parts<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, Annotation<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_inline_annotation_parts_impl(parts, false, doc)
}

fn format_inline_annotation_parts_with_leading<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, Annotation<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_inline_annotation_parts_impl(parts, true, doc)
}

fn format_inline_annotation_parts_impl<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, Annotation<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    leading_space: bool,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut has_parts = false;
    let annotations = doc.concat_list(|docs| {
        for part in parts {
            if has_parts {
                let space = docs.space();
                docs.push(space);
            }
            has_parts = true;
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(annotation) => format_annotation(&annotation, docs),
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
        }
    });
    if has_parts {
        doc_concat!(
            doc,
            [
                if leading_space {
                    doc.space()
                } else {
                    Doc::nil()
                },
                annotations,
                doc.space()
            ]
        )
    } else {
        Doc::nil()
    }
}
