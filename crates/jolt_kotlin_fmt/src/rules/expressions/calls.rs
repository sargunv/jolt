use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Annotation, CallExpression, CollectionLiteralExpression, Expression, IndexExpression,
    KotlinRoleElement, KotlinSyntaxField, KotlinSyntaxInvariantError, KotlinSyntaxListPart,
    KotlinSyntaxToken, KotlinSyntaxView, NavigationExpression, SplitSafeNavigationOperator,
    ValueArgument, ValueArgumentList, ValueArgumentListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_token, token_has_comments,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, compact_square_bracket_list,
    force_parenthesized_list, parenthesized_list, square_bracket_list,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, KotlinFormatListPart, format_optional_field,
    format_or_verbatim, format_required_field, resolve_list_part, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::annotations::format_annotation;
use crate::rules::names::format_name;
use crate::rules::types::format_type_argument_list;

use super::{format_expression_with_leading, lambdas::format_lambda_expression};

pub(super) fn format_navigation_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NavigationExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let expression = Expression::NavigationExpression(*expression);
        if let Some(chain) = format_member_chain(doc, expression, leading) {
            return chain;
        }
        let Expression::NavigationExpression(expression) = expression else {
            unreachable!()
        };
        let receiver = format_required_field(expression.receiver(), doc, |receiver, doc| {
            format_expression_with_leading(doc, &receiver, leading)
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
    })
}

pub(super) fn format_call_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let expression = Expression::CallExpression(*expression);
        if let Some(chain) = format_member_chain(doc, expression, leading) {
            return chain;
        }
        let Expression::CallExpression(expression) = expression else {
            unreachable!()
        };
        let callee = format_required_field(expression.callee(), doc, |callee, doc| {
            format_expression_with_leading(doc, &callee, leading)
        });
        let suffix = format_call_arguments(doc, &expression);
        doc.concat([callee, suffix])
    })
}

pub(super) fn format_index_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IndexExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let expression_family = Expression::IndexExpression(*expression);
        if let Some(chain) = format_member_chain(doc, expression_family, leading) {
            return chain;
        }
        let receiver = format_required_field(expression.receiver(), doc, |receiver, doc| {
            format_expression_with_leading(doc, &receiver, leading)
        });
        let suffix = format_index_suffix(doc, expression);
        let contents = doc.concat([receiver, suffix]);
        doc.group(contents)
    })
}

pub(super) fn format_collection_literal_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CollectionLiteralExpression<'source>,
    _leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        format_square_argument_list(
            doc,
            expression.open_bracket(),
            expression.entries(),
            expression.close_bracket(),
        )
    })
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
            && (self.suffix_count == 0
                || !self.first_suffix_forces_break
                || matches!(root, Expression::CallExpression(_)));
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
        if self.suffix_count == 0 {
            self.first_suffix = Some(suffix);
            self.first_suffix_forces_break = forces_break;
        } else {
            let line = rest.soft_line();
            rest.push(line);
            rest.push(suffix);
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
    let rest = doc.concat_list(|rest| {
        valid = append_chain_expression(rest, &mut builder, expression).is_some();
        if valid {
            builder.flush_field_run(rest);
        }
    });
    valid.then(|| builder.finish(doc, leading, rest)).flatten()
}

fn append_chain_expression<'source>(
    rest: &mut ConcatBuilder<'_, 'source>,
    builder: &mut MemberChainBuilder<'source>,
    expression: Expression<'source>,
) -> Option<()> {
    match expression {
        Expression::CallExpression(call) => {
            let callee = present_required(call.callee())?;
            let Expression::NavigationExpression(navigation) = callee else {
                return None;
            };
            append_chain_receiver(rest, builder, present_required(navigation.receiver())?);
            let forces_break = navigation_operator_has_leading_comments(&navigation)
                || (call_has_lambdas(&call) && !call_has_parenthesized_arguments(&call));
            let navigation = format_navigation_suffix(rest, &navigation)?;
            let arguments = format_call_arguments(rest, &call);
            let suffix = rest.concat([navigation, arguments]);
            builder.push_suffix(rest, suffix, forces_break);
            Some(())
        }
        Expression::NavigationExpression(navigation) => {
            append_chain_receiver(rest, builder, present_required(navigation.receiver())?);
            let suffix = format_navigation_suffix(rest, &navigation)?;
            let forces_break = navigation_operator_has_leading_comments(&navigation);
            builder.push_navigation_suffix(rest, suffix, forces_break);
            Some(())
        }
        Expression::IndexExpression(index) => {
            append_chain_receiver(rest, builder, present_required(index.receiver())?);
            let suffix = format_index_suffix(rest, &index);
            builder.push_suffix(rest, suffix, false);
            Some(())
        }
        _ => None,
    }
}

fn append_chain_receiver<'source>(
    rest: &mut ConcatBuilder<'_, 'source>,
    builder: &mut MemberChainBuilder<'source>,
    receiver: Expression<'source>,
) {
    if append_chain_expression(rest, builder, receiver).is_none() {
        builder.root = Some(receiver);
    }
}

fn member_chain<'source>(
    doc: &mut DocBuilder<'source>,
    root: Doc<'source>,
    first_suffix: Option<Doc<'source>>,
    rest: Doc<'source>,
    suffix_count: u32,
    force_multiline: bool,
    keep_first: bool,
) -> Doc<'source> {
    if suffix_count == 0 {
        return root;
    }
    let first = first_suffix.expect("member chain suffix exists");
    let head = if keep_first {
        doc.concat([root, first])
    } else {
        root
    };
    let rest = if keep_first {
        rest
    } else {
        let line = doc.soft_line();
        doc.concat([line, first, rest])
    };
    if keep_first && suffix_count == 1 {
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
                    KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
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
                    KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
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
    operator: KotlinRoleElement<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    if let Some(token) = operator.token() {
        return format_token(doc, &token, leading, trailing);
    }
    if let Some(split) = operator.cast_node::<SplitSafeNavigationOperator<'source>>() {
        return format_or_verbatim(&split, doc, |doc| {
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
        });
    }
    doc.block_on_invariant("invalid navigation operator");
    Doc::nil()
}

fn format_navigation_selector<'source>(
    doc: &mut DocBuilder<'source>,
    selector: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(token) = selector.token() {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else if let Some(expression) = selector.cast_family::<Expression<'source>>() {
        format_expression_with_leading(doc, &expression, LeadingTrivia::Preserve)
    } else {
        doc.block_on_invariant("invalid navigation selector");
        Doc::nil()
    }
}

fn navigation_operator_has_leading_comments(navigation: &NavigationExpression<'_>) -> bool {
    present_required(navigation.operator())
        .and_then(|operator| {
            operator.token().or_else(|| {
                operator
                    .cast_node::<SplitSafeNavigationOperator<'_>>()
                    .and_then(|operator| operator.first_token())
            })
        })
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

fn format_square_argument_list<'source, Entries>(
    doc: &mut DocBuilder<'source>,
    open: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        KotlinSyntaxInvariantError,
    >,
    entries: Result<KotlinSyntaxField<'source, Entries>, KotlinSyntaxInvariantError>,
    close: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        KotlinSyntaxInvariantError,
    >,
) -> Doc<'source>
where
    Entries: PhysicalValueArgumentList<'source>,
{
    let open = resolve_required_delimiter(open, doc);
    let close = resolve_required_delimiter(close, doc);
    let (items, recovered) = match resolve_required_field(entries, doc) {
        KotlinFormatField::Present(entries) => value_argument_items(doc, entries.parts()),
        KotlinFormatField::Malformed(recovery) => (
            vec![CommaListItem {
                doc: recovery,
                comma: None,
            }],
            true,
        ),
    };
    let trailing = items.last().is_some_and(|item| item.comma.is_some());
    let list = if trailing || recovered {
        square_bracket_list(doc, open.source(), close.source(), items)
    } else {
        compact_square_bracket_list(doc, open.source(), close.source(), items)
    };
    concat_delimiter_recovery(doc, &open, list, &close)
}

trait PhysicalValueArgumentList<'source> {
    fn parts(
        &self,
    ) -> impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, ValueArgument<'source>>,
            KotlinSyntaxInvariantError,
        >,
    > + '_;
}

impl<'source> PhysicalValueArgumentList<'source>
    for jolt_kotlin_syntax::ValueArgumentSeparatedList<'source>
{
    fn parts(
        &self,
    ) -> impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, ValueArgument<'source>>,
            KotlinSyntaxInvariantError,
        >,
    > + '_ {
        self.parts()
    }
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
        Ok(KotlinSyntaxField::Present(lambdas)) => lambdas
            .parts()
            .any(|part| matches!(part, Ok(KotlinSyntaxListPart::Item(_)))),
        Ok(KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_)) | Err(_) => false,
    }
}

fn call_has_parenthesized_arguments(call: &CallExpression<'_>) -> bool {
    matches!(call.arguments(), Ok(KotlinSyntaxField::Present(_)))
}

pub(crate) fn format_value_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &ValueArgumentList<'source>,
) -> Doc<'source> {
    format_or_verbatim(arguments, doc, |doc| {
        let open = resolve_required_delimiter(arguments.open_paren(), doc);
        let close = resolve_required_delimiter(arguments.close_paren(), doc);
        let (items, has_recovery) = match resolve_required_field(arguments.entries(), doc) {
            KotlinFormatField::Present(entries) => {
                value_argument_list_entry_items(doc, entries.parts())
            }
            KotlinFormatField::Malformed(recovery) => (
                vec![CommaListItem {
                    doc: recovery,
                    comma: None,
                }],
                true,
            ),
        };
        let has_comments = items.iter().any(|item| item.doc != Doc::nil())
            && value_argument_list_has_leading_comments(arguments);
        let trailing = items.last().is_some_and(|item| item.comma.is_some());
        let delimiter_comments = open.source().is_some_and(token_has_comments)
            || close.source().is_some_and(token_has_comments);
        let list = if has_comments || has_recovery {
            force_parenthesized_list(doc, open.source(), close.source(), items)
        } else if trailing || delimiter_comments {
            parenthesized_list(doc, open.source(), close.source(), items)
        } else {
            compact_parenthesized_list(doc, open.source(), close.source(), items)
        };
        concat_delimiter_recovery(doc, &open, list, &close)
    })
}

fn value_argument_list_entry_items<'source>(
    doc: &mut DocBuilder<'source>,
    parts: impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, ValueArgumentListEntry<'source>>,
            KotlinSyntaxInvariantError,
        >,
    >,
) -> (Vec<CommaListItem<'source>>, bool) {
    let mut items = Vec::new();
    let mut recovered = false;
    for part in parts {
        let empty_malformed = matches!(
            &part,
            Ok(KotlinSyntaxListPart::Malformed(malformed)) if malformed.first_token().is_none()
        );
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
                items.push(CommaListItem {
                    doc: formatted,
                    comma: None,
                });
            }
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                } else {
                    recovered = true;
                    let comma = format_token(
                        doc,
                        &comma,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    );
                    items.push(CommaListItem {
                        doc: comma,
                        comma: None,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => {
                recovered = true;
                if empty_malformed && let Some(item) = items.last_mut() {
                    item.doc = doc.concat([item.doc, recovery]);
                } else {
                    items.push(CommaListItem {
                        doc: recovery,
                        comma: None,
                    });
                }
            }
        }
    }
    (items, recovered)
}

fn value_argument_list_has_leading_comments(arguments: &ValueArgumentList<'_>) -> bool {
    let Some(entries) = present_required(arguments.entries()) else {
        return false;
    };
    entries.parts().any(|part| match part {
        Ok(KotlinSyntaxListPart::Item(argument)) => argument
            .first_token()
            .is_some_and(|token| !token.leading_comments().is_empty()),
        _ => false,
    })
}

fn value_argument_items<'source>(
    doc: &mut DocBuilder<'source>,
    parts: impl Iterator<
        Item = Result<
            KotlinSyntaxListPart<'source, ValueArgument<'source>>,
            KotlinSyntaxInvariantError,
        >,
    >,
) -> (Vec<CommaListItem<'source>>, bool) {
    let mut items = Vec::new();
    let mut recovered = false;
    for part in parts {
        let empty_malformed = matches!(
            &part,
            Ok(KotlinSyntaxListPart::Malformed(malformed)) if malformed.first_token().is_none()
        );
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(argument) => items.push(CommaListItem {
                doc: format_value_argument(doc, &argument),
                comma: None,
            }),
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                } else {
                    recovered = true;
                    let comma = format_token(
                        doc,
                        &comma,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    );
                    items.push(CommaListItem {
                        doc: comma,
                        comma: None,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => {
                recovered = true;
                if empty_malformed && let Some(item) = items.last_mut() {
                    item.doc = doc.concat([item.doc, recovery]);
                } else {
                    items.push(CommaListItem {
                        doc: recovery,
                        comma: None,
                    });
                }
            }
        }
    }
    (items, recovered)
}

pub(crate) fn format_value_argument<'source>(
    doc: &mut DocBuilder<'source>,
    argument: &ValueArgument<'source>,
) -> Doc<'source> {
    format_or_verbatim(argument, doc, |doc| {
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
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            })
        });
        let name = format_optional_field(argument.name(), doc, |name, doc| format_name(doc, &name));
        let assign = format_optional_field(argument.assign(), doc, |assign, doc| {
            let before = doc.space();
            let assign = format_token(
                doc,
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            let after = doc.space();
            doc.concat([before, assign, after])
        });
        let expression = format_required_field(argument.expression(), doc, |expression, doc| {
            format_expression_with_leading(doc, &expression, LeadingTrivia::Preserve)
        });
        doc.concat([prefix, name, assign, expression])
    })
}

fn format_value_argument_prefix_item<'source>(
    doc: &mut DocBuilder<'source>,
    role: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(token) = role.token() {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else if let Some(annotation) = role.cast_node::<Annotation<'source>>() {
        format_annotation(doc, &annotation)
    } else {
        doc.block_on_invariant("invalid value-argument prefix");
        Doc::nil()
    }
}

fn present_required<T>(
    field: Result<KotlinSyntaxField<'_, T>, KotlinSyntaxInvariantError>,
) -> Option<T> {
    match field.ok()? {
        KotlinSyntaxField::Present(value) => Some(value),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => None,
    }
}

fn delimiter_recovery<'source>(delimiter: &KotlinFormatDelimiter<'source>) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(_) => Doc::nil(),
        KotlinFormatDelimiter::Recovery(recovery) => *recovery,
    }
}

fn concat_delimiter_recovery<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    list: Doc<'source>,
    close: &KotlinFormatDelimiter<'source>,
) -> Doc<'source> {
    let open = delimiter_recovery(open);
    let close = delimiter_recovery(close);
    doc.concat([open, list, close])
}
