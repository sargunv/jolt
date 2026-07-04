use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, text};
use jolt_java_syntax::{
    FieldDeclaration, FormalParameter, LocalVariableDeclaration, ReceiverParameter,
    RecordComponent, VariableDeclarator, VariableDeclaratorEntry, VariableDeclaratorList,
    VariableInitializer,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::{
    InlineLeadingTrivia, TrailingTrivia, comment_forces_line, format_construct_leading_comments,
    format_leading_comments, format_token_text, format_token_with_comments,
    format_token_with_inline_leading_comments, format_trailing_comments,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::rules::annotations::{format_annotation, format_annotation_without_leading_comments};
use crate::rules::expressions::format_variable_initializer_value;
use crate::rules::modifiers::{
    format_typed_modifier_prefix, format_typed_modifier_prefix_from_token_split_parts,
};
use crate::rules::statements::format_statement_semicolon;
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_without_leading_comments,
};

pub(crate) fn format_field_declaration<'source>(
    field: &FieldDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let modifiers = format_typed_modifier_prefix(field.modifiers(), formatter);
    let declaration_prefix = concat([
        format_construct_leading_comments(field.first_token().as_ref()),
        modifiers.declaration_prefix,
    ]);
    let ty = field.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
        format_type_without_leading_comments(&ty, formatter)
    });

    if let Some(declarators) = field.declarators()
        && let Some(declarator) = single_declarator(&declarators)
    {
        return concat([
            format_single_variable_declaration(
                concat([declaration_prefix, modifiers.type_use_prefix, ty]),
                &declarator,
                formatter,
            ),
            format_statement_semicolon(field.semicolon()),
        ]);
    }

    concat([
        declaration_prefix,
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

pub(crate) fn format_local_variable_declaration<'source>(
    declaration: &LocalVariableDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(crate) fn format_formal_parameter<'source>(
    parameter: &FormalParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(parameter.first_token().as_ref()),
            inline_modifier_prefix_from_docs(
                format_construct_prefix_annotations(parameter.prefix_annotations(), formatter),
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
            .map_or_else(jolt_fmt_ir::nil, |name| format_name_after_type_token(&name)),
        parameter
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        parameter.ellipsis_token().as_ref(),
    )
}

pub(crate) fn format_record_component<'source>(
    component: &RecordComponent<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_named_typed_declaration(
        concat([
            format_construct_leading_comments(component.first_token().as_ref()),
            inline_modifier_prefix_from_docs(
                format_construct_prefix_annotations(component.prefix_annotations(), formatter),
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
            .map_or_else(jolt_fmt_ir::nil, |name| format_name_after_type_token(&name)),
        component
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        component.ellipsis_token().as_ref(),
    )
}

pub(crate) fn format_receiver_parameter<'source>(
    parameter: &ReceiverParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_construct_leading_comments(parameter.first_token().as_ref()),
        inline_modifier_prefix_from_docs(
            format_construct_prefix_annotations(parameter.annotations(), formatter),
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
                    parameter.dot().map_or_else(jolt_fmt_ir::nil, |dot| {
                        concat([
                            format_leading_comments(&dot),
                            format_token_text(dot.text()),
                            format_trailing_comments(&dot),
                        ])
                    }),
                ])
            }),
        parameter
            .this_token()
            .map_or_else(jolt_fmt_ir::nil, |this_token| {
                format_token_with_comments(&this_token)
            }),
    ])
}

fn format_construct_prefix_annotations<'source>(
    annotations: impl Iterator<Item = jolt_java_syntax::Annotation<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Vec<Doc<'source>> {
    annotations
        .enumerate()
        .map(|(index, annotation)| {
            if index == 0 {
                format_annotation_without_leading_comments(&annotation, formatter)
            } else {
                format_annotation(&annotation, formatter)
            }
        })
        .collect()
}

fn format_named_typed_declaration<'source>(
    modifiers: Doc<'source>,
    ty: Doc<'source>,
    varargs_annotations: Vec<Doc<'source>>,
    name: Doc<'source>,
    dimensions: Doc<'source>,
    ellipsis: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let has_varargs_annotations = !varargs_annotations.is_empty();
    let name = concat([
        if let Some(ellipsis) = ellipsis {
            if has_varargs_annotations {
                concat([
                    inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                    format_token_with_comments(ellipsis),
                ])
            } else {
                format_token_with_comments(ellipsis)
            }
        } else {
            jolt_fmt_ir::nil()
        },
        if ellipsis.is_some() {
            text(" ")
        } else {
            jolt_fmt_ir::nil()
        },
        name,
        dimensions,
    ]);
    let type_name_separator = if ellipsis.is_some() && !has_varargs_annotations {
        jolt_fmt_ir::nil()
    } else {
        text(" ")
    };

    concat([modifiers, ty, type_name_separator, name])
}

fn local_variable_type<'source>(
    declaration: &LocalVariableDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    declaration.ty().map_or_else(
        || {
            declaration
                .var_token()
                .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token))
        },
        |ty| format_type(&ty, formatter),
    )
}

fn format_variable_declarator_list<'source>(
    declarators: &VariableDeclaratorList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat(declarators.entries().map(|entry| {
        format_variable_declarator_entry(&entry, formatter)
    })))
}

fn single_declarator<'source>(
    declarators: &VariableDeclaratorList<'source>,
) -> Option<VariableDeclarator<'source>> {
    let mut entries = declarators.entries();
    let entry = entries.next()?;
    if entries.next().is_some() || entry.comma.is_some() {
        return None;
    }

    Some(entry.declarator)
}

fn format_single_variable_declaration<'source>(
    typed_prefix: Doc<'source>,
    declarator: &VariableDeclarator<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_variable_declarator_entry<'source>(
    entry: &VariableDeclaratorEntry<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_variable_declarator(&entry.declarator, formatter),
        entry.comma.map_or_else(jolt_fmt_ir::nil, |comma| {
            concat([
                format_leading_comments(&comma),
                format_token_text(comma.text()),
                format_trailing_comments(&comma),
                line(),
            ])
        }),
    ])
}

fn format_variable_declarator<'source>(
    declarator: &VariableDeclarator<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        format_variable_declarator_name_and_dimensions(declarator, formatter),
        declarator
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                format_variable_initializer_split(&initializer, formatter)
            }),
    ]))
}

fn format_variable_declarator_name_and_dimensions<'source>(
    declarator: &VariableDeclarator<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        declarator
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name_after_type_token(&name)),
        declarator
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
    ])
}

fn format_name_after_type_token<'source>(
    name: &jolt_java_syntax::JavaSyntaxToken<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        name,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::Preserve,
    )
}

fn format_variable_initializer_split<'source>(
    initializer: &VariableInitializer<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_variable_initializer_operator<'source>(
    initializer: &VariableInitializer<'source>,
) -> Doc<'source> {
    initializer
        .operator()
        .map_or_else(jolt_fmt_ir::nil, |operator| {
            format_token_with_comments(&operator)
        })
}

fn format_variable_initializer_value_separator<'source>(
    initializer: &VariableInitializer<'source>,
    value: &jolt_java_syntax::VariableInitializerValue<'source>,
) -> Doc<'source> {
    if initializer_operator_trailing_comments_force_line(initializer) {
        jolt_fmt_ir::nil()
    } else if initializer_value_has_leading_comments(value) {
        hard_line()
    } else {
        line()
    }
}

fn initializer_operator_trailing_comments_force_line(
    initializer: &VariableInitializer<'_>,
) -> bool {
    initializer.operator().is_some_and(|operator| {
        operator
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}

fn initializer_value_has_leading_comments(
    value: &jolt_java_syntax::VariableInitializerValue<'_>,
) -> bool {
    value
        .first_token()
        .is_some_and(|token| !token.leading_comments().is_empty())
}
