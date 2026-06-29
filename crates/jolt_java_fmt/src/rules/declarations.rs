use super::{
    AnnotationElementDeclaration, AnnotationInterfaceBody, AnnotationInterfaceBodyMember,
    AnnotationInterfaceDeclaration, ClassBody, ClassBodyMember, ClassDeclaration,
    ConstructorDeclaration, DefaultValue, Doc, EnumBody, EnumConstant, EnumConstantList,
    EnumDeclaration, ExtendsClause, FieldDeclaration, FormalParameter, FormalParameterList,
    FormalParameterModifier, FormatResult, ImplementsClause, InterfaceBody, InterfaceBodyMember,
    InterfaceDeclaration, JavaFormatContext, JavaSyntaxToken, MethodDeclaration, ModifierList,
    PermitsClause, ThrowsClause, Type, TypeDeclaration, TypeParameterList, concat,
    format_annotation, format_annotation_element_value, format_annotation_list,
    format_argument_list, format_array_dimensions, format_block, format_constructor_body,
    format_modifier_list, format_name, format_token, format_type, format_variable_declarator_list,
    join, missing_layout, reject_unhandled_comments_before_end,
    reject_unhandled_comments_before_start, take_dangling_comment_docs, take_leading_comment_docs,
    text, with_leading_and_trailing_comments, wrap,
};

pub(super) fn format_type_declaration(
    declaration: &TypeDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class, context),
        TypeDeclaration::RecordDeclaration(record) => Err(missing_layout(
            "Java formatter does not support record declarations yet",
            record.text_range(),
        )),
        TypeDeclaration::EnumDeclaration(enumeration) => {
            format_enum_declaration(enumeration, context)
        }
        TypeDeclaration::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, context)
        }
        TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, context)
        }
    }
}

pub(super) fn format_class_declaration(
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

pub(super) fn format_interface_declaration(
    interface: &InterfaceDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = interface.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty interface declaration",
            interface.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(interface.modifiers(), "interface", context)?;
    let name = interface.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an interface declaration without a name",
            interface.text_range(),
        )
    })?;
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = interface.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found an interface declaration without a body",
            interface.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty interface body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside interface headers yet",
    )?;
    let type_parameters = interface
        .type_parameters()
        .map(|parameters| format_type_parameter_list(&parameters))
        .transpose()?;
    let extends_clause = interface
        .extends_clause()
        .map(|clause| format_interface_extends_clause(&clause, context))
        .transpose()?;
    let permits_clause = interface
        .permits_clause()
        .map(|clause| format_permits_clause(&clause))
        .transpose()?;
    let body_members = format_interface_body(&body, context)?;

    let mut header = Vec::new();
    header.extend(modifiers.modifier_tokens.iter().map(format_token));
    header.push(text("interface"));
    header.push(if let Some(type_parameters) = type_parameters {
        concat([text(name.text()), type_parameters])
    } else {
        text(name.text())
    });
    if let Some(extends_clause) = extends_clause {
        header.push(extends_clause);
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

pub(super) fn format_annotation_interface_declaration(
    annotation: &AnnotationInterfaceDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = annotation.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty annotation interface declaration",
            annotation.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(annotation.modifiers(), "annotation interface", context)?;
    let name = annotation.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation interface declaration without a name",
            annotation.text_range(),
        )
    })?;
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = annotation.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation interface declaration without a body",
            annotation.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty annotation interface body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside annotation interface headers yet",
    )?;
    let body_members = format_annotation_interface_body(&body, context)?;

    let mut header = Vec::new();
    header.extend(modifiers.modifier_tokens.iter().map(format_token));
    header.push(text("@interface"));
    header.push(text(name.text()));
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

pub(super) fn format_enum_declaration(
    enumeration: &EnumDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = enumeration.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty enum declaration",
            enumeration.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(enumeration.modifiers(), "enum", context)?;

    if !enumeration.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this enum declaration shape yet",
            enumeration.text_range(),
        ));
    }

    let name = enumeration
        .name()
        .expect("validated enum declaration should have a name");
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = enumeration
        .body()
        .expect("validated enum declaration should have a body");
    let body_range = body
        .code_text_range()
        .expect("validated enum body should have code");
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside enum headers yet",
    )?;

    let implements_clause = enumeration
        .implements_clause()
        .map(|clause| format_implements_clause(&clause, context))
        .transpose()?;
    let body = format_enum_body(&body, context)?;

    let mut header = Vec::new();
    header.extend(modifiers.modifier_tokens.iter().map(format_token));
    header.push(text("enum"));
    header.push(format_token(&name));
    if let Some(implements_clause) = implements_clause {
        header.push(implements_clause);
    }
    let header = wrap::declaration_header(header);

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(concat([header, text(" "), body])),
    )
}

pub(super) fn format_enum_body(
    body: &EnumBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !body.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this enum body shape yet",
            body.text_range(),
        ));
    }

    let constants = body
        .constants()
        .map(|constants| format_enum_constant_list(&constants, context))
        .transpose()?
        .unwrap_or_default();
    let members = body
        .members()
        .map(|member| format_class_body_member(&member, context))
        .collect::<FormatResult<Vec<_>>>()?;

    if constants.is_empty() && members.is_empty() && !body.has_semicolon() {
        let code_range = body
            .code_text_range()
            .expect("validated enum body should have code");
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context, code_range,
        )?));
    }

    let mut items = constants;
    if body.has_semicolon() || !members.is_empty() {
        items.push(text(";"));
    }
    items.extend(members);
    Ok(wrap::braced_block(items))
}

pub(super) fn format_enum_constant_list(
    constants: &EnumConstantList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Vec<Doc>> {
    if !constants.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this enum constant list shape yet",
            constants.text_range(),
        ));
    }

    let constants = constants.constants().collect::<Vec<_>>();
    let last_index = constants.len().saturating_sub(1);
    constants
        .into_iter()
        .enumerate()
        .map(|(index, constant)| {
            let doc = format_enum_constant(&constant, context)?;
            if index == last_index {
                Ok(doc)
            } else {
                Ok(concat([doc, text(",")]))
            }
        })
        .collect()
}

pub(super) fn format_enum_constant(
    constant: &EnumConstant,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !constant.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this enum constant shape yet",
            constant.text_range(),
        ));
    }

    let code_range = constant
        .code_text_range()
        .expect("validated enum constant should have code");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = constant.modifiers();
    let mut annotations = constant
        .annotations()
        .map(|annotation| format_annotation(&annotation, context, "declaration"))
        .collect::<FormatResult<Vec<_>>>()?;
    let modifier_annotations = modifiers
        .as_ref()
        .map(|modifiers| format_annotation_list(modifiers.annotations(), context, "declaration"))
        .transpose()?
        .unwrap_or_default();
    annotations.extend(modifier_annotations);
    if let Some(modifiers) = modifiers
        && modifiers.modifier_tokens().next().is_some()
    {
        return Err(missing_layout(
            "Java formatter does not support enum constant modifier tokens yet",
            modifiers.text_range(),
        ));
    }

    let name = constant
        .name()
        .expect("validated enum constant should have a name");
    let arguments = constant
        .arguments()
        .map(|arguments| format_argument_list(&arguments, context))
        .transpose()?;
    let body = constant
        .body()
        .map(|body| format_class_body(&body, context))
        .transpose()?;

    let name = if annotations.is_empty() {
        format_token(&name)
    } else {
        concat([join(text(" "), annotations), text(" "), format_token(&name)])
    };
    let mut parts = vec![name];
    if let Some(arguments) = arguments {
        parts.push(arguments);
    }
    if let Some(body) = body {
        parts.push(text(" "));
        parts.push(wrap::braced_block(body));
    }
    let doc = concat(parts);
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_class_body(
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

pub(super) fn format_class_body_member(
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
        ClassBodyMember::EnumDeclaration(enumeration) => {
            format_enum_declaration(enumeration, context)
        }
        ClassBodyMember::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, context)
        }
        ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, context)
        }
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

pub(super) fn format_interface_body(
    body: &InterfaceBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Vec<Doc>> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body.code_text_range().ok_or_else(|| {
            missing_layout(
                "Java formatter found an empty interface body",
                body.text_range(),
            )
        })?;
        return take_dangling_comment_docs(context, code_range);
    }

    members
        .into_iter()
        .map(|member| format_interface_body_member(&member, context))
        .collect()
}

pub(super) fn format_interface_body_member(
    member: &InterfaceBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty interface body member",
            member.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = match member {
        InterfaceBodyMember::FieldDeclaration(field) => format_field_declaration(field, context),
        InterfaceBodyMember::MethodDeclaration(method) => {
            format_method_declaration(method, context)
        }
        InterfaceBodyMember::EmptyDeclaration(_) => Ok(text(";")),
        InterfaceBodyMember::ClassDeclaration(class) => format_class_declaration(class, context),
        InterfaceBodyMember::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, context)
        }
        InterfaceBodyMember::RecordDeclaration(record) => Err(missing_layout(
            "Java formatter does not support nested interface record declarations yet",
            record.text_range(),
        )),
        InterfaceBodyMember::EnumDeclaration(enumeration) => {
            format_enum_declaration(enumeration, context)
        }
        InterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, context)
        }
    }?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_annotation_interface_body(
    body: &AnnotationInterfaceBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Vec<Doc>> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body.code_text_range().ok_or_else(|| {
            missing_layout(
                "Java formatter found an empty annotation interface body",
                body.text_range(),
            )
        })?;
        return take_dangling_comment_docs(context, code_range);
    }

    members
        .into_iter()
        .map(|member| format_annotation_interface_body_member(&member, context))
        .collect()
}

pub(super) fn format_annotation_interface_body_member(
    member: &AnnotationInterfaceBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty annotation interface body member",
            member.text_range(),
        )
    })?;
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = match member {
        AnnotationInterfaceBodyMember::AnnotationElementDeclaration(element) => {
            format_annotation_element_declaration(element, context)
        }
        AnnotationInterfaceBodyMember::FieldDeclaration(field) => {
            format_field_declaration(field, context)
        }
        AnnotationInterfaceBodyMember::MethodDeclaration(method) => {
            format_method_declaration(method, context)
        }
        AnnotationInterfaceBodyMember::EmptyDeclaration(_) => Ok(text(";")),
        AnnotationInterfaceBodyMember::ClassDeclaration(class) => {
            format_class_declaration(class, context)
        }
        AnnotationInterfaceBodyMember::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, context)
        }
        AnnotationInterfaceBodyMember::EnumDeclaration(enumeration) => {
            format_enum_declaration(enumeration, context)
        }
        AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, context)
        }
        AnnotationInterfaceBodyMember::RecordDeclaration(record) => Err(missing_layout(
            "Java formatter does not support nested annotation interface record declarations yet",
            record.text_range(),
        )),
    }?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_annotation_element_declaration(
    element: &AnnotationElementDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !element.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this annotation element declaration shape yet",
            element.text_range(),
        ));
    }

    let modifiers = format_modifier_list(element.modifiers(), "annotation element", context)?;
    let ty = element.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element declaration without a type",
            element.text_range(),
        )
    })?;
    let name = element.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element declaration without a name",
            element.text_range(),
        )
    })?;
    let dimensions = element
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions))
        .transpose()?;
    let default_value = element
        .default_value()
        .map(|value| format_default_value(&value, context))
        .transpose()?;

    let mut declaration = concat([
        format_type(&ty, context)?,
        text(" "),
        format_token(&name),
        text("()"),
        dimensions.unwrap_or_else(|| text("")),
    ]);
    if let Some(default_value) = default_value {
        declaration = wrap::assignment_expression(declaration, text("default"), default_value);
    }
    Ok(modifiers.with_annotations(concat([declaration, text(";")])))
}

pub(super) fn format_default_value(
    value: &DefaultValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !value.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this default annotation value shape yet",
            value.text_range(),
        ));
    }
    let value = value.value().ok_or_else(|| {
        missing_layout(
            "Java formatter found a default annotation value without a value",
            value.text_range(),
        )
    })?;
    format_annotation_element_value(&value, context)
}

pub(super) fn format_field_declaration(
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
    let declarators = format_variable_declarator_list(&declarators, "field", context)?;

    let mut prefix = Vec::new();
    prefix.extend(modifiers.modifier_tokens.iter().map(format_token));
    prefix.push(format_type(&ty, context)?);
    Ok(modifiers.with_annotations(wrap::variable_declaration(prefix, declarators)))
}

pub(super) fn format_static_initializer(
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

pub(super) fn format_instance_initializer(
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

pub(super) fn format_method_declaration(
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

pub(super) fn format_constructor_declaration(
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

pub(super) fn format_callable_header(
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

pub(super) fn format_type_parameter_list(parameters: &TypeParameterList) -> FormatResult<Doc> {
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

pub(super) fn format_extends_clause(
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

pub(super) fn format_interface_extends_clause(
    clause: &ExtendsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_clause(
        "extends",
        clause.types(),
        clause.has_type_list_layout_shape(),
        clause.text_range(),
        context,
    )
}

pub(super) fn format_implements_clause(
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

pub(super) fn format_type_clause(
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

pub(super) fn format_permits_clause(clause: &PermitsClause) -> FormatResult<Doc> {
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

pub(super) fn format_formal_parameter_list(
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

pub(super) fn format_formal_parameter(
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

    let mut parts = parameter
        .modifiers()
        .map(|modifier| match modifier {
            FormalParameterModifier::Annotation(annotation) => {
                format_annotation(&annotation, context, "declaration")
            }
            FormalParameterModifier::Final(token) => Ok(format_token(&token)),
        })
        .collect::<FormatResult<Vec<_>>>()?;
    let ty = if let Some(ellipsis) = parameter.ellipsis() {
        let annotations = parameter
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation, context, "type-use"))
            .collect::<FormatResult<Vec<_>>>()?;
        if annotations.is_empty() {
            concat([format_type(&ty, context)?, format_token(&ellipsis)])
        } else {
            concat([
                format_type(&ty, context)?,
                text(" "),
                wrap::space_separated(annotations),
                text(" "),
                format_token(&ellipsis),
            ])
        }
    } else {
        format_type(&ty, context)?
    };
    let name = if let Some(dimensions) = parameter.dimensions() {
        concat([format_token(&name), format_array_dimensions(&dimensions)?])
    } else {
        format_token(&name)
    };
    parts.push(ty);
    parts.push(name);
    Ok(wrap::space_separated(parts))
}

pub(super) fn format_throws_clause(
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
