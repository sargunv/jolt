use jolt_fmt_ir::space;
use jolt_fmt_ir::{Doc, concat, empty_line, hard_line};
use jolt_java_syntax::{ImportDeclaration, ImportKind};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_inline_trailing_comment_list,
    format_leading_comment_runs, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
};
use crate::rules::names::{NameSortKey, format_name};

pub(crate) fn format_imports(imports: Vec<ImportDeclaration<'_>>) -> Option<Doc<'_>> {
    if imports.is_empty() {
        return None;
    }

    Some(format_leading_comment_runs(
        imports
            .into_iter()
            .map(|import| FormattedImport::from_declaration(&import)),
        FormattedImport::has_leading_comments,
        format_import_run,
    ))
}

fn format_import_run(imports: Vec<FormattedImport<'_>>) -> Doc<'_> {
    let mut normal_imports = Vec::new();
    let mut static_imports = Vec::new();

    for import in imports {
        if import.is_static {
            static_imports.push(import);
        } else {
            normal_imports.push(import);
        }
    }

    normal_imports.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));
    static_imports.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));

    let mut docs = Vec::new();
    if !normal_imports.is_empty() {
        docs.push(join_hard_lines(
            normal_imports.into_iter().map(FormattedImport::into_doc),
        ));
    }
    if !static_imports.is_empty() {
        if !docs.is_empty() {
            docs.push(empty_line());
        }
        docs.push(join_hard_lines(
            static_imports.into_iter().map(FormattedImport::into_doc),
        ));
    }

    concat(docs)
}

struct FormattedImport<'source> {
    first_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    last_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    is_static: bool,
    path: NameSortKey<'source>,
    import_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    module_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    static_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    path_doc: Doc<'source>,
    semicolon: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
}

impl<'source> FormattedImport<'source> {
    fn from_declaration(import: &ImportDeclaration<'source>) -> Self {
        let kind = import
            .import_kind()
            .expect("clean import declaration should expose an import kind");
        let (is_static, path, path_doc) = format_import_kind(import, &kind);
        Self {
            first_token: import.first_token(),
            last_token: import.last_token(),
            is_static,
            path,
            import_token: import.import_token(),
            module_token: import.module_token(),
            static_token: import.static_token(),
            path_doc,
            semicolon: import.semicolon(),
        }
    }

    fn has_leading_comments(&self) -> bool {
        self.first_token
            .as_ref()
            .is_some_and(|token| !token.leading_comments().is_empty())
    }

    fn into_doc(self) -> Doc<'source> {
        let import = concat([
            self.import_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([
                        format_token_after_relocated_leading_comments(
                            token,
                            TrailingTrivia::Preserve,
                        ),
                        space(),
                    ])
                }),
            self.module_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([format_token_with_comments(token), space()])
                }),
            self.static_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([format_token_with_comments(token), space()])
                }),
            self.path_doc,
            self.semicolon
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    format_token_before_relocated_trailing_comments(token, LeadingTrivia::Preserve)
                }),
            self.last_token.map_or_else(jolt_fmt_ir::nil, |token| {
                format_inline_trailing_comment_list(token.trailing_comments())
            }),
        ]);

        if self
            .first_token
            .as_ref()
            .is_none_or(|token| token.leading_comments().is_empty())
        {
            import
        } else {
            let leading_comments = self
                .first_token
                .into_iter()
                .flat_map(|token| token.leading_comments());
            concat([
                join_hard_lines(leading_comments.map(|comment| format_comment(&comment))),
                hard_line(),
                import,
            ])
        }
    }
}

fn format_import_kind<'source>(
    import: &ImportDeclaration<'source>,
    kind: &ImportKind<'source>,
) -> (bool, NameSortKey<'source>, Doc<'source>) {
    match kind {
        ImportKind::SingleType(name) | ImportKind::SingleModule(name) => {
            let path = NameSortKey::new(name, false);
            (false, path, format_name(name))
        }
        ImportKind::TypeOnDemand(name) => {
            let path = NameSortKey::new(name, true);
            (
                false,
                path,
                concat([
                    format_name(name),
                    import
                        .on_demand_dot_token()
                        .as_ref()
                        .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                    import
                        .star_token()
                        .as_ref()
                        .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                ]),
            )
        }
        ImportKind::SingleStatic(name) => {
            let path = NameSortKey::new(name, false);
            (true, path, format_name(name))
        }
        ImportKind::StaticOnDemand(name) => {
            let path = NameSortKey::new(name, true);
            (
                true,
                path,
                concat([
                    format_name(name),
                    import
                        .on_demand_dot_token()
                        .as_ref()
                        .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                    import
                        .star_token()
                        .as_ref()
                        .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                ]),
            )
        }
    }
}
