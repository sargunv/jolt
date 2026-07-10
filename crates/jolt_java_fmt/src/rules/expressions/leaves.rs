use super::{
    ClassLiteralExpression, Doc, Expression, InlineLeadingTrivia, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, LiteralExpression, NameExpression, SuperExpression, TemplateExpression,
    ThisExpression, TrailingTrivia, format_annotation, format_array_dimensions, format_expression,
    format_member_dot, format_token, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, format_void_type,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_literal_expression<'source>(
    expression: &LiteralExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    expression.literal_token().map_or_else(Doc::nil, |token| {
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
            expression
                .processor()
                .map_or_else(Doc::nil, |processor| format_expression(&processor, doc),),
            format_member_dot(expression.dot_token().as_ref(), doc),
            expression
                .template()
                .map_or_else(Doc::nil, |template| format_literal_expression(
                    &template,
                    LeadingComments::Preserve,
                    doc
                ),),
        ]
    )
}

pub(super) fn format_name_expression<'source>(
    expression: &NameExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_annotation_run(expression.annotations(), doc);
    let name = expression.name().map_or_else(Doc::nil, |name| {
        format_leaf_token(&name, leading_comments, doc)
    });

    if let Some(annotations) = annotations {
        doc_concat!(doc, [annotations, doc.space(), name])
    } else {
        name
    }
}

fn format_annotation_run<'source>(
    annotations: impl IntoIterator<Item = jolt_java_syntax::Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut has_annotations = false;
    let docs = doc.concat_list(|docs| {
        for annotation in annotations {
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            let annotation = format_annotation(&annotation, docs);
            docs.push(annotation);
        }
        has_annotations = !docs.is_empty();
    });

    has_annotations.then_some(docs)
}

pub(super) fn format_this_expression<'source>(
    expression: &ThisExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(Doc::nil, |token| {
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
    let dot = expression.dot_token();

    format_qualified_keyword_expression(
        expression.qualifier(),
        dot.as_ref(),
        expression.keyword().map_or_else(Doc::nil, |token| {
            format_leaf_token(&token, leading_comments, doc)
        }),
        doc,
    )
}

fn format_qualified_keyword_expression<'source>(
    qualifier: Option<Expression<'source>>,
    dot: Option<&JavaSyntaxToken<'source>>,
    keyword: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            qualifier.map_or_else(Doc::nil, |qualifier| format_expression(&qualifier, doc),),
            dot.map_or_else(Doc::nil, |dot| format_member_dot(Some(dot), doc)),
            keyword,
        ]
    )
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
    let target = match expression.target_expression() {
        Some(target) => format_expression(&target, doc),
        None => match expression.void_type() {
            Some(ty) => format_void_type(&ty, doc),
            None => expression
                .primitive_keyword()
                .map_or_else(Doc::nil, |keyword| {
                    format_leaf_token(&keyword, LeadingComments::Preserve, doc)
                }),
        },
    };

    doc_concat!(
        doc,
        [
            target,
            expression
                .dimensions()
                .map_or_else(Doc::nil, |dimensions| format_array_dimensions(
                    &dimensions,
                    doc
                ),),
            expression
                .dot_token()
                .as_ref()
                .map_or_else(Doc::nil, |dot| format_class_literal_dot(dot, doc)),
            expression.class_token().map_or_else(Doc::nil, |token| {
                format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                )
            },),
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
