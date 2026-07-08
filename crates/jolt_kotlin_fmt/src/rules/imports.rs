use jolt_fmt_ir::{Doc, concat, space};
use jolt_kotlin_syntax::{ImportDirective, KotlinSyntaxToken};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comment_runs, format_token,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, format_token_with_normalized_text};
use crate::rules::names::{NameSortKey, format_qualified_name};

pub(crate) fn format_imports(imports: Vec<ImportDirective<'_>>) -> Option<Doc<'_>> {
    if imports.is_empty() {
        return None;
    }

    Some(format_leading_comment_runs(
        imports
            .into_iter()
            .map(|import| FormattedImport::from_directive(&import)),
        FormattedImport::has_leading_comments,
        format_import_run,
    ))
}

fn format_import_run(mut imports: Vec<FormattedImport<'_>>) -> Doc<'_> {
    let preserve_first = imports
        .first()
        .is_some_and(FormattedImport::has_leading_comments);
    if preserve_first {
        imports[1..].sort_by(|left, right| left.path.cmp(&right.path));
    } else {
        imports.sort_by(|left, right| left.path.cmp(&right.path));
    }
    join_hard_lines(imports.into_iter().map(FormattedImport::into_doc))
}

struct FormattedImport<'source> {
    first_token: Option<KotlinSyntaxToken<'source>>,
    path: NameSortKey<'source>,
    import_token: Option<KotlinSyntaxToken<'source>>,
    path_doc: Doc<'source>,
    suffix_doc: Doc<'source>,
}

impl<'source> FormattedImport<'source> {
    fn from_directive(import: &ImportDirective<'source>) -> Self {
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
            path_doc: name
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |name| format_qualified_name(name)),
            suffix_doc: format_import_suffix(import, path_ends_with_trailing_comments),
        }
    }

    fn has_leading_comments(&self) -> bool {
        self.first_token
            .as_ref()
            .is_some_and(|token| !token.leading_comments().is_empty())
    }

    fn into_doc(self) -> Doc<'source> {
        let mut docs = vec![
            self.import_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    format_token_with_normalized_text(
                        token,
                        "import",
                        FormatterInsertedToken::ImportKeyword,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    )
                }),
            space(),
            self.path_doc,
        ];

        docs.push(self.suffix_doc);

        concat(docs)
    }
}

fn format_import_suffix<'source>(
    import: &ImportDirective<'source>,
    path_ends_with_trailing_comments: bool,
) -> Doc<'source> {
    let mut docs = Vec::new();

    if let Some(star) = import.star_token() {
        docs.push(format_import_path_token(&star));
    }

    if let Some(alias_keyword) = import.alias_keyword_token() {
        if !path_ends_with_trailing_comments {
            docs.push(space());
        }
        docs.push(format_token_with_normalized_text(
            &alias_keyword,
            "as",
            FormatterInsertedToken::ImportAliasKeyword,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        ));
        if let Some(alias) = import.alias() {
            docs.push(space());
            docs.push(crate::rules::names::format_name(&alias));
        }
    }

    concat(docs)
}

fn format_import_path_token<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
}
