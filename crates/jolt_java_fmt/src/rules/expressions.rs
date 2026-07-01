use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::{
    ArgumentList, ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer,
    AssignmentExpression, BinaryExpression, CastExpression, ClassLiteralExpression,
    ConditionalExpression, DimExpression, Expression, ExpressionParentRole, FieldAccessExpression,
    InstanceofExpression, JavaSyntaxToken, LambdaExpression, LambdaParameter, LiteralExpression,
    MemberChain, MemberChainSuffix, MethodInvocationExpression, MethodReferenceExpression,
    NameExpression, ObjectCreationExpression, ParenthesizedExpression, PostfixExpression,
    SuperExpression, SwitchExpression, ThisExpression, UnaryExpression, VariableInitializerValue,
};

use crate::context::JavaFormatter;
use crate::helpers::chains::member_chain;
use crate::helpers::comments::{
    comment_forces_line, format_leading_comments, format_token_text, format_token_with_comments,
    format_trailing_comments, format_trailing_comments_before_line_break, token_has_comments,
    tokens_have_comments, trailing_comments_force_line,
};
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::helpers::operators::{assignment_expression, binary_chain, ternary_expression};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_anonymous_class_body;
use crate::rules::patterns::format_pattern;
use crate::rules::statements::{format_block, format_switch_block};
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_argument_list, format_void_type,
};

pub(crate) fn format_expression(expression: &Expression, formatter: &JavaFormatter<'_>) -> Doc {
    format_expression_with_leading_comments(expression, LeadingComments::Preserve, formatter)
}

fn format_expression_with_leading_comments(
    expression: &Expression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match expression {
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(expression, formatter)
        }
        Expression::AssignmentExpression(expression) => {
            format_assignment_expression(expression, formatter)
        }
        Expression::ConditionalExpression(expression) => {
            format_conditional_expression(expression, formatter)
        }
        Expression::BinaryExpression(expression) => format_binary_expression(expression, formatter),
        Expression::UnaryExpression(expression) => format_unary_expression(expression, formatter),
        Expression::PostfixExpression(expression) => {
            format_postfix_expression(expression, formatter)
        }
        Expression::LambdaExpression(expression) => format_lambda_expression(expression, formatter),
        Expression::LiteralExpression(expression) => {
            format_literal_expression(expression, leading_comments)
        }
        Expression::NameExpression(expression) => {
            format_name_expression(expression, leading_comments, formatter)
        }
        Expression::ThisExpression(expression) => {
            format_this_expression(expression, leading_comments, formatter)
        }
        Expression::SuperExpression(expression) => {
            format_super_expression(expression, leading_comments, formatter)
        }
        Expression::ClassLiteralExpression(expression) => {
            format_class_literal_expression(expression, formatter)
        }
        Expression::MethodReferenceExpression(expression) => {
            format_method_reference_expression(expression, formatter)
        }
        Expression::SwitchExpression(expression) => format_switch_expression(expression, formatter),
        Expression::ArrayCreationExpression(expression) => {
            format_array_creation_expression(expression, formatter)
        }
        Expression::InstanceofExpression(expression) => {
            format_instanceof_expression(expression, formatter)
        }
        Expression::CastExpression(expression) => format_cast_expression(expression, formatter),
        Expression::FieldAccessExpression(expression) => {
            format_field_access_expression(expression, formatter)
        }
        Expression::ArrayAccessExpression(expression) => {
            format_array_access_expression(expression, formatter)
        }
        Expression::MethodInvocationExpression(expression) => {
            format_method_invocation_expression_with_leading_comments(
                expression,
                leading_comments,
                formatter,
            )
        }
        Expression::ObjectCreationExpression(expression) => {
            format_object_creation_expression(expression, formatter)
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum LeadingComments {
    Preserve,
    SuppressFirstToken,
}

fn format_literal_expression(
    expression: &LiteralExpression,
    leading_comments: LeadingComments,
) -> Doc {
    expression
        .literal_token()
        .map_or_else(jolt_fmt_ir::nil, |token| {
            format_leaf_token(&token, leading_comments)
        })
}

fn format_name_expression(
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

fn format_this_expression(
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

fn format_super_expression(
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

fn format_leaf_token(
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

fn format_class_literal_expression(
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

fn format_parenthesized_expression(
    expression: &ParenthesizedExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        format_parenthesized_expression_open(expression),
        indent(concat([
            format_open_parenthesized_expression_spacing(expression),
            expression
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression, formatter)
                }),
        ])),
        format_parenthesized_expression_close_with_spacing(expression),
    ]))
}

fn format_parenthesized_expression_open(expression: &ParenthesizedExpression) -> Doc {
    expression.open_paren().map_or_else(
        || text("("),
        |open| concat([format_leading_comments(&open), text("(")]),
    )
}

fn format_open_parenthesized_expression_spacing(expression: &ParenthesizedExpression) -> Doc {
    let Some(open) = expression.open_paren() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_parenthesized_expression_close_with_spacing(expression: &ParenthesizedExpression) -> Doc {
    let close_has_leading_comments = expression
        .close_paren()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        expression.close_paren().map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(&close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments(&close),
                ])
            },
        ),
    ])
}

fn format_assignment_expression(
    expression: &AssignmentExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    assignment_expression(
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left, formatter)),
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
            format_expression(&right, formatter)
        }),
    )
}

fn format_conditional_expression(
    expression: &ConditionalExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    ternary_expression(
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        expression
            .question_token()
            .map_or_else(|| text("?"), |token| format_token_with_comments(&token)),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        expression
            .colon_token()
            .map_or_else(|| text(":"), |token| format_token_with_comments(&token)),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
    )
}

fn format_binary_expression(expression: &BinaryExpression, formatter: &JavaFormatter<'_>) -> Doc {
    let (first, rest) = flatten_binary_expression(expression, formatter);
    binary_chain(format_expression(&first, formatter), rest)
}

fn format_unary_expression(expression: &UnaryExpression, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| {
                format_expression(&operand, formatter)
            }),
    ])
}

fn format_postfix_expression(expression: &PostfixExpression, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| {
                format_expression(&operand, formatter)
            }),
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
    ])
}

fn format_method_invocation_expression_with_leading_comments(
    expression: &MethodInvocationExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let expression = Expression::from(expression.clone());
    let parent_role = expression.parent_role();
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain, formatter);
    }
    let Expression::MethodInvocationExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    group(concat([
        format_method_invocation_callee(&expression, leading_comments, formatter),
        format_argument_list_for_parent_role(expression.arguments(), parent_role, formatter),
    ]))
}

fn format_field_access_expression(
    expression: &FieldAccessExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let expression = Expression::from(expression.clone());
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain, formatter);
    }
    let Expression::FieldAccessExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };
    let dot = expression.dot_token();

    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| {
                format_expression(&receiver, formatter)
            }),
        format_member_dot(dot.as_ref()),
        expression
            .field_name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
    ]))
}

fn format_method_reference_expression(
    expression: &MethodReferenceExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        format_method_reference_receiver(expression, formatter),
        format_method_reference_separator(expression),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
        if expression.is_constructor_reference() {
            expression
                .new_token()
                .map_or_else(|| text("new"), |token| format_token_with_comments(&token))
        } else {
            expression
                .target_name()
                .map_or_else(jolt_fmt_ir::nil, |target| {
                    format_token_with_comments(&target)
                })
        },
    ]))
}

fn format_method_reference_separator(expression: &MethodReferenceExpression) -> Doc {
    expression.double_colon().map_or_else(
        || text("::"),
        |separator| {
            let has_trailing_comments = !separator.trailing_comments().is_empty();
            concat([
                format_leading_comments(&separator),
                text("::"),
                format_trailing_comments_before_line_break(&separator),
                if trailing_comments_force_line(&separator) {
                    hard_line()
                } else if has_trailing_comments {
                    text(" ")
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        },
    )
}

fn format_method_reference_receiver(
    expression: &MethodReferenceExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if let Some(receiver) = expression.receiver_expression() {
        return concat([
            format_expression(&receiver, formatter),
            expression
                .receiver_dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions, formatter)
                }),
        ]);
    }

    expression
        .receiver_type()
        .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter))
}

fn format_array_access_expression(
    expression: &ArrayAccessExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open_bracket = expression.open_bracket();
    let close_bracket = expression.close_bracket();

    group(concat([
        expression.array().map_or_else(jolt_fmt_ir::nil, |array| {
            format_expression(&array, formatter)
        }),
        format_bracketed_expression(
            open_bracket.as_ref(),
            expression.index().map_or_else(jolt_fmt_ir::nil, |index| {
                format_expression(&index, formatter)
            }),
            close_bracket.as_ref(),
        ),
    ]))
}

fn format_object_creation_expression(
    expression: &ObjectCreationExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        expression
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([format_expression(&qualifier, formatter), text(".")])
            }),
        format_creation_new_keyword(expression.new_token().as_ref()),
        expression
            .constructor_type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                concat([format_type_argument_list(&arguments, formatter), text(" ")])
            }),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_argument_list_for_parent_role(
            expression.arguments(),
            Expression::from(expression.clone()).parent_role(),
            formatter,
        ),
        expression.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([
                text(" "),
                jolt_fmt_ir::dedent(format_anonymous_class_body(&body, formatter)),
            ])
        }),
    ]))
}

fn format_array_creation_expression(
    expression: &ArrayCreationExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        format_creation_new_keyword(expression.new_token().as_ref()),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        concat(
            expression
                .dimensions()
                .map(|dimension| format_dim_expression(&dimension, formatter)),
        ),
        expression
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                concat([
                    text(" "),
                    jolt_fmt_ir::dedent(format_array_initializer(&initializer, formatter)),
                ])
            }),
    ]))
}

fn format_creation_new_keyword(keyword: Option<&JavaSyntaxToken>) -> Doc {
    keyword.map_or_else(
        || text("new "),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                text("new"),
                format_trailing_comments_before_line_break(keyword),
                if trailing_comments_force_line(keyword) {
                    hard_line()
                } else {
                    text(" ")
                },
            ])
        },
    )
}

fn format_dim_expression(dimension: &DimExpression, formatter: &JavaFormatter<'_>) -> Doc {
    let open_bracket = dimension.open_bracket();
    let close_bracket = dimension.close_bracket();

    format_bracketed_expression(
        open_bracket.as_ref(),
        dimension
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        close_bracket.as_ref(),
    )
}

fn format_bracketed_expression(
    open: Option<&JavaSyntaxToken>,
    expression: Doc,
    close: Option<&JavaSyntaxToken>,
) -> Doc {
    group(concat([
        format_open_bracket(open),
        indent(concat([format_open_bracket_spacing(open), expression])),
        format_close_bracket_with_spacing(close),
    ]))
}

fn format_open_bracket(open: Option<&JavaSyntaxToken>) -> Doc {
    open.map_or_else(
        || text("["),
        |open| concat([format_leading_comments(open), text("[")]),
    )
}

fn format_open_bracket_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_close_bracket_with_spacing(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            jolt_fmt_ir::nil()
        },
        close.map_or_else(
            || text("]"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text("]"),
                    format_trailing_comments(close),
                ])
            },
        ),
    ])
}

fn format_array_initializer(initializer: &ArrayInitializer, formatter: &JavaFormatter<'_>) -> Doc {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    braced_comma_list_with_trailing_separator(
        open.as_ref(),
        close.as_ref(),
        initializer
            .entries()
            .map(|entry| CommaListItem {
                doc: format_variable_initializer_value(entry.value, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(crate) fn format_variable_initializer_value(
    value: VariableInitializerValue,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::NameExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ThisExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::SuperExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ParenthesizedExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ClassLiteralExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::FieldAccessExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayAccessExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::MethodInvocationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::MethodReferenceExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ObjectCreationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayCreationExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::AssignmentExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ConditionalExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::InstanceofExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::BinaryExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::UnaryExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::PostfixExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::CastExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::LambdaExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::SwitchExpression(expression) => {
            format_expression(&expression.into(), formatter)
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(&initializer, formatter)
        }
    }
}

fn format_cast_expression(expression: &CastExpression, formatter: &JavaFormatter<'_>) -> Doc {
    let open_paren = expression.open_paren();
    let close_paren = expression.close_paren();

    concat([
        format_cast_open_paren(open_paren.as_ref()),
        format_cast_open_paren_spacing(open_paren.as_ref()),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_cast_close_paren(close_paren.as_ref()),
        if close_paren
            .as_ref()
            .is_some_and(trailing_comments_force_line)
        {
            jolt_fmt_ir::nil()
        } else {
            text(" ")
        },
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
    ])
}

fn format_cast_open_paren(open: Option<&JavaSyntaxToken>) -> Doc {
    open.map_or_else(
        || text("("),
        |open| concat([format_leading_comments(open), text("(")]),
    )
}

fn format_cast_open_paren_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_cast_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            jolt_fmt_ir::nil()
        },
        close.map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments(close),
                ])
            },
        ),
    ])
}

fn format_instanceof_expression(
    expression: &InstanceofExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        text(" "),
        expression.instanceof_token().map_or_else(
            || text("instanceof "),
            |token| format_instanceof_operator(&token),
        ),
        expression.ty().map_or_else(
            || {
                expression
                    .pattern()
                    .map_or_else(jolt_fmt_ir::nil, |pattern| {
                        format_pattern(&pattern, formatter)
                    })
            },
            |ty| format_type(&ty, formatter),
        ),
    ])
}

fn format_instanceof_operator(operator: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(operator),
        text("instanceof"),
        format_trailing_comments_before_line_break(operator),
        if trailing_comments_force_line(operator) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_method_invocation_callee(
    expression: &MethodInvocationExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if let Some(name) = expression.direct_method_name() {
        let dot = expression.dot_token();
        return concat([
            expression
                .qualifier()
                .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                    concat([
                        format_expression(&qualifier, formatter),
                        format_member_dot(dot.as_ref()),
                    ])
                }),
            expression
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    format_type_argument_list(&arguments, formatter)
                }),
            format_leaf_token(&name, leading_comments),
        ]);
    }

    expression
        .simple_name_expression()
        .map_or_else(jolt_fmt_ir::nil, |name| {
            format_expression_with_leading_comments(&name, leading_comments, formatter)
        })
}

pub(crate) fn format_argument_list(
    arguments: Option<ArgumentList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(arguments) = arguments else {
        return text("()");
    };
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_expression(&entry.argument, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_argument_list_for_parent_role(
    arguments: Option<ArgumentList>,
    parent_role: Option<ExpressionParentRole>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let arguments = format_argument_list(arguments, formatter);
    if parent_role_has_continuation_indent(parent_role) {
        jolt_fmt_ir::dedent(arguments)
    } else {
        arguments
    }
}

const fn parent_role_has_continuation_indent(parent_role: Option<ExpressionParentRole>) -> bool {
    matches!(
        parent_role,
        Some(
            ExpressionParentRole::AssignmentRight
                | ExpressionParentRole::ReturnValue
                | ExpressionParentRole::ThrowValue
                | ExpressionParentRole::YieldValue
                | ExpressionParentRole::VariableInitializer
        )
    )
}

fn format_lambda_expression(expression: &LambdaExpression, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        format_lambda_parameters(expression, formatter),
        format_lambda_arrow(expression),
        expression.expression_body().map_or_else(
            || {
                expression
                    .block_body()
                    .map_or_else(jolt_fmt_ir::nil, |block| {
                        jolt_fmt_ir::dedent(format_block(&block, formatter))
                    })
            },
            |body| format_expression(&body, formatter),
        ),
    ])
}

fn format_lambda_arrow(expression: &LambdaExpression) -> Doc {
    let Some(arrow) = expression.arrow() else {
        return text(" -> ");
    };

    if arrow.leading_comments().is_empty() && arrow.trailing_comments().is_empty() {
        return text(" -> ");
    }

    let trailing_comments = arrow.trailing_comments();
    let forced_line = trailing_comments.iter().any(comment_forces_line);

    concat([
        text(" "),
        format_leading_comments(&arrow),
        text("->"),
        format_trailing_comments_before_line_break(&arrow),
        if forced_line { hard_line() } else { text(" ") },
    ])
}

fn format_lambda_parameters(expression: &LambdaExpression, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(parameter) = expression.concise_parameter()
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        let tokens = parameter.tokens();
        if tokens_have_comments(&tokens) {
            return format_lambda_parameter(&parameter, formatter);
        }
        return parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text()));
    }

    let parameters = expression
        .parameters()
        .map(|parameters| parameters.parameters().collect::<Vec<_>>())
        .unwrap_or_default();

    if let [parameter] = parameters.as_slice()
        && is_simple_untyped_lambda_parameter(parameter)
    {
        let tokens = parameter.tokens();
        if tokens_have_comments(&tokens) {
            return format_lambda_parameter(parameter, formatter);
        }
        return parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text()));
    }

    concat([
        text("("),
        jolt_fmt_ir::join(
            text(", "),
            parameters
                .into_iter()
                .map(|parameter| format_lambda_parameter(&parameter, formatter)),
        ),
        text(")"),
    ])
}

fn format_switch_expression(expression: &SwitchExpression, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        text("switch ("),
        expression
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| {
                format_expression(&selector, formatter)
            }),
        text(") "),
        expression.block().map_or_else(
            || text("{}"),
            |block| format_switch_block(&block, formatter),
        ),
    ])
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter) -> bool {
    parameter.ty().is_none()
        && parameter.var_token().is_none()
        && !parameter.is_variable_arity()
        && parameter.prefix_annotations().next().is_none()
        && parameter.varargs_annotations().next().is_none()
        && parameter.modifier_tokens().next().is_none()
}

fn format_lambda_parameter(parameter: &LambdaParameter, formatter: &JavaFormatter<'_>) -> Doc {
    let prefix_annotations = parameter
        .prefix_annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let modifier_tokens = parameter.modifier_tokens().collect::<Vec<_>>();
    let has_inline_prefix = !prefix_annotations.is_empty() || !modifier_tokens.is_empty();
    let prefix = inline_modifier_prefix_from_docs(prefix_annotations, modifier_tokens);
    let ty = parameter.ty();
    let var_token = parameter.var_token();
    let has_type_prefix = ty.is_some() || var_token.is_some();
    let varargs_annotations = parameter
        .varargs_annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let ty = ty.map_or_else(
        || var_token.map_or_else(jolt_fmt_ir::nil, |token| format_token_text(token.text())),
        |ty| format_type(&ty, formatter),
    );
    let name = parameter
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));

    if !has_inline_prefix && !has_type_prefix {
        return name;
    }
    if !has_type_prefix {
        return concat([prefix, name]);
    }

    concat([
        prefix,
        ty,
        if parameter.is_variable_arity() {
            if varargs_annotations.is_empty() {
                text("... ")
            } else {
                concat([
                    text(" "),
                    inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                    text("... "),
                ])
            }
        } else {
            text(" ")
        },
        name,
    ])
}

fn format_member_chain(chain: &MemberChain, formatter: &JavaFormatter<'_>) -> Doc {
    let keep_first_suffix_with_root = is_simple_member_chain_root(chain.root());
    concat([
        format_expression_leading_comments(chain.root()),
        member_chain(
            format_expression_with_leading_comments(
                chain.root(),
                LeadingComments::SuppressFirstToken,
                formatter,
            ),
            chain
                .suffixes()
                .iter()
                .map(|suffix| format_member_chain_suffix(suffix, formatter))
                .collect(),
            keep_first_suffix_with_root,
        ),
    ])
}

fn format_expression_leading_comments(expression: &Expression) -> Doc {
    expression
        .tokens()
        .first()
        .map_or_else(jolt_fmt_ir::nil, format_leading_comments)
}

fn format_member_chain_suffix(suffix: &MemberChainSuffix, formatter: &JavaFormatter<'_>) -> Doc {
    match suffix {
        MemberChainSuffix::FieldAccess(access) => {
            let dot = access.dot_token();
            concat([
                format_member_dot(dot.as_ref()),
                access
                    .field_name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
                access
                    .type_arguments()
                    .map_or_else(jolt_fmt_ir::nil, |arguments| {
                        format_type_argument_list(&arguments, formatter)
                    }),
            ])
        }
        MemberChainSuffix::MethodInvocation(invocation) => {
            let dot = invocation.dot_token();
            concat([
                format_member_dot(dot.as_ref()),
                invocation
                    .type_arguments()
                    .map_or_else(jolt_fmt_ir::nil, |arguments| {
                        format_type_argument_list(&arguments, formatter)
                    }),
                invocation
                    .direct_method_name()
                    .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
                format_argument_list(invocation.arguments(), formatter),
            ])
        }
    }
}

fn format_member_dot(dot: Option<&JavaSyntaxToken>) -> Doc {
    dot.map_or_else(
        || text("."),
        |dot| {
            concat([
                format_leading_comments(dot),
                text("."),
                format_trailing_comments_before_line_break(dot),
                if trailing_comments_force_line(dot) {
                    hard_line()
                } else if dot.trailing_comments().is_empty() {
                    jolt_fmt_ir::nil()
                } else {
                    text(" ")
                },
            ])
        },
    )
}

const fn is_simple_member_chain_root(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_)
            | Expression::ThisExpression(_)
            | Expression::SuperExpression(_)
            | Expression::ClassLiteralExpression(_)
    )
}

fn is_member_chain_child(expression: &Expression) -> bool {
    matches!(
        expression.parent_role(),
        Some(
            ExpressionParentRole::FieldAccessReceiver
                | ExpressionParentRole::MethodInvocationQualifier
        )
    )
}

fn flatten_binary_expression(
    expression: &BinaryExpression,
    formatter: &JavaFormatter<'_>,
) -> (Expression, Vec<(Doc, Doc)>) {
    let Some(operator) = expression.operator() else {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            expression
                .right()
                .map(|right| (jolt_fmt_ir::nil(), format_expression(&right, formatter)))
                .into_iter()
                .collect(),
        );
    };
    let operator_text = operator.text();
    if !is_flattenable_binary_operator(operator_text) {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            vec![(
                format_token_with_comments(&operator),
                expression.right().map_or_else(jolt_fmt_ir::nil, |right| {
                    format_expression(&right, formatter)
                }),
            )],
        );
    }

    let mut operands = Vec::new();
    let root = Expression::from(expression.clone());
    if binary_operator_comments_in_tree(&root, operator_text) {
        return (
            expression.left().unwrap_or_else(|| root.clone()),
            expression
                .right()
                .map(|right| {
                    (
                        format_token_with_comments(&operator),
                        format_expression(&right, formatter),
                    )
                })
                .into_iter()
                .collect(),
        );
    }

    collect_binary_operands(&root, operator_text, &mut operands);
    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operands
        .map(|operand| {
            (
                format_token_with_comments(&operator),
                format_expression(&operand, formatter),
            )
        })
        .collect();

    (first, rest)
}

fn collect_binary_operands(
    expression: &Expression,
    operator: &str,
    operands: &mut Vec<Expression>,
) {
    if let Expression::BinaryExpression(binary) = expression
        && binary
            .operator()
            .is_some_and(|token| token.text() == operator)
    {
        if let Some(left) = binary.left() {
            collect_binary_operands(&left, operator, operands);
        }
        if let Some(right) = binary.right() {
            collect_binary_operands(&right, operator, operands);
        }
        return;
    }

    operands.push(expression.clone());
}

fn binary_operator_comments_in_tree(expression: &Expression, operator: &str) -> bool {
    if let Expression::BinaryExpression(binary) = expression
        && binary
            .operator()
            .is_some_and(|token| token.text() == operator)
    {
        if binary
            .operator()
            .is_some_and(|token| token_has_comments(&token))
        {
            return true;
        }
        return binary
            .left()
            .is_some_and(|left| binary_operator_comments_in_tree(&left, operator))
            || binary
                .right()
                .is_some_and(|right| binary_operator_comments_in_tree(&right, operator));
    }

    false
}

const fn is_flattenable_binary_operator(operator: &str) -> bool {
    matches!(operator.as_bytes(), b"&&" | b"||")
}
