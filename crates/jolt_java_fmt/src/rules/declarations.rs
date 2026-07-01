use jolt_fmt_ir::{Doc, concat, group, hard_line, line, literal_text, soft_line, text};
use jolt_java_syntax::{
    AnnotationInterfaceDeclaration, ClassBodyMember, ClassDeclaration, EnumDeclaration,
    FieldDeclaration, FormalParameterList, InterfaceDeclaration, JavaSyntaxKind, JavaSyntaxToken,
    MethodDeclaration, ModifierList, RecordDeclaration, TypeDeclaration,
};

pub(crate) fn format_type_declaration(declaration: &TypeDeclaration) -> Doc {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class),
        TypeDeclaration::InterfaceDeclaration(interface) => format_interface_declaration(interface),
        TypeDeclaration::RecordDeclaration(record) => format_record_declaration(record),
        TypeDeclaration::EnumDeclaration(enum_) => format_enum_declaration(enum_),
        TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation)
        }
    }
}

fn format_class_declaration(class: &ClassDeclaration) -> Doc {
    let Some(name) = class.name() else {
        return source_doc(&class.source_text());
    };
    let Some(body) = class.body() else {
        return source_doc(&class.source_text());
    };
    let members = body.members().collect::<Vec<_>>();

    format_type_with_body(
        class.modifiers(),
        concat([
            text("class "),
            text(name.text().to_owned()),
            optional_source(class.type_parameters().map(|node| node.source_text())),
            optional_clause(class.extends_clause().map(|node| node.source_text())),
            optional_clause(class.implements_clause().map(|node| node.source_text())),
            optional_clause(class.permits_clause().map(|node| node.source_text())),
        ]),
        &members,
    )
}

fn format_interface_declaration(interface: &InterfaceDeclaration) -> Doc {
    let Some(name) = interface.name() else {
        return source_doc(&interface.source_text());
    };
    let members = interface
        .body()
        .map(|body| {
            body.members()
                .map(interface_member_to_class_member_doc)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    format_type_with_body(
        interface.modifiers(),
        concat([
            text("interface "),
            text(name.text().to_owned()),
            optional_source(interface.type_parameters().map(|node| node.source_text())),
            optional_clause(interface.extends_clause().map(|node| node.source_text())),
            optional_clause(interface.permits_clause().map(|node| node.source_text())),
        ]),
        &members,
    )
}

fn format_record_declaration(record: &RecordDeclaration) -> Doc {
    let Some(name) = record.name() else {
        return source_doc(&record.source_text());
    };
    let members = record
        .body()
        .map(|body| body.members().collect::<Vec<_>>())
        .unwrap_or_default();

    format_type_with_body(
        record.modifiers(),
        group(concat([
            text("record "),
            text(name.text().to_owned()),
            optional_source(record.type_parameters().map(|node| node.source_text())),
            format_record_components(record.components()),
            optional_clause(record.implements_clause().map(|node| node.source_text())),
        ])),
        &members,
    )
}

fn format_enum_declaration(enum_: &EnumDeclaration) -> Doc {
    let Some(name) = enum_.name() else {
        return source_doc(&enum_.source_text());
    };

    let Some(body) = enum_.body() else {
        return source_doc(&enum_.source_text());
    };

    let constants = body
        .constants()
        .map(|constants| {
            constants
                .constants()
                .map(|constant| constant.source_text().trim().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let members = body.members().collect::<Vec<_>>();
    let body_doc = format_enum_body_contents(constants, &members);

    format_header_with_body(
        enum_.modifiers(),
        concat([
            text("enum "),
            text(name.text().to_owned()),
            optional_clause(enum_.implements_clause().map(|node| node.source_text())),
        ]),
        body_doc,
    )
}

fn format_annotation_interface_declaration(annotation: &AnnotationInterfaceDeclaration) -> Doc {
    let Some(name) = annotation.name() else {
        return source_doc(&annotation.source_text());
    };

    format_header_with_body(
        annotation.modifiers(),
        concat([text("@interface "), text(name.text().to_owned())]),
        annotation.body().and_then(|body| {
            let members = body.members().collect::<Vec<_>>();
            (!members.is_empty()).then(|| source_doc(&body.source_text()))
        }),
    )
}

fn format_type_with_body(
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    members: &[ClassBodyMember],
) -> Doc {
    let effective_members = effective_members(members);
    let body = (!effective_members.is_empty()).then(|| {
        join_member_docs(
            effective_members
                .into_iter()
                .map(|member| FormattedMember::from_member(&member))
                .collect(),
        )
    });
    format_header_with_body(modifiers, header_tail, body)
}

fn format_header_with_body(
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    body: Option<Doc>,
) -> Doc {
    let header = concat([format_modifier_prefix(modifiers), header_tail, text(" {")]);
    concat([
        header,
        body.map_or_else(
            || concat([hard_line(), text("}")]),
            |body| {
                concat([
                    jolt_fmt_ir::indent(concat([hard_line(), body])),
                    hard_line(),
                    text("}"),
                ])
            },
        ),
    ])
}

fn format_enum_body_contents(constants: Vec<String>, members: &[ClassBodyMember]) -> Option<Doc> {
    let effective_members = effective_members(members);
    if constants.is_empty() && effective_members.is_empty() {
        return None;
    }

    let constants_doc = (!constants.is_empty()).then(|| {
        let constants_len = constants.len();
        join_docs(
            constants
                .into_iter()
                .enumerate()
                .map(|(index, constant)| {
                    let separator = if effective_members.is_empty() || index + 1 < constants_len {
                        ","
                    } else {
                        ";"
                    };
                    text(format!("{constant}{separator}"))
                })
                .collect(),
            &hard_line(),
        )
    });

    let members_doc = (!effective_members.is_empty()).then(|| {
        join_member_docs(
            effective_members
                .into_iter()
                .map(|member| FormattedMember::from_member(&member))
                .collect(),
        )
    });

    match (constants_doc, members_doc) {
        (Some(constants), Some(members)) => {
            Some(concat([constants, jolt_fmt_ir::empty_line(), members]))
        }
        (Some(constants), None) => Some(constants),
        (None, Some(members)) => Some(members),
        (None, None) => None,
    }
}

fn format_modifier_prefix(modifiers: Option<ModifierList>) -> Doc {
    let Some(modifiers) = modifiers else {
        return jolt_fmt_ir::nil();
    };

    let annotations = modifiers
        .annotations()
        .map(|annotation| annotation.source_text().trim().to_owned())
        .collect::<Vec<_>>();
    let modifier_tokens = sorted_modifier_tokens(modifiers.modifier_tokens().collect());

    let mut docs = Vec::new();
    for annotation in annotations {
        docs.push(text(annotation));
        docs.push(hard_line());
    }
    if !modifier_tokens.is_empty() {
        docs.push(jolt_fmt_ir::join(
            text(" "),
            modifier_tokens
                .into_iter()
                .map(|token| text(token.text().to_owned())),
        ));
        docs.push(text(" "));
    }

    concat(docs)
}

fn sorted_modifier_tokens(mut tokens: Vec<JavaSyntaxToken>) -> Vec<JavaSyntaxToken> {
    tokens.sort_by_key(|token| modifier_order(token.kind()));
    tokens
}

const fn modifier_order(kind: JavaSyntaxKind) -> u8 {
    match kind {
        JavaSyntaxKind::PublicKw => 0,
        JavaSyntaxKind::ProtectedKw => 1,
        JavaSyntaxKind::PrivateKw => 2,
        JavaSyntaxKind::AbstractKw => 3,
        JavaSyntaxKind::DefaultKw => 4,
        JavaSyntaxKind::StaticKw => 5,
        JavaSyntaxKind::FinalKw => 6,
        JavaSyntaxKind::TransientKw => 7,
        JavaSyntaxKind::VolatileKw => 8,
        JavaSyntaxKind::SynchronizedKw => 9,
        JavaSyntaxKind::NativeKw => 10,
        JavaSyntaxKind::StrictfpKw => 13,
        _ => u8::MAX,
    }
}

fn effective_members(members: &[ClassBodyMember]) -> Vec<ClassBodyMember> {
    members
        .iter()
        .filter(|member| member.kind() != JavaSyntaxKind::EmptyDeclaration)
        .cloned()
        .collect()
}

fn optional_source(source: Option<String>) -> Doc {
    source.map_or_else(jolt_fmt_ir::nil, |source| text(source.trim().to_owned()))
}

fn optional_clause(source: Option<String>) -> Doc {
    source.map_or_else(jolt_fmt_ir::nil, |source| {
        concat([text(" "), text(source.trim().to_owned())])
    })
}

fn format_record_components(components: Option<jolt_java_syntax::RecordComponentList>) -> Doc {
    let Some(components) = components else {
        return text("()");
    };
    parenthesized_comma_list(
        components
            .components()
            .map(|component| text(component.source_text().trim().to_owned()))
            .collect(),
    )
}

fn join_member_docs(members: Vec<FormattedMember>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_category = None;

    for member in members {
        if !joined.is_empty() {
            if previous_category == Some(member.category) {
                joined.push(hard_line());
            } else {
                joined.push(jolt_fmt_ir::empty_line());
            }
        }
        previous_category = Some(member.category);
        joined.push(member.doc);
    }

    concat(joined)
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

fn interface_member_to_class_member_doc(
    member: jolt_java_syntax::InterfaceBodyMember,
) -> ClassBodyMember {
    match member {
        jolt_java_syntax::InterfaceBodyMember::EmptyDeclaration(member) => {
            ClassBodyMember::EmptyDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::ClassDeclaration(member) => {
            ClassBodyMember::ClassDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::RecordDeclaration(member) => {
            ClassBodyMember::RecordDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::EnumDeclaration(member) => {
            ClassBodyMember::EnumDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::InterfaceDeclaration(member) => {
            ClassBodyMember::InterfaceDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::AnnotationInterfaceDeclaration(member) => {
            ClassBodyMember::AnnotationInterfaceDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::FieldDeclaration(member) => {
            ClassBodyMember::FieldDeclaration(member)
        }
        jolt_java_syntax::InterfaceBodyMember::MethodDeclaration(member) => {
            ClassBodyMember::MethodDeclaration(member)
        }
    }
}

fn source_doc(source: &str) -> Doc {
    literal_text(source.trim().to_owned())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MemberCategory {
    Field,
    Constructor,
    Method,
    Initializer,
    Type,
}

struct FormattedMember {
    category: MemberCategory,
    doc: Doc,
}

impl FormattedMember {
    fn from_member(member: &ClassBodyMember) -> Self {
        match member {
            ClassBodyMember::FieldDeclaration(field) => Self {
                category: MemberCategory::Field,
                doc: format_field_declaration(field),
            },
            ClassBodyMember::ConstructorDeclaration(constructor) => Self {
                category: MemberCategory::Constructor,
                doc: format_constructor_declaration(constructor),
            },
            ClassBodyMember::MethodDeclaration(method) => Self {
                category: MemberCategory::Method,
                doc: format_method_declaration(method),
            },
            ClassBodyMember::StaticInitializer(member) => Self {
                category: MemberCategory::Initializer,
                doc: source_doc(&member.source_text()),
            },
            ClassBodyMember::InstanceInitializer(member) => Self {
                category: MemberCategory::Initializer,
                doc: source_doc(&member.source_text()),
            },
            _ => Self {
                category: MemberCategory::Type,
                doc: source_doc(&member.source_text()),
            },
        }
    }
}

fn format_field_declaration(field: &FieldDeclaration) -> Doc {
    concat([
        format_modifier_prefix(field.modifiers()),
        field.ty().map_or_else(
            || source_doc(&field.source_text()),
            |ty| text(ty.source_text().trim().to_owned()),
        ),
        text(" "),
        field
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                text(declarators.source_text().trim().to_owned())
            }),
        text(";"),
    ])
}

fn format_constructor_declaration(constructor: &jolt_java_syntax::ConstructorDeclaration) -> Doc {
    let Some(name) = constructor.name() else {
        return source_doc(&constructor.source_text());
    };
    concat([
        group(concat([
            format_modifier_prefix(constructor.modifiers()),
            optional_source(constructor.type_parameters().map(|node| node.source_text())),
            text(name.text().to_owned()),
            format_parameters(constructor.parameters()),
            format_throws_clause(constructor.throws_clause()),
        ])),
        format_empty_executable_body(constructor.body().map(|body| body.source_text())),
    ])
}

fn format_method_declaration(method: &MethodDeclaration) -> Doc {
    let Some(name) = method.name() else {
        return source_doc(&method.source_text());
    };
    concat([
        group(concat([
            format_modifier_prefix(method.modifiers()),
            optional_source(method.type_parameters().map(|node| node.source_text())),
            method
                .return_type()
                .map_or_else(jolt_fmt_ir::nil, |return_type| {
                    concat([text(return_type.source_text().trim().to_owned()), text(" ")])
                }),
            text(name.text().to_owned()),
            format_parameters(method.parameters()),
            format_throws_clause(method.throws_clause()),
        ])),
        format_empty_executable_body(method.body().map(|body| body.source_text())),
    ])
}

fn format_parameters(parameters: Option<FormalParameterList>) -> Doc {
    let Some(parameters) = parameters else {
        return text("()");
    };
    parenthesized_comma_list(
        parameters
            .parameters()
            .map(|parameter| text(parameter.source_text().trim().to_owned()))
            .collect(),
    )
}

fn parenthesized_comma_list(items: Vec<Doc>) -> Doc {
    if items.is_empty() {
        return text("()");
    }

    concat([
        text("("),
        jolt_fmt_ir::indent(concat([
            soft_line(),
            join_docs(items, &concat([text(","), line()])),
        ])),
        soft_line(),
        text(")"),
    ])
}

fn format_throws_clause(throws: Option<jolt_java_syntax::ThrowsClause>) -> Doc {
    let Some(throws) = throws else {
        return jolt_fmt_ir::nil();
    };
    let exceptions = throws
        .exceptions()
        .map(|exception| text(exception.source_text().trim().to_owned()))
        .collect::<Vec<_>>();
    if exceptions.is_empty() {
        return jolt_fmt_ir::nil();
    }

    let docs = vec![
        line(),
        text("throws "),
        join_docs(exceptions, &concat([text(","), line()])),
    ];
    jolt_fmt_ir::indent(concat(docs))
}

fn format_empty_executable_body(body: Option<String>) -> Doc {
    let Some(body) = body else {
        return text(";");
    };
    if body.trim() == "{}" {
        concat([text(" {"), hard_line(), text("}")])
    } else {
        concat([text(" "), source_doc(&body)])
    }
}
