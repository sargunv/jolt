use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{ImportDirective, KotlinSyntaxToken};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comment_runs, format_token,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, format_token_with_normalized_text};
use crate::rules::names::{NameSortKey, format_qualified_name};

pub(crate) fn format_imports<'source>(
    doc: &mut DocBuilder<'source>,
    imports: Vec<ImportDirective<'source>>,
) -> Option<Doc<'source>> {
    if imports.is_empty() {
        return None;
    }

    let imports = imports
        .into_iter()
        .map(|import| FormattedImport::from_directive(doc, &import))
        .collect::<Vec<_>>();
    Some(format_leading_comment_runs(
        doc,
        imports,
        FormattedImport::has_leading_comments,
        format_import_run,
    ))
}

fn format_import_run<'source>(
    doc: &mut DocBuilder<'source>,
    mut imports: Vec<FormattedImport<'source>>,
) -> Doc<'source> {
    // Cost model: a comment-delimited run of `r` imports is stably sorted with
    // O(r log r) comparisons. Each comparison streams at most the longer
    // borrowed path key (`p` Unicode scalars), bounding the run by
    // O(r log r * p) time and O(r) formatter-owned storage. There is no layout
    // search or retry.
    let preserve_first = imports
        .first()
        .is_some_and(FormattedImport::has_leading_comments);
    if preserve_first {
        imports[1..].sort_by(|left, right| left.path.cmp(&right.path));
    } else {
        imports.sort_by(|left, right| left.path.cmp(&right.path));
    }
    let imports = imports
        .into_iter()
        .map(|import| import.into_doc(doc))
        .collect::<Vec<_>>();
    join_hard_lines(doc, imports)
}

struct FormattedImport<'source> {
    first_token: Option<KotlinSyntaxToken<'source>>,
    path: NameSortKey<'source>,
    import_token: Option<KotlinSyntaxToken<'source>>,
    path_doc: Doc<'source>,
    suffix_doc: Doc<'source>,
}

impl<'source> FormattedImport<'source> {
    fn from_directive(doc: &mut DocBuilder<'source>, import: &ImportDirective<'source>) -> Self {
        let name = import.name();
        let on_demand = import.star_token().is_some();
        let path_ends_with_trailing_comments = name
            .as_ref()
            .and_then(jolt_kotlin_syntax::QualifiedName::last_token)
            .is_some_and(|token| !token.trailing_comments().is_empty());
        Self {
            first_token: import.first_token(),
            path: name
                .as_ref()
                .map_or_else(NameSortKey::empty, |name| NameSortKey::new(name, on_demand)),
            import_token: import.import_token(),
            path_doc: if let Some(name) = name.as_ref() {
                format_qualified_name(doc, name)
            } else {
                doc.nil()
            },
            suffix_doc: format_import_suffix(doc, import, path_ends_with_trailing_comments),
        }
    }

    fn has_leading_comments(&self) -> bool {
        self.first_token
            .as_ref()
            .is_some_and(|token| !token.leading_comments().is_empty())
    }

    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        doc.concat_list(|docs| {
            let import_token = if let Some(token) = self.import_token.as_ref() {
                format_token_with_normalized_text(
                    docs,
                    token,
                    "import",
                    FormatterInsertedToken::ImportKeyword,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            } else {
                docs.nil()
            };
            docs.push(import_token);
            let space = docs.space();
            docs.push(space);
            docs.push(self.path_doc);
            docs.push(self.suffix_doc);
        })
    }
}

fn format_import_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    import: &ImportDirective<'source>,
    path_ends_with_trailing_comments: bool,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        if let Some(star) = import.star_token() {
            let star = format_import_path_token(docs, &star);
            docs.push(star);
        }

        if let Some(alias_keyword) = import.alias_keyword_token() {
            if !path_ends_with_trailing_comments {
                let space = docs.space();
                docs.push(space);
            }
            let alias_keyword = format_token_with_normalized_text(
                docs,
                &alias_keyword,
                "as",
                FormatterInsertedToken::ImportAliasKeyword,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            );
            docs.push(alias_keyword);
            if let Some(alias) = import.alias() {
                let space = docs.space();
                docs.push(space);
                let alias = crate::rules::names::format_name(docs, &alias);
                docs.push(alias);
            }
        }
    })
}

fn format_import_path_token<'source>(
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
