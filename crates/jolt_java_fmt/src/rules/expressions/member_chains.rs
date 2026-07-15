use super::calls::format_qualified_invocation_name;
use super::{
    Doc, Expression, ExpressionParentRole, FieldAccessExpression, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, MethodInvocationExpression, TrailingTrivia, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token,
    format_token_with_comments, format_type_argument_list, trailing_comments_force_line,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use jolt_fmt_ir::{ConcatBuilder, DocBuilder};
use jolt_java_syntax::{JavaSyntaxField, QualifiedMethodInvocation};

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
    let first_suffix = first_suffix?;
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
            let receiver = present(access.receiver())?;
            append_chain_receiver(builder, receiver, rest_suffixes);
            builder.push_field_access(&access, rest_suffixes);
            Some(())
        }
        Expression::MethodInvocationExpression(invocation) => {
            let qualified = qualified_invocation(&invocation)?;
            let receiver = present(qualified.receiver())?;
            append_chain_receiver(builder, receiver, rest_suffixes);
            builder.push_method_invocation(&invocation, rest_suffixes);
            Some(())
        }
        _ => None,
    }
}

fn present<T>(
    field: Result<JavaSyntaxField<'_, T>, jolt_java_syntax::JavaSyntaxInvariantError>,
) -> Option<T> {
    match field.ok()? {
        JavaSyntaxField::Present(value) => Some(value),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

fn qualified_invocation<'source>(
    invocation: &MethodInvocationExpression<'source>,
) -> Option<QualifiedMethodInvocation<'source>> {
    present(invocation.form())?.cast_node()
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
    first_suffix: Doc<'source>,
    rest_suffixes: Doc<'source>,
    suffix_count: u32,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if suffix_count == 0 {
        return root;
    }

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
    doc_concat!(
        doc,
        [
            format_required_field(access.dot(), doc, |dot, doc| {
                format_member_dot(Some(&dot), doc)
            }),
            format_required_field(access.name(), doc, |name, doc| {
                format_token_with_comments(doc, &name)
            }),
            format_optional_field(access.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
        ]
    )
}

fn format_method_invocation_suffix<'source>(
    invocation: &MethodInvocationExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(invocation) = qualified_invocation(invocation) else {
        doc.block_on_invariant("member-chain invocation was not qualified");
        return Doc::nil();
    };
    doc_concat!(
        doc,
        [
            format_required_field(invocation.dot(), doc, |dot, doc| {
                format_member_dot(Some(&dot), doc)
            }),
            format_optional_field(invocation.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
            format_required_field(invocation.name(), doc, |name, doc| {
                format_qualified_invocation_name(name, LeadingComments::Preserve, doc)
            }),
            format_required_field(invocation.arguments(), doc, |arguments, doc| {
                format_argument_list(Some(arguments), doc)
            }),
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
