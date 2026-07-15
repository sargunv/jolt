use super::{
    ClassLiteralExpression, Doc, Expression, InlineLeadingTrivia, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, LiteralExpression, NameExpression, SuperExpression, TemplateExpression,
    ThisExpression, TrailingTrivia, format_annotation, format_array_dimensions, format_expression,
    format_member_dot, format_token, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, format_void_type,
};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_optional_field, format_required_field,
    resolve_list_part, resolve_optional_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::JavaSyntaxListPart;

pub(super) fn format_literal_expression<'source>(
    expression: &LiteralExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(expression.literal(), doc, |token, doc| {
        format_leaf_token(&token, leading_comments, doc)
    })
}

pub(super) fn format_template_expression<'source>(
    expression: &TemplateExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(expression.processor(), doc, |processor, doc| {
                format_expression(&processor, doc)
            }),
            format_required_field(expression.dot(), doc, |dot, doc| {
                format_member_dot(Some(&dot), doc)
            }),
            format_required_field(expression.template(), doc, |template, doc| {
                format_literal_expression(&template, LeadingComments::Preserve, doc)
            }),
        ]
    )
}

pub(super) fn format_name_expression<'source>(
    expression: &NameExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = match resolve_optional_field(expression.annotations(), doc) {
        JavaFormatField::Present(Some(annotations)) => {
            format_annotation_parts(annotations.parts(), doc)
        }
        JavaFormatField::Present(None) => None,
        JavaFormatField::Malformed(recovery) => Some(recovery),
    };
    let name = format_required_field(expression.identifier(), doc, |name, doc| {
        format_leaf_token(&name, leading_comments, doc)
    });

    if let Some(annotations) = annotations {
        doc_concat!(doc, [annotations, doc.space(), name])
    } else {
        name
    }
}

fn format_annotation_parts<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, jolt_java_syntax::Annotation<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut has_parts = false;
    let result = doc.concat_list(|docs| {
        for part in parts {
            has_parts = true;
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(annotation) => format_annotation(&annotation, docs),
                JavaFormatListPart::Separator(separator) => {
                    crate::helpers::comments::format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
        }
    });
    has_parts.then_some(result)
}

pub(super) fn format_this_expression<'source>(
    expression: &ThisExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_qualified_keyword_expression(
        format_optional_field(expression.qualifier(), doc, |qualifier, doc| {
            format_expression(&qualifier, doc)
        }),
        format_optional_field(expression.dot(), doc, |dot, doc| {
            format_member_dot(Some(&dot), doc)
        }),
        format_required_field(expression.this_keyword(), doc, |token, doc| {
            format_leaf_token(&token, leading_comments, doc)
        }),
        doc,
    )
}

pub(super) fn format_super_expression<'source>(
    expression: &SuperExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_qualified_keyword_expression(
        format_optional_field(expression.qualifier(), doc, |qualifier, doc| {
            format_expression(&qualifier, doc)
        }),
        format_optional_field(expression.dot(), doc, |dot, doc| {
            format_member_dot(Some(&dot), doc)
        }),
        format_required_field(expression.super_keyword(), doc, |token, doc| {
            format_leaf_token(&token, leading_comments, doc)
        }),
        doc,
    )
}

fn format_qualified_keyword_expression<'source>(
    qualifier: Doc<'source>,
    dot: Doc<'source>,
    keyword: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(doc, [qualifier, dot, keyword,])
}

pub(super) fn format_leaf_token<'source>(
    token: &jolt_java_syntax::JavaSyntaxToken<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match leading_comments {
        LeadingComments::Preserve => format_token(
            doc,
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        LeadingComments::SuppressFirstToken => {
            format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
        }
    }
}

pub(super) fn format_class_literal_expression<'source>(
    expression: &ClassLiteralExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let target = format_required_field(expression.target(), doc, |target, doc| {
        if let Some(target) = target.cast_family::<Expression<'source>>() {
            format_expression(&target, doc)
        } else if let Some(ty) = target.cast_node::<jolt_java_syntax::VoidType<'source>>() {
            format_void_type(&ty, doc)
        } else if let Some(keyword) = target.token() {
            format_leaf_token(&keyword, LeadingComments::Preserve, doc)
        } else {
            doc.block_on_invariant("class literal target had an unknown shape");
            Doc::nil()
        }
    });

    doc_concat!(
        doc,
        [
            target,
            format_optional_field(expression.dimensions(), doc, |dimensions, doc| {
                format_array_dimensions(&dimensions, doc)
            }),
            format_required_field(expression.dot(), doc, |dot, doc| {
                format_class_literal_dot(&dot, doc)
            }),
            format_required_field(expression.class_keyword(), doc, |token, doc| {
                format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                )
            }),
        ]
    )
}

fn format_class_literal_dot<'source>(
    dot: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_token_with_inline_leading_comments(
                doc,
                dot,
                InlineLeadingTrivia::AfterPreviousToken,
                TrailingTrivia::BeforeLineBreak,
            ),
            if dot
                .trailing_comments()
                .any(|comment| super::comment_forces_line(&comment))
            {
                doc.hard_line()
            } else if dot.trailing_comments().is_empty() {
                Doc::nil()
            } else {
                doc.space()
            },
        ]
    )
}
