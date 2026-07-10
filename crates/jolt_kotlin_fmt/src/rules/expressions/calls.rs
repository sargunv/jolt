use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CallExpression, CollectionLiteralExpression, Expression, ExpressionParentRole, IndexExpression,
    NavigationExpression, NavigationOperatorTokens, RecoveredSeparatedListEntry, ValueArgument,
    ValueArgumentList,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_token, format_token_sequence,
    token_has_comments,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, compact_square_bracket_list,
    force_parenthesized_list, parenthesized_list, square_bracket_list,
};

use super::{format_expression_with_leading, lambdas::format_lambda_expression};
use crate::rules::types::format_type_argument_list;

pub(super) fn format_navigation_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NavigationExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::NavigationExpression(*expression);
    if !is_member_chain_child(doc, &expression)
        && let Some(chain) = format_member_chain(doc, expression, leading)
    {
        return chain;
    }
    let Expression::NavigationExpression(expression) = expression else {
        return doc.nil();
    };

    let Some(receiver) = expression.receiver() else {
        let prefix = format_token_sequence(doc, expression.recovered_prefix_tokens(), leading);
        let operator = format_navigation_operator(
            doc,
            expression.operator_tokens(),
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeSpaceIfComments,
        );
        let selector = if let Some(selector) = expression.selector_token() {
            format_token(
                doc,
                &selector,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        } else {
            doc.nil()
        };
        return doc.concat([prefix, operator, selector]);
    };
    let operators = expression.operator_tokens();
    if operators.is_empty() {
        return format_expression_with_leading(doc, &receiver, leading);
    }
    let Some(selector) = expression.selector_token() else {
        let receiver = format_expression_with_leading(doc, &receiver, leading);
        let operator = format_navigation_operator(
            doc,
            operators,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        );
        return doc.concat([receiver, operator]);
    };

    let receiver = format_expression_with_leading(doc, &receiver, leading);
    let operator = format_navigation_operator(
        doc,
        operators,
        LeadingTrivia::SuppressAlreadyHandled,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let selector = format_token(
        doc,
        &selector,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    doc.concat([receiver, operator, selector])
}

pub(super) fn format_call_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::CallExpression(*expression);
    if !is_member_chain_child(doc, &expression)
        && let Some(chain) = format_member_chain(doc, expression, leading)
    {
        return chain;
    }
    let Expression::CallExpression(expression) = expression else {
        return doc.nil();
    };

    let callee = if let Some(callee) = expression.callee() {
        format_expression_with_leading(doc, &callee, leading)
    } else {
        doc.nil()
    };
    let type_arguments = expression
        .type_argument_lists()
        .map(|arguments| format_type_argument_list(doc, &arguments))
        .collect::<Vec<_>>();
    let type_arguments = doc.concat(type_arguments);
    let value_arguments = if let Some(arguments) = expression.value_argument_list() {
        format_value_argument_list(doc, &arguments)
    } else {
        doc.nil()
    };
    let lambdas = expression
        .lambdas()
        .map(|lambda| {
            let space = doc.space();
            let lambda = format_lambda_expression(doc, &lambda, LeadingTrivia::Preserve);
            doc.concat([space, lambda])
        })
        .collect::<Vec<_>>();
    let lambdas = doc.concat(lambdas);
    doc.concat([callee, type_arguments, value_arguments, lambdas])
}

pub(super) fn format_index_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IndexExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::IndexExpression(*expression);
    if !is_member_chain_child(doc, &expression)
        && let Some(chain) = format_member_chain(doc, expression, leading)
    {
        return chain;
    }
    let Expression::IndexExpression(expression) = expression else {
        return doc.nil();
    };

    let Some(receiver) = expression.receiver() else {
        return format_index_suffix(doc, &expression);
    };

    let receiver = format_expression_with_leading(doc, &receiver, leading);
    let suffix = format_index_suffix(doc, &expression);
    let contents = doc.concat([receiver, suffix]);
    doc.group(contents)
}

pub(super) fn format_collection_literal_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CollectionLiteralExpression<'source>,
    _leading: LeadingTrivia,
) -> Doc<'source> {
    let SquareArgumentListItems {
        items,
        has_recovered_tokens,
    } = collection_literal_items(doc, expression);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    let open = expression.open_bracket();
    let close = expression.close_bracket();

    if has_trailing_comma || has_recovered_tokens {
        square_bracket_list(doc, open.as_ref(), close.as_ref(), items)
    } else {
        compact_square_bracket_list(doc, open.as_ref(), close.as_ref(), items)
    }
}

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    first_suffix: Option<Doc<'source>>,
    first_suffix_forces_break: bool,
    suffix_count: u32,
    force_multiline: bool,
    field_run: Option<Doc<'source>>,
}

impl<'source> MemberChainBuilder<'source> {
    fn finish(
        self,
        doc: &mut DocBuilder<'source>,
        leading: LeadingTrivia,
        rest_suffixes: Doc<'source>,
    ) -> Option<Doc<'source>> {
        let root = self.root?;
        let keep_first_suffix_with_root = is_simple_member_chain_root(&root)
            && (self.suffix_count == 0 || !self.first_suffix_forces_break);

        let leading_comments = format_expression_leading_comments(doc, &root, leading);
        let root_doc =
            format_expression_with_leading(doc, &root, LeadingTrivia::SuppressAlreadyHandled);
        let chain = member_chain(
            doc,
            root_doc,
            self.first_suffix,
            rest_suffixes,
            self.suffix_count,
            self.force_multiline,
            keep_first_suffix_with_root,
        );
        Some(doc.concat([leading_comments, chain]))
    }

    fn push_suffix(
        &mut self,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break_before: bool,
    ) {
        self.flush_field_run(rest_suffixes);
        self.append_suffix(rest_suffixes, suffix, forces_break_before);
    }

    fn push_navigation_suffix(
        &mut self,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break_before: bool,
    ) {
        if forces_break_before {
            self.push_suffix(rest_suffixes, suffix, true);
        } else {
            self.field_run = Some(match self.field_run.take() {
                Some(run) => rest_suffixes.concat([run, suffix]),
                None => suffix,
            });
        }
    }

    fn flush_field_run(&mut self, rest_suffixes: &mut ConcatBuilder<'_, 'source>) {
        if let Some(run) = self.field_run.take() {
            self.append_suffix(rest_suffixes, run, false);
        }
    }

    fn append_suffix(
        &mut self,
        rest_suffixes: &mut ConcatBuilder<'_, 'source>,
        suffix: Doc<'source>,
        forces_break_before: bool,
    ) {
        self.force_multiline |= forces_break_before;
        if self.suffix_count == 0 {
            self.first_suffix = Some(suffix);
            self.first_suffix_forces_break = forces_break_before;
        } else {
            let soft_line = rest_suffixes.soft_line();
            rest_suffixes.push(soft_line);
            rest_suffixes.push(suffix);
        }
        self.suffix_count += 1;
    }
}

fn format_member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    expression: Expression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let mut builder = MemberChainBuilder {
        root: None,
        first_suffix: None,
        first_suffix_forces_break: false,
        suffix_count: 0,
        force_multiline: false,
        field_run: None,
    };

    let mut valid = false;
    let rest_suffixes = doc.concat_list(|rest_suffixes| {
        valid = append_chain_expression(rest_suffixes, &mut builder, expression).is_some();
        if valid {
            builder.flush_field_run(rest_suffixes);
        }
    });
    if valid {
        builder.finish(doc, leading, rest_suffixes)
    } else {
        None
    }
}

fn append_chain_expression<'source>(
    rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
) -> Option<()> {
    match expression {
        Expression::CallExpression(call) => {
            let callee = call.callee()?;
            let Expression::NavigationExpression(navigation) = callee else {
                return None;
            };
            append_chain_receiver(rest_suffixes, builder, navigation.receiver()?);
            let forces_break_before =
                navigation_operator_has_leading_comments(rest_suffixes, &navigation)
                    || call_has_lambdas(rest_suffixes, &call);
            let suffix = format_call_suffix(rest_suffixes, &navigation, &call)?;
            builder.push_suffix(rest_suffixes, suffix, forces_break_before);
            Some(())
        }
        Expression::NavigationExpression(navigation) => {
            append_chain_receiver(rest_suffixes, builder, navigation.receiver()?);
            let suffix = format_navigation_suffix(rest_suffixes, &navigation)?;
            let forces_break_before =
                navigation_operator_has_leading_comments(rest_suffixes, &navigation);
            builder.push_navigation_suffix(rest_suffixes, suffix, forces_break_before);
            Some(())
        }
        Expression::IndexExpression(index) => {
            append_chain_receiver(rest_suffixes, builder, index.receiver()?);
            let suffix = format_index_suffix(rest_suffixes, &index);
            builder.push_suffix(rest_suffixes, suffix, false);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    rest_suffixes: &mut ConcatBuilder<'_, 'source>,
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
) {
    if append_chain_expression(rest_suffixes, builder, receiver).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Doc<'source>,
    first_suffix: Option<Doc<'source>>,
    rest_suffixes: Doc<'source>,
    suffix_count: u32,
    force_multiline: bool,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if suffix_count == 0 {
        return root;
    }

    let first_suffix = first_suffix.expect("member chain suffix exists");
    let head = if keep_first_suffix_with_root {
        doc.concat([root, first_suffix])
    } else {
        root
    };
    let rest = if keep_first_suffix_with_root {
        rest_suffixes
    } else {
        let soft_line = doc.soft_line();
        doc.concat([soft_line, first_suffix, rest_suffixes])
    };

    if keep_first_suffix_with_root && suffix_count == 1 {
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

    if let Some(token) = expression.first_token() {
        format_leading_comments(doc, &token)
    } else {
        doc.nil()
    }
}

fn format_call_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    navigation: &NavigationExpression<'source>,
    call: &CallExpression<'source>,
) -> Option<Doc<'source>> {
    let navigation = format_navigation_suffix(doc, navigation)?;
    let type_arguments = call
        .type_argument_lists()
        .map(|arguments| format_type_argument_list(doc, &arguments))
        .collect::<Vec<_>>();
    let type_arguments = doc.concat(type_arguments);
    let value_arguments = if let Some(arguments) = call.value_argument_list() {
        format_value_argument_list(doc, &arguments)
    } else {
        doc.nil()
    };
    let lambdas = call
        .lambdas()
        .map(|lambda| {
            let space = doc.space();
            let lambda = format_lambda_expression(doc, &lambda, LeadingTrivia::Preserve);
            doc.concat([space, lambda])
        })
        .collect::<Vec<_>>();
    let lambdas = doc.concat(lambdas);
    Some(doc.concat([navigation, type_arguments, value_arguments, lambdas]))
}

fn format_navigation_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    navigation: &NavigationExpression<'source>,
) -> Option<Doc<'source>> {
    let operators = navigation.operator_tokens();
    if operators.is_empty() {
        return None;
    }
    let selector = navigation.selector_token()?;

    let operator = format_navigation_operator(
        doc,
        operators,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeSpaceIfComments,
    );
    let selector = format_token(
        doc,
        &selector,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    Some(doc.concat([operator, selector]))
}

fn navigation_operator_has_leading_comments(
    _doc: &mut DocBuilder<'_>,
    navigation: &NavigationExpression<'_>,
) -> bool {
    navigation
        .operator_tokens()
        .first()
        .is_some_and(|operator| !operator.leading_comments().is_empty())
}

fn format_navigation_operator<'source>(
    doc: &mut DocBuilder<'source>,
    tokens: NavigationOperatorTokens<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let last_index = tokens.len().saturating_sub(1);
    let tokens = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            format_token(
                doc,
                &token,
                if index == 0 {
                    leading
                } else {
                    LeadingTrivia::SuppressAlreadyHandled
                },
                if index == last_index {
                    trailing
                } else {
                    TrailingTrivia::RelocatedToEnclosingContext
                },
            )
        })
        .collect::<Vec<_>>();
    doc.concat(tokens)
}

fn call_has_lambdas(_doc: &mut DocBuilder<'_>, call: &CallExpression<'_>) -> bool {
    call.lambdas().next().is_some()
}

fn is_member_chain_child(_doc: &mut DocBuilder<'_>, expression: &Expression<'_>) -> bool {
    matches!(
        expression.parent_role(),
        Some(ExpressionParentRole::NavigationReceiver | ExpressionParentRole::IndexReceiver)
    )
}

fn format_index_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    index: &IndexExpression<'source>,
) -> Doc<'source> {
    let SquareArgumentListItems {
        items,
        has_recovered_tokens,
    } = index_argument_items(doc, index);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_trailing_comma || has_recovered_tokens {
        square_bracket_list(
            doc,
            index.open_bracket().as_ref(),
            index.close_bracket().as_ref(),
            items,
        )
    } else {
        compact_square_bracket_list(
            doc,
            index.open_bracket().as_ref(),
            index.close_bracket().as_ref(),
            items,
        )
    }
}

struct SquareArgumentListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_recovered_tokens: bool,
}

fn collection_literal_items<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CollectionLiteralExpression<'source>,
) -> SquareArgumentListItems<'source> {
    let entries = expression.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);
    let mut has_recovered_tokens = false;

    for entry in entries {
        has_recovered_tokens |= push_square_argument_entry(doc, &mut items, entry);
    }

    SquareArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn index_argument_items<'source>(
    doc: &mut DocBuilder<'source>,
    index: &IndexExpression<'source>,
) -> SquareArgumentListItems<'source> {
    let entries = index.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);
    let mut has_recovered_tokens = false;

    for entry in entries {
        has_recovered_tokens |= push_square_argument_entry(doc, &mut items, entry);
    }

    SquareArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn push_square_argument_entry<'source>(
    doc: &mut DocBuilder<'source>,
    items: &mut Vec<CommaListItem<'source>>,
    entry: RecoveredSeparatedListEntry<'source, jolt_kotlin_syntax::ValueArgumentEntry<'source>>,
) -> bool {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => {
            items.push(CommaListItem {
                doc: format_value_argument(doc, &entry.argument),
                comma: entry.comma,
            });
            false
        }
        RecoveredSeparatedListEntry::Token(token) => {
            items.push(CommaListItem {
                doc: format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve),
                comma: None,
            });
            true
        }
        RecoveredSeparatedListEntry::Error(error) => {
            items.push(CommaListItem {
                doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            });
            true
        }
        RecoveredSeparatedListEntry::Node(node) => {
            items.push(CommaListItem {
                doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            });
            true
        }
    }
}

const fn is_simple_member_chain_root(expression: &Expression<'_>) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_)
            | Expression::ThisExpression(_)
            | Expression::SuperExpression(_)
    )
}

pub(crate) fn format_value_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &ValueArgumentList<'source>,
) -> Doc<'source> {
    let ValueArgumentListItems {
        items,
        has_argument_leading_comments,
        has_recovered_tokens,
    } = value_argument_list_items(doc, arguments);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_argument_leading_comments || has_recovered_tokens {
        force_parenthesized_list(
            doc,
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        )
    } else if has_trailing_comma
        || arguments
            .open_paren()
            .as_ref()
            .is_some_and(|token| token_has_comments(token))
        || arguments
            .close_paren()
            .as_ref()
            .is_some_and(|token| token_has_comments(token))
    {
        parenthesized_list(
            doc,
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        )
    } else {
        compact_parenthesized_list(
            doc,
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        )
    }
}

struct ValueArgumentListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_argument_leading_comments: bool,
    has_recovered_tokens: bool,
}

fn value_argument_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &ValueArgumentList<'source>,
) -> ValueArgumentListItems<'source> {
    let entries = arguments.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);
    let mut previous_comma = None;
    let mut has_argument_leading_comments = false;
    let mut has_recovered_tokens = false;

    for entry in entries {
        match entry {
            RecoveredSeparatedListEntry::Entry(entry) => {
                let leading = if previous_comma.is_some() {
                    LeadingTrivia::SuppressAlreadyHandled
                } else {
                    LeadingTrivia::Preserve
                };
                has_argument_leading_comments |= entry
                    .argument
                    .first_token()
                    .is_some_and(|token| !token.leading_comments().is_empty());
                items.push(CommaListItem {
                    doc: format_value_argument_with_leading(doc, &entry.argument, leading),
                    comma: entry.comma,
                });
                previous_comma = entry.comma;
            }
            RecoveredSeparatedListEntry::Token(token) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(
                        doc,
                        std::iter::once(token),
                        LeadingTrivia::Preserve,
                    ),
                    comma: None,
                });
                has_recovered_tokens = true;
            }
            RecoveredSeparatedListEntry::Error(error) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                });
                has_recovered_tokens = true;
            }
            RecoveredSeparatedListEntry::Node(node) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                });
                has_recovered_tokens = true;
            }
        }
    }

    ValueArgumentListItems {
        items,
        has_argument_leading_comments,
        has_recovered_tokens,
    }
}

pub(crate) fn format_value_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
) -> Doc<'source> {
    format_value_argument_with_leading(doc, argument, LeadingTrivia::Preserve)
}

fn format_value_argument_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(expression) = argument.expression() else {
        return format_recovered_value_argument_tokens(doc, argument, leading);
    };
    let has_named_assign = argument.assign_token().is_some();

    let prefix = format_value_argument_prefix(doc, argument);
    let expression = format_expression_with_leading(
        doc,
        &expression,
        if has_named_assign {
            LeadingTrivia::Preserve
        } else {
            leading
        },
    );
    doc.concat([prefix, expression])
}

fn format_recovered_value_argument_tokens<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_token_sequence(doc, argument.token_iter(), leading)
}

fn format_value_argument_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
) -> Doc<'source> {
    let tokens = argument.prefix_tokens();
    doc.concat_list(|docs| {
        for token in tokens {
            if token.kind() == jolt_kotlin_syntax::KotlinSyntaxKind::Assign {
                let space = docs.space();
                docs.push(space);
                let token = format_token(
                    docs,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                );
                docs.push(token);
                let space = docs.space();
                docs.push(space);
            } else {
                let token = format_token(
                    docs,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                );
                docs.push(token);
            }
        }
    })
}
