use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, space};
use jolt_kotlin_syntax::{
    CallableName, ContextParameterClause, Declaration, DestructuringDeclaration, EnumEntry,
    ExplicitBackingField, FunctionDeclaration, InitializerBlock, KotlinFileItem, KotlinSyntaxToken,
    ModifierList, PropertyAccessor, PropertyDeclaration, RecoveredSeparatedListEntry,
    SecondaryConstructor, TypeAliasDeclaration, TypeReference,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
    trailing_comments_force_line,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, recovered_comma_list_items,
};
use crate::helpers::modifiers::modifier_prefix_from_parts;
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

pub(crate) fn format_file_item<'source>(item: &KotlinFileItem<'source>) -> Doc<'source> {
    match item {
        KotlinFileItem::PackageHeader(_) | KotlinFileItem::ImportList(_) => jolt_fmt_ir::nil(),
        KotlinFileItem::ClassDeclaration(declaration) => format_class_declaration(declaration),
        KotlinFileItem::InterfaceDeclaration(declaration) => {
            format_interface_declaration(declaration)
        }
        KotlinFileItem::ObjectDeclaration(declaration) => format_object_declaration(declaration),
        KotlinFileItem::CompanionObject(object) => format_companion_object(object),
        KotlinFileItem::EnumEntry(entry) => format_enum_entry(entry),
        KotlinFileItem::FunctionDeclaration(declaration) => {
            format_function_declaration(declaration)
        }
        KotlinFileItem::PropertyDeclaration(declaration) => {
            format_property_declaration(declaration)
        }
        KotlinFileItem::TypeAliasDeclaration(declaration) => {
            format_type_alias_declaration(declaration)
        }
        KotlinFileItem::SecondaryConstructor(constructor) => {
            format_secondary_constructor(constructor)
        }
        KotlinFileItem::InitializerBlock(block) => format_initializer_block(block),
        KotlinFileItem::Statement(statement) => crate::rules::statements::format_statement_syntax(
            &jolt_kotlin_syntax::StatementSyntax::Statement(*statement),
        ),
    }
}

pub(crate) fn format_fun_interface_file_items<'source>(
    function: &FunctionDeclaration<'source>,
    interface: &jolt_kotlin_syntax::InterfaceDeclaration<'source>,
) -> Option<Doc<'source>> {
    if !function.is_fun_interface_header() {
        return None;
    }
    let fun_token = function.fun_token()?;
    Some(concat([
        format_token(
            &fun_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        format_interface_declaration(interface),
    ]))
}

pub(crate) fn format_declaration<'source>(declaration: &Declaration<'source>) -> Doc<'source> {
    match declaration {
        Declaration::ClassDeclaration(declaration) => format_class_declaration(declaration),
        Declaration::InterfaceDeclaration(declaration) => format_interface_declaration(declaration),
        Declaration::ObjectDeclaration(declaration) => format_object_declaration(declaration),
        Declaration::CompanionObject(object) => format_companion_object(object),
        Declaration::EnumEntry(entry) => format_enum_entry(entry),
        Declaration::FunctionDeclaration(declaration) => format_function_declaration(declaration),
        Declaration::PropertyDeclaration(declaration) => format_property_declaration(declaration),
        Declaration::TypeAliasDeclaration(declaration) => {
            format_type_alias_declaration(declaration)
        }
        Declaration::SecondaryConstructor(constructor) => format_secondary_constructor(constructor),
        Declaration::InitializerBlock(block) => format_initializer_block(block),
    }
}

pub(super) fn format_enum_entry_with_separator<'source>(
    entry: &EnumEntry<'source>,
    comma: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        format_enum_entry(entry),
        comma.map_or_else(jolt_fmt_ir::nil, |comma| {
            format_token(&comma, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
    ])
}

fn format_enum_entry<'source>(entry: &EnumEntry<'source>) -> Doc<'source> {
    entry
        .expression()
        .map_or_else(jolt_fmt_ir::nil, |expression| {
            format_expression(&expression)
        })
}

pub(super) fn format_initializer_block<'source>(block: &InitializerBlock<'source>) -> Doc<'source> {
    concat([
        block.init_token().map_or_else(jolt_fmt_ir::nil, |init| {
            format_token(
                &init,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
        block.block().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([space(), format_block(&body)])
        }),
    ])
}

pub(super) fn format_function_declaration<'source>(
    declaration: &FunctionDeclaration<'source>,
) -> Doc<'source> {
    let fun_token = declaration.fun_token();
    let context_clause = declaration.context_parameter_clause();
    let modifier_lists = declaration.modifier_lists().collect::<Vec<_>>();
    let (leading_modifiers, post_context_modifiers) =
        declaration_prefix_modifier_lists(&modifier_lists, context_clause.as_ref(), fun_token);

    let parameters = declaration.value_parameter_list();
    let receiver_modifiers = modifier_lists.iter().copied().find(|modifiers| {
        fun_token.is_some_and(|fun_token| {
            modifiers.text_range().start() >= fun_token.token_text_range().end()
                && parameters.as_ref().is_none_or(|parameters| {
                    modifiers.text_range().end() <= parameters.text_range().start()
                })
        })
    });
    let callable_name = declaration
        .callable_name()
        .and_then(|name| format_callable_name(&name, receiver_modifiers.as_ref()));
    let block = declaration.block();
    let header_end = declaration
        .type_constraint_list()
        .map(|constraints| constraints.text_range().end().get())
        .or_else(|| {
            declaration
                .return_type()
                .map(|ty| ty.text_range().end().get())
        })
        .or_else(|| {
            parameters
                .as_ref()
                .and_then(jolt_kotlin_syntax::ValueParameterList::last_token)
                .map(|token| token.token_text_range().end().get())
        })
        .or_else(|| {
            callable_name
                .as_ref()
                .map(|name| name.last_token().token_text_range().end().get())
        })
        .or_else(|| fun_token.map(|fun_token| fun_token.token_text_range().end().get()))
        .unwrap_or_else(|| declaration.text_range().start().get());
    let tail = if let Some(block) = block {
        concat([space(), format_block(&block)])
    } else {
        format_optional_declaration_expression_tail(declaration, header_end).unwrap_or_else(|| {
            format_recovered_declaration_tail(
                declaration.tail_tokens_between(header_end, declaration.text_range().end().get()),
            )
        })
    };

    let prefix =
        format_declaration_prefix(leading_modifiers, context_clause, post_context_modifiers);
    let header = group(concat([
        fun_token.map_or_else(jolt_fmt_ir::nil, |fun_token| {
            format_keyword_with_space(&fun_token)
        }),
        format_type_parameter_list(declaration.type_parameter_list()),
        declaration
            .type_parameter_list()
            .map_or_else(jolt_fmt_ir::nil, |_| space()),
        callable_name
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |name| name.doc.clone()),
        parameters
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_value_parameter_list(parameters)
            }),
        format_type_annotation(declaration.colon(), declaration.return_type()),
        format_type_constraint_list(declaration.type_constraint_list()),
    ]));

    concat([prefix, header, tail])
}

pub(super) fn format_secondary_constructor<'source>(
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    let parameters = constructor.value_parameter_list();
    let header = group(concat([
        format_modifier_prefix(constructor.modifiers()),
        constructor
            .constructor_token()
            .map_or_else(jolt_fmt_ir::nil, |constructor_token| {
                format_token(
                    &constructor_token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        parameters
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_value_parameter_list(parameters)
            }),
        constructor_delegation_call_tail(constructor),
    ]));

    constructor.block().map_or(header.clone(), |block| {
        concat([header, space(), format_block(&block)])
    })
}

fn constructor_delegation_call_tail<'source>(
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    let Some(colon) = constructor.colon() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        space(),
        format_token(&colon, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        constructor
            .delegation_call()
            .and_then(|call| call.expression())
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                concat([space(), format_expression(&expression)])
            }),
    ])
}

fn format_optional_declaration_expression_tail<'source>(
    declaration: &FunctionDeclaration<'source>,
    header_end: usize,
) -> Option<Doc<'source>> {
    let assign = declaration.assign_token();
    let expression = declaration.expression();
    if assign.is_none() && expression.is_none() {
        return Some(jolt_fmt_ir::nil());
    }

    declaration_expression_tail(
        |start, end| declaration.tail_is_trivia_between(start, end),
        header_end,
        declaration.text_range().end().get(),
        assign,
        expression,
    )
}

pub(super) fn format_property_declaration<'source>(
    declaration: &PropertyDeclaration<'source>,
) -> Doc<'source> {
    let keyword = declaration.val_token().or_else(|| declaration.var_token());
    let context_clause = declaration.context_parameter_clause();
    let modifier_lists = declaration.modifier_lists().collect::<Vec<_>>();
    let (leading_modifiers, post_context_modifiers) =
        declaration_prefix_modifier_lists(&modifier_lists, context_clause.as_ref(), keyword);

    let property_body_items = property_body_items(declaration);
    let property_delegate = declaration.delegate_token();
    let callable_name = declaration
        .destructuring_declaration()
        .and_then(|declaration| destructuring_callable_name(&declaration))
        .or_else(|| {
            declaration
                .callable_name()
                .and_then(|name| format_callable_name(&name, None))
        });
    let header_end = declaration
        .type_constraint_list()
        .map(|constraints| constraints.text_range().end().get())
        .or_else(|| declaration.ty().map(|ty| ty.text_range().end().get()))
        .or_else(|| {
            callable_name
                .as_ref()
                .map(|name| name.last_token().token_text_range().end().get())
        })
        .or_else(|| keyword.map(|keyword| keyword.token_text_range().end().get()))
        .unwrap_or_else(|| declaration.text_range().start().get());
    let tail = if property_body_items.is_empty() {
        let assign_tail = declaration_expression_tail(
            |start, end| declaration.tail_is_trivia_between(start, end),
            header_end,
            declaration.text_range().end().get(),
            declaration.assign_token(),
            declaration.expression(),
        );
        let delegate_tail = property_delegate.and_then(|by| {
            declaration_expression_tail_between(
                |start, end| declaration.tail_is_trivia_between(start, end),
                header_end,
                by,
                declaration.expression()?,
                declaration.text_range().end().get(),
            )
        });
        assign_tail
            .or(delegate_tail)
            .or_else(|| {
                declaration
                    .tail_is_trivia_between(header_end, declaration.text_range().end().get())
                    .then(jolt_fmt_ir::nil)
            })
            .unwrap_or_else(|| {
                format_recovered_declaration_tail(
                    declaration
                        .tail_tokens_between(header_end, declaration.text_range().end().get()),
                )
            })
    } else {
        format_property_body_tail(declaration, header_end, &property_body_items)
            .unwrap_or_else(jolt_fmt_ir::nil)
    };

    let prefix =
        format_declaration_prefix(leading_modifiers, context_clause, post_context_modifiers);
    let header = group(concat([
        keyword.map_or_else(jolt_fmt_ir::nil, |keyword| {
            format_keyword_with_space(&keyword)
        }),
        format_type_parameter_list(declaration.type_parameter_list()),
        declaration
            .type_parameter_list()
            .map_or_else(jolt_fmt_ir::nil, |_| space()),
        callable_name
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |name| name.doc.clone()),
        format_type_annotation(declaration.colon(), declaration.ty()),
        format_type_constraint_list(declaration.type_constraint_list()),
    ]));

    concat([prefix, header, tail])
}

fn property_body_items<'source>(
    declaration: &PropertyDeclaration<'source>,
) -> Vec<PropertyBodyItem<'source>> {
    let mut items = declaration
        .explicit_backing_fields()
        .map(PropertyBodyItem::ExplicitBackingField)
        .chain(declaration.accessors().map(PropertyBodyItem::Accessor))
        .collect::<Vec<_>>();
    items.sort_by_key(PropertyBodyItem::start);
    items
}

fn format_property_body_tail<'source>(
    declaration: &PropertyDeclaration<'source>,
    header_end: usize,
    body_items: &[PropertyBodyItem<'source>],
) -> Option<Doc<'source>> {
    let body_start = body_items.first()?.start();
    let initializer = declaration.assign_token().map_or_else(
        || {
            declaration
                .tail_is_trivia_between(header_end, body_start)
                .then(jolt_fmt_ir::nil)
        },
        |assign| {
            declaration_expression_tail_between(
                |start, end| declaration.tail_is_trivia_between(start, end),
                header_end,
                assign,
                declaration.expression()?,
                body_start,
            )
        },
    )?;
    let body_docs: Vec<_> = body_items.iter().map(format_property_body_item).collect();

    Some(concat([
        initializer,
        indent(concat([
            hard_line(),
            crate::helpers::blocks::join_hard_lines(body_docs),
        ])),
    ]))
}

fn format_property_body_item<'source>(item: &PropertyBodyItem<'source>) -> Doc<'source> {
    match item {
        PropertyBodyItem::ExplicitBackingField(field) => format_explicit_backing_field(field),
        PropertyBodyItem::Accessor(accessor) => format_property_accessor(accessor),
    }
}

pub(super) fn format_explicit_backing_field<'source>(
    field: &ExplicitBackingField<'source>,
) -> Doc<'source> {
    let mut docs = Vec::with_capacity(5);

    if let Some(keyword) = field.field_token() {
        docs.push(format_token(
            &keyword,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        ));
    }
    if let Some(assign) = field.assign_token() {
        if !docs.is_empty() {
            docs.push(space());
        }
        docs.push(format_token(
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeSpaceIfComments,
        ));
    }
    if let Some(expression) = field.expression() {
        if !docs.is_empty() {
            docs.push(space());
        }
        docs.push(format_expression(&expression));
    }

    if docs.is_empty() {
        format_token_sequence(field.token_iter(), LeadingTrivia::SuppressAlreadyHandled)
    } else {
        concat(docs)
    }
}

pub(super) fn format_property_accessor<'source>(
    accessor: &PropertyAccessor<'source>,
) -> Doc<'source> {
    let body = if let Some(block) = accessor.block() {
        concat([space(), format_block(&block)])
    } else if let Some(assign) = accessor.assign_token() {
        group(concat([
            space(),
            format_token(
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            accessor
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    indent(concat([line(), format_expression(&expression)]))
                }),
        ]))
    } else {
        jolt_fmt_ir::nil()
    };

    concat([
        format_modifier_prefix(accessor.modifiers()),
        accessor
            .keyword_token()
            .map_or_else(jolt_fmt_ir::nil, |keyword| {
                format_token(
                    &keyword,
                    if accessor.modifiers().is_some() {
                        LeadingTrivia::Preserve
                    } else {
                        LeadingTrivia::SuppressAlreadyHandled
                    },
                    TrailingTrivia::Preserve,
                )
            }),
        accessor
            .value_parameter_list()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_value_parameter_list(&parameters)
            }),
        format_type_annotation(accessor.colon(), accessor.return_type()),
        body,
    ])
}

enum PropertyBodyItem<'source> {
    ExplicitBackingField(ExplicitBackingField<'source>),
    Accessor(PropertyAccessor<'source>),
}

impl PropertyBodyItem<'_> {
    fn start(&self) -> usize {
        match self {
            Self::ExplicitBackingField(field) => field.text_range().start().get(),
            Self::Accessor(accessor) => accessor.text_range().start().get(),
        }
    }
}

fn destructuring_callable_name<'source>(
    declaration: &DestructuringDeclaration<'source>,
) -> Option<FormattedCallableName<'source>> {
    let last_token = declaration.close_delimiter()?;
    Some(FormattedCallableName {
        doc: format_destructuring_declaration(declaration),
        last_token,
    })
}

fn format_destructuring_declaration<'source>(
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    compact_parenthesized_list(
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        recovered_comma_list_items(declaration.entries_with_recovered(), |entry| {
            CommaListItem {
                doc: entry
                    .entry
                    .name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
                comma: entry.comma,
            }
        }),
    )
}

fn format_callable_name<'source>(
    name: &CallableName<'source>,
    receiver_modifiers: Option<&ModifierList<'source>>,
) -> Option<FormattedCallableName<'source>> {
    let last_name = name.name()?;
    let last_token = last_name.last_token()?;
    let (doc, last_token) = if let (Some(receiver), Some(separator)) =
        (name.receiver_type(), name.receiver_separator())
    {
        (
            concat([
                format_callable_receiver(&receiver, receiver_modifiers),
                format_token(
                    &separator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                format_name(&last_name),
            ]),
            last_token,
        )
    } else {
        (format_name(&last_name), last_token)
    };

    Some(FormattedCallableName { doc, last_token })
}

fn format_callable_receiver<'source>(
    receiver: &TypeReference<'source>,
    receiver_modifiers: Option<&ModifierList<'source>>,
) -> Doc<'source> {
    let Some(modifiers) = receiver_modifiers else {
        return format_type_reference(receiver);
    };

    concat([
        format_inline_modifier_prefix(modifiers),
        format_type_reference(receiver),
    ])
}

fn format_inline_modifier_prefix<'source>(modifiers: &ModifierList<'source>) -> Doc<'source> {
    let annotations = modifiers.annotations();
    let (annotation_count, _) = annotations.size_hint();
    let modifier_tokens = modifiers.modifier_tokens();
    let (modifier_count, _) = modifier_tokens.size_hint();
    let mut docs = Vec::with_capacity(
        annotation_count
            .saturating_add(modifier_count)
            .saturating_mul(2),
    );
    for annotation in annotations {
        docs.push(format_annotation(&annotation));
        docs.push(space());
    }
    for token in modifier_tokens {
        docs.push(format_token(
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ));
        docs.push(space());
    }
    concat(docs)
}

struct FormattedCallableName<'source> {
    doc: Doc<'source>,
    last_token: KotlinSyntaxToken<'source>,
}

impl<'source> FormattedCallableName<'source> {
    fn last_token(&self) -> KotlinSyntaxToken<'source> {
        self.last_token
    }
}

pub(super) fn format_type_alias_declaration<'source>(
    declaration: &TypeAliasDeclaration<'source>,
) -> Doc<'source> {
    concat([
        declaration
            .typealias_token()
            .map_or_else(jolt_fmt_ir::nil, |keyword| {
                format_keyword_with_space(&keyword)
            }),
        declaration
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        format_type_parameter_list(declaration.type_parameter_list()),
        declaration
            .assign_token()
            .map_or_else(jolt_fmt_ir::nil, |assign| {
                concat([
                    space(),
                    format_token(
                        &assign,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    ),
                    declaration.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
                        concat([space(), crate::rules::types::format_type_reference(&ty)])
                    }),
                ])
            }),
    ])
}

fn format_declaration_prefix<'source>(
    modifiers: Vec<ModifierList<'source>>,
    context_clause: Option<ContextParameterClause<'source>>,
    post_context_modifiers: Vec<ModifierList<'source>>,
) -> Doc<'source> {
    concat([
        format_modifier_lists_prefix(modifiers),
        context_clause.map_or_else(jolt_fmt_ir::nil, |clause| {
            concat([format_context_parameter_clause(&clause), hard_line()])
        }),
        format_modifier_lists_prefix(post_context_modifiers),
    ])
}

fn declaration_prefix_modifier_lists<'source>(
    modifiers: &[ModifierList<'source>],
    context_clause: Option<&ContextParameterClause<'source>>,
    keyword: Option<KotlinSyntaxToken<'source>>,
) -> (Vec<ModifierList<'source>>, Vec<ModifierList<'source>>) {
    let keyword_start = keyword.map(|keyword| keyword.token_text_range().start());
    let context_start = context_clause.map(|clause| clause.text_range().start());
    let context_end = context_clause.map(|clause| clause.text_range().end());
    let prefix_end = keyword_start.or(context_start);

    let mut leading = Vec::new();
    let mut post_context = Vec::new();
    for modifiers in modifiers {
        if prefix_end.is_some_and(|end| modifiers.text_range().end() > end) {
            continue;
        }
        if context_start.is_some_and(|start| modifiers.text_range().end() <= start) {
            leading.push(*modifiers);
        } else if context_end.is_some_and(|end| modifiers.text_range().start() >= end) {
            post_context.push(*modifiers);
        } else if context_clause.is_none() {
            leading.push(*modifiers);
        }
    }

    (leading, post_context)
}

fn format_modifier_lists_prefix<'source>(
    modifiers: impl IntoIterator<Item = ModifierList<'source>>,
) -> Doc<'source> {
    concat(
        modifiers
            .into_iter()
            .map(|modifiers| format_modifier_prefix(Some(modifiers))),
    )
}

fn format_context_parameter_clause<'source>(
    clause: &ContextParameterClause<'source>,
) -> Doc<'source> {
    let ContextParameterClauseItems { items } = context_parameter_clause_items(clause);
    concat([
        clause
            .context_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        compact_parenthesized_list(
            clause.open_paren().as_ref(),
            clause.close_paren().as_ref(),
            items,
        ),
    ])
}

struct ContextParameterClauseItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn context_parameter_clause_items<'source>(
    clause: &ContextParameterClause<'source>,
) -> ContextParameterClauseItems<'source> {
    let entries = clause.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);

    for entry in entries {
        push_context_parameter_entry(&mut items, entry);
    }

    ContextParameterClauseItems { items }
}

fn format_context_parameter<'source>(
    parameter: &jolt_kotlin_syntax::ContextParameter<'source>,
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

fn push_context_parameter_entry<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    entry: RecoveredSeparatedListEntry<
        'source,
        jolt_kotlin_syntax::ContextParameterClauseEntry<'source>,
    >,
) {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => items.push(CommaListItem {
            doc: format_context_parameter(&entry.parameter),
            comma: entry.comma,
        }),
        RecoveredSeparatedListEntry::Token(token) => items.push(CommaListItem {
            doc: format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve),
            comma: None,
        }),
        RecoveredSeparatedListEntry::Error(error) => items.push(CommaListItem {
            doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
            comma: None,
        }),
        RecoveredSeparatedListEntry::Node(node) => items.push(CommaListItem {
            doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
            comma: None,
        }),
    }
}

fn format_keyword_with_space<'source>(keyword: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    concat([
        format_token(
            keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
    ])
}

pub(super) fn format_modifier_prefix(modifiers: Option<ModifierList<'_>>) -> Doc<'_> {
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

pub(crate) fn format_type_annotation<'source>(
    colon: Option<KotlinSyntaxToken<'source>>,
    ty: Option<jolt_kotlin_syntax::TypeReference<'source>>,
) -> Doc<'source> {
    let Some(colon) = colon else {
        return jolt_fmt_ir::nil();
    };
    concat([
        format_token(&colon, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        ty.map_or_else(jolt_fmt_ir::nil, |ty| {
            concat([space(), crate::rules::types::format_type_reference(&ty)])
        }),
    ])
}

fn declaration_expression_tail<'source>(
    is_trivia_between: impl Fn(usize, usize) -> bool,
    header_end: usize,
    tail_end: usize,
    assign: Option<KotlinSyntaxToken<'source>>,
    expression: Option<jolt_kotlin_syntax::Expression<'source>>,
) -> Option<Doc<'source>> {
    let assign = assign?;
    if assign.token_text_range().start().get() < header_end {
        return None;
    }
    if !is_trivia_between(header_end, assign.token_text_range().start().get()) {
        return None;
    }
    let Some(expression) = expression else {
        if !is_trivia_between(assign.token_text_range().end().get(), tail_end) {
            return None;
        }
        return Some(concat([
            space(),
            format_token(&assign, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        ]));
    };
    if !is_trivia_between(
        assign.token_text_range().end().get(),
        expression.text_range().start().get(),
    ) {
        return None;
    }
    if !is_trivia_between(expression.text_range().end().get(), tail_end) {
        return None;
    }

    Some(format_declaration_expression_tail(&assign, &expression))
}

fn format_declaration_expression_tail<'source>(
    assign: &KotlinSyntaxToken<'source>,
    expression: &jolt_kotlin_syntax::Expression<'source>,
) -> Doc<'source> {
    if trailing_comments_force_line(assign) {
        return concat([
            space(),
            format_token(
                assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            hard_line(),
            format_expression(expression),
        ]);
    }

    if matches!(
        expression,
        jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
    ) {
        return concat([
            space(),
            format_token(
                assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            indent(concat([hard_line(), format_expression(expression)])),
        ]);
    }

    group(concat([
        space(),
        format_token(
            assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        format_expression(expression),
    ]))
}

fn declaration_expression_tail_between<'source>(
    is_trivia_between: impl Fn(usize, usize) -> bool,
    header_end: usize,
    assign: KotlinSyntaxToken<'source>,
    expression: jolt_kotlin_syntax::Expression<'source>,
    tail_end: usize,
) -> Option<Doc<'source>> {
    if assign.token_text_range().start().get() < header_end
        || expression.text_range().end().get() > tail_end
    {
        return None;
    }
    if !is_trivia_between(header_end, assign.token_text_range().start().get()) {
        return None;
    }
    if !is_trivia_between(
        assign.token_text_range().end().get(),
        expression.text_range().start().get(),
    ) {
        return None;
    }
    if !is_trivia_between(expression.text_range().end().get(), tail_end) {
        return None;
    }

    Some(format_declaration_expression_tail(&assign, &expression))
}

fn format_recovered_declaration_tail<'source>(
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let mut tokens = tokens.into_iter();

    let Some(first) = tokens.next() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        space(),
        crate::helpers::comments::format_token_sequence(
            std::iter::once(first).chain(tokens),
            LeadingTrivia::Preserve,
        ),
    ])
}
