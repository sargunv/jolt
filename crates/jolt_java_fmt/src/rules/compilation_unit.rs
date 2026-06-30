use super::{
    CompilationUnit, CompilationUnitMember, Doc, FormatResult, ImportDeclaration,
    JavaFormatContext, ModuleDeclaration, ModuleDirective, PackageDeclaration, concat,
    format_annotation_list, format_field_declaration, format_method_declaration, format_name,
    format_type_declaration, hard_line, join, reject_unhandled_comments_before_start,
    take_leading_comment_docs, take_own_line_comment_docs_in_range, text, with_attached_comments,
    with_leading_and_trailing_comments, with_vertical_annotations,
};
use crate::helpers::imports::{self, ImportDeclarationLayout, ImportSectionItem};
use crate::helpers::modules::{self, ModuleDirectiveLayout};
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
        sections.push(imports::import_section(imports, context.policy()));
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

fn format_import_section_item(
    import: &ImportDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<ImportSectionItem> {
    let doc = format_import_declaration(import, context)?;
    let top_level = import
        .name()
        .and_then(|name| name.segments().next().map(|token| token.text().to_owned()));
    Ok(ImportSectionItem::new(
        doc,
        import.is_module(),
        import.is_static(),
        top_level,
    ))
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
        .map(|directive| module_directive_layout(&directive))
        .collect::<Vec<_>>();

    let doc = with_vertical_annotations(
        annotations,
        modules::module_declaration(
            module.is_open(),
            format_name(&name),
            directives,
            context.policy(),
        ),
    );
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn module_directive_layout(directive: &ModuleDirective) -> ModuleDirectiveLayout {
    match directive {
        ModuleDirective::RequiresDirective(directive) => {
            let name = directive
                .name()
                .expect("parser-clean requires directive should have a module name");
            ModuleDirectiveLayout::Requires {
                is_transitive: directive.is_transitive(),
                is_static: directive.is_static(),
                name: format_name(&name),
            }
        }
        ModuleDirective::ExportsDirective(directive) => {
            let package_name = directive
                .package_name()
                .expect("parser-clean exports directive should have a package name");
            ModuleDirectiveLayout::Exports {
                package_name: format_name(&package_name),
                targets: directive
                    .target_modules()
                    .map(|target| format_name(&target))
                    .collect(),
            }
        }
        ModuleDirective::OpensDirective(directive) => {
            let package_name = directive
                .package_name()
                .expect("parser-clean opens directive should have a package name");
            ModuleDirectiveLayout::Opens {
                package_name: format_name(&package_name),
                targets: directive
                    .target_modules()
                    .map(|target| format_name(&target))
                    .collect(),
            }
        }
        ModuleDirective::UsesDirective(directive) => {
            let name = directive
                .service_name()
                .expect("parser-clean uses directive should have a service name");
            ModuleDirectiveLayout::Uses {
                service_name: format_name(&name),
            }
        }
        ModuleDirective::ProvidesDirective(directive) => {
            let name = directive
                .service_name()
                .expect("parser-clean provides directive should have a service name");
            ModuleDirectiveLayout::Provides {
                service_name: format_name(&name),
                implementations: directive
                    .implementation_names()
                    .map(|implementation| format_name(&implementation))
                    .collect(),
            }
        }
    }
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
    let doc = imports::import_declaration(ImportDeclarationLayout {
        is_module: import.is_module(),
        is_static: import.is_static(),
        name: format_name(&name),
        is_on_demand: import.is_on_demand(),
    });
    with_attached_comments(context, code_range, doc)
}
