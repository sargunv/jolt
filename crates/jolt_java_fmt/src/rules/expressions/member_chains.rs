use super::{
    Doc, Expression, ExpressionParentRole, JavaFormatter, JavaSyntaxToken, LeadingComments,
    MemberChain, MemberChainSuffix, concat, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type_argument_list, hard_line, member_chain,
    text, trailing_comments_force_line,
};

pub(super) fn format_member_chain(chain: &MemberChain, formatter: &JavaFormatter<'_>) -> Doc {
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

pub(super) fn format_member_dot(dot: Option<&JavaSyntaxToken>) -> Doc {
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

pub(super) fn is_member_chain_child(expression: &Expression) -> bool {
    matches!(
        expression.parent_role(),
        Some(
            ExpressionParentRole::FieldAccessReceiver
                | ExpressionParentRole::MethodInvocationQualifier
        )
    )
}
