use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ClassBody, ClassDeclaration, CompanionObject, DelegationClause, DelegationSpecifier,
    DelegationSpecifierEntry, InterfaceDeclaration, KotlinSyntaxField, KotlinSyntaxListPart,
    KotlinSyntaxToken, KotlinSyntaxView, ObjectDeclaration, ObjectExpression, PrimaryConstructor,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, comma_list, push_recovery_item};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_optional_field, format_required_field,
    resolve_list_part, resolve_required_field,
};
use crate::rules::expressions::{format_expression, format_value_argument_list};
use crate::rules::names::format_name;
use crate::rules::types::{
    format_type_constraint_list, format_type_parameter_list, format_type_reference,
};
use crate::rules::variables::format_value_parameter_list;

use super::{
    format_declaration_prefix, format_inline_modifier_prefix, member_bodies::format_class_body,
};

pub(super) fn format_class_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &ClassDeclaration<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let keyword = format_keyword(doc, declaration.class_token(), true);
    let name = format_required_field(declaration.name(), doc, |name, doc| format_name(doc, &name));
    let type_parameters =
        format_optional_field(declaration.type_parameters(), doc, |parameters, doc| {
            format_type_parameter_list(doc, Some(parameters))
        });
    let constructor = format_optional_field(
        declaration.primary_constructor(),
        doc,
        |constructor, doc| format_primary_constructor(doc, &constructor),
    );
    let tail = format_type_tail(
        doc,
        declaration.delegation(),
        Some(declaration.constraints()),
        declaration.body(),
    );
    let declaration = doc.concat([prefix, keyword, name, type_parameters, constructor, tail]);
    doc.group(declaration)
}

pub(super) fn format_interface_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &InterfaceDeclaration<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let fun = format_optional_field(declaration.fun_token(), doc, |fun, doc| {
        format_keyword(doc, Ok(KotlinSyntaxField::Present(fun)), true)
    });
    let keyword = format_keyword(doc, declaration.interface_token(), true);
    let name = format_required_field(declaration.name(), doc, |name, doc| format_name(doc, &name));
    let type_parameters =
        format_optional_field(declaration.type_parameters(), doc, |parameters, doc| {
            format_type_parameter_list(doc, Some(parameters))
        });
    let tail = format_type_tail(
        doc,
        declaration.delegation(),
        Some(declaration.constraints()),
        declaration.body(),
    );
    let declaration = doc.concat([prefix, fun, keyword, name, type_parameters, tail]);
    doc.group(declaration)
}

pub(super) fn format_object_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &ObjectDeclaration<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let keyword = format_keyword(doc, declaration.object_token(), false);
    let name = format_optional_field(declaration.name(), doc, |name, doc| {
        let space = doc.space();
        let name = format_name(doc, &name);
        doc.concat([space, name])
    });
    let tail = format_type_tail(doc, declaration.delegation(), None, declaration.body());
    let declaration = doc.concat([prefix, keyword, name, tail]);
    doc.group(declaration)
}

pub(super) fn format_companion_object<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &CompanionObject<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let companion = format_keyword(doc, declaration.companion_token(), true);
    let object = format_optional_field(declaration.object_token(), doc, |token, doc| {
        format_keyword_token(doc, token)
    });
    let name = format_optional_field(declaration.name(), doc, |name, doc| {
        let space = doc.space();
        let name = format_name(doc, &name);
        doc.concat([space, name])
    });
    let tail = format_type_tail(doc, declaration.delegation(), None, declaration.body());
    let declaration = doc.concat([prefix, companion, object, name, tail]);
    doc.group(declaration)
}

pub(crate) fn format_object_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ObjectExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = format_required_field(expression.object_token(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let delegation = format_optional_field(expression.delegation(), doc, |delegation, doc| {
        let delegation = format_delegation_clause(doc, &delegation);
        doc.group(delegation)
    });
    let body = format_required_field(expression.body(), doc, |body, doc| {
        if body.first_token().is_none() {
            Doc::nil()
        } else {
            format_class_body(doc, Some(body))
        }
    });
    let expression = doc.concat([keyword, delegation, body]);
    doc.group(expression)
}

fn format_type_tail<'source>(
    doc: &mut DocBuilder<'source>,
    delegation: Result<
        KotlinSyntaxField<'source, DelegationClause<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    constraints: Option<
        Result<
            KotlinSyntaxField<'source, jolt_kotlin_syntax::TypeConstraintList<'source>>,
            jolt_kotlin_syntax::KotlinSyntaxInvariantError,
        >,
    >,
    body: Result<
        KotlinSyntaxField<'source, ClassBody<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let delegation = format_optional_field(delegation, doc, |delegation, doc| {
        format_delegation_clause(doc, &delegation)
    });
    let constraints = constraints.map_or_else(Doc::nil, |constraints| {
        format_optional_field(constraints, doc, |constraints, doc| {
            format_type_constraint_list(doc, Some(constraints))
        })
    });
    let body = format_optional_field(body, doc, |body, doc| format_class_body(doc, Some(body)));
    doc.concat([delegation, constraints, body])
}

fn format_delegation_clause<'source>(
    doc: &mut DocBuilder<'source>,
    delegation: &DelegationClause<'source>,
) -> Doc<'source> {
    let has_colon = matches!(delegation.colon(), Ok(KotlinSyntaxField::Present(_)));
    let has_specifiers = matches!(
        delegation.specifiers(),
        Ok(KotlinSyntaxField::Present(ref specifiers)) if specifiers.first_token().is_some()
    ) || matches!(
        delegation.specifiers(),
        Ok(KotlinSyntaxField::Malformed(ref malformed)) if malformed.first_token().is_some()
    );
    let colon = format_required_field(delegation.colon(), doc, |colon, doc| {
        format_keyword_token(doc, colon)
    });
    let specifiers = match resolve_required_field(delegation.specifiers(), doc) {
        KotlinFormatField::Present(specifiers) => {
            let items = physical_delegation_items(doc, specifiers.parts());
            comma_list(doc, items)
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let line = doc.line();
    let between = if has_colon && has_specifiers {
        doc.line()
    } else {
        Doc::nil()
    };
    let specifiers = doc.concat([between, specifiers]);
    let specifiers = doc.group(specifiers);
    let specifiers = doc.indent(specifiers);
    let tail = doc.concat([line, colon, specifiers]);
    doc.indent(tail)
}

fn physical_delegation_items<'source>(
    doc: &mut DocBuilder<'source>,
    parts: impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, DelegationSpecifierEntry<'source>>,
            jolt_kotlin_syntax::KotlinSyntaxInvariantError,
        >,
    >,
) -> Vec<CommaListItem<'source>> {
    let mut items = Vec::new();
    for part in parts {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(specifier) => {
                let specifier = match specifier {
                    DelegationSpecifierEntry::DelegationSpecifier(specifier) => {
                        format_delegation_specifier(doc, &specifier)
                    }
                    DelegationSpecifierEntry::BogusDelegationSpecifier(bogus) => {
                        crate::helpers::recovery::format_malformed(&bogus, doc)
                    }
                };
                items.push(CommaListItem {
                    doc: specifier,
                    comma: None,
                    layout_visible: true,
                });
            }
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items
                    .iter_mut()
                    .rev()
                    .find(|item| item.layout_visible && item.comma.is_none())
                {
                    item.comma = Some(comma);
                } else {
                    items.push(CommaListItem {
                        doc: Doc::nil(),
                        comma: Some(comma),
                        layout_visible: true,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => items.push(CommaListItem {
                doc: recovery,
                comma: None,
                layout_visible: true,
            }),
            KotlinFormatListPart::Invisible(recovery) => {
                push_recovery_item(&mut items, recovery, false);
            }
        }
    }
    items
}

fn format_delegation_specifier<'source>(
    doc: &mut DocBuilder<'source>,
    specifier: &DelegationSpecifier<'source>,
) -> Doc<'source> {
    let ty = format_required_field(specifier.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    let arguments = format_optional_field(specifier.arguments(), doc, |arguments, doc| {
        format_value_argument_list(doc, &arguments)
    });
    let by = format_optional_field(specifier.by_clause(), doc, |by, doc| {
        let has_keyword = matches!(by.by_token(), Ok(KotlinSyntaxField::Present(_)));
        let has_delegate = matches!(
            by.delegate(),
            Ok(KotlinSyntaxField::Present(ref delegate)) if delegate.first_token().is_some()
        ) || matches!(
            by.delegate(),
            Ok(KotlinSyntaxField::Malformed(ref malformed)) if malformed.first_token().is_some()
        );
        let before = if has_keyword || has_delegate {
            doc.space()
        } else {
            Doc::nil()
        };
        let keyword = format_required_field(by.by_token(), doc, |by, doc| {
            format_token(doc, &by, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        });
        let delegate = format_required_field(by.delegate(), doc, |delegate, doc| {
            let space = if has_keyword && has_delegate {
                doc.space()
            } else {
                Doc::nil()
            };
            let delegate = format_expression(doc, &delegate);
            doc.concat([space, delegate])
        });
        doc.concat([before, keyword, delegate])
    });
    doc.concat([ty, arguments, by])
}

fn format_primary_constructor<'source>(
    doc: &mut DocBuilder<'source>,
    constructor: &PrimaryConstructor<'source>,
) -> Doc<'source> {
    let modifiers = format_inline_modifier_prefix(doc, constructor.modifiers());
    let keyword = format_optional_field(constructor.constructor_token(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let parameters = format_required_field(constructor.parameters(), doc, |parameters, doc| {
        format_value_parameter_list(doc, &parameters)
    });
    let has_prefix = constructor.modifiers().ok().is_some_and(|field| {
        matches!(field, KotlinSyntaxField::Present(modifiers) if modifiers.first_token().is_some())
    }) || matches!(
        constructor.constructor_token(),
        Ok(KotlinSyntaxField::Present(_))
    );
    if has_prefix {
        let space = doc.space();
        doc.concat([space, modifiers, keyword, parameters])
    } else {
        parameters
    }
}

fn format_keyword<'source>(
    doc: &mut DocBuilder<'source>,
    field: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    trailing_space: bool,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        let token = format_keyword_token(doc, token);
        if trailing_space {
            let space = doc.space();
            doc.concat([token, space])
        } else {
            token
        }
    })
}

fn format_keyword_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        &token,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}
