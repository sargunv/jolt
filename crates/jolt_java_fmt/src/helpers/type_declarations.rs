use jolt_fmt_ir::{
    Doc, concat, group, hard_line, hard_line_without_break_parent, if_group_breaks, indent_by,
    join, line, text,
};

use crate::helpers::lists::TYPE_DECLARATION_TYPE_PARAMETERS_GROUP_ID;
use crate::policy::JavaFormatPolicy;

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

pub(crate) fn type_declaration(declaration: TypeDeclaration, policy: JavaFormatPolicy) -> Doc {
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
    let has_type_parameters = type_parameters.is_some();
    head_parts.push(concat(std::iter::once(name).chain(type_parameters)));

    let mut head = space_separated_head(head_parts);
    if let Some(record_components) = record_components {
        head = concat([head, record_components]);
    }

    let clauses = extends_clause
        .into_iter()
        .chain(implements_clause)
        .chain(permits_clause)
        .collect::<Vec<_>>();
    let header = type_declaration_header(head, clauses, policy, has_type_parameters);
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

/// google-java-format `visitClassDeclaration`: each header clause gets an independent
/// `breakToFill(" ")` decision so a long `implements` list does not force `extends`
/// onto its own line when `class A extends S` fits.
fn type_declaration_header(
    head: Doc,
    clauses: Vec<Doc>,
    policy: JavaFormatPolicy,
    has_type_parameters: bool,
) -> Doc {
    if clauses.is_empty() {
        return group(head);
    }

    if clauses.len() == 1 {
        return group(concat([
            head,
            indent_by(
                policy.continuation_indent_levels(),
                group(concat([
                    clause_separator(has_type_parameters),
                    clauses.into_iter().next().expect("one clause"),
                ])),
            ),
        ]));
    }

    group(concat([
        head,
        indent_by(
            policy.continuation_indent_levels(),
            concat(clauses.into_iter().enumerate().map(|(index, clause)| {
                let separator = if index == 0 {
                    clause_separator(has_type_parameters)
                } else {
                    line()
                };
                group(concat([separator, clause]))
            })),
        ),
    ]))
}

fn clause_separator(has_type_parameters: bool) -> Doc {
    if has_type_parameters {
        if_group_breaks(
            TYPE_DECLARATION_TYPE_PARAMETERS_GROUP_ID,
            hard_line_without_break_parent(),
            line(),
        )
    } else {
        line()
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
