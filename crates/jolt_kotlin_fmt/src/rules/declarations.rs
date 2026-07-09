use jolt_fmt_ir::{Doc, DocBuilder};
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

pub(crate) fn format_file_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &KotlinFileItem<'source>,
) -> Doc<'source> {
    match item {
        KotlinFileItem::PackageHeader(_) | KotlinFileItem::ImportList(_) => doc.nil(),
        KotlinFileItem::ClassDeclaration(declaration) => format_class_declaration(doc, declaration),
        KotlinFileItem::InterfaceDeclaration(declaration) => {
            format_interface_declaration(doc, declaration)
        }
        KotlinFileItem::ObjectDeclaration(declaration) => {
            format_object_declaration(doc, declaration)
        }
        KotlinFileItem::CompanionObject(object) => format_companion_object(doc, object),
        KotlinFileItem::EnumEntry(entry) => format_enum_entry(doc, entry),
        KotlinFileItem::FunctionDeclaration(declaration) => {
            format_function_declaration(doc, declaration)
        }
        KotlinFileItem::PropertyDeclaration(declaration) => {
            format_property_declaration(doc, declaration)
        }
        KotlinFileItem::TypeAliasDeclaration(declaration) => {
            format_type_alias_declaration(doc, declaration)
        }
        KotlinFileItem::SecondaryConstructor(constructor) => {
            format_secondary_constructor(doc, constructor)
        }
        KotlinFileItem::InitializerBlock(block) => format_initializer_block(doc, block),
        KotlinFileItem::Statement(statement) => crate::rules::statements::format_statement_syntax(
            doc,
            &jolt_kotlin_syntax::StatementSyntax::Statement(*statement),
        ),
    }
}

pub(crate) fn format_fun_interface_file_items<'source>(
    doc: &mut DocBuilder<'source>,
    function: &FunctionDeclaration<'source>,
    interface: &jolt_kotlin_syntax::InterfaceDeclaration<'source>,
) -> Option<Doc<'source>> {
    if !function.is_fun_interface_header() {
        return None;
    }
    let fun_token = function.fun_token()?;
    let fun_token = format_token(
        doc,
        &fun_token,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let space = doc.space();
    let interface = format_interface_declaration(doc, interface);
    Some(doc.concat([fun_token, space, interface]))
}

pub(crate) fn format_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &Declaration<'source>,
) -> Doc<'source> {
    match declaration {
        Declaration::ClassDeclaration(declaration) => format_class_declaration(doc, declaration),
        Declaration::InterfaceDeclaration(declaration) => {
            format_interface_declaration(doc, declaration)
        }
        Declaration::ObjectDeclaration(declaration) => format_object_declaration(doc, declaration),
        Declaration::CompanionObject(object) => format_companion_object(doc, object),
        Declaration::EnumEntry(entry) => format_enum_entry(doc, entry),
        Declaration::FunctionDeclaration(declaration) => {
            format_function_declaration(doc, declaration)
        }
        Declaration::PropertyDeclaration(declaration) => {
            format_property_declaration(doc, declaration)
        }
        Declaration::TypeAliasDeclaration(declaration) => {
            format_type_alias_declaration(doc, declaration)
        }
        Declaration::SecondaryConstructor(constructor) => {
            format_secondary_constructor(doc, constructor)
        }
        Declaration::InitializerBlock(block) => format_initializer_block(doc, block),
    }
}

pub(super) fn format_enum_entry_with_separator<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &EnumEntry<'source>,
    comma: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let entry = format_enum_entry(doc, entry);
    let comma = if let Some(comma) = comma {
        format_token(
            doc,
            &comma,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([entry, comma])
}

fn format_enum_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &EnumEntry<'source>,
) -> Doc<'source> {
    if let Some(expression) = entry.expression() {
        format_expression(doc, &expression)
    } else {
        doc.nil()
    }
}

pub(super) fn format_initializer_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &InitializerBlock<'source>,
) -> Doc<'source> {
    let init = if let Some(init) = block.init_token() {
        format_token(
            doc,
            &init,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let body = if let Some(body) = block.block() {
        let space = doc.space();
        let body = format_block(doc, &body);
        doc.concat([space, body])
    } else {
        doc.nil()
    };
    doc.concat([init, body])
}

pub(super) fn format_function_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &FunctionDeclaration<'source>,
) -> Doc<'source> {
    let fun_token = declaration.fun_token();
    let context_clause = declaration.context_parameter_clause();
    let modifier_lists = declaration.modifier_lists().collect::<Vec<_>>();
    let (leading_modifiers, post_context_modifiers) =
        declaration_prefix_modifier_lists(doc, &modifier_lists, context_clause.as_ref(), fun_token);

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
        .and_then(|name| format_callable_name(doc, &name, receiver_modifiers.as_ref()));
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
        let space = doc.space();
        let block = format_block(doc, &block);
        doc.concat([space, block])
    } else {
        format_optional_declaration_expression_tail(doc, declaration, header_end).unwrap_or_else(
            || {
                format_recovered_declaration_tail(
                    doc,
                    declaration
                        .tail_tokens_between(header_end, declaration.text_range().end().get()),
                )
            },
        )
    };

    let prefix = format_declaration_prefix(
        doc,
        leading_modifiers,
        context_clause,
        post_context_modifiers,
    );
    let fun_token = if let Some(fun_token) = fun_token {
        format_keyword_with_space(doc, &fun_token)
    } else {
        doc.nil()
    };
    let type_parameters = format_type_parameter_list(doc, declaration.type_parameter_list());
    let type_parameter_space = if declaration.type_parameter_list().is_some() {
        doc.space()
    } else {
        doc.nil()
    };
    let callable_name = callable_name
        .as_ref()
        .map_or_else(Doc::nil, |name| name.doc);
    let parameters = if let Some(parameters) = parameters.as_ref() {
        format_value_parameter_list(doc, parameters)
    } else {
        doc.nil()
    };
    let return_type = format_type_annotation(doc, declaration.colon(), declaration.return_type());
    let constraints = format_type_constraint_list(doc, declaration.type_constraint_list());
    let header = doc.concat([
        fun_token,
        type_parameters,
        type_parameter_space,
        callable_name,
        parameters,
        return_type,
        constraints,
    ]);
    let header = doc.group(header);

    doc.concat([prefix, header, tail])
}

pub(super) fn format_secondary_constructor<'source>(
    doc: &mut DocBuilder<'source>,
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    let parameters = constructor.value_parameter_list();
    let modifiers = format_modifier_prefix(doc, constructor.modifiers());
    let constructor_token = if let Some(constructor_token) = constructor.constructor_token() {
        format_token(
            doc,
            &constructor_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let parameters = if let Some(parameters) = parameters.as_ref() {
        format_value_parameter_list(doc, parameters)
    } else {
        doc.nil()
    };
    let tail = constructor_delegation_call_tail(doc, constructor);
    let header = doc.concat([modifiers, constructor_token, parameters, tail]);
    let header = doc.group(header);

    if let Some(block) = constructor.block() {
        let space = doc.space();
        let block = format_block(doc, &block);
        doc.concat([header, space, block])
    } else {
        header
    }
}

fn constructor_delegation_call_tail<'source>(
    doc: &mut DocBuilder<'source>,
    constructor: &SecondaryConstructor<'source>,
) -> Doc<'source> {
    let Some(colon) = constructor.colon() else {
        return doc.nil();
    };

    let before_colon = doc.space();
    let colon = format_token(
        doc,
        &colon,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let expression = if let Some(expression) = constructor
        .delegation_call()
        .and_then(|call| call.expression())
    {
        let space = doc.space();
        let expression = format_expression(doc, &expression);
        doc.concat([space, expression])
    } else {
        doc.nil()
    };
    doc.concat([before_colon, colon, expression])
}

fn format_optional_declaration_expression_tail<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &FunctionDeclaration<'source>,
    header_end: usize,
) -> Option<Doc<'source>> {
    let assign = declaration.assign_token();
    let expression = declaration.expression();
    if assign.is_none() && expression.is_none() {
        return Some(doc.nil());
    }

    declaration_expression_tail(
        doc,
        |start, end| declaration.tail_is_trivia_between(start, end),
        header_end,
        declaration.text_range().end().get(),
        assign,
        expression,
    )
}

pub(super) fn format_property_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &PropertyDeclaration<'source>,
) -> Doc<'source> {
    let keyword = declaration.val_token().or_else(|| declaration.var_token());
    let context_clause = declaration.context_parameter_clause();
    let modifier_lists = declaration.modifier_lists().collect::<Vec<_>>();
    let (leading_modifiers, post_context_modifiers) =
        declaration_prefix_modifier_lists(doc, &modifier_lists, context_clause.as_ref(), keyword);

    let property_body_items = property_body_items(doc, declaration);
    let property_delegate = declaration.delegate_token();
    let callable_name = declaration
        .destructuring_declaration()
        .and_then(|declaration| destructuring_callable_name(doc, &declaration))
        .or_else(|| {
            declaration
                .callable_name()
                .and_then(|name| format_callable_name(doc, &name, None))
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
            doc,
            |start, end| declaration.tail_is_trivia_between(start, end),
            header_end,
            declaration.text_range().end().get(),
            declaration.assign_token(),
            declaration.expression(),
        );
        let delegate_tail = property_delegate.and_then(|by| {
            declaration_expression_tail_between(
                doc,
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
                    .then(|| doc.nil())
            })
            .unwrap_or_else(|| {
                format_recovered_declaration_tail(
                    doc,
                    declaration
                        .tail_tokens_between(header_end, declaration.text_range().end().get()),
                )
            })
    } else {
        format_property_body_tail(doc, declaration, header_end, &property_body_items)
            .unwrap_or_else(|| doc.nil())
    };

    let prefix = format_declaration_prefix(
        doc,
        leading_modifiers,
        context_clause,
        post_context_modifiers,
    );
    let keyword = if let Some(keyword) = keyword {
        format_keyword_with_space(doc, &keyword)
    } else {
        doc.nil()
    };
    let type_parameters = format_type_parameter_list(doc, declaration.type_parameter_list());
    let type_parameter_space = if declaration.type_parameter_list().is_some() {
        doc.space()
    } else {
        doc.nil()
    };
    let callable_name = callable_name
        .as_ref()
        .map_or_else(Doc::nil, |name| name.doc);
    let ty = format_type_annotation(doc, declaration.colon(), declaration.ty());
    let constraints = format_type_constraint_list(doc, declaration.type_constraint_list());
    let header = doc.concat([
        keyword,
        type_parameters,
        type_parameter_space,
        callable_name,
        ty,
        constraints,
    ]);
    let header = doc.group(header);

    doc.concat([prefix, header, tail])
}

fn property_body_items<'source>(
    _doc: &mut DocBuilder<'source>,
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
    doc: &mut DocBuilder<'source>,
    declaration: &PropertyDeclaration<'source>,
    header_end: usize,
    body_items: &[PropertyBodyItem<'source>],
) -> Option<Doc<'source>> {
    let body_start = body_items.first()?.start();
    let initializer = if let Some(assign) = declaration.assign_token() {
        declaration_expression_tail_between(
            doc,
            |start, end| declaration.tail_is_trivia_between(start, end),
            header_end,
            assign,
            declaration.expression()?,
            body_start,
        )
    } else if declaration.tail_is_trivia_between(header_end, body_start) {
        Some(doc.nil())
    } else {
        None
    }?;
    let body_docs: Vec<_> = body_items
        .iter()
        .map(|item| format_property_body_item(doc, item))
        .collect();

    let line = doc.hard_line();
    let body = crate::helpers::blocks::join_hard_lines(doc, body_docs);
    let body = doc.concat([line, body]);
    let body = doc.indent(body);
    Some(doc.concat([initializer, body]))
}

fn format_property_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &PropertyBodyItem<'source>,
) -> Doc<'source> {
    match item {
        PropertyBodyItem::ExplicitBackingField(field) => format_explicit_backing_field(doc, field),
        PropertyBodyItem::Accessor(accessor) => format_property_accessor(doc, accessor),
    }
}

pub(super) fn format_explicit_backing_field<'source>(
    doc: &mut DocBuilder<'source>,
    field: &ExplicitBackingField<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();

    if let Some(keyword) = field.field_token() {
        let keyword = format_token(
            doc,
            &keyword,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        docs.push(keyword, doc);
    }
    if let Some(assign) = field.assign_token() {
        if !docs.is_empty() {
            let space = doc.space();
            docs.push(space, doc);
        }
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeSpaceIfComments,
        );
        docs.push(assign, doc);
    }
    if let Some(expression) = field.expression() {
        if !docs.is_empty() {
            let space = doc.space();
            docs.push(space, doc);
        }
        let expression = format_expression(doc, &expression);
        docs.push(expression, doc);
    }

    if docs.is_empty() {
        format_token_sequence(
            doc,
            field.token_iter(),
            LeadingTrivia::SuppressAlreadyHandled,
        )
    } else {
        docs.finish(doc)
    }
}

pub(super) fn format_property_accessor<'source>(
    doc: &mut DocBuilder<'source>,
    accessor: &PropertyAccessor<'source>,
) -> Doc<'source> {
    let body = if let Some(block) = accessor.block() {
        let space = doc.space();
        let block = format_block(doc, &block);
        doc.concat([space, block])
    } else if let Some(assign) = accessor.assign_token() {
        let space = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let expression = if let Some(expression) = accessor.expression() {
            let line = doc.line();
            let expression = format_expression(doc, &expression);
            let expression = doc.concat([line, expression]);
            doc.indent(expression)
        } else {
            doc.nil()
        };
        let body = doc.concat([space, assign, expression]);
        doc.group(body)
    } else {
        doc.nil()
    };

    let modifiers = format_modifier_prefix(doc, accessor.modifiers());
    let keyword = if let Some(keyword) = accessor.keyword_token() {
        format_token(
            doc,
            &keyword,
            if accessor.modifiers().is_some() {
                LeadingTrivia::Preserve
            } else {
                LeadingTrivia::SuppressAlreadyHandled
            },
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let parameters = if let Some(parameters) = accessor.value_parameter_list() {
        format_value_parameter_list(doc, &parameters)
    } else {
        doc.nil()
    };
    let return_type = format_type_annotation(doc, accessor.colon(), accessor.return_type());
    doc.concat([modifiers, keyword, parameters, return_type, body])
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
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Option<FormattedCallableName<'source>> {
    let last_token = declaration.close_delimiter()?;
    Some(FormattedCallableName {
        doc: format_destructuring_declaration(doc, declaration),
        last_token,
    })
}

fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    let items =
        recovered_comma_list_items(doc, declaration.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: if let Some(name) = entry.entry.name() {
                    format_name(doc, &name)
                } else {
                    doc.nil()
                },
                comma: entry.comma,
            }
        });
    compact_parenthesized_list(
        doc,
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        items,
    )
}

fn format_callable_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &CallableName<'source>,
    receiver_modifiers: Option<&ModifierList<'source>>,
) -> Option<FormattedCallableName<'source>> {
    let last_name = name.name()?;
    let last_token = last_name.last_token()?;
    let (doc, last_token) = if let (Some(receiver), Some(separator)) =
        (name.receiver_type(), name.receiver_separator())
    {
        let receiver = format_callable_receiver(doc, &receiver, receiver_modifiers);
        let separator = format_token(
            doc,
            &separator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let name = format_name(doc, &last_name);
        (doc.concat([receiver, separator, name]), last_token)
    } else {
        (format_name(doc, &last_name), last_token)
    };

    Some(FormattedCallableName { doc, last_token })
}

fn format_callable_receiver<'source>(
    doc: &mut DocBuilder<'source>,
    receiver: &TypeReference<'source>,
    receiver_modifiers: Option<&ModifierList<'source>>,
) -> Doc<'source> {
    let Some(modifiers) = receiver_modifiers else {
        return format_type_reference(doc, receiver);
    };

    let modifiers = format_inline_modifier_prefix(doc, modifiers);
    let receiver = format_type_reference(doc, receiver);
    doc.concat([modifiers, receiver])
}

fn format_inline_modifier_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: &ModifierList<'source>,
) -> Doc<'source> {
    let annotations = modifiers.annotations();
    let modifier_tokens = modifiers.modifier_tokens();
    let mut docs = doc.list();
    for annotation in annotations {
        let annotation = format_annotation(doc, &annotation);
        docs.push(annotation, doc);
        let space = doc.space();
        docs.push(space, doc);
    }
    for token in modifier_tokens {
        let token = format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        docs.push(token, doc);
        let space = doc.space();
        docs.push(space, doc);
    }
    docs.finish(doc)
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
    doc: &mut DocBuilder<'source>,
    declaration: &TypeAliasDeclaration<'source>,
) -> Doc<'source> {
    let keyword = if let Some(keyword) = declaration.typealias_token() {
        format_keyword_with_space(doc, &keyword)
    } else {
        doc.nil()
    };
    let name = if let Some(name) = declaration.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let parameters = format_type_parameter_list(doc, declaration.type_parameter_list());
    let assign = if let Some(assign) = declaration.assign_token() {
        let before = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let ty = if let Some(ty) = declaration.ty() {
            let space = doc.space();
            let ty = crate::rules::types::format_type_reference(doc, &ty);
            doc.concat([space, ty])
        } else {
            doc.nil()
        };
        doc.concat([before, assign, ty])
    } else {
        doc.nil()
    };
    doc.concat([keyword, name, parameters, assign])
}

fn format_declaration_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: Vec<ModifierList<'source>>,
    context_clause: Option<ContextParameterClause<'source>>,
    post_context_modifiers: Vec<ModifierList<'source>>,
) -> Doc<'source> {
    let leading = format_modifier_lists_prefix(doc, modifiers);
    let context = if let Some(clause) = context_clause {
        let clause = format_context_parameter_clause(doc, &clause);
        let hard_line = doc.hard_line();
        doc.concat([clause, hard_line])
    } else {
        doc.nil()
    };
    let post_context = format_modifier_lists_prefix(doc, post_context_modifiers);
    doc.concat([leading, context, post_context])
}

fn declaration_prefix_modifier_lists<'source>(
    _doc: &mut DocBuilder<'source>,
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
    doc: &mut DocBuilder<'source>,
    modifiers: impl IntoIterator<Item = ModifierList<'source>>,
) -> Doc<'source> {
    let modifiers = modifiers
        .into_iter()
        .map(|modifiers| format_modifier_prefix(doc, Some(modifiers)))
        .collect::<Vec<_>>();
    doc.concat(modifiers)
}

fn format_context_parameter_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &ContextParameterClause<'source>,
) -> Doc<'source> {
    let ContextParameterClauseItems { items } = context_parameter_clause_items(doc, clause);
    let context = if let Some(token) = clause.context_token() {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let parameters = compact_parenthesized_list(
        doc,
        clause.open_paren().as_ref(),
        clause.close_paren().as_ref(),
        items,
    );
    doc.concat([context, parameters])
}

struct ContextParameterClauseItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn context_parameter_clause_items<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &ContextParameterClause<'source>,
) -> ContextParameterClauseItems<'source> {
    let entries = clause.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);

    for entry in entries {
        push_context_parameter_entry(doc, &mut items, entry);
    }

    ContextParameterClauseItems { items }
}

fn format_context_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &jolt_kotlin_syntax::ContextParameter<'source>,
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

fn push_context_parameter_entry<'source>(
    doc: &mut DocBuilder<'source>,
    items: &mut Vec<CommaListItem<'source>>,
    entry: RecoveredSeparatedListEntry<
        'source,
        jolt_kotlin_syntax::ContextParameterClauseEntry<'source>,
    >,
) {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => items.push(CommaListItem {
            doc: format_context_parameter(doc, &entry.parameter),
            comma: entry.comma,
        }),
        RecoveredSeparatedListEntry::Token(token) => items.push(CommaListItem {
            doc: format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve),
            comma: None,
        }),
        RecoveredSeparatedListEntry::Error(error) => items.push(CommaListItem {
            doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
            comma: None,
        }),
        RecoveredSeparatedListEntry::Node(node) => items.push(CommaListItem {
            doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
            comma: None,
        }),
    }
}

fn format_keyword_with_space<'source>(
    doc: &mut DocBuilder<'source>,
    keyword: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let keyword = format_token(
        doc,
        keyword,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let space = doc.space();
    doc.concat([keyword, space])
}

pub(super) fn format_modifier_prefix<'source>(
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

pub(crate) fn format_type_annotation<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Option<KotlinSyntaxToken<'source>>,
    ty: Option<jolt_kotlin_syntax::TypeReference<'source>>,
) -> Doc<'source> {
    let Some(colon) = colon else {
        return doc.nil();
    };
    let colon = format_token(
        doc,
        &colon,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let ty = if let Some(ty) = ty {
        let space = doc.space();
        let ty = crate::rules::types::format_type_reference(doc, &ty);
        doc.concat([space, ty])
    } else {
        doc.nil()
    };
    doc.concat([colon, ty])
}

fn declaration_expression_tail<'source>(
    doc: &mut DocBuilder<'source>,
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
        let space = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        return Some(doc.concat([space, assign]));
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

    Some(format_declaration_expression_tail(
        doc,
        &assign,
        &expression,
    ))
}

fn format_declaration_expression_tail<'source>(
    doc: &mut DocBuilder<'source>,
    assign: &KotlinSyntaxToken<'source>,
    expression: &jolt_kotlin_syntax::Expression<'source>,
) -> Doc<'source> {
    if trailing_comments_force_line(assign) {
        let space = doc.space();
        let assign = format_token(
            doc,
            assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        );
        let line = doc.hard_line();
        let expression = format_expression(doc, expression);
        return doc.concat([space, assign, line, expression]);
    }

    if matches!(
        expression,
        jolt_kotlin_syntax::Expression::AnnotatedExpression(_)
    ) {
        let space = doc.space();
        let assign = format_token(
            doc,
            assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let line = doc.hard_line();
        let expression = format_expression(doc, expression);
        let expression = doc.concat([line, expression]);
        let expression = doc.indent(expression);
        return doc.concat([space, assign, expression]);
    }

    let before = doc.space();
    let assign = format_token(
        doc,
        assign,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let after = doc.space();
    let expression = format_expression(doc, expression);
    let contents = doc.concat([before, assign, after, expression]);
    doc.group(contents)
}

fn declaration_expression_tail_between<'source>(
    doc: &mut DocBuilder<'source>,
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

    Some(format_declaration_expression_tail(
        doc,
        &assign,
        &expression,
    ))
}

fn format_recovered_declaration_tail<'source>(
    doc: &mut DocBuilder<'source>,
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let mut tokens = tokens.into_iter();

    let Some(first) = tokens.next() else {
        return doc.nil();
    };

    let space = doc.space();
    let tokens = crate::helpers::comments::format_token_sequence(
        doc,
        std::iter::once(first).chain(tokens),
        LeadingTrivia::Preserve,
    );
    doc.concat([space, tokens])
}
