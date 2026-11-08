//! Try statements, catch and finally clauses, and resources.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    CatchClause, CatchClauseList, CatchParameter, CatchTypeList, FinallyClause, Resource,
    ResourceList, ResourceSpecification, ResourceVariableDeclaration, ResourceVariableType,
    TryStatement, TryWithResourcesStatement, VariableAccess,
};
use jolt_syntax::{AstNode, SyntaxElement};

use crate::JavaFormatter;
use crate::helpers::lists::{joined, list_item, parenthesized};
use crate::helpers::spaced;
use crate::rules::types::inline_modifiers;

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
    for clause in catches.into_iter().flat_map(CatchClauseList::elements) {
        docs.extend([f.space(), list_item(f, clause)]);
    }
    let finally = f.fmt_opt(finally);
    docs.push(spaced(f, finally));
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
    let modifiers = inline_modifiers(f, node.modifiers());
    let types = f.fmt_opt(node.types());
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; modifiers, types, f.space(), name, dimensions]
}

pub(crate) fn catch_type_list<'a>(f: &mut JavaFormatter<'a>, node: CatchTypeList<'a>) -> Doc {
    f.fmt_opt(node.types())
}

pub(crate) fn finally_clause<'a>(f: &mut JavaFormatter<'a>, node: FinallyClause<'a>) -> Doc {
    let keyword = f.token_opt(node.finally_keyword());
    let body = f.fmt_opt(node.body());
    cat![f; keyword, f.space(), body]
}

/// `(a; b)` that breaks like a call: one resource per line ending in `;`
/// except the last. A trailing `;` after the last resource is removed (N5);
/// its comments keep their own line before `)`.
pub(crate) fn resource_specification<'a>(
    f: &mut JavaFormatter<'a>,
    node: ResourceSpecification<'a>,
) -> Doc {
    let resources = node
        .resources()
        .into_iter()
        .flat_map(ResourceList::elements);
    let line = f.line();
    let mut docs = vec![joined(f, resources, line)];
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
    parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        inner,
        node.close_paren(),
    )
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
    let modifiers = inline_modifiers(f, node.modifiers());
    let ty = f.element_opt(node.r#type().map(ResourceVariableType::element));
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
