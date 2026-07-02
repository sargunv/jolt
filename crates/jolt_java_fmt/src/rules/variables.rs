use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, text};
use jolt_java_syntax::{
    FieldDeclaration, FormalParameter, LocalVariableDeclaration, ReceiverParameter,
    RecordComponent, VariableDeclarator, VariableDeclaratorEntry, VariableDeclaratorList,
    VariableInitializer,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::{
    comment_forces_line, format_construct_leading_comments, format_leading_comments,
    format_token_text, format_token_with_comments, format_trailing_comments,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::rules::annotations::format_annotation;
use crate::rules::expressions::format_variable_initializer_value;
use crate::rules::modifiers::{
    format_typed_modifier_prefix, format_typed_modifier_prefix_from_token_split_parts,
};
use crate::rules::statements::format_statement_semicolon;
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_without_leading_comments,
};

pub(crate) fn format_field_declaration(
    field: &FieldDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let modifiers = format_typed_modifier_prefix(field.modifiers(), formatter);
    let ty = field
        .ty()
        .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter));

    if let Some(declarators) = field.declarators()
        && let Some(declarator) = single_declarator(&declarators)
    {
        return concat([
            format_single_variable_declaration(
                concat([modifiers.declaration_prefix, modifiers.type_use_prefix, ty]),
                &declarator,
                formatter,
            ),
            format_statement_semicolon(field.semicolon()),
        ]);
    }

    concat([
        modifiers.declaration_prefix,
        modifiers.type_use_prefix,
        ty,
        text(" "),
        field
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                format_variable_declarator_list(&declarators, formatter)
            }),
        format_statement_semicolon(field.semicolon()),
    ])
}

pub(crate) fn format_local_variable_declaration(
    declaration: &LocalVariableDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let modifiers = format_typed_modifier_prefix_from_token_split_parts(
        declaration.declaration_annotations().collect(),
        declaration.type_use_annotations_after_modifiers().collect(),
        declaration.modifier_tokens().collect(),
        formatter,
    );
    let ty = local_variable_type(declaration, formatter);

    if let Some(declarators) = declaration.declarators()
        && let Some(declarator) = single_declarator(&declarators)
    {
        return format_single_variable_declaration(
            concat([modifiers.declaration_prefix, modifiers.type_use_prefix, ty]),
            &declarator,
            formatter,
        );
    }

    concat([
        modifiers.declaration_prefix,
        modifiers.type_use_prefix,
        ty,
        text(" "),
        declaration
            .declarators()
            .map_or_else(jolt_fmt_ir::nil, |declarators| {
                format_variable_declarator_list(&declarators, formatter)
            }),
    ])
}

pub(crate) fn format_formal_parameter(
    parameter: &FormalParameter,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(formatter.comments(), &parameter.tokens()),
            inline_modifier_prefix_from_docs(
                parameter
                    .prefix_annotations()
                    .map(|annotation| format_annotation(&annotation, formatter))
                    .collect(),
                parameter.modifier_tokens().collect(),
            ),
        ]),
        parameter.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty, formatter)
        }),
        parameter
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation, formatter))
            .collect(),
        parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        parameter
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        parameter.is_variable_arity(),
    )
}

pub(crate) fn format_record_component(
    component: &RecordComponent,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(formatter.comments(), &component.tokens()),
            inline_modifier_prefix_from_docs(
                component
                    .prefix_annotations()
                    .map(|annotation| format_annotation(&annotation, formatter))
                    .collect(),
                component.modifier_tokens().collect(),
            ),
        ]),
        component.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty, formatter)
        }),
        component
            .varargs_annotations()
            .map(|annotation| format_annotation(&annotation, formatter))
            .collect(),
        component
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        component
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        component.is_variable_arity(),
    )
}

pub(crate) fn format_receiver_parameter(
    parameter: &ReceiverParameter,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_construct_leading_comments(formatter.comments(), &parameter.tokens()),
        inline_modifier_prefix_from_docs(
            parameter
                .annotations()
                .map(|annotation| format_annotation(&annotation, formatter))
                .collect(),
            Vec::new(),
        ),
        parameter.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
            format_type_without_leading_comments(&ty, formatter)
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

fn format_named_typed_declaration(
    modifiers: Doc,
    ty: Doc,
    varargs_annotations: Vec<Doc>,
    name: Doc,
    dimensions: Doc,
    is_variable_arity: bool,
) -> Doc {
    let has_varargs_annotations = !varargs_annotations.is_empty();
    let name = concat([
        if is_variable_arity {
            if has_varargs_annotations {
                concat([
                    inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                    text("..."),
                ])
            } else {
                text("...")
            }
        } else {
            jolt_fmt_ir::nil()
        },
        if is_variable_arity {
            text(" ")
        } else {
            jolt_fmt_ir::nil()
        },
        name,
        dimensions,
    ]);
    let type_name_separator = if is_variable_arity && !has_varargs_annotations {
        jolt_fmt_ir::nil()
    } else {
        text(" ")
    };

    concat([modifiers, ty, type_name_separator, name])
}

fn local_variable_type(
    declaration: &LocalVariableDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    declaration.ty().map_or_else(
        || {
            declaration
                .var_token()
                .map_or_else(jolt_fmt_ir::nil, |token| format_token_text(token.text()))
        },
        |ty| format_type(&ty, formatter),
    )
}

fn format_variable_declarator_list(
    declarators: &VariableDeclaratorList,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat(
        declarators
            .entries()
            .map(|entry| format_variable_declarator_entry(entry, formatter))
            .collect::<Vec<_>>(),
    ))
}

fn single_declarator(declarators: &VariableDeclaratorList) -> Option<VariableDeclarator> {
    let mut entries = declarators.entries();
    let entry = entries.next()?;
    if entries.next().is_some() || entry.comma.is_some() {
        return None;
    }

    Some(entry.declarator)
}

fn format_single_variable_declaration(
    typed_prefix: Doc,
    declarator: &VariableDeclarator,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let name = format_variable_declarator_name_and_dimensions(declarator, formatter);
    let Some(initializer) = declarator.initializer() else {
        return concat([typed_prefix, text(" "), name]);
    };

    concat([
        typed_prefix,
        text(" "),
        group(concat([
            name,
            format_variable_initializer_split(&initializer, formatter),
        ])),
    ])
}

fn format_variable_declarator_entry(
    entry: VariableDeclaratorEntry,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_variable_declarator(&entry.declarator, formatter),
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

fn format_variable_declarator(
    declarator: &VariableDeclarator,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        format_variable_declarator_name_and_dimensions(declarator, formatter),
        declarator
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                format_variable_initializer_split(&initializer, formatter)
            }),
    ]))
}

fn format_variable_declarator_name_and_dimensions(
    declarator: &VariableDeclarator,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        declarator
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        declarator
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
    ])
}

fn format_variable_initializer_split(
    initializer: &VariableInitializer,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(value) = initializer.value() else {
        return text(" =");
    };

    concat([
        text(" "),
        format_variable_initializer_operator(initializer),
        indent(concat([
            format_variable_initializer_value_separator(initializer, &value),
            format_variable_initializer_value(value, formatter),
        ])),
    ])
}

fn format_variable_initializer_operator(initializer: &VariableInitializer) -> Doc {
    initializer.operator().map_or_else(
        || text("="),
        |operator| format_token_with_comments(&operator),
    )
}

fn format_variable_initializer_value_separator(
    initializer: &VariableInitializer,
    value: &jolt_java_syntax::VariableInitializerValue,
) -> Doc {
    if initializer_operator_trailing_comments_force_line(initializer) {
        jolt_fmt_ir::nil()
    } else if initializer_value_has_leading_comments(value) {
        hard_line()
    } else {
        line()
    }
}

fn initializer_operator_trailing_comments_force_line(initializer: &VariableInitializer) -> bool {
    initializer
        .operator()
        .is_some_and(|operator| operator.trailing_comments().iter().any(comment_forces_line))
}

fn initializer_value_has_leading_comments(
    value: &jolt_java_syntax::VariableInitializerValue,
) -> bool {
    value
        .tokens()
        .first()
        .is_some_and(|token| !token.leading_comments().is_empty())
}
