use jolt_fmt_ir::{Doc, concat, group, indent, line, text};
use jolt_java_syntax::{
    FieldDeclaration, FormalParameter, LocalVariableDeclaration, RecordComponent,
    VariableDeclarator, VariableDeclaratorList, VariableInitializer,
};

use crate::helpers::comments::{format_token_sequence, tokens_have_comments};
use crate::helpers::modifiers::{modifier_prefix, modifier_prefix_from_parts};
use crate::rules::expressions::format_variable_initializer_value;

pub(crate) fn format_field_declaration(field: &FieldDeclaration) -> Doc {
    concat([
        modifier_prefix(field.modifiers()),
        field
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
        text(" "),
        field
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                format_variable_declarator_list(&declarators)
            }),
        text(";"),
    ])
}

pub(crate) fn format_local_variable_declaration(declaration: &LocalVariableDeclaration) -> Doc {
    concat([
        modifier_prefix_from_parts(
            declaration.annotations().collect(),
            declaration.modifier_tokens().collect(),
        ),
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
    let tokens = parameter.tokens();
    let annotations = parameter.annotations().collect::<Vec<_>>();
    if tokens_have_comments(&tokens) || !annotations.is_empty() {
        return format_token_sequence(&tokens);
    }

    format_named_typed_declaration(
        modifier_prefix_from_parts(annotations, parameter.modifier_tokens().collect()),
        parameter
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        parameter
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_token_sequence(&dimensions.tokens())
            }),
        parameter.is_variable_arity(),
    )
}

pub(crate) fn format_record_component(component: &RecordComponent) -> Doc {
    let tokens = component.tokens();
    let annotations = component.annotations().collect::<Vec<_>>();
    if tokens_have_comments(&tokens) || !annotations.is_empty() {
        return format_token_sequence(&tokens);
    }

    format_named_typed_declaration(
        modifier_prefix_from_parts(annotations, component.modifier_tokens().collect()),
        component
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_token_sequence(&ty.tokens())),
        component
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        component
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_token_sequence(&dimensions.tokens())
            }),
        component.is_variable_arity(),
    )
}

fn format_named_typed_declaration(
    modifiers: Doc,
    ty: Doc,
    name: Doc,
    dimensions: Doc,
    is_variable_arity: bool,
) -> Doc {
    concat([
        modifiers,
        ty,
        if is_variable_arity {
            text("... ")
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
                .map_or_else(jolt_fmt_ir::nil, |token| text(token.text().to_owned()))
        },
        |ty| format_token_sequence(&ty.tokens()),
    )
}

fn format_variable_declarator_list(declarators: &VariableDeclaratorList) -> Doc {
    let tokens = declarators.tokens();
    if tokens_have_comments(&tokens) {
        return format_token_sequence(&tokens);
    }

    jolt_fmt_ir::join(
        text(", "),
        declarators
            .declarators()
            .map(|declarator| format_variable_declarator(&declarator)),
    )
}

fn format_variable_declarator(declarator: &VariableDeclarator) -> Doc {
    group(concat([
        declarator
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        declarator
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_token_sequence(&dimensions.tokens())
            }),
        declarator
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                format_variable_initializer(&initializer)
            }),
    ]))
}

fn format_variable_initializer(initializer: &VariableInitializer) -> Doc {
    concat([
        text(" ="),
        indent(concat([
            line(),
            initializer.value().map_or_else(jolt_fmt_ir::nil, |value| {
                format_variable_initializer_value(value)
            }),
        ])),
    ])
}
