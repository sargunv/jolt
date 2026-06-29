use super::{
    CompilationUnit, CompilationUnitMember, Doc, FormatResult, ImportDeclaration,
    JavaFormatContext, PackageDeclaration, concat, format_annotation_list,
    format_field_declaration, format_method_declaration, format_name, format_type_declaration,
    hard_line, join, missing_layout, reject_unhandled_comments_before_start,
    take_leading_comment_docs, text, with_attached_comments, with_leading_and_trailing_comments,
    with_vertical_annotations,
};

pub(crate) fn format_compilation_unit(
    syntax: &CompilationUnit,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Some(module) = syntax.module_declaration() {
        return Err(missing_layout(
            "Java formatter does not support module declarations yet",
            module.text_range(),
        ));
    }

    if let Some(child) = syntax.unsupported_layout_child() {
        return Err(missing_layout(
            format!(
                "Java formatter does not support compilation unit child {:?} yet",
                child.kind()
            ),
            child.text_range(),
        ));
    }

    let package = syntax
        .package_declaration()
        .map(|package| format_package_declaration(&package, context))
        .transpose()?;
    let imports = syntax
        .imports()
        .map(|import| format_import_declaration(&import, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let members = syntax
        .compact_members()
        .map(|member| format_compilation_unit_member(&member, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let mut sections = Vec::new();
    if let Some(package) = package {
        sections.push(package);
    }
    if !imports.is_empty() {
        sections.push(join(hard_line(), imports));
    }
    if !members.is_empty() {
        sections.push(join(concat([hard_line(), hard_line()]), members));
    }

    Ok(join(concat([hard_line(), hard_line()]), sections))
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
    .ok_or_else(|| {
        let range = match member {
            CompilationUnitMember::EmptyDeclaration(declaration) => declaration.text_range(),
            CompilationUnitMember::FieldDeclaration(declaration) => declaration.text_range(),
            CompilationUnitMember::MethodDeclaration(declaration) => declaration.text_range(),
            CompilationUnitMember::TypeDeclaration(declaration) => declaration.text_range(),
        };
        missing_layout(
            "Java formatter found an empty compilation unit member",
            range,
        )
    })?;
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
    let code_range = package.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty package declaration",
            package.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let annotations = format_annotation_list(package.annotations(), context, "declaration")?;
    let name = package.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a package declaration without a name",
            package.text_range(),
        )
    })?;
    if !annotations.is_empty() {
        reject_unhandled_comments_before_start(
            context,
            name.code_text_range().ok_or_else(|| {
                missing_layout(
                    "Java formatter found an empty package name",
                    name.text_range(),
                )
            })?,
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
    if !import.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support malformed import declarations",
            import.text_range(),
        ));
    }

    let code_range = import.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty import declaration",
            import.text_range(),
        )
    })?;
    let name = import.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an import declaration without a name",
            import.text_range(),
        )
    })?;
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
