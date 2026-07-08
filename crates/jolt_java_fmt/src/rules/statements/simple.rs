use super::{
    AssertStatement, Doc, Expression, ExpressionStatement, JavaFormatter, JavaSyntaxToken,
    LabeledStatement, LeadingTrivia, ReturnStatement, ThrowStatement, TrailingTrivia,
    YieldStatement, comment_forces_line, concat, format_expression, format_statement, format_token,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, hard_line, indent, trailing_comments_force_line,
};
use crate::helpers::comments::{comment_is_star_block, format_comment, format_token_sequence};
use jolt_fmt_ir::space;
use jolt_java_syntax::JavaComment;

pub(super) fn format_labeled_statement<'source>(
    statement: &LabeledStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let label = statement
        .label()
        .map_or_else(jolt_fmt_ir::nil, |label| format_token_with_comments(&label));

    concat([
        label,
        statement
            .colon()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        hard_line(),
        statement
            .body()
            .map_or_else(jolt_fmt_ir::nil, |body| format_statement(&body, formatter)),
    ])
}

pub(super) fn format_expression_statement<'source>(
    statement: &ExpressionStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(expression) = statement.expression() else {
        return format_token_sequence(statement.token_iter(), LeadingTrivia::Preserve);
    };

    concat([
        format_expression(&expression, formatter),
        format_statement_semicolon(statement.semicolon()),
    ])
}
pub(super) fn format_assert_statement<'source>(
    statement: &AssertStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_statement_keyword(statement.keyword(), "assert"),
        space(),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([
                space(),
                statement
                    .colon()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                space(),
                format_expression(&detail, formatter),
            ])
        }),
        format_statement_semicolon(statement.semicolon()),
    ])
}

pub(super) fn format_return_statement<'source>(
    statement: &ReturnStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "return",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

pub(super) fn format_throw_statement<'source>(
    statement: &ThrowStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "throw",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

pub(super) fn format_yield_statement<'source>(
    statement: &YieldStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "yield",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

fn format_keyword_expression_statement<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    expression: Option<Expression<'source>>,
    semicolon: Option<JavaSyntaxToken<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_statement_keyword_head(keyword, fallback),
        expression.map_or_else(jolt_fmt_ir::nil, |expression| {
            let expression = format_expression(&expression, formatter);
            let expression = concat([format_keyword_expression_separator(keyword), expression]);
            if keyword_expression_separator_forces_line(keyword) {
                indent(expression)
            } else {
                expression
            }
        }),
        format_statement_semicolon(semicolon),
    ])
}

fn keyword_expression_separator_forces_line(keyword: Option<&JavaSyntaxToken<'_>>) -> bool {
    keyword.is_some_and(trailing_comments_force_line)
}

fn format_keyword_expression_separator<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let Some(keyword) = keyword else {
        return space();
    };

    if keyword.trailing_comments().is_empty() {
        return space();
    }

    concat([
        format_trailing_comments_before_line_break(keyword),
        if trailing_comments_force_line(keyword) {
            hard_line()
        } else {
            space()
        },
    ])
}

pub(super) fn format_jump_statement<'source>(
    keyword: Option<JavaSyntaxToken<'source>>,
    fallback: &'static str,
    label: Option<JavaSyntaxToken<'source>>,
    semicolon: Option<JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        format_statement_keyword(keyword, fallback),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            concat([space(), format_token_with_comments(&label)])
        }),
        format_statement_semicolon(semicolon),
    ])
}

pub(crate) fn format_statement_semicolon(semicolon: Option<JavaSyntaxToken<'_>>) -> Doc<'_> {
    let Some(semicolon) = semicolon else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_semicolon_leading_comments(&semicolon),
        format_token(
            &semicolon,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        format_terminator_trailing_comments(&semicolon),
    ])
}

fn format_semicolon_leading_comments<'source>(
    semicolon: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in semicolon.leading_comments() {
        docs.push(space());
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

fn format_terminator_trailing_comments<'source>(token: &JavaSyntaxToken<'source>) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in token.trailing_comments() {
        if terminator_comment_starts_next_line(&comment) {
            docs.push(hard_line());
        } else {
            docs.push(space());
        }
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

fn terminator_comment_starts_next_line(comment: &JavaComment<'_>) -> bool {
    comment_is_star_block(comment)
}

pub(super) fn format_statement_keyword<'source>(
    keyword: Option<JavaSyntaxToken<'source>>,
    _fallback: &'static str,
) -> Doc<'source> {
    keyword.map_or_else(jolt_fmt_ir::nil, |keyword| {
        format_token_with_comments(&keyword)
    })
}

pub(super) fn format_statement_keyword_head<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    _fallback: &'static str,
) -> Doc<'source> {
    keyword.map_or_else(jolt_fmt_ir::nil, |keyword| {
        format_token_before_relocated_trailing_comments(keyword, LeadingTrivia::Preserve)
    })
}
