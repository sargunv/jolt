use jolt_fmt_ir::{Doc, concat, group, indent, line, space};
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

pub(crate) fn format_type_parameter_list(parameters: Option<TypeParameterList<'_>>) -> Doc<'_> {
    parameters.map_or_else(jolt_fmt_ir::nil, |parameters| {
        let TypeParameterListItems { items } = type_parameter_list_items(&parameters);
        let open = parameters.open_angle();
        let close = parameters.close_angle();
        angle_bracket_list(open.as_ref(), close.as_ref(), items)
    })
}

struct TypeParameterListItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn type_parameter_list_items<'source>(
    parameters: &TypeParameterList<'source>,
) -> TypeParameterListItems<'source> {
    let items =
        recovered_comma_list_items(parameters.entries_with_recovered(), |entry| CommaListItem {
            doc: format_type_parameter(&entry.parameter),
            comma: entry.comma,
        });

    TypeParameterListItems { items }
}

pub(crate) fn format_type_constraint_list(constraints: Option<TypeConstraintList<'_>>) -> Doc<'_> {
    constraints.map_or_else(jolt_fmt_ir::nil, |constraints| {
        let Some(where_token) = constraints.where_token() else {
            return jolt_fmt_ir::nil();
        };
        indent(concat([
            line(),
            format_token(
                &where_token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            ),
            space(),
            group(format_type_constraints(&constraints)),
        ]))
    })
}

fn format_type_parameter<'source>(parameter: &TypeParameter<'source>) -> Doc<'source> {
    concat([
        format_modifier_prefix(parameter.modifiers()),
        parameter
            .variance_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([
                    format_token(
                        &token,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    ),
                    space(),
                ])
            }),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        format_type_bound(parameter.colon(), parameter.bound()),
    ])
}

fn format_type_constraints<'source>(constraints: &TypeConstraintList<'source>) -> Doc<'source> {
    let items = recovered_comma_list_items(constraints.entries_with_recovered(), |entry| {
        CommaListItem {
            doc: format_type_constraint(&entry.constraint),
            comma: entry.comma,
        }
    });
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return jolt_fmt_ir::nil();
    };

    let first_doc = first.doc;
    let mut previous_comma = first.comma;
    let rest = std::iter::from_fn(|| {
        let entry = items.next()?;
        let doc = format_type_constraint_continuation(previous_comma.as_ref(), entry.doc);
        previous_comma = entry.comma;
        Some(doc)
    });

    concat([first_doc, indent(concat(rest))])
}

fn format_type_constraint_continuation<'source>(
    comma: Option<&KotlinSyntaxToken<'source>>,
    constraint: Doc<'source>,
) -> Doc<'source> {
    let separator = if let Some(comma) = comma {
        format_separator_with_comments(comma, line())
    } else {
        line()
    };

    concat([separator, constraint])
}

fn format_type_constraint<'source>(constraint: &TypeConstraint<'source>) -> Doc<'source> {
    concat([
        constraint
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        format_type_bound(constraint.colon(), constraint.bound()),
    ])
}

fn format_type_bound<'source>(
    colon: Option<KotlinSyntaxToken<'source>>,
    bound: Option<TypeReference<'source>>,
) -> Doc<'source> {
    let Some(colon) = colon else {
        return jolt_fmt_ir::nil();
    };
    concat([
        space(),
        format_token(&colon, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        space(),
        bound.map_or_else(jolt_fmt_ir::nil, |bound| format_type_reference(&bound)),
    ])
}

fn format_modifier_prefix(modifiers: Option<ModifierList<'_>>) -> Doc<'_> {
    modifiers.map_or_else(jolt_fmt_ir::nil, |modifiers| {
        modifier_prefix_from_parts(
            modifiers
                .annotations()
                .map(|annotation| format_annotation(&annotation))
                .collect(),
            modifiers.modifier_tokens(),
        )
    })
}

pub(crate) fn format_type_reference<'source>(ty: &TypeReference<'source>) -> Doc<'source> {
    ty.ty().map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty))
}

fn format_type<'source>(ty: &TypeSyntax<'source>) -> Doc<'source> {
    match ty {
        TypeSyntax::UserType(ty) => format_user_type(ty),
        TypeSyntax::NullableType(ty) => format_nullable_type(ty),
        TypeSyntax::FunctionType(ty) => format_function_type(ty),
        TypeSyntax::ContextFunctionType(ty) => format_context_function_type(ty),
        TypeSyntax::ReceiverType(ty) => format_receiver_type(ty),
        TypeSyntax::ParenthesizedType(ty) => format_parenthesized_type(ty),
        TypeSyntax::DefinitelyNonNullableType(ty) => format_definitely_non_nullable_type(ty),
    }
}

fn format_user_type<'source>(ty: &UserType<'source>) -> Doc<'source> {
    let identifiers = ty.identifier_tokens().collect::<Vec<_>>();
    if identifiers.is_empty() {
        return format_token_sequence(ty.token_iter(), LeadingTrivia::Preserve);
    }
    let arguments = ty.type_argument_lists().collect::<Vec<_>>();
    let mut dots = ty.dot_tokens();
    let mut parts = Vec::with_capacity(
        identifiers
            .len()
            .saturating_mul(2)
            .saturating_add(arguments.len()),
    );

    for annotation in ty.annotations() {
        parts.push(format_annotation(&annotation));
        parts.push(space());
    }

    for (index, identifier) in identifiers.iter().enumerate() {
        if index > 0 {
            parts.push(dots.next().map_or_else(jolt_fmt_ir::nil, |dot| {
                format_token(&dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }));
        }
        let identifier_end = identifier.token_text_range().end();
        let next_identifier_start = identifiers.get(index + 1).map_or_else(
            || ty.text_range().end(),
            |next| next.token_text_range().start(),
        );
        parts.push(format_token(
            identifier,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ));
        for arguments in arguments.iter().filter(|arguments| {
            arguments.text_range().start() >= identifier_end
                && arguments.text_range().start() < next_identifier_start
        }) {
            parts.push(format_type_argument_list(arguments));
        }
    }

    group(concat(parts))
}

pub(crate) fn format_type_argument_list<'source>(
    arguments: &TypeArgumentList<'source>,
) -> Doc<'source> {
    let TypeArgumentListItems {
        items,
        has_recovered_tokens,
    } = type_argument_list_items(arguments);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_trailing_comma || has_recovered_tokens {
        angle_bracket_list(
            arguments.open_angle().as_ref(),
            arguments.close_angle().as_ref(),
            items,
        )
    } else {
        compact_angle_bracket_list(
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
                    doc: format_type_argument(&entry.argument),
                    comma: entry.comma,
                },
                RecoveredSeparatedListEntry::Token(token) => CommaListItem {
                    doc: format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                    comma: None,
                },
                RecoveredSeparatedListEntry::Error(error) => CommaListItem {
                    doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                },
                RecoveredSeparatedListEntry::Node(node) => CommaListItem {
                    doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
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

fn format_type_argument<'source>(argument: &TypeArgument<'source>) -> Doc<'source> {
    argument.projection().map_or_else(
        || format_token_sequence(argument.token_iter(), LeadingTrivia::Preserve),
        |projection| format_type_projection(&projection),
    )
}

fn format_type_projection<'source>(projection: &TypeProjection<'source>) -> Doc<'source> {
    if let Some(star) = projection.star_token() {
        return format_token(&star, LeadingTrivia::Preserve, TrailingTrivia::Preserve);
    }

    concat([
        projection
            .variance_token()
            .map_or_else(jolt_fmt_ir::nil, |variance| {
                concat([
                    format_token(
                        &variance,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    ),
                    space(),
                ])
            }),
        projection
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type_reference(&ty)),
    ])
}

fn format_nullable_type<'source>(ty: &NullableType<'source>) -> Doc<'source> {
    concat([
        ty.inner()
            .map_or_else(jolt_fmt_ir::nil, |inner| format_type(&inner)),
        ty.question_token()
            .map_or_else(jolt_fmt_ir::nil, |question| {
                format_token(&question, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }),
    ])
}

fn format_parenthesized_type<'source>(ty: &ParenthesizedType<'source>) -> Doc<'source> {
    let mut docs = Vec::with_capacity(3);
    for annotation in ty.annotations() {
        docs.push(format_annotation(&annotation));
        docs.push(space());
    }

    let open = ty.open_paren();
    let close = ty.close_paren();
    docs.push(parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        recovered_comma_list_items(ty.entries_with_recovered(), |entry| CommaListItem {
            doc: format_function_type_parameter(&entry.parameter),
            comma: entry.comma,
        }),
    ));
    concat(docs)
}

fn format_function_type_parameter<'source>(
    parameter: &FunctionTypeParameter<'source>,
) -> Doc<'source> {
    concat([
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        parameter.colon().map_or_else(jolt_fmt_ir::nil, |colon| {
            concat([
                format_token(
                    &colon,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ),
                space(),
            ])
        }),
        parameter
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type_reference(&ty)),
    ])
}

fn format_receiver_type<'source>(ty: &ReceiverType<'source>) -> Doc<'source> {
    concat([
        ty.receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| format_type(&receiver)),
        ty.dot_token().map_or_else(jolt_fmt_ir::nil, |dot| {
            format_token(&dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        ty.parameter().map_or_else(jolt_fmt_ir::nil, |parameter| {
            format_parenthesized_type(&parameter)
        }),
    ])
}

fn format_function_type<'source>(ty: &FunctionType<'source>) -> Doc<'source> {
    if let Some(suspend) = ty.suspend_token()
        && let Some(nested) = ty.nested_function_type()
    {
        return concat([
            format_token(
                &suspend,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            space(),
            format_function_type(&nested),
        ]);
    }

    let head = ty.receiver().map_or_else(
        || {
            ty.parameter().map_or_else(jolt_fmt_ir::nil, |parameter| {
                format_parenthesized_type(&parameter)
            })
        },
        |receiver| format_receiver_type(&receiver),
    );

    concat([
        ty.suspend_token().map_or_else(jolt_fmt_ir::nil, |suspend| {
            concat([
                format_token(
                    &suspend,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ),
                space(),
            ])
        }),
        head,
        space(),
        ty.arrow_token().map_or_else(jolt_fmt_ir::nil, |arrow| {
            format_token(&arrow, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        space(),
        ty.return_type()
            .map_or_else(jolt_fmt_ir::nil, |return_type| format_type(&return_type)),
    ])
}

fn format_context_function_type<'source>(ty: &ContextFunctionType<'source>) -> Doc<'source> {
    concat([
        ty.context_token().map_or_else(jolt_fmt_ir::nil, |context| {
            format_token(
                &context,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
        ty.open_paren().map_or_else(jolt_fmt_ir::nil, |open| {
            format_token(
                &open,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
        comma_list(recovered_comma_list_items(
            ty.context_parameter_entries_with_recovered(),
            |entry| CommaListItem {
                doc: entry
                    .parameter
                    .ty()
                    .map_or_else(jolt_fmt_ir::nil, |parameter| {
                        format_type_reference(&parameter)
                    }),
                comma: entry.comma,
            },
        )),
        ty.close_paren().map_or_else(jolt_fmt_ir::nil, |close| {
            format_token(&close, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        space(),
        ty.function_type()
            .map_or_else(jolt_fmt_ir::nil, |function| format_function_type(&function)),
    ])
}

fn format_definitely_non_nullable_type<'source>(
    ty: &DefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    let mut types = ty.types();
    let Some(first) = types.next() else {
        return format_token_sequence(ty.token_iter(), LeadingTrivia::Preserve);
    };
    concat([
        format_user_type(&first),
        ty.amp_token().map_or_else(jolt_fmt_ir::nil, |amp| {
            concat([
                space(),
                format_token(
                    &amp,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ),
                space(),
            ])
        }),
        types
            .next()
            .map_or_else(jolt_fmt_ir::nil, |second| format_user_type(&second)),
    ])
}
