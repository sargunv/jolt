use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, soft_line, space};
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
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list};
use crate::rules::names::format_name;
use crate::rules::statements::format_block_item;
use crate::rules::variables::format_value_parameter_list;

use super::format_expression;

pub(super) fn format_if_expression<'source>(
    expression: &IfExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.if_token() else {
        return jolt_fmt_ir::nil();
    };

    let then_branch = expression.then_branch();
    let then_branch_is_nested_if = then_branch
        .as_ref()
        .is_some_and(|branch| matches!(branch, Expression::IfExpression(_)));

    concat([
        format_token(&keyword, leading, TrailingTrivia::Preserve),
        space(),
        format_control_flow_condition(
            expression.open_paren(),
            expression.condition(),
            expression.close_paren(),
        ),
        then_branch.map_or_else(jolt_fmt_ir::nil, |then_branch| {
            if then_branch_is_nested_if {
                indent(concat([hard_line(), format_if_branch(&then_branch)]))
            } else {
                concat([space(), format_if_branch(&then_branch)])
            }
        }),
        format_else_branch(expression, then_branch_is_nested_if),
    ])
}

pub(super) fn format_when_expression<'source>(
    expression: &WhenExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.when_token() else {
        return jolt_fmt_ir::nil();
    };
    let Some(open_brace) = expression.open_brace() else {
        return concat([
            format_token(&keyword, leading, TrailingTrivia::Preserve),
            expression
                .subject()
                .map_or_else(jolt_fmt_ir::nil, |subject| {
                    concat([space(), format_when_subject(&subject)])
                }),
        ]);
    };
    let close_brace = expression.close_brace();
    let entries = expression.entries().collect::<Vec<_>>();

    concat([
        format_token(&keyword, leading, TrailingTrivia::Preserve),
        expression
            .subject()
            .map_or_else(jolt_fmt_ir::nil, |subject| {
                concat([space(), format_when_subject(&subject)])
            }),
        space(),
        format_token(
            &open_brace,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        if entries.is_empty() {
            hard_line()
        } else {
            concat([
                indent(concat([
                    hard_line(),
                    join_hard_lines(entries.iter().map(format_when_entry)),
                ])),
                hard_line(),
            ])
        },
        close_brace.map_or_else(jolt_fmt_ir::nil, |close_brace| {
            format_token(
                &close_brace,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::Preserve,
            )
        }),
    ])
}

pub(super) fn format_try_expression<'source>(
    expression: &TryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.try_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(&keyword, leading, TrailingTrivia::Preserve),
        space(),
        expression.block().map_or_else(jolt_fmt_ir::nil, |block| {
            crate::rules::statements::format_block(&block)
        }),
        concat(
            expression
                .catch_clauses()
                .map(|clause| format_catch_clause(&clause)),
        ),
        expression
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |clause| format_finally_clause(&clause)),
    ])
}

pub(super) fn format_labeled_expression<'source>(
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let label = expression.name_token()?;
    let at = expression.at_token()?;

    Some(concat([
        format_token(&label, leading, TrailingTrivia::RelocatedToEnclosingContext),
        format_token(
            &at,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        expression
            .labeled_expression()
            .map_or_else(jolt_fmt_ir::nil, |labeled| {
                concat([
                    space(),
                    super::format_expression_with_leading(
                        &labeled,
                        LeadingTrivia::SuppressAlreadyHandled,
                    ),
                ])
            }),
    ]))
}

pub(super) fn format_for_statement<'source>(
    statement: &ForStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = statement.for_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(&keyword, leading, TrailingTrivia::Preserve),
        space(),
        format_for_header(statement, statement.open_paren(), statement.close_paren()),
        space(),
        statement.block().map_or_else(
            || {
                statement
                    .body_expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression)
                    })
            },
            |block| crate::rules::statements::format_block(&block),
        ),
    ])
}

fn format_for_header<'source>(
    statement: &ForStatement<'source>,
    open: Option<KotlinSyntaxToken<'source>>,
    close: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let variable = statement
        .destructuring_declaration()
        .map(|declaration| format_destructuring_declaration(&declaration))
        .or_else(|| {
            statement
                .variable_expression()
                .map(|expression| format_expression(&expression))
        });

    concat([
        open.map_or_else(jolt_fmt_ir::nil, |open| {
            format_token(&open, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        variable.unwrap_or_else(jolt_fmt_ir::nil),
        statement
            .in_token()
            .map_or_else(jolt_fmt_ir::nil, |in_token| {
                concat([
                    space(),
                    format_token(&in_token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                ])
            }),
        statement
            .iterable_expression()
            .map_or_else(jolt_fmt_ir::nil, |iterable| {
                concat([space(), format_expression(&iterable)])
            }),
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            format_token(&close, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
    ])
}

fn format_destructuring_declaration<'source>(
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    compact_parenthesized_list(
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        declaration
            .entries_with_commas()
            .map(|entry| CommaListItem {
                doc: entry
                    .entry
                    .name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(super) fn format_while_statement<'source>(
    statement: &WhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = statement.while_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(
            &keyword,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        format_control_flow_condition(
            statement.open_paren(),
            statement.condition(),
            statement.close_paren(),
        ),
        space(),
        statement.block().map_or_else(
            || {
                statement
                    .body_expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression)
                    })
            },
            |block| crate::rules::statements::format_block(&block),
        ),
    ])
}

pub(super) fn format_do_while_statement<'source>(
    statement: &DoWhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(do_token) = statement.do_token() else {
        return jolt_fmt_ir::nil();
    };
    let Some(while_token) = statement.while_token() else {
        return concat([
            format_token(
                &do_token,
                leading,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            space(),
            statement.block().map_or_else(jolt_fmt_ir::nil, |block| {
                crate::rules::statements::format_block(&block)
            }),
        ]);
    };

    concat([
        format_token(
            &do_token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        statement.block().map_or_else(jolt_fmt_ir::nil, |block| {
            crate::rules::statements::format_block(&block)
        }),
        space(),
        format_token(
            &while_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        format_control_flow_condition(
            statement.open_paren(),
            statement.condition(),
            statement.close_paren(),
        ),
    ])
}

pub(super) fn format_jump_expression<'source>(
    expression: &JumpExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.keyword_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(
            &keyword,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        expression.at_token().map_or_else(jolt_fmt_ir::nil, |at| {
            concat([
                format_token(
                    &at,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                ),
                expression
                    .label_token()
                    .map_or_else(jolt_fmt_ir::nil, |label| {
                        format_token(&label, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
                    }),
            ])
        }),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |value| {
                concat([space(), format_expression(&value)])
            }),
    ])
}

pub(super) fn format_throw_expression<'source>(
    expression: &ThrowExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(keyword) = expression.throw_token() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(
            &keyword,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |value| {
                concat([space(), format_expression(&value)])
            }),
    ])
}

fn format_control_flow_condition<'source>(
    mut open: Option<KotlinSyntaxToken<'source>>,
    condition: Option<Expression<'source>>,
    mut close: Option<KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let inner = match condition {
        Some(Expression::ParenthesizedExpression(condition)) => {
            open = open.or_else(|| condition.open_paren());
            close = close.or_else(|| condition.close_paren());
            format_parenthesized_condition_inner(&condition)
        }
        Some(condition) => format_expression(&condition),
        None => jolt_fmt_ir::nil(),
    };

    group(concat([
        open.map_or_else(jolt_fmt_ir::nil, |open| {
            format_token(&open, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        indent(concat([soft_line(), inner])),
        soft_line(),
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            format_token(&close, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
    ]))
}

fn format_parenthesized_condition_inner<'source>(
    condition: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    condition
        .expression()
        .map_or_else(jolt_fmt_ir::nil, |expression| {
            format_expression(&expression)
        })
}

fn format_else_branch<'source>(
    expression: &IfExpression<'source>,
    starts_after_broken_then: bool,
) -> Doc<'source> {
    let Some(else_token) = expression.else_token() else {
        return jolt_fmt_ir::nil();
    };
    let else_branch = expression.else_branch();

    concat([
        if starts_after_broken_then {
            hard_line()
        } else {
            space()
        },
        format_token(
            &else_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        else_branch.map_or_else(jolt_fmt_ir::nil, |else_branch| {
            concat([
                space(),
                match else_branch {
                    Expression::IfExpression(if_expression) => {
                        format_if_expression(&if_expression, LeadingTrivia::Preserve)
                    }
                    _ => format_if_branch(&else_branch),
                },
            ])
        }),
    ])
}

fn format_when_subject<'source>(subject: &WhenSubject<'source>) -> Doc<'source> {
    let Some(open) = subject.open_paren() else {
        return subject
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            });
    };
    let Some(close) = subject.close_paren() else {
        return concat([
            format_token(&open, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
            subject
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression)
                }),
        ]);
    };

    concat([
        format_token(&open, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        format_when_subject_body(subject),
        format_token(&close, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
    ])
}

fn format_when_subject_body<'source>(subject: &WhenSubject<'source>) -> Doc<'source> {
    let Some(val_token) = subject.val_token() else {
        return subject
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            });
    };
    concat([
        format_token(
            &val_token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        space(),
        subject
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        subject
            .assign_token()
            .map_or_else(jolt_fmt_ir::nil, |assign| {
                concat([
                    space(),
                    format_token(&assign, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                    space(),
                ])
            }),
        subject
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    ])
}

fn format_when_entry<'source>(entry: &WhenEntry<'source>) -> Doc<'source> {
    let label = entry.else_token().map_or_else(
        || format_when_conditions(entry),
        |else_token| {
            format_token(
                &else_token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        },
    );

    concat([
        label,
        entry.arrow_token().map_or_else(jolt_fmt_ir::nil, |arrow| {
            concat([
                space(),
                format_token(&arrow, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
            ])
        }),
        entry
            .body_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                concat([space(), format_expression(&expression)])
            }),
    ])
}

fn format_when_conditions<'source>(entry: &WhenEntry<'source>) -> Doc<'source> {
    let entries = entry.condition_entries().collect::<Vec<_>>();
    let mut docs = Vec::new();
    let entry_count = entries.len();
    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_when_condition(&entry.condition));
        if let Some(comma) = entry.comma {
            docs.push(format_token(
                &comma,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeSpaceIfComments,
            ));
            if index + 1 < entry_count {
                docs.push(space());
            }
        }
    }
    if let Some(guard) = entry.guard() {
        docs.push(space());
        docs.push(format_when_guard(&guard));
    }
    concat(docs)
}

fn format_when_condition<'source>(condition: &WhenCondition<'source>) -> Doc<'source> {
    match condition.keyword_token() {
        Some(keyword)
            if matches!(
                keyword.kind(),
                KotlinSyntaxKind::IsKw | KotlinSyntaxKind::NotIs
            ) =>
        {
            concat([
                format_token(&keyword, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                space(),
                condition.ty().map_or_else(jolt_fmt_ir::nil, |ty| {
                    crate::rules::types::format_type_reference(&ty)
                }),
            ])
        }
        Some(keyword)
            if matches!(
                keyword.kind(),
                KotlinSyntaxKind::InKw | KotlinSyntaxKind::NotIn
            ) =>
        {
            concat([
                format_token(&keyword, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                space(),
                condition
                    .expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression)
                    }),
            ])
        }
        _ => condition.expression().map_or_else(
            || format_token_sequence(condition.token_iter(), LeadingTrivia::Preserve),
            |expression| format_expression(&expression),
        ),
    }
}

fn format_when_guard<'source>(guard: &WhenGuard<'source>) -> Doc<'source> {
    concat([
        guard.if_token().map_or_else(jolt_fmt_ir::nil, |if_token| {
            format_token(&if_token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }),
        guard
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                concat([space(), format_expression(&expression)])
            }),
    ])
}

fn format_catch_clause<'source>(clause: &CatchClause<'source>) -> Doc<'source> {
    concat([
        space(),
        clause
            .catch_token()
            .map_or_else(jolt_fmt_ir::nil, |keyword| {
                format_token(
                    &keyword,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        space(),
        clause
            .value_parameter_list()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_value_parameter_list(&parameters)
            }),
        space(),
        clause.block().map_or_else(jolt_fmt_ir::nil, |block| {
            crate::rules::statements::format_block(&block)
        }),
    ])
}

fn format_finally_clause<'source>(clause: &FinallyClause<'source>) -> Doc<'source> {
    concat([
        space(),
        clause
            .finally_token()
            .map_or_else(jolt_fmt_ir::nil, |keyword| {
                format_token(
                    &keyword,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        space(),
        clause.block().map_or_else(jolt_fmt_ir::nil, |block| {
            crate::rules::statements::format_block(&block)
        }),
    ])
}

fn format_if_branch<'source>(branch: &Expression<'source>) -> Doc<'source> {
    match branch {
        Expression::LambdaExpression(lambda) => format_braced_body(lambda),
        expression => format_expression(expression),
    }
}

fn format_braced_body<'source>(lambda: &LambdaExpression<'source>) -> Doc<'source> {
    let Some(open) = lambda.open_brace() else {
        return jolt_fmt_ir::nil();
    };
    let close = lambda.close_brace();
    let items = lambda.body_items().collect::<Vec<_>>();

    if items.is_empty() {
        return concat([
            format_token(
                &open,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            close.map_or_else(jolt_fmt_ir::nil, |close| {
                format_token(
                    &close,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::Preserve,
                )
            }),
        ]);
    }

    concat([
        format_token(
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        indent(concat([
            hard_line(),
            join_hard_lines(items.iter().map(format_block_item)),
        ])),
        hard_line(),
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            format_token(
                &close,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::Preserve,
            )
        }),
    ])
}
