use jolt_fmt_ir::{BodyItemSeparator, Doc, DocBuilder};
use jolt_java_syntax::{ImportDeclaration, JavaSyntaxView, NameSyntax, ReorderClaim};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use crate::rules::names::{NameSortKey, format_name};

/// One import in a compilation unit's sorting batch, with the comments a
/// removed redundant separator directly ahead of it carried. Those comments
/// read as this import's own leading trivia, so they anchor a sorting run
/// exactly as if the separator had never been written.
pub(crate) struct ImportBatchEntry<'source> {
    pub(crate) declaration: ImportDeclaration<'source>,
    pub(crate) salvaged_leading: Option<Doc<'source>>,
    pub(crate) blank_before: bool,
}

pub(crate) fn format_imports<'source>(
    imports: Vec<ImportBatchEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    if imports.is_empty() {
        return None;
    }

    let mut sections = Vec::new();
    let mut pending = PendingImports::default();
    for entry in imports {
        let blank_before = entry.blank_before;
        let declaration = entry.declaration;
        let Some(mut import) = FormattedImport::new(declaration) else {
            pending.flush(&mut sections, doc);
            if let Some(salvaged) = entry.salvaged_leading {
                // The import cannot prove it is sortable, so the salvaged
                // comments stay a barrier ahead of it instead of traveling.
                sections.push(ImportSection {
                    doc: salvaged,
                    blank_before,
                });
            }
            sections.push(ImportSection {
                doc: format_import_in_place(&declaration, doc),
                blank_before,
            });
            continue;
        };
        import.salvaged_leading = entry.salvaged_leading;
        pending.push(import, blank_before);
    }
    pending.flush(&mut sections, doc);
    Some(join_import_sections(sections, doc))
}

/// One block of imports emitted together, with the gap that precedes it.
///
/// `blank_before` says whether a blank line separates this block from the one
/// before it: either because the source put one there, or because the block is
/// the static group this formatter split off from the normal group.
struct ImportSection<'source> {
    doc: Doc<'source>,
    blank_before: bool,
}

fn join_import_sections<'source>(
    sections: Vec<ImportSection<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|joined| {
        for section in sections {
            if !joined.is_empty() {
                let separator = BodyItemSeparator::between(section.blank_before).doc(joined);
                joined.push(separator);
            }
            joined.push(section.doc);
        }
    })
}

/// One batch of imports between unsortable barriers: a normal group and a
/// static group, each a list of runs. An import carrying leading comments
/// anchors the run below it within its own group only; the other group sorts
/// as if the comment were not there. Carried comments are the import's own
/// leading trivia and the comments salvaged from a removed redundant
/// separator directly ahead of it; both anchor the same way, so deleting
/// that separator by hand cannot change the order a later pass computes. The
/// anchor holds the boundary position its comment dictates and never joins
/// the key sort, so sorting never carries a comment across another import of
/// its group; only the normal/static regroup, which is global to the batch,
/// may move an import past a comment. Every other import key-sorts within
/// its run. That ordering is a fixpoint, so formatting is idempotent
/// regardless of the source's import order.
#[derive(Default)]
struct PendingImports<'source> {
    normal: GroupRuns<'source>,
    static_: GroupRuns<'source>,
    first_blank_before: bool,
}

/// The runs of one group (normal or static), split at that group's anchors.
#[derive(Default)]
struct GroupRuns<'source> {
    runs: Vec<GroupRun<'source>>,
}

struct GroupRun<'source> {
    /// The comment-carrying import that started this run, if any. It prints
    /// ahead of the run's sorted imports instead of joining the sort.
    anchor: Option<FormattedImport<'source>>,
    imports: Vec<FormattedImport<'source>>,
    blank_before: bool,
}

impl<'source> GroupRun<'source> {
    fn has_imports(&self) -> bool {
        self.anchor.is_some() || !self.imports.is_empty()
    }

    /// The run's imports in emission order: the anchor first, then the
    /// key-sorted rest.
    fn into_imports(self) -> Vec<FormattedImport<'source>> {
        let mut items = Vec::with_capacity(self.imports.len() + 1);
        if let Some(anchor) = self.anchor {
            items.push(anchor);
        }
        items.extend(self.imports);
        items
    }
}

impl<'source> GroupRuns<'source> {
    fn push(&mut self, import: FormattedImport<'source>, blank_before: bool) {
        if self.runs.is_empty() {
            self.runs.push(GroupRun {
                anchor: None,
                imports: Vec::new(),
                blank_before,
            });
        }
        if import.carries_comments() && self.runs.last().is_some_and(GroupRun::has_imports) {
            self.runs.push(GroupRun {
                anchor: None,
                imports: Vec::new(),
                blank_before,
            });
        }
        let run = self.runs.last_mut().expect("a run always exists");
        if import.carries_comments() {
            run.anchor = Some(import);
        } else {
            run.imports.push(import);
        }
    }

    /// Emits the group's sections in run order and reports whether any were
    /// emitted. The first section of the batch takes `first_blank`; the
    /// group's own first section additionally takes `first_blank_before`
    /// (the static group's owned blank line) when it does not open the batch.
    fn flush(
        &mut self,
        sections: &mut Vec<ImportSection<'source>>,
        doc: &mut DocBuilder<'source>,
        first_section: bool,
        first_blank: bool,
        first_blank_before: bool,
    ) -> bool {
        // Each batch has `r <= represented tokens`. Stable sorting therefore
        // costs O(r log r) time and O(r) scratch, with no layout search or
        // cloning of parser-owned source or syntax buffers.
        let mut emitted = false;
        for run in std::mem::take(&mut self.runs) {
            let blank_before = run.blank_before;
            let mut imports = run.imports;
            imports.sort_by(|left, right| left.key.cmp(&right.key));
            let imports = GroupRun {
                anchor: run.anchor,
                imports,
                blank_before,
            }
            .into_imports();
            sections.push(ImportSection {
                doc: format_import_list(imports, doc),
                blank_before: if emitted {
                    blank_before
                } else if first_section {
                    first_blank
                } else {
                    first_blank_before || blank_before
                },
            });
            emitted = true;
        }
        emitted
    }
}

impl<'source> PendingImports<'source> {
    fn push(&mut self, import: FormattedImport<'source>, blank_before: bool) {
        if self.normal.runs.is_empty() && self.static_.runs.is_empty() {
            self.first_blank_before = blank_before;
        }
        if import.is_static {
            self.static_.push(import, blank_before);
        } else {
            self.normal.push(import, blank_before);
        }
    }

    fn flush(&mut self, sections: &mut Vec<ImportSection<'source>>, doc: &mut DocBuilder<'source>) {
        if self.normal.runs.is_empty() && self.static_.runs.is_empty() {
            return;
        }
        let first_blank = self.first_blank_before;
        let has_normals = self.normal.flush(sections, doc, true, first_blank, false);
        // Grouping static imports apart is this formatter's own split, so it
        // owns the blank line that marks it.
        self.static_
            .flush(sections, doc, !has_normals, first_blank, true);
    }
}

fn format_import_list<'source>(
    imports: Vec<FormattedImport<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for import in imports {
            if !docs.is_empty() {
                let line = docs.hard_line();
                docs.push(line);
            }
            let import = import.into_doc(docs);
            docs.push(import);
        }
    })
}

struct FormattedImport<'source> {
    import: ImportDeclaration<'source>,
    reorder: ReorderClaim<'source>,
    key: NameSortKey<'source>,
    is_static: bool,
    salvaged_leading: Option<Doc<'source>>,
}

impl<'source> FormattedImport<'source> {
    fn new(import: ImportDeclaration<'source>) -> Option<Self> {
        use jolt_java_syntax::JavaSyntaxField as Field;

        if !matches!(import.import_keyword(), Field::Present(_))
            || !matches!(
                import.module_keyword(),
                Field::Present(_) | Field::Missing(_)
            )
            || !matches!(
                import.static_keyword(),
                Field::Present(_) | Field::Missing(_)
            )
            || !matches!(
                import.on_demand_dot(),
                Field::Present(_) | Field::Missing(_)
            )
            || !matches!(import.star(), Field::Present(_) | Field::Missing(_))
            || !matches!(import.semicolon(), Field::Present(_))
        {
            return None;
        }
        let name = match import.name() {
            Field::Present(name) if name.is_recovery_free() => name,
            Field::Present(_) | Field::Missing(_) | Field::Malformed(_) => return None,
        };
        let on_demand = matches!(import.star(), Field::Present(_));
        let key = NameSortKey::new(&name, on_demand)?;
        let is_static = matches!(import.static_keyword(), Field::Present(_));
        let reorder = import.canonical_reorder_claim()?;
        Some(Self {
            import,
            reorder,
            key,
            is_static,
            salvaged_leading: None,
        })
    }

    /// Whether this import carries leading comments — its own leading trivia
    /// or comments salvaged from a removed redundant separator directly ahead
    /// of it — and therefore anchors a sorting run.
    fn carries_comments(&self) -> bool {
        self.salvaged_leading.is_some()
            || self
                .import
                .first_token()
                .is_some_and(|token| !token.leading_comments().is_empty())
    }

    #[allow(clippy::redundant_closure_for_method_calls)]
    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        let Self {
            import,
            reorder,
            salvaged_leading,
            ..
        } = self;
        let formatted = format_with_relocated_boundary_comments(&import, salvaged_leading, doc);
        doc.reordered_source(formatted, reorder)
    }
}

/// Relocates boundary comments together with an import that has proved it is
/// recovery-free and therefore eligible for sorting. Salvaged leading
/// comments come from a removed redundant separator directly ahead of the
/// import and print ahead of its own leading comments, matching the trivia
/// the import would carry had the separator never been written.
fn format_with_relocated_boundary_comments<'source>(
    import: &ImportDeclaration<'source>,
    salvaged_leading: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let first = import.first_token();
    let last = import.last_token();
    let leading = doc.concat_list(|comments| {
        if let Some(token) = first.as_ref() {
            for comment in token.leading_comments() {
                if !comments.is_empty() {
                    let line = comments.hard_line();
                    comments.push(line);
                }
                let formatted = format_comment(comments, &comment);
                comments.push(formatted);
            }
        }
    });
    let keyword = format_required_field(import.import_keyword(), doc, |token, doc| {
        doc_concat!(
            doc,
            [
                format_token_after_relocated_leading_comments(
                    doc,
                    &token,
                    TrailingTrivia::Preserve,
                ),
                doc.space(),
            ]
        )
    });
    let semicolon = format_required_field(import.semicolon(), doc, |token, doc| {
        format_token_before_relocated_trailing_comments(doc, &token, LeadingTrivia::Preserve)
    });
    let body = format_import_fields(import, keyword, semicolon, doc);
    let trailing = doc.concat_list(|comments| {
        if let Some(token) = last.as_ref() {
            for comment in token.trailing_comments() {
                let space = comments.space();
                comments.push(space);
                let formatted = format_comment(comments, &comment);
                comments.push(formatted);
            }
        }
    });
    let has_leading_comments = first.is_some_and(|token| !token.leading_comments().is_empty());
    match (salvaged_leading, has_leading_comments) {
        (Some(salvaged), true) => {
            doc_concat!(
                doc,
                [
                    salvaged,
                    doc.hard_line(),
                    leading,
                    doc.hard_line(),
                    body,
                    trailing
                ]
            )
        }
        (Some(salvaged), false) => {
            doc_concat!(doc, [salvaged, doc.hard_line(), body, trailing])
        }
        (None, true) => doc_concat!(doc, [leading, doc.hard_line(), body, trailing]),
        (None, false) => doc_concat!(doc, [body, trailing]),
    }
}

fn format_import_in_place<'source>(
    import: &ImportDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = format_required_field(import.import_keyword(), doc, |token, doc| {
        doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
    });
    let semicolon = format_required_field(import.semicolon(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    format_import_fields(import, keyword, semicolon, doc)
}

fn format_import_fields<'source>(
    import: &ImportDeclaration<'source>,
    keyword: Doc<'source>,
    semicolon: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let module = format_optional_field(import.module_keyword(), doc, |token, doc| {
        doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
    });
    let static_ = format_optional_field(import.static_keyword(), doc, |token, doc| {
        doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
    });
    let name = format_required_field(import.name(), doc, |name: NameSyntax<'source>, doc| {
        format_name(&name, doc)
    });
    let dot = format_optional_field(import.on_demand_dot(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let star = format_optional_field(import.star(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let suffix = format_optional_field(import.suffix(), doc, |suffix, doc| {
        crate::helpers::recovery::format_malformed(&suffix, doc)
    });
    doc_concat!(
        doc,
        [keyword, module, static_, name, dot, star, suffix, semicolon]
    )
}
