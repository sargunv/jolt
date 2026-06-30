use jolt_fmt_ir::{Doc, best_fitting, concat, group, hard_line, indent_by, join, line, text};

use crate::helpers::expressions as java_expressions;
use crate::policy::JavaFormatPolicy;

pub(crate) struct CallableHeader {
    pub(crate) modifiers: Vec<Doc>,
    pub(crate) type_parameters: Option<Doc>,
    pub(crate) leading_type: Option<Doc>,
    pub(crate) leading_type_policy: Option<DeclarationLeadingTypePolicy>,
    pub(crate) before_name_comments: Vec<Doc>,
    pub(crate) name: Doc,
    pub(crate) after_name_comments: Vec<Doc>,
    pub(crate) parameters: Option<Doc>,
    pub(crate) tail: Option<Doc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeclarationLeadingTypePolicy {
    pub(crate) has_type_arguments: bool,
    pub(crate) rendered_leading_type_source_width: usize,
    pub(crate) rendered_declaration_head_source_width: usize,
}

pub(crate) struct AnnotationElementDeclaration {
    pub(crate) result_type: Doc,
    pub(crate) name: Doc,
    pub(crate) dimensions: Option<Doc>,
    pub(crate) default_value: Option<Doc>,
}

pub(crate) enum CallableDeclarationTail {
    Block {
        header_trailing_comments: Vec<Doc>,
        before_body_comments: Vec<Doc>,
        body: Doc,
    },
    Semicolon {
        signature_tail_comments: Vec<Doc>,
    },
}

pub(crate) fn callable_header(header: CallableHeader, policy: JavaFormatPolicy) -> Doc {
    let CallableHeader {
        modifiers,
        type_parameters,
        leading_type,
        leading_type_policy,
        before_name_comments,
        name,
        after_name_comments,
        parameters,
        tail,
    } = header;

    let has_type_parameters = type_parameters.is_some();
    let mut tail = tail;
    let type_parameters = type_parameters;

    let before_name_comments_in_signature =
        leading_type.is_some() && !before_name_comments.is_empty();
    let parameters = parameters.unwrap_or_else(|| text(""));
    let name_and_parameters = if after_name_comments.is_empty() {
        concat([name, parameters])
    } else if before_name_comments_in_signature {
        concat([name, concat(after_name_comments), hard_line(), parameters])
    } else {
        concat([
            name,
            concat(after_name_comments),
            continuation_indent(policy, concat([hard_line(), parameters])),
        ])
    };
    let signature = callable_signature(
        leading_type,
        leading_type_policy
            .is_some_and(|leading_type_policy| {
                policy.declaration_leading_type_forces_name_break(
                    leading_type_policy.has_type_arguments,
                    leading_type_policy.rendered_leading_type_source_width,
                    leading_type_policy.rendered_declaration_head_source_width,
                )
            }),
        has_type_parameters,
        &before_name_comments,
        name_and_parameters,
        &mut tail,
        policy,
    );

    if before_name_comments.is_empty() || before_name_comments_in_signature {
        callable_declaration_header(modifiers, type_parameters, signature, tail, policy)
    } else {
        callable_commented_declaration_header(
            modifiers,
            type_parameters,
            before_name_comments,
            signature,
            tail,
            policy,
        )
    }
}

pub(crate) fn callable_declaration(
    header: Doc,
    tail: CallableDeclarationTail,
    policy: JavaFormatPolicy,
) -> Doc {
    match tail {
        CallableDeclarationTail::Block {
            header_trailing_comments,
            before_body_comments,
            body,
        } => callable_block_declaration(
            header,
            header_trailing_comments,
            before_body_comments,
            body,
            policy,
        ),
        CallableDeclarationTail::Semicolon {
            signature_tail_comments,
        } => callable_semicolon_declaration(header, signature_tail_comments),
    }
}

fn callable_block_declaration(
    header: Doc,
    header_trailing_comments: Vec<Doc>,
    before_body_comments: Vec<Doc>,
    body: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    if header_trailing_comments.is_empty() && before_body_comments.is_empty() {
        concat([header, text(" "), body])
    } else {
        let has_header_trailing_comments = !header_trailing_comments.is_empty();
        let mut parts = vec![header];
        parts.extend(header_trailing_comments);
        parts.push(hard_line());
        if !before_body_comments.is_empty() {
            parts.push(join(hard_line(), before_body_comments));
            parts.push(hard_line());
        }
        if has_header_trailing_comments {
            parts.push(concat([
                text(" ".repeat(policy.continuation_indent_columns())),
                body,
            ]));
        } else {
            parts.push(body);
        }
        concat(parts)
    }
}

fn callable_semicolon_declaration(header: Doc, signature_tail_comments: Vec<Doc>) -> Doc {
    if signature_tail_comments.is_empty() {
        concat([header, text(";")])
    } else {
        concat([
            header,
            text(" "),
            join(text(" "), signature_tail_comments),
            text(";"),
        ])
    }
}

pub(crate) fn annotation_element_declaration(
    declaration: AnnotationElementDeclaration,
    policy: JavaFormatPolicy,
) -> Doc {
    let AnnotationElementDeclaration {
        result_type,
        name,
        dimensions,
        default_value,
    } = declaration;

    let header = concat([
        result_type,
        text(" "),
        name,
        text("()"),
        dimensions.unwrap_or_else(|| text("")),
    ]);
    let declaration = if let Some(default_value) = default_value {
        java_expressions::simple_assignment_expression(
            header,
            text("default"),
            default_value,
            policy.continuation_indent_levels(),
        )
    } else {
        header
    };

    concat([declaration, text(";")])
}

fn callable_signature(
    leading_type: Option<Doc>,
    break_after_leading_type: bool,
    has_type_parameters: bool,
    before_name_comments: &[Doc],
    name_and_parameters: Doc,
    tail: &mut Option<Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let Some(leading_type) = leading_type else {
        if has_type_parameters {
            return callable_tail(name_and_parameters, tail.take(), policy);
        }
        return name_and_parameters;
    };

    if break_after_leading_type {
        return callable_name_with_leading_type(
            leading_type,
            name_and_parameters,
            tail.take(),
            policy,
        );
    }

    if !before_name_comments.is_empty() {
        let name_and_tail = callable_tail(name_and_parameters, tail.take(), policy);
        return group(concat([
            leading_type,
            concat(before_name_comments.iter().cloned()),
            continuation_indent(policy, concat([hard_line(), name_and_tail])),
        ]));
    }

    let head = concat([leading_type, text(" "), name_and_parameters]);
    if has_type_parameters {
        callable_tail(head, tail.take(), policy)
    } else {
        head
    }
}

fn continuation_indent(policy: JavaFormatPolicy, doc: Doc) -> Doc {
    indent_by(policy.continuation_indent_levels(), doc)
}

fn callable_declaration_header(
    modifiers: Vec<Doc>,
    type_parameters: Option<Doc>,
    signature: Doc,
    tail: Option<Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let head = match type_parameters {
        Some(type_parameters) => concat([
            space_separated(modifiers.into_iter().chain([type_parameters])),
            continuation_indent(policy, concat([line(), signature])),
        ]),
        None => {
            if modifiers.is_empty() {
                signature
            } else {
                concat([space_separated(modifiers), text(" "), signature])
            }
        }
    };

    if let Some(tail) = tail {
        group(concat([
            head,
            continuation_indent(policy, concat([line(), tail])),
        ]))
    } else {
        group(head)
    }
}

fn callable_commented_declaration_header(
    modifiers: Vec<Doc>,
    type_parameters: Option<Doc>,
    before_name_comments: Vec<Doc>,
    signature: Doc,
    tail: Option<Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let mut parts = modifiers;
    parts.extend(type_parameters);
    parts.extend(before_name_comments);
    parts.push(signature);
    parts.extend(tail);

    let mut parts = parts.into_iter();
    let Some(first) = parts.next() else {
        return text("");
    };

    group(concat([
        first,
        continuation_indent(policy, concat(parts.map(|part| concat([line(), part])))),
    ]))
}

fn space_separated(parts: impl IntoIterator<Item = Doc>) -> Doc {
    let mut docs = Vec::new();
    for part in parts {
        if !docs.is_empty() {
            docs.push(text(" "));
        }
        docs.push(part);
    }
    concat(docs)
}

fn callable_name_with_leading_type(
    leading_type: Doc,
    name_and_parameters: Doc,
    tail: Option<Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let name_and_tail = callable_tail(name_and_parameters, tail, policy);
    best_fitting(
        concat([leading_type.clone(), text(" "), name_and_tail.clone()]),
        [concat([
            leading_type,
            continuation_indent(policy, concat([line(), name_and_tail])),
        ])],
    )
}

fn callable_tail(head: Doc, tail: Option<Doc>, policy: JavaFormatPolicy) -> Doc {
    if let Some(tail) = tail {
        group(concat([
            head,
            continuation_indent(policy, concat([line(), tail])),
        ]))
    } else {
        head
    }
}

pub(crate) fn variable_declaration(
    prefix: Vec<Doc>,
    declarators: Doc,
    leading_type_policy: Option<DeclarationLeadingTypePolicy>,
    policy: JavaFormatPolicy,
) -> Doc {
    group(concat([
        variable_declaration_header(prefix, declarators, leading_type_policy, policy),
        text(";"),
    ]))
}

pub(crate) fn variable_declaration_header(
    prefix: Vec<Doc>,
    declarators: Doc,
    leading_type_policy: Option<DeclarationLeadingTypePolicy>,
    policy: JavaFormatPolicy,
) -> Doc {
    let flat = concat([
        space_separated(prefix.clone()),
        text(" "),
        declarators.clone(),
    ]);
    if leading_type_policy
        .is_some_and(|leading_type_policy| {
            policy.field_leading_type_forces_name_break(
                leading_type_policy.rendered_leading_type_source_width,
                leading_type_policy.rendered_declaration_head_source_width,
            )
        })
    {
        concat([
            space_separated(prefix),
            continuation_indent(policy, concat([line(), declarators])),
        ])
    } else {
        flat
    }
}
