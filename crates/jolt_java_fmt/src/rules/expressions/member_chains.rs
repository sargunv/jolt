use super::{
    Doc, Expression, ExpressionParentRole, JavaFormatter, JavaSyntaxToken, LeadingComments,
    MemberChain, MemberChainSuffix, concat, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type_argument_list, group, hard_line,
    indent, soft_line, text, trailing_comments_force_line,
};

pub(super) fn format_member_chain<'source>(
    chain: &MemberChain<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let keep_first_suffix_with_root = is_simple_member_chain_root(chain.root());
    concat([
        format_expression_leading_comments(chain.root()),
        member_chain(
            format_expression_with_leading_comments(
                chain.root(),
                LeadingComments::SuppressFirstToken,
                formatter,
            ),
            format_member_chain_units(chain.suffixes(), formatter),
            keep_first_suffix_with_root,
        ),
    ])
}

fn format_member_chain_units<'source>(
    suffixes: &[MemberChainSuffix<'source>],
    formatter: &JavaFormatter<'_>,
) -> Vec<Doc<'source>> {
    let mut units = Vec::new();
    let mut field_run = Vec::new();

    for suffix in suffixes {
        match suffix {
            MemberChainSuffix::FieldAccess(_) => {
                field_run.push(format_member_chain_suffix(suffix, formatter));
            }
            MemberChainSuffix::MethodInvocation(_) => {
                flush_field_run(&mut units, &mut field_run);
                units.push(format_member_chain_suffix(suffix, formatter));
            }
        }
    }

    flush_field_run(&mut units, &mut field_run);
    units
}

fn member_chain<'source>(
    root: Doc<'source>,
    suffixes: Vec<Doc<'source>>,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if suffixes.is_empty() {
        return root;
    }

    let mut suffixes = suffixes.into_iter();
    let head = if keep_first_suffix_with_root {
        concat([root, suffixes.next().expect("suffixes is not empty")])
    } else {
        root
    };
    let rest = suffixes
        .map(|suffix| concat([soft_line(), suffix]))
        .collect::<Vec<_>>();

    if rest.is_empty() {
        return group(head);
    }

    group(concat([head, indent(concat(rest))]))
}

fn flush_field_run<'source>(units: &mut Vec<Doc<'source>>, field_run: &mut Vec<Doc<'source>>) {
    if field_run.is_empty() {
        return;
    }

    units.push(concat(std::mem::take(field_run)));
}

fn format_expression_leading_comments<'source>(expression: &Expression<'source>) -> Doc<'source> {
    expression
        .first_token()
        .map_or_else(jolt_fmt_ir::nil, |token| format_leading_comments(&token))
}

fn format_member_chain_suffix<'source>(
    suffix: &MemberChainSuffix<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(super) fn format_member_dot<'source>(dot: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
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
