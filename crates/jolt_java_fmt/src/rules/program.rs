use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, literal_text, text};
use jolt_java_syntax::{
    CompilationUnit, ImportDeclaration, ImportKind, ModuleDeclaration, ModuleDirective,
    PackageDeclaration,
};

use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::names::{format_name, name_key};

pub(crate) fn format_compilation_unit(unit: &CompilationUnit) -> Doc {
    let mut sections = Vec::new();

    if let Some(package) = unit.package_declaration() {
        sections.push(format_package_declaration(&package));
    }

    let imports = format_imports(unit.imports().collect());
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = unit.module_declaration() {
        sections.push(format_module_declaration(&module));
    }

    let types = unit
        .type_declarations()
        .map(|declaration| format_type_declaration(&declaration))
        .collect::<Vec<_>>();
    if !types.is_empty() {
        sections.push(join_empty_lines(types));
    }

    concat([
        if sections.is_empty() {
            jolt_fmt_ir::nil()
        } else {
            join_empty_lines(sections)
        },
        hard_line(),
    ])
}

fn format_package_declaration(package: &PackageDeclaration) -> Doc {
    let annotations = package
        .annotations()
        .map(|annotation| format_annotation(&annotation))
        .collect::<Vec<_>>();
    let declaration = concat([
        text("package "),
        package
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        text(";"),
    ]);

    if annotations.is_empty() {
        declaration
    } else {
        concat([join_hard_lines(annotations), hard_line(), declaration])
    }
}

fn format_imports(imports: Vec<ImportDeclaration>) -> Option<Doc> {
    if imports.is_empty() {
        return None;
    }

    let mut runs: Vec<Vec<FormattedImport>> = Vec::new();
    let mut current_run = Vec::new();

    for import in imports {
        if import.has_leading_comment() && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        let formatted = FormattedImport::from_declaration(&import);
        current_run.push(formatted);
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

fn format_module_declaration(module: &ModuleDeclaration) -> Doc {
    concat([
        if module.is_open() {
            text("open module ")
        } else {
            text("module ")
        },
        module
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        text(" {"),
        indent_module_body(format_module_directives(module.directives().collect())),
        hard_line(),
        text("}"),
    ])
}

fn indent_module_body(directives: Option<Doc>) -> Doc {
    directives.map_or_else(jolt_fmt_ir::nil, |directives| {
        jolt_fmt_ir::indent(concat([hard_line(), directives]))
    })
}

fn format_module_directives(directives: Vec<ModuleDirective>) -> Option<Doc> {
    if directives.is_empty() {
        return None;
    }

    let mut runs: Vec<Vec<FormattedModuleDirective>> = Vec::new();
    let mut current_run = Vec::new();

    for directive in directives {
        if directive.has_leading_comment() && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        current_run.push(FormattedModuleDirective::from_directive(&directive));
    }
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    Some(join_empty_lines(
        runs.into_iter().map(format_module_directive_run).collect(),
    ))
}

fn format_module_directive_run(directives: Vec<FormattedModuleDirective>) -> Doc {
    let mut directives = directives;
    directives.sort_by(|lhs, rhs| {
        lhs.kind_order
            .cmp(&rhs.kind_order)
            .then_with(|| lhs.primary_name.cmp(&rhs.primary_name))
    });

    let mut groups = Vec::new();
    let mut current_kind = None;
    let mut current_group = Vec::new();

    for directive in directives {
        if current_kind.is_some_and(|kind| kind != directive.kind_order) {
            groups.push(join_hard_lines(current_group));
            current_group = Vec::new();
        }
        current_kind = Some(directive.kind_order);
        current_group.push(directive.into_doc());
    }
    if !current_group.is_empty() {
        groups.push(join_hard_lines(current_group));
    }

    join_empty_lines(groups)
}

fn comment_doc(comment: &str) -> Doc {
    literal_text(comment.trim().to_owned())
}

fn join_empty_lines(docs: Vec<Doc>) -> Doc {
    join_docs(docs, &empty_line())
}

fn join_hard_lines(docs: Vec<Doc>) -> Doc {
    join_docs(docs, &hard_line())
}

fn join_docs(docs: Vec<Doc>, separator: &Doc) -> Doc {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(separator.clone());
        }
        joined.push(doc);
    }
    concat(joined)
}

struct FormattedImport {
    leading_comments: Vec<String>,
    is_module: bool,
    is_static: bool,
    path: String,
    path_doc: Doc,
}

impl FormattedImport {
    fn from_declaration(import: &ImportDeclaration) -> Self {
        let kind = import
            .import_kind()
            .expect("clean import declaration should expose an import kind");
        let (is_module, is_static, path, path_doc) = format_import_kind(kind);
        Self {
            leading_comments: import.leading_comment_texts(),
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
        ]);

        if self.leading_comments.is_empty() {
            import
        } else {
            concat([
                join_hard_lines(
                    self.leading_comments
                        .iter()
                        .map(|comment| comment_doc(comment))
                        .collect(),
                ),
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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ModuleDirectiveKindOrder {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
}

struct FormattedModuleDirective {
    leading_comments: Vec<String>,
    kind_order: ModuleDirectiveKindOrder,
    primary_name: String,
    doc: Doc,
}

impl FormattedModuleDirective {
    fn from_directive(directive: &ModuleDirective) -> Self {
        let names = directive
            .names()
            .map(|name| FormattedName::from_name(&name))
            .collect::<Vec<_>>();
        let primary_name = names
            .first()
            .map_or_else(String::new, |name| name.key.clone());
        let kind_order = module_directive_kind_order(directive);
        let doc = match directive {
            ModuleDirective::RequiresDirective(requires) => {
                let mut parts = vec![text("requires ")];
                if requires.has_static_modifier() {
                    parts.push(text("static "));
                }
                if requires.has_transitive_modifier() {
                    parts.push(text("transitive "));
                }
                parts.push(
                    names
                        .first()
                        .map_or_else(jolt_fmt_ir::nil, |name| name.doc.clone()),
                );
                parts.push(text(";"));
                concat(parts)
            }
            ModuleDirective::ExportsDirective(_) => {
                format_named_targets_directive("exports", &names, " to ")
            }
            ModuleDirective::OpensDirective(_) => {
                format_named_targets_directive("opens", &names, " to ")
            }
            ModuleDirective::UsesDirective(_) => concat([
                text("uses "),
                names
                    .first()
                    .map_or_else(jolt_fmt_ir::nil, |name| name.doc.clone()),
                text(";"),
            ]),
            ModuleDirective::ProvidesDirective(provides) => {
                let service = provides.service_name().map_or_else(
                    || {
                        names
                            .first()
                            .map_or_else(jolt_fmt_ir::nil, |name| name.doc.clone())
                    },
                    |name| format_name(&name),
                );
                let implementations = provides
                    .implementation_names()
                    .map(|name| format_name(&name))
                    .collect::<Vec<_>>();
                concat([
                    text("provides "),
                    service,
                    text(" with "),
                    join_docs(implementations, &text(", ")),
                    text(";"),
                ])
            }
        };

        Self {
            leading_comments: directive.leading_comment_texts(),
            kind_order,
            primary_name,
            doc,
        }
    }

    fn into_doc(self) -> Doc {
        if self.leading_comments.is_empty() {
            self.doc
        } else {
            concat([
                join_hard_lines(
                    self.leading_comments
                        .iter()
                        .map(|comment| comment_doc(comment))
                        .collect(),
                ),
                hard_line(),
                self.doc,
            ])
        }
    }
}

struct FormattedName {
    key: String,
    doc: Doc,
}

impl FormattedName {
    fn from_name(name: &jolt_java_syntax::NameSyntax) -> Self {
        Self {
            key: name_key(name),
            doc: format_name(name),
        }
    }
}

fn format_named_targets_directive(keyword: &str, names: &[FormattedName], separator: &str) -> Doc {
    let Some(subject) = names.first() else {
        return text(format!("{keyword};"));
    };
    if names.len() == 1 {
        return concat([
            text(keyword.to_owned()),
            text(" "),
            subject.doc.clone(),
            text(";"),
        ]);
    }

    concat([
        text(keyword.to_owned()),
        text(" "),
        subject.doc.clone(),
        text(separator.to_owned()),
        join_docs(
            names[1..].iter().map(|name| name.doc.clone()).collect(),
            &text(", "),
        ),
        text(";"),
    ])
}

const fn module_directive_kind_order(directive: &ModuleDirective) -> ModuleDirectiveKindOrder {
    match directive {
        ModuleDirective::RequiresDirective(_) => ModuleDirectiveKindOrder::Requires,
        ModuleDirective::ExportsDirective(_) => ModuleDirectiveKindOrder::Exports,
        ModuleDirective::OpensDirective(_) => ModuleDirectiveKindOrder::Opens,
        ModuleDirective::UsesDirective(_) => ModuleDirectiveKindOrder::Uses,
        ModuleDirective::ProvidesDirective(_) => ModuleDirectiveKindOrder::Provides,
    }
}
