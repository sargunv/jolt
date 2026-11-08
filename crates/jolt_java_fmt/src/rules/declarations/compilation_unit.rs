//! The compilation unit, package and import declarations, and import sorting
//! (N1).

use jolt_fmt::{Doc, ListItem, Separator, cat};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, ImportDeclaration, JavaSyntaxKind as K,
    PackageDeclaration,
};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode};

use crate::JavaFormatter;
use crate::helpers::spaced_node;
use crate::rules::types::{annotation_lines, name_key};

pub(crate) fn compilation_unit<'a>(f: &mut JavaFormatter<'a>, node: CompilationUnit<'a>) -> Doc {
    let items: Vec<CompilationUnitItem<'a>> = node
        .items()
        .map(|list| list.items().collect())
        .unwrap_or_default();
    let Ordered { items, roles, .. } = order_items(f, &items);
    let body = f.item_list(items, |f, separator| {
        top_level_separator(
            f,
            separator,
            roles[separator.prev_index],
            roles[separator.next_index],
        )
    });
    let Some(eof) = node.eof() else {
        return body;
    };
    let eof_element = SyntaxElement::Token(eof);
    let before_eof = if body.is_nil() || !f.has_leading_comments(eof_element) {
        Doc::NIL
    } else {
        f.line_break(f.blank_line_before(eof_element))
    };
    let eof = f.token(eof);
    cat![f; body, before_eof, eof]
}

pub(crate) fn package_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: PackageDeclaration<'a>,
) -> Doc {
    let mut docs = vec![annotation_lines(f, node.annotations())];
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
    docs.push(spaced_node(f, node.suffix()));
    docs.push(f.token_opt(node.semicolon()));
    f.concat(docs)
}

/// An import in a sorted section: `section` numbers the section, and
/// `anchor` marks an import that carries leading comments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImportRole {
    section: usize,
    is_static: bool,
    anchor: bool,
}

/// The top-level items in output order, each with its import role (`None`
/// for an item that is not an import).
struct Ordered<'a> {
    items: Vec<ListItem<'a>>,
    roles: Vec<Option<ImportRole>>,
    /// The number of import sections so far.
    sections: usize,
}

impl<'a> Ordered<'a> {
    fn push(&mut self, item: ListItem<'a>, role: Option<ImportRole>) {
        self.items.push(item);
        self.roles.push(role);
    }

    /// Emits an import section: barriers (unsortable imports) stay in place
    /// and split it into segments; each segment prints its normal imports,
    /// then its static ones; within each group an anchor starts a new run and
    /// stays first in it, and the rest of the run sorts by name.
    fn import_section(&mut self, units: Vec<Unit<'a>>) {
        let section = self.sections;
        self.sections += 1;
        let mut segment = Vec::new();
        for unit in units {
            if unit.sortable {
                segment.push(unit);
            } else {
                self.import_segment(std::mem::take(&mut segment), section);
                self.import_unit(&unit, section);
            }
        }
        self.import_segment(segment, section);
    }

    fn import_segment(&mut self, units: Vec<Unit<'a>>, section: usize) {
        let (statics, normals): (Vec<_>, Vec<_>) =
            units.into_iter().partition(|unit| unit.is_static);
        for mut group in [normals, statics] {
            for run in group.chunk_by_mut(|_, unit| !unit.anchor) {
                let start = usize::from(run[0].anchor);
                run[start..].sort_by(|a, b| a.key.cmp(&b.key));
            }
            for unit in &group {
                self.import_unit(unit, section);
            }
        }
    }

    fn import_unit(&mut self, unit: &Unit<'a>, section: usize) {
        let role = ImportRole {
            section,
            is_static: unit.is_static,
            anchor: unit.anchor,
        };
        self.push(ListItem::Format(unit.import.syntax()), Some(role));
    }
}

fn top_level_separator<'a>(
    f: &mut JavaFormatter<'a>,
    separator: Separator<'a>,
    prev: Option<ImportRole>,
    next: Option<ImportRole>,
) -> Doc {
    let blank = separator.blank_line_in_source;
    if let (Some(prev), Some(next)) = (prev, next)
        && prev.section == next.section
    {
        return if !prev.is_static && next.is_static {
            // The blank line between the normal and the static imports.
            f.empty_line()
        } else if next.anchor {
            f.line_break(blank)
        } else {
            f.hard_line()
        };
    }
    // Two fields of a compact source file keep the source's choice, as in a
    // class body, and so does a `;` kept for its comments.
    let kind = |node: SyntaxNode<'_>| K::from_raw(node.kind());
    let (prev, next) = (kind(separator.prev), kind(separator.next));
    if (prev, next) == (K::FieldDeclaration, K::FieldDeclaration)
        || prev == K::EmptyDeclaration
        || next == K::EmptyDeclaration
    {
        f.line_break(blank)
    } else {
        f.empty_line()
    }
}

/// One import of a section.
struct Unit<'a> {
    import: ImportDeclaration<'a>,
    key: String,
    is_static: bool,
    anchor: bool,
    sortable: bool,
}

/// A `;` that N5 removes: one without comments.
fn removed_semicolon(f: &JavaFormatter<'_>, item: CompilationUnitItem<'_>) -> bool {
    matches!(item, CompilationUnitItem::EmptyDeclaration(_))
        && !f.has_comments_within(item.syntax())
}

/// The items in output order: each run of imports (with the removed `;`
/// items between them) is grouped and sorted.
fn order_items<'a>(f: &JavaFormatter<'a>, items: &[CompilationUnitItem<'a>]) -> Ordered<'a> {
    let mut ordered = Ordered {
        items: Vec::with_capacity(items.len()),
        roles: Vec::with_capacity(items.len()),
        sections: 0,
    };
    let mut index = 0;
    while index < items.len() {
        let item = items[index];
        if let CompilationUnitItem::ImportDeclaration(_) = item {
            let end = section_end(f, items, index);
            let units = import_units(f, &items[index..end], &mut ordered);
            ordered.import_section(units);
            index = end;
            continue;
        }
        let node = item.syntax();
        if removed_semicolon(f, item) {
            ordered.push(ListItem::Elide(node), None);
        } else {
            ordered.push(ListItem::Format(node), None);
        }
        index += 1;
    }
    ordered
}

/// The end of the import section starting at `start`: imports and the
/// removed `;` items between them.
fn section_end(f: &JavaFormatter<'_>, items: &[CompilationUnitItem<'_>], start: usize) -> usize {
    let mut end = start;
    for (index, &item) in items.iter().enumerate().skip(start) {
        match item {
            CompilationUnitItem::ImportDeclaration(_) => end = index + 1,
            _ if removed_semicolon(f, item) => {}
            _ => break,
        }
    }
    end
}

/// The imports of a section. The removed `;` items between them are listed
/// first, since they print nothing.
fn import_units<'a>(
    f: &JavaFormatter<'a>,
    items: &[CompilationUnitItem<'a>],
    ordered: &mut Ordered<'a>,
) -> Vec<Unit<'a>> {
    let mut units = Vec::new();
    for &item in items {
        let CompilationUnitItem::ImportDeclaration(import) = item else {
            ordered.push(ListItem::Elide(item.syntax()), None);
            continue;
        };
        let name = import.name();
        let mut key = name.map(name_key).unwrap_or_default();
        if import.star().is_some() {
            key.push_str(".*");
        }
        units.push(Unit {
            import,
            key,
            is_static: import.static_keyword().is_some(),
            anchor: f.has_leading_comments(SyntaxElement::Node(import.syntax())),
            sortable: import.syntax().is_recovery_free()
                && name.is_some_and(|name| !f.has_comments_within(name.syntax()))
                && !f.in_ignore_region(import.syntax()),
        });
    }
    units
}
