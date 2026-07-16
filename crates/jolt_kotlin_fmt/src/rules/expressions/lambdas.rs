use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    BlockItem, KotlinSyntaxView, LabeledLambdaExpression, LambdaBody, LambdaExpression, LambdaForm,
    LambdaParameter, LambdaParameterBindingSyntax, LambdaParameterList, LambdaParameterListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_dangling_comments, format_leading_comments,
    format_separator_with_comments, format_token, token_has_comments,
};
use crate::helpers::lists::{CommaListItem, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_optional_field, format_required_field,
    resolve_list_part, resolve_required_field,
};
use crate::rules::declarations::format_destructuring_declaration;
use crate::rules::names::format_name_with_leading;
use crate::rules::statements::format_block_item;
use crate::rules::types::format_type_reference;

pub(super) fn format_lambda_expression<'source>(
    doc: &mut DocBuilder<'source>,
    lambda: &LambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(lambda.form(), doc, |form, doc| match form {
        LambdaForm::LabeledLambdaExpression(labeled) => {
            format_labeled_lambda_expression(doc, &labeled, leading)
        }
        LambdaForm::LambdaBody(body) => format_lambda_body(doc, &body, leading),
        LambdaForm::BogusLambdaForm(bogus) => {
            crate::helpers::recovery::format_malformed(&bogus, doc)
        }
    })
}

fn format_labeled_lambda_expression<'source>(
    doc: &mut DocBuilder<'source>,
    labeled: &LabeledLambdaExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let label = format_required_field(labeled.label(), doc, |label, doc| {
        format_token(
            doc,
            &label,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let at_has_comments = match labeled.at() {
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(at)) => token_has_comments(&at),
        _ => false,
    };
    let at = format_required_field(labeled.at(), doc, |at, doc| {
        format_token(doc, &at, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    });
    let space = if at_has_comments {
        doc.space()
    } else {
        doc.nil()
    };
    let lambda = format_required_field(labeled.lambda(), doc, |lambda, doc| {
        format_lambda_expression(doc, &lambda, LeadingTrivia::SuppressAlreadyHandled)
    });
    doc.concat([label, at, space, lambda])
}

fn format_lambda_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: &LambdaBody<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let close_source = match body.close_brace() {
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(close)) => Some(close),
        _ => None,
    };
    let open = format_required_field(body.open_brace(), doc, |open, doc| {
        format_token(doc, &open, leading, TrailingTrivia::Preserve)
    });
    let close = format_required_field(body.close_brace(), doc, |close, doc| {
        format_token(
            doc,
            &close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    });
    let parameters = match crate::helpers::recovery::resolve_optional_field(body.parameters(), doc)
    {
        KotlinFormatField::Present(Some(parameters)) => {
            Some(format_lambda_parameter_prefix(doc, &parameters))
        }
        KotlinFormatField::Present(None) => None,
        KotlinFormatField::Malformed(recovery) => Some(recovery),
    };
    let body_doc = lambda_body_doc(doc, body, close_source.as_ref());
    if body_doc.is_empty() {
        let parameters = if let Some(parameters) = parameters {
            let before = doc.space();
            let after = doc.space();
            doc.concat([before, parameters, after])
        } else {
            doc.nil()
        };
        return doc.concat([open, parameters, close]);
    }

    let count = body_doc.count;
    let contents = body_doc.doc.expect("non-empty lambda body has a doc");
    let block_parameters = if let Some(parameters) = parameters {
        let space = doc.space();
        doc.concat([space, parameters])
    } else {
        doc.nil()
    };
    let body_line = doc.hard_line();
    let block_body = doc.concat([body_line, contents]);
    let block_body = doc.indent(block_body);
    let close_line = doc.hard_line();
    let block = doc.concat([open, block_parameters, block_body, close_line, close]);

    if count == 1 {
        let open_space = doc.space();
        let inline_parameters = if let Some(parameters) = parameters {
            let space = doc.space();
            doc.concat([parameters, space])
        } else {
            doc.nil()
        };
        let close_space = doc.space();
        let inline = doc.concat([
            open,
            open_space,
            inline_parameters,
            contents,
            close_space,
            close,
        ]);
        let contents = doc.if_break(block, inline);
        return doc.group(contents);
    }
    block
}

fn format_lambda_parameter_prefix<'source>(
    doc: &mut DocBuilder<'source>,
    parameter_list: &LambdaParameterList<'source>,
) -> Doc<'source> {
    let items = match resolve_required_field(parameter_list.parameters(), doc) {
        KotlinFormatField::Present(parameters) => physical_comma_list_items(
            doc,
            parameters.parts().filter(|part| match part {
                Ok(jolt_kotlin_syntax::KotlinSyntaxListPart::Item(
                    LambdaParameterListEntry::BogusLambdaParameter(bogus),
                )) => bogus.first_token().is_some(),
                Ok(jolt_kotlin_syntax::KotlinSyntaxListPart::Malformed(malformed)) => {
                    malformed.first_token().is_some()
                }
                _ => true,
            }),
            |doc, parameter| CommaListItem {
                doc: match parameter {
                    LambdaParameterListEntry::LambdaParameter(parameter) => {
                        format_lambda_parameter(doc, &parameter)
                    }
                    LambdaParameterListEntry::BogusLambdaParameter(bogus) => {
                        crate::helpers::recovery::format_malformed(&bogus, doc)
                    }
                },
                comma: None,
            },
        ),
        KotlinFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
        }],
    };
    let arrow = format_required_field(parameter_list.arrow(), doc, |arrow, doc| {
        format_token(
            doc,
            &arrow,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let mut items = items.into_iter().peekable();
    doc.concat_list(|docs| {
        while let Some(item) = items.next() {
            docs.push(item.doc);
            if let Some(comma) = item.comma {
                let space = docs.space();
                let comma = format_separator_with_comments(docs, &comma, space);
                docs.push(comma);
            } else if items.peek().is_some() {
                let space = docs.space();
                docs.push(space);
            }
        }
        if !docs.is_empty() {
            let space = docs.space();
            docs.push(space);
        }
        docs.push(arrow);
    })
}

fn format_lambda_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &LambdaParameter<'source>,
) -> Doc<'source> {
    let binding = format_required_field(parameter.binding(), doc, |binding, doc| {
        format_required_field(binding.binding(), doc, |binding, doc| {
            match binding.classify() {
                Ok(LambdaParameterBindingSyntax::Name(name)) => {
                    let comments = name
                        .first_token()
                        .map_or_else(Doc::nil, |token| format_leading_comments(doc, &token));
                    let name =
                        format_name_with_leading(doc, &name, LeadingTrivia::SuppressAlreadyHandled);
                    doc.concat([comments, name])
                }
                Ok(LambdaParameterBindingSyntax::Destructuring(pattern)) => {
                    format_destructuring_declaration(doc, &pattern)
                }
                Err(error) => {
                    doc.block_on_invariant(error.to_string());
                    Doc::nil()
                }
            }
        })
    });
    let colon = format_optional_field(parameter.colon(), doc, |colon, doc| {
        let colon = format_token(
            doc,
            &colon,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let space = doc.space();
        doc.concat([colon, space])
    });
    let ty = format_optional_field(parameter.r#type(), doc, |ty, doc| {
        format_type_reference(doc, &ty)
    });
    doc.concat([binding, colon, ty])
}

pub(super) fn lambda_body_doc<'source>(
    doc: &mut DocBuilder<'source>,
    body: &LambdaBody<'source>,
    close: Option<&jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> LambdaBodyDoc<'source> {
    let mut count = 0;
    let body_doc = match resolve_required_field(body.items(), doc) {
        KotlinFormatField::Present(items) => doc.concat_list(|docs| {
            for part in items.parts() {
                let item = match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(role) => {
                        if let Some(item) = role.cast_family::<BlockItem<'source>>() {
                            format_block_item(docs, &item)
                        } else if let Some(token) = role.token() {
                            format_token(
                                docs,
                                &token,
                                LeadingTrivia::Preserve,
                                TrailingTrivia::Preserve,
                            )
                        } else {
                            docs.block_on_invariant("invalid lambda-body item");
                            Doc::nil()
                        }
                    }
                    KotlinFormatListPart::Separator(separator) => {
                        docs.block_on_invariant(format!(
                            "unexpected lambda-body separator: {:?}",
                            separator.kind()
                        ));
                        Doc::nil()
                    }
                    KotlinFormatListPart::Malformed(recovery) => recovery,
                };
                if item != Doc::nil() {
                    push_lambda_body_doc(docs, &mut count, item);
                }
            }
            if let Some(close) = close {
                let comments = close.leading_comments().collect::<Vec<_>>();
                if !comments.is_empty() {
                    let comments = format_dangling_comments(docs, comments);
                    push_lambda_body_doc(docs, &mut count, comments);
                }
            }
        }),
        KotlinFormatField::Malformed(recovery) => {
            count = 1;
            recovery
        }
    };
    LambdaBodyDoc {
        doc: (count > 0).then_some(body_doc),
        count,
    }
}

fn push_lambda_body_doc<'source>(
    body: &mut ConcatBuilder<'_, 'source>,
    count: &mut usize,
    item: Doc<'source>,
) {
    if *count > 0 {
        let line = body.hard_line();
        body.push(line);
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
