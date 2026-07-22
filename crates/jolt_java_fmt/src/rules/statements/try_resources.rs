use super::control_flow::{format_condition_open_paren, format_statement_header_body_separator};
use super::simple::format_statement_keyword;
use super::{
    CatchClause, CatchParameter, Doc, FinallyClause, JavaSyntaxToken, LeadingTrivia, Resource,
    ResourceList, TrailingTrivia, TryStatement, TryWithResourcesStatement, format_annotation,
    format_block, format_dangling_comments, format_expression, format_removed_comments,
    format_resource_variable_declaration, format_token, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type, trailing_comments_force_line,
};
use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, format_malformed, resolve_list_part,
    resolve_optional_field, resolve_required_delimiter, resolve_required_field,
};
use crate::rules::types::format_array_dimensions;
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    JavaSyntaxListPart, JavaSyntaxView, PartitionedModifierItem, ResourceValueSyntax,
    VariableAccessSyntax,
};

pub(super) fn format_try_statement<'source>(
    statement: &TryStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = match resolve_required_field(statement.body(), doc) {
        JavaFormatField::Present(body) => format_block(&body, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let catches = match resolve_required_field(statement.catches(), doc) {
        JavaFormatField::Present(catches) => format_catch_clauses(catches.parts(), doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let finally = match resolve_optional_field(statement.finally(), doc) {
        JavaFormatField::Present(Some(finally)) => format_finally_clause(&finally, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.try_keyword(), doc),
            doc.space(),
            body,
            catches,
            finally,
        ]
    )
}

pub(super) fn format_try_with_resources_statement<'source>(
    statement: &TryWithResourcesStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (resources_doc, separator) = match resolve_required_field(statement.resources(), doc) {
        JavaFormatField::Present(resources) => {
            let close = resolve_required_delimiter(resources.close_paren(), doc);
            let separator = format_statement_header_body_separator(close.source(), doc);
            (
                format_resource_specification(&resources, close, doc),
                separator,
            )
        }
        JavaFormatField::Malformed(malformed) => (malformed, doc.space()),
    };
    let body = match resolve_required_field(statement.body(), doc) {
        JavaFormatField::Present(body) => format_block(&body, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let catches = match resolve_required_field(statement.catches(), doc) {
        JavaFormatField::Present(catches) => format_catch_clauses(catches.parts(), doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let finally = match resolve_optional_field(statement.finally(), doc) {
        JavaFormatField::Present(Some(finally)) => format_finally_clause(&finally, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.try_keyword(), doc),
            doc.space(),
            resources_doc,
            separator,
            body,
            catches,
            finally,
        ]
    )
}

fn format_resource_specification<'source>(
    specification: &jolt_java_syntax::ResourceSpecification<'source>,
    close: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(specification.open_paren(), doc);
    let resources = match resolve_required_field(specification.resources(), doc) {
        JavaFormatField::Present(resources) => resources,
        JavaFormatField::Malformed(malformed) => {
            return doc_concat!(
                doc,
                [
                    format_condition_open_paren(open, doc),
                    malformed,
                    format_resource_close_paren(close, doc)
                ]
            );
        }
    };
    let (trailing, trailing_removal) =
        match resolve_optional_field(specification.trailing_semicolon(), doc) {
            JavaFormatField::Present(Some(separator)) => {
                if let Some(claim) = specification.trailing_separator_removal_claim() {
                    let removed = doc.removed_source(claim);
                    match format_removed_resource_separator_comments(&separator, doc) {
                        Some(comments) => (
                            Some((doc_concat!(doc, [removed, comments]), true)),
                            Doc::nil(),
                        ),
                        None => (None, removed),
                    }
                } else {
                    (
                        Some((
                            format_token(
                                doc,
                                &separator,
                                LeadingTrivia::Preserve,
                                TrailingTrivia::BeforeLineBreak,
                            ),
                            false,
                        )),
                        Doc::nil(),
                    )
                }
            }
            JavaFormatField::Present(None) => (None, Doc::nil()),
            JavaFormatField::Malformed(malformed) => (Some((malformed, true)), Doc::nil()),
        };
    let resources = format_resource_lines(&resources, trailing, doc);
    let open_source = open.source().copied();
    #[allow(clippy::single_match_else)]
    match resources {
        Some(resources) => {
            let spacing = format_resource_open_spacing(open_source.as_ref(), doc);
            doc_concat!(
                doc,
                [
                    format_condition_open_paren(open, doc),
                    doc_indent!(
                        doc,
                        doc_concat!(doc, [spacing, resources, trailing_removal])
                    ),
                    format_resource_close_paren(close, doc),
                ]
            )
        }
        None => doc_concat!(
            doc,
            [
                format_condition_open_paren(open, doc),
                format_resource_close_paren(close, doc)
            ]
        ),
    }
}

fn format_resource_open_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match open {
        Some(open) if open.trailing_comments().next().is_some() => doc_concat!(
            doc,
            [
                format_trailing_comments_before_line_break(doc, open),
                doc.hard_line()
            ]
        ),
        _ => doc.hard_line(),
    }
}

fn format_resource_close_paren<'source>(
    close: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match close {
        JavaFormatDelimiter::Recovery(recovery) => recovery,
        JavaFormatDelimiter::Source(close) => {
            let leading = close.leading_comments();
            doc_concat!(
                doc,
                [
                    if leading.is_empty() {
                        doc.hard_line()
                    } else {
                        let comments = doc_concat!(
                            doc,
                            [doc.hard_line(), format_dangling_comments(doc, leading)]
                        );
                        doc_concat!(doc, [doc_indent!(doc, comments), doc.hard_line()])
                    },
                    format_token(
                        doc,
                        &close,
                        LeadingTrivia::SuppressAlreadyHandled,
                        TrailingTrivia::BeforeLineBreak
                    ),
                    if trailing_comments_force_line(&close) {
                        doc.hard_line()
                    } else {
                        Doc::nil()
                    },
                ]
            )
        }
    }
}

fn format_resource_lines<'source>(
    list: &ResourceList<'source>,
    trailing: Option<(Doc<'source>, bool)>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut parts = list.parts().peekable();
    if parts.peek().is_none() {
        return trailing.map(|(trailing, _)| trailing);
    }
    let mut trailing = trailing;
    Some(doc.concat_list(|joined| {
        let mut needs_line = false;
        for part in parts {
            match resolve_list_part(part, joined) {
                JavaFormatListPart::Item(resource) => {
                    if needs_line {
                        let line = joined.hard_line();
                        joined.push(line);
                    }
                    let resource = format_resource(&resource, joined);
                    joined.push(resource);
                    needs_line = false;
                }
                JavaFormatListPart::Separator(separator) => {
                    let separator = format_token(
                        joined,
                        &separator,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::BeforeLineBreak,
                    );
                    joined.push(separator);
                    needs_line = true;
                }
                JavaFormatListPart::Malformed(malformed) => {
                    if needs_line {
                        let line = joined.hard_line();
                        joined.push(line);
                    }
                    joined.push(malformed);
                    needs_line = true;
                }
            }
        }
        if let Some((trailing, starts_line)) = trailing.take() {
            if starts_line {
                let line = joined.hard_line();
                joined.push(line);
            }
            joined.push(trailing);
        }
    }))
}

fn format_resource<'source>(
    resource: &Resource<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match resolve_required_field(resource.value(), doc) {
        JavaFormatField::Present(value) => match value {
            ResourceValueSyntax::ResourceVariableDeclaration(declaration) => {
                format_resource_variable_declaration(&declaration, doc)
            }
            ResourceValueSyntax::VariableAccess(access) => {
                match resolve_required_field(access.expression(), doc) {
                    JavaFormatField::Present(expression) => match expression.classify() {
                        Ok(VariableAccessSyntax::NameExpression(value)) => {
                            format_expression(&value.into(), doc)
                        }
                        Ok(VariableAccessSyntax::FieldAccessExpression(value)) => {
                            format_expression(&value.into(), doc)
                        }
                        Err(error) => {
                            doc.block_on_invariant(error.to_string());
                            Doc::nil()
                        }
                    },
                    JavaFormatField::Malformed(malformed) => malformed,
                }
            }
            ResourceValueSyntax::BogusResourceValue(bogus) => format_malformed(&bogus, doc),
        },
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn format_catch_clauses<'source>(
    clauses: impl IntoIterator<Item = JavaSyntaxListPart<'source, CatchClause<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for clause in clauses {
            let clause = match resolve_list_part(clause, docs) {
                JavaFormatListPart::Item(clause) => format_catch_clause(&clause, docs),
                JavaFormatListPart::Malformed(malformed) => malformed,
                JavaFormatListPart::Separator(separator) => {
                    docs.block_on_invariant("unseparated catch list had a separator");
                    format_token_with_comments(docs, &separator)
                }
            };
            docs.push(clause);
        }
    })
}

fn format_catch_clause<'source>(
    clause: &CatchClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameter = match resolve_required_field(clause.parameter(), doc) {
        JavaFormatField::Present(value) => {
            format_parenthesized_catch_parameter(clause, &value, doc)
        }
        JavaFormatField::Malformed(value) => value,
    };
    let body = match resolve_required_field(clause.body(), doc) {
        JavaFormatField::Present(value) => format_block(&value, doc),
        JavaFormatField::Malformed(value) => value,
    };
    doc_concat!(
        doc,
        [
            doc.space(),
            format_statement_keyword(clause.catch_keyword(), doc),
            doc.space(),
            parameter,
            doc.space(),
            body
        ]
    )
}

fn format_parenthesized_catch_parameter<'source>(
    clause: &CatchClause<'source>,
    parameter: &CatchParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(clause.open_paren(), doc);
    let close = resolve_required_delimiter(clause.close_paren(), doc);
    let inner = format_catch_parameter(parameter, doc);
    let open = match open {
        JavaFormatDelimiter::Source(open) => format_token_with_comments(doc, &open),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    };
    let close = match close {
        JavaFormatDelimiter::Source(close) => format_token_with_comments(doc, &close),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    };
    let inner = doc_indent!(doc, doc_concat!(doc, [doc.soft_line(), inner]));
    doc_group!(doc, doc_concat!(doc, [open, inner, doc.soft_line(), close]))
}

fn format_catch_parameter<'source>(
    parameter: &CatchParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = match resolve_required_field(parameter.modifiers(), doc) {
        JavaFormatField::Present(value) => format_parameter_modifiers(&value, doc),
        JavaFormatField::Malformed(value) => value,
    };
    let types = match resolve_required_field(parameter.types(), doc) {
        JavaFormatField::Present(value) => format_catch_type_list(&value, doc),
        JavaFormatField::Malformed(value) => value,
    };
    let name = match resolve_required_field(parameter.name(), doc) {
        JavaFormatField::Present(value) => format_token_with_comments(doc, &value),
        JavaFormatField::Malformed(value) => value,
    };
    let dimensions = match resolve_optional_field(parameter.dimensions(), doc) {
        JavaFormatField::Present(Some(dimensions)) => format_array_dimensions(&dimensions, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(value) => value,
    };
    doc_concat!(doc, [modifiers, types, doc.space(), name, dimensions])
}

fn format_parameter_modifiers<'source>(
    modifiers: &jolt_java_syntax::ParameterModifierList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut has_visible_item = false;
    let modifiers = doc.concat_list(|docs| {
        for item in modifiers.partitioned_items() {
            let (item, visible) = match item {
                Ok(
                    PartitionedModifierItem::DeclarationAnnotation(annotation)
                    | PartitionedModifierItem::TypeUseAnnotation(annotation),
                ) => (format_annotation(&annotation, docs), true),
                Ok(
                    PartitionedModifierItem::Token(token) | PartitionedModifierItem::Sealed(token),
                ) => (format_token_with_comments(docs, &token), true),
                Ok(PartitionedModifierItem::NonSealed(modifier)) => {
                    docs.block_on_invariant(format!(
                        "unexpected non-sealed catch modifier at {:?}",
                        modifier.text_range()
                    ));
                    (Doc::nil(), false)
                }
                Ok(PartitionedModifierItem::Bogus(bogus)) => {
                    let visible = bogus.first_token().is_some();
                    (format_malformed(&bogus, docs), visible)
                }
                Ok(PartitionedModifierItem::Missing(missing)) => (
                    crate::helpers::recovery::format_missing(&missing, docs),
                    false,
                ),
                Ok(PartitionedModifierItem::Malformed(malformed)) => {
                    let visible = malformed.first_token().is_some();
                    (format_malformed(&malformed, docs), visible)
                }
                Err(error) => {
                    docs.block_on_invariant(error.to_string());
                    (Doc::nil(), false)
                }
            };
            if visible && has_visible_item {
                let space = docs.space();
                docs.push(space);
            }
            docs.push(item);
            has_visible_item |= visible;
        }
    });
    if has_visible_item {
        doc_concat!(doc, [modifiers, doc.space()])
    } else {
        modifiers
    }
}

#[allow(clippy::map_unwrap_or)]
fn format_catch_type_list<'source>(
    types: &jolt_java_syntax::CatchTypeList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match resolve_required_field(types.types(), doc) {
        JavaFormatField::Present(role) => role
            .as_type()
            .map(|ty| format_type(&ty, doc))
            .unwrap_or_else(|error| {
                doc.block_on_invariant(error.to_string());
                Doc::nil()
            }),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn format_finally_clause<'source>(
    clause: &FinallyClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = match resolve_required_field(clause.body(), doc) {
        JavaFormatField::Present(value) => format_block(&value, doc),
        JavaFormatField::Malformed(value) => value,
    };
    doc_concat!(
        doc,
        [
            doc.space(),
            format_statement_keyword(clause.finally_keyword(), doc),
            doc.space(),
            body
        ]
    )
}

fn format_removed_resource_separator_comments<'source>(
    separator: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    format_removed_comments(
        doc,
        separator
            .leading_comments()
            .chain(separator.trailing_comments()),
    )
}
