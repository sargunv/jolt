//! Module declarations and directives, with directive and `requires`
//! modifier ordering (N3).

use std::collections::HashMap;

use jolt_fmt::{Doc, ListItem, Separator, cat};
use jolt_java_syntax::{
    ExportsDirective, JavaSyntaxKind as K, ModuleDeclaration, ModuleDirective,
    ModuleImplementationClause, ModuleTargetClause, NameSyntax, OpensDirective, ProvidesDirective,
    RequiresDirective, UsesDirective,
};
use jolt_syntax::{AstNode, NodeId, SyntaxElement, SyntaxToken};

use super::types::header_clause;
use crate::JavaFormatter;
use crate::helpers::bodies::braced_body;

/// Annotations one per line, `open module name {`, then the directives:
/// grouped by kind, sorted by name within a group, groups separated by a
/// blank line. A directive that carries leading comments, is not
/// recovery-free, or is formatter-ignored is a barrier: it stays in place,
/// on its own, and the directives on each side sort separately.
pub(crate) fn module_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleDeclaration<'a>,
) -> Doc {
    let mut docs = Vec::new();
    for annotation in node
        .annotations()
        .into_iter()
        .flat_map(jolt_java_syntax::AnnotationList::items)
    {
        docs.extend([f.fmt(annotation), f.hard_line()]);
    }
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
        .flat_map(jolt_java_syntax::ModuleDirectiveList::items)
        .collect();
    let mut groups = HashMap::new();
    let items = order_directives(f, &directives, &mut groups);
    docs.push(braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        items,
        node.close_brace(),
        |f, separator: Separator<'a>| {
            let group = |node: jolt_syntax::SyntaxNode<'_>| groups.get(&node.id());
            if group(separator.prev) == group(separator.next) {
                f.hard_line()
            } else {
                f.empty_line()
            }
        },
    ));
    f.concat(docs)
}

/// The directives in output order. `groups` maps each directive to its
/// group: the segment between barriers and the directive kind.
fn order_directives<'a>(
    f: &JavaFormatter<'a>,
    directives: &[ModuleDirective<'a>],
    groups: &mut HashMap<NodeId, (usize, u8)>,
) -> Vec<ListItem<'a>> {
    let mut ordered = Vec::with_capacity(directives.len());
    let mut segment: Vec<(u8, String, ModuleDirective<'a>)> = Vec::new();
    let mut segments = 0;
    let mut flush = |segment: &mut Vec<(u8, String, ModuleDirective<'a>)>,
                     ordered: &mut Vec<ListItem<'a>>,
                     groups: &mut HashMap<NodeId, (usize, u8)>| {
        segment.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        for (kind, _, directive) in segment.drain(..) {
            groups.insert(directive.syntax().id(), (segments, kind));
            ordered.push(ListItem::Format(directive.syntax()));
        }
        segments += 1;
    };
    for &directive in directives {
        if let Some((kind, key)) = sort_key(f, directive) {
            segment.push((kind, key, directive));
        } else {
            flush(&mut segment, &mut ordered, groups);
            segment.push((0, String::new(), directive));
            flush(&mut segment, &mut ordered, groups);
        }
    }
    flush(&mut segment, &mut ordered, groups);
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

/// The dotted name without whitespace or comments.
fn name_key(name: NameSyntax<'_>) -> String {
    name.syntax().tokens().map(SyntaxToken::text).collect()
}

/// `requires static transitive name;`: the modifiers are ordered `static`
/// first unless one of them carries a comment.
pub(crate) fn requires_directive<'a>(
    f: &mut JavaFormatter<'a>,
    node: RequiresDirective<'a>,
) -> Doc {
    let mut docs = vec![f.token_opt(node.requires_keyword()), f.space()];
    let mut modifiers: Vec<SyntaxToken<'a>> = node
        .modifiers()
        .into_iter()
        .flat_map(jolt_java_syntax::RequiresModifierList::items)
        .collect();
    let sortable = modifiers.iter().all(|&modifier| {
        let element = SyntaxElement::Token(modifier);
        !f.has_leading_comments(element) && !f.has_trailing_comments(element)
    });
    if sortable {
        modifiers.sort_by_key(|modifier| K::from_raw(modifier.kind()) != K::StaticKw);
    }
    for modifier in modifiers {
        docs.extend([f.token(modifier), f.space()]);
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
    let targets = match targets {
        Some(targets) => {
            let targets = f.fmt(targets);
            cat![f; f.space(), targets]
        }
        None => Doc::NIL,
    };
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
    let implementation = match node.implementation() {
        Some(implementation) => {
            let implementation = f.fmt(implementation);
            cat![f; f.space(), implementation]
        }
        None => Doc::NIL,
    };
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
        .flat_map(jolt_java_syntax::ModuleNameList::elements);
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
        .flat_map(jolt_java_syntax::ModuleNameList::elements);
    header_clause(f, node.with_keyword(), names)
}
