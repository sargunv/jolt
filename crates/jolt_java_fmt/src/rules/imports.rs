use jolt_fmt_ir::{Doc, concat, hard_line, text};
use jolt_java_syntax::{ImportDeclaration, ImportKind};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::format_comment;
use crate::rules::names::{format_name, name_key};

pub(crate) fn format_imports(
    imports: Vec<ImportDeclaration>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    if imports.is_empty() {
        return None;
    }

    let mut runs: Vec<Vec<FormattedImport>> = Vec::new();
    let mut current_run = Vec::new();

    for import in imports {
        let tokens = import.tokens();
        if formatter.comments().has_leading_comment_for_tokens(&tokens) && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        let import_entry = FormattedImport::from_declaration(&import, formatter);
        current_run.push(import_entry);
    }
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    Some(join_empty_lines(
        runs.into_iter().map(format_import_run).collect(),
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

struct FormattedImport {
    leading_comments: Vec<jolt_java_syntax::JavaComment>,
    trailing_comments: Vec<jolt_java_syntax::JavaComment>,
    is_module: bool,
    is_static: bool,
    path: String,
    path_doc: Doc,
}

impl FormattedImport {
    fn from_declaration(import: &ImportDeclaration, formatter: &JavaFormatter<'_>) -> Self {
        let kind = import
            .import_kind()
            .expect("clean import declaration should expose an import kind");
        let (is_module, is_static, path, path_doc) = format_import_kind(kind);
        let tokens = import.tokens();
        Self {
            leading_comments: formatter
                .comments()
                .leading_comments_for_tokens(&tokens)
                .to_vec(),
            trailing_comments: formatter
                .comments()
                .trailing_comments_for_tokens(&tokens)
                .to_vec(),
            is_module,
            is_static,
            path,
            path_doc,
        }
    }

    fn into_doc(self) -> Doc {
        let import = concat([
            text("import "),
            if self.is_module {
                text("module ")
            } else {
                jolt_fmt_ir::nil()
            },
            if self.is_static {
                text("static ")
            } else {
                jolt_fmt_ir::nil()
            },
            self.path_doc,
            text(";"),
            format_inline_trailing_comments(&self.trailing_comments),
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

fn format_import_kind(kind: ImportKind) -> (bool, bool, String, Doc) {
    match kind {
        ImportKind::SingleType(name) => {
            let path = name_key(&name);
            (false, false, path, format_name(&name))
        }
        ImportKind::TypeOnDemand(name) => {
            let mut path = name_key(&name);
            path.push_str(".*");
            (false, false, path, concat([format_name(&name), text(".*")]))
        }
        ImportKind::SingleStatic(name) => {
            let path = name_key(&name);
            (false, true, path, format_name(&name))
        }
        ImportKind::StaticOnDemand(name) => {
            let mut path = name_key(&name);
            path.push_str(".*");
            (false, true, path, concat([format_name(&name), text(".*")]))
        }
        ImportKind::SingleModule(name) => {
            let path = name_key(&name);
            (true, false, path, format_name(&name))
        }
    }
}

fn format_inline_trailing_comments(comments: &[jolt_java_syntax::JavaComment]) -> Doc {
    concat(
        comments
            .iter()
            .map(|comment| concat([text(" "), format_comment(comment)]))
            .collect::<Vec<_>>(),
    )
}
