use super::calls::format_qualified_invocation_name;
use super::{
    Doc, Expression, ExpressionParentRole, FieldAccessExpression, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, MethodInvocationExpression, TrailingTrivia, format_argument_list,
    format_expression, format_expression_with_leading_comments, format_leading_comments,
    format_token, format_token_with_comments, format_type_argument_list,
    trailing_comments_force_line,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use jolt_fmt_ir::{ConcatBuilder, DocBuilder};
use jolt_java_syntax::{JavaSyntaxField, MethodInvocationFormSyntax, QualifiedMethodInvocation};

type ChainSuffix<'source> = (Doc<'source>, bool);

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    first_suffix: Option<ChainSuffix<'source>>,
    has_rest_suffixes: bool,
    field_run: Option<ChainSuffix<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn push_field_access(
        &mut self,
        access: &FieldAccessExpression<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        let has_leading_comments = required_dot_has_leading_comments(access.dot());
        let suffix = format_field_access_suffix(access, rest_suffixes);
        self.field_run = Some(match self.field_run.take() {
            Some((run, run_has_leading_comments)) => (
                doc_concat!(
                    rest_suffixes,
                    [
                        run,
                        if has_leading_comments {
                            rest_suffixes.line()
                        } else {
                            Doc::nil()
                        },
                        suffix
                    ]
                ),
                run_has_leading_comments,
            ),
            None => (suffix, has_leading_comments),
        });
    }

    fn push_method_invocation(
        &mut self,
        invocation: &QualifiedMethodInvocation<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        self.flush_field_run(rest_suffixes);
        let has_leading_comments = required_dot_has_leading_comments(invocation.dot());
        let suffix = format_method_invocation_suffix(invocation, rest_suffixes);
        self.push_suffix((suffix, has_leading_comments), rest_suffixes);
    }

    fn flush_field_run(&mut self, rest_suffixes: &mut ConcatBuilder<'_, 'source>) {
        if let Some(run) = self.field_run.take() {
            self.push_suffix(run, rest_suffixes);
        }
    }

    fn push_suffix(
        &mut self,
        suffix: ChainSuffix<'source>,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    ) {
        if self.first_suffix.is_none() {
            self.first_suffix = Some(suffix);
        } else {
            self.has_rest_suffixes = true;
            let (suffix, has_leading_comments) = suffix;
            let line = if has_leading_comments {
                rest_suffixes.line()
            } else {
                rest_suffixes.soft_line()
            };
            rest_suffixes.push(line);
            rest_suffixes.push(suffix);
        }
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
            has_rest_suffixes: false,
            field_run: None,
        };
        if append_chain_expression(&mut builder, expression, rest_suffixes).is_some() {
            builder.flush_field_run(rest_suffixes);
            chain = Some((
                builder.root,
                builder.first_suffix,
                builder.has_rest_suffixes,
            ));
        }
    });
    let (root, first_suffix, has_rest_suffixes) = chain?;
    let root = root?;
    let (first_suffix, first_suffix_has_leading_comments) = first_suffix?;
    let keep_first_suffix_with_root =
        is_simple_member_chain_root(&root) && !first_suffix_has_leading_comments;
    let relocate_leading_comments = matches!(
        root,
        Expression::LiteralExpression(_) | Expression::NameExpression(_)
    );
    let leading_comments = if relocate_leading_comments {
        format_expression_leading_comments(&root, doc)
    } else {
        Doc::nil()
    };
    let root_doc = if relocate_leading_comments {
        format_expression_with_leading_comments(&root, LeadingComments::SuppressFirstToken, doc)
    } else {
        format_expression(&root, doc)
    };
    let chain = member_chain(
        doc,
        root_doc,
        first_suffix,
        rest_suffixes,
        has_rest_suffixes,
        keep_first_suffix_with_root,
    );

    Some(doc_concat!(doc, [leading_comments, chain]))
}

fn required_dot_has_leading_comments(dot: JavaSyntaxField<'_, JavaSyntaxToken<'_>>) -> bool {
    matches!(dot, JavaSyntaxField::Present(dot) if !dot.leading_comments().is_empty())
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
            builder.push_method_invocation(&qualified, rest_suffixes);
            Some(())
        }
        _ => None,
    }
}

fn present<T>(field: JavaSyntaxField<'_, T>) -> Option<T> {
    match field {
        JavaSyntaxField::Present(value) => Some(value),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

fn qualified_invocation<'source>(
    invocation: &MethodInvocationExpression<'source>,
) -> Option<QualifiedMethodInvocation<'source>> {
    match present(invocation.form())? {
        MethodInvocationFormSyntax::QualifiedMethodInvocation(invocation) => Some(invocation),
        MethodInvocationFormSyntax::UnqualifiedMethodInvocation(_)
        | MethodInvocationFormSyntax::BogusMethodInvocationForm(_) => None,
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
    first_suffix: Doc<'source>,
    rest_suffixes: Doc<'source>,
    has_rest_suffixes: bool,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
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

    if keep_first_suffix_with_root && !has_rest_suffixes {
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
                format_member_dot(&dot, doc)
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
    invocation: &QualifiedMethodInvocation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(invocation.dot(), doc, |dot, doc| {
                format_member_dot(&dot, doc)
            }),
            format_optional_field(invocation.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
            format_required_field(invocation.name(), doc, |name, doc| {
                format_qualified_invocation_name(name, LeadingComments::Preserve, doc)
            }),
            format_required_field(invocation.arguments(), doc, |arguments, doc| {
                format_argument_list(arguments, doc)
            }),
        ]
    )
}

pub(super) fn format_member_dot<'source>(
    dot: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
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
