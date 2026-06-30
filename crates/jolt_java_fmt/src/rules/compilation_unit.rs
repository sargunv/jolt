use super::{
    CompilationUnit, CompilationUnitMember, Doc, FormatResult, ImportDeclaration,
    JavaFormatContext, ModuleDeclaration, ModuleDirective, PackageDeclaration, concat,
    format_annotation_list, format_field_declaration, format_method_declaration, format_name,
    format_type_declaration, hard_line, java_lists, join, reject_unhandled_comments_before_start,
    take_leading_comment_docs, take_own_line_comment_docs_in_range, text, with_attached_comments,
    with_leading_and_trailing_comments, with_vertical_annotations, wrap,
};
use crate::policy::JavaFormatPolicy;
use jolt_diagnostics::TextRange;

pub(crate) fn format_compilation_unit(
    syntax: &CompilationUnit,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    assert!(
        syntax.unsupported_layout_child().is_none(),
        "parser-clean compilation unit should not contain unsupported top-level children"
    );

    let package_node = syntax.package_declaration();
    let import_nodes = syntax.imports().collect::<Vec<_>>();
    let module_node = syntax.module_declaration();
    let member_nodes = syntax.compact_members().collect::<Vec<_>>();

    let package = package_node
        .as_ref()
        .map(|package| format_package_declaration(package, context))
        .transpose()?;
    let imports = import_nodes
        .iter()
        .map(|import| format_import_section_item(import, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let module = module_node
        .as_ref()
        .map(|module| format_module_declaration(module, context))
        .transpose()?;
    let members = member_nodes
        .iter()
        .map(|member| format_compilation_unit_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let mut sections = Vec::new();
    if let Some(package) = package {
        sections.push(package);
    }
    if !imports.is_empty() {
        sections.push(format_import_section(imports, context.policy()));
    }
    if let Some(module) = module {
        sections.push(module);
    }
    if !members.is_empty() {
        sections.push(join(concat([hard_line(), hard_line()]), members));
    }
    if let Some(tail_start) = compilation_unit_tail_start(
        &member_nodes,
        module_node.as_ref(),
        &import_nodes,
        package_node.as_ref(),
    ) {
        let trailing_comments = take_own_line_comment_docs_in_range(
            context,
            TextRange::new(tail_start.end(), syntax.text_range().end()),
        )?;
        if !trailing_comments.is_empty() {
            let comments = join(hard_line(), trailing_comments);
            if let Some(last) = sections.last_mut() {
                *last = concat([last.clone(), hard_line(), comments]);
            } else {
                sections.push(comments);
            }
        }
    }

    Ok(join(concat([hard_line(), hard_line()]), sections))
}

fn compilation_unit_tail_start(
    members: &[CompilationUnitMember],
    module: Option<&ModuleDeclaration>,
    imports: &[ImportDeclaration],
    package: Option<&PackageDeclaration>,
) -> Option<TextRange> {
    members
        .iter()
        .filter_map(compilation_unit_member_code_range)
        .next_back()
        .or_else(|| module.and_then(jolt_java_syntax::ModuleDeclaration::code_text_range))
        .or_else(|| {
            imports
                .iter()
                .filter_map(ImportDeclaration::code_text_range)
                .next_back()
        })
        .or_else(|| package.and_then(jolt_java_syntax::PackageDeclaration::code_text_range))
}

fn compilation_unit_member_code_range(member: &CompilationUnitMember) -> Option<TextRange> {
    match member {
        CompilationUnitMember::EmptyDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::FieldDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::MethodDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::TypeDeclaration(declaration) => declaration.code_text_range(),
    }
}

struct ImportSectionItem {
    doc: Doc,
    group: Option<String>,
}

fn format_import_section(imports: Vec<ImportSectionItem>, policy: JavaFormatPolicy) -> Doc {
    let mut imports = imports.into_iter();
    let Some(first) = imports.next() else {
        return text("");
    };

    let mut docs = vec![first.doc];
    let mut previous_group = first.group;
    for import in imports {
        let separator = if policy.separates_static_import_section()
            && previous_group.is_some()
            && import.group.is_some()
            && previous_group != import.group
        {
            concat([hard_line(), hard_line()])
        } else {
            hard_line()
        };
        docs.push(separator);
        docs.push(import.doc);
        previous_group = import.group;
    }

    concat(docs)
}

fn format_import_section_item(
    import: &ImportDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<ImportSectionItem> {
    let group = import_group(import);
    let doc = format_import_declaration(import, context)?;
    Ok(ImportSectionItem { doc, group })
}

fn import_group(import: &ImportDeclaration) -> Option<String> {
    if import.is_module() {
        return Some("module".to_owned());
    }
    if import.is_static() {
        return Some("static".to_owned());
    }
    import
        .name()?
        .segments()
        .next()
        .map(|token| token.text().to_owned())
}

pub(super) fn format_module_declaration(
    module: &ModuleDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = module
        .code_text_range()
        .unwrap_or_else(|| module.text_range());
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let annotations = format_annotation_list(module.annotations(), context, "declaration")?;
    let name = module
        .name()
        .expect("parser-clean module declaration should have a name");
    let directives = module
        .directives()
        .map(|directive| format_module_directive(&directive))
        .collect::<Vec<_>>();

    let mut header = Vec::new();
    if module.is_open() {
        header.push(text("open "));
    }
    header.push(text("module "));
    header.push(format_name(&name));
    header.push(text(" "));

    let doc = with_vertical_annotations(
        annotations,
        concat([concat(header), wrap::braced_block(directives)]),
    );
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn format_module_directive(directive: &ModuleDirective) -> Doc {
    match directive {
        ModuleDirective::RequiresDirective(directive) => {
            let mut parts = vec![text("requires ")];
            if directive.is_transitive() {
                parts.push(text("transitive "));
            }
            if directive.is_static() {
                parts.push(text("static "));
            }
            let name = directive
                .name()
                .expect("parser-clean requires directive should have a module name");
            parts.push(format_name(&name));
            parts.push(text(";"));
            concat(parts)
        }
        ModuleDirective::ExportsDirective(directive) => format_module_package_directive(
            "exports",
            directive.package_name(),
            directive.target_modules().collect(),
            "to",
        ),
        ModuleDirective::OpensDirective(directive) => format_module_package_directive(
            "opens",
            directive.package_name(),
            directive.target_modules().collect(),
            "to",
        ),
        ModuleDirective::UsesDirective(directive) => {
            let name = directive
                .service_name()
                .expect("parser-clean uses directive should have a service name");
            concat([text("uses "), format_name(&name), text(";")])
        }
        ModuleDirective::ProvidesDirective(directive) => format_module_package_directive(
            "provides",
            directive.service_name(),
            directive.implementation_names().collect(),
            "with",
        ),
    }
}

fn format_module_package_directive(
    keyword: &'static str,
    name: Option<super::NameSyntax>,
    targets: Vec<super::NameSyntax>,
    target_keyword: &'static str,
) -> Doc {
    let name = name.expect("parser-clean module package directive should have a required name");

    let mut parts = vec![text(format!("{keyword} ")), format_name(&name)];
    if !targets.is_empty() {
        parts.push(text(format!(" {target_keyword} ")));
        parts.push(java_lists::comma_list(
            targets.into_iter().map(|target| format_name(&target)),
        ));
    }
    parts.push(text(";"));
    concat(parts)
}

pub(super) fn format_compilation_unit_member(
    member: &CompilationUnitMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = match member {
        CompilationUnitMember::EmptyDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::FieldDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::MethodDeclaration(declaration) => declaration.code_text_range(),
        CompilationUnitMember::TypeDeclaration(declaration) => declaration.code_text_range(),
    }
    .unwrap_or_else(|| match member {
        CompilationUnitMember::EmptyDeclaration(declaration) => declaration.text_range(),
        CompilationUnitMember::FieldDeclaration(declaration) => declaration.text_range(),
        CompilationUnitMember::MethodDeclaration(declaration) => declaration.text_range(),
        CompilationUnitMember::TypeDeclaration(declaration) => declaration.text_range(),
    });
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = match member {
        CompilationUnitMember::EmptyDeclaration(_) => Ok(text(";")),
        CompilationUnitMember::FieldDeclaration(field) => format_field_declaration(field, context),
        CompilationUnitMember::MethodDeclaration(method) => {
            format_method_declaration(method, context)
        }
        CompilationUnitMember::TypeDeclaration(declaration) => {
            format_type_declaration(declaration, context)
        }
    }?;

    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_package_declaration(
    package: &PackageDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = package
        .code_text_range()
        .unwrap_or_else(|| package.text_range());
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let annotations = format_annotation_list(package.annotations(), context, "declaration")?;
    let name = package
        .name()
        .expect("parser-clean package declaration should have a name");
    if !annotations.is_empty()
        && let Some(name_range) = name.code_text_range()
    {
        reject_unhandled_comments_before_start(
            context,
            name_range,
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        with_vertical_annotations(
            annotations,
            concat([text("package "), format_name(&name), text(";")]),
        ),
    )
}

pub(super) fn format_import_declaration(
    import: &ImportDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = import
        .code_text_range()
        .unwrap_or_else(|| import.text_range());
    let name = import
        .name()
        .expect("parser-clean import declaration should have a name");
    let mut parts = vec![text("import ")];
    if import.is_module() {
        parts.push(text("module "));
    }
    if import.is_static() {
        parts.push(text("static "));
    }
    parts.push(format_name(&name));
    if import.is_on_demand() {
        parts.push(text(".*"));
    }
    parts.push(text(";"));
    with_attached_comments(context, code_range, concat(parts))
}
