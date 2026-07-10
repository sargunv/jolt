use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    BlockItem, DestructuringDeclaration, LambdaExpression, LambdaParameter, LambdaParameterList,
    RecoveredSeparatedListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_separator_with_comments, format_token,
    format_token_sequence, token_has_comments,
};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, recovered_comma_list_items,
};
use crate::rules::names::format_name;
use crate::rules::statements::format_block_item;
use crate::rules::types::format_type_reference;

pub(super) fn format_lambda_expression<'source>(
    doc: &mut DocBuilder<'source>,
    lambda: &LambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(labeled) = format_labeled_lambda_expression(doc, lambda, leading) {
        return labeled;
    }

    let Some(open) = lambda.open_brace() else {
        return doc.nil();
    };
    let close = lambda.close_brace();

    let items = lambda.body_items().collect::<Vec<_>>();

    let open = format_token(doc, &open, leading, TrailingTrivia::Preserve);
    let close = if let Some(close) = close {
        format_token(
            doc,
            &close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };

    let parameter_prefix = lambda
        .parameter_list()
        .and_then(|parameters| format_lambda_parameter_prefix(doc, &parameters));
    let body = lambda_body_doc(doc, lambda, &items);
    if body.is_empty() {
        let empty_body_parameters = if let Some(prefix) = parameter_prefix {
            let before = doc.space();
            let after = doc.space();
            doc.concat([before, prefix, after])
        } else {
            doc.nil()
        };
        return doc.concat([open, empty_body_parameters, close]);
    }

    let body_doc_count = body.count;
    let body = body.doc.expect("non-empty lambda body has a doc");

    let block_parameters = if let Some(prefix) = parameter_prefix {
        let space = doc.space();
        doc.concat([space, prefix])
    } else {
        doc.nil()
    };
    let body_line = doc.hard_line();
    let block_body = doc.concat([body_line, body]);
    let block_body = doc.indent(block_body);
    let close_line = doc.hard_line();
    let block = doc.concat([open, block_parameters, block_body, close_line, close]);

    if items.len() == 1 && body_doc_count == 1 {
        let open_space = doc.space();
        let inline_parameters = if let Some(prefix) = parameter_prefix {
            let space = doc.space();
            doc.concat([prefix, space])
        } else {
            doc.nil()
        };
        let close_space = doc.space();
        let inline = doc.concat([
            open,
            open_space,
            inline_parameters,
            body,
            close_space,
            close,
        ]);
        let contents = doc.if_break(block, inline);
        return doc.group(contents);
    }

    block
}

fn format_labeled_lambda_expression<'source>(
    doc: &mut DocBuilder<'source>,
    lambda: &LambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let inner = lambda.inner_lambda()?;
    let label = lambda.label_token()?;
    let at = lambda.at_token()?;
    let at_has_comments = token_has_comments(&at);

    let label = format_token(
        doc,
        &label,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let at = format_token(doc, &at, LeadingTrivia::Preserve, TrailingTrivia::Preserve);
    let space = if at_has_comments {
        doc.space()
    } else {
        doc.nil()
    };
    let inner = format_lambda_expression(doc, &inner, LeadingTrivia::SuppressAlreadyHandled);
    Some(doc.concat([label, at, space, inner]))
}

fn format_lambda_parameter_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    parameter_list: &LambdaParameterList<'source>,
) -> Option<Doc<'source>> {
    let arrow = parameter_list.arrow_token()?;
    let mut entries = recovered_comma_list_items(
        doc,
        parameter_list.parameter_entries_with_recovered(),
        |doc, entry| CommaListItem {
            doc: format_lambda_parameter(doc, &entry.parameter),
            comma: entry.comma,
        },
    )
    .into_iter()
    .peekable();
    let result = doc.concat_list(|docs| {
        while let Some(entry) = entries.next() {
            docs.push(entry.doc);
            if let Some(comma) = entry.comma {
                let space = docs.space();
                let comma = format_separator_with_comments(docs, &comma, space);
                docs.push(comma);
            } else if entries.peek().is_some() {
                let space = docs.space();
                docs.push(space);
            }
        }

        if !docs.is_empty() {
            let space = docs.space();
            docs.push(space);
        }
        let arrow = format_token(
            docs,
            &arrow,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        docs.push(arrow);
    });

    Some(result)
}

fn format_lambda_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &LambdaParameter<'source>,
) -> Doc<'source> {
    let name = if let Some(declaration) = parameter.destructuring_declaration() {
        format_destructuring_declaration(doc, &declaration)
    } else if let Some(name) = parameter.name() {
        format_name(doc, &name)
    } else {
        doc.nil()
    };
    let colon = if let Some(colon) = parameter.colon() {
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        doc.concat([colon, space])
    } else {
        doc.nil()
    };
    let ty = if let Some(ty) = parameter.ty() {
        format_type_reference(doc, &ty)
    } else {
        doc.nil()
    };
    doc.concat([name, colon, ty])
}

fn format_destructuring_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: &DestructuringDeclaration<'source>,
) -> Doc<'source> {
    let items =
        recovered_comma_list_items(doc, declaration.entries_with_recovered(), |doc, entry| {
            CommaListItem {
                doc: if let Some(name) = entry.entry.name() {
                    format_name(doc, &name)
                } else {
                    doc.nil()
                },
                comma: entry.comma,
            }
        });
    compact_parenthesized_list(
        doc,
        declaration.open_delimiter().as_ref(),
        declaration.close_delimiter().as_ref(),
        items,
    )
}

pub(super) fn lambda_body_doc<'source>(
    doc: &mut DocBuilder<'source>,
    lambda: &LambdaExpression<'source>,
    items: &[BlockItem<'source>],
) -> LambdaBodyDoc<'source> {
    let mut count = 0;
    let mut entries = lambda.body_items_with_recovered().peekable();
    let body = doc.concat_list(|body| {
        while let Some(entry) = entries.next() {
            match entry {
                RecoveredSeparatedListEntry::Entry(item) => {
                    let item = format_block_item(body, &item);
                    push_lambda_body_doc(body, &mut count, item);
                }
                recovered_entry => {
                    let mut recovered_is_empty = true;
                    let recovered = body.concat_list(|recovered_docs| {
                        push_recovered_lambda_entry(recovered_docs, recovered_entry);
                        while entries.peek().is_some_and(|entry| {
                            !matches!(entry, RecoveredSeparatedListEntry::Entry(_))
                        }) {
                            let entry = entries.next().expect("peeked lambda body entry exists");
                            push_recovered_lambda_entry(recovered_docs, entry);
                        }
                        recovered_is_empty = recovered_docs.is_empty();
                    });
                    if !recovered_is_empty {
                        push_lambda_body_doc(body, &mut count, recovered);
                    }
                }
            }
        }

        if count == 0 {
            for item in items {
                let item = format_block_item(body, item);
                push_lambda_body_doc(body, &mut count, item);
            }
        }
    });

    LambdaBodyDoc {
        doc: (count > 0).then_some(body),
        count,
    }
}

fn push_recovered_lambda_entry<'source>(
    recovered_docs: &mut ConcatBuilder<'_, 'source>,
    entry: RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) {
    let recovered = match entry {
        RecoveredSeparatedListEntry::Entry(_) => return,
        RecoveredSeparatedListEntry::Token(token) => format_token_sequence(
            recovered_docs,
            std::iter::once(token),
            LeadingTrivia::Preserve,
        ),
        RecoveredSeparatedListEntry::Error(error) => {
            format_token_sequence(recovered_docs, error.token_iter(), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Node(node) => {
            format_token_sequence(recovered_docs, node.token_iter(), LeadingTrivia::Preserve)
        }
    };
    recovered_docs.push(recovered);
}

fn push_lambda_body_doc<'source>(
    body: &mut ConcatBuilder<'_, 'source>,
    count: &mut usize,
    item: Doc<'source>,
) {
    if *count > 0 {
        let hard_line = body.hard_line();
        body.push(hard_line);
    }
    body.push(item);
    *count += 1;
}

pub(super) struct LambdaBodyDoc<'source> {
    pub(super) doc: Option<Doc<'source>>,
    pub(super) count: usize,
}

impl LambdaBodyDoc<'_> {
    pub(super) const fn is_empty(&self) -> bool {
        self.count == 0
    }
}
