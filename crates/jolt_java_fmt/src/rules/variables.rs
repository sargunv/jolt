use jolt_fmt_ir::{Doc, concat, group, indent, line, text};
use jolt_java_syntax::{
    FieldDeclaration, FormalParameter, LocalVariableDeclaration, ReceiverParameter,
    RecordComponent, VariableDeclarator, VariableDeclaratorEntry, VariableDeclaratorList,
    VariableInitializer,
};

use crate::helpers::comments::{
    format_leading_comments, format_token_text, format_token_with_comments,
    format_trailing_comments,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_variable_initializer_value;
use crate::rules::modifiers::{
    format_typed_modifier_prefix, format_typed_modifier_prefix_from_parts,
};
use crate::rules::statements::format_statement_semicolon;
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_without_leading_comments,
};

pub(crate) fn format_field_declaration(field: &FieldDeclaration) -> Doc {
    let modifiers = format_typed_modifier_prefix(field.modifiers());

    concat([
        modifiers.declaration_prefix,
        modifiers.type_use_prefix,
        field
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        text(" "),
        field
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                format_variable_declarator_list(&declarators)
            }),
        format_statement_semicolon(field.semicolon()),
    ])
}

pub(crate) fn format_local_variable_declaration(declaration: &LocalVariableDeclaration) -> Doc {
    let modifiers = format_typed_modifier_prefix_from_parts(
        declaration.annotations().collect(),
        declaration.modifier_tokens().collect(),
    );

    concat([
        modifiers.declaration_prefix,
        modifiers.type_use_prefix,
        local_variable_type(declaration),
        text(" "),
        declaration
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                format_variable_declarator_list(&declarators)
            }),
    ])
}

pub(crate) fn format_formal_parameter(parameter: &FormalParameter) -> Doc {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(&parameter.tokens()),
            inline_modifier_prefix_from_docs(
                parameter
                    .prefix_annotations()
                    .map(|annotation| format_annotation(&annotation))
                    .collect(),
                parameter.modifier_tokens().collect(),
            ),
        ]),
        parameter.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty)
        }),
        parameter
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation))
            .collect(),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        parameter
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        parameter.is_variable_arity(),
    )
}

pub(crate) fn format_record_component(component: &RecordComponent) -> Doc {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(&component.tokens()),
            inline_modifier_prefix_from_docs(
                component
                    .prefix_annotations()
                    .map(|annotation| format_annotation(&annotation))
                    .collect(),
                component.modifier_tokens().collect(),
            ),
        ]),
        component.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty)
        }),
        component
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation))
            .collect(),
        component
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        component
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        component.is_variable_arity(),
    )
}

pub(crate) fn format_receiver_parameter(parameter: &ReceiverParameter) -> Doc {
    concat([
        format_construct_leading_comments(&parameter.tokens()),
        inline_modifier_prefix_from_docs(
            parameter
                .annotations()
                .map(|annotation| format_annotation(&annotation))
                .collect(),
            Vec::new(),
        ),
        parameter.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty)
        }),
        text(" "),
        parameter
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([
                    format_token_with_comments(&qualifier),
                    parameter.dot().map_or_else(
                        || text("."),
                        |dot| {
                            concat([
                                format_leading_comments(&dot),
                                text("."),
                                format_trailing_comments(&dot),
                            ])
                        },
                    ),
                ])
            }),
        parameter.this_token().map_or_else(
            || text("this"),
            |this_token| format_token_with_comments(&this_token),
        ),
    ])
}

fn format_construct_leading_comments(tokens: &[jolt_java_syntax::JavaSyntaxToken]) -> Doc {
    tokens.first().map_or_else(
        jolt_fmt_ir::nil,
        crate::helpers::comments::format_leading_comments,
    )
}

fn format_named_typed_declaration(
    modifiers: Doc,
    ty: Doc,
    varargs_annotations: Vec<Doc>,
    name: Doc,
    dimensions: Doc,
    is_variable_arity: bool,
) -> Doc {
    concat([
        modifiers,
        ty,
        if is_variable_arity {
            if varargs_annotations.is_empty() {
                text("... ")
            } else {
                concat([
                    text(" "),
                    inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                    text("... "),
                ])
            }
        } else {
            text(" ")
        },
        name,
        dimensions,
    ])
}

fn local_variable_type(declaration: &LocalVariableDeclaration) -> Doc {
    declaration.ty().map_or_else(
        || {
            declaration
                .var_token()
                .map_or_else(jolt_fmt_ir::nil, |token| format_token_text(token.text()))
        },
        |ty| format_type(&ty),
    )
}

fn format_variable_declarator_list(declarators: &VariableDeclaratorList) -> Doc {
    group(concat(
        declarators
            .entries()
            .map(format_variable_declarator_entry)
            .collect::<Vec<_>>(),
    ))
}

fn format_variable_declarator_entry(entry: VariableDeclaratorEntry) -> Doc {
    concat([
        format_variable_declarator(&entry.declarator),
        entry.comma.map_or_else(jolt_fmt_ir::nil, |comma| {
            concat([
                format_leading_comments(&comma),
                text(","),
                format_trailing_comments(&comma),
                line(),
            ])
        }),
    ])
}

fn format_variable_declarator(declarator: &VariableDeclarator) -> Doc {
    group(concat([
        declarator
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        declarator
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        declarator
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                format_variable_initializer(&initializer)
            }),
    ]))
}

fn format_variable_initializer(initializer: &VariableInitializer) -> Doc {
    let Some(value) = initializer.value() else {
        return text(" =");
    };

    match value {
        jolt_java_syntax::VariableInitializerValue::ArrayInitializer(initializer) => concat([
            text(" = "),
            format_variable_initializer_value(
                jolt_java_syntax::VariableInitializerValue::ArrayInitializer(initializer),
            ),
        ]),
        value => concat([
            text(" ="),
            indent(concat([line(), format_variable_initializer_value(value)])),
        ]),
    }
}
