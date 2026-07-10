use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CatchClause, DestructuringDeclaration, DoWhileStatement, Expression, FinallyClause,
    ForStatement, IfExpression, JumpExpression, KotlinSyntaxKind, KotlinSyntaxToken,
    LambdaExpression, NameExpression, ParenthesizedExpression, ThrowExpression, TryExpression,
    WhenCondition, WhenEntry, WhenExpression, WhenGuard, WhenSubject, WhileStatement,
};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, recovered_comma_list_items,
};
use crate::rules::names::format_name;
use crate::rules::variables::format_value_parameter_list;

use super::{format_expression, lambdas::lambda_body_doc};

pub(super) fn format_if_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.if_token() else {
        return doc.nil();
    };

    let then_branch = expression.then_branch();
    let then_branch_is_nested_if = then_branch
        .as_ref()
        .is_some_and(|branch| matches!(branch, Expression::IfExpression(_)));

    let keyword = format_token(doc, &keyword, leading, TrailingTrivia::Preserve);
    let space = doc.space();
    let condition = format_control_flow_condition(
        doc,
        expression.open_paren(),
        expression.condition(),
        expression.close_paren(),
    );
    let then_branch = if let Some(then_branch) = then_branch {
        if then_branch_is_nested_if {
            let line = doc.hard_line();
            let branch = format_if_branch(doc, &then_branch);
            let branch = doc.concat([line, branch]);
            doc.indent(branch)
        } else {
            let space = doc.space();
            let branch = format_if_branch(doc, &then_branch);
            doc.concat([space, branch])
        }
    } else {
        doc.nil()
    };
    let else_branch = format_else_branch(doc, expression, then_branch_is_nested_if);
    doc.concat([keyword, space, condition, then_branch, else_branch])
}

pub(super) fn format_when_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &WhenExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.when_token() else {
        return doc.nil();
    };
    let Some(open_brace) = expression.open_brace() else {
        let keyword = format_token(doc, &keyword, leading, TrailingTrivia::Preserve);
        let subject = if let Some(subject) = expression.subject() {
            let space = doc.space();
            let subject = format_when_subject(doc, &subject);
            doc.concat([space, subject])
        } else {
            doc.nil()
        };
        return doc.concat([keyword, subject]);
    };
    let close_brace = expression.close_brace();
    let mut entries = expression.entries().peekable();

    let keyword = format_token(doc, &keyword, leading, TrailingTrivia::Preserve);
    let subject = if let Some(subject) = expression.subject() {
        let space = doc.space();
        let subject = format_when_subject(doc, &subject);
        doc.concat([space, subject])
    } else {
        doc.nil()
    };
    let space = doc.space();
    let open_brace = format_token(
        doc,
        &open_brace,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let entries = if entries.peek().is_none() {
        doc.hard_line()
    } else {
        let line = doc.hard_line();
        let entries = entries
            .map(|entry| format_when_entry(doc, &entry))
            .collect::<Vec<_>>();
        let entries = join_hard_lines(doc, entries);
        let entries = doc.concat([line, entries]);
        let entries = doc.indent(entries);
        let trailing = doc.hard_line();
        doc.concat([entries, trailing])
    };
    let close_brace = if let Some(close_brace) = close_brace {
        format_token(
            doc,
            &close_brace,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([keyword, subject, space, open_brace, entries, close_brace])
}

pub(super) fn format_try_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &TryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.try_token() else {
        return doc.nil();
    };

    let keyword = format_token(doc, &keyword, leading, TrailingTrivia::Preserve);
    let space = doc.space();
    let block = if let Some(block) = expression.block() {
        crate::rules::statements::format_block(doc, &block)
    } else {
        doc.nil()
    };
    let catches = expression
        .catch_clauses()
        .map(|clause| format_catch_clause(doc, &clause))
        .collect::<Vec<_>>();
    let catches = doc.concat(catches);
    let finally = if let Some(clause) = expression.finally_clause() {
        format_finally_clause(doc, &clause)
    } else {
        doc.nil()
    };
    doc.concat([keyword, space, block, catches, finally])
}

pub(super) fn format_labeled_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let label = expression.name_token()?;
    let at = expression.at_token()?;

    let label = format_token(
        doc,
        &label,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let at = format_token(
        doc,
        &at,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let labeled = if let Some(labeled) = expression.labeled_expression() {
        let space = doc.space();
        let labeled = super::format_expression_with_leading(
            doc,
            &labeled,
            LeadingTrivia::SuppressAlreadyHandled,
        );
        doc.concat([space, labeled])
    } else {
        doc.nil()
    };
    Some(doc.concat([label, at, labeled]))
}

pub(super) fn format_for_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ForStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = statement.for_token() else {
        return doc.nil();
    };

    let keyword = format_token(doc, &keyword, leading, TrailingTrivia::Preserve);
    let before_header = doc.space();
    let header = format_for_header(
        doc,
        statement,
        statement.open_paren(),
        statement.close_paren(),
    );
    let before_body = doc.space();
    let body = if let Some(block) = statement.block() {
        crate::rules::statements::format_block(doc, &block)
    } else if let Some(expression) = statement.body_expression() {
        format_expression(doc, &expression)
    } else {
        doc.nil()
    };
    doc.concat([keyword, before_header, header, before_body, body])
}

fn format_for_header<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ForStatement<'source>,
    open: Option<KotlinSyntaxToken<'source>>,
    close: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let variable = statement
        .destructuring_declaration()
        .map(|declaration| format_destructuring_declaration(doc, &declaration))
        .or_else(|| {
            statement
                .variable_expression()
                .map(|expression| format_expression(doc, &expression))
        });

    let open = if let Some(open) = open {
        format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let variable = variable.unwrap_or_else(|| doc.nil());
    let in_token = if let Some(in_token) = statement.in_token() {
        let space = doc.space();
        let in_token = format_token(
            doc,
            &in_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        doc.concat([space, in_token])
    } else {
        doc.nil()
    };
    let iterable = if let Some(iterable) = statement.iterable_expression() {
        let space = doc.space();
        let iterable = format_expression(doc, &iterable);
        doc.concat([space, iterable])
    } else {
        doc.nil()
    };
    let close = if let Some(close) = close {
        format_token(
            doc,
            &close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([open, variable, in_token, iterable, close])
}

fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    let items =
        recovered_comma_list_items(doc, declaration.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: if let Some(name) = entry.entry.name() {
                    format_name(doc, &name)
                } else {
                    doc.nil()
                },
                comma: entry.comma,
            }
        });
    compact_parenthesized_list(
        doc,
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        items,
    )
}

pub(super) fn format_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &WhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = statement.while_token() else {
        return doc.nil();
    };

    let keyword = format_token(
        doc,
        &keyword,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let before_condition = doc.space();
    let condition = format_control_flow_condition(
        doc,
        statement.open_paren(),
        statement.condition(),
        statement.close_paren(),
    );
    let before_body = doc.space();
    let body = if let Some(block) = statement.block() {
        crate::rules::statements::format_block(doc, &block)
    } else if let Some(expression) = statement.body_expression() {
        format_expression(doc, &expression)
    } else {
        doc.nil()
    };
    doc.concat([keyword, before_condition, condition, before_body, body])
}

pub(super) fn format_do_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &DoWhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(do_token) = statement.do_token() else {
        return doc.nil();
    };
    let Some(while_token) = statement.while_token() else {
        let do_token = format_token(
            doc,
            &do_token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        let block = if let Some(block) = statement.block() {
            crate::rules::statements::format_block(doc, &block)
        } else {
            doc.nil()
        };
        return doc.concat([do_token, space, block]);
    };

    let do_token = format_token(
        doc,
        &do_token,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let before_block = doc.space();
    let block = if let Some(block) = statement.block() {
        crate::rules::statements::format_block(doc, &block)
    } else {
        doc.nil()
    };
    let before_while = doc.space();
    let while_token = format_token(
        doc,
        &while_token,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let before_condition = doc.space();
    let condition = format_control_flow_condition(
        doc,
        statement.open_paren(),
        statement.condition(),
        statement.close_paren(),
    );
    doc.concat([
        do_token,
        before_block,
        block,
        before_while,
        while_token,
        before_condition,
        condition,
    ])
}

pub(super) fn format_jump_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &JumpExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.keyword_token() else {
        return doc.nil();
    };

    let keyword = format_token(
        doc,
        &keyword,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let label = if let Some(at) = expression.at_token() {
        let at = format_token(
            doc,
            &at,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let label = if let Some(label) = expression.label_token() {
            format_token(
                doc,
                &label,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        } else {
            doc.nil()
        };
        doc.concat([at, label])
    } else {
        doc.nil()
    };
    let value = if let Some(value) = expression.expression() {
        let space = doc.space();
        let value = format_expression(doc, &value);
        doc.concat([space, value])
    } else {
        doc.nil()
    };
    doc.concat([keyword, label, value])
}

pub(super) fn format_throw_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ThrowExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.throw_token() else {
        return doc.nil();
    };

    let keyword = format_token(
        doc,
        &keyword,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let value = if let Some(value) = expression.expression() {
        let space = doc.space();
        let value = format_expression(doc, &value);
        doc.concat([space, value])
    } else {
        doc.nil()
    };
    doc.concat([keyword, value])
}

fn format_control_flow_condition<'source>(
    doc: &mut DocBuilder<'source>,
    mut open: Option<KotlinSyntaxToken<'source>>,
    condition: Option<Expression<'source>>,
    mut close: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let inner = match condition {
        Some(Expression::ParenthesizedExpression(condition)) => {
            open = open.or_else(|| condition.open_paren());
            close = close.or_else(|| condition.close_paren());
            format_parenthesized_condition_inner(doc, &condition)
        }
        Some(condition) => format_expression(doc, &condition),
        None => doc.nil(),
    };

    let open = if let Some(open) = open {
        format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let soft_line = doc.soft_line();
    let inner = doc.concat([soft_line, inner]);
    let inner = doc.indent(inner);
    let trailing = doc.soft_line();
    let close = if let Some(close) = close {
        format_token(
            doc,
            &close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let contents = doc.concat([open, inner, trailing, close]);
    doc.group(contents)
}

fn format_parenthesized_condition_inner<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    if let Some(expression) = condition.expression() {
        format_expression(doc, &expression)
    } else {
        doc.nil()
    }
}

fn format_else_branch<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    starts_after_broken_then: bool,
) -> Doc<'source> {
    let Some(else_token) = expression.else_token() else {
        return doc.nil();
    };
    let else_branch = expression.else_branch();

    let separator = if starts_after_broken_then {
        doc.hard_line()
    } else {
        doc.space()
    };
    let else_token = format_token(
        doc,
        &else_token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let else_branch = if let Some(else_branch) = else_branch {
        let space = doc.space();
        let branch = match else_branch {
            Expression::IfExpression(if_expression) => {
                format_if_expression(doc, &if_expression, LeadingTrivia::Preserve)
            }
            _ => format_if_branch(doc, &else_branch),
        };
        doc.concat([space, branch])
    } else {
        doc.nil()
    };
    doc.concat([separator, else_token, else_branch])
}

fn format_when_subject<'source>(
    doc: &mut DocBuilder<'source>,
    subject: &WhenSubject<'source>,
) -> Doc<'source> {
    let Some(open) = subject.open_paren() else {
        return if let Some(expression) = subject.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
    };
    let Some(close) = subject.close_paren() else {
        let open = format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let expression = if let Some(expression) = subject.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
        return doc.concat([open, expression]);
    };

    let open = format_token(
        doc,
        &open,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let body = format_when_subject_body(doc, subject);
    let close = format_token(
        doc,
        &close,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    doc.concat([open, body, close])
}

fn format_when_subject_body<'source>(
    doc: &mut DocBuilder<'source>,
    subject: &WhenSubject<'source>,
) -> Doc<'source> {
    let Some(val_token) = subject.val_token() else {
        return if let Some(expression) = subject.expression() {
            format_expression(doc, &expression)
        } else {
            doc.nil()
        };
    };
    let val_token = format_token(
        doc,
        &val_token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let space = doc.space();
    let name = if let Some(name) = subject.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let assign = if let Some(assign) = subject.assign_token() {
        let before = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let after = doc.space();
        doc.concat([before, assign, after])
    } else {
        doc.nil()
    };
    let expression = if let Some(expression) = subject.expression() {
        format_expression(doc, &expression)
    } else {
        doc.nil()
    };
    doc.concat([val_token, space, name, assign, expression])
}

fn format_when_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    let label = if let Some(else_token) = entry.else_token() {
        format_token(
            doc,
            &else_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        format_when_conditions(doc, entry)
    };
    let arrow = if let Some(arrow) = entry.arrow_token() {
        let space = doc.space();
        let arrow = format_token(
            doc,
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        doc.concat([space, arrow])
    } else {
        doc.nil()
    };
    let body = if let Some(expression) = entry.body_expression() {
        let space = doc.space();
        let expression = format_expression(doc, &expression);
        doc.concat([space, expression])
    } else {
        doc.nil()
    };
    doc.concat([label, arrow, body])
}

fn format_when_conditions<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    let mut entries = recovered_comma_list_items(
        doc,
        entry.condition_entries_with_recovered(),
        |doc, entry| CommaListItem {
            doc: format_when_condition(doc, &entry.condition),
            comma: entry.comma,
        },
    )
    .into_iter()
    .peekable();
    doc.concat_list(|docs| {
        while let Some(entry) = entries.next() {
            docs.push(entry.doc);
            if let Some(comma) = entry.comma {
                let comma = format_token(
                    docs,
                    &comma,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeSpaceIfComments,
                );
                docs.push(comma);
                if entries.peek().is_some() {
                    let space = docs.space();
                    docs.push(space);
                }
            } else if entries.peek().is_some() {
                let space = docs.space();
                docs.push(space);
            }
        }
        if let Some(guard) = entry.guard() {
            let space = docs.space();
            docs.push(space);
            let guard = format_when_guard(docs, &guard);
            docs.push(guard);
        }
    })
}

fn format_when_condition<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &WhenCondition<'source>,
) -> Doc<'source> {
    match condition.keyword_token() {
        Some(keyword)
            if matches!(
                keyword.kind(),
                KotlinSyntaxKind::IsKw | KotlinSyntaxKind::NotIs
            ) =>
        {
            let keyword = format_token(
                doc,
                &keyword,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let space = doc.space();
            let ty = if let Some(ty) = condition.ty() {
                crate::rules::types::format_type_reference(doc, &ty)
            } else {
                doc.nil()
            };
            doc.concat([keyword, space, ty])
        }
        Some(keyword)
            if matches!(
                keyword.kind(),
                KotlinSyntaxKind::InKw | KotlinSyntaxKind::NotIn
            ) =>
        {
            let keyword = format_token(
                doc,
                &keyword,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let space = doc.space();
            let expression = if let Some(expression) = condition.expression() {
                format_expression(doc, &expression)
            } else {
                doc.nil()
            };
            doc.concat([keyword, space, expression])
        }
        _ => {
            if let Some(expression) = condition.expression() {
                format_expression(doc, &expression)
            } else {
                format_token_sequence(doc, condition.token_iter(), LeadingTrivia::Preserve)
            }
        }
    }
}

fn format_when_guard<'source>(
    doc: &mut DocBuilder<'source>,
    guard: &WhenGuard<'source>,
) -> Doc<'source> {
    let if_token = if let Some(if_token) = guard.if_token() {
        format_token(
            doc,
            &if_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let expression = if let Some(expression) = guard.expression() {
        let space = doc.space();
        let expression = format_expression(doc, &expression);
        doc.concat([space, expression])
    } else {
        doc.nil()
    };
    doc.concat([if_token, expression])
}

fn format_catch_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &CatchClause<'source>,
) -> Doc<'source> {
    let before_keyword = doc.space();
    let keyword = if let Some(keyword) = clause.catch_token() {
        format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let before_parameters = doc.space();
    let parameters = if let Some(parameters) = clause.value_parameter_list() {
        format_value_parameter_list(doc, &parameters)
    } else {
        doc.nil()
    };
    let before_block = doc.space();
    let block = if let Some(block) = clause.block() {
        crate::rules::statements::format_block(doc, &block)
    } else {
        doc.nil()
    };
    doc.concat([
        before_keyword,
        keyword,
        before_parameters,
        parameters,
        before_block,
        block,
    ])
}

fn format_finally_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &FinallyClause<'source>,
) -> Doc<'source> {
    let before_keyword = doc.space();
    let keyword = if let Some(keyword) = clause.finally_token() {
        format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let before_block = doc.space();
    let block = if let Some(block) = clause.block() {
        crate::rules::statements::format_block(doc, &block)
    } else {
        doc.nil()
    };
    doc.concat([before_keyword, keyword, before_block, block])
}

fn format_if_branch<'source>(
    doc: &mut DocBuilder<'source>,
    branch: &Expression<'source>,
) -> Doc<'source> {
    match branch {
        Expression::LambdaExpression(lambda) => format_braced_body(doc, lambda),
        expression => format_expression(doc, expression),
    }
}

fn format_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    lambda: &LambdaExpression<'source>,
) -> Doc<'source> {
    let Some(open) = lambda.open_brace() else {
        return doc.nil();
    };
    let close = lambda.close_brace();
    let items = lambda.body_items().collect::<Vec<_>>();
    let body = lambda_body_doc(doc, lambda, &items);

    if body.is_empty() {
        let open = format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let close = if let Some(close) = close {
            format_token(
                doc,
                &close,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::Preserve,
            )
        } else {
            doc.nil()
        };
        return doc.concat([open, close]);
    }

    let open = format_token(
        doc,
        &open,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let line = doc.hard_line();
    let body = body.doc.expect("non-empty lambda body has a doc");
    let body = doc.concat([line, body]);
    let body = doc.indent(body);
    let trailing = doc.hard_line();
    let close = if let Some(close) = close {
        format_token(
            doc,
            &close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([open, body, trailing, close])
}
