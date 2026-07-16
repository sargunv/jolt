use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CallableDeclarationName, CallableName, ContextParameter, ContextParameterClause,
    ContextParameterListEntry, Declaration, DeclarationBody, DestructuringDeclaration,
    DestructuringEntry, EnumEntry, ExplicitBackingField, ExpressionBody, FunctionDeclaration,
    InitializerBlock, KotlinFileItem, KotlinNode, KotlinRoleElement, KotlinSyntaxField,
    KotlinSyntaxListPart, KotlinSyntaxToken, KotlinSyntaxView, ModifierList, PropertyAccessor,
    PropertyBinding, PropertyBodyMember, PropertyDeclaration, PropertyInitializer,
    SecondaryConstructor, TypeAliasDeclaration, TypeReference,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_removed_separator, format_token,
    trailing_comments_force_line,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, physical_comma_list_items, push_recovery_item,
};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_malformed, format_optional_field,
    format_required_field, join_delimited_recovery, resolve_list_part, resolve_optional_field,
    resolve_required_delimiter, resolve_required_field,
};
use crate::rules::annotations::format_annotation_with_leading;
use crate::rules::expressions::{format_expression, format_value_argument_list};
use crate::rules::names::format_name;
use crate::rules::statements::format_block;
use crate::rules::types::{
    format_bogus_list_entry, format_type_constraint_list, format_type_parameter_list,
    format_type_reference,
};
use crate::rules::variables::format_value_parameter_list;

mod member_bodies;
mod type_declarations;

pub(crate) use type_declarations::format_object_expression;

use type_declarations::{
    format_class_declaration, format_companion_object, format_interface_declaration,
    format_object_declaration,
};

pub(crate) fn format_file_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &KotlinFileItem<'source>,
) -> Doc<'source> {
    match item {
        KotlinFileItem::PackageHeader(_) | KotlinFileItem::ImportDirectiveList(_) => Doc::nil(),
        KotlinFileItem::ClassDeclaration(node) => format_class_declaration(doc, node),
        KotlinFileItem::InterfaceDeclaration(node) => format_interface_declaration(doc, node),
        KotlinFileItem::ObjectDeclaration(node) => format_object_declaration(doc, node),
        KotlinFileItem::CompanionObject(node) => format_companion_object(doc, node),
        KotlinFileItem::EnumEntry(node) => format_enum_entry(doc, node),
        KotlinFileItem::FunctionDeclaration(node) => format_function_declaration(doc, node),
        KotlinFileItem::PropertyDeclaration(node) => format_property_declaration(doc, node),
        KotlinFileItem::TypeAliasDeclaration(node) => format_type_alias_declaration(doc, node),
        KotlinFileItem::SecondaryConstructor(node) => format_secondary_constructor(doc, node),
        KotlinFileItem::InitializerBlock(node) => format_initializer_block(doc, node),
        KotlinFileItem::Statement(statement) => crate::rules::statements::format_statement_syntax(
            doc,
            &jolt_kotlin_syntax::StatementSyntax::Statement(*statement),
        ),
        KotlinFileItem::BogusKotlinFileItem(malformed) => format_malformed(malformed, doc),
    }
}

pub(crate) fn format_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &Declaration<'source>,
) -> Doc<'source> {
    match declaration {
        Declaration::ClassDeclaration(node) => format_class_declaration(doc, node),
        Declaration::InterfaceDeclaration(node) => format_interface_declaration(doc, node),
        Declaration::ObjectDeclaration(node) => format_object_declaration(doc, node),
        Declaration::CompanionObject(node) => format_companion_object(doc, node),
        Declaration::EnumEntry(node) => format_enum_entry(doc, node),
        Declaration::FunctionDeclaration(node) => format_function_declaration(doc, node),
        Declaration::PropertyDeclaration(node) => format_property_declaration(doc, node),
        Declaration::TypeAliasDeclaration(node) => format_type_alias_declaration(doc, node),
        Declaration::SecondaryConstructor(node) => format_secondary_constructor(doc, node),
        Declaration::InitializerBlock(node) => format_initializer_block(doc, node),
        Declaration::BogusDeclaration(malformed) => format_malformed(malformed, doc),
    }
}

fn format_enum_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &EnumEntry<'source>,
) -> Doc<'source> {
    let modifiers = format_modifier_prefix(doc, entry.modifiers());
    let name = format_required_field(entry.name(), doc, |name, doc| format_name(doc, &name));
    let arguments = format_optional_field(entry.arguments(), doc, |arguments, doc| {
        format_value_argument_list(doc, &arguments)
    });
    let body = format_optional_field(entry.body(), doc, |body, doc| {
        member_bodies::format_class_body(doc, Some(body))
    });
    let comma = format_optional_field(entry.comma(), doc, |comma, doc| {
        format_token(
            doc,
            &comma,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([modifiers, name, arguments, body, comma])
}

pub(super) fn format_initializer_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &InitializerBlock<'source>,
) -> Doc<'source> {
    let keyword = format_required_field(block.init_token(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let body = format_required_field(block.block(), doc, |body, doc| {
        let space = doc.space();
        let body = format_block(doc, &body);
        doc.concat([space, body])
    });
    doc.concat([keyword, body])
}

pub(super) fn format_function_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &FunctionDeclaration<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let keyword = keyword_with_space(doc, declaration.fun_token());
    let has_type_parameters = matches!(
        declaration.type_parameters(),
        Ok(KotlinSyntaxField::Present(_))
    );
    let type_parameters =
        format_optional_field(declaration.type_parameters(), doc, |parameters, doc| {
            format_type_parameter_list(doc, Some(parameters))
        });
    let type_parameter_space = if has_type_parameters {
        doc.space()
    } else {
        Doc::nil()
    };
    let receiver_modifiers = format_inline_modifier_prefix(doc, declaration.receiver_modifiers());
    let name = format_required_field(declaration.name(), doc, |name, doc| {
        format_callable_declaration_name(doc, &name)
    });
    let parameters = format_required_field(declaration.parameters(), doc, |parameters, doc| {
        format_value_parameter_list(doc, &parameters)
    });
    let return_type =
        format_type_annotation(doc, declaration.return_colon(), declaration.return_type());
    let constraints = format_optional_field(declaration.constraints(), doc, |constraints, doc| {
        format_type_constraint_list(doc, Some(constraints))
    });
    let body = format_optional_declaration_body(doc, declaration.body());
    let header = doc.concat([
        keyword,
        type_parameters,
        type_parameter_space,
        receiver_modifiers,
        name,
        parameters,
        return_type,
        constraints,
    ]);
    let header = doc.group(header);
    doc.concat([prefix, header, body])
}

pub(super) fn format_secondary_constructor<'source>(
    doc: &mut DocBuilder<'source>,
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    let prefix = format_declaration_prefix(
        doc,
        constructor.leading_modifiers(),
        constructor.context(),
        constructor.post_context_modifiers(),
    );
    let keyword = format_required_field(constructor.constructor_token(), doc, |token, doc| {
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
    let delegation = format_optional_field(constructor.delegation(), doc, |delegation, doc| {
        let has_colon = matches!(delegation.colon(), Ok(KotlinSyntaxField::Present(_)));
        let has_call = syntax_field_has_token(delegation.call());
        let before = if has_colon || has_call {
            doc.space()
        } else {
            Doc::nil()
        };
        let colon = format_required_field(delegation.colon(), doc, |colon, doc| {
            keyword_token(doc, colon)
        });
        let call = format_required_field(delegation.call(), doc, |call, doc| {
            let space = if has_colon && has_call {
                doc.space()
            } else {
                Doc::nil()
            };
            let call = format_constructor_delegation(doc, &call);
            doc.concat([space, call])
        });
        doc.concat([before, colon, call])
    });
    let body = format_optional_field(constructor.body(), doc, |body, doc| {
        let space = doc.space();
        let body = format_block(doc, &body);
        doc.concat([space, body])
    });
    doc.concat([prefix, keyword, parameters, delegation, body])
}

fn format_constructor_delegation<'source>(
    doc: &mut DocBuilder<'source>,
    call: &jolt_kotlin_syntax::ConstructorDelegationCall<'source>,
) -> Doc<'source> {
    format_required_field(call.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    })
}

pub(super) fn format_property_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &PropertyDeclaration<'source>,
) -> Doc<'source> {
    let has_header_after_keyword = syntax_field_has_token(declaration.type_parameters())
        || syntax_field_has_token(declaration.binding())
        || syntax_field_has_token(declaration.r#type())
        || syntax_field_has_token(declaration.constraints());
    let prefix = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let keyword = keyword_with_space(doc, declaration.binding_keyword());
    let type_parameters =
        format_optional_field(declaration.type_parameters(), doc, |parameters, doc| {
            let parameters = format_type_parameter_list(doc, Some(parameters));
            let space = doc.space();
            doc.concat([parameters, space])
        });
    let binding = format_required_field(declaration.binding(), doc, |binding, doc| {
        format_property_binding(doc, &binding)
    });
    let ty = format_type_annotation(doc, declaration.type_colon(), declaration.r#type());
    let constraints = format_optional_field(declaration.constraints(), doc, |constraints, doc| {
        format_type_constraint_list(doc, Some(constraints))
    });
    let initializer = format_optional_property_initializer(
        doc,
        declaration.initializer(),
        has_header_after_keyword,
    );
    let body = format_required_field(declaration.body_members(), doc, |members, doc| {
        format_property_members(doc, &members)
    });
    let declaration = doc.concat([
        prefix,
        keyword,
        type_parameters,
        binding,
        ty,
        constraints,
        initializer,
        body,
    ]);
    doc.group(declaration)
}

fn format_property_binding<'source>(
    doc: &mut DocBuilder<'source>,
    binding: &PropertyBinding<'source>,
) -> Doc<'source> {
    match binding {
        PropertyBinding::Name(name) => format_name(doc, name),
        PropertyBinding::CallableName(name) => format_callable_name(doc, name),
        PropertyBinding::DestructuringDeclaration(pattern) => {
            format_destructuring_declaration(doc, pattern)
        }
        PropertyBinding::BogusPropertyBinding(bogus) => format_malformed(bogus, doc),
    }
}

fn format_property_initializer<'source>(
    doc: &mut DocBuilder<'source>,
    initializer: &PropertyInitializer<'source>,
    leading_space: bool,
) -> Doc<'source> {
    let operator_token = match initializer.operator() {
        Ok(KotlinSyntaxField::Present(operator)) => operator.token(),
        _ => None,
    };
    let has_expression = syntax_field_has_token(initializer.expression());
    let before = if leading_space {
        doc.space()
    } else {
        Doc::nil()
    };
    let operator = format_required_field(initializer.operator(), doc, |operator, doc| {
        let Some(operator) = operator.token() else {
            doc.block_on_invariant("property initializer operator is not a token");
            return Doc::nil();
        };
        if trailing_comments_force_line(&operator) {
            format_token(
                doc,
                &operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            )
        } else {
            format_token(
                doc,
                &operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }
    });
    let expression = format_required_field(initializer.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    });
    if has_expression
        && operator_token.is_some_and(|operator| trailing_comments_force_line(&operator))
    {
        let line = doc.hard_line();
        return doc.concat([before, operator, line, expression]);
    }
    if matches!(
        initializer.expression(),
        Ok(KotlinSyntaxField::Present(
            jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
        ))
    ) {
        let line = doc.hard_line();
        let expression = doc.concat([line, expression]);
        let expression = doc.indent(expression);
        return doc.concat([before, operator, expression]);
    }
    let after = if operator_token.is_some() && has_expression {
        doc.space()
    } else {
        Doc::nil()
    };
    let contents = doc.concat([before, operator, after, expression]);
    doc.group(contents)
}

fn format_optional_property_initializer<'source>(
    doc: &mut DocBuilder<'source>,
    initializer: Result<
        KotlinSyntaxField<'source, PropertyInitializer<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    leading_space: bool,
) -> Doc<'source> {
    match resolve_optional_field(initializer, doc) {
        KotlinFormatField::Present(Some(initializer)) => {
            format_property_initializer(doc, &initializer, leading_space)
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) if leading_space => {
            let space = doc.space();
            doc.concat([space, recovery])
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn format_property_members<'source>(
    doc: &mut DocBuilder<'source>,
    members: &jolt_kotlin_syntax::PropertyBodyMemberList<'source>,
) -> Doc<'source> {
    let contents = doc.concat_list(|docs| {
        for part in members.parts() {
            match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(member) => {
                    let Some(member) = member.cast_family::<PropertyBodyMember<'source>>() else {
                        docs.block_on_invariant("invalid property body member");
                        continue;
                    };
                    let formatted = match member {
                        PropertyBodyMember::ExplicitBackingField(field) => {
                            format_explicit_backing_field(docs, &field)
                        }
                        PropertyBodyMember::PropertyAccessor(accessor) => {
                            format_property_accessor(docs, &accessor)
                        }
                        PropertyBodyMember::BogusPropertyBodyMember(bogus) => {
                            format_malformed(&bogus, docs)
                        }
                    };
                    let line = docs.hard_line();
                    docs.push(line);
                    docs.push(formatted);
                }
                KotlinFormatListPart::Separator(token) => {
                    let removed = format_removed_separator(
                        docs,
                        &token,
                        members.separator_removal_claim(token),
                        false,
                    );
                    docs.push(removed);
                }
                KotlinFormatListPart::Malformed(recovery)
                | KotlinFormatListPart::Invisible(recovery) => docs.push(recovery),
            }
        }
    });
    doc.indent(contents)
}

pub(super) fn format_explicit_backing_field<'source>(
    doc: &mut DocBuilder<'source>,
    field: &ExplicitBackingField<'source>,
) -> Doc<'source> {
    let has_assign = token_field_has_token(field.assign());
    let has_value = syntax_field_has_token(field.expression());
    let keyword = format_required_field(field.field_token(), doc, |field, doc| {
        let Some(field) = field.token() else {
            doc.block_on_invariant("backing-field role is not a token");
            return Doc::nil();
        };
        format_token(
            doc,
            &field,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let assign = format_required_field(field.assign(), doc, |assign, doc| {
        format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let value = format_required_field(field.expression(), doc, |value, doc| {
        format_expression(doc, &value)
    });
    let first_space = if has_assign || has_value {
        doc.space()
    } else {
        Doc::nil()
    };
    let second_space = if has_assign && has_value {
        doc.space()
    } else {
        Doc::nil()
    };
    doc.concat([keyword, first_space, assign, second_space, value])
}

pub(super) fn format_property_accessor<'source>(
    doc: &mut DocBuilder<'source>,
    accessor: &PropertyAccessor<'source>,
) -> Doc<'source> {
    let modifiers = format_modifier_prefix(doc, accessor.modifiers());
    let keyword = format_required_field(accessor.keyword(), doc, |keyword, doc| {
        let Some(keyword) = keyword.token() else {
            doc.block_on_invariant("property accessor keyword is not a token");
            return Doc::nil();
        };
        format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let parameters = format_optional_field(accessor.parameters(), doc, |parameters, doc| {
        format_value_parameter_list(doc, &parameters)
    });
    let return_type = format_type_annotation(doc, accessor.return_colon(), accessor.return_type());
    let body = format_optional_declaration_body(doc, accessor.body());
    doc.concat([modifiers, keyword, parameters, return_type, body])
}

pub(crate) fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(declaration.open_delimiter(), doc);
    let close = resolve_required_delimiter(declaration.close_delimiter(), doc);
    let items = match resolve_required_field(declaration.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            syntax_comma_items(doc, entries.parts(), |entry, doc| match entry {
                jolt_kotlin_syntax::DestructuringPatternEntry::DestructuringEntry(entry) => {
                    format_destructuring_entry(doc, &entry)
                }
                jolt_kotlin_syntax::DestructuringPatternEntry::BogusDestructuringEntry(
                    malformed,
                ) => format_malformed(&malformed, doc),
            })
        }
        KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
            layout_visible: true,
        }],
    };
    let list = compact_parenthesized_list(doc, open.source(), close.source(), items);
    join_delimited_recovery(doc, &open, list, &close)
}

fn format_destructuring_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &DestructuringEntry<'source>,
) -> Doc<'source> {
    let modifier = format_optional_field(entry.modifier(), doc, |token, doc| {
        let modifier = keyword_token(doc, token);
        let space = doc.space();
        doc.concat([modifier, space])
    });
    let name = format_required_field(entry.name(), doc, |name, doc| format_name(doc, &name));
    let has_assign = matches!(entry.assign(), Ok(KotlinSyntaxField::Present(_)));
    let assign = format_optional_field(entry.assign(), doc, |assign, doc| {
        let before = doc.space();
        let assign = keyword_token(doc, assign);
        let after = doc.space();
        doc.concat([before, assign, after])
    });
    let default = format_optional_field(entry.default(), doc, |default, doc| {
        let default = format_expression(doc, &default);
        if has_assign {
            default
        } else {
            let space = doc.space();
            doc.concat([space, default])
        }
    });
    doc.concat([modifier, name, assign, default])
}

fn format_callable_declaration_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &CallableDeclarationName<'source>,
) -> Doc<'source> {
    match name {
        CallableDeclarationName::Name(name) => format_name(doc, name),
        CallableDeclarationName::CallableName(name) => format_callable_name(doc, name),
        CallableDeclarationName::BogusCallableDeclarationName(bogus) => {
            format_malformed(bogus, doc)
        }
    }
}

fn format_callable_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &CallableName<'source>,
) -> Doc<'source> {
    let has_dot = matches!(name.dot(), Ok(KotlinSyntaxField::Present(_)));
    let receiver = format_required_field(name.receiver(), doc, |receiver, doc| {
        format_type_reference(doc, &receiver)
    });
    let dot = format_required_field(name.dot(), doc, |dot, doc| {
        format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    });
    let name = format_required_field(name.name(), doc, |name, doc| format_name(doc, &name));
    let missing_dot_separator = if has_dot { Doc::nil() } else { doc.space() };
    doc.concat([receiver, dot, missing_dot_separator, name])
}

pub(super) fn format_type_alias_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &TypeAliasDeclaration<'source>,
) -> Doc<'source> {
    let has_assign = token_field_has_token(declaration.assign());
    let has_type = syntax_field_has_token(declaration.r#type());
    let modifiers = format_declaration_prefix(
        doc,
        declaration.leading_modifiers(),
        declaration.context(),
        declaration.post_context_modifiers(),
    );
    let keyword = keyword_with_space(doc, declaration.typealias_token());
    let name = format_required_field(declaration.name(), doc, |name, doc| format_name(doc, &name));
    let parameters =
        format_optional_field(declaration.type_parameters(), doc, |parameters, doc| {
            format_type_parameter_list(doc, Some(parameters))
        });
    let assign = format_required_field(declaration.assign(), doc, |assign, doc| {
        keyword_token(doc, assign)
    });
    let ty = format_required_field(declaration.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    let first_space = if has_assign || has_type {
        doc.space()
    } else {
        Doc::nil()
    };
    let second_space = if has_assign && has_type {
        doc.space()
    } else {
        Doc::nil()
    };
    doc.concat([
        modifiers,
        keyword,
        name,
        parameters,
        first_space,
        assign,
        second_space,
        ty,
    ])
}

pub(super) fn format_declaration_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    leading: Result<
        KotlinSyntaxField<'source, ModifierList<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    context: Result<
        KotlinSyntaxField<'source, ContextParameterClause<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    trailing: Result<
        KotlinSyntaxField<'source, ModifierList<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let leading = format_modifier_prefix(doc, leading);
    let context = format_optional_field(context, doc, |context, doc| {
        let context = format_context_parameter_clause(doc, &context);
        let hard_line = doc.hard_line();
        doc.concat([context, hard_line])
    });
    let trailing = format_modifier_prefix(doc, trailing);
    doc.concat([leading, context, trailing])
}

fn format_context_parameter_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &ContextParameterClause<'source>,
) -> Doc<'source> {
    let context = format_required_field(clause.context_token(), doc, |token, doc| {
        keyword_token(doc, token)
    });
    let open = resolve_required_delimiter(clause.open_paren(), doc);
    let close = resolve_required_delimiter(clause.close_paren(), doc);
    let items = match resolve_required_field(clause.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            physical_comma_list_items(doc, entries.parts(), |doc, parameter| CommaListItem {
                doc: match parameter {
                    ContextParameterListEntry::ContextParameter(parameter) => {
                        format_context_parameter(doc, &parameter)
                    }
                    ContextParameterListEntry::BogusContextParameter(bogus) => {
                        format_bogus_list_entry(doc, &bogus)
                    }
                },
                comma: None,
                layout_visible: true,
            })
        }
        KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
            layout_visible: true,
        }],
    };
    let parameters = compact_parenthesized_list(doc, open.source(), close.source(), items);
    let parameters = join_delimited_recovery(doc, &open, parameters, &close);
    doc.concat([context, parameters])
}

fn format_context_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &ContextParameter<'source>,
) -> Doc<'source> {
    let has_name = matches!(parameter.name(), Ok(KotlinSyntaxField::Present(_)));
    let has_colon = matches!(parameter.colon(), Ok(KotlinSyntaxField::Present(_)));
    let has_type = matches!(
        parameter.r#type(),
        Ok(KotlinSyntaxField::Present(ty)) if ty.first_token().is_some()
    );
    let has_assign = matches!(parameter.assign(), Ok(KotlinSyntaxField::Present(_)));
    let name = format_optional_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
    let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
        keyword_token(doc, colon)
    });
    let separation = if has_type && (has_name || has_colon) {
        doc.space()
    } else {
        Doc::nil()
    };
    let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    let assign = format_optional_field(parameter.assign(), doc, |assign, doc| {
        let before = doc.space();
        let assign = keyword_token(doc, assign);
        let after = doc.space();
        doc.concat([before, assign, after])
    });
    let default = format_optional_field(parameter.default(), doc, |expression, doc| {
        let expression = format_expression(doc, &expression);
        if has_assign {
            expression
        } else {
            let space = doc.space();
            doc.concat([space, expression])
        }
    });
    doc.concat([name, colon, separation, ty, assign, default])
}

pub(super) fn format_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierList<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    format_modifier_prefix_with_annotation_break(doc, lists, true)
}

pub(super) fn format_inline_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierList<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    format_modifier_prefix_with_annotation_break(doc, lists, false)
}

fn format_modifier_prefix_with_annotation_break<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierList<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    annotations_break: bool,
) -> Doc<'source> {
    match resolve_required_field(lists, doc) {
        KotlinFormatField::Present(list) => format_modifier_list(doc, &list, annotations_break),
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn format_modifier_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ModifierList<'source>,
    annotations_break: bool,
) -> Doc<'source> {
    format_modifier_list_with_leading(doc, list, annotations_break, LeadingTrivia::Preserve)
}

pub(crate) fn format_modifier_list_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ModifierList<'source>,
    annotations_break: bool,
    leading: LeadingTrivia,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        let mut first = true;
        for part in list.parts() {
            match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(KotlinRoleElement::Node(node)) => {
                    if let Some(annotation) = jolt_kotlin_syntax::Annotation::cast(node) {
                        let item_leading = if first {
                            leading
                        } else {
                            LeadingTrivia::Preserve
                        };
                        let annotation =
                            format_annotation_with_leading(docs, &annotation, item_leading);
                        docs.push(annotation);
                        let separator = if annotations_break {
                            docs.hard_line()
                        } else {
                            docs.space()
                        };
                        docs.push(separator);
                    } else {
                        docs.block_on_invariant("invalid modifier node");
                    }
                }
                KotlinFormatListPart::Item(KotlinRoleElement::Token(token)) => {
                    let item_leading = if first {
                        leading
                    } else {
                        LeadingTrivia::Preserve
                    };
                    let token = format_token(docs, &token, item_leading, TrailingTrivia::Preserve);
                    docs.push(token);
                    let space = docs.space();
                    docs.push(space);
                }
                KotlinFormatListPart::Separator(_) => {}
                KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                KotlinFormatListPart::Invisible(recovery) => {
                    docs.push(recovery);
                    continue;
                }
            }
            first = false;
        }
    })
}

pub(crate) fn format_type_annotation<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    ty: Result<
        KotlinSyntaxField<'source, TypeReference<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let colon = format_optional_field(colon, doc, |colon, doc| {
        format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let ty = format_optional_field(ty, doc, |ty, doc| {
        let space = doc.space();
        let ty = format_type_reference(doc, &ty);
        doc.concat([space, ty])
    });
    doc.concat([colon, ty])
}

pub(crate) fn format_declaration_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: &DeclarationBody<'source>,
) -> Doc<'source> {
    match body {
        DeclarationBody::BlockBody(block) => {
            let space = doc.space();
            let body =
                format_required_field(block.block(), doc, |block, doc| format_block(doc, &block));
            doc.concat([space, body])
        }
        DeclarationBody::ExpressionBody(expression) => format_expression_body(doc, expression),
        DeclarationBody::BogusDeclarationBody(bogus) => format_malformed(bogus, doc),
    }
}

pub(crate) fn format_optional_declaration_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: Result<
        KotlinSyntaxField<'source, DeclarationBody<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    match crate::helpers::recovery::resolve_optional_field(body, doc) {
        KotlinFormatField::Present(Some(body)) => format_declaration_body(doc, &body),
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => {
            let space = doc.space();
            doc.concat([space, recovery])
        }
    }
}

fn format_expression_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: &ExpressionBody<'source>,
) -> Doc<'source> {
    let has_expression = syntax_field_has_token(body.expression());
    let assign_token = match body.assign() {
        Ok(KotlinSyntaxField::Present(assign)) => Some(assign),
        _ => None,
    };
    let before = doc.space();
    let assign = format_required_field(body.assign(), doc, |assign, doc| {
        if trailing_comments_force_line(&assign) {
            format_token(
                doc,
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            )
        } else {
            keyword_token(doc, assign)
        }
    });
    let expression = format_required_field(body.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    });
    if has_expression && assign_token.is_some_and(|assign| trailing_comments_force_line(&assign)) {
        let line = doc.hard_line();
        return doc.concat([before, assign, line, expression]);
    }
    if matches!(
        body.expression(),
        Ok(KotlinSyntaxField::Present(
            jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
        ))
    ) {
        let line = doc.hard_line();
        let expression = doc.concat([line, expression]);
        let expression = doc.indent(expression);
        return doc.concat([before, assign, expression]);
    }
    let after = if assign_token.is_some() && has_expression {
        doc.space()
    } else {
        Doc::nil()
    };
    let contents = doc.concat([before, assign, after, expression]);
    doc.group(contents)
}

fn keyword_with_space<'source>(
    doc: &mut DocBuilder<'source>,
    field: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        let token = keyword_token(doc, token);
        let space = doc.space();
        doc.concat([token, space])
    })
}

fn keyword_token<'source>(
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

fn syntax_field_has_token<'source, T>(
    field: Result<KotlinSyntaxField<'source, T>, jolt_kotlin_syntax::KotlinSyntaxInvariantError>,
) -> bool
where
    T: KotlinSyntaxView<'source>,
{
    match field {
        Ok(KotlinSyntaxField::Present(value)) => value.first_token().is_some(),
        Ok(KotlinSyntaxField::Malformed(malformed)) => malformed.first_token().is_some(),
        Ok(KotlinSyntaxField::Missing(_)) | Err(_) => false,
    }
}

fn token_field_has_token(
    field: Result<
        KotlinSyntaxField<'_, KotlinSyntaxToken<'_>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> bool {
    match field {
        Ok(KotlinSyntaxField::Present(_)) => true,
        Ok(KotlinSyntaxField::Malformed(malformed)) => malformed.first_token().is_some(),
        Ok(KotlinSyntaxField::Missing(_)) | Err(_) => false,
    }
}

fn syntax_comma_items<'source, T>(
    doc: &mut DocBuilder<'source>,
    parts: impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, T>,
            jolt_kotlin_syntax::KotlinSyntaxInvariantError,
        >,
    >,
    mut format_item: impl FnMut(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Vec<CommaListItem<'source>> {
    let mut items = Vec::new();
    for part in parts {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(item) => items.push(CommaListItem {
                doc: format_item(item, doc),
                comma: None,
                layout_visible: true,
            }),
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.iter_mut().rev().find(|item| item.layout_visible)
                    && item.comma.is_none()
                {
                    item.comma = Some(comma);
                } else {
                    items.push(CommaListItem {
                        doc: keyword_token(doc, comma),
                        comma: None,
                        layout_visible: true,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => {
                push_recovery_item(&mut items, recovery, true);
            }
            KotlinFormatListPart::Invisible(recovery) => {
                push_recovery_item(&mut items, recovery, false);
            }
        }
    }
    items
}
