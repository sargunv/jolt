use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{ImportDeclaration, ImportKind};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_inline_trailing_comment_list,
    format_leading_comment_runs, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_sequence,
    format_token_with_comments,
};
use crate::rules::names::{NameSortKey, format_name};

pub(crate) fn format_imports<'source>(
    imports: Vec<ImportDeclaration<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    if imports.is_empty() {
        return None;
    }

    let formatted_imports = imports
        .into_iter()
        .map(|import| FormattedImport::from_declaration(&import, doc))
        .collect::<Vec<_>>();
    Some(format_leading_comment_runs(
        doc,
        formatted_imports,
        FormattedImport::has_leading_comments,
        format_import_run,
    ))
}

fn format_import_run<'source>(
    imports: Vec<FormattedImport<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut normal_imports = Vec::with_capacity(imports.len());
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

    let mut docs = doc.list();
    if !normal_imports.is_empty() {
        let normal_imports = format_import_list(normal_imports, doc);
        docs.push(normal_imports, doc);
    }
    if !static_imports.is_empty() {
        if !docs.is_empty() {
            docs.push(doc.empty_line(), doc);
        }
        let static_imports = format_import_list(static_imports, doc);
        docs.push(static_imports, doc);
    }

    docs.finish(doc)
}

fn format_import_list<'source>(
    imports: Vec<FormattedImport<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for import in imports {
        if !docs.is_empty() {
            docs.push(doc.hard_line(), doc);
        }
        let import = import.into_doc(doc);
        docs.push(import, doc);
    }
    docs.finish(doc)
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
    fn from_declaration(
        import: &ImportDeclaration<'source>,
        doc: &mut DocBuilder<'source>,
    ) -> Self {
        let Some(kind) = import.import_kind() else {
            return Self {
                first_token: None,
                last_token: None,
                is_static: false,
                path: NameSortKey::recovered(),
                import_token: None,
                module_token: None,
                static_token: None,
                path_doc: format_token_sequence(doc, import.token_iter(), LeadingTrivia::Preserve),
                semicolon: None,
            };
        };
        let (is_static, path, path_doc) = format_import_kind(import, &kind, doc);
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

    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        let import = doc_concat!(
            doc,
            [
                self.import_token.as_ref().map_or_else(Doc::nil, |token| {
                    doc_concat!(
                        doc,
                        [
                            format_token_after_relocated_leading_comments(
                                doc,
                                token,
                                TrailingTrivia::Preserve,
                            ),
                            doc.space(),
                        ]
                    )
                },),
                self.module_token
                    .as_ref()
                    .map_or_else(Doc::nil, |token| doc_concat!(
                        doc,
                        [format_token_with_comments(doc, token), doc.space()]
                    ),),
                self.static_token
                    .as_ref()
                    .map_or_else(Doc::nil, |token| doc_concat!(
                        doc,
                        [format_token_with_comments(doc, token), doc.space()]
                    ),),
                self.path_doc,
                self.semicolon.as_ref().map_or_else(Doc::nil, |token| {
                    format_token_before_relocated_trailing_comments(
                        doc,
                        token,
                        LeadingTrivia::Preserve,
                    )
                },),
                self.last_token.map_or_else(Doc::nil, |token| {
                    format_inline_trailing_comment_list(doc, token.trailing_comments())
                },),
            ]
        );

        if self
            .first_token
            .as_ref()
            .is_none_or(|token| token.leading_comments().is_empty())
        {
            import
        } else {
            let mut leading_comments = doc.list();
            for comment in self
                .first_token
                .into_iter()
                .flat_map(|token| token.leading_comments())
            {
                if !leading_comments.is_empty() {
                    leading_comments.push(doc.hard_line(), doc);
                }
                let comment = format_comment(doc, &comment);
                leading_comments.push(comment, doc);
            }
            let leading_comments = leading_comments.finish(doc);
            doc_concat!(doc, [leading_comments, doc.hard_line(), import,])
        }
    }
}

fn format_import_kind<'source>(
    import: &ImportDeclaration<'source>,
    kind: &ImportKind<'source>,
    doc: &mut DocBuilder<'source>,
) -> (bool, NameSortKey<'source>, Doc<'source>) {
    match kind {
        ImportKind::SingleType(name) | ImportKind::SingleModule(name) => {
            let path = NameSortKey::new(name, false);
            (false, path, format_name(name, doc))
        }
        ImportKind::TypeOnDemand(name) => {
            let path = NameSortKey::new(name, true);
            (
                false,
                path,
                doc_concat!(
                    doc,
                    [
                        format_name(name, doc),
                        import
                            .on_demand_dot_token()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                        import
                            .star_token()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                    ]
                ),
            )
        }
        ImportKind::SingleStatic(name) => {
            let path = NameSortKey::new(name, false);
            (true, path, format_name(name, doc))
        }
        ImportKind::StaticOnDemand(name) => {
            let path = NameSortKey::new(name, true);
            (
                true,
                path,
                doc_concat!(
                    doc,
                    [
                        format_name(name, doc),
                        import
                            .on_demand_dot_token()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                        import
                            .star_token()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                    ]
                ),
            )
        }
    }
}
