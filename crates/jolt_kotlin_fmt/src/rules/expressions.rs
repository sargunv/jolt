use jolt_fmt_ir::{Doc, concat, hard_line};
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

pub(crate) fn format_expression<'source>(expression: &Expression<'source>) -> Doc<'source> {
    format_expression_with_leading(expression, LeadingTrivia::Preserve)
}

pub(crate) fn format_expression_without_leading<'source>(
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading(expression, LeadingTrivia::SuppressAlreadyHandled)
}

fn format_expression_with_leading<'source>(
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match expression {
        Expression::LiteralExpression(expression) => format_literal_expression(expression, leading),
        Expression::StringTemplateExpression(expression) => {
            format_string_template_expression(expression, leading)
        }
        Expression::NameExpression(expression) => {
            if let Some(labeled) = format_labeled_expression(expression, leading) {
                labeled
            } else {
                format_name_expression(expression, leading)
            }
        }
        Expression::ThisExpression(expression) => format_this_expression(expression, leading),
        Expression::SuperExpression(expression) => format_super_expression(expression, leading),
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(expression, leading)
        }
        Expression::AnnotatedExpression(expression) => {
            format_annotated_expression(expression, leading)
        }
        Expression::AssignmentExpression(expression) => {
            format_assignment_expression(expression, leading)
        }
        Expression::BinaryExpression(expression) => format_binary_expression(expression, leading),
        Expression::UnaryExpression(expression) => format_unary_expression(expression, leading),
        Expression::PostfixExpression(expression) => format_postfix_expression(expression, leading),
        Expression::NavigationExpression(expression) => {
            format_navigation_expression(expression, leading)
        }
        Expression::CallExpression(expression) => format_call_expression(expression, leading),
        Expression::IndexExpression(expression) => format_index_expression(expression, leading),
        Expression::CallableReferenceExpression(expression) => {
            format_callable_reference_expression(expression, leading)
        }
        Expression::IfExpression(expression) => format_if_expression(expression, leading),
        Expression::WhenExpression(expression) => format_when_expression(expression, leading),
        Expression::TryExpression(expression) => format_try_expression(expression, leading),
        Expression::ForStatement(expression) => format_for_statement(expression, leading),
        Expression::WhileStatement(expression) => format_while_statement(expression, leading),
        Expression::DoWhileStatement(expression) => format_do_while_statement(expression, leading),
        Expression::LoopExpression(expression) => {
            let mut docs = Vec::new();
            if let Some(keyword) = expression.loop_token() {
                docs.push(format_token(&keyword, leading, TrailingTrivia::Preserve));
            }
            if let Some(condition) = expression.condition() {
                docs.push(format_parenthesized_expression(
                    &condition,
                    LeadingTrivia::Preserve,
                ));
            }
            if let Some(block) = expression.block_body() {
                docs.push(crate::rules::statements::format_block(&block));
            } else if let Some(body) = expression.expression_body() {
                docs.push(format_expression_with_leading(
                    &body,
                    LeadingTrivia::Preserve,
                ));
            }
            concat(docs)
        }
        Expression::JumpExpression(expression) => format_jump_expression(expression, leading),
        Expression::ThrowExpression(expression) => format_throw_expression(expression, leading),
        Expression::LambdaExpression(expression) => format_lambda_expression(expression, leading),
        Expression::AnonymousFunctionExpression(expression) => {
            format_anonymous_function_expression(expression, leading)
        }
        Expression::ObjectExpression(expression) => {
            crate::rules::declarations::format_object_expression(expression, leading)
        }
        Expression::CollectionLiteralExpression(expression) => {
            format_collection_literal_expression(expression, leading)
        }
    }
}

fn format_annotated_expression<'source>(
    expression: &AnnotatedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for (index, annotation) in expression.annotations().enumerate() {
        docs.push(format_annotation_with_leading(
            &annotation,
            if index == 0 {
                leading
            } else {
                LeadingTrivia::Preserve
            },
        ));
        if !annotation_trailing_comment_forces_line(&annotation) {
            docs.push(hard_line());
        }
    }
    docs.push(
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |inner| {
                format_expression_without_leading(&inner)
            }),
    );
    concat(docs)
}

fn annotation_trailing_comment_forces_line(annotation: &Annotation<'_>) -> bool {
    annotation.last_token().is_some_and(|token| {
        token
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}
