use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, literal_text, text};
use jolt_java_syntax::{
    CompilationUnit, ImportDeclaration, ModuleDeclaration, ModuleDirective, PackageDeclaration,
};

use crate::rules::declarations::format_type_declaration;

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
    let Some(name) = package.name() else {
        return source_doc(&package.source_text());
    };
    concat([text("package "), text(name_text(&name)), text(";")])
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
    let Some(name) = module.name() else {
        return source_doc(&module.source_text());
    };

    concat([
        if module.is_open() {
            text("open module ")
        } else {
            text("module ")
        },
        text(name_text(&name)),
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

fn source_doc(source: &str) -> Doc {
    literal_text(source.trim().to_owned())
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
}

impl FormattedImport {
    fn from_declaration(import: &ImportDeclaration) -> Self {
        Self {
            leading_comments: import.leading_comment_texts(),
            is_module: import.is_module(),
            is_static: import.is_static(),
            path: import
                .import_path()
                .unwrap_or_else(|| import.source_text().trim().to_owned()),
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
            text(self.path),
            text(";"),
        ]);

        if self.leading_comments.is_empty() {
            import
        } else {
            concat([
                join_hard_lines(self.leading_comments.into_iter().map(text).collect()),
                hard_line(),
                import,
            ])
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
            .map(|name| name_text(&name))
            .collect::<Vec<_>>();
        let primary_name = names.first().cloned().unwrap_or_default();
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
                parts.push(text(primary_name.clone()));
                parts.push(text(";"));
                concat(parts)
            }
            ModuleDirective::ExportsDirective(_) => {
                format_named_targets_directive("exports", &names, " to ")
            }
            ModuleDirective::OpensDirective(_) => {
                format_named_targets_directive("opens", &names, " to ")
            }
            ModuleDirective::UsesDirective(_) => {
                concat([text("uses "), text(primary_name.clone()), text(";")])
            }
            ModuleDirective::ProvidesDirective(provides) => {
                let service = provides
                    .service_name()
                    .map_or_else(|| primary_name.clone(), |name| name_text(&name));
                let implementations = provides
                    .implementation_names()
                    .map(|name| name_text(&name))
                    .collect::<Vec<_>>();
                concat([
                    text("provides "),
                    text(service),
                    text(" with "),
                    join_docs(implementations.into_iter().map(text).collect(), &text(", ")),
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
                join_hard_lines(self.leading_comments.into_iter().map(text).collect()),
                hard_line(),
                self.doc,
            ])
        }
    }
}

fn format_named_targets_directive(keyword: &str, names: &[String], separator: &str) -> Doc {
    let Some(subject) = names.first() else {
        return text(format!("{keyword};"));
    };
    if names.len() == 1 {
        return concat([
            text(keyword.to_owned()),
            text(" "),
            text(subject.clone()),
            text(";"),
        ]);
    }

    concat([
        text(keyword.to_owned()),
        text(" "),
        text(subject.clone()),
        text(separator.to_owned()),
        join_docs(names[1..].iter().cloned().map(text).collect(), &text(", ")),
        text(";"),
    ])
}

fn name_text(name: &jolt_java_syntax::NameSyntax) -> String {
    name.source_text().trim().to_owned()
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
