use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{AnnotatedExpression, Expression};

use crate::helpers::comments::{LeadingTrivia, format_leading_comments};
use crate::helpers::recovery::{KotlinFormatField, format_malformed, resolve_required_field};
use crate::rules::declarations::format_modifier_list_with_leading;

mod calls;
mod control_flow;
mod functions;
mod lambdas;
mod leaves;
mod operators;
mod references;

use calls::{format_collection_literal_expression, format_suffix_expression};
pub(crate) use calls::{format_value_argument, format_value_argument_list};
pub(crate) use control_flow::format_throw_expression_with_suffix;
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
    format_unary_expression,
};

pub(crate) fn format_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading_and_context(
        doc,
        expression,
        LeadingTrivia::Preserve,
        ExpressionContext::Default,
    )
}

/// Formats an expression whose first token must remain beside an inline
/// introducer such as `return` or `throw`.
pub(crate) fn format_inline_value_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading_and_context(
        doc,
        expression,
        LeadingTrivia::Preserve,
        ExpressionContext::InlineValue,
    )
}

pub(crate) fn format_expression_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
) -> Doc<'source> {
    format_expression_with_leading_and_context(
        doc,
        expression,
        LeadingTrivia::SuppressAlreadyHandled,
        ExpressionContext::Default,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpressionContext {
    Default,
    InlineValue,
}

fn format_expression_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_expression_with_leading_and_context(doc, expression, leading, ExpressionContext::Default)
}

pub(super) fn format_expression_with_leading_and_context<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
    context: ExpressionContext,
) -> Doc<'source> {
    match expression {
        Expression::LiteralExpression(expression) => {
            format_literal_expression(doc, expression, leading)
        }
        Expression::StringTemplateExpression(expression) => {
            format_string_template_expression(doc, expression, leading)
        }
        Expression::NameExpression(expression) => {
            if let Some(labeled) = format_labeled_expression(doc, expression, leading, context) {
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
            format_parenthesized_expression(doc, expression, leading, context)
        }
        Expression::AnnotatedExpression(expression) => {
            format_annotated_expression(doc, expression, leading, context)
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
        expression @ (Expression::PostfixExpression(_)
        | Expression::NavigationExpression(_)
        | Expression::CallExpression(_)
        | Expression::IndexExpression(_)
        | Expression::CallableReferenceExpression(_)) => {
            format_suffix_expression(doc, *expression, leading)
        }
        Expression::IfExpression(expression) => format_if_expression(
            doc,
            expression,
            leading,
            context == ExpressionContext::InlineValue,
        ),
        Expression::WhenExpression(expression) => format_when_expression(doc, expression, leading),
        Expression::TryExpression(expression) => format_try_expression(doc, expression, leading),
        Expression::ForStatement(expression) => format_for_statement(
            doc,
            expression,
            leading,
            context == ExpressionContext::InlineValue,
        ),
        Expression::WhileStatement(expression) => format_while_statement(
            doc,
            expression,
            leading,
            context == ExpressionContext::InlineValue,
        ),
        Expression::DoWhileStatement(expression) => format_do_while_statement(
            doc,
            expression,
            leading,
            context == ExpressionContext::InlineValue,
        ),
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
        Expression::BogusExpression(expression) => format_malformed(expression, doc),
    }
}

fn format_annotated_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnnotatedExpression<'source>,
    leading: LeadingTrivia,
    context: ExpressionContext,
) -> Doc<'source> {
    let prefix = match resolve_required_field(expression.prefix(), doc) {
        KotlinFormatField::Present(prefix) => {
            format_modifier_list_with_leading(doc, &prefix, true, leading)
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let inner = match resolve_required_field(expression.expression(), doc) {
        KotlinFormatField::Present(inner) => {
            // The annotation prefix ends its line, so the inner expression's
            // leading comments keep lines of their own: hoisted here, with
            // the inner's own leading trivia suppressed to claim them once.
            let comments = inner
                .first_token()
                .map_or_else(Doc::nil, |token| format_leading_comments(doc, &token));
            let inner = format_expression_with_leading_and_context(
                doc,
                &inner,
                LeadingTrivia::SuppressAlreadyHandled,
                context,
            );
            doc.concat([comments, inner])
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    doc.concat([prefix, inner])
}
