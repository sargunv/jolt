use crate::comments::{
    reject_unhandled_comments_before_end, reject_unhandled_comments_before_start,
    take_dangling_comment_docs, take_leading_comment_docs, with_attached_comments,
    with_leading_and_trailing_comments,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::{FormatResult, missing_layout};
use crate::layout as wrap;
use jolt_fmt_ir::{Doc, concat, hard_line, join, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationElementValue, AnnotationElementValuePair, Block,
    BlockItem, BlockStatement, BreakStatement, ClassBody, ClassBodyMember, ClassDeclaration,
    CompilationUnit, ConstructorBody, ConstructorDeclaration, ContinueStatement, EmptyStatement,
    Expression, ExtendsClause, FieldDeclaration, FormalParameter, FormalParameterList, IfStatement,
    ImplementsClause, ImportDeclaration, JavaSyntaxKind, JavaSyntaxToken, LocalVariableDeclaration,
    MethodDeclaration, ModifierList, NameSyntax, PackageDeclaration, PermitsClause,
    ReturnStatement, Statement, ThrowStatement, ThrowsClause, Type, TypeDeclaration,
    TypeLayoutPart, TypeParameterList, VariableDeclarator, YieldStatement,
};

#[cfg(test)]
use crate::{
    JavaFormatDiagnosticCode, JavaFormatOptions, JavaFormatStatus, format_java_source,
    format_java_source_with_options,
};
#[cfg(test)]
use jolt_diagnostics::{DiagnosticCode, DiagnosticStage, Severity};
#[cfg(test)]
use jolt_fmt_ir::RenderOptions;

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
    let types = syntax
        .type_declarations()
        .map(|declaration| format_type_declaration(&declaration, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let mut sections = Vec::new();
    if let Some(package) = package {
        sections.push(package);
    }
    if !imports.is_empty() {
        sections.push(join(hard_line(), imports));
    }
    if !types.is_empty() {
        sections.push(join(concat([hard_line(), hard_line()]), types));
    }

    Ok(join(concat([hard_line(), hard_line()]), sections))
}

fn format_package_declaration(
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

fn format_import_declaration(
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

fn format_name(name: &NameSyntax) -> Doc {
    join(
        text("."),
        name.segments().map(|segment| text(segment.text())),
    )
}

fn format_type_declaration(
    declaration: &TypeDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class, context),
        TypeDeclaration::RecordDeclaration(record) => Err(missing_layout(
            "Java formatter does not support record declarations yet",
            record.text_range(),
        )),
        TypeDeclaration::EnumDeclaration(enumeration) => Err(missing_layout(
            "Java formatter does not support enum declarations yet",
            enumeration.text_range(),
        )),
        TypeDeclaration::InterfaceDeclaration(interface) => Err(missing_layout(
            "Java formatter does not support interface declarations yet",
            interface.text_range(),
        )),
        TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => Err(missing_layout(
            "Java formatter does not support annotation interface declarations yet",
            annotation.text_range(),
        )),
    }
}

fn format_class_declaration(
    class: &ClassDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = class.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty class declaration",
            class.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(class.modifiers(), "class", context)?;

    if !class.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this class declaration shape yet",
            class.text_range(),
        ));
    }

    let name = class.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a class declaration without a name",
            class.text_range(),
        )
    })?;
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = class.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a class declaration without a body",
            class.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty class body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside class headers yet",
    )?;
    let type_parameters = class
        .type_parameters()
        .map(|parameters| format_type_parameter_list(&parameters))
        .transpose()?;
    let extends_clause = class
        .extends_clause()
        .map(|clause| format_extends_clause(&clause, context))
        .transpose()?;
    let implements_clause = class
        .implements_clause()
        .map(|clause| format_implements_clause(&clause, context))
        .transpose()?;
    let permits_clause = class
        .permits_clause()
        .map(|clause| format_permits_clause(&clause))
        .transpose()?;
    let body_members = format_class_body(&body, context)?;

    let mut header = Vec::new();
    header.extend(modifiers.modifier_tokens.iter().map(format_token));
    header.push(text("class"));
    header.push(if let Some(type_parameters) = type_parameters {
        concat([text(name.text()), type_parameters])
    } else {
        text(name.text())
    });
    if let Some(extends_clause) = extends_clause {
        header.push(extends_clause);
    }
    if let Some(implements_clause) = implements_clause {
        header.push(implements_clause);
    }
    if let Some(permits_clause) = permits_clause {
        header.push(permits_clause);
    }
    let header = wrap::declaration_header(header);

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(concat([
            header,
            text(" "),
            wrap::braced_block(body_members),
        ])),
    )
}

fn format_class_body(
    body: &ClassBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Vec<Doc>> {
    if !body.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this class body shape yet",
            body.text_range(),
        ));
    }

    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body.code_text_range().ok_or_else(|| {
            missing_layout(
                "Java formatter found an empty class body",
                body.text_range(),
            )
        })?;
        return take_dangling_comment_docs(context, code_range);
    }

    members
        .into_iter()
        .map(|member| format_class_body_member(&member, context))
        .collect()
}

fn format_class_body_member(
    member: &ClassBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty class body member",
            member.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = match member {
        ClassBodyMember::FieldDeclaration(field) => format_field_declaration(field, context),
        ClassBodyMember::MethodDeclaration(method) => format_method_declaration(method, context),
        ClassBodyMember::ConstructorDeclaration(constructor) => {
            format_constructor_declaration(constructor, context)
        }
        ClassBodyMember::EmptyDeclaration(_) => Ok(text(";")),
        ClassBodyMember::ClassDeclaration(class) => format_class_declaration(class, context),
        ClassBodyMember::RecordDeclaration(record) => Err(missing_layout(
            "Java formatter does not support nested record declarations yet",
            record.text_range(),
        )),
        ClassBodyMember::EnumDeclaration(enumeration) => Err(missing_layout(
            "Java formatter does not support nested enum declarations yet",
            enumeration.text_range(),
        )),
        ClassBodyMember::InterfaceDeclaration(interface) => Err(missing_layout(
            "Java formatter does not support nested interface declarations yet",
            interface.text_range(),
        )),
        ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => Err(missing_layout(
            "Java formatter does not support nested annotation interface declarations yet",
            annotation.text_range(),
        )),
        ClassBodyMember::CompactConstructorDeclaration(constructor) => Err(missing_layout(
            "Java formatter does not support compact constructors yet",
            constructor.text_range(),
        )),
        ClassBodyMember::StaticInitializer(initializer) => {
            format_static_initializer(initializer, context)
        }
        ClassBodyMember::InstanceInitializer(initializer) => {
            format_instance_initializer(initializer, context)
        }
    }?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn format_field_declaration(
    field: &FieldDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !field.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this field declaration shape yet",
            field.text_range(),
        ));
    }

    let modifiers = format_modifier_list(field.modifiers(), "field", context)?;
    let ty = field.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field declaration without a type",
            field.text_range(),
        )
    })?;
    if modifiers.has_annotations() {
        let ty_range = ty.code_text_range().ok_or_else(|| {
            missing_layout("Java formatter found an empty field type", ty.text_range())
        })?;
        reject_unhandled_comments_before_start(
            context,
            ty_range,
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let declarators = field.declarators().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field declaration without declarators",
            field.text_range(),
        )
    })?;
    let declarators = format_variable_declarator_list(&declarators, "field")?;

    let mut prefix = Vec::new();
    prefix.extend(modifiers.modifier_tokens.iter().map(format_token));
    prefix.push(format_type(&ty, context)?);
    Ok(modifiers.with_annotations(wrap::variable_declaration(prefix, declarators)))
}

fn format_static_initializer(
    initializer: &jolt_java_syntax::StaticInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = initializer.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a static initializer without a body",
            initializer.text_range(),
        )
    })?;
    Ok(concat([text("static "), format_block(&body, context)?]))
}

fn format_instance_initializer(
    initializer: &jolt_java_syntax::InstanceInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = initializer.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found an instance initializer without a body",
            initializer.text_range(),
        )
    })?;
    format_block(&body, context)
}

fn format_method_declaration(
    method: &MethodDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !method.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method declaration shape yet",
            method.text_range(),
        ));
    }

    let return_type = method.return_type().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method declaration without a return type",
            method.text_range(),
        )
    })?;
    let name = method.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method declaration without a name",
            method.text_range(),
        )
    })?;
    let return_type = format_type(&return_type, context)?;
    let header = format_callable_header(
        method.modifiers(),
        "method",
        context,
        method.type_parameters(),
        Some(return_type),
        &name,
        method.parameters(),
        method.throws_clause(),
    )?;

    if method.has_semicolon_body() {
        let code_range = method.code_text_range().ok_or_else(|| {
            missing_layout(
                "Java formatter found an empty method declaration",
                method.text_range(),
            )
        })?;
        reject_unhandled_comments_before_end(
            context,
            code_range,
            "Java formatter does not support comments inside method signatures yet",
        )?;
        return Ok(concat([header, text(";")]));
    }

    let body = method.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method declaration without a body",
            method.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty method body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside method signatures yet",
    )?;
    Ok(concat([header, text(" "), format_block(&body, context)?]))
}

fn format_constructor_declaration(
    constructor: &ConstructorDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !constructor.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this constructor declaration shape yet",
            constructor.text_range(),
        ));
    }

    let name = constructor.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a constructor declaration without a name",
            constructor.text_range(),
        )
    })?;
    let body = constructor.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a constructor declaration without a body",
            constructor.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty constructor body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside constructor signatures yet",
    )?;
    let header = format_callable_header(
        constructor.modifiers(),
        "constructor",
        context,
        constructor.type_parameters(),
        None,
        &name,
        constructor.parameters(),
        constructor.throws_clause(),
    )?;
    Ok(concat([
        header,
        text(" "),
        format_constructor_body(&body, context)?,
    ]))
}

fn format_callable_header(
    modifiers: Option<ModifierList>,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
    type_parameters: Option<TypeParameterList>,
    leading_type: Option<Doc>,
    name: &JavaSyntaxToken,
    parameters: Option<FormalParameterList>,
    throws_clause: Option<ThrowsClause>,
) -> FormatResult<Doc> {
    let modifiers = format_modifier_list(modifiers, declaration_kind, context)?;
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let type_parameters = type_parameters
        .map(|parameters| format_type_parameter_list(&parameters))
        .transpose()?;
    let parameters = parameters
        .map(|parameters| format_formal_parameter_list(&parameters, context))
        .transpose()?
        .unwrap_or_else(|| wrap::parenthesized_comma_list(std::iter::empty()));
    let throws_clause = throws_clause
        .map(|throws| format_throws_clause(&throws, context))
        .transpose()?;

    let mut header = Vec::new();
    header.extend(modifiers.modifier_tokens.iter().map(format_token));
    if let Some(type_parameters) = type_parameters {
        header.push(type_parameters);
    }
    if let Some(leading_type) = leading_type {
        header.push(leading_type);
    }
    header.push(concat([text(name.text()), parameters]));
    if let Some(throws_clause) = throws_clause {
        header.push(throws_clause);
    }

    Ok(modifiers.with_annotations(wrap::declaration_header(header)))
}

fn format_type_parameter_list(parameters: &TypeParameterList) -> FormatResult<Doc> {
    if !parameters.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports simple type parameter lists yet",
            parameters.text_range(),
        ));
    }

    let parameter_docs = parameters
        .parameters()
        .map(|parameter| {
            if !parameter.has_simple_layout_shape() {
                return Err(missing_layout(
                    "Java formatter only supports unbounded type parameters yet",
                    parameter.text_range(),
                ));
            }
            let name = parameter.name().ok_or_else(|| {
                missing_layout(
                    "Java formatter found a type parameter without a name",
                    parameter.text_range(),
                )
            })?;
            Ok(format_token(&name))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    if parameter_docs.is_empty() {
        return Err(missing_layout(
            "Java formatter found an empty type parameter list",
            parameters.text_range(),
        ));
    }

    Ok(wrap::angle_comma_list(parameter_docs))
}

fn format_extends_clause(
    clause: &ExtendsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_clause(
        "extends",
        clause.types(),
        clause.has_supported_layout_shape(),
        clause.text_range(),
        context,
    )
}

fn format_implements_clause(
    clause: &ImplementsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_clause(
        "implements",
        clause.types(),
        clause.has_supported_layout_shape(),
        clause.text_range(),
        context,
    )
}

fn format_type_clause(
    keyword: &'static str,
    types: impl Iterator<Item = Type>,
    has_supported_shape: bool,
    range: jolt_diagnostics::TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !has_supported_shape {
        return Err(missing_layout(
            format!("Java formatter does not support this {keyword} clause shape yet"),
            range,
        ));
    }

    let types = types
        .map(|ty| format_type(&ty, context))
        .collect::<FormatResult<Vec<_>>>()?;
    if types.is_empty() {
        return Err(missing_layout(
            format!("Java formatter found an empty {keyword} clause"),
            range,
        ));
    }

    Ok(concat([text(keyword), text(" "), wrap::comma_list(types)]))
}

fn format_permits_clause(clause: &PermitsClause) -> FormatResult<Doc> {
    if !clause.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this permits clause shape yet",
            clause.text_range(),
        ));
    }

    let names = clause
        .names()
        .map(|name| format_name(&name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        return Err(missing_layout(
            "Java formatter found an empty permits clause",
            clause.text_range(),
        ));
    }

    Ok(concat([text("permits "), wrap::comma_list(names)]))
}

fn format_formal_parameter_list(
    parameters: &FormalParameterList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !parameters.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports simple formal parameter lists yet",
            parameters.text_range(),
        ));
    }

    let parameter_docs = parameters
        .parameters()
        .map(|parameter| format_formal_parameter(&parameter, context))
        .collect::<FormatResult<Vec<_>>>()?;
    if parameter_docs.is_empty() {
        return Err(missing_layout(
            "Java formatter found an empty formal parameter list",
            parameters.text_range(),
        ));
    }

    Ok(wrap::parenthesized_comma_list(parameter_docs))
}

fn format_formal_parameter(
    parameter: &FormalParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !parameter.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports simple formal parameters yet",
            parameter.text_range(),
        ));
    }

    let ty = parameter.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found a formal parameter without a type",
            parameter.text_range(),
        )
    })?;
    let name = parameter.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a formal parameter without a name",
            parameter.text_range(),
        )
    })?;

    let mut parts = Vec::new();
    if let Some(final_token) = parameter.final_token() {
        parts.push(format_token(&final_token));
    }
    let ty = if parameter.is_varargs() {
        concat([format_type(&ty, context)?, text("...")])
    } else {
        format_type(&ty, context)?
    };
    parts.push(ty);
    parts.push(format_token(&name));
    Ok(wrap::space_separated(parts))
}

fn format_throws_clause(
    throws: &ThrowsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !throws.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports simple throws clauses yet",
            throws.text_range(),
        ));
    }

    let types = throws
        .types()
        .map(|ty| format_type(&ty, context))
        .collect::<FormatResult<Vec<_>>>()?;
    if types.is_empty() {
        return Err(missing_layout(
            "Java formatter found an empty throws clause",
            throws.text_range(),
        ));
    }

    Ok(concat([text("throws "), wrap::comma_list(types)]))
}

fn format_block(block: &Block, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    if !block.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this block shape yet",
            block.text_range(),
        ));
    }
    let code_range = block
        .code_text_range()
        .ok_or_else(|| missing_layout("Java formatter found an empty block", block.text_range()))?;
    format_block_statements(code_range, block.block_statements(), context)
}

fn format_constructor_body(
    body: &ConstructorBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !body.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support constructor invocations or this constructor body shape yet",
            body.text_range(),
        ));
    }
    let code_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty constructor body",
            body.text_range(),
        )
    })?;
    format_block_statements(code_range, body.block_statements(), context)
}

fn format_block_statements(
    container_range: jolt_diagnostics::TextRange,
    statements: impl Iterator<Item = BlockStatement>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let statements = statements.collect::<Vec<_>>();
    if statements.is_empty() {
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context,
            container_range,
        )?));
    }

    let statements = statements
        .into_iter()
        .map(|statement| format_block_statement(&statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(wrap::braced_block(statements))
}

fn format_block_statement(
    statement: &BlockStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this block statement shape yet",
            statement.text_range(),
        ));
    }

    let item = statement.item().ok_or_else(|| {
        missing_layout(
            "Java formatter found a block statement without an item",
            statement.text_range(),
        )
    })?;
    let code_range = statement.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty block statement",
            statement.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = format_block_item(&item, context)?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn format_block_item(item: &BlockItem, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    match item {
        BlockItem::LocalVariableDeclaration(declaration) => {
            format_local_variable_declaration(declaration, context)
        }
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => Err(missing_layout(
            "Java formatter does not support local class or interface declarations yet",
            declaration.text_range(),
        )),
        BlockItem::Block(block) => format_statement_rule(StatementRule::Block(block), context),
        BlockItem::EmptyStatement(empty) => {
            format_statement_rule(StatementRule::Empty(empty), context)
        }
        BlockItem::ExpressionStatement(expression) => {
            format_statement_rule(StatementRule::Expression(expression), context)
        }
        BlockItem::IfStatement(if_statement) => {
            format_statement_rule(StatementRule::If(if_statement), context)
        }
        BlockItem::BreakStatement(break_statement) => {
            format_statement_rule(StatementRule::Break(break_statement), context)
        }
        BlockItem::ContinueStatement(continue_statement) => {
            format_statement_rule(StatementRule::Continue(continue_statement), context)
        }
        BlockItem::ReturnStatement(return_statement) => {
            format_statement_rule(StatementRule::Return(return_statement), context)
        }
        BlockItem::ThrowStatement(throw_statement) => {
            format_statement_rule(StatementRule::Throw(throw_statement), context)
        }
        BlockItem::YieldStatement(yield_statement) => {
            format_statement_rule(StatementRule::Yield(yield_statement), context)
        }
        BlockItem::LabeledStatement(labeled) => Err(missing_layout(
            "Java formatter does not support labeled statements yet",
            labeled.text_range(),
        )),
        BlockItem::AssertStatement(assert_statement) => Err(missing_layout(
            "Java formatter does not support assert statements yet",
            assert_statement.text_range(),
        )),
        BlockItem::SwitchStatement(switch_statement) => Err(missing_layout(
            "Java formatter does not support switch statements yet",
            switch_statement.text_range(),
        )),
        BlockItem::WhileStatement(while_statement) => Err(missing_layout(
            "Java formatter does not support while statements yet",
            while_statement.text_range(),
        )),
        BlockItem::DoStatement(do_statement) => Err(missing_layout(
            "Java formatter does not support do statements yet",
            do_statement.text_range(),
        )),
        BlockItem::ForStatement(for_statement) => Err(missing_layout(
            "Java formatter does not support for statements yet",
            for_statement.text_range(),
        )),
        BlockItem::SynchronizedStatement(synchronized) => Err(missing_layout(
            "Java formatter does not support synchronized statements yet",
            synchronized.text_range(),
        )),
        BlockItem::TryStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try statements yet",
            try_statement.text_range(),
        )),
        BlockItem::TryWithResourcesStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try-with-resources statements yet",
            try_statement.text_range(),
        )),
    }
}

fn format_unbraced_statement(
    statement: &Statement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_statement_rule(statement_rule(statement)?, context)
}

fn statement_rule(statement: &Statement) -> FormatResult<StatementRule<'_>> {
    match statement {
        Statement::Block(block) => Ok(StatementRule::Block(block)),
        Statement::EmptyStatement(empty) => Ok(StatementRule::Empty(empty)),
        Statement::ExpressionStatement(expression) => Ok(StatementRule::Expression(expression)),
        Statement::IfStatement(if_statement) => Ok(StatementRule::If(if_statement)),
        Statement::BreakStatement(break_statement) => Ok(StatementRule::Break(break_statement)),
        Statement::ContinueStatement(continue_statement) => {
            Ok(StatementRule::Continue(continue_statement))
        }
        Statement::ReturnStatement(return_statement) => Ok(StatementRule::Return(return_statement)),
        Statement::ThrowStatement(throw_statement) => Ok(StatementRule::Throw(throw_statement)),
        Statement::YieldStatement(yield_statement) => Ok(StatementRule::Yield(yield_statement)),
        Statement::LabeledStatement(labeled) => Err(missing_layout(
            "Java formatter does not support labeled statements yet",
            labeled.text_range(),
        )),
        Statement::AssertStatement(assert_statement) => Err(missing_layout(
            "Java formatter does not support assert statements yet",
            assert_statement.text_range(),
        )),
        Statement::SwitchStatement(switch_statement) => Err(missing_layout(
            "Java formatter does not support switch statements yet",
            switch_statement.text_range(),
        )),
        Statement::WhileStatement(while_statement) => Err(missing_layout(
            "Java formatter does not support while statements yet",
            while_statement.text_range(),
        )),
        Statement::DoStatement(do_statement) => Err(missing_layout(
            "Java formatter does not support do statements yet",
            do_statement.text_range(),
        )),
        Statement::ForStatement(for_statement) => Err(missing_layout(
            "Java formatter does not support for statements yet",
            for_statement.text_range(),
        )),
        Statement::SynchronizedStatement(synchronized) => Err(missing_layout(
            "Java formatter does not support synchronized statements yet",
            synchronized.text_range(),
        )),
        Statement::TryStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try statements yet",
            try_statement.text_range(),
        )),
        Statement::TryWithResourcesStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try-with-resources statements yet",
            try_statement.text_range(),
        )),
    }
}

enum StatementRule<'a> {
    Block(&'a Block),
    Empty(&'a EmptyStatement),
    Expression(&'a jolt_java_syntax::ExpressionStatement),
    If(&'a IfStatement),
    Break(&'a BreakStatement),
    Continue(&'a ContinueStatement),
    Return(&'a ReturnStatement),
    Throw(&'a ThrowStatement),
    Yield(&'a YieldStatement),
}

fn format_statement_rule(
    rule: StatementRule<'_>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match rule {
        StatementRule::Block(block) => format_block(block, context),
        StatementRule::Empty(empty) => format_empty_statement(empty),
        StatementRule::Expression(expression) => format_expression_statement(expression),
        StatementRule::If(if_statement) => format_if_statement(if_statement, context),
        StatementRule::Break(break_statement) => format_break_statement(break_statement),
        StatementRule::Continue(continue_statement) => {
            format_continue_statement(continue_statement)
        }
        StatementRule::Return(return_statement) => format_return_statement(return_statement),
        StatementRule::Throw(throw_statement) => format_throw_statement(throw_statement),
        StatementRule::Yield(yield_statement) => format_yield_statement(yield_statement),
    }
}

fn format_empty_statement(statement: &EmptyStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this empty statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(text(";"))
}

fn format_if_statement(
    statement: &IfStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this if statement shape yet",
            statement.text_range(),
        ));
    }

    let condition = statement.condition().ok_or_else(|| {
        missing_layout(
            "Java formatter found an if statement without a condition",
            statement.text_range(),
        )
    })?;
    let then_statement = statement.then_statement().ok_or_else(|| {
        missing_layout(
            "Java formatter found an if statement without a then statement",
            statement.text_range(),
        )
    })?;
    let then_range = then_statement.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty if statement body",
            then_statement.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        then_range,
        "Java formatter does not support comments before if statement bodies yet",
    )?;
    let then_is_block = matches!(then_statement, Statement::Block(_));
    let then_statement = format_unbraced_statement(&then_statement, context)?;
    let else_statement = statement
        .else_statement()
        .map(|else_statement| {
            let else_range = else_statement.code_text_range().ok_or_else(|| {
                missing_layout(
                    "Java formatter found an empty else statement body",
                    else_statement.text_range(),
                )
            })?;
            reject_unhandled_comments_before_start(
                context,
                else_range,
                "Java formatter does not support comments before else statement bodies yet",
            )?;
            let follows_keyword = matches!(
                else_statement,
                Statement::Block(_) | Statement::IfStatement(_)
            );
            Ok((
                format_unbraced_statement(&else_statement, context)?,
                follows_keyword,
            ))
        })
        .transpose()?;

    Ok(wrap::if_statement(
        format_expression(&condition)?,
        then_statement,
        then_is_block,
        else_statement,
    ))
}

fn format_break_statement(statement: &BreakStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this break statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(wrap::keyword_label_statement(
        "break",
        statement.label().map(|label| format_token(&label)),
    ))
}

fn format_continue_statement(statement: &ContinueStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this continue statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(wrap::keyword_label_statement(
        "continue",
        statement.label().map(|label| format_token(&label)),
    ))
}

fn format_local_variable_declaration(
    declaration: &LocalVariableDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !declaration.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this local variable declaration shape yet",
            declaration.text_range(),
        ));
    }

    let ty = if let Some(ty) = declaration.ty() {
        format_type(&ty, context)?
    } else {
        let token = declaration.var_type_token().ok_or_else(|| {
            missing_layout(
                "Java formatter found a local variable declaration without a type",
                declaration.text_range(),
            )
        })?;
        format_token(&token)
    };
    let declarators = declaration.declarators().ok_or_else(|| {
        missing_layout(
            "Java formatter found a local variable declaration without declarators",
            declaration.text_range(),
        )
    })?;
    let declarators = format_variable_declarator_list(&declarators, "local variable")?;

    let mut prefix = Vec::new();
    if let Some(final_token) = declaration.final_token() {
        prefix.push(format_token(&final_token));
    }
    prefix.push(ty);

    Ok(wrap::variable_declaration(prefix, declarators))
}

fn format_variable_declarator_list(
    declarators: &jolt_java_syntax::VariableDeclaratorList,
    declaration_kind: &str,
) -> FormatResult<Doc> {
    let declarator_docs = declarators
        .declarators()
        .map(|declarator| {
            if !declarator.has_identifier_layout_shape() {
                return Err(missing_layout(
                    format!(
                        "Java formatter only supports identifier {declaration_kind} declarators without array dimensions"
                    ),
                    declarator.text_range(),
                ));
            }
            format_variable_declarator(&declarator)
        })
        .collect::<FormatResult<Vec<_>>>()?;

    if declarator_docs.is_empty() {
        return Err(missing_layout(
            format!("Java formatter found an empty {declaration_kind} declarator list"),
            declarators.text_range(),
        ));
    }

    Ok(wrap::comma_list(declarator_docs))
}

fn format_variable_declarator(declarator: &VariableDeclarator) -> FormatResult<Doc> {
    let name = declarator.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a variable declarator without a name",
            declarator.text_range(),
        )
    })?;
    let Some(initializer) = declarator.initializer() else {
        return Ok(wrap::variable_declarator(text(name.text()), None));
    };
    if !initializer.has_expression_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports expression variable initializers",
            initializer.text_range(),
        ));
    }
    let expression = initializer.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a variable initializer without an expression",
            initializer.text_range(),
        )
    })?;

    Ok(wrap::variable_declarator(
        text(name.text()),
        Some(format_expression(&expression)?),
    ))
}

fn format_return_statement(statement: &ReturnStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this return statement shape yet",
            statement.text_range(),
        ));
    }

    let expression = statement
        .expression()
        .map(|expression| format_expression(&expression))
        .transpose()?;
    Ok(wrap::keyword_expression_statement("return", expression))
}

fn format_throw_statement(statement: &ThrowStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this throw statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a throw statement without an expression",
            statement.text_range(),
        )
    })?;
    Ok(wrap::keyword_expression_statement(
        "throw",
        Some(format_expression(&expression)?),
    ))
}

fn format_yield_statement(statement: &YieldStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this yield statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a yield statement without an expression",
            statement.text_range(),
        )
    })?;
    Ok(wrap::keyword_expression_statement(
        "yield",
        Some(format_expression(&expression)?),
    ))
}

fn format_expression_statement(
    statement: &jolt_java_syntax::ExpressionStatement,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this expression statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found an expression statement without an expression",
            statement.text_range(),
        )
    })?;

    if !matches!(
        expression,
        Expression::AssignmentExpression(_)
            | Expression::MethodInvocationExpression(_)
            | Expression::PostfixExpression(_)
            | Expression::UnaryExpression(_)
    ) {
        return Err(missing_layout(
            "Java formatter does not support this expression statement kind yet",
            expression.text_range(),
        ));
    }

    Ok(wrap::expression_statement(format_expression(&expression)?))
}

fn format_expression(expression: &Expression) -> FormatResult<Doc> {
    match expression {
        Expression::LiteralExpression(literal) => format_literal_expression(literal),
        Expression::NameExpression(name) => format_name_expression(name),
        Expression::ThisExpression(this) => format_this_expression(this),
        Expression::SuperExpression(super_expression) => format_super_expression(super_expression),
        Expression::ParenthesizedExpression(parenthesized) => {
            format_parenthesized_expression(parenthesized)
        }
        Expression::FieldAccessExpression(_) | Expression::MethodInvocationExpression(_) => {
            format_selector_chain(expression)
        }
        Expression::UnaryExpression(unary) => format_unary_expression(unary),
        Expression::PostfixExpression(postfix) => format_postfix_expression(postfix),
        Expression::BinaryExpression(binary) => format_binary_expression(binary),
        Expression::AssignmentExpression(assignment) => format_assignment_expression(assignment),
        _ => Err(missing_layout(
            format!(
                "Java formatter does not support expression kind {:?} yet",
                expression.kind()
            ),
            expression.text_range(),
        )),
    }
}

fn format_selector_chain(expression: &Expression) -> FormatResult<Doc> {
    let (base, selectors) = collect_selector_chain(expression)?;
    Ok(wrap::dot_chain(base, selectors))
}

fn collect_selector_chain(expression: &Expression) -> FormatResult<(Doc, Vec<Doc>)> {
    match expression {
        Expression::NameExpression(name) => Ok((format_name_expression(name)?, Vec::new())),
        Expression::ThisExpression(this) => Ok((format_this_expression(this)?, Vec::new())),
        Expression::SuperExpression(super_expression) => {
            Ok((format_super_expression(super_expression)?, Vec::new()))
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            Ok((format_parenthesized_expression(parenthesized)?, Vec::new()))
        }
        Expression::FieldAccessExpression(field) => collect_field_access_chain(field),
        Expression::MethodInvocationExpression(invocation) => {
            collect_method_invocation_chain(invocation)
        }
        _ => Err(missing_layout(
            "Java formatter does not support this selector chain expression yet",
            expression.text_range(),
        )),
    }
}

fn collect_field_access_chain(
    field: &jolt_java_syntax::FieldAccessExpression,
) -> FormatResult<(Doc, Vec<Doc>)> {
    if !field.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this field access expression shape yet",
            field.text_range(),
        ));
    }
    let receiver = field.receiver().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field access expression without a receiver",
            field.text_range(),
        )
    })?;
    if !is_supported_selector_receiver(&receiver) {
        return Err(missing_layout(
            "Java formatter does not support this field access receiver yet",
            receiver.text_range(),
        ));
    }
    let name = field.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field access expression without a name",
            field.text_range(),
        )
    })?;

    let (base, mut selectors) = collect_selector_chain(&receiver)?;
    selectors.push(text(name.text()));
    Ok((base, selectors))
}

fn collect_method_invocation_chain(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
) -> FormatResult<(Doc, Vec<Doc>)> {
    if !invocation.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method invocation shape yet",
            invocation.text_range(),
        ));
    }

    let arguments = invocation.arguments().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method invocation without arguments",
            invocation.text_range(),
        )
    })?;
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method invocation argument shape yet",
            arguments.text_range(),
        ));
    }
    let arguments = format_argument_list(&arguments)?;

    if let Some(receiver) = invocation.receiver() {
        if !is_supported_selector_receiver(&receiver) {
            return Err(missing_layout(
                "Java formatter does not support this method invocation receiver yet",
                receiver.text_range(),
            ));
        }
        let name = invocation.name().ok_or_else(|| {
            missing_layout(
                "Java formatter found a qualified method invocation without a name",
                invocation.text_range(),
            )
        })?;
        let (base, mut selectors) = collect_selector_chain(&receiver)?;
        selectors.push(concat([text(name.text()), arguments]));
        return Ok((base, selectors));
    }

    let name = invocation.simple_name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method invocation without a simple name",
            invocation.text_range(),
        )
    })?;
    Ok((concat([text(name.text()), arguments]), Vec::new()))
}

fn format_literal_expression(literal: &jolt_java_syntax::LiteralExpression) -> FormatResult<Doc> {
    let token = literal.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this literal expression shape yet",
            literal.text_range(),
        )
    })?;
    if token.text().contains(is_line_terminator) {
        return Err(missing_layout(
            "Java formatter does not support multiline literals yet",
            token.text_range(),
        ));
    }
    Ok(format_token(&token))
}

fn format_name_expression(name: &jolt_java_syntax::NameExpression) -> FormatResult<Doc> {
    let identifier = name.identifier().ok_or_else(|| {
        missing_layout(
            "Java formatter only supports simple name expressions yet",
            name.text_range(),
        )
    })?;
    Ok(format_token(&identifier))
}

fn format_this_expression(this: &jolt_java_syntax::ThisExpression) -> FormatResult<Doc> {
    let token = this.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this expression shape yet",
            this.text_range(),
        )
    })?;
    Ok(format_token(&token))
}

fn format_super_expression(
    super_expression: &jolt_java_syntax::SuperExpression,
) -> FormatResult<Doc> {
    let token = super_expression.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support super expression shape yet",
            super_expression.text_range(),
        )
    })?;
    Ok(format_token(&token))
}

fn format_parenthesized_expression(
    parenthesized: &jolt_java_syntax::ParenthesizedExpression,
) -> FormatResult<Doc> {
    if !parenthesized.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this parenthesized expression shape yet",
            parenthesized.text_range(),
        ));
    }
    let expression = parenthesized.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a parenthesized expression without an expression",
            parenthesized.text_range(),
        )
    })?;
    Ok(wrap::parenthesized_expression(format_expression(
        &expression,
    )?))
}

fn format_unary_expression(unary: &jolt_java_syntax::UnaryExpression) -> FormatResult<Doc> {
    if !unary.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this unary expression shape yet",
            unary.text_range(),
        ));
    }
    let operator = unary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a unary expression without an operator",
            unary.text_range(),
        )
    })?;
    let operand = unary.operand().ok_or_else(|| {
        missing_layout(
            "Java formatter found a unary expression without an operand",
            unary.text_range(),
        )
    })?;
    if matches!(
        operand,
        Expression::AssignmentExpression(_) | Expression::BinaryExpression(_)
    ) {
        return Err(missing_layout(
            "Java formatter does not support this unary operand without parentheses",
            operand.text_range(),
        ));
    }
    if matches!(
        operator.kind(),
        JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
    ) && !is_supported_assignment_left(&operand)
    {
        return Err(missing_layout(
            "Java formatter does not support this update operand yet",
            operand.text_range(),
        ));
    }
    let separator = if unary_operator_needs_separator(&operator, &operand) {
        text(" ")
    } else {
        text("")
    };
    Ok(concat([
        format_token(&operator),
        separator,
        format_expression(&operand)?,
    ]))
}

fn format_postfix_expression(postfix: &jolt_java_syntax::PostfixExpression) -> FormatResult<Doc> {
    if !postfix.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this postfix expression shape yet",
            postfix.text_range(),
        ));
    }
    let operand = postfix.operand().ok_or_else(|| {
        missing_layout(
            "Java formatter found a postfix expression without an operand",
            postfix.text_range(),
        )
    })?;
    if matches!(
        operand,
        Expression::AssignmentExpression(_) | Expression::BinaryExpression(_)
    ) {
        return Err(missing_layout(
            "Java formatter does not support this postfix operand without parentheses",
            operand.text_range(),
        ));
    }
    if !is_supported_assignment_left(&operand) {
        return Err(missing_layout(
            "Java formatter does not support this postfix operand yet",
            operand.text_range(),
        ));
    }
    let operator = postfix.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a postfix expression without an operator",
            postfix.text_range(),
        )
    })?;
    Ok(concat([
        format_expression(&operand)?,
        format_token(&operator),
    ]))
}

fn format_assignment_expression(
    assignment: &jolt_java_syntax::AssignmentExpression,
) -> FormatResult<Doc> {
    if !assignment.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this assignment expression shape yet",
            assignment.text_range(),
        ));
    }
    let left = assignment.left().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without a left side",
            assignment.text_range(),
        )
    })?;
    if !is_supported_assignment_left(&left) {
        return Err(missing_layout(
            "Java formatter does not support this assignment left side yet",
            left.text_range(),
        ));
    }
    let operator = assignment.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without an operator",
            assignment.text_range(),
        )
    })?;
    let right = assignment.right().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without a right side",
            assignment.text_range(),
        )
    })?;
    Ok(wrap::assignment_expression(
        format_expression(&left)?,
        format_token(&operator),
        format_expression(&right)?,
    ))
}

fn format_argument_list(arguments: &jolt_java_syntax::ArgumentList) -> FormatResult<Doc> {
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this argument list shape yet",
            arguments.text_range(),
        ));
    }
    let arguments = arguments
        .arguments()
        .map(|argument| format_expression(&argument))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(wrap::parenthesized_comma_list(arguments))
}

fn format_binary_expression(binary: &jolt_java_syntax::BinaryExpression) -> FormatResult<Doc> {
    if !binary.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this binary expression shape yet",
            binary.text_range(),
        ));
    }
    let operator = binary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without an operator",
            binary.text_range(),
        )
    })?;
    let precedence = binary_precedence(operator.kind()).ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this binary operator yet",
            operator.text_range(),
        )
    })?;
    let left = binary.left().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a left side",
            binary.text_range(),
        )
    })?;
    let right = binary.right().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a right side",
            binary.text_range(),
        )
    })?;

    let mut first = None;
    let mut rest = Vec::new();
    collect_binary_left_chain(&left, precedence, &mut first, &mut rest)?;
    rest.push((
        format_token(&operator),
        format_binary_operand(&right, precedence, BinarySide::Right)?,
    ));

    let first = first.ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a left chain",
            binary.text_range(),
        )
    })?;
    Ok(wrap::binary_chain(first, rest))
}

#[derive(Clone, Copy)]
enum BinarySide {
    Left,
    Right,
}

fn collect_binary_left_chain(
    expression: &Expression,
    parent_precedence: u8,
    first: &mut Option<Doc>,
    rest: &mut Vec<(Doc, Doc)>,
) -> FormatResult<()> {
    if let Expression::BinaryExpression(binary) = expression
        && binary.has_supported_layout_shape()
    {
        let operator = binary.operator().ok_or_else(|| {
            missing_layout(
                "Java formatter found a binary expression without an operator",
                binary.text_range(),
            )
        })?;
        let child_precedence = binary_precedence(operator.kind()).ok_or_else(|| {
            missing_layout(
                "Java formatter does not support this binary operator yet",
                operator.text_range(),
            )
        })?;
        if child_precedence == parent_precedence {
            let left = binary.left().ok_or_else(|| {
                missing_layout(
                    "Java formatter found a binary expression without a left side",
                    binary.text_range(),
                )
            })?;
            let right = binary.right().ok_or_else(|| {
                missing_layout(
                    "Java formatter found a binary expression without a right side",
                    binary.text_range(),
                )
            })?;

            collect_binary_left_chain(&left, parent_precedence, first, rest)?;
            rest.push((
                format_token(&operator),
                format_binary_operand(&right, parent_precedence, BinarySide::Right)?,
            ));
            return Ok(());
        }
    }

    *first = Some(format_binary_operand(
        expression,
        parent_precedence,
        BinarySide::Left,
    )?);
    Ok(())
}

fn format_binary_operand(
    operand: &Expression,
    parent_precedence: u8,
    side: BinarySide,
) -> FormatResult<Doc> {
    let doc = format_expression(operand)?;
    let Expression::BinaryExpression(binary) = operand else {
        return Ok(doc);
    };
    let operator = binary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without an operator",
            binary.text_range(),
        )
    })?;
    let child_precedence = binary_precedence(operator.kind()).ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this binary operator yet",
            operator.text_range(),
        )
    })?;
    let needs_parentheses = child_precedence < parent_precedence
        || (child_precedence == parent_precedence && matches!(side, BinarySide::Right));
    if needs_parentheses {
        Ok(concat([text("("), doc, text(")")]))
    } else {
        Ok(doc)
    }
}

fn binary_precedence(kind: JavaSyntaxKind) -> Option<u8> {
    match kind {
        JavaSyntaxKind::OrOr => Some(3),
        JavaSyntaxKind::AndAnd => Some(4),
        JavaSyntaxKind::Bar => Some(5),
        JavaSyntaxKind::Caret => Some(6),
        JavaSyntaxKind::Amp => Some(7),
        JavaSyntaxKind::EqEq | JavaSyntaxKind::BangEq => Some(8),
        JavaSyntaxKind::Lt | JavaSyntaxKind::Gt | JavaSyntaxKind::LtEq | JavaSyntaxKind::GtEq => {
            Some(9)
        }
        JavaSyntaxKind::LShift | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => {
            Some(10)
        }
        JavaSyntaxKind::Plus | JavaSyntaxKind::Minus => Some(11),
        JavaSyntaxKind::Star | JavaSyntaxKind::Slash | JavaSyntaxKind::Percent => Some(12),
        _ => None,
    }
}

fn is_supported_selector_receiver(expression: &Expression) -> bool {
    match expression {
        Expression::NameExpression(_)
        | Expression::ThisExpression(_)
        | Expression::SuperExpression(_)
        | Expression::FieldAccessExpression(_)
        | Expression::MethodInvocationExpression(_) => true,
        Expression::ParenthesizedExpression(parenthesized) => parenthesized
            .expression()
            .is_some_and(|inner| is_supported_selector_receiver(&inner)),
        _ => false,
    }
}

fn is_supported_assignment_left(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_) | Expression::FieldAccessExpression(_)
    )
}

fn unary_operator_needs_separator(operator: &JavaSyntaxToken, operand: &Expression) -> bool {
    let Expression::UnaryExpression(operand) = operand else {
        return false;
    };
    let Some(operand_operator) = operand.operator() else {
        return false;
    };
    matches!(
        (operator.kind(), operand_operator.kind()),
        (
            JavaSyntaxKind::Plus,
            JavaSyntaxKind::Plus | JavaSyntaxKind::PlusPlus
        ) | (
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Minus | JavaSyntaxKind::MinusMinus
        )
    )
}

const fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn format_modifier_list(
    modifiers: Option<ModifierList>,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<ModifierDocs> {
    let Some(modifiers) = modifiers else {
        return Ok(ModifierDocs::default());
    };

    let annotations = format_annotation_list(modifiers.annotations(), context, "declaration")?;
    let tokens = modifiers.tokens().collect::<Vec<_>>();
    let keyword_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
    if tokens.len() != keyword_tokens.len() {
        return Err(missing_layout(
            format!("Java formatter does not support contextual {declaration_kind} modifiers yet"),
            modifiers.text_range(),
        ));
    }
    if !annotations.is_empty()
        && let Some(first_modifier) = keyword_tokens.first()
    {
        reject_unhandled_comments_before_start(
            context,
            first_modifier.token_text_range(),
            "Java formatter does not support comments between declaration annotations and modifiers yet",
        )?;
    }

    Ok(ModifierDocs {
        annotations,
        modifier_tokens: keyword_tokens,
    })
}

#[derive(Default)]
struct ModifierDocs {
    annotations: Vec<Doc>,
    modifier_tokens: Vec<JavaSyntaxToken>,
}

impl ModifierDocs {
    fn has_annotations(&self) -> bool {
        !self.annotations.is_empty()
    }

    fn with_annotations(self, declaration: Doc) -> Doc {
        with_vertical_annotations(self.annotations, declaration)
    }
}

fn with_vertical_annotations(annotations: Vec<Doc>, declaration: Doc) -> Doc {
    if annotations.is_empty() {
        return declaration;
    }

    concat([join(hard_line(), annotations), hard_line(), declaration])
}

fn format_annotation_list(
    annotations: impl Iterator<Item = Annotation>,
    context: &mut JavaFormatContext<'_>,
    annotation_kind: &'static str,
) -> FormatResult<Vec<Doc>> {
    annotations
        .map(|annotation| format_annotation(&annotation, context, annotation_kind))
        .collect()
}

fn format_annotation(
    annotation: &Annotation,
    context: &mut JavaFormatContext<'_>,
    annotation_kind: &'static str,
) -> FormatResult<Doc> {
    let messages = annotation_messages(annotation_kind);
    let code_range = annotation
        .code_text_range()
        .ok_or_else(|| missing_layout(messages.empty, annotation.text_range()))?;
    reject_unhandled_comments_before_start(context, code_range, messages.between)?;
    reject_unhandled_comments_before_end(context, code_range, messages.inside)?;
    if !annotation.has_supported_layout_shape() {
        return Err(missing_layout(messages.shape, annotation.text_range()));
    }

    let name = annotation
        .name()
        .ok_or_else(|| missing_layout(messages.missing_name, annotation.text_range()))?;
    let Some(arguments) = annotation.arguments() else {
        return Ok(concat([text("@"), format_name(&name)]));
    };

    Ok(concat([
        text("@"),
        format_name(&name),
        format_annotation_argument_list(&arguments)?,
    ]))
}

struct AnnotationMessages {
    empty: &'static str,
    between: &'static str,
    inside: &'static str,
    shape: &'static str,
    missing_name: &'static str,
}

fn annotation_messages(annotation_kind: &'static str) -> AnnotationMessages {
    match annotation_kind {
        "type-use" => AnnotationMessages {
            empty: "Java formatter found an empty type-use annotation",
            between: "Java formatter does not support comments between type-use annotations yet",
            inside: "Java formatter does not support comments inside type-use annotations yet",
            shape: "Java formatter does not support this type-use annotation shape yet",
            missing_name: "Java formatter found a type-use annotation without a name",
        },
        "declaration" => AnnotationMessages {
            empty: "Java formatter found an empty declaration annotation",
            between: "Java formatter does not support comments between declaration annotations yet",
            inside: "Java formatter does not support comments inside declaration annotations yet",
            shape: "Java formatter does not support this declaration annotation shape yet",
            missing_name: "Java formatter found a declaration annotation without a name",
        },
        _ => unreachable!("unknown annotation kind"),
    }
}

fn format_annotation_argument_list(arguments: &AnnotationArgumentList) -> FormatResult<Doc> {
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this annotation argument list shape yet",
            arguments.text_range(),
        ));
    }
    let Some(elements) = arguments.elements() else {
        return Ok(wrap::parenthesized_comma_list(std::iter::empty()));
    };

    if elements.has_pair_list_layout_shape() {
        return Ok(wrap::parenthesized_comma_list(
            elements
                .pairs()
                .map(|pair| format_annotation_element_value_pair(&pair))
                .collect::<FormatResult<Vec<_>>>()?,
        ));
    }

    if elements.has_value_list_layout_shape() {
        let values = elements.values().collect::<Vec<_>>();
        if values.len() != 1 {
            return Err(missing_layout(
                "Java formatter only supports single-member annotation values yet",
                elements.text_range(),
            ));
        }
        return Ok(wrap::parenthesized_comma_list([
            format_annotation_element_value(&values[0])?,
        ]));
    }

    Err(missing_layout(
        "Java formatter does not support mixed annotation argument lists yet",
        elements.text_range(),
    ))
}

fn format_annotation_element_value_pair(pair: &AnnotationElementValuePair) -> FormatResult<Doc> {
    if !pair.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this annotation element pair shape yet",
            pair.text_range(),
        ));
    }
    let name = pair.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element pair without a name",
            pair.text_range(),
        )
    })?;
    let value = pair.value().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element pair without a value",
            pair.text_range(),
        )
    })?;

    Ok(wrap::assignment_expression(
        format_token(&name),
        text("="),
        format_annotation_element_value(&value)?,
    ))
}

fn format_annotation_element_value(value: &AnnotationElementValue) -> FormatResult<Doc> {
    if !value.has_expression_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports expression annotation values yet",
            value.text_range(),
        ));
    }
    let expression = value.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element value without an expression",
            value.text_range(),
        )
    })?;

    format_expression(&expression)
}

fn format_type(ty: &Type, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    let parts = ty.simple_layout_parts().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this type shape yet",
            ty.text_range(),
        )
    })?;

    let mut docs = Vec::new();
    let mut previous_was_annotation = false;
    let mut previous_was_dot = false;
    for part in parts {
        match part {
            TypeLayoutPart::Annotation(annotation) => {
                if !docs.is_empty() && !previous_was_dot {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(&annotation, context, "type-use")?);
                previous_was_annotation = true;
                previous_was_dot = false;
            }
            TypeLayoutPart::Token(token) => {
                if previous_was_annotation && token.kind() == JavaSyntaxKind::Identifier {
                    reject_unhandled_comments_before_start(
                        context,
                        token.token_text_range(),
                        "Java formatter does not support comments between type-use annotations and types yet",
                    )?;
                    docs.push(text(" "));
                }
                previous_was_dot = token.kind() == JavaSyntaxKind::Dot;
                previous_was_annotation = false;
                docs.push(format_token(&token));
            }
        }
    }

    Ok(concat(docs))
}

fn format_token(token: &JavaSyntaxToken) -> Doc {
    text(token.text())
}

#[cfg(test)]
fn assert_formatted(source: &str, expected: &str) {
    assert_formatted_with_width(source, expected, 100);
}

#[cfg(test)]
fn assert_formatted_with_width(source: &str, expected: &str, line_width: u32) {
    let result = format_java_source_with_options(
        source,
        JavaFormatOptions {
            render: RenderOptions {
                line_width: jolt_fmt_ir::TextWidth::new(line_width),
                ..RenderOptions::default()
            },
        },
    );
    let expected = expected.to_owned() + "\n";

    assert_eq!(
        result.status,
        JavaFormatStatus::Formatted,
        "{source}\n{result:#?}"
    );
    assert_eq!(
        result.formatted_source.as_deref(),
        Some(expected.as_str()),
        "{source}"
    );
    assert!(result.diagnostics.is_empty(), "{source}");
}

#[cfg(test)]
fn assert_blocked_missing_layout(source: &str) {
    let result = format_java_source(source);

    assert_eq!(result.status, JavaFormatStatus::Blocked, "{source}");
    assert_eq!(result.formatted_source, None, "{source}");
    assert_eq!(result.diagnostics.len(), 1, "{source}");
    assert_eq!(
        result.diagnostics[0].code.as_str(),
        JavaFormatDiagnosticCode::MissingLayoutRules.id().as_str(),
        "{source}"
    );
    assert_eq!(
        result.diagnostics[0].stage,
        DiagnosticStage::Formatter,
        "{source}"
    );
    assert_eq!(result.diagnostics[0].severity, Severity::Error, "{source}");
    assert!(
        result.diagnostics[0].range.is_some(),
        "diagnostic should carry a source range for {source}"
    );
}

#[cfg(test)]
fn assert_blocked_parser(source: &str) {
    let result = format_java_source(source);

    assert_eq!(result.status, JavaFormatStatus::Blocked);
    assert_eq!(result.formatted_source, None);
    assert!(!result.diagnostics.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == DiagnosticStage::Parser)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_preserve_source_order() {
        assert_formatted(
            "import z.Z; import a.A; import java.util.*; import module java.base; import module.foo.Bar; class A {}",
            "import z.Z;\nimport a.A;\nimport java.util.*;\nimport module java.base;\nimport module.foo.Bar;\n\nclass A {}",
        );
    }

    #[test]
    fn class_body_empty_declarations_format() {
        assert_formatted(
            "class A { ; int value; ; // trailing\n; }",
            "class A {\n  ;\n  int value;\n  ; // trailing\n  ;\n}",
        );
    }

    #[test]
    fn method_and_constructor_signatures_format_structurally() {
        assert_formatted(
            "abstract class A { public <T, U> T pick(final T first, U second) throws Problem, java.io.IOException { return first; } private A(int count, String... names) throws Problem {} abstract void reset(int count) throws Problem; }",
            "abstract class A {\n  public <T, U> T pick(final T first, U second) throws Problem, java.io.IOException {\n    return first;\n  }\n  private A(int count, String... names) throws Problem {}\n  abstract void reset(int count) throws Problem;\n}",
        );
    }

    #[test]
    fn class_headers_and_nested_classes_format_structurally() {
        assert_formatted(
            "class A<T, U> extends base.Parent implements First, second.Third permits One, two.Three { private static class Nested extends Parent implements Marker {} }",
            "class A<T, U> extends base.Parent implements First, second.Third permits One, two.Three {\n  private static class Nested extends Parent implements Marker {}\n}",
        );
    }

    #[test]
    fn declaration_marker_annotations_format_vertically() {
        assert_formatted(
            "@Pkg package com.example; @Type public class A { @Field private String value; @Method public String name() { return value; } @Ctor A() {} @Nested static class Nested {} }",
            "@Pkg\npackage com.example;\n\n@Type\npublic class A {\n  @Field\n  private String value;\n  @Method\n  public String name() {\n    return value;\n  }\n  @Ctor\n  A() {}\n  @Nested\n  static class Nested {}\n}",
        );
    }

    #[test]
    fn declaration_annotation_arguments_format_structurally() {
        assert_formatted(
            "@Single(\"type\") @Normal(first = 1, second=value + 2) class A { @SuppressWarnings(\"unchecked\") String value; }",
            "@Single(\"type\")\n@Normal(first = 1, second = value + 2)\nclass A {\n  @SuppressWarnings(\"unchecked\")\n  String value;\n}",
        );
    }

    #[test]
    fn type_use_annotations_in_simple_types_format_structurally() {
        assert_formatted(
            "class A { java.lang.@Anno String value; void m() { java.lang.@Anno String local; } }",
            "class A {\n  java.lang.@Anno String value;\n  void m() {\n    java.lang.@Anno String local;\n  }\n}",
        );
    }

    #[test]
    fn non_empty_method_and_constructor_blocks_format_in_source_order() {
        assert_formatted(
            "class A { A() { int local; { return; } } int one() { return 1; } Object self() { return this; } Object parent() { return super; } void done() { return; } }",
            "class A {\n  A() {\n    int local;\n    {\n      return;\n    }\n  }\n  int one() {\n    return 1;\n  }\n  Object self() {\n    return this;\n  }\n  Object parent() {\n    return super;\n  }\n  void done() {\n    return;\n  }\n}",
        );
    }

    #[test]
    fn local_variable_types_and_throw_statements_format_structurally() {
        assert_formatted(
            "class A { void fail() { java.lang.Exception ex; var var = ex; final var copy = var; throw ex; } }",
            "class A {\n  void fail() {\n    java.lang.Exception ex;\n    var var = ex;\n    final var copy = var;\n    throw ex;\n  }\n}",
        );
    }

    #[test]
    fn field_and_local_initializers_format_supported_expressions() {
        assert_formatted(
            "class A { int value = 1; Object output = System.out; int total = a + b * c; int grouped = (a + b) * -c; int negative = - -1; int positive = + +1; int first, second = 2; void m() { int local = (value + 1), other; } int sum() { return a + b * c; } }",
            "class A {\n  int value = 1;\n  Object output = System.out;\n  int total = a + b * c;\n  int grouped = (a + b) * -c;\n  int negative = - -1;\n  int positive = + +1;\n  int first, second = 2;\n  void m() {\n    int local = (value + 1), other;\n  }\n  int sum() {\n    return a + b * c;\n  }\n}",
        );
    }

    #[test]
    fn initializer_blocks_format_as_class_body_members() {
        assert_formatted(
            "class A { static { int ready; } { call(); } }",
            "class A {\n  static {\n    int ready;\n  }\n  {\n    call();\n  }\n}",
        );
    }

    #[test]
    fn expression_statements_format_supported_calls_assignments_and_updates() {
        assert_formatted(
            "class A { void m() { call(); target.call(1, this.value); System.out.println((value)); builder.first().second(value); this.value = value + 1; value += -delta; value++; ++value; } }",
            "class A {\n  void m() {\n    call();\n    target.call(1, this.value);\n    System.out.println((value));\n    builder.first().second(value);\n    this.value = value + 1;\n    value += -delta;\n    value++;\n    ++value;\n  }\n}",
        );
    }

    #[test]
    fn narrow_width_wraps_existing_argument_lists() {
        assert_formatted_with_width(
            "class A { void m() { call(alpha, beta, gamma); } }",
            "class A {\n  void m() {\n    call(\n        alpha, beta,\n        gamma);\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_method_signature_parameters() {
        assert_formatted_with_width(
            "class A { void combine(int alpha, int beta, int gamma) throws FirstProblem, SecondProblem {} }",
            "class A {\n  void\n  combine(\n      int alpha,\n      int beta,\n      int gamma)\n  throws FirstProblem,\n  SecondProblem {}\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_variable_declarations() {
        assert_formatted_with_width(
            "class A { int total = alpha + beta + gamma; void m() { final int local = alpha + beta + gamma; } }",
            "class A {\n  int total =\n      alpha\n          + beta\n          + gamma;\n  void m() {\n    final int local =\n        alpha\n            + beta\n            + gamma;\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_assignments_and_binary_expressions() {
        assert_formatted_with_width(
            "class A { void m() { target.value = alpha + beta + gamma; } }",
            "class A {\n  void m() {\n    target.value =\n        alpha\n            + beta\n            + gamma;\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_selector_chains() {
        assert_formatted_with_width(
            "class A { void m() { builder.first().second(value).third(); } }",
            "class A {\n  void m() {\n    builder.first()\n        .second(\n            value)\n        .third();\n  }\n}",
            20,
        );
    }

    #[test]
    fn invalid_java_blocks_and_forwards_parser_diagnostics() {
        assert_blocked_parser("class A {");
    }

    #[test]
    fn leading_comments_before_compilation_unit_declarations_format() {
        assert_formatted(
            "// package\npackage com.example;\n// import\nimport java.util.List;\n// type\nclass A {}",
            "// package\npackage com.example;\n\n// import\nimport java.util.List;\n\n// type\nclass A {}",
        );
    }

    #[test]
    fn leading_comments_before_members_and_block_statements_format() {
        assert_formatted(
            "class A {\n// field\nint value;\n/** method */\nvoid clear() {\n// local\nint local = 1;\n// call\ncall();\n{\n// nested\nreturn;\n}\n}\n}",
            "class A {\n  // field\n  int value;\n  /** method */\n  void clear() {\n    // local\n    int local = 1;\n    // call\n    call();\n    {\n      // nested\n      return;\n    }\n  }\n}",
        );
    }

    #[test]
    fn leading_javadocs_before_class_and_method_format() {
        assert_formatted(
            "/** class docs */\nclass A {\n/** method docs */\nvoid clear() {} }",
            "/** class docs */\nclass A {\n  /** method docs */\n  void clear() {}\n}",
        );
    }

    #[test]
    fn multiline_leading_block_comments_and_javadocs_format() {
        assert_formatted(
            "/*\n * class docs\n */\nclass A {\n/**\n * field docs\n */\nint value;\nvoid clear() {\n/*\n * local docs\n */\nreturn;\n}\n}",
            "/*\n * class docs\n */\nclass A {\n  /**\n   * field docs\n   */\n  int value;\n  void clear() {\n    /*\n     * local docs\n     */\n    return;\n  }\n}",
        );
    }

    #[test]
    fn already_indented_multiline_javadocs_format_idempotently() {
        assert_formatted(
            "class A {\n  /**\n   * field docs\n   */\n  int value;\n}",
            "class A {\n  /**\n   * field docs\n   */\n  int value;\n}",
        );
    }

    #[test]
    fn dangling_comments_inside_empty_class_bodies_format() {
        assert_formatted(
            "class A {\n/*\n * block\n */\n/** docs */\n// line\n}",
            "class A {\n  /*\n   * block\n   */\n  /** docs */\n  // line\n}",
        );
    }

    #[test]
    fn dangling_comments_inside_empty_blocks_format() {
        assert_formatted(
            "class A { void clear() {\n// line\n} A() {\n/**\n * constructor\n */\n} }",
            "class A {\n  void clear() {\n    // line\n  }\n  A() {\n    /**\n     * constructor\n     */\n  }\n}",
        );
    }

    #[test]
    fn trailing_line_comments_after_declarations_and_statements_format() {
        assert_formatted(
            "class A { int value = 1; // field\nint one() { call(); // call\nreturn 1; // answer\n} }",
            "class A {\n  int value = 1; // field\n  int one() {\n    call(); // call\n    return 1; // answer\n  }\n}",
        );
    }

    #[test]
    fn ambiguous_or_unsupported_comments_still_block() {
        for source in [
            "class A { // dangling\n}",
            "class A { void clear() { // dangling\n} }",
            "class A // header\n{}",
            "class A { int /* inline */ value; }",
            "class A { void /* inline */ clear() {} }",
            "class A { void clear(\n// parameter\nint value) {} }",
            "class A { abstract void clear(\n// parameter\nint value); }",
            "class A { void clear() throws\n// throws\nException {} }",
            "class A { void clear() { if (ready)\n// branch\nreturn; call(); } }",
            "class A { void clear() { if (ready) { return; }\n// else\nelse return; } }",
            "class A { /* body */ }",
            "class A { void clear() { /* body */ } }",
            "class A {}\u{001A}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_annotation_forms_block() {
        for source in [
            "class A { @Anno(\n// value\n1) int value; }",
            "@Anno /* between */ class A {}",
            "@First /* between */ @Second class A {}",
            "@ /* inside */ Anno class A {}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_declaration_forms_block() {
        for source in [
            "class A<T extends B> {}",
            "class A extends B, C {}",
            "class A extends java.util.List<String> {}",
            "sealed class A {}",
            "non-sealed class A {}",
            "import java.util.List garbage; class A {}",
            "void main() {}",
            "import java.util.List; void main() {}",
            "; class A {}",
            "record A() {}",
            "enum A {}",
            "interface A {}",
            "@interface A {}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_member_forms_block() {
        for source in [
            "class A { int value[]; }",
            "class A { String[] names() {} }",
            "class A { java.util.List<String> names; }",
            "class A { <T extends B> void clear() {} }",
            "class A { void clear(@Deprecated int count) {} }",
            "class A { void clear(A this) {} }",
            "void main() {}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_statement_forms_block() {
        for source in [
            "class A { void m() { while (ready) return; } }",
            "class A { void m() { for (;;) return; } }",
            "class A { void m() { try { return; } catch (Exception ex) { return; } } }",
            "class A { int m() { switch (value) { default: return 0; } } }",
            "class A { void m() { assert ready; } }",
            "class A { void m() { label: return; } }",
            "class A { void m() { class Local {} } }",
            "class A { A() { this(); } }",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn simple_statement_forms_format() {
        assert_formatted(
            "class A { void m() { ; if (ready) { return; } else if (other) break label; else continue; } }",
            "class A {\n  void m() {\n    ;\n    if (ready) {\n      return;\n    } else if (other)\n      break label;\n    else\n      continue;\n  }\n}",
        );
    }

    #[test]
    fn unsupported_statement_expression_shapes_block() {
        for source in [
            "class A { void m() { int local[]; } }",
            "class A { void m() { int local = ready ? 1 : 2; } }",
            "class A { void m() { int local = (int) value; } }",
            "class A { void m() { Object local = new Object(); } }",
            "class A { void m() { Object local = String.class; } }",
            "class A { void m() { Object local = this::call; } }",
            "class A { void m() { boolean local = value instanceof String; } }",
            "class A { void m() { Runnable local = () -> call(); } }",
            "class A { void m() { String local = \"\"\"\ntext\n\"\"\"; } }",
            "class A { void m() { int[] local = {1}; } }",
            "class A { void m() { call(new Object()); } }",
            "class A { void m() { this.<String>call(); } }",
            "class A { void m() { target.<String>call(); } }",
            "class A { void m() { values[0] = 1; } }",
        ] {
            assert_blocked_missing_layout(source);
        }
    }
}
