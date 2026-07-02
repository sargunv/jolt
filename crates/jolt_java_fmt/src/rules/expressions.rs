use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::{
    ArgumentList, ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer,
    AssignmentExpression, BinaryExpression, CastExpression, ClassLiteralExpression,
    ConditionalExpression, DimExpression, Expression, ExpressionParentRole, FieldAccessExpression,
    InstanceofExpression, JavaSyntaxToken, LambdaExpression, LambdaParameter, LiteralExpression,
    MemberChain, MemberChainSuffix, MethodInvocationExpression, MethodReferenceExpression,
    NameExpression, ObjectCreationExpression, ParenthesizedExpression, PostfixExpression,
    SuperExpression, SwitchExpression, ThisExpression, UnaryExpression, VariableInitializerValue,
};

use crate::context::JavaFormatter;
use crate::helpers::chains::member_chain;
use crate::helpers::comments::{
    comment_forces_line, format_leading_comments, format_token_text, format_token_with_comments,
    format_trailing_comments, format_trailing_comments_before_line_break, tokens_have_comments,
    trailing_comments_force_line,
};
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::helpers::operators::{assignment_expression, binary_chain, ternary_expression};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_anonymous_class_body;
use crate::rules::patterns::format_pattern;
use crate::rules::statements::{format_block, format_switch_block};
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_argument_list, format_void_type,
};

mod arrays_objects;
mod calls;
mod casts_patterns;
mod lambdas;
mod leaves;
mod member_chains;
mod method_references;
mod operators;
mod parenthesized;
mod switches;

pub(crate) use arrays_objects::format_variable_initializer_value;
use arrays_objects::{
    format_array_access_expression, format_array_creation_expression,
    format_object_creation_expression,
};
pub(crate) use calls::format_argument_list;
use calls::{
    format_field_access_expression, format_method_invocation_expression_with_leading_comments,
};
use casts_patterns::{format_cast_expression, format_instanceof_expression};
use lambdas::format_lambda_expression;
use leaves::{
    format_class_literal_expression, format_literal_expression, format_name_expression,
    format_super_expression, format_this_expression,
};
use member_chains::{format_member_chain, format_member_dot, is_member_chain_child};
use method_references::format_method_reference_expression;
use operators::{
    format_assignment_expression, format_binary_expression, format_conditional_expression,
    format_postfix_expression, format_unary_expression,
};
use parenthesized::format_parenthesized_expression;
use switches::format_switch_expression;

pub(crate) fn format_expression(expression: &Expression, formatter: &JavaFormatter<'_>) -> Doc {
    format_expression_with_leading_comments(expression, LeadingComments::Preserve, formatter)
}

fn format_expression_with_leading_comments(
    expression: &Expression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match expression {
        Expression::ParenthesizedExpression(expression) => {
            format_parenthesized_expression(expression, formatter)
        }
        Expression::AssignmentExpression(expression) => {
            format_assignment_expression(expression, formatter)
        }
        Expression::ConditionalExpression(expression) => {
            format_conditional_expression(expression, formatter)
        }
        Expression::BinaryExpression(expression) => format_binary_expression(expression, formatter),
        Expression::UnaryExpression(expression) => format_unary_expression(expression, formatter),
        Expression::PostfixExpression(expression) => {
            format_postfix_expression(expression, formatter)
        }
        Expression::LambdaExpression(expression) => format_lambda_expression(expression, formatter),
        Expression::LiteralExpression(expression) => {
            format_literal_expression(expression, leading_comments)
        }
        Expression::NameExpression(expression) => {
            format_name_expression(expression, leading_comments, formatter)
        }
        Expression::ThisExpression(expression) => {
            format_this_expression(expression, leading_comments, formatter)
        }
        Expression::SuperExpression(expression) => {
            format_super_expression(expression, leading_comments, formatter)
        }
        Expression::ClassLiteralExpression(expression) => {
            format_class_literal_expression(expression, formatter)
        }
        Expression::MethodReferenceExpression(expression) => {
            format_method_reference_expression(expression, formatter)
        }
        Expression::SwitchExpression(expression) => format_switch_expression(expression, formatter),
        Expression::ArrayCreationExpression(expression) => {
            format_array_creation_expression(expression, formatter)
        }
        Expression::InstanceofExpression(expression) => {
            format_instanceof_expression(expression, formatter)
        }
        Expression::CastExpression(expression) => format_cast_expression(expression, formatter),
        Expression::FieldAccessExpression(expression) => {
            format_field_access_expression(expression, formatter)
        }
        Expression::ArrayAccessExpression(expression) => {
            format_array_access_expression(expression, formatter)
        }
        Expression::MethodInvocationExpression(expression) => {
            format_method_invocation_expression_with_leading_comments(
                expression,
                leading_comments,
                formatter,
            )
        }
        Expression::ObjectCreationExpression(expression) => {
            format_object_creation_expression(expression, formatter)
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum LeadingComments {
    Preserve,
    SuppressFirstToken,
}
