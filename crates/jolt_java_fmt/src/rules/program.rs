use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, text};
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, ImportDeclaration, ImportKind, JavaLexer, JavaSyntaxKind,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveRole, PackageDeclaration, TriviaKind,
};

use crate::context::{FormatRule, JavaFormatter};
use crate::helpers::comments::{format_comment, format_raw_comment};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::names::{format_name, name_key};

pub(crate) struct ProgramRule;

impl FormatRule<CompilationUnit> for ProgramRule {
    fn fmt(&self, unit: &CompilationUnit, formatter: &mut JavaFormatter<'_>) -> Doc {
        format_compilation_unit(unit, formatter)
    }
}

fn format_compilation_unit(unit: &CompilationUnit, formatter: &mut JavaFormatter<'_>) -> Doc {
    let items = unit.items().collect::<Vec<_>>();
    let contents = if items.is_empty() {
        format_comment_only_compilation_unit(unit)
    } else {
        let ignored_ranges = formatter_ignore_ranges(&unit.source_text());
        let item_ranges = items
            .iter()
            .map(compilation_unit_item_token_range)
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        if ignored_runs.is_empty() {
            format_compilation_unit_items(items, formatter).unwrap_or_else(jolt_fmt_ir::nil)
        } else {
            format_compilation_unit_items_with_ignored(items, &ignored_runs, formatter)
        }
    };

    concat([contents, hard_line()])
}

fn format_compilation_unit_items(
    items: Vec<CompilationUnitItem>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let mut sections = Vec::new();
    let mut package = None;
    let mut imports = Vec::new();
    let mut module = None;
    let mut types = Vec::new();

    for item in items {
        match item {
            CompilationUnitItem::Package(declaration) => package = Some(declaration),
            CompilationUnitItem::Import(declaration) => imports.push(declaration),
            CompilationUnitItem::Module(declaration) => module = Some(declaration),
            CompilationUnitItem::Type(declaration) => types.push(declaration),
            CompilationUnitItem::EmptyDeclaration(_) => {}
        }
    }

    if let Some(package) = package {
        sections.push(format_package_declaration(&package));
    }

    let imports = format_imports(imports, formatter);
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = module {
        sections.push(format_module_declaration(&module, formatter));
    }

    let types = types
        .into_iter()
        .map(|declaration| format_type_declaration(&declaration))
        .collect::<Vec<_>>();
    if !types.is_empty() {
        sections.push(join_empty_lines(types));
    }

    (!sections.is_empty()).then(|| join_empty_lines(sections))
}

fn format_compilation_unit_items_with_ignored(
    items: Vec<CompilationUnitItem>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun],
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_compilation_unit_segment(&mut sections, &mut segment, formatter);
            let run = &ignored_runs[ignored_index];
            sections.push(ProgramSection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= item_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(item_index) {
            continue;
        }

        segment.push(item);
    }

    push_compilation_unit_segment(&mut sections, &mut segment, formatter);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ProgramSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_program_sections(sections)
}

fn push_compilation_unit_segment(
    sections: &mut Vec<ProgramSection>,
    segment: &mut Vec<CompilationUnitItem>,
    formatter: &JavaFormatter<'_>,
) {
    if segment.is_empty() {
        return;
    }
    let items = std::mem::take(segment);
    if let Some(doc) = format_compilation_unit_items(items, formatter) {
        sections.push(ProgramSection {
            doc,
            hard_line_after: false,
        });
    }
}

fn join_program_sections(sections: Vec<ProgramSection>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            joined.push(if previous_hard_line_after {
                hard_line()
            } else {
                empty_line()
            });
        }
        joined.push(section.doc);
        previous_hard_line_after = section.hard_line_after;
    }
    concat(joined)
}

struct ProgramSection {
    doc: Doc,
    hard_line_after: bool,
}

fn compilation_unit_item_token_range(item: &CompilationUnitItem) -> Option<Range<usize>> {
    let tokens = match item {
        CompilationUnitItem::Package(item) => item.tokens(),
        CompilationUnitItem::Import(item) => item.tokens(),
        CompilationUnitItem::Module(item) => item.tokens(),
        CompilationUnitItem::Type(item) => item.tokens(),
        CompilationUnitItem::EmptyDeclaration(item) => item.tokens(),
    };
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(first.token_text_range().start().get()..last.token_text_range().end().get())
}

fn format_comment_only_compilation_unit(unit: &CompilationUnit) -> Doc {
    let source = unit.source_text();
    let mut lexer = JavaLexer::new(&source);
    let token = lexer.next_token();
    if token.kind != JavaSyntaxKind::Eof {
        return jolt_fmt_ir::nil();
    }

    join_hard_lines(
        token
            .leading
            .into_iter()
            .filter(|trivia| {
                matches!(
                    trivia.kind,
                    TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::JavadocComment
                )
            })
            .map(|trivia| {
                let range = trivia.range;
                let text = &source[range.start().get()..range.end().get()];
                format_raw_comment(trivia.kind, text)
            })
            .collect(),
    )
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

fn format_imports(imports: Vec<ImportDeclaration>, formatter: &JavaFormatter<'_>) -> Option<Doc> {
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

fn format_module_declaration(module: &ModuleDeclaration, formatter: &JavaFormatter<'_>) -> Doc {
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
        indent_module_body(format_module_directives(module, formatter)),
        hard_line(),
        text("}"),
    ])
}

fn indent_module_body(directives: Option<Doc>) -> Doc {
    directives.map_or_else(jolt_fmt_ir::nil, |directives| {
        jolt_fmt_ir::indent(concat([hard_line(), directives]))
    })
}

fn format_module_directives(
    module: &ModuleDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let directives = module.directives().collect::<Vec<_>>();
    if directives.is_empty() {
        return None;
    }

    let ignored_ranges = formatter_ignore_ranges(&module.source_text());
    let directive_ranges = directives
        .iter()
        .map(|directive| module_directive_token_range(directive, module.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &directive_ranges);
    if !ignored_runs.is_empty() {
        return Some(format_module_directives_with_ignored(
            directives,
            &ignored_runs,
            formatter,
        ));
    }

    Some(format_module_directive_segments(directives, formatter))
}

fn format_module_directives_with_ignored(
    directives: Vec<ModuleDirective>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun],
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (directive_index, directive) in directives.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == directive_index
        {
            push_module_directive_segment(&mut sections, &mut segment, formatter);
            let run = &ignored_runs[ignored_index];
            sections.push(ModuleDirectiveSection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len()
            && ignored_runs[skip_index].skip_end <= directive_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(directive_index) {
            continue;
        }

        segment.push(directive);
    }

    push_module_directive_segment(&mut sections, &mut segment, formatter);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ModuleDirectiveSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_module_directive_sections(sections)
}

fn push_module_directive_segment(
    sections: &mut Vec<ModuleDirectiveSection>,
    segment: &mut Vec<ModuleDirective>,
    formatter: &JavaFormatter<'_>,
) {
    if segment.is_empty() {
        return;
    }
    sections.push(ModuleDirectiveSection {
        doc: format_module_directive_segments(std::mem::take(segment), formatter),
        hard_line_after: false,
    });
}

fn join_module_directive_sections(sections: Vec<ModuleDirectiveSection>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            joined.push(if previous_hard_line_after {
                hard_line()
            } else {
                empty_line()
            });
        }
        joined.push(section.doc);
        previous_hard_line_after = section.hard_line_after;
    }
    concat(joined)
}

struct ModuleDirectiveSection {
    doc: Doc,
    hard_line_after: bool,
}

fn format_module_directive_segments(
    directives: Vec<ModuleDirective>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut runs: Vec<Vec<FormattedModuleDirective>> = Vec::new();
    let mut current_run = Vec::new();

    for directive in directives {
        let tokens = directive.tokens();
        if formatter.comments().has_leading_comment_for_tokens(&tokens) && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        current_run.push(FormattedModuleDirective::from_directive(
            &directive, formatter,
        ));
    }
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    join_empty_lines(runs.into_iter().map(format_module_directive_run).collect())
}

fn module_directive_token_range(
    directive: &ModuleDirective,
    module_start: usize,
) -> Option<Range<usize>> {
    let tokens = directive.tokens();
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - module_start
            ..last.token_text_range().end().get() - module_start,
    )
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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ModuleDirectiveKindOrder {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
}

struct FormattedModuleDirective {
    leading_comments: Vec<jolt_java_syntax::JavaComment>,
    trailing_comments: Vec<jolt_java_syntax::JavaComment>,
    kind_order: ModuleDirectiveKindOrder,
    primary_name: String,
    doc: Doc,
}

impl FormattedModuleDirective {
    fn from_directive(directive: &ModuleDirective, formatter: &JavaFormatter<'_>) -> Self {
        let role = directive
            .directive_role()
            .expect("clean module directive should expose a directive role");
        let primary_name = module_directive_primary_name(&role);
        let kind_order = module_directive_kind_order(&role);
        let doc = match role {
            ModuleDirectiveRole::Requires {
                module,
                is_static,
                is_transitive,
            } => {
                let mut parts = vec![text("requires ")];
                if is_static {
                    parts.push(text("static "));
                }
                if is_transitive {
                    parts.push(text("transitive "));
                }
                parts.push(format_name(&module));
                parts.push(text(";"));
                concat(parts)
            }
            ModuleDirectiveRole::Exports { package, targets } => {
                format_named_targets_directive("exports", &package, targets, " to ")
            }
            ModuleDirectiveRole::Opens { package, targets } => {
                format_named_targets_directive("opens", &package, targets, " to ")
            }
            ModuleDirectiveRole::Uses { service } => {
                concat([text("uses "), format_name(&service), text(";")])
            }
            ModuleDirectiveRole::Provides {
                service,
                implementations,
            } => {
                let implementations = implementations
                    .into_iter()
                    .map(|name| format_name(&name))
                    .collect::<Vec<_>>();
                concat([
                    text("provides "),
                    format_name(&service),
                    text(" with "),
                    join_docs(implementations, &text(", ")),
                    text(";"),
                ])
            }
        };

        let tokens = directive.tokens();
        Self {
            leading_comments: formatter
                .comments()
                .leading_comments_for_tokens(&tokens)
                .to_vec(),
            trailing_comments: formatter
                .comments()
                .trailing_comments_for_tokens(&tokens)
                .to_vec(),
            kind_order,
            primary_name,
            doc,
        }
    }

    fn into_doc(self) -> Doc {
        let doc = concat([
            self.doc,
            format_inline_trailing_comments(&self.trailing_comments),
        ]);
        if self.leading_comments.is_empty() {
            doc
        } else {
            concat([
                join_hard_lines(self.leading_comments.iter().map(format_comment).collect()),
                hard_line(),
                doc,
            ])
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

fn format_named_targets_directive(
    keyword: &str,
    subject: &jolt_java_syntax::NameSyntax,
    targets: Vec<jolt_java_syntax::NameSyntax>,
    separator: &str,
) -> Doc {
    if targets.is_empty() {
        return concat([
            text(keyword.to_owned()),
            text(" "),
            format_name(subject),
            text(";"),
        ]);
    }

    concat([
        text(keyword.to_owned()),
        text(" "),
        format_name(subject),
        text(separator.to_owned()),
        join_docs(
            targets.into_iter().map(|name| format_name(&name)).collect(),
            &text(", "),
        ),
        text(";"),
    ])
}

fn module_directive_primary_name(role: &ModuleDirectiveRole) -> String {
    match role {
        ModuleDirectiveRole::Requires { module, .. } => name_key(module),
        ModuleDirectiveRole::Exports { package, .. }
        | ModuleDirectiveRole::Opens { package, .. } => name_key(package),
        ModuleDirectiveRole::Uses { service } | ModuleDirectiveRole::Provides { service, .. } => {
            name_key(service)
        }
    }
}

const fn module_directive_kind_order(role: &ModuleDirectiveRole) -> ModuleDirectiveKindOrder {
    match role {
        ModuleDirectiveRole::Requires { .. } => ModuleDirectiveKindOrder::Requires,
        ModuleDirectiveRole::Exports { .. } => ModuleDirectiveKindOrder::Exports,
        ModuleDirectiveRole::Opens { .. } => ModuleDirectiveKindOrder::Opens,
        ModuleDirectiveRole::Uses { .. } => ModuleDirectiveKindOrder::Uses,
        ModuleDirectiveRole::Provides { .. } => ModuleDirectiveKindOrder::Provides,
    }
}
