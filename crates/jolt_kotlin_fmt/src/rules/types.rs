use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Annotation, ArrowFunctionType, BangDefinitelyNonNullableType, ContextFunctionType,
    DefinitelyNonNullableType, FunctionType, FunctionTypeParameter,
    IntersectionDefinitelyNonNullableType, KotlinRoleElement, KotlinSyntaxField,
    KotlinSyntaxInvariantError, KotlinSyntaxToken, ModifierList, ModifierListSequence,
    NullableType, ParenthesizedType, ReceiverType, SuspendedFunctionType, TypeArgument,
    TypeArgumentList, TypeConstraint, TypeConstraintList, TypeParameter, TypeParameterList,
    TypeProjection, TypeReference, TypeSyntax, UserType,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_separator_with_comments, format_token,
};
use crate::helpers::lists::{
    CommaListItem, angle_bracket_list, compact_angle_bracket_list, parenthesized_list,
    physical_comma_list_items,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, KotlinFormatListPart, format_optional_field,
    format_or_verbatim, format_required_field, resolve_list_part, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::names::format_name;

pub(crate) fn format_type_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    parameters: Option<TypeParameterList<'source>>,
) -> Doc<'source> {
    let Some(parameters) = parameters else {
        return doc.nil();
    };
    format_or_verbatim(&parameters, doc, |doc| {
        let open = resolve_required_delimiter(parameters.open_angle(), doc);
        let close = resolve_required_delimiter(parameters.close_angle(), doc);
        let items = match resolve_required_field(parameters.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, parameter| CommaListItem {
                    doc: format_type_parameter(doc, &parameter),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
        };
        format_angle_delimiters(doc, &open, &close, items, false)
    })
}

pub(crate) fn format_type_constraint_list<'source>(
    doc: &mut DocBuilder<'source>,
    constraints: Option<TypeConstraintList<'source>>,
) -> Doc<'source> {
    let Some(constraints) = constraints else {
        return doc.nil();
    };
    format_or_verbatim(&constraints, doc, |doc| {
        let where_token = format_required_field(constraints.where_token(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        });
        let items = match resolve_required_field(constraints.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, constraint| CommaListItem {
                    doc: format_type_constraint(doc, &constraint),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
        };
        let constraints = format_constraint_items(doc, items);
        let line = doc.line();
        let space = doc.space();
        let constraints = doc.group(constraints);
        let contents = doc.concat([line, where_token, space, constraints]);
        doc.indent(contents)
    })
}

fn format_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &TypeParameter<'source>,
) -> Doc<'source> {
    format_or_verbatim(parameter, doc, |doc| {
        let modifiers = format_required_field(parameter.modifiers(), doc, |modifiers, doc| {
            format_modifier_sequence(doc, &modifiers)
        });
        let variance = format_optional_field(parameter.variance(), doc, |role, doc| {
            let token = format_role_token(doc, role, TrailingTrivia::RelocatedToEnclosingContext);
            let space = doc.space();
            doc.concat([token, space])
        });
        let name =
            format_required_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
        let bound = format_optional_type_bound(doc, parameter.colon(), parameter.bound());
        doc.concat([modifiers, variance, name, bound])
    })
}

fn format_type_constraint<'source>(
    doc: &mut DocBuilder<'source>,
    constraint: &TypeConstraint<'source>,
) -> Doc<'source> {
    format_or_verbatim(constraint, doc, |doc| {
        let name =
            format_required_field(constraint.name(), doc, |name, doc| format_name(doc, &name));
        let bound = format_required_type_bound(doc, constraint.colon(), constraint.bound());
        doc.concat([name, bound])
    })
}

fn format_optional_type_bound<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        KotlinSyntaxInvariantError,
    >,
    bound: Result<KotlinSyntaxField<'source, TypeReference<'source>>, KotlinSyntaxInvariantError>,
) -> Doc<'source> {
    match crate::helpers::recovery::resolve_optional_field(colon, doc) {
        KotlinFormatField::Present(None) => {
            format_optional_field(bound, doc, |bound, doc| format_type_reference(doc, &bound))
        }
        KotlinFormatField::Present(Some(colon)) => {
            let before = doc.space();
            let colon = format_token(
                doc,
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let after = doc.space();
            let bound =
                format_optional_field(bound, doc, |bound, doc| format_type_reference(doc, &bound));
            doc.concat([before, colon, after, bound])
        }
        KotlinFormatField::Malformed(recovery) => {
            let bound =
                format_optional_field(bound, doc, |bound, doc| format_type_reference(doc, &bound));
            doc.concat([recovery, bound])
        }
    }
}

fn format_required_type_bound<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        KotlinSyntaxInvariantError,
    >,
    bound: Result<KotlinSyntaxField<'source, TypeReference<'source>>, KotlinSyntaxInvariantError>,
) -> Doc<'source> {
    let before = doc.space();
    let colon = format_required_field(colon, doc, |colon, doc| {
        format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let after = doc.space();
    let bound = format_required_field(bound, doc, |bound, doc| format_type_reference(doc, &bound));
    doc.concat([before, colon, after, bound])
}

pub(crate) fn format_type_reference<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &TypeReference<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        format_required_field(ty.r#type(), doc, |ty, doc| format_type(doc, &ty))
    })
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
        TypeSyntax::BogusType(ty) => crate::helpers::recovery::format_malformed(ty, doc),
    }
}

fn format_user_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &UserType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let parts = match resolve_required_field(ty.segments(), doc) {
            KotlinFormatField::Present(segments) => doc.concat_list(|docs| {
                for part in segments.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(role) => {
                            let formatted = format_user_type_segment(docs, role);
                            docs.push(formatted);
                        }
                        KotlinFormatListPart::Separator(separator) => {
                            docs.block_on_invariant(format!(
                                "unexpected user-type separator: {:?}",
                                separator.kind()
                            ));
                        }
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        doc.group(parts)
    })
}

fn format_user_type_segment<'source>(
    doc: &mut DocBuilder<'source>,
    role: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(annotation) = role.cast_node::<Annotation<'source>>() {
        let annotation = format_annotation(doc, &annotation);
        let space = doc.space();
        return doc.concat([annotation, space]);
    }
    if let Some(name) = role.cast_node::<jolt_kotlin_syntax::Name<'source>>() {
        return format_name(doc, &name);
    }
    if let Some(arguments) = role.cast_node::<TypeArgumentList<'source>>() {
        return format_type_argument_list(doc, &arguments);
    }
    if let Some(token) = role.token() {
        return format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
    }
    doc.block_on_invariant("invalid user-type segment");
    Doc::nil()
}

pub(crate) fn format_type_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &TypeArgumentList<'source>,
) -> Doc<'source> {
    format_or_verbatim(arguments, doc, |doc| {
        let open = resolve_required_delimiter(arguments.open_angle(), doc);
        let close = resolve_required_delimiter(arguments.close_angle(), doc);
        let mut has_recovery = false;
        let items = match resolve_required_field(arguments.projections(), doc) {
            KotlinFormatField::Present(projections) => {
                match resolve_required_field(projections.entries(), doc) {
                    KotlinFormatField::Present(entries) => {
                        let mut items = Vec::new();
                        for part in entries.parts() {
                            match resolve_list_part(part, doc) {
                                KotlinFormatListPart::Item(role) => items.push(CommaListItem {
                                    doc: format_type_argument_role(doc, role),
                                    comma: None,
                                }),
                                KotlinFormatListPart::Separator(comma) => {
                                    if let Some(item) = items.last_mut() {
                                        item.comma = Some(comma);
                                    }
                                }
                                KotlinFormatListPart::Malformed(recovery) => {
                                    has_recovery = true;
                                    items.push(CommaListItem {
                                        doc: recovery,
                                        comma: None,
                                    });
                                }
                            }
                        }
                        items
                    }
                    KotlinFormatField::Malformed(recovery) => {
                        has_recovery = true;
                        malformed_item(recovery)
                    }
                }
            }
            KotlinFormatField::Malformed(recovery) => {
                has_recovery = true;
                malformed_item(recovery)
            }
        };
        let expanded = has_recovery || items.last().is_some_and(|item| item.comma.is_some());
        format_angle_delimiters(doc, &open, &close, items, !expanded)
    })
}

fn format_type_argument_role<'source>(
    doc: &mut DocBuilder<'source>,
    role: jolt_kotlin_syntax::TypeArgumentListEntry<'source>,
) -> Doc<'source> {
    match role {
        jolt_kotlin_syntax::TypeArgumentListEntry::TypeArgument(argument) => {
            format_type_argument(doc, &argument)
        }
        jolt_kotlin_syntax::TypeArgumentListEntry::TypeProjection(projection) => {
            format_type_projection(doc, &projection)
        }
        jolt_kotlin_syntax::TypeArgumentListEntry::BogusTypeArgument(bogus) => {
            crate::helpers::recovery::format_malformed(&bogus, doc)
        }
    }
}

fn format_type_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &TypeArgument<'source>,
) -> Doc<'source> {
    format_or_verbatim(argument, doc, |doc| {
        format_required_field(argument.projection(), doc, |role, doc| {
            if let Some(projection) = role.cast_node::<TypeProjection<'source>>() {
                format_type_projection(doc, &projection)
            } else if let Some(ty) = role.cast_node::<TypeReference<'source>>() {
                format_type_reference(doc, &ty)
            } else {
                doc.block_on_invariant("invalid type-argument projection");
                Doc::nil()
            }
        })
    })
}

fn format_type_projection<'source>(
    doc: &mut DocBuilder<'source>,
    projection: &TypeProjection<'source>,
) -> Doc<'source> {
    format_or_verbatim(projection, doc, |doc| {
        let variance = format_optional_field(projection.variance(), doc, |role, doc| {
            let variance =
                format_role_token(doc, role, TrailingTrivia::RelocatedToEnclosingContext);
            let space = doc.space();
            doc.concat([variance, space])
        });
        let star = format_optional_field(projection.star(), doc, |star, doc| {
            format_token(
                doc,
                &star,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        });
        let ty = format_optional_field(projection.r#type(), doc, |ty, doc| {
            format_type_reference(doc, &ty)
        });
        doc.concat([variance, star, ty])
    })
}

fn format_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &NullableType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let inner = format_required_field(ty.inner(), doc, |inner, doc| format_type(doc, &inner));
        let question = format_required_field(ty.question(), doc, |question, doc| {
            format_token(
                doc,
                &question,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        });
        doc.concat([inner, question])
    })
}

fn format_parenthesized_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ParenthesizedType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let annotations = format_required_field(ty.annotations(), doc, |annotations, doc| {
            doc.concat_list(|docs| {
                for part in annotations.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(annotation) => {
                            let annotation = format_annotation(docs, &annotation);
                            docs.push(annotation);
                            let space = docs.space();
                            docs.push(space);
                        }
                        KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(
                            format!("unexpected annotation separator: {:?}", separator.kind()),
                        ),
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            })
        });
        let open = resolve_required_delimiter(ty.open_paren(), doc);
        let close = resolve_required_delimiter(ty.close_paren(), doc);
        let items = match resolve_required_field(ty.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, role| CommaListItem {
                    doc: format_parenthesized_type_entry(doc, role),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
        };
        let list = format_parenthesized_delimiters(doc, &open, &close, items);
        doc.concat([annotations, list])
    })
}

fn format_parenthesized_type_entry<'source>(
    doc: &mut DocBuilder<'source>,
    role: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(parameter) = role.cast_node::<FunctionTypeParameter<'source>>() {
        format_function_type_parameter(doc, &parameter)
    } else if let Some(ty) = role.cast_node::<TypeReference<'source>>() {
        format_type_reference(doc, &ty)
    } else {
        doc.block_on_invariant("invalid parenthesized-type entry");
        Doc::nil()
    }
}

fn format_function_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &FunctionTypeParameter<'source>,
) -> Doc<'source> {
    format_or_verbatim(parameter, doc, |doc| {
        let name =
            format_optional_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
        let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
            let colon = format_token(
                doc,
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            );
            let space = doc.space();
            doc.concat([colon, space])
        });
        let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
            format_type_reference(doc, &ty)
        });
        doc.concat([name, colon, ty])
    })
}

fn format_receiver_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ReceiverType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let receiver = format_required_field(ty.receiver(), doc, |receiver, doc| {
            format_type(doc, &receiver)
        });
        let dot = format_required_field(ty.dot(), doc, |dot, doc| {
            format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        });
        let parameter = format_required_field(ty.parameter(), doc, |parameter, doc| {
            format_type(doc, &parameter)
        });
        doc.concat([receiver, dot, parameter])
    })
}

fn format_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &FunctionType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        format_required_field(ty.form(), doc, |form, doc| {
            if let Some(suspended) = form.cast_node::<SuspendedFunctionType<'source>>() {
                format_suspended_function_type(doc, &suspended)
            } else if let Some(arrow) = form.cast_node::<ArrowFunctionType<'source>>() {
                format_arrow_function_type(doc, &arrow)
            } else {
                doc.block_on_invariant("invalid function-type form");
                Doc::nil()
            }
        })
    })
}

fn format_suspended_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &SuspendedFunctionType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let suspend = format_required_field(ty.suspend_token(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });
        let nested = format_required_field(ty.function_type(), doc, |nested, doc| {
            format_function_type(doc, &nested)
        });
        let space = doc.space();
        doc.concat([suspend, space, nested])
    })
}

fn format_arrow_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ArrowFunctionType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let parameter = format_required_field(ty.parameter_type(), doc, |parameter, doc| {
            format_type(doc, &parameter)
        });
        let arrow = format_required_field(ty.arrow(), doc, |arrow, doc| {
            format_token(
                doc,
                &arrow,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        });
        let return_type = format_required_field(ty.return_type(), doc, |return_type, doc| {
            format_type(doc, &return_type)
        });
        let before = doc.space();
        let after = doc.space();
        doc.concat([parameter, before, arrow, after, return_type])
    })
}

fn format_context_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ContextFunctionType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let context = format_required_field(ty.context_token(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });
        let open = resolve_required_delimiter(ty.open_paren(), doc);
        let close = resolve_required_delimiter(ty.close_paren(), doc);
        let items = match resolve_required_field(ty.context_parameters(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, role| CommaListItem {
                    doc: format_parenthesized_type_entry(doc, role),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
        };
        let parameters = format_parenthesized_delimiters(doc, &open, &close, items);
        let function = format_required_field(ty.function_type(), doc, |function, doc| {
            format_function_type(doc, &function)
        });
        let space = doc.space();
        doc.concat([context, parameters, space, function])
    })
}

fn format_definitely_non_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &DefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        format_required_field(ty.form(), doc, |form, doc| {
            if let Some(intersection) =
                form.cast_node::<IntersectionDefinitelyNonNullableType<'source>>()
            {
                format_intersection_dnn(doc, &intersection)
            } else if let Some(bang) = form.cast_node::<BangDefinitelyNonNullableType<'source>>() {
                format_bang_dnn(doc, &bang)
            } else {
                doc.block_on_invariant("invalid definitely-non-null type form");
                Doc::nil()
            }
        })
    })
}

fn format_intersection_dnn<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &IntersectionDefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let left = format_required_field(ty.left(), doc, |left, doc| format_type(doc, &left));
        let amp = format_required_field(ty.amp(), doc, |amp, doc| {
            format_token(
                doc,
                &amp,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });
        let right = format_required_field(ty.right(), doc, |right, doc| format_type(doc, &right));
        let before = doc.space();
        let after = doc.space();
        doc.concat([left, before, amp, after, right])
    })
}

fn format_bang_dnn<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &BangDefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    format_or_verbatim(ty, doc, |doc| {
        let inner = format_required_field(ty.inner(), doc, |inner, doc| format_type(doc, &inner));
        let bang = format_required_field(ty.bang_bang(), doc, |bang, doc| {
            format_token(
                doc,
                &bang,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        });
        doc.concat([inner, bang])
    })
}

pub(crate) fn format_modifier_sequence<'source>(
    doc: &mut DocBuilder<'source>,
    sequence: &ModifierListSequence<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in sequence.parts() {
            match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(modifiers) => {
                    let formatted = format_modifier_list(docs, &modifiers);
                    docs.push(formatted);
                }
                KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(format!(
                    "unexpected modifier-list separator: {:?}",
                    separator.kind()
                )),
                KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
            }
        }
    })
}

fn format_modifier_list<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: &ModifierList<'source>,
) -> Doc<'source> {
    format_or_verbatim(modifiers, doc, |doc| {
        match resolve_required_field(modifiers.modifiers(), doc) {
            KotlinFormatField::Present(items) => doc.concat_list(|docs| {
                for part in items.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(role) => {
                            if let Some(annotation) = role.cast_node::<Annotation<'source>>() {
                                let annotation = format_annotation(docs, &annotation);
                                docs.push(annotation);
                                let line = docs.hard_line();
                                docs.push(line);
                            } else if let Some(token) = role.token() {
                                let token = format_token(
                                    docs,
                                    &token,
                                    LeadingTrivia::Preserve,
                                    TrailingTrivia::Preserve,
                                );
                                docs.push(token);
                                let space = docs.space();
                                docs.push(space);
                            } else {
                                docs.block_on_invariant("invalid modifier-list item");
                            }
                        }
                        KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(
                            format!("unexpected modifier separator: {:?}", separator.kind()),
                        ),
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        }
    })
}

fn format_role_token<'source>(
    doc: &mut DocBuilder<'source>,
    role: KotlinRoleElement<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let Some(token) = role.token() else {
        doc.block_on_invariant("expected token role");
        return Doc::nil();
    };
    format_token(doc, &token, LeadingTrivia::Preserve, trailing)
}

fn format_constraint_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return doc.nil();
    };
    let first_doc = first.doc;
    let mut previous_comma = first.comma;
    let rest = doc.concat_list(|rest| {
        for entry in items {
            let separator = if let Some(comma) = previous_comma.as_ref() {
                let line = rest.line();
                format_separator_with_comments(rest, comma, line)
            } else {
                rest.line()
            };
            rest.push(separator);
            rest.push(entry.doc);
            previous_comma = entry.comma;
        }
    });
    let rest = doc.indent(rest);
    doc.concat([first_doc, rest])
}

fn malformed_item(recovery: Doc<'_>) -> Vec<CommaListItem<'_>> {
    vec![CommaListItem {
        doc: recovery,
        comma: None,
    }]
}

fn delimiter_recovery<'source>(delimiter: &KotlinFormatDelimiter<'source>) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(recovery) => *recovery,
    }
}

fn format_angle_delimiters<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    close: &KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
    compact: bool,
) -> Doc<'source> {
    let open_recovery = delimiter_recovery(open);
    let close_recovery = delimiter_recovery(close);
    let list = if compact {
        compact_angle_bracket_list(doc, open.source(), close.source(), items)
    } else {
        angle_bracket_list(doc, open.source(), close.source(), items)
    };
    doc.concat([open_recovery, list, close_recovery])
}

fn format_parenthesized_delimiters<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    close: &KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let open_recovery = delimiter_recovery(open);
    let close_recovery = delimiter_recovery(close);
    let list = parenthesized_list(doc, open.source(), close.source(), items);
    doc.concat([open_recovery, list, close_recovery])
}
