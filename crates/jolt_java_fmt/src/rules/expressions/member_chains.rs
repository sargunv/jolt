use super::{
    Doc, Expression, ExpressionParentRole, FieldAccessExpression, JavaFormatter, JavaSyntaxToken,
    LeadingComments, LeadingTrivia, MethodInvocationExpression, TrailingTrivia, concat,
    format_argument_list, format_expression_with_leading_comments, format_leading_comments,
    format_token, format_token_with_comments, format_type_argument_list, group, hard_line, indent,
    soft_line, trailing_comments_force_line,
};
use jolt_fmt_ir::space;

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    units: Vec<Doc<'source>>,
    field_run: Vec<Doc<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn finish(mut self, formatter: &JavaFormatter<'_>) -> Option<Doc<'source>> {
        self.flush_field_run();
        let root = self.root?;
        let keep_first_suffix_with_root = is_simple_member_chain_root(&root);

        Some(concat([
            format_expression_leading_comments(&root),
            member_chain(
                format_expression_with_leading_comments(
                    &root,
                    LeadingComments::SuppressFirstToken,
                    formatter,
                ),
                self.units,
                keep_first_suffix_with_root,
            ),
        ]))
    }

    fn push_field_access(
        &mut self,
        access: &FieldAccessExpression<'source>,
        formatter: &JavaFormatter<'_>,
    ) {
        self.field_run
            .push(format_field_access_suffix(access, formatter));
    }

    fn push_method_invocation(
        &mut self,
        invocation: &MethodInvocationExpression<'source>,
        formatter: &JavaFormatter<'_>,
    ) {
        self.flush_field_run();
        self.units
            .push(format_method_invocation_suffix(invocation, formatter));
    }

    fn flush_field_run(&mut self) {
        if self.field_run.is_empty() {
            return;
        }

        self.units.push(concat(std::mem::take(&mut self.field_run)));
    }
}

pub(super) fn format_member_chain<'source>(
    expression: Expression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut builder = MemberChainBuilder {
        root: None,
        units: Vec::new(),
        field_run: Vec::new(),
    };

    append_chain_expression(&mut builder, expression, formatter)?;
    builder.finish(formatter)
}

fn append_chain_expression<'source>(
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<()> {
    match expression {
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            append_chain_receiver(builder, receiver, formatter);
            builder.push_field_access(&access, formatter);
            Some(())
        }
        Expression::MethodInvocationExpression(invocation) => {
            invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            append_chain_receiver(builder, qualifier, formatter);
            builder.push_method_invocation(&invocation, formatter);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
    formatter: &JavaFormatter<'_>,
) {
    if append_chain_expression(builder, receiver, formatter).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    root: Doc<'source>,
    units: Vec<Doc<'source>>,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if units.is_empty() {
        return root;
    }

    let mut suffixes = units.into_iter();
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

fn format_expression_leading_comments<'source>(expression: &Expression<'source>) -> Doc<'source> {
    expression
        .first_token()
        .map_or_else(jolt_fmt_ir::nil, |token| format_leading_comments(&token))
}

fn format_field_access_suffix<'source>(
    access: &FieldAccessExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_method_invocation_suffix<'source>(
    invocation: &MethodInvocationExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

pub(super) fn format_member_dot<'source>(dot: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    dot.map_or_else(jolt_fmt_ir::nil, |dot| {
        concat([
            format_token(
                dot,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if trailing_comments_force_line(dot) {
                hard_line()
            } else if dot.trailing_comments().is_empty() {
                jolt_fmt_ir::nil()
            } else {
                space()
            },
        ])
    })
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
