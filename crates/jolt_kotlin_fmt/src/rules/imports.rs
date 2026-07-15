use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ImportAlias, ImportDirective, KotlinSyntaxField, KotlinSyntaxToken, KotlinSyntaxView,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_terminator_list, format_token,
};
use crate::helpers::recovery::{
    KotlinFormatField, format_optional_field, format_or_verbatim, format_required_field,
    resolve_optional_field, resolve_required_field,
};
use crate::rules::names::{NameSortKey, format_name, format_qualified_name};

pub(crate) fn format_imports<'source>(
    doc: &mut DocBuilder<'source>,
    imports: Vec<ImportDirective<'source>>,
) -> Option<Doc<'source>> {
    if imports.is_empty() {
        return None;
    }

    let mut sections = Vec::new();
    let mut sortable = Vec::new();
    for import in imports {
        if is_sortable_import(&import) {
            if import
                .first_token()
                .is_some_and(|token| !token.leading_comments().is_empty())
            {
                flush_sortable(doc, &mut sortable, &mut sections);
                sections.push(ImportSection {
                    doc: format_import(doc, &import),
                    starts_comment_barrier: true,
                });
                continue;
            }
            sortable.push(FormattedImport::new(doc, import));
        } else {
            flush_sortable(doc, &mut sortable, &mut sections);
            sections.push(ImportSection {
                doc: format_import(doc, &import),
                starts_comment_barrier: false,
            });
        }
    }
    flush_sortable(doc, &mut sortable, &mut sections);
    Some(doc.concat_list(|docs| {
        for (index, section) in sections.into_iter().enumerate() {
            if index != 0 {
                let separator = if section.starts_comment_barrier {
                    docs.empty_line()
                } else {
                    docs.hard_line()
                };
                docs.push(separator);
            }
            docs.push(section.doc);
        }
    }))
}

struct ImportSection<'source> {
    doc: Doc<'source>,
    starts_comment_barrier: bool,
}

fn is_sortable_import(import: &ImportDirective<'_>) -> bool {
    fn required<T>(
        field: &Result<KotlinSyntaxField<'_, T>, jolt_kotlin_syntax::KotlinSyntaxInvariantError>,
    ) -> bool {
        matches!(field, Ok(KotlinSyntaxField::Present(_)))
    }
    fn optional<T>(
        field: &Result<KotlinSyntaxField<'_, T>, jolt_kotlin_syntax::KotlinSyntaxInvariantError>,
    ) -> bool {
        matches!(
            field,
            Ok(KotlinSyntaxField::Present(_) | KotlinSyntaxField::Missing(_))
        )
    }

    import.is_recovery_free()
        && required(&import.import_token())
        && matches!(import.name(), Ok(KotlinSyntaxField::Present(ref name)) if name.is_recovery_free())
        && optional(&import.star_separator())
        && optional(&import.star())
        && optional(&import.alias())
        && required(&import.terminators())
}

fn flush_sortable<'source>(
    doc: &mut DocBuilder<'source>,
    imports: &mut Vec<FormattedImport<'source>>,
    sections: &mut Vec<ImportSection<'source>>,
) {
    if imports.is_empty() {
        return;
    }
    imports.sort_by(|left, right| left.path.cmp(&right.path));
    sections.extend(
        std::mem::take(imports)
            .into_iter()
            .map(|import| ImportSection {
                doc: import.into_doc(doc),
                starts_comment_barrier: false,
            }),
    );
}

struct FormattedImport<'source> {
    import: ImportDirective<'source>,
    path: NameSortKey<'source>,
}

impl<'source> FormattedImport<'source> {
    fn new(doc: &mut DocBuilder<'source>, import: ImportDirective<'source>) -> Self {
        let on_demand = matches!(
            resolve_optional_field(import.star(), doc),
            KotlinFormatField::Present(Some(_))
        );
        let path = match resolve_required_field(import.name(), doc) {
            KotlinFormatField::Present(name) => NameSortKey::new(&name, on_demand),
            KotlinFormatField::Malformed(_) => NameSortKey::empty(),
        };
        Self { import, path }
    }

    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        format_import(doc, &self.import)
    }
}

fn format_import<'source>(
    doc: &mut DocBuilder<'source>,
    import: &ImportDirective<'source>,
) -> Doc<'source> {
    format_or_verbatim(import, doc, |doc| {
        let keyword = format_required_field(import.import_token(), doc, |token, doc| {
            let keyword = format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            );
            let space = doc.space();
            doc.concat([keyword, space])
        });
        let name = format_required_field(import.name(), doc, |name, doc| {
            format_qualified_name(doc, &name)
        });
        let dot = format_optional_field(import.star_separator(), doc, |dot, doc| {
            format_path_token(doc, &dot)
        });
        let star = format_optional_field(import.star(), doc, |star, doc| {
            format_path_token(doc, &star)
        });
        let alias = format_optional_field(import.alias(), doc, |alias, doc| {
            let space = doc.space();
            let alias = format_import_alias(doc, &alias);
            doc.concat([space, alias])
        });
        let terminators = format_required_field(import.terminators(), doc, |terminators, doc| {
            format_terminator_list(doc, &terminators, true)
        });
        doc.concat([keyword, name, dot, star, alias, terminators])
    })
}

fn format_import_alias<'source>(
    doc: &mut DocBuilder<'source>,
    alias: &ImportAlias<'source>,
) -> Doc<'source> {
    format_or_verbatim(alias, doc, |doc| {
        let keyword = format_required_field(alias.alias_keyword(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });
        let name = format_required_field(alias.name(), doc, |name, doc| {
            let space = doc.space();
            let name = format_name(doc, &name);
            doc.concat([space, name])
        });
        doc.concat([keyword, name])
    })
}

fn format_path_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}
