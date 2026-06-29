//! Java formatter implementation for Jolt.

mod context;
mod wrapping;

use crate::wrapping as wrap;
use context::{JavaCommentTrivia, JavaFormatContext};
use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
    TextRange,
};
use jolt_fmt_ir::{
    Doc, RenderOptions, concat, hard_line, join, line_suffix, line_suffix_boundary, render, text,
};
use jolt_java_syntax::{
    Block, BlockItem, BlockStatement, ClassBody, ClassBodyMember, ClassDeclaration,
    CompilationUnit, ConstructorBody, ConstructorDeclaration, Expression, FieldDeclaration,
    ImportDeclaration, JavaSyntaxKind, JavaSyntaxToken, LocalVariableDeclaration,
    MethodDeclaration, ModifierList, NameSyntax, PackageDeclaration, ReturnStatement,
    ThrowStatement, Type, TypeDeclaration, VariableDeclarator, YieldStatement,
    parse_compilation_unit,
};

/// Formatter operation status for Java formatting.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatStatus {
    /// Java source was formatted.
    Formatted,
    /// Java formatting was blocked and no formatted source was produced.
    Blocked,
}

/// Java formatter output plus diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaFormatResult {
    /// Formatted source text, absent when formatting was blocked.
    pub formatted_source: Option<String>,
    /// Diagnostics produced while formatting.
    pub diagnostics: Vec<Diagnostic>,
    /// Formatter operation status.
    pub status: JavaFormatStatus,
}

/// Java formatter options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaFormatOptions {
    /// Language-neutral rendering options used by the Java formatter.
    pub render: RenderOptions,
}

impl Default for JavaFormatOptions {
    fn default() -> Self {
        Self::for_profile(JavaFormatProfile::Google)
    }
}

impl JavaFormatOptions {
    /// Returns concrete Java formatter options for a compatibility profile.
    #[must_use]
    pub fn for_profile(profile: JavaFormatProfile) -> Self {
        let render = match profile {
            JavaFormatProfile::Google | JavaFormatProfile::Palantir => RenderOptions::default(),
            JavaFormatProfile::Aosp => RenderOptions {
                indent_width: 4,
                ..RenderOptions::default()
            },
        };

        Self { render }
    }
}

/// Java formatter compatibility profile convenience selector.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum JavaFormatProfile {
    /// Compatibility target for Google Java Format.
    #[default]
    Google,
    /// Compatibility target for Google Java Format AOSP mode.
    Aosp,
    /// Compatibility target for Palantir Java Format.
    Palantir,
}

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatDiagnosticCode {
    /// Parsed Java syntax is not covered by the current layout skeleton.
    MissingLayoutRules,
    /// Rendering the formatter document failed.
    RenderFailed,
}

impl DiagnosticCode for JavaFormatDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::MissingLayoutRules => DiagnosticCodeId::new("java.format.missing_layout_rules"),
            Self::RenderFailed => DiagnosticCodeId::new("java.format.render_failed"),
        }
    }
}

/// Formats Java source text.
#[must_use]
pub fn format_java_source(source: &str) -> JavaFormatResult {
    format_java_source_with_options(source, JavaFormatOptions::default())
}

/// Formats Java source text with options resolved from a compatibility profile.
#[must_use]
pub fn format_java_source_with_profile(
    source: &str,
    profile: JavaFormatProfile,
) -> JavaFormatResult {
    format_java_source_with_options(source, JavaFormatOptions::for_profile(profile))
}

/// Formats Java source text with explicit Java formatter options.
#[must_use]
pub fn format_java_source_with_options(
    source: &str,
    options: JavaFormatOptions,
) -> JavaFormatResult {
    let parse = parse_compilation_unit(source);
    let (syntax, diagnostics, outcome) = parse.into_parts();

    if outcome != SyntaxOutcome::Clean {
        return blocked(diagnostics);
    }

    let Some(syntax) = syntax else {
        return blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::MissingLayoutRules.id(),
            severity: Severity::InternalError,
            stage: DiagnosticStage::Formatter,
            message: "Java parser produced a clean outcome without syntax".to_owned(),
            range: None,
        }]);
    };

    let mut context = JavaFormatContext::new(source);
    let doc = match format_compilation_unit(&syntax, &mut context) {
        Ok(doc) => doc,
        Err(diagnostic) => return blocked(vec![diagnostic]),
    };

    if context.has_unhandled_comment_trivia() {
        let Some(trivia) = context.next_unhandled_comment_trivia() else {
            return blocked(vec![Diagnostic {
                code: JavaFormatDiagnosticCode::MissingLayoutRules.id(),
                severity: Severity::InternalError,
                stage: DiagnosticStage::Formatter,
                message: "Java formatter context reported unhandled trivia without a record"
                    .to_owned(),
                range: Some(syntax.text_range()),
            }]);
        };
        return blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::MissingLayoutRules.id(),
            severity: Severity::Error,
            stage: DiagnosticStage::Formatter,
            message: "Java formatter found unhandled comment or ignored trivia".to_owned(),
            range: Some(trivia.trivia.range),
        }]);
    }

    match render(&doc, options.render) {
        Ok(rendered) => {
            let mut formatted_source = rendered.text;
            if !formatted_source.ends_with('\n') {
                formatted_source.push('\n');
            }
            JavaFormatResult {
                formatted_source: Some(formatted_source),
                diagnostics,
                status: JavaFormatStatus::Formatted,
            }
        }
        Err(error) => blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::RenderFailed.id(),
            severity: Severity::InternalError,
            stage: DiagnosticStage::Formatter,
            message: format!("Java formatter failed to render document IR: {error}"),
            range: Some(syntax.text_range()),
        }]),
    }
}

fn blocked(diagnostics: Vec<Diagnostic>) -> JavaFormatResult {
    JavaFormatResult {
        formatted_source: None,
        diagnostics,
        status: JavaFormatStatus::Blocked,
    }
}

type FormatResult<T> = Result<T, Diagnostic>;

fn format_compilation_unit(
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
    if let Some(annotation) = package.annotations().next() {
        return Err(missing_layout(
            "Java formatter does not support package annotations yet",
            annotation.text_range(),
        ));
    }
    let code_range = package.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty package declaration",
            package.text_range(),
        )
    })?;
    let name = package.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a package declaration without a name",
            package.text_range(),
        )
    })?;
    with_attached_comments(
        context,
        code_range,
        concat([text("package "), format_name(&name), text(";")]),
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
    let modifiers = format_modifier_list(class.modifiers(), "class")?;

    if let Some(type_parameters) = class.type_parameters() {
        return Err(missing_layout(
            "Java formatter does not support class type parameters yet",
            type_parameters.text_range(),
        ));
    }
    if let Some(extends_clause) = class.extends_clause() {
        return Err(missing_layout(
            "Java formatter does not support extends clauses yet",
            extends_clause.text_range(),
        ));
    }
    if let Some(implements_clause) = class.implements_clause() {
        return Err(missing_layout(
            "Java formatter does not support implements clauses yet",
            implements_clause.text_range(),
        ));
    }
    if let Some(permits_clause) = class.permits_clause() {
        return Err(missing_layout(
            "Java formatter does not support permits clauses yet",
            permits_clause.text_range(),
        ));
    }
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
    let body = class.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a class declaration without a body",
            class.text_range(),
        )
    })?;
    let body_members = format_class_body(&body, context)?;

    let mut header = Vec::new();
    header.extend(modifiers.iter().map(format_token));
    header.push(text("class"));
    header.push(text(name.text()));
    let header = wrap::declaration_header(header);

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        concat([header, text(" "), wrap::braced_block(body_members)]),
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

    body.members()
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
        ClassBodyMember::FieldDeclaration(field) => format_field_declaration(field),
        ClassBodyMember::MethodDeclaration(method) => format_method_declaration(method, context),
        ClassBodyMember::ConstructorDeclaration(constructor) => {
            format_constructor_declaration(constructor, context)
        }
        ClassBodyMember::EmptyDeclaration(_) => Ok(text(";")),
        ClassBodyMember::ClassDeclaration(class) => Err(missing_layout(
            "Java formatter does not support nested class declarations yet",
            class.text_range(),
        )),
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

fn format_field_declaration(field: &FieldDeclaration) -> FormatResult<Doc> {
    if !field.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this field declaration shape yet",
            field.text_range(),
        ));
    }

    let modifiers = format_modifier_list(field.modifiers(), "field")?;
    let ty = field.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field declaration without a type",
            field.text_range(),
        )
    })?;
    let declarators = field.declarators().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field declaration without declarators",
            field.text_range(),
        )
    })?;
    let declarators = format_variable_declarator_list(&declarators, "field")?;

    let mut prefix = Vec::new();
    prefix.extend(modifiers.iter().map(format_token));
    prefix.push(format_type(&ty)?);
    Ok(wrap::variable_declaration(prefix, declarators))
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

    let modifiers = format_modifier_list(method.modifiers(), "method")?;
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
    let body = method.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method declaration without a body",
            method.text_range(),
        )
    })?;
    let mut header = Vec::new();
    header.extend(modifiers.iter().map(format_token));
    header.push(format_type(&return_type)?);
    header.push(concat([
        text(name.text()),
        wrap::parenthesized_comma_list(std::iter::empty()),
    ]));
    Ok(concat([
        wrap::declaration_header(header),
        text(" "),
        format_block(&body, context)?,
    ]))
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

    let modifiers = format_modifier_list(constructor.modifiers(), "constructor")?;
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
    let mut header = Vec::new();
    header.extend(modifiers.iter().map(format_token));
    header.push(concat([
        text(name.text()),
        wrap::parenthesized_comma_list(std::iter::empty()),
    ]));
    Ok(concat([
        wrap::declaration_header(header),
        text(" "),
        format_constructor_body(&body, context)?,
    ]))
}

fn format_block(block: &Block, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    if !block.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this block shape yet",
            block.text_range(),
        ));
    }
    format_block_statements(block.block_statements(), context)
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
    format_block_statements(body.block_statements(), context)
}

fn format_block_statements(
    statements: impl Iterator<Item = BlockStatement>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let statements = statements
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
    let doc = match item {
        BlockItem::LocalVariableDeclaration(declaration) => {
            format_local_variable_declaration(&declaration)
        }
        BlockItem::Block(block) => format_block(&block, context),
        BlockItem::ReturnStatement(return_statement) => format_return_statement(&return_statement),
        BlockItem::ThrowStatement(throw_statement) => format_throw_statement(&throw_statement),
        BlockItem::YieldStatement(yield_statement) => format_yield_statement(&yield_statement),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => Err(missing_layout(
            "Java formatter does not support local class or interface declarations yet",
            declaration.text_range(),
        )),
        BlockItem::EmptyStatement(empty) => Err(missing_layout(
            "Java formatter does not support empty statements yet",
            empty.text_range(),
        )),
        BlockItem::LabeledStatement(labeled) => Err(missing_layout(
            "Java formatter does not support labeled statements yet",
            labeled.text_range(),
        )),
        BlockItem::ExpressionStatement(expression) => format_expression_statement(&expression),
        BlockItem::IfStatement(if_statement) => Err(missing_layout(
            "Java formatter does not support if statements yet",
            if_statement.text_range(),
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
        BlockItem::BreakStatement(break_statement) => Err(missing_layout(
            "Java formatter does not support break statements yet",
            break_statement.text_range(),
        )),
        BlockItem::ContinueStatement(continue_statement) => Err(missing_layout(
            "Java formatter does not support continue statements yet",
            continue_statement.text_range(),
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
    }?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn format_local_variable_declaration(declaration: &LocalVariableDeclaration) -> FormatResult<Doc> {
    if !declaration.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this local variable declaration shape yet",
            declaration.text_range(),
        ));
    }

    let ty = if let Some(ty) = declaration.ty() {
        format_type(&ty)?
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
        Expression::FieldAccessExpression(field) => format_field_access_expression(field),
        Expression::MethodInvocationExpression(invocation) => format_method_invocation(invocation),
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

fn format_field_access_expression(
    field: &jolt_java_syntax::FieldAccessExpression,
) -> FormatResult<Doc> {
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
    Ok(wrap::dot_chain(
        format_expression(&receiver)?,
        [text(name.text())],
    ))
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

fn format_method_invocation(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
) -> FormatResult<Doc> {
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
        return Ok(wrap::dot_chain(
            format_expression(&receiver)?,
            [concat([
                text(name.text()),
                format_argument_list(&arguments)?,
            ])],
        ));
    }

    let name = invocation.simple_name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method invocation without a simple name",
            invocation.text_range(),
        )
    })?;

    Ok(concat([
        text(name.text()),
        format_argument_list(&arguments)?,
    ]))
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
) -> FormatResult<Vec<JavaSyntaxToken>> {
    let Some(modifiers) = modifiers else {
        return Ok(Vec::new());
    };
    if let Some(annotation) = modifiers.annotations().next() {
        return Err(missing_layout(
            "Java formatter does not support declaration annotations yet",
            annotation.text_range(),
        ));
    }

    let tokens = modifiers.tokens().collect::<Vec<_>>();
    let keyword_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
    if tokens.len() != keyword_tokens.len() {
        return Err(missing_layout(
            format!("Java formatter does not support contextual {declaration_kind} modifiers yet"),
            modifiers.text_range(),
        ));
    }

    Ok(keyword_tokens)
}

fn format_type(ty: &Type) -> FormatResult<Doc> {
    let tokens = ty.simple_layout_tokens().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this type shape yet",
            ty.text_range(),
        )
    })?;
    Ok(join(text("."), tokens.iter().map(format_token)))
}

fn format_token(token: &JavaSyntaxToken) -> Doc {
    text(token.text())
}

fn with_attached_comments(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    doc: Doc,
) -> FormatResult<Doc> {
    let leading = take_leading_comment_docs(context, code_range)?;

    with_leading_and_trailing_comments(context, code_range, leading, doc)
}

fn take_leading_comment_docs(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    context
        .take_leading_comments(code_range)
        .map_err(|error| missing_layout(error.message, error.range))
        .map(|comments| {
            comments
                .into_iter()
                .map(|comment| format_own_line_comment(context, &comment))
                .collect()
        })
}

fn with_leading_and_trailing_comments(
    context: &mut JavaFormatContext<'_>,
    code_range: TextRange,
    leading: Vec<Doc>,
    doc: Doc,
) -> FormatResult<Doc> {
    let trailing = context
        .take_trailing_line_comment(code_range)
        .map_err(|error| missing_layout(error.message, error.range))?;

    let doc = if let Some(comment) = trailing {
        concat([
            doc,
            line_suffix(text(format!(" {}", context.raw_text(&comment)))),
            line_suffix_boundary(),
        ])
    } else {
        doc
    };

    if leading.is_empty() {
        return Ok(doc);
    }

    Ok(concat([join(hard_line(), leading), hard_line(), doc]))
}

fn format_own_line_comment(context: &JavaFormatContext<'_>, comment: &JavaCommentTrivia) -> Doc {
    text(context.raw_text(comment))
}

fn missing_layout(message: impl Into<String>, range: TextRange) -> Diagnostic {
    Diagnostic {
        code: JavaFormatDiagnosticCode::MissingLayoutRules.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: message.into(),
        range: Some(range),
    }
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
    fn minimal_clean_java_formats_successfully() {
        assert_formatted("class A {}", "class A {}");
    }

    #[test]
    fn package_imports_and_class_format_as_compilation_unit_sections() {
        assert_formatted(
            "package   com.example ; import java.util.List; import static java.util.Collections.emptyList; public class A {}",
            "package com.example;\n\nimport java.util.List;\nimport static java.util.Collections.emptyList;\n\npublic class A {}",
        );
    }

    #[test]
    fn imports_preserve_source_order() {
        assert_formatted(
            "import z.Z; import a.A; import java.util.*; import module java.base; import module.foo.Bar; class A {}",
            "import z.Z;\nimport a.A;\nimport java.util.*;\nimport module java.base;\nimport module.foo.Bar;\n\nclass A {}",
        );
    }

    #[test]
    fn multiple_top_level_empty_classes_format_in_order() {
        assert_formatted("class A {} class B {}", "class A {}\n\nclass B {}");
    }

    #[test]
    fn simple_class_modifiers_format_in_source_order() {
        assert_formatted(
            "public final class A {} abstract class B {} strictfp class C {}",
            "public final class A {}\n\nabstract class B {}\n\nstrictfp class C {}",
        );
    }

    #[test]
    fn simple_class_body_members_format_in_source_order() {
        assert_formatted(
            "class A { int value; String name; A() {} void clear() {} int size() {} }",
            "class A {\n  int value;\n  String name;\n  A() {}\n  void clear() {}\n  int size() {}\n}",
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
    fn member_modifiers_format_with_members() {
        assert_formatted(
            "public class A { private final int value; public A() {} protected static void reset() {} }",
            "public class A {\n  private final int value;\n  public A() {}\n  protected static void reset() {}\n}",
        );
    }

    #[test]
    fn qualified_member_types_format_structurally() {
        assert_formatted(
            "class A { java.util.List value; java.lang.String name() {} }",
            "class A {\n  java.util.List value;\n  java.lang.String name() {}\n}",
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
            "class A {\n  void m() {\n    call(\n      alpha,\n      beta,\n      gamma\n    );\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_variable_declarations() {
        assert_formatted_with_width(
            "class A { int total = alpha + beta + gamma; void m() { final int local = alpha + beta + gamma; } }",
            "class A {\n  int total =\n    alpha\n      + beta\n      + gamma;\n  void m() {\n    final int local =\n      alpha\n        + beta\n        + gamma;\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_assignments_and_binary_expressions() {
        assert_formatted_with_width(
            "class A { void m() { target.value = alpha + beta + gamma; } }",
            "class A {\n  void m() {\n    target.value =\n      alpha\n        + beta\n        + gamma;\n  }\n}",
            20,
        );
    }

    #[test]
    fn narrow_width_wraps_existing_selector_chains() {
        assert_formatted_with_width(
            "class A { void m() { builder.first().second(value).third(); } }",
            "class A {\n  void m() {\n    builder.first()\n      .second(value)\n      .third();\n  }\n}",
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
            "class A { int /* inline */ value; }",
            "class A { void /* inline */ clear() {} }",
            "class A { /* body */ }",
            "class A { void clear() { /* body */ } }",
            "class A {}\u{001A}",
            "/*\n * multiline\n */\nclass A {}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_annotations_block() {
        for source in [
            "@Deprecated class A {}",
            "public @Deprecated class A {}",
            "@Deprecated package com.example; class A {}",
            "class A { @Deprecated int value; }",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_declaration_forms_block() {
        for source in [
            "class A<T> {}",
            "class A extends B {}",
            "class A implements B {}",
            "class A permits B {}",
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
            "class A { void clear() throws Exception {} }",
            "class A { <T> void clear() {} }",
            "class A { void clear(int count) {} }",
            "class A { String[] names() {} }",
            "class A { java.util.List<String> names; }",
            "class A { class Nested {} }",
            "void main() {}",
        ] {
            assert_blocked_missing_layout(source);
        }
    }

    #[test]
    fn unsupported_statement_forms_block() {
        for source in [
            "class A { void m() { ; } }",
            "class A { void m() { if (ready) return; } }",
            "class A { void m() { while (ready) return; } }",
            "class A { void m() { for (;;) return; } }",
            "class A { void m() { try { return; } catch (Exception ex) { return; } } }",
            "class A { int m() { switch (value) { default: return 0; } } }",
            "class A { void m() { break; } }",
            "class A { void m() { continue; } }",
            "class A { void m() { assert ready; } }",
            "class A { void m() { label: return; } }",
            "class A { void m() { class Local {} } }",
            "class A { A() { this(); } }",
        ] {
            assert_blocked_missing_layout(source);
        }
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
