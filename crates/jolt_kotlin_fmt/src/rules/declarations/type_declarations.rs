use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ClassBody, ClassDeclaration, CompanionObject, DelegationSpecifier, DelegationSpecifierList,
    InterfaceDeclaration, KotlinSyntaxToken, ModifierList, Name, ObjectDeclaration,
    ObjectExpression, PrimaryConstructor, RecoveredSeparatedListEntry,
};
use jolt_syntax::source_gap_is_trivia;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::lists::{CommaListItem, comma_list};
use crate::rules::expressions::{format_expression, format_value_argument_list};
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;
use crate::rules::types::{format_type_constraint_list, format_type_parameter_list};
use crate::rules::variables::format_value_parameter_list;

use super::{
    format_inline_modifier_prefix, format_modifier_prefix, member_bodies::format_class_body,
};

pub(super) fn format_class_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &ClassDeclaration<'source>,
) -> Doc<'source> {
    let constructor = declaration.primary_constructor();
    let type_parameters = declaration.type_parameter_list();
    let tail = if let Some(constructor) = constructor {
        if let Some(name) = declaration.name() {
            simple_primary_constructor_tail(
                doc,
                &name,
                type_parameters,
                &constructor,
                declaration.source_text(),
                declaration.text_range().start().get(),
                || declaration.token_iter(),
            )
        } else {
            None
        }
    } else {
        None
    };
    format_simple_type_declaration(
        doc,
        SimpleTypeDeclaration {
            modifiers: declaration.modifiers(),
            keyword: declaration.class_token(),
            name: declaration.name(),
            type_parameters,
            tail,
            colon: declaration.colon(),
            delegation: declaration.delegation_specifier_list(),
            constraints: declaration.type_constraint_list(),
            body: declaration.body(),
        },
    )
}

pub(super) fn format_interface_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &InterfaceDeclaration<'source>,
) -> Doc<'source> {
    format_simple_type_declaration(
        doc,
        SimpleTypeDeclaration {
            modifiers: declaration.modifiers(),
            keyword: declaration.interface_token(),
            name: declaration.name(),
            type_parameters: declaration.type_parameter_list(),
            tail: None,
            colon: declaration.colon(),
            delegation: declaration.delegation_specifier_list(),
            constraints: declaration.type_constraint_list(),
            body: declaration.body(),
        },
    )
}

pub(super) fn format_object_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &ObjectDeclaration<'source>,
) -> Doc<'source> {
    let modifiers = declaration.modifiers();
    format_simple_type_declaration(
        doc,
        SimpleTypeDeclaration {
            modifiers,
            keyword: declaration.object_token(),
            name: declaration.name(),
            type_parameters: None,
            tail: None,
            colon: declaration.colon(),
            delegation: declaration.delegation_specifier_list(),
            constraints: None,
            body: declaration.body(),
        },
    )
}

pub(super) fn format_companion_object<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &CompanionObject<'source>,
) -> Doc<'source> {
    format_simple_type_declaration(
        doc,
        SimpleTypeDeclaration {
            modifiers: declaration.modifiers(),
            keyword: declaration.object_token(),
            name: declaration.name(),
            type_parameters: None,
            tail: None,
            colon: declaration.colon(),
            delegation: declaration.delegation_specifier_list(),
            constraints: None,
            body: declaration.body(),
        },
    )
}

pub(crate) fn format_object_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ObjectExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = if let Some(keyword) = expression.object_token() {
        format_token(
            doc,
            &keyword,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let delegation = format_object_expression_delegation(
        doc,
        expression.colon(),
        expression.delegation_specifier_list(),
    );
    let body = format_class_body(doc, expression.body());
    let expression = doc.concat([keyword, delegation, body]);
    doc.group(expression)
}

fn format_object_expression_delegation<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Option<KotlinSyntaxToken<'source>>,
    delegation: Option<DelegationSpecifierList<'source>>,
) -> Doc<'source> {
    let Some(delegation) = delegation else {
        return doc.nil();
    };
    let mut entries = delegation.entries_with_recovered();
    let first = entries.next();
    if entries.next().is_none()
        && let Some(RecoveredSeparatedListEntry::Entry(entry)) = first
        && entry.comma.is_none()
    {
        let before_colon = doc.space();
        let colon = if let Some(colon) = colon {
            format_token(
                doc,
                &colon,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        } else {
            doc.nil()
        };
        let after_colon = doc.space();
        let specifier = format_delegation_specifier(doc, &entry.specifier);
        return doc.concat([before_colon, colon, after_colon, specifier]);
    }

    format_delegation_specifier_list(doc, colon, Some(delegation))
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

fn format_simple_type_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: SimpleTypeDeclaration<'source>,
) -> Doc<'source> {
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
    let modifiers = format_modifier_prefix(doc, modifiers);
    let keyword = if let Some(keyword) = keyword {
        format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let name = if let Some(name) = name.as_ref() {
        let space = doc.space();
        let name = format_name(doc, name);
        doc.concat([space, name])
    } else {
        doc.nil()
    };
    let type_parameters = format_type_parameter_list(doc, type_parameters);
    let tail = tail.map_or_else(|| doc.nil(), |tail| tail.doc);
    let delegation = format_delegation_specifier_list(doc, colon, delegation);
    let constraints = format_type_constraint_list(doc, constraints);
    let body = format_class_body(doc, body);
    let declaration = doc.concat([
        modifiers,
        keyword,
        name,
        type_parameters,
        tail,
        delegation,
        constraints,
        body,
    ]);
    doc.group(declaration)
}

fn format_delegation_specifier_list<'source>(
    doc: &mut DocBuilder<'source>,
    colon: Option<KotlinSyntaxToken<'source>>,
    delegation: Option<DelegationSpecifierList<'source>>,
) -> Doc<'source> {
    let Some(delegation) = delegation else {
        return doc.nil();
    };
    let DelegationSpecifierListItems { items } = delegation_specifier_list_items(doc, &delegation);
    if items.is_empty() {
        return doc.nil();
    }

    let line = doc.line();
    let colon = if let Some(colon) = colon {
        format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let inner_line = doc.line();
    let specifiers = comma_list(doc, items);
    let specifiers = doc.concat([inner_line, specifiers]);
    let specifiers = doc.group(specifiers);
    let specifiers = doc.indent(specifiers);
    let list = doc.concat([line, colon, specifiers]);
    doc.indent(list)
}

struct DelegationSpecifierListItems<'source> {
    items: Vec<CommaListItem<'source>>,
}

fn delegation_specifier_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    delegation: &DelegationSpecifierList<'source>,
) -> DelegationSpecifierListItems<'source> {
    let entries = delegation.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);

    for entry in entries {
        push_delegation_specifier_entry(doc, &mut items, entry);
    }

    DelegationSpecifierListItems { items }
}

fn push_delegation_specifier_entry<'source>(
    doc: &mut DocBuilder<'source>,
    items: &mut Vec<CommaListItem<'source>>,
    entry: RecoveredSeparatedListEntry<
        'source,
        jolt_kotlin_syntax::DelegationSpecifierListEntry<'source>,
    >,
) {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => items.push(CommaListItem {
            doc: format_delegation_specifier(doc, &entry.specifier),
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

fn format_delegation_specifier<'source>(
    doc: &mut DocBuilder<'source>,
    specifier: &DelegationSpecifier<'source>,
) -> Doc<'source> {
    let Some(ty) = specifier.ty() else {
        return if let Some(expression) = specifier.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
    };

    let ty = format_type_reference(doc, &ty);
    let arguments = if let Some(arguments) = specifier.value_argument_list() {
        format_value_argument_list(doc, &arguments)
    } else {
        doc.nil()
    };
    let by = if let Some(by) = specifier.by_token() {
        let before_by = doc.space();
        let by = format_token(doc, &by, LeadingTrivia::Preserve, TrailingTrivia::Preserve);
        let after_by = doc.space();
        let expression = if let Some(expression) = specifier.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
        doc.concat([before_by, by, after_by, expression])
    } else {
        doc.nil()
    };
    doc.concat([ty, arguments, by])
}

fn simple_primary_constructor_tail<'source, Tokens, MakeTokens>(
    doc: &mut DocBuilder<'source>,
    name: &Name<'source>,
    type_parameters: Option<jolt_kotlin_syntax::TypeParameterList<'source>>,
    constructor: &PrimaryConstructor<'source>,
    declaration_source: &'source str,
    declaration_start: usize,
    tokens: MakeTokens,
) -> Option<DeclarationTail<'source>>
where
    Tokens: IntoIterator<Item = KotlinSyntaxToken<'source>>,
    MakeTokens: Fn() -> Tokens + Copy,
{
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
            tokens(),
            expected_open_start.get(),
            open.token_text_range().start().get(),
        ) {
            return None;
        }
        return Some(DeclarationTail {
            doc: format_value_parameter_list(doc, &parameters),
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
        tokens(),
        expected_open_start.get(),
        first_tail_start.get(),
    ) {
        return None;
    }

    let space = doc.space();
    let modifiers = if let Some(modifiers) = modifiers {
        format_inline_modifier_prefix(doc, &modifiers)
    } else {
        doc.nil()
    };
    let constructor_token = if let Some(token) = constructor_token {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let parameters = format_value_parameter_list(doc, &parameters);
    Some(DeclarationTail {
        doc: doc.concat([space, modifiers, constructor_token, parameters]),
    })
}

struct DeclarationTail<'source> {
    doc: Doc<'source>,
}
