use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CallExpression, CallableReferenceExpression, CallableReferenceReceiver,
    CallableReferenceReceiverSyntax, CollectionLiteralExpression, Expression, IndexExpression,
    KotlinFamily, KotlinNode, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxNode,
    KotlinSyntaxToken, KotlinSyntaxView, NavigationExpression, NavigationOperatorSyntax,
    NavigationOperatorValue, NavigationSelectorSyntax, NavigationSelectorValue, ValueArgument,
    ValueArgumentEntryList, ValueArgumentList, ValueArgumentListEntry, ValueArgumentPrefix,
    ValueArgumentPrefixSyntax,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_token,
};
use crate::helpers::lists::{CommaListItem, delimited_comma_list, force_parenthesized_list};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_optional_field, format_required_field,
    join_delimited_recovery, resolve_list_part, resolve_required_delimiter, resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::names::format_name;
use crate::rules::types::format_type_argument_list;

use super::{
    format_expression_with_leading, lambdas::format_lambda_expression,
    operators::format_postfix_expression, references::format_callable_reference_expression,
};

pub(super) fn format_suffix_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(outer) = expression.syntax_node() else {
        doc.block_on_invariant("Kotlin suffix expression has no syntax node");
        return Doc::nil();
    };
    let mut current = expression;
    let mut outer_member = None;
    while let Some(inner) = suffix_inner(current) {
        if outer_member.is_none() && is_member_chain_expression(current) {
            outer_member = current.syntax_node();
        }
        current = inner;
    }

    let comments = if outer_member.is_some() {
        format_expression_leading_comments(doc, &expression, leading)
    } else {
        Doc::nil()
    };
    let base_leading = if outer_member.is_some() {
        LeadingTrivia::SuppressAlreadyHandled
    } else {
        leading
    };
    let Some(mut current_node) = current.syntax_node() else {
        doc.block_on_invariant("Kotlin suffix base has no syntax node");
        return Doc::nil();
    };
    let mut formatted = format_suffix_base(doc, &current, base_leading);
    while current_node != outer {
        if let Some((chain, parent, parent_node)) =
            format_member_chain(doc, current, formatted, current_node, outer)
        {
            formatted = if outer_member == Some(parent_node) {
                doc.concat([comments, chain])
            } else {
                chain
            };
            current = parent;
            current_node = parent_node;
            continue;
        }
        let Some((parent, parent_node)) = suffix_parent(current_node) else {
            doc.block_on_invariant("expression suffix spine crossed an unexpected parent");
            break;
        };
        formatted = match parent {
            Expression::CallExpression(call) => {
                let suffix = format_call_arguments(doc, &call);
                doc.concat([formatted, suffix])
            }
            Expression::NavigationExpression(navigation) => {
                format_navigation_expression(doc, &navigation, base_leading, Some(formatted))
            }
            Expression::PostfixExpression(postfix) => {
                format_postfix_expression(doc, &postfix, base_leading, Some(formatted))
            }
            Expression::CallableReferenceExpression(reference) => {
                format_callable_reference_expression(doc, &reference, base_leading, Some(formatted))
            }
            _ => {
                doc.block_on_invariant("expression suffix parent was not a tight suffix");
                break;
            }
        };
        current = parent;
        current_node = parent_node;
    }
    formatted
}

fn is_member_chain_expression(expression: Expression<'_>) -> bool {
    match expression {
        Expression::IndexExpression(index) => present_required(index.receiver()).is_some(),
        Expression::NavigationExpression(navigation) => {
            present_required(navigation.receiver()).is_some()
                && present_required(navigation.operator()).is_some()
                && present_required(navigation.selector()).is_some()
        }
        Expression::CallExpression(call) => {
            let Some(Expression::NavigationExpression(navigation)) =
                present_required(call.callee())
            else {
                return false;
            };
            is_member_chain_expression(Expression::NavigationExpression(navigation))
        }
        _ => false,
    }
}

fn suffix_inner(expression: Expression<'_>) -> Option<Expression<'_>> {
    match expression {
        Expression::CallExpression(call) => present_required(call.callee()),
        Expression::IndexExpression(index) => present_required(index.receiver()),
        Expression::NavigationExpression(navigation) => present_required(navigation.receiver()),
        Expression::PostfixExpression(postfix) => present_required(postfix.operand()),
        Expression::CallableReferenceExpression(reference) => {
            let receiver = match reference.receiver() {
                KotlinSyntaxField::Present(receiver) => receiver,
                KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => return None,
            };
            match present_required(receiver.receiver())?.classify().ok()? {
                CallableReferenceReceiverSyntax::Expression(receiver) => Some(receiver),
                CallableReferenceReceiverSyntax::TypeReference(_) => None,
            }
        }
        _ => None,
    }
}

fn suffix_parent(current: KotlinSyntaxNode<'_>) -> Option<(Expression<'_>, KotlinSyntaxNode<'_>)> {
    let parent = current.parent()?;
    if let Some(expression) = Expression::cast(parent) {
        return Some((expression, parent));
    }
    CallableReferenceReceiver::cast(parent)?;
    let owner = parent.parent()?;
    Some((
        Expression::CallableReferenceExpression(CallableReferenceExpression::cast(owner)?),
        owner,
    ))
}

fn format_suffix_base<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match expression {
        Expression::CallExpression(call) => format_call_expression(doc, call, leading),
        Expression::IndexExpression(index) => format_index_expression(doc, index, leading),
        Expression::NavigationExpression(navigation) => {
            format_navigation_expression(doc, navigation, leading, None)
        }
        Expression::PostfixExpression(postfix) => {
            format_postfix_expression(doc, postfix, leading, None)
        }
        Expression::CallableReferenceExpression(reference) => {
            format_callable_reference_expression(doc, reference, leading, None)
        }
        _ => format_expression_with_leading(doc, expression, leading),
    }
}

pub(super) fn format_navigation_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NavigationExpression<'source>,
    leading: LeadingTrivia,
    receiver: Option<Doc<'source>>,
) -> Doc<'source> {
    let receiver = receiver.unwrap_or_else(|| {
        format_required_field(expression.receiver(), doc, |receiver, doc| {
            format_expression_with_leading(doc, &receiver, leading)
        })
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_navigation_operator(
            doc,
            operator,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let selector = format_required_field(expression.selector(), doc, |selector, doc| {
        format_navigation_selector(doc, selector)
    });
    doc.concat([receiver, operator, selector])
}

pub(super) fn format_call_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let callee = format_required_field(expression.callee(), doc, |callee, doc| {
        format_expression_with_leading(doc, &callee, leading)
    });
    let suffix = format_call_arguments(doc, expression);
    doc.concat([callee, suffix])
}

pub(super) fn format_index_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IndexExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let receiver = format_required_field(expression.receiver(), doc, |receiver, doc| {
        format_expression_with_leading(doc, &receiver, leading)
    });
    let suffix = format_index_suffix(doc, expression);
    let contents = doc.concat([receiver, suffix]);
    doc.group(contents)
}

pub(super) fn format_collection_literal_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CollectionLiteralExpression<'source>,
    _leading: LeadingTrivia,
) -> Doc<'source> {
    format_square_argument_list(
        doc,
        expression.open_bracket(),
        expression.entries(),
        expression.close_bracket(),
    )
}

struct MemberChainBuilder<'source> {
    first_suffix: Option<Doc<'source>>,
    first_suffix_forces_break: bool,
    has_rest_suffixes: bool,
    force_multiline: bool,
    field_run: Option<Doc<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn finish(
        self,
        doc: &mut DocBuilder<'source>,
        root: &Expression<'source>,
        root_doc: Doc<'source>,
        rest_suffixes: Doc<'source>,
    ) -> Option<Doc<'source>> {
        let first_suffix = self.first_suffix?;
        let keep_first_suffix_with_root = is_simple_member_chain_root(root)
            && (!self.first_suffix_forces_break || matches!(root, Expression::CallExpression(_)));
        Some(member_chain(
            doc,
            root_doc,
            first_suffix,
            rest_suffixes,
            self.has_rest_suffixes,
            self.force_multiline,
            keep_first_suffix_with_root,
        ))
    }

    fn push_suffix(
        &mut self,
        rest: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break: bool,
    ) {
        self.flush_field_run(rest);
        self.append_suffix(rest, suffix, forces_break);
    }

    fn push_navigation_suffix(
        &mut self,
        rest: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break: bool,
    ) {
        if forces_break {
            self.push_suffix(rest, suffix, true);
        } else {
            self.field_run = Some(match self.field_run.take() {
                Some(run) => rest.concat([run, suffix]),
                None => suffix,
            });
        }
    }

    fn flush_field_run(&mut self, rest: &mut ConcatBuilder<'_, 'source>) {
        if let Some(run) = self.field_run.take() {
            self.append_suffix(rest, run, false);
        }
    }

    fn append_suffix(
        &mut self,
        rest: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break: bool,
    ) {
        self.force_multiline |= forces_break;
        if self.first_suffix.is_none() {
            self.first_suffix = Some(suffix);
            self.first_suffix_forces_break = forces_break;
        } else {
            self.has_rest_suffixes = true;
            let line = rest.soft_line();
            rest.push(line);
            rest.push(suffix);
        }
    }
}

fn format_member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Expression<'source>,
    root_doc: Doc<'source>,
    root_node: KotlinSyntaxNode<'source>,
    outer: KotlinSyntaxNode<'source>,
) -> Option<(Doc<'source>, Expression<'source>, KotlinSyntaxNode<'source>)> {
    let mut builder = MemberChainBuilder {
        first_suffix: None,
        first_suffix_forces_break: false,
        has_rest_suffixes: false,
        force_multiline: false,
        field_run: None,
    };
    let mut current = root;
    let mut current_node = root_node;
    let rest = doc.concat_list(|rest| {
        loop {
            if current_node == outer {
                break;
            }
            let Some((parent, parent_node)) = suffix_parent(current_node) else {
                break;
            };
            match parent {
                Expression::NavigationExpression(navigation) => {
                    let forces_break = navigation_operator_has_leading_comments(&navigation);
                    let Some(navigation_doc) = format_navigation_suffix(rest, &navigation) else {
                        break;
                    };
                    if parent_node != outer
                        && let Some((Expression::CallExpression(call), call_node)) =
                            suffix_parent(parent_node)
                    {
                        let forces_break = forces_break
                            || (call_has_lambdas(&call)
                                && !call_has_parenthesized_arguments(&call));
                        let arguments = format_call_arguments(rest, &call);
                        let suffix = rest.concat([navigation_doc, arguments]);
                        builder.push_suffix(rest, suffix, forces_break);
                        current = Expression::CallExpression(call);
                        current_node = call_node;
                        continue;
                    }
                    builder.push_navigation_suffix(rest, navigation_doc, forces_break);
                    current = Expression::NavigationExpression(navigation);
                    current_node = parent_node;
                }
                Expression::IndexExpression(index) => {
                    let suffix = format_index_suffix(rest, &index);
                    builder.push_suffix(rest, suffix, false);
                    current = Expression::IndexExpression(index);
                    current_node = parent_node;
                }
                _ => break,
            }
        }
        builder.flush_field_run(rest);
    });
    builder
        .finish(doc, &root, root_doc, rest)
        .map(|chain| (chain, current, current_node))
}

fn member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Doc<'source>,
    first_suffix: Doc<'source>,
    rest: Doc<'source>,
    has_rest_suffixes: bool,
    force_multiline: bool,
    keep_first: bool,
) -> Doc<'source> {
    let head = if keep_first {
        doc.concat([root, first_suffix])
    } else {
        root
    };
    let rest = if keep_first {
        rest
    } else {
        let line = doc.soft_line();
        doc.concat([line, first_suffix, rest])
    };
    if keep_first && !has_rest_suffixes {
        return doc.group(head);
    }
    let rest = doc.indent(rest);
    let contents = doc.concat([head, rest]);
    if force_multiline {
        doc.force_group(contents)
    } else {
        doc.group(contents)
    }
}

fn format_expression_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if leading == LeadingTrivia::SuppressAlreadyHandled {
        return doc.nil();
    }
    let Some(token) = expression.first_token() else {
        return doc.nil();
    };
    format_leading_comments(doc, &token)
}

fn format_call_arguments<'source>(
    doc: &mut DocBuilder<'source>,
    call: &CallExpression<'source>,
) -> Doc<'source> {
    let type_arguments = format_required_field(call.type_arguments(), doc, |arguments, doc| {
        doc.concat_list(|docs| {
            for part in arguments.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(arguments) => {
                        let formatted = format_type_argument_list(docs, &arguments);
                        docs.push(formatted);
                    }
                    KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(format!(
                        "unexpected type-argument-list separator: {:?}",
                        separator.kind()
                    )),
                    KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                }
            }
        })
    });
    let values = format_optional_field(call.arguments(), doc, |arguments, doc| {
        format_value_argument_list(doc, &arguments)
    });
    let lambdas = format_required_field(call.lambdas(), doc, |lambdas, doc| {
        doc.concat_list(|docs| {
            for part in lambdas.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(lambda) => {
                        let space = docs.space();
                        docs.push(space);
                        let lambda =
                            format_lambda_expression(docs, &lambda, LeadingTrivia::Preserve);
                        docs.push(lambda);
                    }
                    KotlinFormatListPart::Separator(separator) => docs.block_on_invariant(format!(
                        "unexpected lambda-list separator: {:?}",
                        separator.kind()
                    )),
                    KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                }
            }
        })
    });
    doc.concat([type_arguments, values, lambdas])
}

fn format_navigation_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    navigation: &NavigationExpression<'source>,
) -> Option<Doc<'source>> {
    let operator = present_required(navigation.operator())?;
    let selector = present_required(navigation.selector())?;
    let operator = format_navigation_operator(
        doc,
        operator,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeSpaceIfComments,
    );
    let selector = format_navigation_selector(doc, selector);
    Some(doc.concat([operator, selector]))
}

fn format_navigation_operator<'source>(
    doc: &mut DocBuilder<'source>,
    operator: NavigationOperatorValue<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match operator.classify() {
        Ok(NavigationOperatorSyntax::Token(token)) => format_token(doc, &token, leading, trailing),
        Ok(NavigationOperatorSyntax::SplitSafe(split)) => {
            let question = format_required_field(split.question(), doc, |token, doc| {
                format_token(
                    doc,
                    &token,
                    leading,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            });
            let dot = format_required_field(split.dot(), doc, |token, doc| {
                format_token(doc, &token, LeadingTrivia::SuppressAlreadyHandled, trailing)
            });
            doc.concat([question, dot])
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_navigation_selector<'source>(
    doc: &mut DocBuilder<'source>,
    selector: NavigationSelectorValue<'source>,
) -> Doc<'source> {
    match selector.classify() {
        Ok(NavigationSelectorSyntax::Name(selector)) => format_token(
            doc,
            &selector,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        Ok(NavigationSelectorSyntax::This(selector)) => format_expression_with_leading(
            doc,
            &Expression::ThisExpression(selector),
            LeadingTrivia::Preserve,
        ),
        Ok(NavigationSelectorSyntax::Super(selector)) => format_expression_with_leading(
            doc,
            &Expression::SuperExpression(selector),
            LeadingTrivia::Preserve,
        ),
        Ok(NavigationSelectorSyntax::Bogus(bogus)) => {
            crate::helpers::recovery::format_malformed(&bogus, doc)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn navigation_operator_has_leading_comments(navigation: &NavigationExpression<'_>) -> bool {
    present_required(navigation.operator())
        .and_then(jolt_kotlin_syntax::NavigationOperatorValue::first_token)
        .is_some_and(|operator| !operator.leading_comments().is_empty())
}

fn format_index_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    index: &IndexExpression<'source>,
) -> Doc<'source> {
    format_square_argument_list(
        doc,
        index.open_bracket(),
        index.entries(),
        index.close_bracket(),
    )
}

fn format_square_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
    entries: KotlinSyntaxField<'source, ValueArgumentEntryList<'source>>,
    close: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(open, doc);
    let close = resolve_required_delimiter(close, doc);
    let items = match resolve_required_field(entries, doc) {
        KotlinFormatField::Present(entries) => {
            value_argument_list_entry_items(doc, entries.parts())
        }
        KotlinFormatField::Malformed(recovery) => {
            vec![CommaListItem::visible(recovery)]
        }
    };
    let list = delimited_comma_list(doc, open.source(), close.source(), items);
    join_delimited_recovery(doc, &open, list, &close)
}
const fn is_simple_member_chain_root(expression: &Expression<'_>) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_)
            | Expression::ThisExpression(_)
            | Expression::SuperExpression(_)
            | Expression::CallExpression(_)
    )
}

fn call_has_lambdas(call: &CallExpression<'_>) -> bool {
    match call.lambdas() {
        KotlinSyntaxField::Present(lambdas) => lambdas
            .parts()
            .any(|part| matches!(part, KotlinSyntaxListPart::Item(_))),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => false,
    }
}

fn call_has_parenthesized_arguments(call: &CallExpression<'_>) -> bool {
    matches!(call.arguments(), KotlinSyntaxField::Present(_))
}

pub(crate) fn format_value_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &ValueArgumentList<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_paren(), doc);
    let close = resolve_required_delimiter(arguments.close_paren(), doc);
    let items = match resolve_required_field(arguments.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            value_argument_list_entry_items(doc, entries.parts())
        }
        KotlinFormatField::Malformed(recovery) => {
            vec![CommaListItem::visible(recovery)]
        }
    };
    let has_comments = items.iter().any(CommaListItem::is_visible)
        && value_argument_list_has_leading_comments(arguments);
    let list = if has_comments {
        force_parenthesized_list(doc, open.source(), close.source(), items)
    } else {
        delimited_comma_list(doc, open.source(), close.source(), items)
    };
    join_delimited_recovery(doc, &open, list, &close)
}

fn value_argument_list_entry_items<'source>(
    doc: &mut DocBuilder<'source>,
    parts: impl Iterator<Item = KotlinSyntaxListPart<'source, ValueArgumentListEntry<'source>>>,
) -> Vec<CommaListItem<'source>> {
    let mut items: Vec<CommaListItem<'source>> = Vec::new();
    for part in parts {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(entry) => {
                let formatted = match entry {
                    ValueArgumentListEntry::ValueArgument(argument) => {
                        format_value_argument(doc, &argument)
                    }
                    ValueArgumentListEntry::BogusValueArgument(bogus) => {
                        crate::helpers::recovery::format_malformed(&bogus, doc)
                    }
                };
                items.push(CommaListItem::visible(formatted));
            }
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.iter_mut().rev().find(|item| item.is_visible())
                    && item.comma.is_none()
                {
                    item.comma = Some(comma);
                } else {
                    items.push(CommaListItem::visible_with_comma(Doc::nil(), comma));
                }
            }
            KotlinFormatListPart::Recovery(recovery) => {
                items.push(CommaListItem::recovery(recovery));
            }
        }
    }
    items
}

fn value_argument_list_has_leading_comments(arguments: &ValueArgumentList<'_>) -> bool {
    let Some(entries) = present_required(arguments.entries()) else {
        return false;
    };
    entries.parts().any(|part| match part {
        KotlinSyntaxListPart::Item(argument) => argument
            .first_token()
            .is_some_and(|token| !token.leading_comments().is_empty()),
        _ => false,
    })
}

pub(crate) fn format_value_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
) -> Doc<'source> {
    let has_prefix = matches!(
        argument.prefix(),
        KotlinSyntaxField::Present(ref prefix) if prefix.first_token().is_some()
    );
    let has_name = matches!(
        argument.name(),
        KotlinSyntaxField::Present(ref name) if name.first_token().is_some()
    );
    let has_assign = matches!(argument.assign(), KotlinSyntaxField::Present(_));
    let has_expression = matches!(
        argument.expression(),
        KotlinSyntaxField::Present(ref expression) if expression.first_token().is_some()
    );
    let prefix = format_required_field(argument.prefix(), doc, |prefix, doc| {
        doc.concat_list(|docs| {
            for part in prefix.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(role) => {
                        let formatted = format_value_argument_prefix_item(docs, role);
                        docs.push(formatted);
                    }
                    KotlinFormatListPart::Separator(separator) => {
                        docs.block_on_invariant(format!(
                            "unexpected argument-prefix separator: {:?}",
                            separator.kind()
                        ));
                    }
                    KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                }
            }
        })
    });
    let name = format_optional_field(argument.name(), doc, |name, doc| format_name(doc, &name));
    let assign = format_optional_field(argument.assign(), doc, |assign, doc| {
        let before = if has_prefix || has_name {
            doc.space()
        } else {
            Doc::nil()
        };
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        let after = if has_expression {
            doc.space()
        } else {
            Doc::nil()
        };
        doc.concat([before, assign, after])
    });
    let missing_assign_separator = if !has_assign && has_name && has_expression {
        doc.space()
    } else {
        Doc::nil()
    };
    let expression = format_required_field(argument.expression(), doc, |expression, doc| {
        let comments =
            format_expression_leading_comments(doc, &expression, LeadingTrivia::Preserve);
        let expression =
            format_expression_with_leading(doc, &expression, LeadingTrivia::SuppressAlreadyHandled);
        doc.concat([comments, expression])
    });
    doc.concat([prefix, name, assign, missing_assign_separator, expression])
}

fn format_value_argument_prefix_item<'source>(
    doc: &mut DocBuilder<'source>,
    prefix: ValueArgumentPrefix<'source>,
) -> Doc<'source> {
    format_required_field(prefix.prefix(), doc, |prefix, doc| {
        match prefix.classify() {
            Ok(ValueArgumentPrefixSyntax::Spread(token)) => format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            Ok(ValueArgumentPrefixSyntax::Annotation(annotation)) => {
                format_annotation(doc, &annotation)
            }
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                Doc::nil()
            }
        }
    })
}

fn present_required<T>(field: KotlinSyntaxField<'_, T>) -> Option<T> {
    match field {
        KotlinSyntaxField::Present(value) => Some(value),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => None,
    }
}
