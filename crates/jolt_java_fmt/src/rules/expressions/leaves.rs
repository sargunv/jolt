use super::{
    ClassLiteralExpression, Doc, Expression, InlineLeadingTrivia, JavaFormatter, JavaSyntaxToken,
    LeadingComments, LeadingTrivia, LiteralExpression, NameExpression, SuperExpression,
    ThisExpression, TrailingTrivia, concat, format_annotation, format_array_dimensions,
    format_expression, format_member_dot, format_token,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    format_void_type, hard_line, text,
};

pub(super) fn format_literal_expression(
    expression: &LiteralExpression,
    leading_comments: LeadingComments,
) -> Doc {
    expression
        .literal_token()
        .map_or_else(jolt_fmt_ir::nil, |token| {
            format_leaf_token(&token, leading_comments)
        })
}

pub(super) fn format_name_expression(
    expression: &NameExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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
        concat([jolt_fmt_ir::join(text(" "), annotations), text(" "), name])
    }
}

pub(super) fn format_this_expression(
    expression: &ThisExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(
            || text("this"),
            |token| format_leaf_token(&token, leading_comments),
        ),
        formatter,
    )
}

pub(super) fn format_super_expression(
    expression: &SuperExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(
            || text("super"),
            |token| format_leaf_token(&token, leading_comments),
        ),
        formatter,
    )
}

fn format_qualified_keyword_expression(
    qualifier: Option<Expression>,
    dot: Option<&JavaSyntaxToken>,
    keyword: Doc,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match qualifier {
        Some(qualifier) => concat([
            format_expression(&qualifier, formatter),
            format_member_dot(dot),
            keyword,
        ]),
        None => keyword,
    }
}

pub(super) fn format_leaf_token(
    token: &jolt_java_syntax::JavaSyntaxToken,
    leading_comments: LeadingComments,
) -> Doc {
    match leading_comments {
        LeadingComments::Preserve => {
            format_token(token, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        }
        LeadingComments::SuppressFirstToken => {
            format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
        }
    }
}

pub(super) fn format_class_literal_expression(
    expression: &ClassLiteralExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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

fn format_class_literal_dot(dot: &JavaSyntaxToken) -> Doc {
    concat([
        format_token_with_inline_leading_comments(
            dot,
            InlineLeadingTrivia::AfterPreviousToken,
            TrailingTrivia::BeforeLineBreak,
        ),
        if dot
            .trailing_comments()
            .iter()
            .any(super::comment_forces_line)
        {
            hard_line()
        } else if dot.trailing_comments().is_empty() {
            jolt_fmt_ir::nil()
        } else {
            text(" ")
        },
    ])
}
