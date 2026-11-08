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
    // The items after the last formatted one are removed `;`s, whose
    // comments lead the end of the file.
    let last_content = items
        .iter()
        .rposition(|item| matches!(item, ListItem::Format(_)))
        .unwrap_or(0);
    let body = f.item_list(items, |f, separator| {
        let at_end = separator.next_index > last_content;
        top_level_separator(
            f,
            separator,
            roles[separator.prev_index],
            roles[separator.next_index],
            at_end,
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

/// An import in a sorted section, or a removed `;` whose comments lead it
/// (a carrier): `section` numbers the section, and `anchor` marks an import
/// that carries leading comments or carriers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImportRole {
    section: usize,
    is_static: bool,
    anchor: bool,
}

/// The top-level items in output order, each with its import role (`None`
/// for an item that is not an import or a carrier).
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
        let role = Some(ImportRole {
            section,
            is_static: unit.is_static,
            anchor: unit.anchor,
        });
        for &carrier in &unit.carriers {
            self.push(ListItem::Elide(carrier), role);
        }
        self.push(ListItem::Format(unit.import.syntax()), role);
    }
}

fn top_level_separator<'a>(
    f: &mut JavaFormatter<'a>,
    separator: Separator<'a>,
    prev: Option<ImportRole>,
    next: Option<ImportRole>,
    at_end: bool,
) -> Doc {
    let blank = separator.blank_line_in_source;
    if let (Some(prev), Some(next)) = (prev, next)
        && prev.section == next.section
    {
        return if is_semicolon(separator.prev) {
            // A carrier and the import it leads.
            f.hard_line()
        } else if !prev.is_static && next.is_static {
            // The blank line between the normal and the static imports.
            f.empty_line()
        } else if next.anchor {
            f.line_break(blank)
        } else {
            f.hard_line()
        };
    }
    // The comments of a removed `;` keep the source's line breaks after
    // them, and before them at the end of the file. Two fields of a compact
    // source file keep the source's choice, as in a class body.
    let is_field = |node: SyntaxNode<'_>| K::from_raw(node.kind()) == K::FieldDeclaration;
    if is_semicolon(separator.prev)
        || at_end
        || (is_field(separator.prev) && is_field(separator.next))
    {
        f.line_break(blank)
    } else {
        f.empty_line()
    }
}

fn is_semicolon(node: SyntaxNode<'_>) -> bool {
    K::from_raw(node.kind()) == K::EmptyDeclaration
}

/// One import of a section, with the `;` items before it, whose comments
/// lead it.
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
            let end = section_end(items, index);
            let units = import_units(f, &items[index..end]);
            ordered.import_section(units);
            index = end;
            continue;
        }
        let node = item.syntax();
        if is_semicolon(node) {
            ordered.push(ListItem::Elide(node), None);
        } else {
            ordered.push(ListItem::Format(node), None);
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

/// The imports of a section, each with the `;` items before it.
fn import_units<'a>(f: &JavaFormatter<'a>, items: &[CompilationUnitItem<'a>]) -> Vec<Unit<'a>> {
    let mut units = Vec::new();
    let mut start = 0;
    for (index, &item) in items.iter().enumerate() {
        let CompilationUnitItem::ImportDeclaration(import) = item else {
            continue;
        };
        let carriers: Vec<SyntaxNode<'a>> = items[start..index]
            .iter()
            .map(|item| item.syntax())
            .collect();
        start = index + 1;
        let anchor = carriers
            .iter()
            .any(|&carrier| f.has_comments_within(carrier))
            || f.has_leading_comments(SyntaxElement::Node(import.syntax()));
        let name = import.name();
        let mut key = name.map(name_key).unwrap_or_default();
        if import.star().is_some() {
            key.push_str(".*");
        }
        units.push(Unit {
            carriers,
            import,
            key,
            is_static: import.static_keyword().is_some(),
            anchor,
            sortable: import.syntax().is_recovery_free()
                && name.is_some_and(|name| !f.has_comments_within(name.syntax()))
                && !f.in_ignore_region(import.syntax()),
        });
    }
    units
}
