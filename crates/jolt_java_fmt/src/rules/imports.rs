use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{ImportDeclaration, JavaSyntaxView, NameSyntax, ReorderClaim};

use crate::helpers::blocks::join_empty_lines;
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
    let mut sortable = Vec::new();
    for declaration in imports.iter().copied() {
        let Some(import) = FormattedImport::new(declaration) else {
            flush_sortable(&mut sortable, &mut sections, doc);
            sections.push(format_import(&declaration, doc));
            continue;
        };
        if !sortable.is_empty()
            && import
                .import
                .first_token()
                .is_some_and(|token| !token.leading_comments().is_empty())
        {
            flush_sortable(&mut sortable, &mut sections, doc);
        }
        sortable.push(import);
    }
    flush_sortable(&mut sortable, &mut sections, doc);
    Some(join_empty_lines(doc, sections))
}

fn flush_sortable<'source>(
    imports: &mut Vec<FormattedImport<'source>>,
    sections: &mut Vec<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    if imports.is_empty() {
        return;
    }
    let mut normal = Vec::new();
    let mut static_ = Vec::new();
    for import in std::mem::take(imports) {
        if import.is_static {
            static_.push(import);
        } else {
            normal.push(import);
        }
    }
    // Each recovery- and comment-delimited run has `r <= represented tokens`.
    // Stable sorting therefore costs O(r log r) time and O(r) scratch, with no
    // layout search or cloning of parser-owned source or syntax buffers.
    normal.sort_by(|left, right| left.key.cmp(&right.key));
    static_.sort_by(|left, right| left.key.cmp(&right.key));
    if !normal.is_empty() {
        sections.push(format_import_list(normal, doc));
    }
    if !static_.is_empty() {
        sections.push(format_import_list(static_, doc));
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
        use jolt_java_syntax::JavaSyntaxField::{Malformed, Missing, Present};

        let reorder = import.canonical_reorder_claim()?;
        if !matches!(import.import_keyword(), Ok(Present(_)))
            || !matches!(import.module_keyword(), Ok(Present(_) | Missing(_)))
            || !matches!(import.static_keyword(), Ok(Present(_) | Missing(_)))
            || !matches!(import.on_demand_dot(), Ok(Present(_) | Missing(_)))
            || !matches!(import.star(), Ok(Present(_) | Missing(_)))
            || !matches!(import.semicolon(), Ok(Present(_)))
        {
            return None;
        }
        let name = match import.name() {
            Ok(Present(name)) if name.is_recovery_free() => name,
            Ok(Present(_) | Missing(_) | Malformed(_)) | Err(_) => return None,
        };
        let on_demand = matches!(import.star(), Ok(Present(_)));
        let key = NameSortKey::new(&name, on_demand)?;
        let is_static = matches!(import.static_keyword(), Ok(Present(_)));
        Some(Self {
            import,
            reorder,
            key,
            is_static,
        })
    }

    #[allow(clippy::redundant_closure_for_method_calls)]
    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        let formatted = format_import(&self.import, doc);
        doc.reordered_source(formatted, self.reorder)
    }
}

/// Formats import boundary comments in one place regardless of whether the
/// import participates in a sortable run. The body deliberately suppresses
/// those boundary comments because moving a valid import must move them too.
#[allow(clippy::redundant_closure_for_method_calls)]
fn format_import<'source>(
    import: &ImportDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let first = import.first_token();
    let last = import.last_token();
    let leading = doc.concat_list(|comments| {
        for comment in first.iter().flat_map(|token| token.leading_comments()) {
            if !comments.is_empty() {
                let line = comments.hard_line();
                comments.push(line);
            }
            let formatted = format_comment(comments, &comment);
            comments.push(formatted);
        }
    });
    let body = format_import_body(import, doc);
    let trailing = doc.concat_list(|comments| {
        for comment in last.iter().flat_map(|token| token.trailing_comments()) {
            let space = comments.space();
            comments.push(space);
            let formatted = format_comment(comments, &comment);
            comments.push(formatted);
        }
    });
    if first.is_some_and(|token| !token.leading_comments().is_empty()) {
        doc_concat!(doc, [leading, doc.hard_line(), body, trailing])
    } else {
        doc_concat!(doc, [body, trailing])
    }
}

fn format_import_body<'source>(
    import: &ImportDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
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
    let semicolon = format_required_field(import.semicolon(), doc, |token, doc| {
        format_token_before_relocated_trailing_comments(doc, &token, LeadingTrivia::Preserve)
    });
    doc_concat!(
        doc,
        [keyword, module, static_, name, dot, star, suffix, semicolon]
    )
}
