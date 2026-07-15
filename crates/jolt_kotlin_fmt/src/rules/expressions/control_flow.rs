use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CatchClause, Declaration, DestructuringDeclaration, DestructuringPatternEntry,
    DoWhileStatement, Expression, FinallyClause, ForStatement, IfExpression, JumpExpression,
    KotlinRoleElement, KotlinSyntaxToken, NameExpression, ParenthesizedExpression, ThrowExpression,
    TryExpression, TypeReference, WhenCondition, WhenEntry, WhenExpression, WhenGuard, WhenSubject,
    WhileStatement,
};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, KotlinFormatListPart, format_optional_field,
    format_or_verbatim, format_required_field, resolve_list_part, resolve_optional_field,
    resolve_required_delimiter, resolve_required_field,
};
use crate::rules::names::format_name;
use crate::rules::variables::format_value_parameter_list;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_if_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = format_required_token(expression.if_token(), doc, leading);
        let condition = format_required_field(expression.condition(), doc, |condition, doc| {
            format_control_flow_condition(doc, &condition)
        });
        let then_branch = resolve_required_field(expression.then_branch(), doc);
        let then_branch_is_nested_if = matches!(
            &then_branch,
            KotlinFormatField::Present(branch)
                if branch.cast_node::<IfExpression<'source>>().is_some()
        );
        let then_branch = match then_branch {
            KotlinFormatField::Present(branch) => {
                let branch = format_if_branch(doc, branch);
                if then_branch_is_nested_if {
                    let line = doc.hard_line();
                    let branch = doc.concat([line, branch]);
                    doc.indent(branch)
                } else {
                    let space = doc.space();
                    doc.concat([space, branch])
                }
            }
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let else_branch = format_else_branch(doc, expression, then_branch_is_nested_if);
        let space = doc.space();
        doc.concat([keyword, space, condition, then_branch, else_branch])
    })
}

pub(super) fn format_when_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &WhenExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = format_required_token(expression.when_token(), doc, leading);
        let subject = match resolve_optional_field(expression.subject(), doc) {
            KotlinFormatField::Present(Some(subject)) => {
                let space = doc.space();
                let subject = format_when_subject(doc, &subject);
                doc.concat([space, subject])
            }
            KotlinFormatField::Present(None) => Doc::nil(),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let open = resolve_required_delimiter(expression.open_brace(), doc);
        let close = resolve_required_delimiter(expression.close_brace(), doc);
        let entries = match resolve_required_field(expression.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                let mut parts = Vec::new();
                for part in entries.parts() {
                    let part = match resolve_list_part(part, doc) {
                        KotlinFormatListPart::Item(element) => {
                            format_when_entry_element(doc, element)
                        }
                        KotlinFormatListPart::Separator(token) => format_plain_token(doc, token),
                        KotlinFormatListPart::Malformed(recovery) => recovery,
                    };
                    parts.push(part);
                }
                parts
            }
            KotlinFormatField::Malformed(recovery) => vec![recovery],
        };
        let entries = if entries.is_empty() {
            doc.hard_line()
        } else {
            let line = doc.hard_line();
            let entries = join_hard_lines(doc, entries);
            let entries = doc.concat([line, entries]);
            let entries = doc.indent(entries);
            let trailing = doc.hard_line();
            doc.concat([entries, trailing])
        };
        let space = if open.source().is_some() {
            doc.space()
        } else {
            Doc::nil()
        };
        let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
        let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
        doc.concat([keyword, subject, space, open, entries, close])
    })
}

pub(super) fn format_try_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &TryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = format_required_token(expression.try_token(), doc, leading);
        let block = format_required_field(expression.block(), doc, |block, doc| {
            crate::rules::statements::format_block(doc, &block)
        });
        let catches = match resolve_required_field(expression.catches(), doc) {
            KotlinFormatField::Present(catches) => doc.concat_list(|docs| {
                for part in catches.parts() {
                    let part = match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(clause) => format_catch_clause(docs, &clause),
                        KotlinFormatListPart::Separator(token) => format_plain_token(docs, token),
                        KotlinFormatListPart::Malformed(recovery) => recovery,
                    };
                    docs.push(part);
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let finally = format_optional_field(expression.finally(), doc, |clause, doc| {
            format_finally_clause(doc, &clause)
        });
        let space = doc.space();
        doc.concat([keyword, space, block, catches, finally])
    })
}

pub(super) fn format_labeled_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let at = match resolve_optional_field(expression.at(), doc) {
        KotlinFormatField::Present(Some(at)) => at,
        KotlinFormatField::Present(None) => return None,
        KotlinFormatField::Malformed(recovery) => return Some(recovery),
    };
    Some(format_or_verbatim(expression, doc, |doc| {
        let label = format_required_token(expression.name(), doc, leading);
        let at = format_token(
            doc,
            &at,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let labeled = match resolve_optional_field(expression.labeled_expression(), doc) {
            KotlinFormatField::Present(Some(labeled)) => {
                let space = doc.space();
                let labeled = format_expression_with_leading(
                    doc,
                    &labeled,
                    LeadingTrivia::SuppressAlreadyHandled,
                );
                doc.concat([space, labeled])
            }
            KotlinFormatField::Present(None) => Doc::nil(),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        doc.concat([label, at, labeled])
    }))
}

pub(super) fn format_for_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ForStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let keyword = format_required_token(statement.for_token(), doc, leading);
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let variable = format_required_field(statement.variable(), doc, |variable, doc| {
            format_for_variable(doc, variable)
        });
        let in_token = format_required_token(statement.in_token(), doc, LeadingTrivia::Preserve);
        let iterable = format_required_field(statement.iterable(), doc, |iterable, doc| {
            format_expression(doc, &iterable)
        });
        let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
        let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
        let space = doc.space();
        let header = doc.concat([open, variable, space, in_token, space, iterable, close]);
        let body = format_optional_field(statement.body(), doc, |body, doc| {
            format_body_role(doc, body)
        });
        let before_header = doc.space();
        let before_body = doc.space();
        doc.concat([keyword, before_header, header, before_body, body])
    })
}

fn format_for_variable<'source>(
    doc: &mut DocBuilder<'source>,
    variable: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(declaration) = variable.cast_node::<DestructuringDeclaration<'source>>() {
        format_destructuring_declaration(doc, &declaration)
    } else if let Some(expression) = variable.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else {
        doc.block_on_invariant("Kotlin for variable had an unsupported generated element");
        Doc::nil()
    }
}

fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    format_or_verbatim(declaration, doc, |doc| {
        let open = resolve_required_delimiter(declaration.open_delimiter(), doc);
        let close = resolve_required_delimiter(declaration.close_delimiter(), doc);
        let items = match resolve_required_field(declaration.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                physical_comma_list_items(doc, entries.parts(), |doc, entry| CommaListItem {
                    doc: format_destructuring_entry(doc, &entry),
                    comma: None,
                })
            }
            KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
                doc: recovery,
                comma: None,
            }],
        };
        let list = compact_parenthesized_list(doc, open.source(), close.source(), items);
        let open_recovery = delimiter_recovery(open);
        let close_recovery = delimiter_recovery(close);
        doc.concat([open_recovery, list, close_recovery])
    })
}

fn format_destructuring_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &DestructuringPatternEntry<'source>,
) -> Doc<'source> {
    match entry {
        DestructuringPatternEntry::DestructuringEntry(entry) => {
            format_or_verbatim(entry, doc, |doc| {
                let modifier = format_optional_field(entry.modifier(), doc, |modifier, doc| {
                    let modifier = format_plain_token(doc, modifier);
                    let space = doc.space();
                    doc.concat([modifier, space])
                });
                let name =
                    format_required_field(entry.name(), doc, |name, doc| format_name(doc, &name));
                let assign = format_optional_field(entry.assign(), doc, |assign, doc| {
                    let before = doc.space();
                    let assign = format_plain_token(doc, assign);
                    let after = doc.space();
                    doc.concat([before, assign, after])
                });
                let default = format_optional_field(entry.default(), doc, |default, doc| {
                    format_expression(doc, &default)
                });
                doc.concat([modifier, name, assign, default])
            })
        }
        DestructuringPatternEntry::BogusDestructuringEntry(entry) => {
            crate::helpers::recovery::format_malformed(entry, doc)
        }
    }
}

pub(super) fn format_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &WhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let keyword = format_required_token(statement.while_token(), doc, leading);
        let condition = format_required_field(statement.condition(), doc, |condition, doc| {
            format_control_flow_condition(doc, &condition)
        });
        let body = format_optional_field(statement.body(), doc, |body, doc| {
            format_body_role(doc, body)
        });
        let space = doc.space();
        doc.concat([keyword, space, condition, space, body])
    })
}

pub(super) fn format_do_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &DoWhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let do_token = format_required_token(statement.do_token(), doc, leading);
        let body = format_optional_field(statement.body(), doc, |body, doc| {
            format_body_role(doc, body)
        });
        let (while_token, has_while) = match resolve_required_field(statement.while_token(), doc) {
            KotlinFormatField::Present(token) => (
                format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                true,
            ),
            KotlinFormatField::Malformed(recovery) => (recovery, false),
        };
        let (condition, has_condition) = match resolve_required_field(statement.condition(), doc) {
            KotlinFormatField::Present(condition) => {
                (format_control_flow_condition(doc, &condition), true)
            }
            KotlinFormatField::Malformed(recovery) => (recovery, false),
        };
        let after_do = doc.space();
        let before_while = if has_while { doc.space() } else { Doc::nil() };
        let before_condition = if has_condition {
            doc.space()
        } else {
            Doc::nil()
        };
        doc.concat([
            do_token,
            after_do,
            body,
            before_while,
            while_token,
            before_condition,
            condition,
        ])
    })
}

pub(super) fn format_jump_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &JumpExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = format_required_token(expression.keyword(), doc, leading);
        let at = format_optional_field(expression.at(), doc, |at, doc| format_plain_token(doc, at));
        let label = format_optional_field(expression.label(), doc, |label, doc| {
            format_plain_token(doc, label)
        });
        let value = match resolve_optional_field(expression.expression(), doc) {
            KotlinFormatField::Present(Some(value)) => {
                let space = doc.space();
                let value = format_expression(doc, &value);
                doc.concat([space, value])
            }
            KotlinFormatField::Present(None) => Doc::nil(),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        doc.concat([keyword, at, label, value])
    })
}

pub(super) fn format_throw_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ThrowExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = format_required_token(expression.throw_token(), doc, leading);
        let value = format_required_field(expression.expression(), doc, |value, doc| {
            let space = doc.space();
            let value = format_expression(doc, &value);
            doc.concat([space, value])
        });
        doc.concat([keyword, value])
    })
}

fn format_control_flow_condition<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    format_or_verbatim(condition, doc, |doc| {
        let open = resolve_required_delimiter(condition.open_paren(), doc);
        let close = resolve_required_delimiter(condition.close_paren(), doc);
        let expression = format_required_field(condition.expression(), doc, |expression, doc| {
            format_expression(doc, &expression)
        });
        let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
        let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
        let soft_line = doc.soft_line();
        let inner = doc.concat([soft_line, expression]);
        let inner = doc.indent(inner);
        let trailing = doc.soft_line();
        let contents = doc.concat([open, inner, trailing, close]);
        doc.group(contents)
    })
}

fn format_else_branch<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    starts_after_broken_then: bool,
) -> Doc<'source> {
    let else_token = match resolve_optional_field(expression.else_token(), doc) {
        KotlinFormatField::Present(Some(token)) => token,
        KotlinFormatField::Present(None) => return Doc::nil(),
        KotlinFormatField::Malformed(recovery) => return recovery,
    };
    let token = format_plain_token(doc, else_token);
    let branch = match resolve_optional_field(expression.else_branch(), doc) {
        KotlinFormatField::Present(Some(branch)) => {
            let space = doc.space();
            let branch = format_if_branch(doc, branch);
            doc.concat([space, branch])
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let separator = if starts_after_broken_then {
        doc.hard_line()
    } else {
        doc.space()
    };
    doc.concat([separator, token, branch])
}

fn format_when_subject<'source>(
    doc: &mut DocBuilder<'source>,
    subject: &WhenSubject<'source>,
) -> Doc<'source> {
    format_or_verbatim(subject, doc, |doc| {
        let open = resolve_required_delimiter(subject.open_paren(), doc);
        let close = resolve_required_delimiter(subject.close_paren(), doc);
        let val_token = format_optional_field(subject.val_token(), doc, |token, doc| {
            let token = format_plain_token(doc, token);
            let space = doc.space();
            doc.concat([token, space])
        });
        let name = format_optional_field(subject.name(), doc, |name, doc| format_name(doc, &name));
        let assign = format_optional_field(subject.assign(), doc, |assign, doc| {
            let before = doc.space();
            let assign = format_plain_token(doc, assign);
            let after = doc.space();
            doc.concat([before, assign, after])
        });
        let expression = format_required_field(subject.expression(), doc, |expression, doc| {
            format_expression(doc, &expression)
        });
        let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
        let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
        doc.concat([open, val_token, name, assign, expression, close])
    })
}

fn format_when_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    format_or_verbatim(entry, doc, |doc| {
        let else_token = resolve_optional_field(entry.else_token(), doc);
        let label = match else_token {
            KotlinFormatField::Present(Some(token)) => format_plain_token(doc, token),
            KotlinFormatField::Present(None) => format_when_conditions(doc, entry),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let guard = format_optional_field(entry.guard(), doc, |guard, doc| {
            let space = doc.space();
            let guard = format_when_guard(doc, &guard);
            doc.concat([space, guard])
        });
        let arrow = format_required_field(entry.arrow(), doc, |arrow, doc| {
            let space = doc.space();
            let arrow = format_plain_token(doc, arrow);
            doc.concat([space, arrow])
        });
        let body = format_required_field(entry.body(), doc, |body, doc| {
            let space = doc.space();
            let body = format_body_role(doc, body);
            doc.concat([space, body])
        });
        doc.concat([label, guard, arrow, body])
    })
}

fn format_when_entry_element<'source>(
    doc: &mut DocBuilder<'source>,
    element: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(entry) = element.cast_node::<WhenEntry<'source>>() {
        format_when_entry(doc, &entry)
    } else if let Some(token) = element.token() {
        format_plain_token(doc, token)
    } else {
        doc.block_on_invariant("Kotlin when entry list contained an unsupported element");
        Doc::nil()
    }
}

fn format_when_conditions<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    match resolve_required_field(entry.conditions(), doc) {
        KotlinFormatField::Present(conditions) => {
            let items = physical_comma_list_items(doc, conditions.parts(), |doc, condition| {
                CommaListItem {
                    doc: format_when_condition(doc, &condition),
                    comma: None,
                }
            });
            let mut items = items.into_iter().peekable();
            doc.concat_list(|docs| {
                while let Some(item) = items.next() {
                    docs.push(item.doc);
                    if let Some(comma) = item.comma {
                        let comma = format_token(
                            docs,
                            &comma,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeSpaceIfComments,
                        );
                        docs.push(comma);
                    }
                    if items.peek().is_some() {
                        let space = docs.space();
                        docs.push(space);
                    }
                }
            })
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn format_when_condition<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &WhenCondition<'source>,
) -> Doc<'source> {
    format_or_verbatim(condition, doc, |doc| {
        let keyword = resolve_optional_field(condition.keyword(), doc);
        let value = match resolve_required_field(condition.value(), doc) {
            KotlinFormatField::Present(value) => format_when_condition_value(doc, value),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        match keyword {
            KotlinFormatField::Present(Some(keyword)) => {
                let keyword = format_plain_token(doc, keyword);
                let space = doc.space();
                doc.concat([keyword, space, value])
            }
            KotlinFormatField::Present(None) => value,
            KotlinFormatField::Malformed(recovery) => doc.concat([recovery, value]),
        }
    })
}

fn format_when_condition_value<'source>(
    doc: &mut DocBuilder<'source>,
    value: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(ty) = value.cast_node::<TypeReference<'source>>() {
        crate::rules::types::format_type_reference(doc, &ty)
    } else if let Some(expression) = value.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else {
        doc.block_on_invariant("Kotlin when condition contained an unsupported value");
        Doc::nil()
    }
}

fn format_when_guard<'source>(
    doc: &mut DocBuilder<'source>,
    guard: &WhenGuard<'source>,
) -> Doc<'source> {
    format_or_verbatim(guard, doc, |doc| {
        let if_token = format_required_token(guard.if_token(), doc, LeadingTrivia::Preserve);
        let expression = format_required_field(guard.expression(), doc, |expression, doc| {
            let space = doc.space();
            let expression = format_expression(doc, &expression);
            doc.concat([space, expression])
        });
        doc.concat([if_token, expression])
    })
}

fn format_catch_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &CatchClause<'source>,
) -> Doc<'source> {
    format_or_verbatim(clause, doc, |doc| {
        let keyword = format_required_token(clause.catch_token(), doc, LeadingTrivia::Preserve);
        let parameters = format_required_field(clause.parameters(), doc, |parameters, doc| {
            format_value_parameter_list(doc, &parameters)
        });
        let block = format_required_field(clause.block(), doc, |block, doc| {
            crate::rules::statements::format_block(doc, &block)
        });
        let space = doc.space();
        doc.concat([space, keyword, space, parameters, space, block])
    })
}

fn format_finally_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &FinallyClause<'source>,
) -> Doc<'source> {
    format_or_verbatim(clause, doc, |doc| {
        let keyword = format_required_token(clause.finally_token(), doc, LeadingTrivia::Preserve);
        let block = format_required_field(clause.block(), doc, |block, doc| {
            crate::rules::statements::format_block(doc, &block)
        });
        let space = doc.space();
        doc.concat([space, keyword, space, block])
    })
}

fn format_if_branch<'source>(
    doc: &mut DocBuilder<'source>,
    branch: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(block) = branch.cast_node::<jolt_kotlin_syntax::Block<'source>>() {
        crate::rules::statements::format_block(doc, &block)
    } else if let Some(expression) = branch.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else {
        doc.block_on_invariant("if branch had an unsupported physical role");
        Doc::nil()
    }
}

fn format_body_role<'source>(
    doc: &mut DocBuilder<'source>,
    body: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(block) = body.cast_node::<jolt_kotlin_syntax::Block<'source>>() {
        crate::rules::statements::format_block(doc, &block)
    } else if let Some(expression) = body.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else if let Some(declaration) = body.cast_family::<Declaration<'source>>() {
        crate::rules::declarations::format_declaration(doc, &declaration)
    } else {
        doc.block_on_invariant("Kotlin control-flow body had an unsupported generated element");
        Doc::nil()
    }
}

fn format_required_token<'source>(
    field: Result<
        jolt_kotlin_syntax::KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    })
}

fn format_plain_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        &token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

fn format_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    delimiter: KotlinFormatDelimiter<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(token) => {
            format_token(doc, &token, leading, TrailingTrivia::Preserve)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery,
    }
}

fn delimiter_recovery(delimiter: KotlinFormatDelimiter<'_>) -> Doc<'_> {
    match delimiter {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(recovery) => recovery,
    }
}
