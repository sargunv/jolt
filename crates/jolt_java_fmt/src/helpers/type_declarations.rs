use jolt_fmt_ir::{Doc, concat, hard_line, join, text};

use crate::layout as wrap;

pub(crate) struct TypeDeclaration {
    pub(crate) modifiers: Vec<Doc>,
    pub(crate) keyword: Doc,
    pub(crate) before_name_comments: Vec<Doc>,
    pub(crate) name: Doc,
    pub(crate) type_parameters: Option<Doc>,
    pub(crate) record_components: Option<Doc>,
    pub(crate) extends_clause: Option<Doc>,
    pub(crate) implements_clause: Option<Doc>,
    pub(crate) permits_clause: Option<Doc>,
    pub(crate) before_body_comments: Vec<Doc>,
    pub(crate) body: Doc,
}

pub(crate) fn type_declaration(declaration: TypeDeclaration) -> Doc {
    let TypeDeclaration {
        modifiers,
        keyword,
        before_name_comments,
        name,
        type_parameters,
        record_components,
        extends_clause,
        implements_clause,
        permits_clause,
        before_body_comments,
        body,
    } = declaration;

    let mut head_parts = modifiers;
    head_parts.push(keyword);
    head_parts.extend(before_name_comments);
    head_parts.push(concat(std::iter::once(name).chain(type_parameters)));

    let mut head = space_separated_head(head_parts);
    if let Some(record_components) = record_components {
        head = concat([head, record_components]);
    }

    let mut header = vec![head];
    header.extend(extends_clause);
    header.extend(implements_clause);
    header.extend(permits_clause);
    let header = wrap::declaration_header(header);
    if before_body_comments.is_empty() {
        concat([header, text(" "), body])
    } else {
        concat([
            header,
            hard_line(),
            join(hard_line(), before_body_comments),
            hard_line(),
            body,
        ])
    }
}

fn space_separated_head(parts: impl IntoIterator<Item = Doc>) -> Doc {
    let mut docs = Vec::new();
    for part in parts {
        if !docs.is_empty() {
            docs.push(text(" "));
        }
        docs.push(part);
    }
    concat(docs)
}
