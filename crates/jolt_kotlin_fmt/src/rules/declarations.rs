use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CallableName, ContextParameter, ContextParameterClause, Declaration, DestructuringDeclaration,
    DestructuringEntry, EnumEntry, ExplicitBackingField, FunctionDeclaration, InitializerBlock,
    KotlinFileItem, KotlinNode, KotlinRoleElement, KotlinSyntaxField, KotlinSyntaxListPart,
    KotlinSyntaxToken, KotlinSyntaxView, ModifierList, ModifierListSequence, Name,
    PropertyAccessor, PropertyDeclaration, SecondaryConstructor, TypeAliasDeclaration,
    TypeReference,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_removed_separator, format_token,
    trailing_comments_force_line,
};
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, KotlinFormatListPart, format_malformed,
    format_optional_field, format_or_verbatim, format_required_field, resolve_list_part,
    resolve_required_delimiter, resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;
use crate::rules::statements::format_block;
use crate::rules::types::{
    format_type_constraint_list, format_type_parameter_list, format_type_reference,
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
        KotlinFileItem::PackageHeader(_) | KotlinFileItem::ImportList(_) => Doc::nil(),
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

pub(crate) fn format_fun_interface_file_items<'source>(
    doc: &mut DocBuilder<'source>,
    function: &FunctionDeclaration<'source>,
    interface: &jolt_kotlin_syntax::InterfaceDeclaration<'source>,
) -> Option<Doc<'source>> {
    if !is_fun_interface_header(function) {
        return None;
    }
    let Ok(KotlinSyntaxField::Present(fun)) = function.fun_token() else {
        return None;
    };
    let fun = format_token(
        doc,
        &fun,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let space = doc.space();
    let interface = format_interface_declaration(doc, interface);
    Some(doc.concat([fun, space, interface]))
}

fn is_fun_interface_header(function: &FunctionDeclaration<'_>) -> bool {
    function.is_recovery_free()
        && matches!(function.fun_token(), Ok(KotlinSyntaxField::Present(_)))
        && matches!(function.context(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(
            function.type_parameters(),
            Ok(KotlinSyntaxField::Missing(_))
        )
        && matches!(function.name(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.parameters(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.return_colon(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.return_type(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.constraints(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.assign(), Ok(KotlinSyntaxField::Missing(_)))
        && matches!(function.body(), Ok(KotlinSyntaxField::Missing(_)))
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

pub(super) fn format_enum_entry_with_separator<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &EnumEntry<'source>,
    comma: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let entry = format_enum_entry(doc, entry);
    let comma = comma.map_or_else(Doc::nil, |comma| {
        format_token(
            doc,
            &comma,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([entry, comma])
}

fn format_enum_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &EnumEntry<'source>,
) -> Doc<'source> {
    format_or_verbatim(entry, doc, |doc| {
        format_required_field(entry.expression(), doc, |expression, doc| {
            format_expression(doc, &expression)
        })
    })
}

pub(super) fn format_initializer_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &InitializerBlock<'source>,
) -> Doc<'source> {
    format_or_verbatim(block, doc, |doc| {
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
    })
}

pub(super) fn format_function_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &FunctionDeclaration<'source>,
) -> Doc<'source> {
    format_or_verbatim(declaration, doc, |doc| {
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
        let receiver_modifiers =
            format_inline_modifier_prefix(doc, declaration.receiver_modifiers());
        let name = format_optional_field(declaration.name(), doc, |name, doc| {
            format_callable_role(doc, name)
        });
        let parameters = format_optional_field(declaration.parameters(), doc, |parameters, doc| {
            format_value_parameter_list(doc, &parameters)
        });
        let return_type =
            format_type_annotation(doc, declaration.return_colon(), declaration.return_type());
        let constraints =
            format_optional_field(declaration.constraints(), doc, |constraints, doc| {
                format_type_constraint_list(doc, Some(constraints))
            });
        let body = format_declaration_body(doc, declaration.assign(), declaration.body());
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
    })
}

pub(super) fn format_secondary_constructor<'source>(
    doc: &mut DocBuilder<'source>,
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    format_or_verbatim(constructor, doc, |doc| {
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
        let has_delegation = !matches!(constructor.colon(), Ok(KotlinSyntaxField::Missing(_)))
            || !matches!(
                constructor.delegation_call(),
                Ok(KotlinSyntaxField::Missing(_))
            );
        let colon = format_optional_field(constructor.colon(), doc, |colon, doc| {
            keyword_token(doc, colon)
        });
        let call = format_optional_field(constructor.delegation_call(), doc, |call, doc| {
            let call = format_constructor_delegation(doc, &call);
            let space = doc.space();
            doc.concat([space, call])
        });
        let delegation = if has_delegation {
            let before = doc.space();
            doc.concat([before, colon, call])
        } else {
            Doc::nil()
        };
        let body = format_optional_field(constructor.body(), doc, |body, doc| {
            let space = doc.space();
            let body = format_block(doc, &body);
            doc.concat([space, body])
        });
        doc.concat([prefix, keyword, parameters, delegation, body])
    })
}

fn format_constructor_delegation<'source>(
    doc: &mut DocBuilder<'source>,
    call: &jolt_kotlin_syntax::ConstructorDelegationCall<'source>,
) -> Doc<'source> {
    format_or_verbatim(call, doc, |doc| {
        format_required_field(call.expression(), doc, |expression, doc| {
            format_expression(doc, &expression)
        })
    })
}

pub(super) fn format_property_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &PropertyDeclaration<'source>,
) -> Doc<'source> {
    format_or_verbatim(declaration, doc, |doc| {
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
            if let Some(name) = binding.cast_node::<Name<'source>>() {
                format_name(doc, &name)
            } else if let Some(name) = binding.cast_node::<CallableName<'source>>() {
                format_callable_name(doc, &name)
            } else if let Some(pattern) = binding.cast_node::<DestructuringDeclaration<'source>>() {
                format_destructuring_declaration(doc, &pattern)
            } else {
                doc.block_on_invariant("invalid property binding role");
                Doc::nil()
            }
        });
        let ty = format_type_annotation(doc, declaration.type_colon(), declaration.r#type());
        let constraints =
            format_optional_field(declaration.constraints(), doc, |constraints, doc| {
                format_type_constraint_list(doc, Some(constraints))
            });
        let initializer = format_optional_initializer(
            doc,
            declaration.initializer_operator(),
            declaration.initializer(),
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
    })
}

fn format_optional_initializer<'source>(
    doc: &mut DocBuilder<'source>,
    operator: Result<
        KotlinSyntaxField<'source, KotlinRoleElement<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    expression: Result<
        KotlinSyntaxField<'source, jolt_kotlin_syntax::Expression<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let operator = match crate::helpers::recovery::resolve_optional_field(operator, doc) {
        KotlinFormatField::Present(Some(operator)) => {
            if let Some(token) = operator.token() {
                Ok(Some(token))
            } else {
                doc.block_on_invariant("property initializer operator is not a token");
                Ok(None)
            }
        }
        KotlinFormatField::Present(None) => Ok(None),
        KotlinFormatField::Malformed(malformed) => Err(malformed),
    };
    let expression = crate::helpers::recovery::resolve_optional_field(expression, doc);

    match (operator, expression) {
        (Ok(None), KotlinFormatField::Present(None)) => Doc::nil(),
        (Ok(Some(operator)), KotlinFormatField::Present(Some(expression))) => {
            let before = doc.space();
            if trailing_comments_force_line(&operator) {
                let operator = format_token(
                    doc,
                    &operator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                );
                let line = doc.hard_line();
                let expression = format_expression(doc, &expression);
                return doc.concat([before, operator, line, expression]);
            }
            let operator = format_token(
                doc,
                &operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            );
            if matches!(
                expression,
                jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
            ) {
                let line = doc.hard_line();
                let expression = format_expression(doc, &expression);
                let expression = doc.concat([line, expression]);
                let expression = doc.indent(expression);
                return doc.concat([before, operator, expression]);
            }
            let after = doc.space();
            let expression = format_expression(doc, &expression);
            let contents = doc.concat([before, operator, after, expression]);
            doc.group(contents)
        }
        (operator, expression) => {
            let operator = match operator {
                Ok(Some(operator)) => format_token(
                    doc,
                    &operator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                Ok(None) => Doc::nil(),
                Err(malformed) => malformed,
            };
            let expression = match expression {
                KotlinFormatField::Present(Some(expression)) => format_expression(doc, &expression),
                KotlinFormatField::Present(None) => Doc::nil(),
                KotlinFormatField::Malformed(malformed) => malformed,
            };
            let before = doc.space();
            let after = doc.space();
            doc.concat([before, operator, after, expression])
        }
    }
}

fn format_property_members<'source>(
    doc: &mut DocBuilder<'source>,
    members: &jolt_kotlin_syntax::PropertyBodyMemberList<'source>,
) -> Doc<'source> {
    let contents = doc.concat_list(|docs| {
        for part in members.parts() {
            match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(KotlinRoleElement::Node(node)) => {
                    let formatted = if let Some(field) = ExplicitBackingField::cast(node) {
                        format_explicit_backing_field(docs, &field)
                    } else if let Some(accessor) = PropertyAccessor::cast(node) {
                        format_property_accessor(docs, &accessor)
                    } else {
                        docs.block_on_invariant("invalid property body member");
                        Doc::nil()
                    };
                    let line = docs.hard_line();
                    docs.push(line);
                    docs.push(formatted);
                }
                KotlinFormatListPart::Item(KotlinRoleElement::Token(token))
                | KotlinFormatListPart::Separator(token) => {
                    let removed = format_removed_separator(
                        docs,
                        &token,
                        members.separator_removal_claim(token),
                        false,
                    );
                    docs.push(removed);
                }
                KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
            }
        }
    });
    doc.indent(contents)
}

pub(super) fn format_explicit_backing_field<'source>(
    doc: &mut DocBuilder<'source>,
    field: &ExplicitBackingField<'source>,
) -> Doc<'source> {
    format_or_verbatim(field, doc, |doc| {
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
        let first_space = doc.space();
        let second_space = doc.space();
        doc.concat([keyword, first_space, assign, second_space, value])
    })
}

pub(super) fn format_property_accessor<'source>(
    doc: &mut DocBuilder<'source>,
    accessor: &PropertyAccessor<'source>,
) -> Doc<'source> {
    format_or_verbatim(accessor, doc, |doc| {
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
        let return_type =
            format_type_annotation(doc, accessor.return_colon(), accessor.return_type());
        let body = format_property_accessor_body(doc, accessor.assign(), accessor.body());
        doc.concat([modifiers, keyword, parameters, return_type, body])
    })
}

fn format_property_accessor_body<'source>(
    doc: &mut DocBuilder<'source>,
    assign: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    body: Result<
        KotlinSyntaxField<'source, KotlinRoleElement<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let assign = crate::helpers::recovery::resolve_optional_field(assign, doc);
    let body = crate::helpers::recovery::resolve_optional_field(body, doc);
    match (assign, body) {
        (KotlinFormatField::Present(None), KotlinFormatField::Present(None)) => Doc::nil(),
        (KotlinFormatField::Present(Some(assign)), KotlinFormatField::Present(Some(body))) => {
            let before = doc.space();
            let assign = keyword_token(doc, assign);
            if let Some(expression) = body.cast_family::<jolt_kotlin_syntax::Expression<'source>>()
            {
                let line = doc.line();
                let expression = format_expression(doc, &expression);
                let expression = doc.concat([line, expression]);
                let expression = doc.indent(expression);
                let body = doc.concat([before, assign, expression]);
                return doc.group(body);
            }
            let after = doc.space();
            let body = format_declaration_body_role(doc, body);
            doc.concat([before, assign, after, body])
        }
        (KotlinFormatField::Present(None), KotlinFormatField::Present(Some(body))) => {
            let space = doc.space();
            let body = format_declaration_body_role(doc, body);
            doc.concat([space, body])
        }
        (KotlinFormatField::Present(Some(assign)), KotlinFormatField::Present(None)) => {
            let space = doc.space();
            let assign = keyword_token(doc, assign);
            doc.concat([space, assign])
        }
        (assign, body) => {
            let assign = match assign {
                KotlinFormatField::Present(Some(assign)) => keyword_token(doc, assign),
                KotlinFormatField::Present(None) => Doc::nil(),
                KotlinFormatField::Malformed(malformed) => malformed,
            };
            let body = match body {
                KotlinFormatField::Present(Some(body)) => format_declaration_body_role(doc, body),
                KotlinFormatField::Present(None) => Doc::nil(),
                KotlinFormatField::Malformed(malformed) => malformed,
            };
            let before = doc.space();
            let after = doc.space();
            doc.concat([before, assign, after, body])
        }
    }
}

fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    format_or_verbatim(declaration, doc, |doc| {
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
            }],
        };
        format_delimited_with_recovery(doc, &open, &close, items)
    })
}

fn format_destructuring_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &DestructuringEntry<'source>,
) -> Doc<'source> {
    format_or_verbatim(entry, doc, |doc| {
        let modifier = format_optional_field(entry.modifier(), doc, |token, doc| {
            let modifier = keyword_token(doc, token);
            let space = doc.space();
            doc.concat([modifier, space])
        });
        let name = format_required_field(entry.name(), doc, |name, doc| format_name(doc, &name));
        let default = format_optional_initializer(
            doc,
            entry
                .assign()
                .map(|field| field.map(KotlinRoleElement::Token)),
            entry.default(),
        );
        doc.concat([modifier, name, default])
    })
}

fn format_callable_role<'source>(
    doc: &mut DocBuilder<'source>,
    role: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(name) = role.cast_node::<Name<'source>>() {
        format_name(doc, &name)
    } else if let Some(name) = role.cast_node::<CallableName<'source>>() {
        format_callable_name(doc, &name)
    } else {
        doc.block_on_invariant("invalid callable name role");
        Doc::nil()
    }
}

fn format_callable_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &CallableName<'source>,
) -> Doc<'source> {
    format_or_verbatim(name, doc, |doc| {
        match resolve_required_field(name.parts(), doc) {
            KotlinFormatField::Present(parts) => doc.concat_list(|docs| {
                for part in parts.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(KotlinRoleElement::Node(node)) => {
                            let formatted = if let Some(name) = Name::cast(node) {
                                format_name(docs, &name)
                            } else if let Some(ty) = TypeReference::cast(node) {
                                format_type_reference(docs, &ty)
                            } else {
                                docs.block_on_invariant("invalid callable-name node");
                                Doc::nil()
                            };
                            docs.push(formatted);
                        }
                        KotlinFormatListPart::Item(KotlinRoleElement::Token(dot)) => {
                            let dot = format_token(
                                docs,
                                &dot,
                                LeadingTrivia::Preserve,
                                TrailingTrivia::Preserve,
                            );
                            docs.push(dot);
                        }
                        KotlinFormatListPart::Separator(_) => {}
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        }
    })
}

pub(super) fn format_type_alias_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &TypeAliasDeclaration<'source>,
) -> Doc<'source> {
    format_or_verbatim(declaration, doc, |doc| {
        let modifiers = format_declaration_prefix(
            doc,
            declaration.leading_modifiers(),
            declaration.context(),
            declaration.post_context_modifiers(),
        );
        let keyword = keyword_with_space(doc, declaration.typealias_token());
        let name =
            format_required_field(declaration.name(), doc, |name, doc| format_name(doc, &name));
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
        let first_space = doc.space();
        let second_space = doc.space();
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
    })
}

pub(super) fn format_declaration_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    leading: Result<
        KotlinSyntaxField<'source, ModifierListSequence<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    context: Result<
        KotlinSyntaxField<'source, ContextParameterClause<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    trailing: Result<
        KotlinSyntaxField<'source, ModifierListSequence<'source>>,
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
    format_or_verbatim(clause, doc, |doc| {
        let context = format_required_field(clause.context_token(), doc, |token, doc| {
            keyword_token(doc, token)
        });
        let open = resolve_required_delimiter(clause.open_paren(), doc);
        let close = resolve_required_delimiter(clause.close_paren(), doc);
        let items = match resolve_required_field(clause.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                syntax_comma_items(doc, entries.parts(), |parameter, doc| {
                    format_context_parameter(doc, &parameter)
                })
            }
            KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
                doc: recovery,
                comma: None,
            }],
        };
        let parameters = format_delimited_with_recovery(doc, &open, &close, items);
        doc.concat([context, parameters])
    })
}

fn format_context_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &ContextParameter<'source>,
) -> Doc<'source> {
    format_or_verbatim(parameter, doc, |doc| {
        let name =
            format_optional_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
        let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
            keyword_token(doc, colon)
        });
        let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
            format_type_reference(doc, &ty)
        });
        let space = if matches!(parameter.name(), Ok(KotlinSyntaxField::Present(_))) {
            doc.space()
        } else {
            Doc::nil()
        };
        doc.concat([name, colon, space, ty])
    })
}

pub(super) fn format_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierListSequence<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    format_modifier_prefix_with_annotation_break(doc, lists, true)
}

pub(super) fn format_inline_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierListSequence<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    format_modifier_prefix_with_annotation_break(doc, lists, false)
}

fn format_modifier_prefix_with_annotation_break<'source>(
    doc: &mut DocBuilder<'source>,
    lists: Result<
        KotlinSyntaxField<'source, ModifierListSequence<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    annotations_break: bool,
) -> Doc<'source> {
    match resolve_required_field(lists, doc) {
        KotlinFormatField::Present(lists) => doc.concat_list(|docs| {
            for part in lists.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(list) => {
                        let formatted = format_modifier_list(docs, &list, annotations_break);
                        docs.push(formatted);
                    }
                    KotlinFormatListPart::Separator(_) => {}
                    KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                }
            }
        }),
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn format_modifier_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ModifierList<'source>,
    annotations_break: bool,
) -> Doc<'source> {
    format_or_verbatim(list, doc, |doc| {
        match resolve_required_field(list.modifiers(), doc) {
            KotlinFormatField::Present(items) => doc.concat_list(|docs| {
                for part in items.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(KotlinRoleElement::Node(node)) => {
                            if let Some(annotation) = jolt_kotlin_syntax::Annotation::cast(node) {
                                let annotation = format_annotation(docs, &annotation);
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
                            let token = format_token(
                                docs,
                                &token,
                                LeadingTrivia::Preserve,
                                TrailingTrivia::Preserve,
                            );
                            docs.push(token);
                            let space = docs.space();
                            docs.push(space);
                        }
                        KotlinFormatListPart::Separator(_) => {}
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
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

fn format_declaration_body<'source>(
    doc: &mut DocBuilder<'source>,
    assign: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    body: Result<
        KotlinSyntaxField<'source, KotlinRoleElement<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let assign = crate::helpers::recovery::resolve_optional_field(assign, doc);
    let body = crate::helpers::recovery::resolve_optional_field(body, doc);
    match (assign, body) {
        (KotlinFormatField::Present(None), KotlinFormatField::Present(None)) => Doc::nil(),
        (KotlinFormatField::Present(Some(assign)), KotlinFormatField::Present(Some(body))) => {
            let before = doc.space();
            if let Some(expression) = body.cast_family::<jolt_kotlin_syntax::Expression<'source>>()
            {
                if trailing_comments_force_line(&assign) {
                    let assign = format_token(
                        doc,
                        &assign,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::BeforeLineBreak,
                    );
                    let line = doc.hard_line();
                    let expression = format_expression(doc, &expression);
                    return doc.concat([before, assign, line, expression]);
                }
                let assign = keyword_token(doc, assign);
                if matches!(
                    expression,
                    jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
                ) {
                    let line = doc.hard_line();
                    let expression = format_expression(doc, &expression);
                    let expression = doc.concat([line, expression]);
                    let expression = doc.indent(expression);
                    return doc.concat([before, assign, expression]);
                }
                let after = doc.space();
                let expression = format_expression(doc, &expression);
                let contents = doc.concat([before, assign, after, expression]);
                return doc.group(contents);
            }
            let assign = keyword_token(doc, assign);
            let after = doc.space();
            let body = format_declaration_body_role(doc, body);
            doc.concat([before, assign, after, body])
        }
        (KotlinFormatField::Present(None), KotlinFormatField::Present(Some(body))) => {
            let space = doc.space();
            let body = format_declaration_body_role(doc, body);
            doc.concat([space, body])
        }
        (KotlinFormatField::Present(Some(assign)), KotlinFormatField::Present(None)) => {
            let space = doc.space();
            let assign = keyword_token(doc, assign);
            doc.concat([space, assign])
        }
        (assign, body) => {
            let assign = match assign {
                KotlinFormatField::Present(Some(assign)) => keyword_token(doc, assign),
                KotlinFormatField::Present(None) => Doc::nil(),
                KotlinFormatField::Malformed(malformed) => malformed,
            };
            let body = match body {
                KotlinFormatField::Present(Some(body)) => format_declaration_body_role(doc, body),
                KotlinFormatField::Present(None) => Doc::nil(),
                KotlinFormatField::Malformed(malformed) => malformed,
            };
            let before = doc.space();
            let after = doc.space();
            doc.concat([before, assign, after, body])
        }
    }
}

fn format_declaration_body_role<'source>(
    doc: &mut DocBuilder<'source>,
    body: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(block) = body.cast_node::<jolt_kotlin_syntax::Block<'source>>() {
        format_block(doc, &block)
    } else if let Some(expression) = body.cast_family::<jolt_kotlin_syntax::Expression<'source>>() {
        format_expression(doc, &expression)
    } else {
        doc.block_on_invariant("invalid declaration body role");
        Doc::nil()
    }
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
            }),
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                } else {
                    items.push(CommaListItem {
                        doc: keyword_token(doc, comma),
                        comma: None,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => items.push(CommaListItem {
                doc: recovery,
                comma: None,
            }),
        }
    }
    items
}

fn format_delimited_with_recovery<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    close: &KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let open_recovery = match open {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(doc) => *doc,
    };
    let close_recovery = match close {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(doc) => *doc,
    };
    let list = compact_parenthesized_list(doc, open.source(), close.source(), items);
    doc.concat([open_recovery, list, close_recovery])
}
