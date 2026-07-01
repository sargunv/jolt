use jolt_fmt_ir::{Doc, concat, group, hard_line, line, soft_line, text};
use jolt_java_syntax::{
    AnnotationElementDeclaration, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    ClassBody, ClassBodyMember, ClassDeclaration, EnumConstant, EnumDeclaration, ExtendsClause,
    FormalParameterList, ImplementsClause, InterfaceDeclaration, JavaSyntaxKind, MethodDeclaration,
    ModifierList, PermitsClause, RecordDeclaration, TypeDeclaration,
};

use crate::helpers::blocks::braced_body;
use crate::helpers::comments::{
    format_leading_comments, format_token_sequence, format_trailing_comments,
    tokens_end_with_forced_line, tokens_have_comments,
};
use crate::helpers::modifiers::{modifier_prefix, modifier_prefix_from_parts};
use crate::rules::annotations::format_annotation_element_value;
use crate::rules::expressions::format_argument_list;
use crate::rules::names::format_name;
use crate::rules::statements::{format_block, format_block_items};
use crate::rules::types::{format_array_dimensions, format_type, format_type_parameter_list};
use crate::rules::variables::{
    format_field_declaration, format_formal_parameter, format_record_component,
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

pub(crate) fn format_anonymous_class_body(body: &ClassBody) -> Doc {
    let members = body.members().collect::<Vec<_>>();
    braced_body(format_class_body(&members))
}

fn format_class_declaration(class: &ClassDeclaration) -> Doc {
    let members = class
        .body()
        .map(|body| body.members().collect::<Vec<_>>())
        .unwrap_or_default();

    format_type_with_body(
        class.modifiers(),
        concat([
            text("class "),
            class
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            format_type_parameter_list(class.type_parameters()),
            format_extends_clause(class.extends_clause()),
            format_implements_clause(class.implements_clause()),
            format_permits_clause(class.permits_clause()),
        ]),
        &members,
    )
}

fn format_interface_declaration(interface: &InterfaceDeclaration) -> Doc {
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
            interface
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            format_type_parameter_list(interface.type_parameters()),
            format_extends_clause(interface.extends_clause()),
            format_permits_clause(interface.permits_clause()),
        ]),
        &members,
    )
}

fn format_record_declaration(record: &RecordDeclaration) -> Doc {
    let members = record
        .body()
        .map(|body| body.members().collect::<Vec<_>>())
        .unwrap_or_default();

    format_type_with_body(
        record.modifiers(),
        group(concat([
            text("record "),
            record
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            format_type_parameter_list(record.type_parameters()),
            format_record_components(record.components()),
            format_implements_clause(record.implements_clause()),
        ])),
        &members,
    )
}

fn format_enum_declaration(enum_: &EnumDeclaration) -> Doc {
    let constants = enum_
        .body()
        .and_then(|body| body.constants())
        .map(|constants| {
            constants
                .constants()
                .map(|constant| format_enum_constant(&constant))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let members = enum_
        .body()
        .map(|body| body.members().collect::<Vec<_>>())
        .unwrap_or_default();
    let body_doc = format_enum_body_contents(constants, &members);

    format_header_with_body(
        enum_.modifiers(),
        concat([
            text("enum "),
            enum_
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            format_implements_clause(enum_.implements_clause()),
        ]),
        body_doc,
    )
}

fn format_annotation_interface_declaration(annotation: &AnnotationInterfaceDeclaration) -> Doc {
    format_header_with_body(
        annotation.modifiers(),
        concat([
            text("@interface "),
            annotation
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        ]),
        annotation.body().and_then(|body| {
            let members = body
                .members()
                .filter(|member| member.kind() != JavaSyntaxKind::EmptyDeclaration)
                .map(|member| FormattedMember::from_annotation_member(&member))
                .collect::<Vec<_>>();
            (!members.is_empty()).then(|| join_member_docs(members))
        }),
    )
}

fn format_type_with_body(
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    members: &[ClassBodyMember],
) -> Doc {
    format_header_with_body(modifiers, header_tail, format_class_body(members))
}

fn format_class_body(members: &[ClassBodyMember]) -> Option<Doc> {
    let effective_members = effective_members(members);
    (!effective_members.is_empty()).then(|| {
        join_member_docs(
            effective_members
                .into_iter()
                .map(|member| FormattedMember::from_member(&member))
                .collect(),
        )
    })
}

fn format_header_with_body(
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    body: Option<Doc>,
) -> Doc {
    concat([
        modifier_prefix(modifiers),
        header_tail,
        text(" "),
        braced_body(body),
    ])
}

fn format_enum_body_contents(constants: Vec<Doc>, members: &[ClassBodyMember]) -> Option<Doc> {
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
                    concat([constant, text(separator)])
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
        (None, Some(members)) => Some(concat([text(";"), jolt_fmt_ir::empty_line(), members])),
        (None, None) => None,
    }
}

fn format_enum_constant(constant: &EnumConstant) -> Doc {
    let tokens = constant.tokens();
    let Some(name) = constant.name() else {
        return format_token_sequence(&tokens);
    };

    concat([
        modifier_prefix_from_parts(constant.annotations().collect(), Vec::new()),
        format_leading_comments(&name),
        text(name.text().to_owned()),
        format_trailing_comments(&name),
        constant
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_argument_list(Some(arguments))
            }),
        constant.body().map_or_else(jolt_fmt_ir::nil, |body| {
            let members = body.members().collect::<Vec<_>>();
            concat([text(" "), braced_body(format_class_body(&members))])
        }),
    ])
}

fn effective_members(members: &[ClassBodyMember]) -> Vec<ClassBodyMember> {
    members
        .iter()
        .filter(|member| member.kind() != JavaSyntaxKind::EmptyDeclaration)
        .cloned()
        .collect()
}

fn format_record_components(components: Option<jolt_java_syntax::RecordComponentList>) -> Doc {
    let Some(components) = components else {
        return text("()");
    };
    let tokens = components.tokens();
    if tokens_have_comments(&tokens) {
        return concat([text("("), format_token_sequence(&tokens), text(")")]);
    }
    parenthesized_comma_list(
        components
            .components()
            .map(|component| format_record_component(&component))
            .collect(),
    )
}

fn format_extends_clause(clause: Option<ExtendsClause>) -> Doc {
    format_type_clause(
        "extends",
        clause.map(|clause| {
            clause
                .types()
                .map(|ty| format_type(&ty))
                .collect::<Vec<_>>()
        }),
    )
}

fn format_implements_clause(clause: Option<ImplementsClause>) -> Doc {
    format_type_clause(
        "implements",
        clause.map(|clause| {
            clause
                .types()
                .map(|ty| format_type(&ty))
                .collect::<Vec<_>>()
        }),
    )
}

fn format_permits_clause(clause: Option<PermitsClause>) -> Doc {
    format_type_clause(
        "permits",
        clause.map(|clause| {
            clause
                .names()
                .map(|name| format_name(&name))
                .collect::<Vec<_>>()
        }),
    )
}

fn format_type_clause(keyword: &'static str, items: Option<Vec<Doc>>) -> Doc {
    let Some(items) = items else {
        return jolt_fmt_ir::nil();
    };
    if items.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        text(" "),
        text(keyword),
        text(" "),
        jolt_fmt_ir::join(text(", "), items),
    ])
}

fn join_member_docs(members: Vec<FormattedMember>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_category = None;

    for member in members {
        if !joined.is_empty() {
            if member.starts_after_blank_line
                || previous_category != Some(MemberCategory::Field)
                || member.category != MemberCategory::Field
            {
                joined.push(jolt_fmt_ir::empty_line());
            } else {
                joined.push(hard_line());
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
    starts_after_blank_line: bool,
    doc: Doc,
}

impl FormattedMember {
    fn from_member(member: &ClassBodyMember) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            ClassBodyMember::FieldDeclaration(field) => Self {
                category: MemberCategory::Field,
                starts_after_blank_line,
                doc: format_field_declaration(field),
            },
            ClassBodyMember::ConstructorDeclaration(constructor) => Self {
                category: MemberCategory::Constructor,
                starts_after_blank_line,
                doc: format_constructor_declaration(constructor),
            },
            ClassBodyMember::CompactConstructorDeclaration(constructor) => Self {
                category: MemberCategory::Constructor,
                starts_after_blank_line,
                doc: format_compact_constructor_declaration(constructor),
            },
            ClassBodyMember::MethodDeclaration(method) => Self {
                category: MemberCategory::Method,
                starts_after_blank_line,
                doc: format_method_declaration(method),
            },
            ClassBodyMember::StaticInitializer(member) => Self {
                category: MemberCategory::Initializer,
                starts_after_blank_line,
                doc: concat([
                    text("static "),
                    member
                        .body()
                        .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body)),
                ]),
            },
            ClassBodyMember::InstanceInitializer(member) => Self {
                category: MemberCategory::Initializer,
                starts_after_blank_line,
                doc: member
                    .body()
                    .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body)),
            },
            ClassBodyMember::ClassDeclaration(class) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_class_declaration(class),
            },
            ClassBodyMember::RecordDeclaration(record) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_record_declaration(record),
            },
            ClassBodyMember::EnumDeclaration(enum_) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_enum_declaration(enum_),
            },
            ClassBodyMember::InterfaceDeclaration(interface) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_interface_declaration(interface),
            },
            ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation),
            },
            ClassBodyMember::EmptyDeclaration(_) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: jolt_fmt_ir::nil(),
            },
        }
    }

    fn from_annotation_member(member: &AnnotationInterfaceBodyMember) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            AnnotationInterfaceBodyMember::FieldDeclaration(field) => Self {
                category: MemberCategory::Field,
                starts_after_blank_line,
                doc: format_field_declaration(field),
            },
            AnnotationInterfaceBodyMember::MethodDeclaration(method) => Self {
                category: MemberCategory::Method,
                starts_after_blank_line,
                doc: format_method_declaration(method),
            },
            AnnotationInterfaceBodyMember::AnnotationElementDeclaration(member) => Self {
                category: MemberCategory::Method,
                starts_after_blank_line,
                doc: format_annotation_element_declaration(member),
            },
            AnnotationInterfaceBodyMember::ClassDeclaration(class) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_class_declaration(class),
            },
            AnnotationInterfaceBodyMember::RecordDeclaration(record) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_record_declaration(record),
            },
            AnnotationInterfaceBodyMember::EnumDeclaration(enum_) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_enum_declaration(enum_),
            },
            AnnotationInterfaceBodyMember::InterfaceDeclaration(interface) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_interface_declaration(interface),
            },
            AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation),
            },
            AnnotationInterfaceBodyMember::EmptyDeclaration(_) => Self {
                category: MemberCategory::Type,
                starts_after_blank_line,
                doc: jolt_fmt_ir::nil(),
            },
        }
    }
}

fn format_constructor_declaration(constructor: &jolt_java_syntax::ConstructorDeclaration) -> Doc {
    let Some(name) = constructor.name() else {
        return format_token_sequence(&constructor.tokens());
    };
    let header_tokens = constructor.header_tokens();
    if tokens_have_comments(&header_tokens) {
        return concat([
            group(format_token_sequence(&header_tokens)),
            format_constructor_body_after_header(
                constructor.body(),
                tokens_end_with_forced_line(&header_tokens),
            ),
        ]);
    }
    concat([
        group(concat([
            modifier_prefix(constructor.modifiers()),
            format_type_parameter_list(constructor.type_parameters()),
            text(name.text().to_owned()),
            format_parameters(constructor.parameters()),
            format_throws_clause(constructor.throws_clause()),
        ])),
        format_constructor_body(constructor.body()),
    ])
}

fn format_compact_constructor_declaration(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration,
) -> Doc {
    concat([
        group(concat([
            modifier_prefix(constructor.modifiers()),
            constructor
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        ])),
        format_constructor_body(constructor.body()),
    ])
}

fn format_method_declaration(method: &MethodDeclaration) -> Doc {
    let Some(name) = method.name() else {
        return format_token_sequence(&method.tokens());
    };
    let header_tokens = method.header_tokens();
    if tokens_have_comments(&header_tokens) {
        return concat([
            group(format_token_sequence(&header_tokens)),
            format_method_body_after_header(
                method.body(),
                tokens_end_with_forced_line(&header_tokens),
            ),
        ]);
    }
    concat([
        group(concat([
            modifier_prefix(method.modifiers()),
            format_type_parameter_list(method.type_parameters()),
            method
                .return_type()
                .map_or_else(jolt_fmt_ir::nil, |return_type| {
                    concat([format_type(&return_type), text(" ")])
                }),
            text(name.text().to_owned()),
            format_parameters(method.parameters()),
            format_throws_clause(method.throws_clause()),
        ])),
        format_method_body(method.body()),
    ])
}

fn format_annotation_element_declaration(element: &AnnotationElementDeclaration) -> Doc {
    let Some(name) = element.name() else {
        return format_token_sequence(&element.tokens());
    };

    concat([
        group(concat([
            modifier_prefix(element.modifiers()),
            element
                .ty()
                .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
            text(" "),
            text(name.text().to_owned()),
            text("()"),
            element
                .dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions)
                }),
            format_annotation_element_default(element.default_value()),
        ])),
        text(";"),
    ])
}

fn format_annotation_element_default(default: Option<jolt_java_syntax::DefaultValue>) -> Doc {
    default.map_or_else(jolt_fmt_ir::nil, |default| {
        concat([
            line(),
            text("default "),
            default.value().map_or_else(jolt_fmt_ir::nil, |value| {
                format_annotation_element_value(&value)
            }),
        ])
    })
}

fn format_parameters(parameters: Option<FormalParameterList>) -> Doc {
    let Some(parameters) = parameters else {
        return text("()");
    };
    parenthesized_comma_list(
        parameters
            .parameters()
            .map(|parameter| format_formal_parameter(&parameter))
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
        .map(|exception| format_type(&exception))
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

fn format_constructor_body(body: Option<jolt_java_syntax::ConstructorBody>) -> Doc {
    format_constructor_body_after_header(body, false)
}

fn format_constructor_body_after_header(
    body: Option<jolt_java_syntax::ConstructorBody>,
    header_ends_with_line: bool,
) -> Doc {
    body.map_or_else(
        || text(";"),
        |body| {
            concat([
                if header_ends_with_line {
                    jolt_fmt_ir::nil()
                } else {
                    text(" ")
                },
                format_block_items(body.items()),
            ])
        },
    )
}

fn format_method_body(body: Option<jolt_java_syntax::Block>) -> Doc {
    format_method_body_after_header(body, false)
}

fn format_method_body_after_header(
    body: Option<jolt_java_syntax::Block>,
    header_ends_with_line: bool,
) -> Doc {
    body.map_or_else(
        || text(";"),
        |body| {
            concat([
                if header_ends_with_line {
                    jolt_fmt_ir::nil()
                } else {
                    text(" ")
                },
                format_block(&body),
            ])
        },
    )
}
