use jolt_fmt_ir::{BodyItemSeparator, Doc, DocBuilder};
use jolt_java_syntax::{ImportDeclaration, JavaSyntaxView, NameSyntax, ReorderClaim};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use crate::rules::names::{NameSortKey, format_name};

pub(crate) fn format_imports<'source>(
    imports: &[ImportDeclaration<'source>],
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    if imports.is_empty() {
        return None;
    }

    let mut sections = Vec::new();
    let mut pending = PendingImports::default();
    for declaration in imports.iter().copied() {
        let blank_before = declaration.starts_after_blank_line();
        let Some(import) = FormattedImport::new(declaration) else {
            pending.flush(&mut sections, doc);
            sections.push(ImportSection {
                doc: format_import_in_place(&declaration, doc),
                blank_before,
            });
            continue;
        };
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

/// One batch of imports between unsortable barriers: a list of runs, where
/// each import carrying leading comments starts the run below it. Key
/// sorting happens within a run, so it never crosses a comment (comments
/// travel with their import), while the normal/static regroup is global to
/// the batch — a later import may land above an earlier comment through the
/// regroup, but nothing above a comment ever moves below it. That ordering
/// is a fixpoint, so formatting is idempotent regardless of the source's
/// import order.
#[derive(Default)]
struct PendingImports<'source> {
    runs: Vec<ImportRun<'source>>,
    first_blank_before: bool,
}

struct ImportRun<'source> {
    normal: Vec<FormattedImport<'source>>,
    static_: Vec<FormattedImport<'source>>,
    blank_before: bool,
}

impl<'source> PendingImports<'source> {
    fn push(&mut self, import: FormattedImport<'source>, blank_before: bool) {
        if self.runs.is_empty() {
            self.first_blank_before = blank_before;
        }
        let starts_run = self.runs.is_empty()
            || import
                .import
                .first_token()
                .is_some_and(|token| !token.leading_comments().is_empty());
        if starts_run {
            self.runs.push(ImportRun {
                normal: Vec::new(),
                static_: Vec::new(),
                blank_before,
            });
        }
        let run = self.runs.last_mut().expect("a run always exists");
        if import.is_static {
            run.static_.push(import);
        } else {
            run.normal.push(import);
        }
    }

    fn flush(&mut self, sections: &mut Vec<ImportSection<'source>>, doc: &mut DocBuilder<'source>) {
        let mut runs = std::mem::take(&mut self.runs);
        if runs.is_empty() {
            return;
        }
        // Each batch has `r <= represented tokens`. Stable sorting therefore
        // costs O(r log r) time and O(r) scratch, with no layout search or
        // cloning of parser-owned source or syntax buffers.
        for run in &mut runs {
            run.normal.sort_by(|left, right| left.key.cmp(&right.key));
            run.static_.sort_by(|left, right| left.key.cmp(&right.key));
        }
        let has_normals = runs.iter().any(|run| !run.normal.is_empty());
        let first_blank = self.first_blank_before;
        let mut first = true;
        for run in &mut runs {
            let normal = std::mem::take(&mut run.normal);
            if normal.is_empty() {
                continue;
            }
            sections.push(ImportSection {
                doc: format_import_list(normal, doc),
                blank_before: if first { first_blank } else { run.blank_before },
            });
            first = false;
        }
        let mut first_static = true;
        for run in &mut runs {
            let blank_before = (first_static && has_normals) || run.blank_before;
            first_static = false;
            let static_ = std::mem::take(&mut run.static_);
            if static_.is_empty() {
                continue;
            }
            // Grouping static imports apart is this formatter's own split, so
            // it owns the blank line that marks it.
            sections.push(ImportSection {
                doc: format_import_list(static_, doc),
                blank_before: if first { first_blank } else { blank_before },
            });
            first = false;
        }
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
        })
    }

    #[allow(clippy::redundant_closure_for_method_calls)]
    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        let formatted = self.format_with_relocated_boundary_comments(doc);
        doc.reordered_source(formatted, self.reorder)
    }

    /// Relocates boundary comments together with an import that has proved it
    /// is recovery-free and therefore eligible for sorting.
    fn format_with_relocated_boundary_comments(
        &self,
        doc: &mut DocBuilder<'source>,
    ) -> Doc<'source> {
        let first = self.import.first_token();
        let last = self.import.last_token();
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
        let keyword = format_required_field(self.import.import_keyword(), doc, |token, doc| {
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
        let semicolon = format_required_field(self.import.semicolon(), doc, |token, doc| {
            format_token_before_relocated_trailing_comments(doc, &token, LeadingTrivia::Preserve)
        });
        let body = format_import_fields(&self.import, keyword, semicolon, doc);
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
        if first.is_some_and(|token| !token.leading_comments().is_empty()) {
            doc_concat!(doc, [leading, doc.hard_line(), body, trailing])
        } else {
            doc_concat!(doc, [body, trailing])
        }
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
