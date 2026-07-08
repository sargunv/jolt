use jolt_fmt_ir::{Doc, concat, space};
use jolt_kotlin_syntax::{ImportDirective, KotlinSyntaxKind, KotlinSyntaxToken};

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
    path_ends_with_trailing_comments: bool,
    suffix_tokens: Vec<KotlinSyntaxToken<'source>>,
}

impl<'source> FormattedImport<'source> {
    fn from_directive(import: &ImportDirective<'source>) -> Self {
        let tokens = import.token_iter().collect::<Vec<_>>();
        let name = import.name();
        let on_demand = tokens
            .iter()
            .any(|token| token.kind() == KotlinSyntaxKind::Star);
        let suffix_tokens = if let Some(name) = name.as_ref() {
            let suffix_start = name.text_range().end();
            tokens
                .iter()
                .copied()
                .filter(|token| token.text_range().start() >= suffix_start)
                .collect()
        } else {
            tokens.iter().copied().skip(1).collect()
        };
        Self {
            first_token: import.first_token(),
            path: name
                .as_ref()
                .map_or_else(NameSortKey::empty, |name| NameSortKey::new(name, on_demand)),
            import_token: tokens.first().copied(),
            path_doc: name
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |name| format_qualified_name(name)),
            path_ends_with_trailing_comments: name
                .as_ref()
                .and_then(jolt_kotlin_syntax::QualifiedName::last_token)
                .is_some_and(|token| !token.trailing_comments().is_empty()),
            suffix_tokens,
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

        let mut previous_was_alias_keyword = false;
        for token in self.suffix_tokens {
            if token.kind() == KotlinSyntaxKind::Star {
                docs.push(format_import_path_token(&token));
                continue;
            }
            if is_alias_keyword(token) {
                if !self.path_ends_with_trailing_comments {
                    docs.push(space());
                }
                docs.push(format_token_with_normalized_text(
                    &token,
                    "as",
                    FormatterInsertedToken::ImportAliasKeyword,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ));
                previous_was_alias_keyword = true;
                continue;
            }

            if previous_was_alias_keyword {
                docs.push(space());
                previous_was_alias_keyword = false;
            }
            docs.push(format_import_path_token(&token));
        }

        concat(docs)
    }
}

fn format_import_path_token<'source>(token: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
}

fn is_alias_keyword(token: KotlinSyntaxToken<'_>) -> bool {
    token.kind() == KotlinSyntaxKind::AsKw || token.text() == "as"
}
