use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Annotation, AnnotationList, ArrowFunctionType, BangDefinitelyNonNullableType,
    ContextFunctionType, DefinitelyNonNullableType, DefinitelyNonNullableTypeForm, FunctionType,
    FunctionTypeForm, FunctionTypeParameter, FunctionTypeParameterListEntry,
    IntersectionDefinitelyNonNullableType, KotlinFamily, KotlinNode, KotlinRoleElement,
    KotlinSyntaxField, KotlinSyntaxNode, KotlinSyntaxToken, KotlinSyntaxView, ModifierList,
    NullableType, ParenthesizedType, ReceiverType, StarProjection, SuspendedFunctionType,
    TypeArgumentList, TypeArgumentListEntry, TypeConstraint, TypeConstraintList,
    TypeConstraintListEntry, TypeParameter, TypeParameterList, TypeParameterListEntry,
    TypeProjection, TypeReference, TypeSyntax, UserType, UserTypeSegment, UserTypeSegmentSyntax,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, delimited_comma_list, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_malformed, format_optional_field,
    format_required_field, join_delimited_recovery, resolve_list_part, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::names::format_name;

pub(crate) fn format_type_parameter_list<'source>(
    doc: &mut DocBuilder<'source>,
    parameters: TypeParameterList<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(parameters.open_angle(), doc);
    let close = resolve_required_delimiter(parameters.close_angle(), doc);
    let items = match resolve_required_field(parameters.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            physical_comma_list_items(doc, entries.parts(), |doc, parameter| {
                CommaListItem::visible(match parameter {
                    TypeParameterListEntry::TypeParameter(parameter) => {
                        format_type_parameter(doc, &parameter)
                    }
                    TypeParameterListEntry::BogusTypeParameter(bogus) => {
                        format_bogus_list_entry(doc, &bogus)
                    }
                })
            })
        }
        KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
    };
    let list = delimited_comma_list(doc, open.source(), close.source(), items);
    join_delimited_recovery(doc, &open, list, &close)
}

pub(crate) fn format_type_constraint_list<'source>(
    doc: &mut DocBuilder<'source>,
    constraints: TypeConstraintList<'source>,
) -> Doc<'source> {
    let has_where = matches!(constraints.where_token(), KotlinSyntaxField::Present(_));
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
            physical_comma_list_items(doc, entries.parts(), |doc, constraint| {
                CommaListItem::visible(match constraint {
                    TypeConstraintListEntry::TypeConstraint(constraint) => {
                        format_type_constraint(doc, &constraint)
                    }
                    TypeConstraintListEntry::BogusTypeConstraint(bogus) => {
                        format_bogus_list_entry(doc, &bogus)
                    }
                })
            })
        }
        KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
    };
    let constraints = format_indented_comma_items(doc, items);
    let line = doc.line();
    let space = if has_where { doc.space() } else { Doc::nil() };
    let constraints = doc.group(constraints);
    let contents = doc.concat([line, where_token, space, constraints]);
    doc.indent(contents)
}

fn format_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &TypeParameter<'source>,
) -> Doc<'source> {
    let modifiers = format_required_field(parameter.modifiers(), doc, |modifiers, doc| {
        format_modifier_sequence(doc, &modifiers)
    });
    let variance = format_optional_field(parameter.variance(), doc, |role, doc| {
        let token = format_role_token(doc, role, TrailingTrivia::RelocatedToEnclosingContext);
        let space = doc.space();
        doc.concat([token, space])
    });
    let name = format_required_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
    let bound = format_optional_type_bound(doc, parameter.colon(), parameter.bound());
    doc.concat([modifiers, variance, name, bound])
}

fn format_type_constraint<'source>(
    doc: &mut DocBuilder<'source>,
    constraint: &TypeConstraint<'source>,
) -> Doc<'source> {
    let name = format_required_field(constraint.name(), doc, |name, doc| format_name(doc, &name));
    let bound = format_required_type_bound(doc, constraint.colon(), constraint.bound());
    doc.concat([name, bound])
}

fn format_optional_type_bound<'source>(
    doc: &mut DocBuilder<'source>,
    colon: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
    bound: KotlinSyntaxField<'source, TypeReference<'source>>,
) -> Doc<'source> {
    let has_bound = matches!(
        bound,
        KotlinSyntaxField::Present(bound) if bound.first_token().is_some()
    );
    match crate::helpers::recovery::resolve_optional_field(colon, doc) {
        KotlinFormatField::Present(None) => format_optional_field(bound, doc, |bound, doc| {
            let bound = format_type_reference(doc, &bound);
            if has_bound {
                let space = doc.space();
                doc.concat([space, bound])
            } else {
                bound
            }
        }),
        KotlinFormatField::Present(Some(colon)) => {
            let before = doc.space();
            let colon = format_token(
                doc,
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let after = if has_bound { doc.space() } else { Doc::nil() };
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
    colon: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
    bound: KotlinSyntaxField<'source, TypeReference<'source>>,
) -> Doc<'source> {
    let has_bound = matches!(
        bound,
        KotlinSyntaxField::Present(bound) if bound.first_token().is_some()
    );
    let colon = format_required_field(colon, doc, |colon, doc| {
        let before = doc.space();
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        doc.concat([before, colon])
    });
    let bound = format_required_field(bound, doc, |bound, doc| {
        let bound = format_type_reference(doc, &bound);
        if has_bound {
            let before = doc.space();
            doc.concat([before, bound])
        } else {
            bound
        }
    });
    doc.concat([colon, bound])
}

pub(crate) fn format_type_reference<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &TypeReference<'source>,
) -> Doc<'source> {
    format_required_field(ty.r#type(), doc, |ty, doc| format_type(doc, &ty))
}

fn format_type<'source>(doc: &mut DocBuilder<'source>, ty: &TypeSyntax<'source>) -> Doc<'source> {
    let Some(outer) = ty.syntax_node() else {
        doc.block_on_invariant("Kotlin type has no syntax node");
        return Doc::nil();
    };
    let mut current = *ty;

    loop {
        let inner = match current {
            TypeSyntax::NullableType(nullable) => present_type(nullable.inner()),
            TypeSyntax::ReceiverType(receiver) => present_type(receiver.receiver()),
            TypeSyntax::DefinitelyNonNullableType(dnn) => match present_type(dnn.form()) {
                Some(DefinitelyNonNullableTypeForm::IntersectionDefinitelyNonNullableType(
                    intersection,
                )) => present_type(intersection.left()),
                Some(DefinitelyNonNullableTypeForm::BangDefinitelyNonNullableType(bang)) => {
                    present_type(bang.inner())
                }
                Some(DefinitelyNonNullableTypeForm::BogusDefinitelyNonNullableTypeForm(_))
                | None => None,
            },
            _ => None,
        };
        let Some(inner) = inner else {
            break;
        };
        current = inner;
    }

    let Some(mut current_node) = current.syntax_node() else {
        doc.block_on_invariant("Kotlin type base has no syntax node");
        return Doc::nil();
    };
    let mut formatted = format_type_base(doc, &current);
    while current_node != outer {
        let Some((suffix, parent)) = format_parent_type_suffix(doc, formatted, current_node) else {
            doc.block_on_invariant("type suffix spine crossed an unexpected parent");
            break;
        };
        formatted = suffix;
        current_node = parent;
    }
    formatted
}

fn format_type_base<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &TypeSyntax<'source>,
) -> Doc<'source> {
    match ty {
        TypeSyntax::UserType(ty) => format_user_type(doc, ty),
        TypeSyntax::NullableType(ty) => format_nullable_type(doc, ty, None),
        TypeSyntax::FunctionType(ty) => format_function_type(doc, ty),
        TypeSyntax::ContextFunctionType(ty) => format_context_function_type(doc, ty),
        TypeSyntax::ReceiverType(ty) => format_receiver_type(doc, ty, None),
        TypeSyntax::ParenthesizedType(ty) => format_parenthesized_type(doc, ty),
        TypeSyntax::DefinitelyNonNullableType(ty) => format_definitely_non_nullable_type(doc, ty),
        TypeSyntax::BogusType(ty) => crate::helpers::recovery::format_malformed(ty, doc),
    }
}

fn present_type<T>(field: KotlinSyntaxField<'_, T>) -> Option<T> {
    match field {
        KotlinSyntaxField::Present(value) => Some(value),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => None,
    }
}

fn format_parent_type_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    formatted: Doc<'source>,
    current: KotlinSyntaxNode<'source>,
) -> Option<(Doc<'source>, KotlinSyntaxNode<'source>)> {
    let parent_node = current.parent()?;
    if let Some(parent) = TypeSyntax::cast(parent_node) {
        let suffix = match parent {
            TypeSyntax::NullableType(nullable) => {
                format_nullable_type(doc, &nullable, Some(formatted))
            }
            TypeSyntax::ReceiverType(receiver) => {
                format_receiver_type(doc, &receiver, Some(formatted))
            }
            _ => return None,
        };
        return Some((suffix, parent_node));
    }

    let suffix = match DefinitelyNonNullableTypeForm::cast(parent_node)? {
        DefinitelyNonNullableTypeForm::IntersectionDefinitelyNonNullableType(intersection) => {
            format_intersection_dnn(doc, &intersection, Some(formatted))
        }
        DefinitelyNonNullableTypeForm::BangDefinitelyNonNullableType(bang) => {
            format_bang_dnn(doc, &bang, Some(formatted))
        }
        DefinitelyNonNullableTypeForm::BogusDefinitelyNonNullableTypeForm(_) => return None,
    };
    let container_node = parent_node.parent()?;
    DefinitelyNonNullableType::cast(container_node)?;
    Some((suffix, container_node))
}

fn format_user_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &UserType<'source>,
) -> Doc<'source> {
    let parts = match resolve_required_field(ty.segments(), doc) {
        KotlinFormatField::Present(segments) => doc.concat_list(|docs| {
            for part in segments.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(segment) => {
                        let segment = match segment {
                            UserTypeSegmentSyntax::UserTypeSegment(segment) => {
                                format_user_type_segment(docs, &segment)
                            }
                            UserTypeSegmentSyntax::BogusUserTypeSegment(bogus) => {
                                format_bogus_list_entry(docs, &bogus)
                            }
                        };
                        docs.push(segment);
                    }
                    KotlinFormatListPart::Separator(separator) => {
                        let separator = format_token(
                            docs,
                            &separator,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::Preserve,
                        );
                        docs.push(separator);
                    }
                    KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                }
            }
        }),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    doc.group(parts)
}

fn format_user_type_segment<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &UserTypeSegment<'source>,
) -> Doc<'source> {
    let annotations = format_optional_field(segment.annotations(), doc, format_type_annotations);
    let name = format_required_field(segment.name(), doc, |name, doc| {
        format_token(
            doc,
            &name,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let arguments = format_optional_field(segment.arguments(), doc, |arguments, doc| {
        format_type_argument_list(doc, &arguments)
    });
    doc.concat([annotations, name, arguments])
}

pub(crate) fn format_type_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &TypeArgumentList<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_angle(), doc);
    let close = resolve_required_delimiter(arguments.close_angle(), doc);
    let items = match resolve_required_field(arguments.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            physical_comma_list_items(doc, entries.parts(), |doc, argument| {
                CommaListItem::visible(format_type_argument(doc, &argument))
            })
        }
        KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
    };
    let list = delimited_comma_list(doc, open.source(), close.source(), items);
    join_delimited_recovery(doc, &open, list, &close)
}

fn format_type_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &TypeArgumentListEntry<'source>,
) -> Doc<'source> {
    match argument {
        TypeArgumentListEntry::TypeReference(ty) => format_type_reference(doc, ty),
        TypeArgumentListEntry::TypeProjection(projection) => {
            format_type_projection(doc, projection)
        }
        TypeArgumentListEntry::StarProjection(projection) => {
            format_star_projection(doc, projection)
        }
        TypeArgumentListEntry::BogusTypeArgument(bogus) => format_bogus_list_entry(doc, bogus),
    }
}

fn format_type_projection<'source>(
    doc: &mut DocBuilder<'source>,
    projection: &TypeProjection<'source>,
) -> Doc<'source> {
    let has_type = matches!(
        projection.r#type(),
        KotlinSyntaxField::Present(ty) if ty.first_token().is_some()
    );
    let variance = format_required_field(projection.variance(), doc, |role, doc| {
        let variance = format_role_token(doc, role, TrailingTrivia::RelocatedToEnclosingContext);
        if has_type {
            let space = doc.space();
            doc.concat([variance, space])
        } else {
            variance
        }
    });
    let ty = format_required_field(projection.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    doc.concat([variance, ty])
}

fn format_star_projection<'source>(
    doc: &mut DocBuilder<'source>,
    projection: &StarProjection<'source>,
) -> Doc<'source> {
    format_required_field(projection.star(), doc, |star, doc| {
        format_token(
            doc,
            &star,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    })
}

fn format_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &NullableType<'source>,
    inner: Option<Doc<'source>>,
) -> Doc<'source> {
    let inner = inner.unwrap_or_else(|| {
        format_required_field(ty.inner(), doc, |inner, doc| format_type(doc, &inner))
    });
    let question = format_required_field(ty.question(), doc, |question, doc| {
        format_token(
            doc,
            &question,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([inner, question])
}

fn format_parenthesized_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ParenthesizedType<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(ty.annotations(), doc, format_type_annotations);
    let open = resolve_required_delimiter(ty.open_paren(), doc);
    let close = resolve_required_delimiter(ty.close_paren(), doc);
    let items = match resolve_required_field(ty.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            physical_comma_list_items(doc, entries.parts(), |doc, entry| {
                CommaListItem::visible(format_function_type_parameter_entry(doc, &entry))
            })
        }
        KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
    };
    let list = delimited_comma_list(doc, open.source(), close.source(), items);
    let list = join_delimited_recovery(doc, &open, list, &close);
    doc.concat([annotations, list])
}

fn format_function_type_parameter_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &FunctionTypeParameterListEntry<'source>,
) -> Doc<'source> {
    match entry {
        FunctionTypeParameterListEntry::FunctionTypeParameter(parameter) => {
            format_function_type_parameter(doc, parameter)
        }
        FunctionTypeParameterListEntry::BogusFunctionTypeParameter(bogus) => {
            format_bogus_list_entry(doc, bogus)
        }
    }
}

fn format_function_type_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &FunctionTypeParameter<'source>,
) -> Doc<'source> {
    let has_type = matches!(
        parameter.r#type(),
        KotlinSyntaxField::Present(ty) if ty.first_token().is_some()
    );
    let name = format_optional_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
    let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        if has_type {
            let space = doc.space();
            doc.concat([colon, space])
        } else {
            colon
        }
    });
    let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    doc.concat([name, colon, ty])
}

fn format_receiver_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ReceiverType<'source>,
    receiver: Option<Doc<'source>>,
) -> Doc<'source> {
    let receiver = receiver.unwrap_or_else(|| {
        format_required_field(ty.receiver(), doc, |receiver, doc| {
            format_type(doc, &receiver)
        })
    });
    let dot = format_required_field(ty.dot(), doc, |dot, doc| {
        format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    });
    let parameter = format_required_field(ty.parameter(), doc, |parameter, doc| {
        format_type(doc, &parameter)
    });
    doc.concat([receiver, dot, parameter])
}

fn format_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &FunctionType<'source>,
) -> Doc<'source> {
    let annotations = format_optional_field(ty.annotations(), doc, format_type_annotations);
    let form = format_required_field(ty.form(), doc, |form, doc| match form {
        FunctionTypeForm::SuspendedFunctionType(suspended) => {
            format_suspended_function_type(doc, &suspended)
        }
        FunctionTypeForm::ArrowFunctionType(arrow) => format_arrow_function_type(doc, &arrow),
        FunctionTypeForm::BogusFunctionTypeForm(bogus) => format_malformed(&bogus, doc),
    });
    doc.concat([annotations, form])
}

fn format_suspended_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &SuspendedFunctionType<'source>,
) -> Doc<'source> {
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
}

fn format_arrow_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ArrowFunctionType<'source>,
) -> Doc<'source> {
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
}

fn format_context_function_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &ContextFunctionType<'source>,
) -> Doc<'source> {
    let annotations = format_optional_field(ty.annotations(), doc, format_type_annotations);
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
            physical_comma_list_items(doc, entries.parts(), |doc, entry| {
                CommaListItem::visible(format_function_type_parameter_entry(doc, &entry))
            })
        }
        KotlinFormatField::Malformed(recovery) => malformed_item(recovery),
    };
    let parameters = delimited_comma_list(doc, open.source(), close.source(), items);
    let parameters = join_delimited_recovery(doc, &open, parameters, &close);
    let function = format_required_field(ty.function_type(), doc, |function, doc| {
        format_function_type(doc, &function)
    });
    let space = doc.space();
    doc.concat([annotations, context, parameters, space, function])
}

fn format_type_annotations<'source>(
    annotations: AnnotationList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in annotations.parts() {
            match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(annotation) => {
                    let annotation = format_annotation(docs, &annotation);
                    docs.push(annotation);
                    let space = docs.space();
                    docs.push(space);
                }
                KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(format!(
                    "unexpected annotation separator: {:?}",
                    separator.kind()
                )),
                KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
            }
        }
    })
}

fn format_definitely_non_nullable_type<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &DefinitelyNonNullableType<'source>,
) -> Doc<'source> {
    format_required_field(ty.form(), doc, |form, doc| match form {
        DefinitelyNonNullableTypeForm::IntersectionDefinitelyNonNullableType(intersection) => {
            format_intersection_dnn(doc, &intersection, None)
        }
        DefinitelyNonNullableTypeForm::BangDefinitelyNonNullableType(bang) => {
            format_bang_dnn(doc, &bang, None)
        }
        DefinitelyNonNullableTypeForm::BogusDefinitelyNonNullableTypeForm(bogus) => {
            format_malformed(&bogus, doc)
        }
    })
}

fn format_intersection_dnn<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &IntersectionDefinitelyNonNullableType<'source>,
    left: Option<Doc<'source>>,
) -> Doc<'source> {
    let left = left.unwrap_or_else(|| {
        format_required_field(ty.left(), doc, |left, doc| format_type(doc, &left))
    });
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
}

fn format_bang_dnn<'source>(
    doc: &mut DocBuilder<'source>,
    ty: &BangDefinitelyNonNullableType<'source>,
    inner: Option<Doc<'source>>,
) -> Doc<'source> {
    let inner = inner.unwrap_or_else(|| {
        format_required_field(ty.inner(), doc, |inner, doc| format_type(doc, &inner))
    });
    let bang = format_required_field(ty.bang_bang(), doc, |bang, doc| {
        format_token(
            doc,
            &bang,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([inner, bang])
}

pub(crate) fn format_modifier_sequence<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: &ModifierList<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in modifiers.parts() {
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
                KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(format!(
                    "unexpected modifier separator: {:?}",
                    separator.kind()
                )),
                KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
            }
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

fn malformed_item(recovery: Doc<'_>) -> Vec<CommaListItem<'_>> {
    vec![CommaListItem::visible(recovery)]
}

pub(crate) fn format_bogus_list_entry<'source>(
    doc: &mut DocBuilder<'source>,
    bogus: &impl KotlinSyntaxView<'source>,
) -> Doc<'source> {
    format_malformed(bogus, doc)
}

fn format_indented_comma_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return doc.nil();
    };
    let first_doc = first.doc();
    let mut previous_comma = first.comma;
    let rest = doc.concat_list(|rest| {
        for entry in items {
            if let Some(comma) = previous_comma.take() {
                let line = rest.line();
                let separator =
                    crate::helpers::comments::format_separator_with_comments(rest, &comma, line);
                rest.push(separator);
            } else {
                let line = rest.line();
                rest.push(line);
            }
            rest.push(entry.doc());
            previous_comma = entry.comma;
        }
    });
    let rest = doc.indent(rest);
    let trailing_comma = previous_comma.map_or_else(Doc::nil, |comma| {
        crate::helpers::comments::format_separator_with_comments(doc, &comma, Doc::nil())
    });
    doc.concat([first_doc, rest, trailing_comma])
}
