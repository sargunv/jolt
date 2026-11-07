//! Try statements, catch and finally clauses, and resources.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{
    CatchClause, CatchClauseList, CatchParameter, CatchTypeList, FinallyClause, ParameterModifier,
    ParameterModifierList, Resource, ResourceList, ResourceSpecification,
    ResourceVariableDeclaration, ResourceVariableType, TryStatement, TryWithResourcesStatement,
    VariableAccess,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement};

use super::{parenthesized, with_dangling};
use crate::JavaFormatter;

/// `try {` … `} catch (…) {` … `} finally {` … `}`.
pub(crate) fn try_statement<'a>(f: &mut JavaFormatter<'a>, node: TryStatement<'a>) -> Doc {
    let keyword = f.token_opt(node.try_keyword());
    let body = f.fmt_opt(node.body());
    let clauses = clauses(f, node.catches(), node.finally());
    cat![f; keyword, f.space(), body, clauses]
}

/// `try (` resources `) {` … with the same clauses as `try`.
pub(crate) fn try_with_resources_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: TryWithResourcesStatement<'a>,
) -> Doc {
    let keyword = f.token_opt(node.try_keyword());
    let resources = f.fmt_opt(node.resources());
    let body = f.fmt_opt(node.body());
    let clauses = clauses(f, node.catches(), node.finally());
    cat![f; keyword, f.space(), resources, f.space(), body, clauses]
}

/// The catch clauses and the finally clause, each after a space on the line
/// of the preceding `}`.
fn clauses<'a>(
    f: &mut JavaFormatter<'a>,
    catches: Option<CatchClauseList<'a>>,
    finally: Option<FinallyClause<'a>>,
) -> Doc {
    let mut docs = Vec::new();
    let catches = catches.into_iter().flat_map(|catches| {
        catches
            .syntax()
            .children()
            .filter_map(SyntaxElement::as_node)
    });
    for clause in catches {
        docs.extend([f.space(), f.node(clause)]);
    }
    if let Some(finally) = finally {
        docs.extend([f.space(), f.fmt(finally)]);
    }
    f.concat(docs)
}

pub(crate) fn catch_clause<'a>(f: &mut JavaFormatter<'a>, node: CatchClause<'a>) -> Doc {
    let keyword = f.token_opt(node.catch_keyword());
    let parameter = f.fmt_opt(node.parameter());
    let parameter = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        parameter,
        node.close_paren(),
    );
    let body = f.fmt_opt(node.body());
    cat![f; keyword, f.space(), parameter, f.space(), body]
}

/// `final @A Type | Other name`, modifiers inline.
pub(crate) fn catch_parameter<'a>(f: &mut JavaFormatter<'a>, node: CatchParameter<'a>) -> Doc {
    let modifiers = node
        .modifiers()
        .map_or(Doc::NIL, |modifiers| inline_modifiers(f, modifiers));
    let types = f.fmt_opt(node.types());
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; modifiers, types, f.space(), name, dimensions]
}

/// REQUEST(shared): a private copy of `types::inline_modifiers`, which the
/// types area does not export. Every annotation and keyword on one line,
/// each followed by a space.
fn inline_modifiers<'a>(f: &mut JavaFormatter<'a>, node: ParameterModifierList<'a>) -> Doc {
    let mut docs = Vec::new();
    for modifier in node.items() {
        docs.push(match modifier {
            ParameterModifier::Annotation(annotation) => f.fmt(annotation),
            ParameterModifier::Bogus(bogus) => f.fmt(bogus),
            ParameterModifier::Token(token) => f.token(token),
        });
        docs.push(f.space());
    }
    f.concat(docs)
}

pub(crate) fn catch_type_list<'a>(f: &mut JavaFormatter<'a>, node: CatchTypeList<'a>) -> Doc {
    f.fmt_opt(node.types())
}

pub(crate) fn finally_clause<'a>(f: &mut JavaFormatter<'a>, node: FinallyClause<'a>) -> Doc {
    let keyword = f.token_opt(node.finally_keyword());
    let body = f.fmt_opt(node.body());
    cat![f; keyword, f.space(), body]
}

/// Always broken: `(`, one resource per line ending in `;` except the last,
/// `)`. A trailing `;` after the last resource is removed (N5); its comments
/// keep their own line before `)`.
pub(crate) fn resource_specification<'a>(
    f: &mut JavaFormatter<'a>,
    node: ResourceSpecification<'a>,
) -> Doc {
    let open = f.token_opt(node.open_paren());
    let mut docs = Vec::new();
    for element in node
        .resources()
        .into_iter()
        .flat_map(ResourceList::elements)
    {
        match element {
            ListElement::Item(resource) => docs.push(f.fmt(resource)),
            ListElement::Bogus(bogus) => docs.push(f.node(bogus)),
            ListElement::Separator(semicolon) => docs.extend([f.token(semicolon), f.hard_line()]),
            ListElement::Missing => {}
        }
    }
    if let Some(semicolon) = node.trailing_semicolon() {
        if node.syntax().is_recovery_free() {
            let removed = f.removed(SyntaxElement::Token(semicolon));
            if !removed.is_nil() {
                docs.extend([f.hard_line(), removed]);
            }
        } else {
            docs.push(f.token(semicolon));
        }
    }
    let inner = f.concat(docs);
    let inner = with_dangling(f, node.syntax(), inner);
    let inner = indent![f; f.hard_line(), inner];
    let close = f.token_opt(node.close_paren());
    cat![f; open, inner, f.hard_line(), close]
}

pub(crate) fn resource<'a>(f: &mut JavaFormatter<'a>, node: Resource<'a>) -> Doc {
    f.fmt_opt(node.value())
}

/// `modifiers Type name = initializer`; the initializer never breaks after
/// `=`.
pub(crate) fn resource_variable_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ResourceVariableDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = match node.r#type() {
        Some(ResourceVariableType::Type(ty)) => f.fmt(ty),
        Some(ResourceVariableType::Token(token)) => f.token(token),
        None => Doc::NIL,
    };
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    let assign = f.token_opt(node.assign());
    let initializer = f.fmt_opt(node.initializer());
    cat![
        f;
        modifiers,
        ty,
        f.space(),
        name,
        dimensions,
        f.space(),
        assign,
        f.space(),
        initializer,
    ]
}

pub(crate) fn variable_access<'a>(f: &mut JavaFormatter<'a>, node: VariableAccess<'a>) -> Doc {
    f.fmt_opt(node.expression())
}
