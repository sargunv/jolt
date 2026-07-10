use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{AnnotatedExpression, Annotation, Expression};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, comment_forces_line, format_token};
use crate::rules::annotations::format_annotation_with_leading;

mod calls;
mod control_flow;
mod functions;
mod lambdas;
mod leaves;
mod operators;
mod references;

use calls::{
    format_call_expression, format_collection_literal_expression, format_index_expression,
    format_navigation_expression,
};
pub(crate) use calls::{format_value_argument, format_value_argument_list};
use control_flow::{
    format_do_while_statement, format_for_statement, format_if_expression, format_jump_expression,
    format_labeled_expression, format_throw_expression, format_try_expression,
    format_when_expression, format_while_statement,
};
use functions::format_anonymous_function_expression;
use lambdas::format_lambda_expression;
use leaves::{
    format_literal_expression, format_name_expression, format_string_template_expression,
    format_super_expression, format_this_expression,
};
use operators::{
    format_assignment_expression, format_binary_expression, format_parenthesized_expression,
    format_postfix_expression, format_unary_expression,
};
use references::format_callable_reference_expression;

pub(crate) fn format_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading(doc, expression, LeadingTrivia::Preserve)
}

pub(crate) fn format_expression_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading(doc, expression, LeadingTrivia::SuppressAlreadyHandled)
}

fn format_expression_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match expression {
        Expression::LiteralExpression(expression) => {
            format_literal_expression(doc, expression, leading)
        }
        Expression::StringTemplateExpression(expression) => {
            format_string_template_expression(doc, expression, leading)
        }
        Expression::NameExpression(expression) => {
            if let Some(labeled) = format_labeled_expression(doc, expression, leading) {
                labeled
            } else {
                format_name_expression(doc, expression, leading)
            }
        }
        Expression::ThisExpression(expression) => format_this_expression(doc, expression, leading),
        Expression::SuperExpression(expression) => {
            format_super_expression(doc, expression, leading)
        }
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(doc, expression, leading)
        }
        Expression::AnnotatedExpression(expression) => {
            format_annotated_expression(doc, expression, leading)
        }
        Expression::AssignmentExpression(expression) => {
            format_assignment_expression(doc, expression, leading)
        }
        Expression::BinaryExpression(expression) => {
            format_binary_expression(doc, expression, leading)
        }
        Expression::UnaryExpression(expression) => {
            format_unary_expression(doc, expression, leading)
        }
        Expression::PostfixExpression(expression) => {
            format_postfix_expression(doc, expression, leading)
        }
        Expression::NavigationExpression(expression) => {
            format_navigation_expression(doc, expression, leading)
        }
        Expression::CallExpression(expression) => format_call_expression(doc, expression, leading),
        Expression::IndexExpression(expression) => {
            format_index_expression(doc, expression, leading)
        }
        Expression::CallableReferenceExpression(expression) => {
            format_callable_reference_expression(doc, expression, leading)
        }
        Expression::IfExpression(expression) => format_if_expression(doc, expression, leading),
        Expression::WhenExpression(expression) => format_when_expression(doc, expression, leading),
        Expression::TryExpression(expression) => format_try_expression(doc, expression, leading),
        Expression::ForStatement(expression) => format_for_statement(doc, expression, leading),
        Expression::WhileStatement(expression) => format_while_statement(doc, expression, leading),
        Expression::DoWhileStatement(expression) => {
            format_do_while_statement(doc, expression, leading)
        }
        Expression::LoopExpression(expression) => doc.concat_list(|docs| {
            if let Some(keyword) = expression.loop_token() {
                let keyword = format_token(docs, &keyword, leading, TrailingTrivia::Preserve);
                docs.push(keyword);
            }
            if let Some(condition) = expression.condition() {
                let condition =
                    format_parenthesized_expression(docs, &condition, LeadingTrivia::Preserve);
                docs.push(condition);
            }
            if let Some(block) = expression.block_body() {
                let block = crate::rules::statements::format_block(docs, &block);
                docs.push(block);
            } else if let Some(body) = expression.expression_body() {
                let body = format_expression_with_leading(docs, &body, LeadingTrivia::Preserve);
                docs.push(body);
            }
        }),
        Expression::JumpExpression(expression) => format_jump_expression(doc, expression, leading),
        Expression::ThrowExpression(expression) => {
            format_throw_expression(doc, expression, leading)
        }
        Expression::LambdaExpression(expression) => {
            format_lambda_expression(doc, expression, leading)
        }
        Expression::AnonymousFunctionExpression(expression) => {
            format_anonymous_function_expression(doc, expression, leading)
        }
        Expression::ObjectExpression(expression) => {
            crate::rules::declarations::format_object_expression(doc, expression, leading)
        }
        Expression::CollectionLiteralExpression(expression) => {
            format_collection_literal_expression(doc, expression, leading)
        }
    }
}

fn format_annotated_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnnotatedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let annotations = expression.annotations();
    doc.concat_list(|docs| {
        for (index, annotation) in annotations.enumerate() {
            let annotation_doc = format_annotation_with_leading(
                docs,
                &annotation,
                if index == 0 {
                    leading
                } else {
                    LeadingTrivia::Preserve
                },
            );
            docs.push(annotation_doc);
            if !annotation_trailing_comment_forces_line(&annotation) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
        let expression = if let Some(inner) = expression.expression() {
            format_expression_without_leading(docs, &inner)
        } else {
            docs.nil()
        };
        docs.push(expression);
    })
}

fn annotation_trailing_comment_forces_line(annotation: &Annotation<'_>) -> bool {
    annotation.last_token().is_some_and(|token| {
        token
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}
