use jolt_fmt_ir::{Doc, concat, group, indent, line, space};
use jolt_kotlin_syntax::{
    ClassBody, ClassDeclaration, CompanionObject, DelegationSpecifier, DelegationSpecifierList,
    InterfaceDeclaration, KotlinSyntaxToken, ModifierList, Name, ObjectDeclaration,
    ObjectExpression, PrimaryConstructor,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::lists::{CommaListItem, comma_list};
use crate::helpers::source::source_gap_is_trivia;
use crate::rules::expressions::{format_expression, format_value_argument_list};
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;
use crate::rules::types::{format_type_constraint_list, format_type_parameter_list};
use crate::rules::variables::format_value_parameter_list;

use super::{
    format_inline_modifier_prefix, format_modifier_prefix, member_bodies::format_class_body,
};

pub(super) fn format_class_declaration<'source>(
    declaration: &ClassDeclaration<'source>,
) -> Doc<'source> {
    let constructor = declaration.primary_constructor();
    let type_parameters = declaration.type_parameter_list();
    format_simple_type_declaration(SimpleTypeDeclaration {
        modifiers: declaration.modifiers(),
        keyword: declaration.class_token(),
        name: declaration.name(),
        type_parameters,
        tail: constructor.and_then(|constructor| {
            simple_primary_constructor_tail(
                &declaration.name()?,
                type_parameters,
                &constructor,
                declaration.source_text(),
                declaration.text_range().start().get(),
                declaration.token_iter(),
            )
        }),
        colon: declaration.colon(),
        delegation: declaration.delegation_specifier_list(),
        constraints: declaration.type_constraint_list(),
        body: declaration.body(),
    })
}

pub(super) fn format_interface_declaration<'source>(
    declaration: &InterfaceDeclaration<'source>,
) -> Doc<'source> {
    format_simple_type_declaration(SimpleTypeDeclaration {
        modifiers: declaration.modifiers(),
        keyword: declaration.interface_token(),
        name: declaration.name(),
        type_parameters: declaration.type_parameter_list(),
        tail: None,
        colon: declaration.colon(),
        delegation: declaration.delegation_specifier_list(),
        constraints: declaration.type_constraint_list(),
        body: declaration.body(),
    })
}

pub(super) fn format_object_declaration<'source>(
    declaration: &ObjectDeclaration<'source>,
) -> Doc<'source> {
    let modifiers = declaration.modifiers();
    format_simple_type_declaration(SimpleTypeDeclaration {
        modifiers,
        keyword: declaration.object_token(),
        name: declaration.name(),
        type_parameters: None,
        tail: None,
        colon: declaration.colon(),
        delegation: declaration.delegation_specifier_list(),
        constraints: None,
        body: declaration.body(),
    })
}

pub(super) fn format_companion_object<'source>(
    declaration: &CompanionObject<'source>,
) -> Doc<'source> {
    format_simple_type_declaration(SimpleTypeDeclaration {
        modifiers: declaration.modifiers(),
        keyword: declaration.object_token(),
        name: declaration.name(),
        type_parameters: None,
        tail: None,
        colon: declaration.colon(),
        delegation: declaration.delegation_specifier_list(),
        constraints: None,
        body: declaration.body(),
    })
}

pub(crate) fn format_object_expression<'source>(
    expression: &ObjectExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = expression
        .object_token()
        .map_or_else(jolt_fmt_ir::nil, |keyword| {
            format_token(
                &keyword,
                leading,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });

    group(concat([
        keyword,
        format_object_expression_delegation(
            expression.colon(),
            expression.delegation_specifier_list(),
        ),
        format_class_body(expression.body()),
    ]))
}

fn format_object_expression_delegation<'source>(
    colon: Option<KotlinSyntaxToken<'source>>,
    delegation: Option<DelegationSpecifierList<'source>>,
) -> Doc<'source> {
    let Some(delegation) = delegation else {
        return jolt_fmt_ir::nil();
    };
    let entries = delegation.entries().collect::<Vec<_>>();
    match entries.as_slice() {
        [] => jolt_fmt_ir::nil(),
        [entry] if entry.comma.is_none() => concat([
            space(),
            colon.map_or_else(jolt_fmt_ir::nil, |colon| {
                format_token(
                    &colon,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
            space(),
            format_delegation_specifier(&entry.specifier),
        ]),
        _ => format_delegation_specifier_list(colon, Some(delegation)),
    }
}

struct SimpleTypeDeclaration<'source> {
    modifiers: Option<ModifierList<'source>>,
    keyword: Option<KotlinSyntaxToken<'source>>,
    name: Option<Name<'source>>,
    type_parameters: Option<jolt_kotlin_syntax::TypeParameterList<'source>>,
    tail: Option<DeclarationTail<'source>>,
    colon: Option<KotlinSyntaxToken<'source>>,
    delegation: Option<DelegationSpecifierList<'source>>,
    constraints: Option<jolt_kotlin_syntax::TypeConstraintList<'source>>,
    body: Option<ClassBody<'source>>,
}

fn format_simple_type_declaration(declaration: SimpleTypeDeclaration<'_>) -> Doc<'_> {
    let SimpleTypeDeclaration {
        modifiers,
        keyword,
        name,
        type_parameters,
        tail,
        colon,
        delegation,
        constraints,
        body,
    } = declaration;
    group(concat([
        format_modifier_prefix(modifiers),
        keyword.map_or_else(jolt_fmt_ir::nil, |keyword| {
            format_token(
                &keyword,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
        name.as_ref().map_or_else(jolt_fmt_ir::nil, |name| {
            concat([space(), format_name(name)])
        }),
        format_type_parameter_list(type_parameters),
        tail.map_or_else(jolt_fmt_ir::nil, |tail| tail.doc),
        format_delegation_specifier_list(colon, delegation),
        format_type_constraint_list(constraints),
        format_class_body(body),
    ]))
}

fn format_delegation_specifier_list<'source>(
    colon: Option<KotlinSyntaxToken<'source>>,
    delegation: Option<DelegationSpecifierList<'source>>,
) -> Doc<'source> {
    let Some(delegation) = delegation else {
        return jolt_fmt_ir::nil();
    };
    let DelegationSpecifierListItems { items } = delegation_specifier_list_items(&delegation);
    if items.is_empty() {
        return jolt_fmt_ir::nil();
    }

    indent(concat([
        line(),
        colon.map_or_else(jolt_fmt_ir::nil, |colon| {
            format_token(
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
        indent(group(concat([line(), comma_list(items)]))),
    ]))
}

struct DelegationSpecifierListItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn delegation_specifier_list_items<'source>(
    delegation: &DelegationSpecifierList<'source>,
) -> DelegationSpecifierListItems<'source> {
    let source_start = delegation.text_range().start().get();
    let source = delegation.source_text();
    let tokens = delegation.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut covered_until = delegation.text_range().start().get();
    let mut items = Vec::new();

    for entry in delegation.entries() {
        push_recovered_delegation_specifier_gap(
            &mut items,
            source,
            source_start,
            &tokens,
            &mut token_cursor,
            covered_until,
            entry.specifier.text_range().start().get(),
        );
        items.push(CommaListItem {
            doc: format_delegation_specifier(&entry.specifier),
            comma: entry.comma,
        });
        covered_until = entry.comma.map_or_else(
            || entry.specifier.text_range().end().get(),
            |comma| comma.token_text_range().end().get(),
        );
    }

    push_recovered_delegation_specifier_gap(
        &mut items,
        source,
        source_start,
        &tokens,
        &mut token_cursor,
        covered_until,
        delegation.text_range().end().get(),
    );

    DelegationSpecifierListItems { items }
}

fn push_recovered_delegation_specifier_gap<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return;
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

    if !gap_tokens.is_empty() {
        items.push(CommaListItem {
            doc: format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
            comma: None,
        });
    }
}

fn format_delegation_specifier<'source>(specifier: &DelegationSpecifier<'source>) -> Doc<'source> {
    let Some(ty) = specifier.ty() else {
        return specifier
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            });
    };

    concat([
        format_type_reference(&ty),
        specifier
            .value_argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_value_argument_list(&arguments)
            }),
        specifier.by_token().map_or_else(jolt_fmt_ir::nil, |by| {
            concat([
                space(),
                format_token(&by, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                space(),
                specifier
                    .expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression)
                    }),
            ])
        }),
    ])
}

fn simple_primary_constructor_tail<'source>(
    name: &Name<'source>,
    type_parameters: Option<jolt_kotlin_syntax::TypeParameterList<'source>>,
    constructor: &PrimaryConstructor<'source>,
    declaration_source: &'source str,
    declaration_start: usize,
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> Option<DeclarationTail<'source>> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let expected_open_start = type_parameters
        .and_then(|parameters| parameters.last_token())
        .or_else(|| name.last_token())?
        .token_text_range()
        .end();
    let parameters = constructor.value_parameter_list()?;
    let open = parameters.open_paren()?;
    let modifiers = constructor.modifiers();
    let constructor_token = constructor.constructor_token();

    if modifiers.is_none() && constructor_token.is_none() {
        if !source_gap_is_trivia(
            declaration_source,
            declaration_start,
            tokens.iter().copied(),
            expected_open_start.get(),
            open.token_text_range().start().get(),
        ) {
            return None;
        }
        return Some(DeclarationTail {
            doc: format_value_parameter_list(&parameters),
        });
    }

    let first_tail_start = modifiers
        .and_then(|modifiers| modifiers.first_token())
        .or(constructor_token)
        .map(|token| token.token_text_range().start())?;
    if first_tail_start < expected_open_start {
        return None;
    }
    if !source_gap_is_trivia(
        declaration_source,
        declaration_start,
        tokens.iter().copied(),
        expected_open_start.get(),
        first_tail_start.get(),
    ) {
        return None;
    }

    Some(DeclarationTail {
        doc: concat([
            space(),
            modifiers.map_or_else(jolt_fmt_ir::nil, |modifiers| {
                format_inline_modifier_prefix(&modifiers)
            }),
            constructor_token.map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }),
            format_value_parameter_list(&parameters),
        ]),
    })
}

struct DeclarationTail<'source> {
    doc: Doc<'source>,
}
