use super::{
    Doc, Expression, ExpressionParentRole, FieldAccessExpression, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, MethodInvocationExpression, TrailingTrivia, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token,
    format_token_with_comments, format_type_argument_list, trailing_comments_force_line,
};
use jolt_fmt_ir::{DocBuilder, DocList};

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    first_suffix: Option<Doc<'source>>,
    rest_suffixes: DocList<'source>,
    suffix_count: u32,
    field_run: Option<Doc<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn finish(mut self, doc: &mut DocBuilder<'source>) -> Option<Doc<'source>> {
        self.flush_field_run(doc);
        let root = self.root?;
        let keep_first_suffix_with_root = is_simple_member_chain_root(&root);
        let leading_comments = format_expression_leading_comments(&root, doc);
        let root_doc = format_expression_with_leading_comments(
            &root,
            LeadingComments::SuppressFirstToken,
            doc,
        );
        let chain = member_chain(
            doc,
            root_doc,
            self.first_suffix,
            self.rest_suffixes,
            self.suffix_count,
            keep_first_suffix_with_root,
        );

        Some(doc_concat!(doc, [leading_comments, chain]))
    }

    fn push_field_access(
        &mut self,
        access: &FieldAccessExpression<'source>,
        doc: &mut DocBuilder<'source>,
    ) {
        let suffix = format_field_access_suffix(access, doc);
        self.field_run = Some(match self.field_run.take() {
            Some(run) => doc_concat!(doc, [run, suffix]),
            None => suffix,
        });
    }

    fn push_method_invocation(
        &mut self,
        invocation: &MethodInvocationExpression<'source>,
        doc: &mut DocBuilder<'source>,
    ) {
        self.flush_field_run(doc);
        let suffix = format_method_invocation_suffix(invocation, doc);
        self.push_suffix(suffix, doc);
    }

    fn flush_field_run(&mut self, doc: &mut DocBuilder<'source>) {
        if let Some(run) = self.field_run.take() {
            self.push_suffix(run, doc);
        }
    }

    fn push_suffix(&mut self, suffix: Doc<'source>, doc: &mut DocBuilder<'source>) {
        if self.suffix_count == 0 {
            self.first_suffix = Some(suffix);
        } else {
            let line = doc.soft_line();
            let suffix = doc_concat!(doc, [line, suffix]);
            self.rest_suffixes.push(suffix, doc);
        }
        self.suffix_count += 1;
    }
}

pub(super) fn format_member_chain<'source>(
    expression: Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut builder = MemberChainBuilder {
        root: None,
        first_suffix: None,
        rest_suffixes: doc.list(),
        suffix_count: 0,
        field_run: None,
    };

    append_chain_expression(&mut builder, expression, doc)?;
    builder.finish(doc)
}

fn append_chain_expression<'source>(
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<()> {
    match expression {
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            append_chain_receiver(builder, receiver, doc);
            builder.push_field_access(&access, doc);
            Some(())
        }
        Expression::MethodInvocationExpression(invocation) => {
            invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            append_chain_receiver(builder, qualifier, doc);
            builder.push_method_invocation(&invocation, doc);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
    doc: &mut DocBuilder<'source>,
) {
    if append_chain_expression(builder, receiver, doc).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Doc<'source>,
    first_suffix: Option<Doc<'source>>,
    rest_suffixes: DocList<'source>,
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
        rest_suffixes.finish(doc)
    } else {
        let mut rest = doc.list();
        rest.push(doc_concat!(doc, [doc.soft_line(), first_suffix]), doc);
        rest.push(rest_suffixes.finish(doc), doc);
        rest.finish(doc)
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
