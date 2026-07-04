use jolt_fmt_ir::{Doc, concat, hard_line, text};
use jolt_java_syntax::{ImportDeclaration, ImportKind};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_inline_trailing_comment_list,
    format_token_after_relocated_leading_comments, format_token_before_relocated_trailing_comments,
    format_token_with_comments, split_leading_comment_barrier_runs,
};
use crate::rules::names::{format_name, name_key};

pub(crate) fn format_imports(
    imports: Vec<ImportDeclaration>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    if imports.is_empty() {
        return None;
    }

    let runs = split_leading_comment_barrier_runs(imports, |import| {
        import
            .first_token()
            .is_some_and(|token| formatter.comments().has_leading_comment_for_token(&token))
    });

    Some(join_empty_lines(
        runs.into_iter()
            .map(|run| {
                format_import_run(
                    run.into_iter()
                        .map(|import| FormattedImport::from_declaration(&import, formatter))
                        .collect(),
                )
            })
            .collect(),
    ))
}

fn format_import_run(imports: Vec<FormattedImport>) -> Doc {
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

    let mut groups = Vec::new();
    if !normal_imports.is_empty() {
        groups.push(join_hard_lines(
            normal_imports
                .into_iter()
                .map(FormattedImport::into_doc)
                .collect(),
        ));
    }
    if !static_imports.is_empty() {
        groups.push(join_hard_lines(
            static_imports
                .into_iter()
                .map(FormattedImport::into_doc)
                .collect(),
        ));
    }

    join_empty_lines(groups)
}

struct FormattedImport<'source> {
    leading_comments: Vec<jolt_java_syntax::JavaComment<'source>>,
    trailing_comments: Vec<jolt_java_syntax::JavaComment<'source>>,
    is_static: bool,
    path: String,
    import_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    module_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    static_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    path_doc: Doc,
    semicolon: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
}

impl<'source> FormattedImport<'source> {
    fn from_declaration(
        import: &ImportDeclaration<'source>,
        formatter: &JavaFormatter<'source>,
    ) -> Self {
        let kind = import
            .import_kind()
            .expect("clean import declaration should expose an import kind");
        let (is_static, path, path_doc) = format_import_kind(import, kind);
        let first_token = import.first_token();
        let last_token = import.last_token();
        Self {
            leading_comments: formatter
                .comments()
                .leading_comments_for_token_option(first_token.as_ref())
                .to_vec(),
            trailing_comments: formatter
                .comments()
                .trailing_comments_for_token_option(last_token.as_ref())
                .to_vec(),
            is_static,
            path,
            import_token: import.import_token(),
            module_token: import.module_token(),
            static_token: import.static_token(),
            path_doc,
            semicolon: import.semicolon(),
        }
    }

    fn into_doc(self) -> Doc {
        let import = concat([
            self.import_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([
                        format_token_after_relocated_leading_comments(
                            token,
                            TrailingTrivia::Preserve,
                        ),
                        text(" "),
                    ])
                }),
            self.module_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([format_token_with_comments(token), text(" ")])
                }),
            self.static_token
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([format_token_with_comments(token), text(" ")])
                }),
            self.path_doc,
            self.semicolon
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    format_token_before_relocated_trailing_comments(token, LeadingTrivia::Preserve)
                }),
            format_inline_trailing_comment_list(&self.trailing_comments),
        ]);

        if self.leading_comments.is_empty() {
            import
        } else {
            concat([
                join_hard_lines(self.leading_comments.iter().map(format_comment).collect()),
                hard_line(),
                import,
            ])
        }
    }
}

fn format_import_kind(import: &ImportDeclaration, kind: ImportKind) -> (bool, String, Doc) {
    match kind {
        ImportKind::SingleType(name) | ImportKind::SingleModule(name) => {
            let path = name_key(&name);
            (false, path, format_name(&name))
        }
        ImportKind::TypeOnDemand(name) => {
            let mut path = name_key(&name);
            path.push_str(".*");
            (
                false,
                path,
                concat([
                    format_name(&name),
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
            let path = name_key(&name);
            (true, path, format_name(&name))
        }
        ImportKind::StaticOnDemand(name) => {
            let mut path = name_key(&name);
            path.push_str(".*");
            (
                true,
                path,
                concat([
                    format_name(&name),
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
