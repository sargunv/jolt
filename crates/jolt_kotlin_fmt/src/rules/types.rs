use jolt_fmt_ir::{Doc, concat, group, indent, line, space};
use jolt_kotlin_syntax::{
    ContextFunctionType, DefinitelyNonNullableType, FunctionType, FunctionTypeParameter,
    KotlinSyntaxToken, ModifierList, NullableType, ParenthesizedType, ReceiverType, TypeArgument,
    TypeArgumentList, TypeConstraint, TypeConstraintList, TypeParameter, TypeParameterList,
    TypeProjection, TypeReference, TypeSyntax, UserType,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::lists::{
    CommaListItem, angle_bracket_list, comma_list, compact_angle_bracket_list, parenthesized_list,
};
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::helpers::source::source_gap_is_trivia;
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
    let source_start = parameters.text_range().start().get();
    let source = parameters.source_text();
    let tokens = parameters.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut covered_until = parameters.open_angle().map_or_else(
        || parameters.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let mut items = Vec::new();

    for entry in parameters.entries() {
        push_recovered_type_parameter_gap(
            &mut items,
            source,
            source_start,
            &tokens,
            &mut token_cursor,
            covered_until,
            entry.parameter.text_range().start().get(),
        );
        items.push(CommaListItem {
            doc: format_type_parameter(&entry.parameter),
            comma: entry.comma,
        });
        covered_until = entry.comma.map_or_else(
            || entry.parameter.text_range().end().get(),
            |comma| comma.token_text_range().end().get(),
        );
    }

    let list_end = parameters.close_angle().map_or_else(
        || parameters.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    push_recovered_type_parameter_gap(
        &mut items,
        source,
        source_start,
        &tokens,
        &mut token_cursor,
        covered_until,
        list_end,
    );

    TypeParameterListItems { items }
}

fn push_recovered_type_parameter_gap<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) -> bool {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return false;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return false;
    }

    items.push(CommaListItem {
        doc: format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        comma: None,
    });
    true
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
    let mut entries = constraints.entries();
    let Some(mut previous) = entries.next() else {
        return jolt_fmt_ir::nil();
    };

    let first = format_type_constraint(&previous.constraint);
    let rest = std::iter::from_fn(|| {
        let entry = entries.next()?;
        let doc = format_type_constraint_continuation(
            previous.comma.as_ref(),
            format_type_constraint(&entry.constraint),
        );
        previous = entry;
        Some(doc)
    });

    concat([first, indent(concat(rest))])
}

fn format_type_constraint_continuation<'source>(
    comma: Option<&KotlinSyntaxToken<'source>>,
    constraint: Doc<'source>,
) -> Doc<'source> {
    let separator = if let Some(comma) = comma {
        concat([format_comma(comma), line()])
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
        let mut modifier_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
        modifier_prefix_from_parts(
            modifiers
                .annotations()
                .map(|annotation| format_annotation(&annotation))
                .collect(),
            &mut modifier_tokens,
        )
    })
}

fn format_comma<'source>(comma: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(
        comma,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
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
    let mut parts = Vec::new();

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
    let source_start = arguments.text_range().start().get();
    let source = arguments.source_text();
    let tokens = arguments.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut covered_until = arguments.open_angle().map_or_else(
        || arguments.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let mut items = Vec::new();
    let mut has_recovered_tokens = false;

    if let Some(projections) = arguments.projection_list() {
        for entry in projections.entries() {
            has_recovered_tokens |= push_recovered_type_argument_gap(
                &mut items,
                source,
                source_start,
                &tokens,
                &mut token_cursor,
                covered_until,
                entry.argument.text_range().start().get(),
            );
            items.push(CommaListItem {
                doc: format_type_argument(&entry.argument),
                comma: entry.comma,
            });
            covered_until = entry.comma.map_or_else(
                || entry.argument.text_range().end().get(),
                |comma| comma.token_text_range().end().get(),
            );
        }
    }

    let list_end = arguments.close_angle().map_or_else(
        || arguments.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    has_recovered_tokens |= push_recovered_type_argument_gap(
        &mut items,
        source,
        source_start,
        &tokens,
        &mut token_cursor,
        covered_until,
        list_end,
    );

    TypeArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn push_recovered_type_argument_gap<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) -> bool {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return false;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return false;
    }

    items.push(CommaListItem {
        doc: format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        comma: None,
    });
    true
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
    let mut docs = Vec::new();
    for annotation in ty.annotations() {
        docs.push(format_annotation(&annotation));
        docs.push(space());
    }

    let open = ty.open_paren();
    let close = ty.close_paren();
    docs.push(parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        ty.entries()
            .map(|entry| CommaListItem {
                doc: format_function_type_parameter(&entry.parameter),
                comma: entry.comma,
            })
            .collect(),
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
        comma_list(
            ty.context_parameters()
                .map(|parameter| CommaListItem {
                    doc: format_type_reference(&parameter),
                    comma: None,
                })
                .collect(),
        ),
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
