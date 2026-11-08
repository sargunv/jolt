//! Module declarations and directives, with directive and `requires`
//! modifier ordering (N3).

use jolt_fmt::{Doc, ListItem, cat};
use jolt_java_syntax::{
    ExportsDirective, JavaSyntaxKind as K, ModuleDeclaration, ModuleDirective, ModuleDirectiveList,
    ModuleImplementationClause, ModuleNameList, ModuleTargetClause, NameSyntax, OpensDirective,
    ProvidesDirective, RequiresDirective, RequiresModifierList, UsesDirective,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxToken};

use super::types::header_clause;
use crate::JavaFormatter;
use crate::helpers::bodies::braced_body;
use crate::helpers::spaced_node;
use crate::rules::types::{annotation_lines, name_key};

/// Annotations one per line, `open module name {`, then the directives:
/// grouped by kind, sorted by name within a group, groups separated by a
/// blank line. A directive that carries leading comments, is not
/// recovery-free, or is formatter-ignored is a barrier: it stays in place,
/// on its own, and the directives on each side sort separately.
pub(crate) fn module_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleDeclaration<'a>,
) -> Doc {
    let mut docs = vec![annotation_lines(f, node.annotations())];
    if let Some(open) = node.open_keyword() {
        docs.extend([f.token(open), f.space()]);
    }
    docs.extend([
        f.token_opt(node.module_keyword()),
        f.space(),
        f.fmt_opt(node.name()),
        f.space(),
    ]);
    let directives: Vec<_> = node
        .directives()
        .into_iter()
        .flat_map(ModuleDirectiveList::items)
        .collect();
    let ordered = order_directives(f, &directives);
    let items = ordered
        .iter()
        .map(|&(_, directive)| ListItem::Format(directive.syntax()));
    let group = |index: usize| {
        let (segment, kind, _) = ordered[index].0;
        (segment, kind)
    };
    docs.push(braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        items,
        node.close_brace(),
        |f, separator| f.line_break(group(separator.prev_index) != group(separator.next_index)),
    ));
    f.concat(docs)
}

/// The directives in output order, each with its sort key: the segment
/// between barriers (a barrier is a segment of its own), the directive kind
/// and the name.
fn order_directives<'a>(
    f: &JavaFormatter<'a>,
    directives: &[ModuleDirective<'a>],
) -> Vec<((usize, u8, String), ModuleDirective<'a>)> {
    let mut segment = 0;
    let mut ordered: Vec<_> = directives
        .iter()
        .map(|&directive| {
            let key = if let Some((kind, name)) = sort_key(f, directive) {
                (segment, kind, name)
            } else {
                let key = (segment + 1, 0, String::new());
                segment += 2;
                key
            };
            (key, directive)
        })
        .collect();
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    ordered
}

/// The directive's kind rank and name, or `None` for a barrier.
fn sort_key<'a>(f: &JavaFormatter<'a>, directive: ModuleDirective<'a>) -> Option<(u8, String)> {
    let syntax = directive.syntax();
    if !syntax.is_recovery_free()
        || f.in_ignore_region(syntax)
        || f.has_leading_comments(SyntaxElement::Node(syntax))
    {
        return None;
    }
    let (kind, name) = match directive {
        ModuleDirective::RequiresDirective(node) => (0, node.module()),
        ModuleDirective::ExportsDirective(node) => (1, node.package()),
        ModuleDirective::OpensDirective(node) => (2, node.package()),
        ModuleDirective::UsesDirective(node) => (3, node.service()),
        ModuleDirective::ProvidesDirective(node) => (4, node.service()),
        ModuleDirective::Bogus(_) => return None,
    };
    Some((kind, name_key(name?)))
}

/// `requires static transitive name;`: the modifiers are ordered `static`
/// first unless one of them carries a comment.
pub(crate) fn requires_directive<'a>(
    f: &mut JavaFormatter<'a>,
    node: RequiresDirective<'a>,
) -> Doc {
    let mut docs = vec![f.token_opt(node.requires_keyword()), f.space()];
    let mut modifiers: Vec<_> = node
        .modifiers()
        .into_iter()
        .flat_map(RequiresModifierList::elements)
        .collect();
    // N3 sorts only a recovery-free directive: in a recovered one the order
    // can decide which word is the module name on the next pass.
    let sortable = node.syntax().is_recovery_free()
        && modifiers.iter().all(|&modifier| match modifier {
            ListElement::Item(token) => !f.has_comments(SyntaxElement::Token(token)),
            _ => false,
        });
    if sortable {
        modifiers.sort_by_key(|modifier| {
            !matches!(modifier, ListElement::Item(token) if K::from_raw(token.kind()) == K::StaticKw)
        });
    }
    for modifier in modifiers {
        let doc = match modifier {
            ListElement::Item(token) => f.token(token),
            ListElement::Bogus(bogus) => f.node(bogus),
            ListElement::Separator(_) | ListElement::Missing => continue,
        };
        docs.extend([doc, f.space()]);
    }
    docs.push(f.fmt_opt(node.module()));
    docs.push(f.token_opt(node.semicolon()));
    f.concat(docs)
}

pub(crate) fn exports_directive<'a>(f: &mut JavaFormatter<'a>, node: ExportsDirective<'a>) -> Doc {
    package_directive(
        f,
        node.exports_keyword(),
        node.package(),
        node.targets(),
        node.semicolon(),
    )
}

pub(crate) fn opens_directive<'a>(f: &mut JavaFormatter<'a>, node: OpensDirective<'a>) -> Doc {
    package_directive(
        f,
        node.opens_keyword(),
        node.package(),
        node.targets(),
        node.semicolon(),
    )
}

/// `keyword package to a, b;`.
fn package_directive<'a>(
    f: &mut JavaFormatter<'a>,
    keyword: Option<SyntaxToken<'a>>,
    package: Option<NameSyntax<'a>>,
    targets: Option<ModuleTargetClause<'a>>,
    semicolon: Option<SyntaxToken<'a>>,
) -> Doc {
    let keyword = f.token_opt(keyword);
    let package = f.fmt_opt(package);
    let targets = spaced_node(f, targets);
    let semicolon = f.token_opt(semicolon);
    cat![f; keyword, f.space(), package, targets, semicolon]
}

pub(crate) fn uses_directive<'a>(f: &mut JavaFormatter<'a>, node: UsesDirective<'a>) -> Doc {
    let keyword = f.token_opt(node.uses_keyword());
    let service = f.fmt_opt(node.service());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; keyword, f.space(), service, semicolon]
}

/// `provides service with a, b;`.
pub(crate) fn provides_directive<'a>(
    f: &mut JavaFormatter<'a>,
    node: ProvidesDirective<'a>,
) -> Doc {
    let keyword = f.token_opt(node.provides_keyword());
    let service = f.fmt_opt(node.service());
    let implementation = spaced_node(f, node.implementation());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; keyword, f.space(), service, implementation, semicolon]
}

/// `to a, b`: on the directive's line if it fits, else the names one per
/// line one indent deeper.
pub(crate) fn module_target_clause<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleTargetClause<'a>,
) -> Doc {
    let names = node
        .targets()
        .into_iter()
        .flat_map(ModuleNameList::elements);
    header_clause(f, node.to_keyword(), names)
}

/// `with a, b`, laid out like `to`.
pub(crate) fn module_implementation_clause<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleImplementationClause<'a>,
) -> Doc {
    let names = node
        .implementations()
        .into_iter()
        .flat_map(ModuleNameList::elements);
    header_clause(f, node.with_keyword(), names)
}
