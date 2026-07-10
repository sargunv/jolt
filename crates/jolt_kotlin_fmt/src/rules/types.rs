use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ContextFunctionType, DefinitelyNonNullableType, FunctionType, FunctionTypeParameter,
    KotlinSyntaxToken, ModifierList, NullableType, ParenthesizedType, ReceiverType,
    RecoveredSeparatedListEntry, TypeArgument, TypeArgumentList, TypeConstraint,
    TypeConstraintList, TypeParameter, TypeParameterList, TypeProjection, TypeReference,
    TypeSyntax, UserType,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_separator_with_comments, format_token,
    format_token_sequence,
};
use crate::helpers::lists::{
    CommaListItem, angle_bracket_list, comma_list, compact_angle_bracket_list, parenthesized_list,
    recovered_comma_list_items,
};
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::rules::annotations::format_annotation;
use crate::rules::names::format_name;

pub(crate) fn format_type_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    parameters: Option<TypeParameterList<'source>>,
) -> Doc<'source> {
    if let Some(parameters) = parameters {
        let TypeParameterListItems { items } = type_parameter_list_items(doc, &parameters);
        let open = parameters.open_angle();
        let close = parameters.close_angle();
        angle_bracket_list(doc, open.as_ref(), close.as_ref(), items)
    } else {
        doc.nil()
    }
}

struct TypeParameterListItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn type_parameter_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    parameters: &TypeParameterList<'source>,
) -> TypeParameterListItems<'source> {
    let items =
        recovered_comma_list_items(doc, parameters.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: format_type_parameter(doc, &entry.parameter),
                comma: entry.comma,
            }
        });

    TypeParameterListItems { items }
}

pub(crate) fn format_type_constraint_list<'source>(
    doc: &mut DocBuilder<'source>,
    constraints: Option<TypeConstraintList<'source>>,
) -> Doc<'source> {
    let Some(constraints) = constraints else {
        return doc.nil();
    };
    let Some(where_token) = constraints.where_token() else {
        return doc.nil();
    };
    let line = doc.line();
    let where_token = format_token(
        doc,
        &where_token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let space = doc.space();
    let constraints = format_type_constraints(doc, &constraints);
    let constraints = doc.group(constraints);
    let contents = doc.concat([line, where_token, space, constraints]);
    doc.indent(contents)
}

fn format_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &TypeParameter<'source>,
) -> Doc<'source> {
    let modifiers = format_modifier_prefix(doc, parameter.modifiers());
    let variance = if let Some(token) = parameter.variance_token() {
        let token = format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        doc.concat([token, space])
    } else {
        doc.nil()
    };
    let name = if let Some(name) = parameter.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let bound = format_type_bound(doc, parameter.colon(), parameter.bound());
    doc.concat([modifiers, variance, name, bound])
}

fn format_type_constraints<'source>(
    doc: &mut DocBuilder<'source>,
    constraints: &TypeConstraintList<'source>,
) -> Doc<'source> {
    let items =
        recovered_comma_list_items(doc, constraints.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: format_type_constraint(doc, &entry.constraint),
                comma: entry.comma,
            }
        });
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return doc.nil();
    };

    let first_doc = first.doc;
    let mut previous_comma = first.comma;
    let rest = doc.concat_list(|rest| {
        for entry in items {
            let continuation =
                format_type_constraint_continuation(rest, previous_comma.as_ref(), entry.doc);
            previous_comma = entry.comma;
            rest.push(continuation);
        }
    });
    let rest = doc.indent(rest);
    doc.concat([first_doc, rest])
}

fn format_type_constraint_continuation<'source>(
    doc: &mut DocBuilder<'source>,
    comma: Option<&KotlinSyntaxToken<'source>>,
    constraint: Doc<'source>,
) -> Doc<'source> {
    let separator = if let Some(comma) = comma {
        let line = doc.line();
        format_separator_with_comments(doc, comma, line)
    } else {
        doc.line()
    };

    doc.concat([separator, constraint])
}

fn format_type_constraint<'source>(
    doc: &mut DocBuilder<'source>,
    constraint: &TypeConstraint<'source>,
) -> Doc<'source> {
    let name = if let Some(name) = constraint.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let bound = format_type_bound(doc, constraint.colon(), constraint.bound());
    doc.concat([name, bound])
}

fn format_type_bound<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Option<KotlinSyntaxToken<'source>>,
    bound: Option<TypeReference<'source>>,
) -> Doc<'source> {
    let Some(colon) = colon else {
        return doc.nil();
    };
    let before = doc.space();
    let colon = format_token(
        doc,
        &colon,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let after = doc.space();
    let bound = if let Some(bound) = bound {
        format_type_reference(doc, &bound)
    } else {
        doc.nil()
    };
    doc.concat([before, colon, after, bound])
}

fn format_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: Option<ModifierList<'source>>,
) -> Doc<'source> {
    if let Some(modifiers) = modifiers {
        let annotations = modifiers
            .annotations()
            .map(|annotation| format_annotation(doc, &annotation))
            .collect::<Vec<_>>();
        modifier_prefix_from_parts(doc, annotations, modifiers.modifier_tokens())
    } else {
        doc.nil()
    }
}

pub(crate) fn format_type_reference<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &TypeReference<'source>,
) -> Doc<'source> {
    if let Some(ty) = ty.ty() {
        format_type(doc, &ty)
    } else {
        doc.nil()
    }
}

fn format_type<'source>(doc: &mut DocBuilder<'source>, ty: &TypeSyntax<'source>) -> Doc<'source> {
    match ty {
        TypeSyntax::UserType(ty) => format_user_type(doc, ty),
        TypeSyntax::NullableType(ty) => format_nullable_type(doc, ty),
        TypeSyntax::FunctionType(ty) => format_function_type(doc, ty),
        TypeSyntax::ContextFunctionType(ty) => format_context_function_type(doc, ty),
        TypeSyntax::ReceiverType(ty) => format_receiver_type(doc, ty),
        TypeSyntax::ParenthesizedType(ty) => format_parenthesized_type(doc, ty),
        TypeSyntax::DefinitelyNonNullableType(ty) => format_definitely_non_nullable_type(doc, ty),
    }
}

fn format_user_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &UserType<'source>,
) -> Doc<'source> {
    let identifiers = ty.identifier_tokens().collect::<Vec<_>>();
    if identifiers.is_empty() {
        return format_token_sequence(doc, ty.token_iter(), LeadingTrivia::Preserve);
    }
    let arguments = ty.type_argument_lists().collect::<Vec<_>>();
    let mut dots = ty.dot_tokens();
    let parts = doc.concat_list(|parts| {
        for annotation in ty.annotations() {
            let annotation = format_annotation(parts, &annotation);
            parts.push(annotation);
            let space = parts.space();
            parts.push(space);
        }

        for (index, identifier) in identifiers.iter().enumerate() {
            if index > 0 {
                let dot = if let Some(dot) = dots.next() {
                    format_token(
                        parts,
                        &dot,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    )
                } else {
                    parts.nil()
                };
                parts.push(dot);
            }
            let identifier_end = identifier.token_text_range().end();
            let next_identifier_start = identifiers.get(index + 1).map_or_else(
                || ty.text_range().end(),
                |next| next.token_text_range().start(),
            );
            let identifier = format_token(
                parts,
                identifier,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            parts.push(identifier);
            for arguments in arguments.iter().filter(|arguments| {
                arguments.text_range().start() >= identifier_end
                    && arguments.text_range().start() < next_identifier_start
            }) {
                let arguments = format_type_argument_list(parts, arguments);
                parts.push(arguments);
            }
        }
    });
    doc.group(parts)
}

pub(crate) fn format_type_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &TypeArgumentList<'source>,
) -> Doc<'source> {
    let TypeArgumentListItems {
        items,
        has_recovered_tokens,
    } = type_argument_list_items(doc, arguments);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_trailing_comma || has_recovered_tokens {
        angle_bracket_list(
            doc,
            arguments.open_angle().as_ref(),
            arguments.close_angle().as_ref(),
            items,
        )
    } else {
        compact_angle_bracket_list(
            doc,
            arguments.open_angle().as_ref(),
            arguments.close_angle().as_ref(),
            items,
        )
    }
}

struct TypeArgumentListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_recovered_tokens: bool,
}

fn type_argument_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &TypeArgumentList<'source>,
) -> TypeArgumentListItems<'source> {
    if let Some(projections) = arguments.projection_list() {
        let entries = projections.entries_with_recovered();
        let (lower, _) = entries.size_hint();
        let mut items = Vec::with_capacity(lower);
        let mut has_recovered_tokens = false;
        for entry in entries {
            has_recovered_tokens |= !matches!(entry, RecoveredSeparatedListEntry::Entry(_));
            items.push(match entry {
                RecoveredSeparatedListEntry::Entry(entry) => CommaListItem {
                    doc: format_type_argument(doc, &entry.argument),
                    comma: entry.comma,
                },
                RecoveredSeparatedListEntry::Token(token) => CommaListItem {
                    doc: format_token(
                        doc,
                        &token,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    ),
                    comma: None,
                },
                RecoveredSeparatedListEntry::Error(error) => CommaListItem {
                    doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                },
                RecoveredSeparatedListEntry::Node(node) => CommaListItem {
                    doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                },
            });
        }
        return TypeArgumentListItems {
            items,
            has_recovered_tokens,
        };
    }

    TypeArgumentListItems {
        items: Vec::new(),
        has_recovered_tokens: false,
    }
}

fn format_type_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &TypeArgument<'source>,
) -> Doc<'source> {
    if let Some(projection) = argument.projection() {
        format_type_projection(doc, &projection)
    } else {
        format_token_sequence(doc, argument.token_iter(), LeadingTrivia::Preserve)
    }
}

fn format_type_projection<'source>(
    doc: &mut DocBuilder<'source>,
    projection: &TypeProjection<'source>,
) -> Doc<'source> {
    if let Some(star) = projection.star_token() {
        return format_token(
            doc,
            &star,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
    }

    let variance = if let Some(variance) = projection.variance_token() {
        let variance = format_token(
            doc,
            &variance,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        doc.concat([variance, space])
    } else {
        doc.nil()
    };
    let ty = if let Some(ty) = projection.ty() {
        format_type_reference(doc, &ty)
    } else {
        doc.nil()
    };
    doc.concat([variance, ty])
}

fn format_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &NullableType<'source>,
) -> Doc<'source> {
    let inner = if let Some(inner) = ty.inner() {
        format_type(doc, &inner)
    } else {
        doc.nil()
    };
    let question = if let Some(question) = ty.question_token() {
        format_token(
            doc,
            &question,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([inner, question])
}

fn format_parenthesized_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ParenthesizedType<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for annotation in ty.annotations() {
            let annotation = format_annotation(docs, &annotation);
            docs.push(annotation);
            let space = docs.space();
            docs.push(space);
        }

        let open = ty.open_paren();
        let close = ty.close_paren();
        let items = recovered_comma_list_items(docs, ty.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: format_function_type_parameter(doc, &entry.parameter),
                comma: entry.comma,
            }
        });
        let list = parenthesized_list(docs, open.as_ref(), close.as_ref(), items);
        docs.push(list);
    })
}

fn format_function_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &FunctionTypeParameter<'source>,
) -> Doc<'source> {
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
            TrailingTrivia::RelocatedToEnclosingContext,
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
    doc.concat([name, colon, ty])
}

fn format_receiver_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ReceiverType<'source>,
) -> Doc<'source> {
    let receiver = if let Some(receiver) = ty.receiver() {
        format_type(doc, &receiver)
    } else {
        doc.nil()
    };
    let dot = if let Some(dot) = ty.dot_token() {
        format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    } else {
        doc.nil()
    };
    let parameter = if let Some(parameter) = ty.parameter() {
        format_parenthesized_type(doc, &parameter)
    } else {
        doc.nil()
    };
    doc.concat([receiver, dot, parameter])
}

fn format_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &FunctionType<'source>,
) -> Doc<'source> {
    if let Some(suspend) = ty.suspend_token()
        && let Some(nested) = ty.nested_function_type()
    {
        let suspend = format_token(
            doc,
            &suspend,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        let nested = format_function_type(doc, &nested);
        return doc.concat([suspend, space, nested]);
    }

    let head = if let Some(receiver) = ty.receiver() {
        format_receiver_type(doc, &receiver)
    } else if let Some(parameter) = ty.parameter() {
        format_parenthesized_type(doc, &parameter)
    } else {
        doc.nil()
    };

    let suspend = if let Some(suspend) = ty.suspend_token() {
        let suspend = format_token(
            doc,
            &suspend,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        doc.concat([suspend, space])
    } else {
        doc.nil()
    };
    let before_arrow = doc.space();
    let arrow = if let Some(arrow) = ty.arrow_token() {
        format_token(
            doc,
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let after_arrow = doc.space();
    let return_type = if let Some(return_type) = ty.return_type() {
        format_type(doc, &return_type)
    } else {
        doc.nil()
    };

    doc.concat([suspend, head, before_arrow, arrow, after_arrow, return_type])
}

fn format_context_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ContextFunctionType<'source>,
) -> Doc<'source> {
    let context = if let Some(context) = ty.context_token() {
        format_token(
            doc,
            &context,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let open = if let Some(open) = ty.open_paren() {
        format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let items = recovered_comma_list_items(
        doc,
        ty.context_parameter_entries_with_recovered(),
        |doc, entry| CommaListItem {
            doc: if let Some(parameter) = entry.parameter.ty() {
                format_type_reference(doc, &parameter)
            } else {
                doc.nil()
            },
            comma: entry.comma,
        },
    );
    let parameters = comma_list(doc, items);
    let close = if let Some(close) = ty.close_paren() {
        format_token(
            doc,
            &close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let space = doc.space();
    let function_type = if let Some(function) = ty.function_type() {
        format_function_type(doc, &function)
    } else {
        doc.nil()
    };
    doc.concat([context, open, parameters, close, space, function_type])
}

fn format_definitely_non_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &DefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    let mut types = ty.types();
    let Some(first) = types.next() else {
        return format_token_sequence(doc, ty.token_iter(), LeadingTrivia::Preserve);
    };
    let first = format_user_type(doc, &first);
    let amp = if let Some(amp) = ty.amp_token() {
        let before = doc.space();
        let amp = format_token(
            doc,
            &amp,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let after = doc.space();
        doc.concat([before, amp, after])
    } else {
        doc.nil()
    };
    let second = if let Some(second) = types.next() {
        format_user_type(doc, &second)
    } else {
        doc.nil()
    };
    doc.concat([first, amp, second])
}
