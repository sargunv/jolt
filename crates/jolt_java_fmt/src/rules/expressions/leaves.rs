use super::{
    ClassLiteralExpression, Doc, Expression, InlineLeadingTrivia, JavaFormatter, JavaSyntaxToken,
    LeadingComments, LeadingTrivia, LiteralExpression, NameExpression, SuperExpression,
    TemplateExpression, ThisExpression, TrailingTrivia, concat, format_annotation,
    format_array_dimensions, format_expression, format_member_dot, format_token,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    format_void_type, hard_line, text,
};

pub(super) fn format_literal_expression<'source>(
    expression: &LiteralExpression<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    expression
        .literal_token()
        .map_or_else(jolt_fmt_ir::nil, |token| {
            format_leaf_token(&token, leading_comments)
        })
}

pub(super) fn format_template_expression<'source>(
    expression: &TemplateExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        expression
            .processor()
            .map_or_else(jolt_fmt_ir::nil, |processor| {
                format_expression(&processor, formatter)
            }),
        format_member_dot(expression.dot_token().as_ref()),
        expression
            .template()
            .map_or_else(jolt_fmt_ir::nil, |template| {
                format_literal_expression(&template, LeadingComments::Preserve)
            }),
    ])
}

pub(super) fn format_name_expression<'source>(
    expression: &NameExpression<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let annotations = expression
        .annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let name = expression.name().map_or_else(jolt_fmt_ir::nil, |name| {
        format_leaf_token(&name, leading_comments)
    });

    if annotations.is_empty() {
        name
    } else {
        concat([jolt_fmt_ir::join(&text(" "), annotations), text(" "), name])
    }
}

pub(super) fn format_this_expression<'source>(
    expression: &ThisExpression<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(jolt_fmt_ir::nil, |token| {
            format_leaf_token(&token, leading_comments)
        }),
        formatter,
    )
}

pub(super) fn format_super_expression<'source>(
    expression: &SuperExpression<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(jolt_fmt_ir::nil, |token| {
            format_leaf_token(&token, leading_comments)
        }),
        formatter,
    )
}

fn format_qualified_keyword_expression<'source>(
    qualifier: Option<Expression<'source>>,
    dot: Option<&JavaSyntaxToken<'source>>,
    keyword: Doc<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match qualifier {
        Some(qualifier) => concat([
            format_expression(&qualifier, formatter),
            format_member_dot(dot),
            keyword,
        ]),
        None => keyword,
    }
}

pub(super) fn format_leaf_token<'source>(
    token: &jolt_java_syntax::JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
) -> Doc<'source> {
    match leading_comments {
        LeadingComments::Preserve => {
            format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }
        LeadingComments::SuppressFirstToken => {
            format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
        }
    }
}

pub(super) fn format_class_literal_expression<'source>(
    expression: &ClassLiteralExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let target = expression.target_expression().map_or_else(
        || {
            expression.void_type().map_or_else(
                || {
                    expression
                        .primitive_keyword()
                        .map_or_else(jolt_fmt_ir::nil, |keyword| {
                            format_leaf_token(&keyword, LeadingComments::Preserve)
                        })
                },
                |ty| format_void_type(&ty),
            )
        },
        |target| format_expression(&target, formatter),
    );

    concat([
        target,
        expression
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions, formatter)
            }),
        expression
            .dot_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_class_literal_dot),
        expression
            .class_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }),
    ])
}

fn format_class_literal_dot<'source>(dot: &JavaSyntaxToken<'source>) -> Doc<'source> {
    concat([
        format_token_with_inline_leading_comments(
            dot,
            InlineLeadingTrivia::AfterPreviousToken,
            TrailingTrivia::BeforeLineBreak,
        ),
        if dot
            .trailing_comments()
            .any(|comment| super::comment_forces_line(&comment))
        {
            hard_line()
        } else if dot.trailing_comments().is_empty() {
            jolt_fmt_ir::nil()
        } else {
            text(" ")
        },
    ])
}
