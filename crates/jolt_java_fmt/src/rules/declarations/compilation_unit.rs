//! The compilation unit, package and import declarations, and import sorting
//! (N1).

use std::collections::HashMap;

use jolt_fmt::{Doc, ListItem, Separator, cat};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, ImportDeclaration, JavaSyntaxKind as K,
    PackageDeclaration,
};
use jolt_syntax::{AstNode, NodeId, SyntaxElement, SyntaxNode};

use crate::JavaFormatter;

pub(crate) fn compilation_unit<'a>(f: &mut JavaFormatter<'a>, node: CompilationUnit<'a>) -> Doc {
    let items: Vec<CompilationUnitItem<'a>> = node
        .items()
        .map(|list| list.items().collect())
        .unwrap_or_default();
    let mut roles = HashMap::new();
    let ordered = order_items(f, &items, &mut roles);
    let body = f.item_list(ordered, |f, separator| {
        top_level_separator(f, separator, &roles)
    });
    let Some(eof) = node.eof() else {
        return body;
    };
    let eof_element = SyntaxElement::Token(eof);
    let before_eof = if body.is_nil() || !f.has_leading_comments(eof_element) {
        Doc::NIL
    } else if f.blank_line_before(eof_element) {
        f.empty_line()
    } else {
        f.hard_line()
    };
    let eof = f.token(eof);
    cat![f; body, before_eof, eof]
}

pub(crate) fn package_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: PackageDeclaration<'a>,
) -> Doc {
    let mut docs = Vec::new();
    for annotation in node
        .annotations()
        .into_iter()
        .flat_map(jolt_java_syntax::AnnotationList::items)
    {
        docs.push(f.fmt(annotation));
        docs.push(f.hard_line());
    }
    docs.push(f.token_opt(node.package_keyword()));
    docs.push(f.space());
    docs.push(f.fmt_opt(node.name()));
    docs.push(f.token_opt(node.semicolon()));
    f.concat(docs)
}

pub(crate) fn import_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ImportDeclaration<'a>,
) -> Doc {
    let mut docs = vec![f.token_opt(node.import_keyword())];
    for keyword in [node.module_keyword(), node.static_keyword()]
        .into_iter()
        .flatten()
    {
        docs.push(f.space());
        docs.push(f.token(keyword));
    }
    docs.push(f.space());
    docs.push(f.fmt_opt(node.name()));
    docs.push(f.token_opt(node.on_demand_dot()));
    docs.push(f.token_opt(node.star()));
    if let Some(suffix) = node.suffix() {
        docs.push(f.space());
        docs.push(f.fmt(suffix));
    }
    docs.push(f.token_opt(node.semicolon()));
    f.concat(docs)
}

/// How a top-level item is separated from the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// An import in a sorted section: `section` numbers the section, and
    /// `anchor` marks an import that carries leading comments.
    Import {
        section: usize,
        is_static: bool,
        anchor: bool,
    },
    /// A removed `;` whose comments lead the import after it.
    ImportCarrier { section: usize, is_static: bool },
    /// A removed `;` whose comments stand alone; `last` when no item follows
    /// it (its comments then lead the end of the file).
    Carrier { last: bool },
}

fn top_level_separator<'a>(
    f: &mut JavaFormatter<'a>,
    separator: Separator<'a>,
    roles: &HashMap<NodeId, Role>,
) -> Doc {
    let role = |node: SyntaxNode<'_>| roles.get(&node.id()).copied();
    let section = |role: Option<Role>| match role {
        Some(Role::Import { section, .. } | Role::ImportCarrier { section, .. }) => Some(section),
        _ => None,
    };
    let (prev, next) = (role(separator.prev), role(separator.next));
    let blank = separator.blank_line_in_source;
    let line_or_blank = |f: &JavaFormatter<'a>, blank| {
        if blank { f.empty_line() } else { f.hard_line() }
    };
    let is_static = |role: Option<Role>| {
        matches!(
            role,
            Some(
                Role::Import {
                    is_static: true,
                    ..
                } | Role::ImportCarrier {
                    is_static: true,
                    ..
                }
            )
        )
    };
    if section(prev).is_some() && section(prev) == section(next) {
        return match (prev, next) {
            (Some(Role::ImportCarrier { .. }), _) => f.hard_line(),
            // The blank line between the normal and the static imports.
            _ if !is_static(prev) && is_static(next) => f.empty_line(),
            (_, Some(Role::ImportCarrier { .. } | Role::Import { anchor: true, .. })) => {
                line_or_blank(f, blank)
            }
            _ => f.hard_line(),
        };
    }
    // A carrier's comments lead whatever follows it, so it is separated
    // like that item would be.
    match (prev, next) {
        (Some(Role::Carrier { .. }), _) => f.hard_line(),
        (_, Some(Role::Carrier { last: true })) => line_or_blank(f, blank),
        _ => f.empty_line(),
    }
}

/// One import with the comment-carrying `;` items that lead it.
struct Unit<'a> {
    carriers: Vec<SyntaxNode<'a>>,
    import: ImportDeclaration<'a>,
    key: String,
    is_static: bool,
    anchor: bool,
    sortable: bool,
}

/// The items in output order: each run of imports (with the `;` items
/// between them) is grouped and sorted.
fn order_items<'a>(
    f: &JavaFormatter<'a>,
    items: &[CompilationUnitItem<'a>],
    roles: &mut HashMap<NodeId, Role>,
) -> Vec<ListItem<'a>> {
    let mut ordered = Vec::with_capacity(items.len());
    let mut sections = 0;
    let mut index = 0;
    while index < items.len() {
        let item = items[index];
        if let CompilationUnitItem::ImportDeclaration(_) = item {
            let end = section_end(items, index);
            let units = import_units(f, &items[index..end], &mut ordered);
            emit_section(units, sections, &mut ordered, roles);
            sections += 1;
            index = end;
            continue;
        }
        let node = item.syntax();
        if let CompilationUnitItem::EmptyDeclaration(_) = item {
            let last = items[index + 1..]
                .iter()
                .all(|item| matches!(item, CompilationUnitItem::EmptyDeclaration(_)));
            roles.insert(node.id(), Role::Carrier { last });
            ordered.push(ListItem::Elide(node));
        } else {
            ordered.push(ListItem::Format(node));
        }
        index += 1;
    }
    ordered
}

/// The end of the import section starting at `start`: imports and the `;`
/// items between them.
fn section_end(items: &[CompilationUnitItem<'_>], start: usize) -> usize {
    let mut end = start;
    for (index, item) in items.iter().enumerate().skip(start) {
        match item {
            CompilationUnitItem::ImportDeclaration(_) => end = index + 1,
            CompilationUnitItem::EmptyDeclaration(_) => {}
            _ => break,
        }
    }
    end
}

fn import_units<'a>(
    f: &JavaFormatter<'a>,
    items: &[CompilationUnitItem<'a>],
    ordered: &mut Vec<ListItem<'a>>,
) -> Vec<Unit<'a>> {
    let mut units = Vec::new();
    let mut carriers = Vec::new();
    for &item in items {
        match item {
            CompilationUnitItem::ImportDeclaration(import) => {
                let name = import.name().map(AstNode::syntax);
                let anchor = !carriers.is_empty()
                    || f.has_leading_comments(SyntaxElement::Node(import.syntax()));
                units.push(Unit {
                    carriers: std::mem::take(&mut carriers),
                    import,
                    key: import_key(import),
                    is_static: import.static_keyword().is_some(),
                    anchor,
                    sortable: import.syntax().is_recovery_free()
                        && name.is_some_and(|name| !f.has_comments_within(name))
                        && !f.in_ignore_region(import.syntax()),
                });
            }
            _ if f.has_comments_within(item.syntax()) => carriers.push(item.syntax()),
            // A `;` without comments vanishes wherever it is listed.
            _ => ordered.push(ListItem::Elide(item.syntax())),
        }
    }
    units
}

fn import_key(import: ImportDeclaration<'_>) -> String {
    let mut key = String::new();
    if let Some(name) = import.name() {
        for token in name.syntax().tokens() {
            key.push_str(token.text());
        }
    }
    if import.star().is_some() {
        key.push_str(".*");
    }
    key
}

/// Emits a section: barriers (unsortable imports) stay in place and split it
/// into segments; each segment prints its normal imports, then its static
/// ones; within each group an anchor starts a new run and stays first in it,
/// and the rest of the run sorts by name.
fn emit_section<'a>(
    units: Vec<Unit<'a>>,
    section: usize,
    ordered: &mut Vec<ListItem<'a>>,
    roles: &mut HashMap<NodeId, Role>,
) {
    let mut segment = Vec::new();
    for unit in units {
        if unit.sortable {
            segment.push(unit);
        } else {
            emit_segment(std::mem::take(&mut segment), section, ordered, roles);
            emit_unit(&unit, section, ordered, roles);
        }
    }
    emit_segment(segment, section, ordered, roles);
}

fn emit_segment<'a>(
    units: Vec<Unit<'a>>,
    section: usize,
    ordered: &mut Vec<ListItem<'a>>,
    roles: &mut HashMap<NodeId, Role>,
) {
    let (statics, normals): (Vec<_>, Vec<_>) = units.into_iter().partition(|unit| unit.is_static);
    for group in [normals, statics] {
        for mut run in runs(group) {
            let start = usize::from(run[0].anchor);
            run[start..].sort_by(|a, b| a.key.cmp(&b.key));
            for unit in &run {
                emit_unit(unit, section, ordered, roles);
            }
        }
    }
}

/// Splits a group before each anchor.
fn runs(group: Vec<Unit<'_>>) -> Vec<Vec<Unit<'_>>> {
    let mut runs: Vec<Vec<Unit<'_>>> = Vec::new();
    for unit in group {
        match runs.last_mut() {
            Some(run) if !unit.anchor => run.push(unit),
            _ => runs.push(vec![unit]),
        }
    }
    runs
}

fn emit_unit<'a>(
    unit: &Unit<'a>,
    section: usize,
    ordered: &mut Vec<ListItem<'a>>,
    roles: &mut HashMap<NodeId, Role>,
) {
    let is_static = unit.is_static;
    for &carrier in &unit.carriers {
        roles.insert(carrier.id(), Role::ImportCarrier { section, is_static });
        ordered.push(ListItem::Elide(carrier));
    }
    let node = unit.import.syntax();
    roles.insert(
        node.id(),
        Role::Import {
            section,
            is_static,
            anchor: unit.anchor,
        },
    );
    ordered.push(ListItem::Format(node));
    debug_assert_eq!(K::from_raw(node.kind()), K::ImportDeclaration);
}
