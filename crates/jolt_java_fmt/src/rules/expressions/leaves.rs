use super::{
    ClassLiteralExpression, Doc, Expression, JavaFormatter, JavaSyntaxToken, LeadingComments,
    LiteralExpression, NameExpression, SuperExpression, ThisExpression, concat, format_annotation,
    format_array_dimensions, format_expression, format_leading_comments, format_member_dot,
    format_token_text, format_trailing_comments, format_void_type, text,
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
    concat([
        match leading_comments {
            LeadingComments::Preserve => format_leading_comments(token),
            LeadingComments::SuppressFirstToken => jolt_fmt_ir::nil(),
        },
        format_token_text(token.text()),
        format_trailing_comments(token),
    ])
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
        text(".class"),
    ])
}
