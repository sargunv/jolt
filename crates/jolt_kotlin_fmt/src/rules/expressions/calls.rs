use jolt_fmt_ir::{Doc, concat, force_group, group, indent, soft_line, space};
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
    expression: &NavigationExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::NavigationExpression(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, leading)
    {
        return chain;
    }
    let Expression::NavigationExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    let Some(receiver) = expression.receiver() else {
        return concat([
            format_navigation_operator(
                expression.operator_tokens(),
                leading,
                TrailingTrivia::BeforeSpaceIfComments,
            ),
            expression
                .selector_token()
                .map_or_else(jolt_fmt_ir::nil, |selector| {
                    format_token(&selector, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
                }),
        ]);
    };
    let operators = expression.operator_tokens();
    if operators.is_empty() {
        return format_expression_with_leading(&receiver, leading);
    }
    let Some(selector) = expression.selector_token() else {
        return concat([
            format_expression_with_leading(&receiver, leading),
            format_navigation_operator(
                operators,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::Preserve,
            ),
        ]);
    };

    concat([
        format_expression_with_leading(&receiver, leading),
        format_navigation_operator(
            operators,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        format_token(&selector, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
    ])
}

pub(super) fn format_call_expression<'source>(
    expression: &CallExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::CallExpression(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, leading)
    {
        return chain;
    }
    let Expression::CallExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    let callee = expression.callee();
    concat([
        callee.map_or_else(jolt_fmt_ir::nil, |callee| {
            format_expression_with_leading(&callee, leading)
        }),
        concat(
            expression
                .type_argument_lists()
                .map(|arguments| format_type_argument_list(&arguments)),
        ),
        expression
            .value_argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_value_argument_list(&arguments)
            }),
        concat(expression.lambdas().map(|lambda| {
            concat([
                space(),
                format_lambda_expression(&lambda, LeadingTrivia::Preserve),
            ])
        })),
    ])
}

pub(super) fn format_index_expression<'source>(
    expression: &IndexExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression = Expression::IndexExpression(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, leading)
    {
        return chain;
    }
    let Expression::IndexExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    let Some(receiver) = expression.receiver() else {
        return format_index_suffix(&expression);
    };

    group(concat([
        format_expression_with_leading(&receiver, leading),
        format_index_suffix(&expression),
    ]))
}

pub(super) fn format_collection_literal_expression<'source>(
    expression: &CollectionLiteralExpression<'source>,
    _leading: LeadingTrivia,
) -> Doc<'source> {
    let SquareArgumentListItems {
        items,
        has_recovered_tokens,
    } = collection_literal_items(expression);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    let open = expression.open_bracket();
    let close = expression.close_bracket();

    if has_trailing_comma || has_recovered_tokens {
        square_bracket_list(open.as_ref(), close.as_ref(), items)
    } else {
        compact_square_bracket_list(open.as_ref(), close.as_ref(), items)
    }
}

struct MemberChainBuilder<'source> {
    root: Option<Expression<'source>>,
    units: Vec<MemberChainUnit<'source>>,
    field_run: Vec<Doc<'source>>,
}

struct MemberChainUnit<'source> {
    doc: Doc<'source>,
    forces_break_before: bool,
}

impl<'source> MemberChainBuilder<'source> {
    fn finish(mut self, leading: LeadingTrivia) -> Option<Doc<'source>> {
        self.flush_field_run();
        let root = self.root?;
        let keep_first_suffix_with_root = is_simple_member_chain_root(&root)
            && self
                .units
                .first()
                .is_none_or(|unit| !unit.forces_break_before);

        Some(concat([
            format_expression_leading_comments(&root, leading),
            member_chain(
                format_expression_with_leading(&root, LeadingTrivia::SuppressAlreadyHandled),
                self.units,
                keep_first_suffix_with_root,
            ),
        ]))
    }

    fn push_suffix(&mut self, suffix: Doc<'source>, forces_break_before: bool) {
        self.flush_field_run();
        self.units.push(MemberChainUnit {
            doc: suffix,
            forces_break_before,
        });
    }

    fn push_navigation_suffix(&mut self, suffix: Doc<'source>, forces_break_before: bool) {
        if forces_break_before {
            self.push_suffix(suffix, true);
        } else {
            self.field_run.push(suffix);
        }
    }

    fn flush_field_run(&mut self) {
        if self.field_run.is_empty() {
            return;
        }

        self.units.push(MemberChainUnit {
            doc: concat(std::mem::take(&mut self.field_run)),
            forces_break_before: false,
        });
    }
}

fn format_member_chain(expression: Expression<'_>, leading: LeadingTrivia) -> Option<Doc<'_>> {
    let mut builder = MemberChainBuilder {
        root: None,
        units: Vec::new(),
        field_run: Vec::new(),
    };

    append_chain_expression(&mut builder, expression)?;
    builder.finish(leading)
}

fn append_chain_expression<'source>(
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
) -> Option<()> {
    match expression {
        Expression::CallExpression(call) => {
            let callee = call.callee()?;
            let Expression::NavigationExpression(navigation) = callee else {
                return None;
            };
            append_chain_receiver(builder, navigation.receiver()?);
            let forces_break_before =
                navigation_operator_has_leading_comments(&navigation) || call_has_lambdas(&call);
            builder.push_suffix(format_call_suffix(&navigation, &call)?, forces_break_before);
            Some(())
        }
        Expression::NavigationExpression(navigation) => {
            append_chain_receiver(builder, navigation.receiver()?);
            builder.push_navigation_suffix(
                format_navigation_suffix(&navigation)?,
                navigation_operator_has_leading_comments(&navigation),
            );
            Some(())
        }
        Expression::IndexExpression(index) => {
            append_chain_receiver(builder, index.receiver()?);
            builder.push_suffix(format_index_suffix(&index), false);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
) {
    if append_chain_expression(builder, receiver).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    root: Doc<'source>,
    units: Vec<MemberChainUnit<'source>>,
    keep_first_suffix_with_root: bool,
) -> Doc<'source> {
    if units.is_empty() {
        return root;
    }

    let force_multiline = units.iter().any(|unit| unit.forces_break_before);
    let mut suffixes = units.into_iter();
    let head = if keep_first_suffix_with_root {
        if let Some(suffix) = suffixes.next() {
            concat([root, suffix.doc])
        } else {
            root
        }
    } else {
        root
    };
    let rest = suffixes
        .map(|suffix| concat([soft_line(), suffix.doc]))
        .collect::<Vec<_>>();

    if rest.is_empty() {
        return group(head);
    }

    let doc = concat([head, indent(concat(rest))]);
    if force_multiline {
        force_group(doc)
    } else {
        group(doc)
    }
}

fn format_expression_leading_comments<'source>(
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if leading == LeadingTrivia::SuppressAlreadyHandled {
        return jolt_fmt_ir::nil();
    }

    expression
        .first_token()
        .map_or_else(jolt_fmt_ir::nil, |token| format_leading_comments(&token))
}

fn format_call_suffix<'source>(
    navigation: &NavigationExpression<'source>,
    call: &CallExpression<'source>,
) -> Option<Doc<'source>> {
    Some(concat([
        format_navigation_suffix(navigation)?,
        concat(
            call.type_argument_lists()
                .map(|arguments| format_type_argument_list(&arguments)),
        ),
        call.value_argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_value_argument_list(&arguments)
            }),
        concat(call.lambdas().map(|lambda| {
            concat([
                space(),
                format_lambda_expression(&lambda, LeadingTrivia::Preserve),
            ])
        })),
    ]))
}

fn format_navigation_suffix<'source>(
    navigation: &NavigationExpression<'source>,
) -> Option<Doc<'source>> {
    let operators = navigation.operator_tokens();
    if operators.is_empty() {
        return None;
    }
    let selector = navigation.selector_token()?;

    Some(concat([
        format_navigation_operator(
            operators,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeSpaceIfComments,
        ),
        format_token(&selector, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
    ]))
}

fn navigation_operator_has_leading_comments(navigation: &NavigationExpression<'_>) -> bool {
    navigation
        .operator_tokens()
        .first()
        .is_some_and(|operator| !operator.leading_comments().is_empty())
}

fn format_navigation_operator(
    tokens: NavigationOperatorTokens<'_>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'_> {
    let last_index = tokens.len().saturating_sub(1);
    concat(tokens.iter().enumerate().map(move |(index, token)| {
        format_token(
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
    }))
}

fn call_has_lambdas(call: &CallExpression<'_>) -> bool {
    call.lambdas().next().is_some()
}

fn is_member_chain_child(expression: &Expression<'_>) -> bool {
    matches!(
        expression.parent_role(),
        Some(ExpressionParentRole::NavigationReceiver | ExpressionParentRole::IndexReceiver)
    )
}

fn format_index_suffix<'source>(index: &IndexExpression<'source>) -> Doc<'source> {
    let SquareArgumentListItems {
        items,
        has_recovered_tokens,
    } = index_argument_items(index);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_trailing_comma || has_recovered_tokens {
        square_bracket_list(
            index.open_bracket().as_ref(),
            index.close_bracket().as_ref(),
            items,
        )
    } else {
        compact_square_bracket_list(
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
    expression: &CollectionLiteralExpression<'source>,
) -> SquareArgumentListItems<'source> {
    let mut items = Vec::new();
    let mut has_recovered_tokens = false;

    for entry in expression.entries_with_recovered() {
        has_recovered_tokens |= push_square_argument_entry(&mut items, entry);
    }

    SquareArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn index_argument_items<'source>(
    index: &IndexExpression<'source>,
) -> SquareArgumentListItems<'source> {
    let mut items = Vec::new();
    let mut has_recovered_tokens = false;

    for entry in index.entries_with_recovered() {
        has_recovered_tokens |= push_square_argument_entry(&mut items, entry);
    }

    SquareArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn push_square_argument_entry<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    entry: RecoveredSeparatedListEntry<'source, jolt_kotlin_syntax::ValueArgumentEntry<'source>>,
) -> bool {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => {
            items.push(CommaListItem {
                doc: format_value_argument(&entry.argument),
                comma: entry.comma,
            });
            false
        }
        RecoveredSeparatedListEntry::Token(token) => {
            items.push(CommaListItem {
                doc: format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve),
                comma: None,
            });
            true
        }
        RecoveredSeparatedListEntry::Error(error) => {
            items.push(CommaListItem {
                doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            });
            true
        }
        RecoveredSeparatedListEntry::Node(node) => {
            items.push(CommaListItem {
                doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
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
    arguments: &ValueArgumentList<'source>,
) -> Doc<'source> {
    let ValueArgumentListItems {
        items,
        has_argument_leading_comments,
        has_recovered_tokens,
    } = value_argument_list_items(arguments);
    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    if has_argument_leading_comments || has_recovered_tokens {
        force_parenthesized_list(
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        )
    } else if has_trailing_comma
        || arguments
            .open_paren()
            .as_ref()
            .is_some_and(token_has_comments)
        || arguments
            .close_paren()
            .as_ref()
            .is_some_and(token_has_comments)
    {
        parenthesized_list(
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        )
    } else {
        compact_parenthesized_list(
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
    arguments: &ValueArgumentList<'source>,
) -> ValueArgumentListItems<'source> {
    let mut items = Vec::new();
    let mut previous_comma = None;
    let mut has_argument_leading_comments = false;
    let mut has_recovered_tokens = false;

    for entry in arguments.entries_with_recovered() {
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
                    doc: format_value_argument_with_leading(&entry.argument, leading),
                    comma: entry.comma,
                });
                previous_comma = entry.comma;
            }
            RecoveredSeparatedListEntry::Token(token) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve),
                    comma: None,
                });
                has_recovered_tokens = true;
            }
            RecoveredSeparatedListEntry::Error(error) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                    comma: None,
                });
                has_recovered_tokens = true;
            }
            RecoveredSeparatedListEntry::Node(node) => {
                items.push(CommaListItem {
                    doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
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

pub(crate) fn format_value_argument<'source>(argument: &ValueArgument<'source>) -> Doc<'source> {
    format_value_argument_with_leading(argument, LeadingTrivia::Preserve)
}

fn format_value_argument_with_leading<'source>(
    argument: &ValueArgument<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(expression) = argument.expression() else {
        return format_recovered_value_argument_tokens(argument, leading);
    };
    let has_named_assign = argument.assign_token().is_some();

    concat([
        format_value_argument_prefix(argument),
        format_expression_with_leading(
            &expression,
            if has_named_assign {
                LeadingTrivia::Preserve
            } else {
                leading
            },
        ),
    ])
}

fn format_recovered_value_argument_tokens<'source>(
    argument: &ValueArgument<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_token_sequence(argument.token_iter(), leading)
}

fn format_value_argument_prefix<'source>(argument: &ValueArgument<'source>) -> Doc<'source> {
    let mut docs = Vec::new();
    for token in argument.prefix_tokens() {
        match token.kind() {
            jolt_kotlin_syntax::KotlinSyntaxKind::Assign => {
                docs.push(space());
                docs.push(format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ));
                docs.push(space());
            }
            _ => docs.push(format_token(
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )),
        }
    }
    concat(docs)
}
