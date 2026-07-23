use super::arrays_objects::{format_array_access_expression, format_object_creation_expression};
use super::calls::{
    format_field_access_expression, format_method_invocation_expression_with_leading_comments,
    format_qualified_invocation_name,
};
use super::leaves::{format_super_expression, format_this_expression};
use super::operators::format_postfix_expression;
use super::{
    Doc, Expression, FieldAccessExpression, JavaSyntaxToken, LeadingComments, LeadingTrivia,
    MethodInvocationExpression, TrailingTrivia, format_argument_list,
    format_expression_with_leading_comments, format_leading_comments, format_token,
    format_token_with_comments, format_type_argument_list, trailing_comments_force_line,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use jolt_fmt_ir::{ConcatBuilder, DocBuilder};
use jolt_java_syntax::{
    JavaFamily, JavaNode, JavaSyntaxField, JavaSyntaxNode, JavaSyntaxView,
    MethodInvocationFormSyntax, QualifiedMethodInvocation,
};

type ChainSuffix<'source> = (Doc<'source>, bool);

pub(super) fn format_postfix_family_expression<'source>(
    expression: Expression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(outer) = expression.syntax_node() else {
        doc.block_on_invariant("Java postfix expression has no syntax node");
        return Doc::nil();
    };
    let mut current = expression;
    while let Some(inner) = postfix_inner(current) {
        current = inner;
    }

    let Some(mut current_node) = current.syntax_node() else {
        doc.block_on_invariant("Java postfix base has no syntax node");
        return Doc::nil();
    };
    let relocate_comments = matches!(
        current,
        Expression::LiteralExpression(_) | Expression::NameExpression(_)
    ) && postfix_parent(current_node)
        .is_some_and(|(parent, _)| is_member_expression(parent));
    let mut comments = relocate_comments.then(|| format_expression_leading_comments(&current, doc));
    let base_leading = if relocate_comments {
        LeadingComments::SuppressFirstToken
    } else {
        leading_comments
    };
    let mut formatted = format_postfix_base(&current, base_leading, doc);

    while current_node != outer {
        if let Some((chain, parent, parent_node)) =
            format_member_chain(current, formatted, current_node, outer, doc)
        {
            formatted = if let Some(comments) = comments.take() {
                doc_concat!(doc, [comments, chain])
            } else {
                chain
            };
            current = parent;
            current_node = parent_node;
            continue;
        }
        let Some((parent, parent_node)) = postfix_parent(current_node) else {
            doc.block_on_invariant("Java postfix spine crossed an unexpected parent");
            break;
        };
        formatted = match parent {
            Expression::ArrayAccessExpression(access) => {
                format_array_access_expression(&access, Some(formatted), doc)
            }
            Expression::PostfixExpression(postfix) => {
                format_postfix_expression(&postfix, Some(formatted), doc)
            }
            Expression::ObjectCreationExpression(creation) => {
                format_object_creation_expression(&creation, Some(formatted), doc)
            }
            Expression::ThisExpression(this) => {
                format_this_expression(&this, leading_comments, Some(formatted), doc)
            }
            Expression::SuperExpression(super_expression) => {
                format_super_expression(&super_expression, leading_comments, Some(formatted), doc)
            }
            _ => {
                doc.block_on_invariant("Java postfix parent was not a suffix expression");
                break;
            }
        };
        current = parent;
        current_node = parent_node;
    }
    formatted
}

fn postfix_inner(expression: Expression<'_>) -> Option<Expression<'_>> {
    match expression {
        Expression::FieldAccessExpression(access) => present(access.receiver()),
        Expression::ArrayAccessExpression(access) => present(access.array()),
        Expression::PostfixExpression(postfix) => present(postfix.operand()),
        Expression::ObjectCreationExpression(creation) => present(creation.qualifier()),
        Expression::ThisExpression(this) => present(this.qualifier()),
        Expression::SuperExpression(super_expression) => present(super_expression.qualifier()),
        Expression::MethodInvocationExpression(invocation) => {
            present(qualified_invocation(&invocation)?.receiver())
        }
        _ => None,
    }
}

fn postfix_parent(current: JavaSyntaxNode<'_>) -> Option<(Expression<'_>, JavaSyntaxNode<'_>)> {
    let parent = current.parent()?;
    if let Some(expression) = Expression::cast(parent) {
        return Some((expression, parent));
    }
    QualifiedMethodInvocation::cast(parent)?;
    let owner = parent.parent()?;
    Some((
        Expression::MethodInvocationExpression(MethodInvocationExpression::cast(owner)?),
        owner,
    ))
}

fn format_postfix_base<'source>(
    expression: &Expression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match expression {
        Expression::FieldAccessExpression(access) => format_field_access_expression(access, doc),
        Expression::MethodInvocationExpression(invocation) => {
            format_method_invocation_expression_with_leading_comments(
                invocation,
                leading_comments,
                doc,
            )
        }
        Expression::ArrayAccessExpression(access) => {
            format_array_access_expression(access, None, doc)
        }
        Expression::PostfixExpression(postfix) => format_postfix_expression(postfix, None, doc),
        Expression::ObjectCreationExpression(creation) => {
            format_object_creation_expression(creation, None, doc)
        }
        Expression::ThisExpression(this) => {
            format_this_expression(this, leading_comments, None, doc)
        }
        Expression::SuperExpression(super_expression) => {
            format_super_expression(super_expression, leading_comments, None, doc)
        }
        _ => format_expression_with_leading_comments(expression, leading_comments, doc),
    }
}

fn is_member_expression(expression: Expression<'_>) -> bool {
    matches!(expression, Expression::FieldAccessExpression(_))
        || matches!(
            expression,
            Expression::MethodInvocationExpression(invocation)
                if qualified_invocation(&invocation)
                    .and_then(|qualified| present(qualified.receiver()))
                    .is_some()
        )
}

struct MemberChainBuilder<'source> {
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
    root: Expression<'source>,
    root_doc: Doc<'source>,
    root_node: JavaSyntaxNode<'source>,
    outer: JavaSyntaxNode<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<(Doc<'source>, Expression<'source>, JavaSyntaxNode<'source>)> {
    let mut chain = None;
    let mut current = root;
    let mut current_node = root_node;
    let rest_suffixes = doc.concat_list(|rest_suffixes| {
        let mut builder = MemberChainBuilder {
            first_suffix: None,
            has_rest_suffixes: false,
            field_run: None,
        };
        loop {
            if current_node == outer {
                break;
            }
            let Some((parent, parent_node)) = postfix_parent(current_node) else {
                break;
            };
            match parent {
                Expression::FieldAccessExpression(access) => {
                    builder.push_field_access(&access, rest_suffixes);
                }
                Expression::MethodInvocationExpression(invocation) => {
                    let Some(qualified) = qualified_invocation(&invocation) else {
                        break;
                    };
                    builder.push_method_invocation(&qualified, rest_suffixes);
                }
                _ => break,
            }
            current = parent;
            current_node = parent_node;
        }
        builder.flush_field_run(rest_suffixes);
        chain = Some((builder.first_suffix, builder.has_rest_suffixes));
    });
    let (first_suffix, has_rest_suffixes) = chain?;
    let (first_suffix, first_suffix_has_leading_comments) = first_suffix?;
    let keep_first_suffix_with_root =
        is_simple_member_chain_root(&root) && !first_suffix_has_leading_comments;
    let chain = member_chain(
        doc,
        root_doc,
        first_suffix,
        rest_suffixes,
        has_rest_suffixes,
        keep_first_suffix_with_root,
    );
    Some((chain, current, current_node))
}

fn required_dot_has_leading_comments(dot: JavaSyntaxField<'_, JavaSyntaxToken<'_>>) -> bool {
    matches!(dot, JavaSyntaxField::Present(dot) if !dot.leading_comments().is_empty())
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
