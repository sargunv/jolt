use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, ArrayDimension, ArrayDimensions, ClassType, IntersectionType,
    IntersectionTypeEntry, JavaSyntaxToken, NameSyntax, PrimitiveType, RecoveredSeparatedListEntry,
    Type, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter, TypeParameterList,
    UnionType, UnionTypeEntry, VoidType, WildcardType,
};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line, format_token,
    format_token_after_relocated_leading_comments, format_token_sequence,
    format_token_with_comments, format_token_with_inline_leading_comments,
};
use crate::helpers::lists::{CommaListItem, angle_bracket_list, recovered_comma_list_items};
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
            let element = match ty.element_type() {
                Some(element) => format_type_with_leading_comments(&element, leading_comments, doc),
                None => Doc::nil(),
            };
            let dimensions = match ty.dimensions() {
                Some(dimensions) => format_array_dimensions(&dimensions, doc),
                None => Doc::nil(),
            };
            doc_concat!(doc, [element, dimensions])
        }
        Type::IntersectionType(ty) => format_intersection_type(ty, doc),
        Type::UnionType(ty) => format_union_type(ty, doc),
        Type::WildcardType(ty) => format_wildcard_type(ty, doc),
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
            let open = parameters.open_angle();
            let close = parameters.close_angle();
            let items = type_parameter_list_items(&parameters, doc);
            angle_bracket_list(doc, open.as_ref(), close.as_ref(), items)
        }
        None => Doc::nil(),
    }
}

pub(crate) fn format_type_argument_list<'source>(
    arguments: &TypeArgumentList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = arguments.open_angle();
    let close = arguments.close_angle();
    let items = type_argument_list_items(arguments, doc);
    angle_bracket_list(doc, open.as_ref(), close.as_ref(), items)
}

fn type_parameter_list_items<'source, 'fmt>(
    parameters: &'fmt TypeParameterList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, parameters.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_type_parameter(&entry.parameter, doc),
            comma: entry.comma,
        }
    })
}

fn type_argument_list_items<'source, 'fmt>(
    arguments: &'fmt TypeArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, arguments.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_type_argument(&entry.argument, doc),
            comma: entry.comma,
        }
    })
}

pub(crate) fn format_array_dimensions<'source>(
    dimensions: &ArrayDimensions<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for dimension in dimensions.dimensions() {
        let dimension = format_array_dimension(&dimension, doc);
        docs.push(dimension, doc);
    }
    docs.finish(doc)
}

fn format_primitive_type<'source>(
    ty: &PrimitiveType<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_inline_annotations(ty.annotations(), doc);
    let keyword = match ty.keyword() {
        Some(keyword) => format_type_head_token(&keyword, leading_comments, doc),
        None => Doc::nil(),
    };
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
    match ty.keyword() {
        Some(keyword) => format_type_head_token(&keyword, leading_comments, doc),
        None => Doc::nil(),
    }
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
    let segments = ty.segments();
    let mut docs = doc.list();
    for (index, segment) in segments.enumerate() {
        if index > 0 {
            let dot = match segment.dot_before.as_ref() {
                Some(token) => format_token_with_comments(doc, token),
                None => Doc::nil(),
            };
            docs.push(dot, doc);
        }
        let segment = format_class_type_segment(
            segment,
            if index == 0 {
                leading_comments
            } else {
                LeadingComments::Preserve
            },
            doc,
        );
        docs.push(segment, doc);
    }
    let contents = docs.finish(doc);
    doc_group!(doc, contents)
}

fn format_class_type_segment<'source>(
    segment: jolt_java_syntax::ClassTypeSegment<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_inline_annotations(segment.annotations, doc);
    let name = format_type_name(&segment.name, leading_comments, doc);
    let type_arguments = match segment.type_arguments {
        Some(arguments) => format_type_argument_list(&arguments, doc),
        None => Doc::nil(),
    };
    doc_concat!(doc, [annotations, name, type_arguments])
}

fn format_type_name<'source>(
    name: &NameSyntax<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let segments = name.segments_with_annotations();
    let mut docs = doc.list();
    for (index, segment) in segments.enumerate() {
        if index > 0 {
            let dot = match segment.dot_before.as_ref() {
                Some(token) => format_token_with_comments(doc, token),
                None => Doc::nil(),
            };
            docs.push(dot, doc);
        }
        let annotations = format_inline_annotations(segment.annotations, doc);
        docs.push(annotations, doc);
        let identifier = if index == 0 {
            format_type_head_token(&segment.identifier, leading_comments, doc)
        } else {
            format_token_with_comments(doc, &segment.identifier)
        };
        docs.push(identifier, doc);
    }
    docs.finish(doc)
}

fn format_intersection_type<'source>(
    ty: &IntersectionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_intersection_entries(ty, doc)
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
    let annotations = format_inline_annotations(parameter.annotations(), doc);
    let name = match parameter.name() {
        Some(name) => format_token_with_comments(doc, &name),
        None => Doc::nil(),
    };
    let bounds = match parameter.bounds() {
        Some(bounds) => {
            let space_before_extends = doc.space();
            let extends = match bounds.extends_token().as_ref() {
                Some(token) => format_token_with_comments(doc, token),
                None => Doc::nil(),
            };
            let space_after_extends = doc.space();
            let bounds = format_type_bounds(&bounds, doc);
            doc_concat!(
                doc,
                [space_before_extends, extends, space_after_extends, bounds]
            )
        }
        None => Doc::nil(),
    };
    doc_concat!(doc, [annotations, name, bounds])
}

fn format_type_bounds<'source>(
    bounds: &TypeBoundList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parts = type_operator_parts(doc, bounds.entries_with_recovered());
    format_type_operator_entries_doc(parts, doc)
}

fn format_intersection_entries<'source>(
    ty: &IntersectionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parts = type_operator_parts(doc, ty.entries_with_recovered());
    format_type_operator_entries(parts, doc)
}

fn format_union_entries<'source>(
    ty: &UnionType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parts = type_operator_parts(doc, ty.entries_with_recovered());
    format_type_operator_entries(parts, doc)
}

fn format_type_operator_entries<'source>(
    entries: impl IntoIterator<Item = TypeOperatorPart<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_group!(doc, format_type_operator_entries_doc(entries, doc))
}

fn format_type_operator_entries_doc<'source>(
    entries: impl IntoIterator<Item = TypeOperatorPart<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let Some(first) = entries.next() else {
        return Doc::nil();
    };

    let (first, mut previous_separator) = format_type_operator_first_part(first, doc);
    let mut rest = doc.list();
    for part in entries {
        match part {
            TypeOperatorPart::Type { ty, separator } => {
                let operand = format_type(&ty, doc);
                let continuation =
                    format_type_operator_continuation(previous_separator.as_ref(), operand, doc);
                rest.push(continuation, doc);
                previous_separator = separator;
            }
            TypeOperatorPart::Recovered(recovered) => {
                let continuation =
                    format_type_operator_continuation(previous_separator.as_ref(), recovered, doc);
                rest.push(continuation, doc);
                previous_separator = None;
            }
        }
    }
    if previous_separator.is_some() {
        let continuation =
            format_type_operator_continuation(previous_separator.as_ref(), Doc::nil(), doc);
        rest.push(continuation, doc);
    }

    let rest = rest.finish(doc);
    let rest = doc_indent!(doc, rest);
    doc_concat!(doc, [first, rest])
}

#[derive(Clone, Copy)]
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
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
) -> Vec<TypeOperatorPart<'source>>
where
    Entry: TypeOperatorEntry<'source>,
{
    entries
        .into_iter()
        .map(|entry| match entry {
            RecoveredSeparatedListEntry::Entry(entry) => TypeOperatorPart::Type {
                ty: entry.ty(),
                separator: entry.separator(),
            },
            RecoveredSeparatedListEntry::Token(token) => TypeOperatorPart::Recovered(format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )),
            RecoveredSeparatedListEntry::Error(error) => TypeOperatorPart::Recovered(
                format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
            ),
            RecoveredSeparatedListEntry::Node(node) => TypeOperatorPart::Recovered(
                format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
            ),
        })
        .collect()
}

fn format_type_operator_first_part<'source>(
    part: TypeOperatorPart<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, Option<JavaSyntaxToken<'source>>) {
    match part {
        TypeOperatorPart::Type { ty, separator } => (format_type(&ty, doc), separator),
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
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let separator_doc = format_type_operator_separator_before_operand(separator, doc);

    if type_operator_separator_forces_line(separator) {
        let hard_line = doc.hard_line();
        let operand = doc_concat!(doc, [hard_line, operand]);
        let operand = doc_indent!(doc, operand);
        doc_concat!(doc, [separator_doc, operand])
    } else {
        doc_concat!(doc, [separator_doc, operand])
    }
}

fn format_type_operator_separator_before_operand<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(separator) = separator {
        format_type_operator_separator(separator, doc)
    } else {
        doc.line()
    }
}

fn format_type_operator_separator<'source>(
    separator: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let line = doc.line();
    let forces_line = separator
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
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
    let annotations = format_inline_annotations(argument.annotations(), doc);
    let ty = match argument.ty() {
        Some(ty) => format_type(&ty, doc),
        None => Doc::nil(),
    };
    let has_annotations = argument.annotations().next().is_some();
    if argument.ty().is_none() && !has_annotations {
        return format_token_sequence(doc, argument.token_iter(), LeadingTrivia::Preserve);
    }

    doc_concat!(doc, [annotations, ty])
}

fn format_wildcard_type<'source>(
    ty: &WildcardType<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let question = match ty.question_token().as_ref() {
        Some(token) => format_token_with_comments(doc, token),
        None => Doc::nil(),
    };
    let bound = match ty.bound_keyword() {
        Some(keyword) => {
            let before_keyword = doc.space();
            let keyword = format_token_with_comments(doc, &keyword);
            let bound = match ty.bound() {
                Some(bound) => {
                    let space = doc.space();
                    let bound = format_type(&bound, doc);
                    doc_concat!(doc, [space, bound])
                }
                None => Doc::nil(),
            };
            doc_concat!(doc, [before_keyword, keyword, bound])
        }
        None => Doc::nil(),
    };
    doc_concat!(doc, [question, bound])
}

fn format_array_dimension<'source>(
    dimension: &ArrayDimension<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut annotations = dimension.annotations().peekable();
    if annotations.peek().is_none() {
        let open = match dimension.open_bracket().as_ref() {
            Some(token) => format_token_with_comments(doc, token),
            None => Doc::nil(),
        };
        let close = match dimension.close_bracket().as_ref() {
            Some(token) => format_array_dimension_close_bracket(token, doc),
            None => Doc::nil(),
        };
        return doc_concat!(doc, [open, close]);
    }

    let space = doc.space();
    let annotations = format_inline_annotations(annotations, doc);
    let open = match dimension.open_bracket().as_ref() {
        Some(token) => format_token_with_comments(doc, token),
        None => Doc::nil(),
    };
    let close = match dimension.close_bracket().as_ref() {
        Some(token) => format_array_dimension_close_bracket(token, doc),
        None => Doc::nil(),
    };
    doc_concat!(doc, [space, annotations, open, close])
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

pub(crate) fn format_inline_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut annotations = annotations.into_iter().peekable();
    if annotations.peek().is_none() {
        return Doc::nil();
    }

    let mut docs = doc.list();
    for annotation in annotations {
        if !docs.is_empty() {
            let space = doc.space();
            docs.push(space, doc);
        }
        let annotation = format_annotation(&annotation, doc);
        docs.push(annotation, doc);
    }
    let annotations = docs.finish(doc);
    let trailing_space = doc.space();
    doc_concat!(doc, [annotations, trailing_space])
}
