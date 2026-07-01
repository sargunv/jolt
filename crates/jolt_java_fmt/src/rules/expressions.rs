use jolt_fmt_ir::{
    Doc, concat, force_group, group, hard_line, if_break, indent, line, soft_line, text,
};
use jolt_java_syntax::{
    ArgumentList, ArgumentListEntry, ArrayAccessExpression, ArrayCreationExpression,
    ArrayInitializer, ArrayInitializerEntry, AssignmentExpression, BinaryExpression,
    CastExpression, ClassLiteralExpression, ConditionalExpression, DimExpression, Expression,
    ExpressionParentRole, FieldAccessExpression, InstanceofExpression, JavaComment,
    JavaSyntaxToken, LambdaExpression, LambdaParameter, LiteralExpression, MemberChain,
    MemberChainSuffix, MethodInvocationExpression, MethodReferenceExpression, NameExpression,
    ObjectCreationExpression, ParenthesizedExpression, PostfixExpression, SuperExpression,
    SwitchExpression, ThisExpression, UnaryExpression, VariableInitializerValue,
};

use crate::helpers::chains::member_chain;
use crate::helpers::comments::{
    comment_forces_line, format_comment, format_dangling_comments, format_leading_comments,
    format_token_text, format_token_with_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, tokens_have_comments, trailing_comments_force_line,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::helpers::operators::{assignment_expression, binary_chain, ternary_expression};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_anonymous_class_body;
use crate::rules::patterns::format_pattern;
use crate::rules::statements::{format_block, format_switch_block};
use crate::rules::types::{format_array_dimensions, format_type, format_type_argument_list};

pub(crate) fn format_expression(expression: &Expression) -> Doc {
    format_expression_with_leading_comments(expression, LeadingComments::Preserve)
}

fn format_expression_with_leading_comments(
    expression: &Expression,
    leading_comments: LeadingComments,
) -> Doc {
    match expression {
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(expression)
        }
        Expression::AssignmentExpression(expression) => format_assignment_expression(expression),
        Expression::ConditionalExpression(expression) => format_conditional_expression(expression),
        Expression::BinaryExpression(expression) => format_binary_expression(expression),
        Expression::UnaryExpression(expression) => format_unary_expression(expression),
        Expression::PostfixExpression(expression) => format_postfix_expression(expression),
        Expression::LambdaExpression(expression) => format_lambda_expression(expression),
        Expression::LiteralExpression(expression) => {
            format_literal_expression(expression, leading_comments)
        }
        Expression::NameExpression(expression) => {
            format_name_expression(expression, leading_comments)
        }
        Expression::ThisExpression(expression) => {
            format_this_expression(expression, leading_comments)
        }
        Expression::SuperExpression(expression) => {
            format_super_expression(expression, leading_comments)
        }
        Expression::ClassLiteralExpression(expression) => {
            format_class_literal_expression(expression)
        }
        Expression::MethodReferenceExpression(expression) => {
            format_method_reference_expression(expression)
        }
        Expression::SwitchExpression(expression) => format_switch_expression(expression),
        Expression::ArrayCreationExpression(expression) => {
            format_array_creation_expression(expression)
        }
        Expression::InstanceofExpression(expression) => format_instanceof_expression(expression),
        Expression::CastExpression(expression) => format_cast_expression(expression),
        Expression::FieldAccessExpression(expression) => format_field_access_expression(expression),
        Expression::ArrayAccessExpression(expression) => format_array_access_expression(expression),
        Expression::MethodInvocationExpression(expression) => {
            format_method_invocation_expression_with_leading_comments(expression, leading_comments)
        }
        Expression::ObjectCreationExpression(expression) => {
            format_object_creation_expression(expression)
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

fn format_name_expression(expression: &NameExpression, leading_comments: LeadingComments) -> Doc {
    let annotations = expression
        .annotations()
        .map(|annotation| format_annotation(&annotation))
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

fn format_this_expression(expression: &ThisExpression, leading_comments: LeadingComments) -> Doc {
    format_qualified_keyword_expression(
        expression.qualifier(),
        expression.keyword().map_or_else(
            || text("this"),
            |token| format_leaf_token(&token, leading_comments),
        ),
    )
}

fn format_super_expression(expression: &SuperExpression, leading_comments: LeadingComments) -> Doc {
    format_qualified_keyword_expression(
        expression.qualifier(),
        expression.keyword().map_or_else(
            || text("super"),
            |token| format_leaf_token(&token, leading_comments),
        ),
    )
}

fn format_qualified_keyword_expression(qualifier: Option<Expression>, keyword: Doc) -> Doc {
    match qualifier {
        Some(qualifier) => concat([format_expression(&qualifier), text("."), keyword]),
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

fn format_class_literal_expression(expression: &ClassLiteralExpression) -> Doc {
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
                |ty| crate::rules::types::format_void_type(&ty),
            )
        },
        |target| format_expression(&target),
    );

    concat([
        target,
        expression
            .dimensions()
            .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                format_array_dimensions(&dimensions)
            }),
        text(".class"),
    ])
}

fn format_parenthesized_expression(expression: &ParenthesizedExpression) -> Doc {
    group(concat([
        format_parenthesized_expression_open(expression),
        indent(concat([
            format_open_parenthesized_expression_spacing(expression),
            expression
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression)
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

fn format_assignment_expression(expression: &AssignmentExpression) -> Doc {
    assignment_expression(
        expression
            .left()
            .map_or_else(jolt_fmt_ir::nil, |left| format_expression(&left)),
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression
            .right()
            .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
    )
}

fn format_conditional_expression(expression: &ConditionalExpression) -> Doc {
    ternary_expression(
        expression
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        expression
            .question_token()
            .map_or_else(|| text("?"), |token| format_token_with_comments(&token)),
        expression
            .true_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        expression
            .colon_token()
            .map_or_else(|| text(":"), |token| format_token_with_comments(&token)),
        expression
            .false_expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    )
}

fn format_binary_expression(expression: &BinaryExpression) -> Doc {
    let (first, rest) = flatten_binary_expression(expression);
    binary_chain(format_expression(&first), rest)
}

fn format_unary_expression(expression: &UnaryExpression) -> Doc {
    concat([
        expression
            .operator()
            .map_or_else(jolt_fmt_ir::nil, |operator| {
                format_token_with_comments(&operator)
            }),
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| format_expression(&operand)),
    ])
}

fn format_postfix_expression(expression: &PostfixExpression) -> Doc {
    concat([
        expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| format_expression(&operand)),
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
) -> Doc {
    let expression = Expression::from(expression.clone());
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain);
    }
    let Expression::MethodInvocationExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    group(concat([
        format_method_invocation_callee(&expression, leading_comments),
        format_argument_list(expression.arguments()),
    ]))
}

fn format_field_access_expression(expression: &FieldAccessExpression) -> Doc {
    let expression = Expression::from(expression.clone());
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain);
    }
    let Expression::FieldAccessExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| format_expression(&receiver)),
        text("."),
        text(
            expression
                .field_name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        ),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
            }),
    ]))
}

fn format_method_reference_expression(expression: &MethodReferenceExpression) -> Doc {
    group(concat([
        format_method_reference_receiver(expression),
        format_method_reference_separator(expression),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
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

fn format_method_reference_receiver(expression: &MethodReferenceExpression) -> Doc {
    if let Some(receiver) = expression.receiver_expression() {
        return concat([
            format_expression(&receiver),
            expression
                .receiver_dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions)
                }),
        ]);
    }

    expression
        .receiver_type()
        .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty))
}

fn format_array_access_expression(expression: &ArrayAccessExpression) -> Doc {
    let open_bracket = expression.open_bracket();
    let close_bracket = expression.close_bracket();

    group(concat([
        expression
            .array()
            .map_or_else(jolt_fmt_ir::nil, |array| format_expression(&array)),
        format_bracketed_expression(
            open_bracket.as_ref(),
            expression
                .index()
                .map_or_else(jolt_fmt_ir::nil, |index| format_expression(&index)),
            close_bracket.as_ref(),
        ),
    ]))
}

fn format_object_creation_expression(expression: &ObjectCreationExpression) -> Doc {
    group(concat([
        expression
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([format_expression(&qualifier), text(".")])
            }),
        text("new "),
        expression
            .constructor_type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                concat([format_type_argument_list(&arguments), text(" ")])
            }),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        format_argument_list(expression.arguments()),
        expression.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([
                text(" "),
                jolt_fmt_ir::dedent(format_anonymous_class_body(&body)),
            ])
        }),
    ]))
}

fn format_array_creation_expression(expression: &ArrayCreationExpression) -> Doc {
    group(concat([
        text("new "),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        concat(
            expression
                .dimensions()
                .map(|dimension| format_dim_expression(&dimension)),
        ),
        expression
            .initializer()
            .map_or_else(jolt_fmt_ir::nil, |initializer| {
                concat([
                    text(" "),
                    jolt_fmt_ir::dedent(format_array_initializer(&initializer)),
                ])
            }),
    ]))
}

fn format_dim_expression(dimension: &DimExpression) -> Doc {
    let open_bracket = dimension.open_bracket();
    let close_bracket = dimension.close_bracket();

    format_bracketed_expression(
        open_bracket.as_ref(),
        dimension
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
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

fn format_array_initializer(initializer: &ArrayInitializer) -> Doc {
    let entries = initializer.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return format_empty_array_initializer(initializer);
    }

    let has_dangling_comments = array_initializer_has_dangling_comments(initializer);
    let doc = group(concat([
        format_array_initializer_open(initializer),
        indent(concat([
            format_open_array_initializer_spacing(initializer),
            format_array_initializer_entries(entries),
        ])),
        format_array_initializer_close_with_spacing(initializer),
    ]));

    if has_dangling_comments {
        force_group(doc)
    } else {
        doc
    }
}

fn format_empty_array_initializer(initializer: &ArrayInitializer) -> Doc {
    if !array_initializer_has_dangling_comments(initializer) {
        return concat([
            format_array_initializer_open(initializer),
            format_array_initializer_close_delimiter(initializer),
        ]);
    }

    force_group(concat([
        format_array_initializer_open(initializer),
        indent(concat([
            hard_line(),
            format_array_initializer_dangling_comments(initializer),
        ])),
        hard_line(),
        format_array_initializer_close_delimiter_without_leading(initializer),
    ]))
}

fn array_initializer_has_dangling_comments(initializer: &ArrayInitializer) -> bool {
    initializer
        .open_brace()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || initializer
            .close_brace()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_array_initializer_open(initializer: &ArrayInitializer) -> Doc {
    initializer.open_brace().map_or_else(
        || text("{"),
        |open| concat([format_leading_comments(&open), text("{")]),
    )
}

fn format_open_array_initializer_spacing(initializer: &ArrayInitializer) -> Doc {
    let Some(open) = initializer.open_brace() else {
        return soft_line();
    };

    let comments = open.trailing_comments();
    if comments.is_empty() {
        return soft_line();
    }

    concat([hard_line(), format_dangling_comments(comments), hard_line()])
}

fn format_array_initializer_entries(entries: Vec<ArrayInitializerEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_variable_initializer_value(entry.value));
        if let Some(comma) = entry.comma {
            docs.push(format_array_initializer_separator(
                &comma,
                index + 1 == entries_len,
            ));
        } else if index + 1 < entries_len {
            docs.push(line());
        } else {
            docs.push(if_break(text(","), jolt_fmt_ir::nil()));
        }
    }

    concat(docs)
}

fn format_array_initializer_separator(comma: &JavaSyntaxToken, is_last: bool) -> Doc {
    let trailing_comments = comma.trailing_comments();
    let has_trailing_comments = !trailing_comments.is_empty();
    let force_line = trailing_comments.iter().any(comment_forces_line);

    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if is_last {
            if has_trailing_comments && !force_line {
                text(" ")
            } else {
                jolt_fmt_ir::nil()
            }
        } else if force_line {
            hard_line()
        } else if has_trailing_comments {
            text(" ")
        } else {
            line()
        },
    ])
}

fn format_array_initializer_close_with_spacing(initializer: &ArrayInitializer) -> Doc {
    let close_has_leading_comments = initializer
        .close_brace()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_array_initializer_close_delimiter(initializer),
    ])
}

fn format_array_initializer_close_delimiter(initializer: &ArrayInitializer) -> Doc {
    let close = initializer.close_brace();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text("}"),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(&close)
                } else {
                    jolt_fmt_ir::nil()
                },
                text("}"),
                format_trailing_comments(&close),
            ])
        },
    )
}

fn format_array_initializer_close_delimiter_without_leading(initializer: &ArrayInitializer) -> Doc {
    initializer.close_brace().map_or_else(
        || text("}"),
        |close| concat([text("}"), format_trailing_comments(&close)]),
    )
}

fn format_array_initializer_dangling_comments(initializer: &ArrayInitializer) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = initializer.open_brace() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = initializer.close_brace() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

pub(crate) fn format_variable_initializer_value(value: VariableInitializerValue) -> Doc {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::NameExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ThisExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::SuperExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ParenthesizedExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ClassLiteralExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::FieldAccessExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayAccessExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::MethodInvocationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::MethodReferenceExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ObjectCreationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayCreationExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::AssignmentExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ConditionalExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::InstanceofExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::BinaryExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::UnaryExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::PostfixExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::CastExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::LambdaExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::SwitchExpression(expression) => {
            format_expression(&expression.into())
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(&initializer)
        }
    }
}

fn format_cast_expression(expression: &CastExpression) -> Doc {
    concat([
        text("("),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        text(") "),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
    ])
}

fn format_instanceof_expression(expression: &InstanceofExpression) -> Doc {
    concat([
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(" instanceof "),
        expression.ty().map_or_else(
            || {
                expression
                    .pattern()
                    .map_or_else(jolt_fmt_ir::nil, |pattern| format_pattern(&pattern))
            },
            |ty| format_type(&ty),
        ),
    ])
}

fn format_method_invocation_callee(
    expression: &MethodInvocationExpression,
    leading_comments: LeadingComments,
) -> Doc {
    if let Some(name) = expression.direct_method_name() {
        return concat([
            expression
                .qualifier()
                .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                    concat([format_expression(&qualifier), text(".")])
                }),
            expression
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    format_type_argument_list(&arguments)
                }),
            format_leaf_token(&name, leading_comments),
        ]);
    }

    expression
        .simple_name_expression()
        .map_or_else(jolt_fmt_ir::nil, |name| {
            format_expression_with_leading_comments(&name, leading_comments)
        })
}

pub(crate) fn format_argument_list(arguments: Option<ArgumentList>) -> Doc {
    let Some(arguments) = arguments else {
        return text("()");
    };
    let entries = arguments.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return format_empty_argument_list(&arguments);
    }

    group(concat([
        format_argument_list_open(&arguments),
        indent(concat([
            format_open_argument_list_spacing(&arguments),
            format_argument_list_entries(entries),
        ])),
        format_argument_list_close_with_spacing(&arguments),
    ]))
}

fn format_empty_argument_list(arguments: &ArgumentList) -> Doc {
    if !argument_list_has_dangling_comments(arguments) {
        return concat([
            format_argument_list_open(arguments),
            format_argument_list_close_delimiter(arguments),
        ]);
    }

    force_group(concat([
        format_argument_list_open(arguments),
        indent(concat([
            hard_line(),
            format_argument_list_dangling_comments(arguments),
        ])),
        hard_line(),
        format_argument_list_close_delimiter_without_leading(arguments),
    ]))
}

fn argument_list_has_dangling_comments(arguments: &ArgumentList) -> bool {
    arguments
        .open_paren()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || arguments
            .close_paren()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_argument_list_open(arguments: &ArgumentList) -> Doc {
    arguments.open_paren().map_or_else(
        || text("("),
        |open| concat([format_leading_comments(&open), text("(")]),
    )
}

fn format_open_argument_list_spacing(arguments: &ArgumentList) -> Doc {
    let Some(open) = arguments.open_paren() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if trailing_comments_force_line(&open) {
            hard_line()
        } else {
            soft_line()
        },
    ])
}

fn format_argument_list_entries(entries: Vec<ArgumentListEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_expression(&entry.argument));
        if let Some(comma) = entry.comma {
            docs.push(format_argument_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_argument_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if trailing_comments_force_line(comma) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn format_argument_list_close_with_spacing(arguments: &ArgumentList) -> Doc {
    let close_has_leading_comments = arguments
        .close_paren()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_argument_list_close_delimiter(arguments),
    ])
}

fn format_argument_list_close_delimiter(arguments: &ArgumentList) -> Doc {
    let close = arguments.close_paren();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
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
    )
}

fn format_argument_list_close_delimiter_without_leading(arguments: &ArgumentList) -> Doc {
    arguments.close_paren().map_or_else(
        || text(")"),
        |close| concat([text(")"), format_trailing_comments(&close)]),
    )
}

fn format_argument_list_dangling_comments(arguments: &ArgumentList) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = arguments.open_paren() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = arguments.close_paren() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

fn push_dangling_comments(docs: &mut Vec<Doc>, comments: Vec<JavaComment>) {
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
}

fn format_lambda_expression(expression: &LambdaExpression) -> Doc {
    concat([
        format_lambda_parameters(expression),
        text(" -> "),
        expression.expression_body().map_or_else(
            || {
                expression
                    .block_body()
                    .map_or_else(jolt_fmt_ir::nil, |block| format_block(&block))
            },
            |body| format_expression(&body),
        ),
    ])
}

fn format_lambda_parameters(expression: &LambdaExpression) -> Doc {
    if let Some(parameter) = expression.concise_parameter()
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        let tokens = parameter.tokens();
        if tokens_have_comments(&tokens) {
            return format_lambda_parameter(&parameter);
        }
        return text(
            parameter
                .name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        );
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
            return format_lambda_parameter(parameter);
        }
        return text(
            parameter
                .name()
                .map_or_else(String::new, |name| name.text().to_owned()),
        );
    }

    concat([
        text("("),
        jolt_fmt_ir::join(
            text(", "),
            parameters
                .into_iter()
                .map(|parameter| format_lambda_parameter(&parameter)),
        ),
        text(")"),
    ])
}

fn format_switch_expression(expression: &SwitchExpression) -> Doc {
    concat([
        text("switch ("),
        expression
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| format_expression(&selector)),
        text(") "),
        expression
            .block()
            .map_or_else(|| text("{}"), |block| format_switch_block(&block)),
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

fn format_lambda_parameter(parameter: &LambdaParameter) -> Doc {
    let prefix_annotations = parameter
        .prefix_annotations()
        .map(|annotation| format_annotation(&annotation))
        .collect::<Vec<_>>();
    let modifier_tokens = parameter.modifier_tokens().collect::<Vec<_>>();
    let has_inline_prefix = !prefix_annotations.is_empty() || !modifier_tokens.is_empty();
    let prefix = inline_modifier_prefix_from_docs(prefix_annotations, modifier_tokens);
    let ty = parameter.ty();
    let var_token = parameter.var_token();
    let has_type_prefix = ty.is_some() || var_token.is_some();
    let varargs_annotations = parameter
        .varargs_annotations()
        .map(|annotation| format_annotation(&annotation))
        .collect::<Vec<_>>();
    let ty = ty.map_or_else(
        || var_token.map_or_else(jolt_fmt_ir::nil, |token| text(token.text().to_owned())),
        |ty| format_type(&ty),
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

fn format_member_chain(chain: &MemberChain) -> Doc {
    let keep_first_suffix_with_root = is_simple_member_chain_root(chain.root());
    concat([
        format_expression_leading_comments(chain.root()),
        member_chain(
            format_expression_with_leading_comments(
                chain.root(),
                LeadingComments::SuppressFirstToken,
            ),
            chain
                .suffixes()
                .iter()
                .map(format_member_chain_suffix)
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

fn format_member_chain_suffix(suffix: &MemberChainSuffix) -> Doc {
    match suffix {
        MemberChainSuffix::FieldAccess(access) => concat([
            text("."),
            access
                .field_name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            access
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    format_type_argument_list(&arguments)
                }),
        ]),
        MemberChainSuffix::MethodInvocation(invocation) => concat([
            text("."),
            invocation
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    format_type_argument_list(&arguments)
                }),
            invocation
                .direct_method_name()
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
            format_argument_list(invocation.arguments()),
        ]),
    }
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

fn flatten_binary_expression(expression: &BinaryExpression) -> (Expression, Vec<(Doc, Doc)>) {
    let Some(operator) = expression.operator() else {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            expression
                .right()
                .map(|right| (jolt_fmt_ir::nil(), format_expression(&right)))
                .into_iter()
                .collect(),
        );
    };
    let operator_text = operator.text().to_owned();
    if !is_flattenable_binary_operator(&operator_text) {
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(expression.clone())),
            vec![(
                format_token_with_comments(&operator),
                expression
                    .right()
                    .map_or_else(jolt_fmt_ir::nil, |right| format_expression(&right)),
            )],
        );
    }

    let mut operands = Vec::new();
    let root = Expression::from(expression.clone());
    if binary_operator_comments_in_tree(&root, &operator_text) {
        return (
            expression.left().unwrap_or_else(|| root.clone()),
            expression
                .right()
                .map(|right| {
                    (
                        format_token_with_comments(&operator),
                        format_expression(&right),
                    )
                })
                .into_iter()
                .collect(),
        );
    }

    collect_binary_operands(&root, &operator_text, &mut operands);
    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operands
        .map(|operand| {
            (
                format_token_with_comments(&operator),
                format_expression(&operand),
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
            .is_some_and(|token| tokens_have_comments(&[token]))
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
