use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    AnnotatedExpression, Annotation, Expression, KotlinRoleElement, ModifierList,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_malformed, format_or_verbatim,
    resolve_list_part, resolve_optional_field, resolve_required_field,
};
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
        Expression::LoopExpression(expression) => format_loop_expression(doc, expression, leading),
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

fn format_loop_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &jolt_kotlin_syntax::LoopExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let keyword = match resolve_required_field(expression.loop_token(), doc) {
            KotlinFormatField::Present(keyword) => {
                format_token(doc, &keyword, leading, TrailingTrivia::Preserve)
            }
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let condition = match resolve_optional_field(expression.condition(), doc) {
            KotlinFormatField::Present(Some(condition)) => {
                let space = doc.space();
                let condition = format_expression(doc, &condition);
                doc.concat([space, condition])
            }
            KotlinFormatField::Present(None) => Doc::nil(),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let body = match resolve_optional_field(expression.body(), doc) {
            KotlinFormatField::Present(Some(body)) => {
                let space = doc.space();
                let body = format_expression_body_role(doc, body);
                doc.concat([space, body])
            }
            KotlinFormatField::Present(None) => Doc::nil(),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        doc.concat([keyword, condition, body])
    })
}

fn format_annotated_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnnotatedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(expression, doc, |doc| {
        let prefix = match resolve_required_field(expression.prefix(), doc) {
            KotlinFormatField::Present(prefix) => doc.concat_list(|docs| {
                let mut first = true;
                for part in prefix.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(element) => {
                            let formatted = format_annotation_prefix_element(
                                docs,
                                element,
                                if first {
                                    leading
                                } else {
                                    LeadingTrivia::Preserve
                                },
                            );
                            docs.push(formatted);
                            let hard_line = docs.hard_line();
                            docs.push(hard_line);
                            first = false;
                        }
                        KotlinFormatListPart::Separator(token) => {
                            docs.block_on_invariant(format!(
                                "unexpected annotated-expression separator: {:?}",
                                token.kind()
                            ));
                        }
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        let inner = match resolve_required_field(expression.expression(), doc) {
            KotlinFormatField::Present(inner) => format_expression_without_leading(doc, &inner),
            KotlinFormatField::Malformed(recovery) => recovery,
        };
        doc.concat([prefix, inner])
    })
}

fn format_expression_body_role<'source>(
    doc: &mut DocBuilder<'source>,
    body: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(block) = body.cast_node::<jolt_kotlin_syntax::Block<'source>>() {
        crate::rules::statements::format_block(doc, &block)
    } else if let Some(expression) = body.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else {
        doc.block_on_invariant("Kotlin loop body contained an unsupported generated element");
        Doc::nil()
    }
}

fn format_annotation_prefix_element<'source>(
    doc: &mut DocBuilder<'source>,
    element: KotlinRoleElement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(annotation) = element.cast_node::<Annotation<'source>>() {
        return format_annotation_with_leading(doc, &annotation, leading);
    }
    if let Some(modifiers) = element.cast_node::<ModifierList<'source>>() {
        return format_annotated_modifier_list(doc, &modifiers, leading);
    }
    doc.block_on_invariant("Kotlin annotated expression prefix contained an unsupported element");
    Doc::nil()
}

fn format_annotated_modifier_list<'source>(
    doc: &mut DocBuilder<'source>,
    modifiers: &ModifierList<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(modifiers, doc, |doc| {
        match resolve_required_field(modifiers.modifiers(), doc) {
            KotlinFormatField::Present(items) => doc.concat_list(|docs| {
                let mut first = true;
                for part in items.parts() {
                    match resolve_list_part(part, docs) {
                        KotlinFormatListPart::Item(element) => {
                            let item_leading = if first {
                                leading
                            } else {
                                LeadingTrivia::Preserve
                            };
                            if let Some(annotation) = element.cast_node::<Annotation<'source>>() {
                                let annotation =
                                    format_annotation_with_leading(docs, &annotation, item_leading);
                                docs.push(annotation);
                            } else if let Some(token) = element.token() {
                                let token = format_token(
                                    docs,
                                    &token,
                                    item_leading,
                                    TrailingTrivia::Preserve,
                                );
                                docs.push(token);
                            } else {
                                docs.block_on_invariant(
                                    "Kotlin modifier list contained an unsupported element",
                                );
                            }
                            let space = docs.space();
                            docs.push(space);
                            first = false;
                        }
                        KotlinFormatListPart::Separator(token) => docs.block_on_invariant(format!(
                            "unexpected modifier separator: {:?}",
                            token.kind()
                        )),
                        KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                    }
                }
            }),
            KotlinFormatField::Malformed(recovery) => recovery,
        }
    })
}
