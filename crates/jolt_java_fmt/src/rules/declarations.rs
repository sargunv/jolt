use super::{
    AnnotationElementDeclaration, AnnotationInterfaceBody, AnnotationInterfaceBodyMember,
    AnnotationInterfaceDeclaration, ClassBody, ClassBodyMember, ClassDeclaration,
    CompactConstructorDeclaration, ConstructorDeclaration, DefaultValue, Doc, EnumBody,
    EnumConstant, EnumConstantList, EnumDeclaration, ExtendsClause, FieldDeclaration,
    FormalParameter, FormalParameterList, FormalParameterModifier, FormatResult, ImplementsClause,
    InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaFormatContext, JavaSyntaxToken,
    MethodDeclaration, ModifierList, PermitsClause, ReceiverParameter, RecordComponent,
    RecordComponentList, RecordDeclaration, ThrowsClause, Type, TypeBoundList, TypeDeclaration,
    TypeLayoutPart, TypeParameterList, concat, format_annotation, format_annotation_element_value,
    format_annotation_list, format_argument_list, format_array_dimensions, format_block,
    format_constructor_body, format_modifier_list, format_name, format_token, format_type,
    format_variable_declarator_list, java_lists, join, reject_unhandled_comments_before_end,
    reject_unhandled_comments_before_start, take_dangling_comment_docs, take_leading_comment_docs,
    text, with_leading_and_trailing_comments, wrap,
};
use crate::helpers::{callables, type_declarations};
use jolt_fmt_ir::{empty_line, hard_line};

pub(super) fn format_type_declaration(
    declaration: &TypeDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class, context),
        TypeDeclaration::RecordDeclaration(record) => format_record_declaration(record, context),
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

pub(super) fn format_record_declaration(
    record: &RecordDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = record
        .code_text_range()
        .expect("parser-clean record declaration should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(record.modifiers(), "record", context)?;

    let name = record
        .name()
        .expect("parser-clean record declaration should have a name");
    let components = record
        .components()
        .map(|components| format_record_component_list(&components, context))
        .transpose()?
        .unwrap_or_else(|| java_lists::empty_argument_list(context.policy()));
    let type_parameters = record
        .type_parameters()
        .map(|parameters| format_type_parameter_list(&parameters, context))
        .transpose()?;
    let implements_clause = record
        .implements_clause()
        .map(|clause| format_implements_clause(&clause, context))
        .transpose()?;
    let body = record
        .body()
        .expect("parser-clean record declaration should have a body");
    let body_members = format_record_body(&body, context)?;

    let doc = type_declarations::type_declaration(type_declarations::TypeDeclaration {
        modifiers: modifiers.modifier_docs(),
        keyword: text("record"),
        name: format_token(&name),
        type_parameters,
        record_components: Some(components),
        extends_clause: None,
        implements_clause,
        permits_clause: None,
        body: braced_type_body(body_members),
    });
    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(doc),
    )
}

pub(super) fn format_class_declaration(
    class: &ClassDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = class
        .code_text_range()
        .expect("parser-clean class declaration should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(class.modifiers(), "class", context)?;

    let name = class
        .name()
        .expect("parser-clean class declaration should have a name");
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = class
        .body()
        .expect("parser-clean class declaration should have a body");
    let type_parameters = class
        .type_parameters()
        .map(|parameters| format_type_parameter_list(&parameters, context))
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
        .map(|clause| format_permits_clause(&clause, context))
        .transpose()?;
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside class headers yet",
        )?;
    }
    let body_members = format_class_body(&body, context)?;

    let doc = type_declarations::type_declaration(type_declarations::TypeDeclaration {
        modifiers: modifiers.modifier_docs(),
        keyword: text("class"),
        name: format_token(&name),
        type_parameters,
        record_components: None,
        extends_clause,
        implements_clause,
        permits_clause,
        body: braced_type_body(body_members),
    });

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(doc),
    )
}

pub(super) struct TypeBodyLayout {
    members: Vec<Doc>,
    separators: Vec<Doc>,
    has_members: bool,
}

pub(super) fn braced_type_body(body: TypeBodyLayout) -> Doc {
    if body.has_members {
        wrap::braced_block_with_separators(body.members, body.separators)
    } else {
        wrap::braced_block(body.members)
    }
}

pub(super) fn format_interface_declaration(
    interface: &InterfaceDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = interface
        .code_text_range()
        .expect("parser-clean interface declaration should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(interface.modifiers(), "interface", context)?;
    let name = interface
        .name()
        .expect("parser-clean interface declaration should have a name");
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = interface
        .body()
        .expect("parser-clean interface declaration should have a body");
    let type_parameters = interface
        .type_parameters()
        .map(|parameters| format_type_parameter_list(&parameters, context))
        .transpose()?;
    let extends_clause = interface
        .extends_clause()
        .map(|clause| format_interface_extends_clause(&clause, context))
        .transpose()?;
    let permits_clause = interface
        .permits_clause()
        .map(|clause| format_permits_clause(&clause, context))
        .transpose()?;
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside interface headers yet",
        )?;
    }
    let body_members = format_interface_body(&body, context)?;

    let doc = type_declarations::type_declaration(type_declarations::TypeDeclaration {
        modifiers: modifiers.modifier_docs(),
        keyword: text("interface"),
        name: format_token(&name),
        type_parameters,
        record_components: None,
        extends_clause,
        implements_clause: None,
        permits_clause,
        body: braced_type_body(body_members),
    });

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(doc),
    )
}

pub(super) fn format_annotation_interface_declaration(
    annotation: &AnnotationInterfaceDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = annotation
        .code_text_range()
        .expect("parser-clean annotation interface declaration should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(annotation.modifiers(), "annotation interface", context)?;
    let name = annotation
        .name()
        .expect("parser-clean annotation interface declaration should have a name");
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = annotation
        .body()
        .expect("parser-clean annotation interface declaration should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside annotation interface headers yet",
        )?;
    }
    let body_members = format_annotation_interface_body(&body, context)?;

    let doc = type_declarations::type_declaration(type_declarations::TypeDeclaration {
        modifiers: modifiers.modifier_docs(),
        keyword: text("@interface"),
        name: format_token(&name),
        type_parameters: None,
        record_components: None,
        extends_clause: None,
        implements_clause: None,
        permits_clause: None,
        body: braced_type_body(body_members),
    });

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(doc),
    )
}

pub(super) fn format_enum_declaration(
    enumeration: &EnumDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = enumeration
        .code_text_range()
        .expect("parser-clean enum declaration should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let modifiers = format_modifier_list(enumeration.modifiers(), "enum", context)?;

    let name = enumeration
        .name()
        .expect("parser-clean enum declaration should have a name");
    if modifiers.has_annotations() {
        reject_unhandled_comments_before_start(
            context,
            name.token_text_range(),
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let body = enumeration
        .body()
        .expect("parser-clean enum declaration should have a body");
    let implements_clause = enumeration
        .implements_clause()
        .map(|clause| format_implements_clause(&clause, context))
        .transpose()?;
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside enum headers yet",
        )?;
    }
    let body = format_enum_body(&body, context)?;

    let doc = type_declarations::type_declaration(type_declarations::TypeDeclaration {
        modifiers: modifiers.modifier_docs(),
        keyword: text("enum"),
        name: format_token(&name),
        type_parameters: None,
        record_components: None,
        extends_clause: None,
        implements_clause,
        permits_clause: None,
        body,
    });

    with_leading_and_trailing_comments(
        context,
        code_range,
        leading_comments,
        modifiers.with_annotations(doc),
    )
}

pub(super) fn format_enum_body(
    body: &EnumBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
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
            .expect("parser-clean enum body should have a code range");
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
    let has_trailing_comma = constants.has_trailing_comma();
    let constants = constants.constants().collect::<Vec<_>>();
    let last_index = constants.len().saturating_sub(1);
    constants
        .into_iter()
        .enumerate()
        .map(|(index, constant)| {
            let doc = format_enum_constant(&constant, context)?;
            if index != last_index || has_trailing_comma {
                Ok(concat([doc, text(",")]))
            } else {
                Ok(doc)
            }
        })
        .collect()
}

pub(super) fn format_enum_constant(
    constant: &EnumConstant,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = constant
        .code_text_range()
        .expect("parser-clean enum constant should have a code range");
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
        panic!("parser-clean enum constant should not contain modifier keywords");
    }

    let name = constant
        .name()
        .expect("parser-clean enum constant should have a name");
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
        parts.push(braced_type_body(body));
    }
    let doc = concat(parts);
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_class_body(
    body: &ClassBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<TypeBodyLayout> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body
            .code_text_range()
            .expect("parser-clean class body should have a code range");
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, code_range)?,
            separators: Vec::new(),
            has_members: false,
        });
    }

    let separators = class_body_member_separators(&members, context);
    let members = members
        .iter()
        .map(|member| format_class_body_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(TypeBodyLayout {
        members,
        separators,
        has_members: true,
    })
}

pub(super) fn format_record_body(
    body: &jolt_java_syntax::RecordBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<TypeBodyLayout> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body
            .code_text_range()
            .expect("parser-clean record body should have a code range");
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, code_range)?,
            separators: Vec::new(),
            has_members: false,
        });
    }

    let separators = class_body_member_separators(&members, context);
    let members = members
        .iter()
        .map(|member| format_class_body_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(TypeBodyLayout {
        members,
        separators,
        has_members: true,
    })
}

pub(super) fn format_class_body_member(
    member: &ClassBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member
        .code_text_range()
        .expect("parser-clean class body member should have a code range");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = match member {
        ClassBodyMember::FieldDeclaration(field) => format_field_declaration(field, context),
        ClassBodyMember::MethodDeclaration(method) => format_method_declaration(method, context),
        ClassBodyMember::ConstructorDeclaration(constructor) => {
            format_constructor_declaration(constructor, context)
        }
        ClassBodyMember::EmptyDeclaration(_) => Ok(text(";")),
        ClassBodyMember::ClassDeclaration(class) => format_class_declaration(class, context),
        ClassBodyMember::RecordDeclaration(record) => format_record_declaration(record, context),
        ClassBodyMember::EnumDeclaration(enumeration) => {
            format_enum_declaration(enumeration, context)
        }
        ClassBodyMember::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, context)
        }
        ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, context)
        }
        ClassBodyMember::CompactConstructorDeclaration(constructor) => {
            format_compact_constructor_declaration(constructor, context)
        }
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
) -> FormatResult<TypeBodyLayout> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body
            .code_text_range()
            .expect("parser-clean interface body should have a code range");
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, code_range)?,
            separators: Vec::new(),
            has_members: false,
        });
    }

    let separators = interface_body_member_separators(&members, context);
    let members = members
        .iter()
        .map(|member| format_interface_body_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(TypeBodyLayout {
        members,
        separators,
        has_members: true,
    })
}

pub(super) fn format_interface_body_member(
    member: &InterfaceBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member
        .code_text_range()
        .expect("parser-clean interface body member should have a code range");
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
        InterfaceBodyMember::RecordDeclaration(record) => {
            format_record_declaration(record, context)
        }
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
) -> FormatResult<TypeBodyLayout> {
    let members = body.members().collect::<Vec<_>>();
    if members.is_empty() {
        let code_range = body
            .code_text_range()
            .expect("parser-clean annotation interface body should have a code range");
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, code_range)?,
            separators: Vec::new(),
            has_members: false,
        });
    }

    let separators = annotation_interface_body_member_separators(&members, context);
    let members = members
        .iter()
        .map(|member| format_annotation_interface_body_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(TypeBodyLayout {
        members,
        separators,
        has_members: true,
    })
}

fn class_body_member_separators(
    members: &[ClassBodyMember],
    context: &JavaFormatContext<'_>,
) -> Vec<Doc> {
    type_body_member_separators(
        members,
        jolt_java_syntax::ClassBodyMember::code_text_range,
        |left, right| {
            matches!(left, ClassBodyMember::FieldDeclaration(_))
                && matches!(right, ClassBodyMember::FieldDeclaration(_))
        },
        context,
    )
}

fn interface_body_member_separators(
    members: &[InterfaceBodyMember],
    context: &JavaFormatContext<'_>,
) -> Vec<Doc> {
    type_body_member_separators(
        members,
        jolt_java_syntax::InterfaceBodyMember::code_text_range,
        |left, right| {
            matches!(left, InterfaceBodyMember::FieldDeclaration(_))
                && matches!(right, InterfaceBodyMember::FieldDeclaration(_))
        },
        context,
    )
}

fn annotation_interface_body_member_separators(
    members: &[AnnotationInterfaceBodyMember],
    context: &JavaFormatContext<'_>,
) -> Vec<Doc> {
    type_body_member_separators(
        members,
        jolt_java_syntax::AnnotationInterfaceBodyMember::code_text_range,
        |left, right| {
            matches!(left, AnnotationInterfaceBodyMember::FieldDeclaration(_))
                && matches!(right, AnnotationInterfaceBodyMember::FieldDeclaration(_))
        },
        context,
    )
}

fn type_body_member_separators<Member>(
    members: &[Member],
    range: impl Fn(&Member) -> Option<jolt_diagnostics::TextRange>,
    keep_adjacent: impl Fn(&Member, &Member) -> bool,
    context: &JavaFormatContext<'_>,
) -> Vec<Doc> {
    members
        .windows(2)
        .map(|window| {
            let left = range(&window[0]);
            let right = range(&window[1]);
            if let (Some(left), Some(right)) = (left, right)
                && context.has_blank_line_between(left, right)
            {
                return empty_line();
            }
            if keep_adjacent(&window[0], &window[1]) {
                return hard_line();
            }
            empty_line()
        })
        .collect()
}

pub(super) fn format_annotation_interface_body_member(
    member: &AnnotationInterfaceBodyMember,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = member
        .code_text_range()
        .expect("parser-clean annotation interface body member should have a code range");
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
        AnnotationInterfaceBodyMember::RecordDeclaration(record) => {
            format_record_declaration(record, context)
        }
    }?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_annotation_element_declaration(
    element: &AnnotationElementDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let modifiers = format_modifier_list(element.modifiers(), "annotation element", context)?;
    let ty = element
        .ty()
        .expect("parser-clean annotation element should have a type");
    let name = element
        .name()
        .expect("parser-clean annotation element should have a name");
    let dimensions = element
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions, context))
        .transpose()?;
    let default_value = element
        .default_value()
        .map(|value| format_default_value(&value, context))
        .transpose()?;

    let declaration =
        callables::annotation_element_declaration(callables::AnnotationElementDeclaration {
            result_type: format_type(&ty, context)?,
            name: format_token(&name),
            dimensions,
            default_value,
        });
    Ok(modifiers.with_annotations(declaration))
}

pub(super) fn format_default_value(
    value: &DefaultValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let value = value
        .value()
        .expect("parser-clean default value should have an annotation element value");
    format_annotation_element_value(&value, context)
        .map(super::super::helpers::annotations::AnnotationValue::into_doc)
}

pub(super) fn format_field_declaration(
    field: &FieldDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let modifiers = format_modifier_list(field.modifiers(), "field", context)?;
    let ty = field
        .ty()
        .expect("parser-clean field declaration should have a type");
    if modifiers.has_annotations()
        && let Some(ty_range) = ty.code_text_range()
    {
        reject_unhandled_comments_before_start(
            context,
            ty_range,
            "Java formatter does not support comments between declaration annotations and declaration headers yet",
        )?;
    }
    let declarators = field
        .declarators()
        .expect("parser-clean field declaration should have declarators");
    let declarators = format_variable_declarator_list(&declarators, "field", context)?;

    let mut prefix = Vec::new();
    prefix.extend(modifiers.modifier_docs());
    prefix.push(format_type(&ty, context)?);
    Ok(modifiers.with_annotations(wrap::variable_declaration(prefix, declarators)))
}

pub(super) fn format_static_initializer(
    initializer: &jolt_java_syntax::StaticInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = initializer
        .body()
        .expect("parser-clean static initializer should have a body");
    Ok(concat([text("static "), format_block(&body, context)?]))
}

pub(super) fn format_instance_initializer(
    initializer: &jolt_java_syntax::InstanceInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = initializer
        .body()
        .expect("parser-clean instance initializer should have a body");
    format_block(&body, context)
}

pub(super) fn format_method_declaration(
    method: &MethodDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let return_type = method
        .return_type()
        .expect("parser-clean method declaration should have a return type");
    let break_after_return_type = type_contains_type_arguments(&return_type);
    let name = method
        .name()
        .expect("parser-clean method declaration should have a name");
    let mut return_type_parts = method
        .result_annotations()
        .map(|annotation| format_annotation(&annotation, context, "type-use"))
        .collect::<FormatResult<Vec<_>>>()?;
    return_type_parts.push(format_type(&return_type, context)?);
    if let Some(dimensions) = method.dimensions() {
        return_type_parts.push(format_array_dimensions(&dimensions, context)?);
    }
    let return_type = wrap::space_separated(return_type_parts);
    let header = format_callable_header(
        method.modifiers(),
        "method",
        context,
        method.type_parameters(),
        Some(return_type),
        break_after_return_type,
        &name,
        method.parameters(),
        method.throws_clause(),
    )?;

    if method.has_semicolon_body() {
        if let Some(code_range) = method.code_text_range() {
            reject_unhandled_comments_before_end(
                context,
                code_range,
                "Java formatter does not support comments inside method signatures yet",
            )?;
        }
        return Ok(concat([header, text(";")]));
    }

    let body = method
        .body()
        .expect("parser-clean method declaration should have a body or semicolon");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside method signatures yet",
        )?;
    }
    Ok(concat([header, text(" "), format_block(&body, context)?]))
}

pub(super) fn format_constructor_declaration(
    constructor: &ConstructorDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let name = constructor
        .name()
        .expect("parser-clean constructor declaration should have a name");
    let body = constructor
        .body()
        .expect("parser-clean constructor declaration should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside constructor signatures yet",
        )?;
    }
    let header = format_callable_header(
        constructor.modifiers(),
        "constructor",
        context,
        constructor.type_parameters(),
        None,
        false,
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

pub(super) fn format_compact_constructor_declaration(
    constructor: &CompactConstructorDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let modifiers = format_modifier_list(constructor.modifiers(), "constructor", context)?;
    let name = constructor
        .name()
        .expect("parser-clean compact constructor declaration should have a name");
    let body = constructor
        .body()
        .expect("parser-clean compact constructor declaration should have a body");
    let header = callables::callable_header(callables::CallableHeader {
        modifiers: modifiers.modifier_docs(),
        type_parameters: None,
        leading_type: None,
        break_after_leading_type: false,
        name: format_token(&name),
        parameters: None,
        tail: None,
    });

    Ok(modifiers.with_annotations(concat([
        header,
        text(" "),
        format_constructor_body(&body, context)?,
    ])))
}

pub(super) fn format_callable_header(
    modifiers: Option<ModifierList>,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
    type_parameters: Option<TypeParameterList>,
    leading_type: Option<Doc>,
    break_after_leading_type: bool,
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
        .map(|parameters| format_type_parameter_list(&parameters, context))
        .transpose()?;
    let parameters = parameters
        .map(|parameters| format_formal_parameter_list(&parameters, context))
        .transpose()?
        .unwrap_or_else(|| java_lists::empty_formal_parameter_list(context.policy()));
    let throws_clause = throws_clause
        .map(|throws| format_throws_clause(&throws, context))
        .transpose()?;

    let header = callables::callable_header(callables::CallableHeader {
        modifiers: modifiers.modifier_docs(),
        type_parameters,
        leading_type,
        break_after_leading_type,
        name: text(name.text()),
        parameters: Some(parameters),
        tail: throws_clause,
    });

    Ok(modifiers.with_annotations(header))
}

fn type_contains_type_arguments(ty: &Type) -> bool {
    ty.layout_parts()
        .iter()
        .any(|part| matches!(part, TypeLayoutPart::Token(token) if token.text() == "<"))
}

pub(super) fn format_type_parameter_list(
    parameters: &TypeParameterList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = parameters.text_range();
    let parameter_docs = parameters
        .parameters()
        .map(|parameter| {
            let range = parameter
                .code_text_range()
                .expect("parser-clean type parameter should have a code range");
            let parameter = parameter.clone();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type_parameter(&parameter, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    assert!(
        !parameter_docs.is_empty(),
        "parser-clean type parameter list should not be empty"
    );

    java_lists::type_parameter_list(parameter_docs, list_range, context)
}

fn format_type_parameter(
    parameter: &jolt_java_syntax::TypeParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let annotations = format_annotation_list(parameter.annotations(), context, "type-use")?;
    let name = parameter
        .name()
        .expect("parser-clean type parameter should have a name");

    let mut parts = annotations;
    parts.push(format_token(&name));
    if let Some(bounds) = parameter.bounds() {
        parts.push(format_type_bound_list(&bounds, context)?);
    }

    Ok(wrap::space_separated(parts))
}

fn format_type_bound_list(
    bounds: &TypeBoundList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = bounds
        .ty()
        .expect("parser-clean type bound list should have a type");

    Ok(concat([text("extends "), format_type(&ty, context)?]))
}

pub(super) fn format_extends_clause(
    clause: &ExtendsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let clause_range = clause.text_range();
    let ownership_range = clause
        .code_text_range()
        .expect("parser-clean extends clause should have a code range");
    format_type_clause(
        "extends",
        clause_range,
        ownership_range,
        clause.types(),
        context,
    )
}

pub(super) fn format_interface_extends_clause(
    clause: &ExtendsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let clause_range = clause.text_range();
    let ownership_range = clause
        .code_text_range()
        .expect("parser-clean extends clause should have a code range");
    format_type_clause(
        "extends",
        clause_range,
        ownership_range,
        clause.types(),
        context,
    )
}

pub(super) fn format_implements_clause(
    clause: &ImplementsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let clause_range = clause.text_range();
    let ownership_range = clause
        .code_text_range()
        .expect("parser-clean implements clause should have a code range");
    format_type_clause(
        "implements",
        clause_range,
        ownership_range,
        clause.types(),
        context,
    )
}

pub(super) fn format_type_clause(
    keyword: &'static str,
    clause_range: jolt_diagnostics::TextRange,
    ownership_range: jolt_diagnostics::TextRange,
    types: impl Iterator<Item = Type>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let types = types
        .map(|ty| {
            let range = ty
                .code_text_range()
                .expect("parser-clean type clause item should have a code range");
            let ty = ty.clone();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type(&ty, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    assert!(
        !types.is_empty(),
        "parser-clean {keyword} clause should contain at least one type"
    );

    java_lists::keyword_prefixed_clause_list(keyword, types, clause_range, ownership_range, context)
}

pub(super) fn format_permits_clause(
    clause: &PermitsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = clause.text_range();
    let ownership_range = clause
        .code_text_range()
        .expect("parser-clean permits clause should have a code range");
    let names = clause
        .names()
        .map(|name| {
            let range = name
                .code_text_range()
                .expect("parser-clean permits clause item should have a code range");
            java_lists::ListItem::doc(format_name(&name), range)
        })
        .collect::<Vec<_>>();
    assert!(
        !names.is_empty(),
        "parser-clean permits clause should contain at least one name"
    );

    java_lists::keyword_prefixed_clause_list("permits", names, list_range, ownership_range, context)
}

pub(super) fn format_formal_parameter_list(
    parameters: &FormalParameterList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = parameters.text_range();
    let mut parameter_docs = Vec::new();
    if let Some(receiver) = parameters.receiver() {
        let range = receiver
            .code_text_range()
            .expect("parser-clean receiver parameter should have a code range");
        let receiver = receiver.clone();
        parameter_docs.push(java_lists::ListItem::new(range, move |context| {
            format_receiver_parameter(&receiver, context)
        }));
    }
    parameter_docs.extend(
        parameters
            .parameters()
            .map(|parameter| {
                let range = parameter
                    .code_text_range()
                    .expect("parser-clean formal parameter should have a code range");
                let parameter = parameter.clone();
                Ok(java_lists::ListItem::new(range, move |context| {
                    format_formal_parameter(&parameter, context)
                }))
            })
            .collect::<FormatResult<Vec<_>>>()?,
    );
    assert!(
        !parameter_docs.is_empty(),
        "parser-clean formal parameter list node should contain parameters"
    );

    java_lists::formal_parameter_list(parameter_docs, list_range, context)
}

fn format_receiver_parameter(
    parameter: &ReceiverParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let annotations = format_annotation_list(parameter.annotations(), context, "type-use")?;
    let ty = parameter
        .ty()
        .expect("parser-clean receiver parameter should have a type");
    let this_token = parameter
        .this_token()
        .expect("parser-clean receiver parameter should have `this`");

    let mut parts = annotations;
    parts.push(format_type(&ty, context)?);
    if let Some(qualifier) = parameter.qualifier() {
        parts.push(concat([format_token(&qualifier), text(".")]));
    }
    parts.push(format_token(&this_token));

    Ok(wrap::space_separated(parts))
}

pub(super) fn format_formal_parameter(
    parameter: &FormalParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = parameter
        .ty()
        .expect("parser-clean formal parameter should have a type");
    let name = parameter
        .name()
        .expect("parser-clean formal parameter should have a name");

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
        concat([
            format_token(&name),
            format_array_dimensions(&dimensions, context)?,
        ])
    } else {
        format_token(&name)
    };
    parts.push(ty);
    parts.push(name);
    Ok(wrap::space_separated(parts))
}

pub(super) fn format_record_component_list(
    components: &RecordComponentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = components.text_range();
    let component_docs = components
        .components()
        .map(|component| {
            let range = component
                .code_text_range()
                .expect("parser-clean record component should have a code range");
            let component = component.clone();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_record_component(&component, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;

    java_lists::formal_parameter_list(component_docs, list_range, context)
}

pub(super) fn format_record_component(
    component: &RecordComponent,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = component
        .ty()
        .expect("parser-clean record component should have a type");
    let name = component
        .name()
        .expect("parser-clean record component should have a name");

    let annotations = component
        .annotations()
        .map(|annotation| format_annotation(&annotation, context, "declaration"))
        .collect::<FormatResult<Vec<_>>>()?;
    let ty = if let Some(ellipsis) = component.ellipsis() {
        let varargs_annotations = component
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation, context, "type-use"))
            .collect::<FormatResult<Vec<_>>>()?;
        if varargs_annotations.is_empty() {
            concat([format_type(&ty, context)?, format_token(&ellipsis)])
        } else {
            concat([
                format_type(&ty, context)?,
                text(" "),
                wrap::space_separated(varargs_annotations),
                text(" "),
                format_token(&ellipsis),
            ])
        }
    } else {
        format_type(&ty, context)?
    };
    let name = if let Some(dimensions) = component.dimensions() {
        concat([
            format_token(&name),
            format_array_dimensions(&dimensions, context)?,
        ])
    } else {
        format_token(&name)
    };
    let component = concat([ty, text(" "), name]);
    if annotations.is_empty() {
        return Ok(component);
    }

    let inline = wrap::space_separated(
        annotations
            .iter()
            .cloned()
            .chain(std::iter::once(component.clone())),
    );
    let vertical = concat([join(hard_line(), annotations), hard_line(), component]);
    Ok(jolt_fmt_ir::best_fitting(inline, [vertical]))
}

pub(super) fn format_throws_clause(
    throws: &ThrowsClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = throws.text_range();
    let ownership_range = throws
        .code_text_range()
        .expect("parser-clean throws clause should have a code range");
    let types = throws
        .types()
        .map(|ty| {
            let range = ty
                .code_text_range()
                .expect("parser-clean throws clause item should have a code range");
            let ty = ty.clone();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type(&ty, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    assert!(
        !types.is_empty(),
        "parser-clean throws clause should contain at least one type"
    );

    java_lists::keyword_prefixed_clause_list("throws", types, list_range, ownership_range, context)
}
