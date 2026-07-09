use jolt_fmt_ir::{Doc, DocBuilder, DocList};
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
    let mut docs = doc.list();

    while let Some(entry) = entries.next() {
        docs.push(entry.doc, doc);
        if let Some(comma) = entry.comma {
            let space = doc.space();
            let comma = format_separator_with_comments(doc, &comma, space);
            docs.push(comma, doc);
        } else if entries.peek().is_some() {
            let space = doc.space();
            docs.push(space, doc);
        }
    }

    if !docs.is_empty() {
        let space = doc.space();
        docs.push(space, doc);
    }
    let arrow = format_token(
        doc,
        &arrow,
        LeadingTrivia::SuppressAlreadyHandled,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    docs.push(arrow, doc);

    Some(docs.finish(doc))
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
    let mut body = doc.list();
    let mut count = 0;
    let mut recovered_docs = doc.list();

    for entry in lambda.body_items_with_recovered() {
        match entry {
            RecoveredSeparatedListEntry::Entry(item) => {
                push_recovered_lambda_docs(doc, &mut body, &mut count, &mut recovered_docs);
                let item = format_block_item(doc, &item);
                push_lambda_body_doc(doc, &mut body, &mut count, item);
            }
            RecoveredSeparatedListEntry::Token(token) => {
                let token =
                    format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve);
                recovered_docs.push(token, doc);
            }
            RecoveredSeparatedListEntry::Error(error) => {
                let error = format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve);
                recovered_docs.push(error, doc);
            }
            RecoveredSeparatedListEntry::Node(node) => {
                let node = format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve);
                recovered_docs.push(node, doc);
            }
        }
    }

    push_recovered_lambda_docs(doc, &mut body, &mut count, &mut recovered_docs);

    if count == 0 {
        for item in items {
            let item = format_block_item(doc, item);
            push_lambda_body_doc(doc, &mut body, &mut count, item);
        }
    }

    LambdaBodyDoc {
        doc: (count > 0).then(|| body.finish(doc)),
        count,
    }
}

fn push_recovered_lambda_docs<'source>(
    doc: &mut DocBuilder<'source>,
    body: &mut DocList<'source>,
    count: &mut usize,
    recovered_docs: &mut DocList<'source>,
) {
    if recovered_docs.is_empty() {
        return;
    }

    let empty = doc.list();
    let recovered = std::mem::replace(recovered_docs, empty).finish(doc);
    push_lambda_body_doc(doc, body, count, recovered);
}

fn push_lambda_body_doc<'source>(
    doc: &mut DocBuilder<'source>,
    body: &mut DocList<'source>,
    count: &mut usize,
    item: Doc<'source>,
) {
    if *count > 0 {
        let hard_line = doc.hard_line();
        body.push(hard_line, doc);
    }
    body.push(item, doc);
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
