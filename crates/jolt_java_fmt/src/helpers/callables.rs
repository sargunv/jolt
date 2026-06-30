use jolt_fmt_ir::{Doc, best_fitting, concat, group, hard_line, indent_by, join, line, text};

use crate::layout as wrap;

const CONTINUATION_INDENT_LEVELS: u16 = 2;

pub(crate) struct CallableHeader {
    pub(crate) modifiers: Vec<Doc>,
    pub(crate) type_parameters: Option<Doc>,
    pub(crate) leading_type: Option<Doc>,
    pub(crate) break_after_leading_type: bool,
    pub(crate) before_name_comments: Vec<Doc>,
    pub(crate) name: Doc,
    pub(crate) after_name_comments: Vec<Doc>,
    pub(crate) parameters: Option<Doc>,
    pub(crate) tail: Option<Doc>,
}

pub(crate) struct AnnotationElementDeclaration {
    pub(crate) result_type: Doc,
    pub(crate) name: Doc,
    pub(crate) dimensions: Option<Doc>,
    pub(crate) default_value: Option<Doc>,
}

pub(crate) fn callable_header(header: CallableHeader) -> Doc {
    let CallableHeader {
        modifiers,
        type_parameters,
        leading_type,
        break_after_leading_type,
        before_name_comments,
        name,
        after_name_comments,
        parameters,
        tail,
    } = header;

    let has_type_parameters = type_parameters.is_some();
    let mut tail = tail;
    let mut declaration_parts = modifiers;
    declaration_parts.extend(type_parameters);

    let name_and_parameters = if after_name_comments.is_empty() {
        concat([name, parameters.unwrap_or_else(|| text(""))])
    } else {
        concat([
            name,
            continuation_indent(concat([
                hard_line(),
                join(hard_line(), after_name_comments),
            ])),
            parameters.unwrap_or_else(|| text("")),
        ])
    };
    let signature = callable_signature(
        leading_type,
        break_after_leading_type,
        has_type_parameters,
        name_and_parameters,
        &mut tail,
    );
    declaration_parts.extend(before_name_comments);
    declaration_parts.push(signature);
    declaration_parts.extend(tail);

    wrap::declaration_header(declaration_parts)
}

pub(crate) fn annotation_element_declaration(declaration: AnnotationElementDeclaration) -> Doc {
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
        wrap::assignment_expression(header, text("default"), default_value)
    } else {
        header
    };

    concat([declaration, text(";")])
}

fn callable_signature(
    leading_type: Option<Doc>,
    break_after_leading_type: bool,
    has_type_parameters: bool,
    name_and_parameters: Doc,
    tail: &mut Option<Doc>,
) -> Doc {
    let Some(leading_type) = leading_type else {
        if has_type_parameters {
            return callable_tail(name_and_parameters, tail.take());
        }
        return name_and_parameters;
    };

    if break_after_leading_type {
        return callable_name_with_leading_type(leading_type, name_and_parameters, tail.take());
    }

    let head = concat([leading_type, text(" "), name_and_parameters]);
    if has_type_parameters {
        callable_tail(head, tail.take())
    } else {
        head
    }
}

fn continuation_indent(doc: Doc) -> Doc {
    indent_by(CONTINUATION_INDENT_LEVELS, doc)
}

fn callable_name_with_leading_type(
    leading_type: Doc,
    name_and_parameters: Doc,
    tail: Option<Doc>,
) -> Doc {
    let name_and_tail = callable_tail(name_and_parameters, tail);
    best_fitting(
        concat([leading_type.clone(), text(" "), name_and_tail.clone()]),
        [concat([
            leading_type,
            continuation_indent(concat([line(), name_and_tail])),
        ])],
    )
}

fn callable_tail(head: Doc, tail: Option<Doc>) -> Doc {
    if let Some(tail) = tail {
        group(concat([head, continuation_indent(concat([line(), tail]))]))
    } else {
        head
    }
}
