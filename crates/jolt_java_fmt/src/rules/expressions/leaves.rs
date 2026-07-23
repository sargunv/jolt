use super::{
    ClassLiteralExpression, Doc, InlineLeadingTrivia, JavaSyntaxToken, LeadingComments,
    LeadingTrivia, LiteralExpression, NameExpression, SuperExpression, ThisExpression,
    TrailingTrivia, format_annotation, format_array_dimensions, format_expression,
    format_member_dot, format_token, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, format_type, format_void_type,
};
use crate::helpers::recovery::{
    JavaFormatListPart, format_malformed, format_optional_field, format_required_field,
    resolve_list_part,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    ClassLiteralTargetSyntax, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxView,
};

pub(super) fn format_literal_expression<'source>(
    expression: &LiteralExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(expression.literal(), doc, |token, doc| {
        format_leaf_token(&token, leading_comments, doc)
    })
}

pub(super) fn format_name_expression<'source>(
    expression: &NameExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (annotations, annotations_visible) = match expression.annotations() {
        JavaSyntaxField::Present(annotations) => format_annotation_parts(annotations.parts(), doc),
        JavaSyntaxField::Missing(_) => (Doc::nil(), false),
        JavaSyntaxField::Malformed(malformed) => {
            let visible = malformed.first_token().is_some();
            (format_malformed(&malformed, doc), visible)
        }
    };
    let name = format_required_field(expression.identifier(), doc, |name, doc| {
        format_leaf_token(&name, leading_comments, doc)
    });

    if annotations_visible {
        doc_concat!(doc, [annotations, doc.space(), name])
    } else {
        doc_concat!(doc, [annotations, name])
    }
}

fn format_annotation_parts<'source>(
    parts: impl IntoIterator<Item = JavaSyntaxListPart<'source, jolt_java_syntax::Annotation<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    let mut has_parts = false;
    let result = doc.concat_list(|docs| {
        for part in parts {
            let part = resolve_list_part(part, docs);
            let visible = part.is_visible(|item| item.first_token().is_some(), |_| true);
            let part = match part {
                JavaFormatListPart::Item(annotation) => format_annotation(&annotation, docs),
                JavaFormatListPart::Separator(separator) => {
                    crate::helpers::comments::format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Recovery(malformed) => malformed.doc(),
            };
            if visible && has_parts {
                let space = docs.space();
                docs.push(space);
            }
            docs.push(part);
            has_parts |= visible;
        }
    });
    (result, has_parts)
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
            format_member_dot(&dot, doc)
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
            format_member_dot(&dot, doc)
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
    let target = format_required_field(expression.target(), doc, |target, doc| match target {
        ClassLiteralTargetSyntax::PrimitiveType(ty) => format_type(&ty.into(), doc),
        ClassLiteralTargetSyntax::VoidType(ty) => format_void_type(&ty, doc),
        ClassLiteralTargetSyntax::NameExpression(target) => {
            format_name_expression(&target, LeadingComments::Preserve, doc)
        }
        ClassLiteralTargetSyntax::FieldAccessExpression(target) => {
            super::format_field_access_expression(&target, doc)
        }
        ClassLiteralTargetSyntax::BogusClassLiteralTarget(bogus) => format_malformed(&bogus, doc),
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
