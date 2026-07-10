use super::{
    Doc, Expression, ExpressionParentRole, FieldAccessExpression, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, MethodInvocationExpression, TrailingTrivia, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token,
    format_token_with_comments, format_type_argument_list, trailing_comments_force_line,
};
use jolt_fmt_ir::{ConcatBuilder, DocBuilder};

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    first_suffix: Option<Doc<'source>>,
    suffix_count: u32,
    field_run: Option<Doc<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn push_field_access(
        &mut self,
        access: &FieldAccessExpression<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        let suffix = format_field_access_suffix(access, rest_suffixes);
        self.field_run = Some(match self.field_run.take() {
            Some(run) => doc_concat!(rest_suffixes, [run, suffix]),
            None => suffix,
        });
    }

    fn push_method_invocation(
        &mut self,
        invocation: &MethodInvocationExpression<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        self.flush_field_run(rest_suffixes);
        let suffix = format_method_invocation_suffix(invocation, rest_suffixes);
        self.push_suffix(suffix, rest_suffixes);
    }

    fn flush_field_run(&mut self, rest_suffixes: &mut ConcatBuilder<'_, 'source>) {
        if let Some(run) = self.field_run.take() {
            self.push_suffix(run, rest_suffixes);
        }
    }

    fn push_suffix(
        &mut self,
        suffix: Doc<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        if self.suffix_count == 0 {
            self.first_suffix = Some(suffix);
        } else {
            let line = rest_suffixes.soft_line();
            rest_suffixes.push(line);
            rest_suffixes.push(suffix);
        }
        self.suffix_count += 1;
    }
}

pub(super) fn format_member_chain<'source>(
    expression: Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut chain = None;
    let rest_suffixes = doc.concat_list(|rest_suffixes| {
        let mut builder = MemberChainBuilder {
            root: None,
            first_suffix: None,
            suffix_count: 0,
            field_run: None,
        };
        if append_chain_expression(&mut builder, expression, rest_suffixes).is_some() {
            builder.flush_field_run(rest_suffixes);
            chain = Some((builder.root, builder.first_suffix, builder.suffix_count));
        }
    });
    let (root, first_suffix, suffix_count) = chain?;
    let root = root?;
    let keep_first_suffix_with_root = is_simple_member_chain_root(&root);
    let leading_comments = format_expression_leading_comments(&root, doc);
    let root_doc =
        format_expression_with_leading_comments(&root, LeadingComments::SuppressFirstToken, doc);
    let chain = member_chain(
        doc,
        root_doc,
        first_suffix,
        rest_suffixes,
        suffix_count,
        keep_first_suffix_with_root,
    );

    Some(doc_concat!(doc, [leading_comments, chain]))
}

fn append_chain_expression<'source>(
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
    rest_suffixes: &mut ConcatBuilder<'_, 'source>,
) -> Option<()> {
    match expression {
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            append_chain_receiver(builder, receiver, rest_suffixes);
            builder.push_field_access(&access, rest_suffixes);
            Some(())
        }
        Expression::MethodInvocationExpression(invocation) => {
            invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            append_chain_receiver(builder, qualifier, rest_suffixes);
            builder.push_method_invocation(&invocation, rest_suffixes);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
    rest_suffixes: &mut ConcatBuilder<'_, 'source>,
) {
    if append_chain_expression(builder, receiver, rest_suffixes).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Doc<'source>,
    first_suffix: Option<Doc<'source>>,
    rest_suffixes: Doc<'source>,
    suffix_count: u32,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if suffix_count == 0 {
        return root;
    }

    let first_suffix = first_suffix.expect("member chain suffix exists");
    let head = if keep_first_suffix_with_root {
        doc_concat!(doc, [root, first_suffix])
    } else {
        root
    };
    let rest = if keep_first_suffix_with_root {
        rest_suffixes
    } else {
        doc_concat!(doc, [doc.soft_line(), first_suffix, rest_suffixes])
    };

    if keep_first_suffix_with_root && suffix_count == 1 {
        return doc_group!(doc, head);
    }

    doc_group!(doc, doc_concat!(doc, [head, doc_indent!(doc, rest)]))
}

fn format_expression_leading_comments<'source>(
    expression: &Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    expression
        .first_token()
        .map_or_else(Doc::nil, |token| format_leading_comments(doc, &token))
}

fn format_field_access_suffix<'source>(
    access: &FieldAccessExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let dot = access.dot_token();
    doc_concat!(
        doc,
        [
            format_member_dot(dot.as_ref(), doc),
            access
                .field_name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            access
                .type_arguments()
                .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                    &arguments, doc
                ),),
        ]
    )
}

fn format_method_invocation_suffix<'source>(
    invocation: &MethodInvocationExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let dot = invocation.dot_token();
    doc_concat!(
        doc,
        [
            format_member_dot(dot.as_ref(), doc),
            invocation
                .type_arguments()
                .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                    &arguments, doc
                ),),
            invocation
                .direct_method_name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            format_argument_list(invocation.arguments(), doc),
        ]
    )
}

pub(super) fn format_member_dot<'source>(
    dot: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    dot.map_or_else(Doc::nil, |dot| {
        doc_concat!(
            doc,
            [
                format_token(
                    doc,
                    dot,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(dot) {
                    doc.hard_line()
                } else if dot.trailing_comments().is_empty() {
                    Doc::nil()
                } else {
                    doc.space()
                },
            ]
        )
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
