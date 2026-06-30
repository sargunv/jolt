use super::{
    ArrayDimensions, ArrayInitializer, Doc, Expression, FormatResult, JavaFormatContext,
    JavaSyntaxKind, JavaSyntaxToken, MethodReferenceExpression, Pattern, Type, TypeLayoutPart,
    VariableInitializerValue, braced_type_body, concat, format_annotation, format_annotation_list,
    format_block, format_class_body, format_local_variable_declaration_header,
    format_multiline_token, format_selector_type_argument_list_variants, format_switch_expression,
    format_token, format_type, format_type_argument_list, format_type_layout_parts, hard_line,
    java_lists, join, take_inline_leading_block_comment_docs,
    take_inline_leading_block_comment_docs_in_range, take_inline_trailing_block_comment_docs,
    take_leading_comment_docs_in_range, take_same_line_trailing_block_comment_docs_in_range,
    take_trailing_line_comment_docs_in_range_as_own_line, text, wrap,
};
use crate::analyzers::binary::{self as binary_analysis, BinarySide};
use crate::analyzers::chains::{BaseMetadata, Chain, ChainMember, ChainRole};
use crate::analyzers::expressions::ExpressionLayout;
use crate::context::JavaCommentBucket;
use crate::helpers::chains as java_chains;
use crate::helpers::expressions as java_expressions;
use crate::helpers::lambdas as java_lambdas;
use crate::helpers::literals as java_literals;
use crate::helpers::switches as java_switches;
use crate::policy::JavaFormatPolicy;
use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{group, indent_by, soft_line};

pub(super) fn format_expression(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_expression_with_chain_role(expression, context, ChainRole::Default)
}

fn format_expression_with_chain_role(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
    chain_role: ChainRole,
) -> FormatResult<Doc> {
    match expression {
        Expression::LiteralExpression(literal) => format_literal_expression(literal, context),
        Expression::NameExpression(name) => format_name_expression(name, context),
        Expression::ThisExpression(this) => format_this_expression(this, context),
        Expression::SuperExpression(super_expression) => {
            format_super_expression(super_expression, context)
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            format_parenthesized_expression_with_chain_role(parenthesized, context, chain_role)
        }
        Expression::ClassLiteralExpression(class_literal) => {
            format_class_literal_expression(class_literal, context)
        }
        Expression::FieldAccessExpression(_)
        | Expression::MethodInvocationExpression(_)
        | Expression::ArrayAccessExpression(_) => {
            format_selector_chain(expression, context, chain_role)
        }
        Expression::UnaryExpression(unary) => format_unary_expression(unary, context),
        Expression::PostfixExpression(postfix) => format_postfix_expression(postfix, context),
        Expression::BinaryExpression(binary) => format_binary_expression(binary, context),
        Expression::AssignmentExpression(assignment) => {
            format_assignment_expression(assignment, context)
        }
        Expression::CastExpression(cast) => format_cast_expression(cast, context),
        Expression::ObjectCreationExpression(creation) => {
            format_object_creation_expression(creation, context)
        }
        Expression::ArrayCreationExpression(creation) => {
            format_array_creation_expression(creation, context)
        }
        Expression::LambdaExpression(lambda) => format_lambda_expression(lambda, context),
        Expression::MethodReferenceExpression(reference) => {
            format_method_reference_expression(reference, context)
        }
        Expression::ConditionalExpression(conditional) => {
            format_conditional_expression(conditional, context)
        }
        Expression::InstanceofExpression(instanceof) => {
            format_instanceof_expression(instanceof, context)
        }
        Expression::SwitchExpression(switch) => format_switch_expression(switch, context),
    }
}

pub(super) fn format_conditional_expression(
    conditional: &jolt_java_syntax::ConditionalExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let condition = conditional
        .condition()
        .map(|expression| format_expression(&expression, context))
        .transpose()?
        .unwrap_or_else(|| text(""));
    let true_expression = conditional
        .true_expression()
        .map(|expression| format_expression(&expression, context))
        .transpose()?
        .unwrap_or_else(|| text(""));
    let false_expression = conditional
        .false_expression()
        .map(|expression| format_expression(&expression, context))
        .transpose()?
        .unwrap_or_else(|| text(""));

    Ok(java_expressions::conditional_expression(
        condition,
        true_expression,
        false_expression,
        context.policy(),
    ))
}

pub(super) fn format_instanceof_expression(
    instanceof: &jolt_java_syntax::InstanceofExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = instanceof
        .expression()
        .map(|expression| format_expression(&expression, context))
        .transpose()?
        .unwrap_or_else(|| text(""));
    let right = if let Some(ty) = instanceof.ty() {
        format_type(&ty, context)?
    } else if let Some(pattern) = instanceof.pattern() {
        format_pattern(&pattern, context)?
    } else {
        text("")
    };

    Ok(concat([expression, text(" instanceof "), right]))
}

pub(super) fn format_pattern(
    pattern: &Pattern,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_pattern_with_layout(pattern, context, PatternLayout::Default)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PatternLayout {
    Default,
    SwitchLabel,
}

pub(super) fn format_pattern_with_layout(
    pattern: &Pattern,
    context: &mut JavaFormatContext<'_>,
    layout: PatternLayout,
) -> FormatResult<Doc> {
    match pattern {
        Pattern::TypePattern(pattern) => {
            let declaration = pattern
                .local_variable_declaration()
                .expect("parser-clean type pattern should have a local variable declaration");
            format_local_variable_declaration_header(&declaration, context)
        }
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern, context, layout),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern, context, layout),
        Pattern::MatchAllPattern(pattern) => {
            let token = pattern
                .token()
                .expect("parser-clean match-all pattern should have `_`");
            Ok(format_token(&token))
        }
    }
}

fn format_record_pattern(
    pattern: &jolt_java_syntax::RecordPattern,
    context: &mut JavaFormatContext<'_>,
    layout: PatternLayout,
) -> FormatResult<Doc> {
    let ty = pattern
        .ty()
        .expect("parser-clean record pattern should have a type");
    let components = pattern
        .components()
        .map(|component| format_component_pattern(&component, context, layout))
        .collect::<FormatResult<Vec<_>>>()?;
    let ty = format_type(&ty, context)?;
    if layout == PatternLayout::SwitchLabel {
        return Ok(java_switches::switch_record_pattern_components(
            ty,
            components,
            context.policy(),
        ));
    }
    Ok(concat([
        ty,
        java_lists::argument_list_docs(components, context.policy()),
    ]))
}

fn format_component_pattern(
    pattern: &jolt_java_syntax::ComponentPattern,
    context: &mut JavaFormatContext<'_>,
    layout: PatternLayout,
) -> FormatResult<Doc> {
    let pattern = pattern
        .pattern()
        .expect("parser-clean component pattern should wrap a pattern");
    format_pattern_with_layout(&pattern, context, layout)
}

pub(super) fn format_selector_chain(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
    role: ChainRole,
) -> FormatResult<Doc> {
    let chain = collect_selector_chain(expression, context)?;
    Ok(java_chains::selector_chain(chain, context.policy(), role))
}

pub(super) fn collect_selector_chain(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    match expression {
        Expression::NameExpression(name) => {
            let identifier = name
                .identifier()
                .expect("parser-clean name expression should have an identifier");
            Ok(Chain::simple_base(
                format_name_expression(name, context)?,
                node_width(name.code_text_range()),
                Some(identifier.text().to_string()),
            )
            .with_tail_range(name.code_text_range()))
        }
        Expression::ThisExpression(this) => {
            let metadata = this_super_base_metadata(
                "this",
                node_width(this.code_text_range()),
                this.receiver().as_ref(),
            );
            Ok(Chain::with_base_metadata(
                format_this_expression(this, context)?,
                Vec::new(),
                metadata,
            )
            .with_tail_range(this.code_text_range()))
        }
        Expression::SuperExpression(super_expression) => {
            let metadata = this_super_base_metadata(
                "super",
                node_width(super_expression.code_text_range()),
                super_expression.receiver().as_ref(),
            );
            Ok(Chain::with_base_metadata(
                format_super_expression(super_expression, context)?,
                Vec::new(),
                metadata,
            )
            .with_tail_range(super_expression.code_text_range()))
        }
        Expression::ObjectCreationExpression(creation) => Ok(Chain::object_creation_base(
            format_object_creation_expression(creation, context)?,
            node_width(creation.code_text_range()),
        )
        .with_tail_range(creation.code_text_range())),
        Expression::ArrayAccessExpression(array_access) => {
            collect_array_access_chain(array_access, context)
        }
        Expression::ClassLiteralExpression(class_literal) => Ok(Chain::complex_base(
            format_class_literal_expression(class_literal, context)?,
            node_width(class_literal.code_text_range()),
        )
        .with_tail_range(class_literal.code_text_range())),
        Expression::ParenthesizedExpression(parenthesized) => Ok(Chain::primary_expression_base(
            format_parenthesized_chain_base(parenthesized, context)?,
            node_width(parenthesized.code_text_range()),
        )
        .with_tail_range(parenthesized.code_text_range())),
        Expression::ConditionalExpression(conditional) => Ok(Chain::primary_expression_base(
            format_conditional_expression(conditional, context)?,
            node_width(conditional.code_text_range()),
        )
        .with_tail_range(conditional.code_text_range())),
        Expression::CastExpression(cast) => collect_cast_chain(cast, context),
        Expression::FieldAccessExpression(field) => collect_field_access_chain(field, context),
        Expression::MethodInvocationExpression(invocation) => {
            collect_method_invocation_chain(invocation, context)
        }
        _ => Ok(Chain::complex_base(
            format_expression(expression, context)?,
            node_width(expression.code_text_range()),
        )
        .with_tail_range(expression.code_text_range())),
    }
}

fn this_super_base_metadata(
    keyword: &str,
    source_width: usize,
    receiver: Option<&Expression>,
) -> BaseMetadata {
    let simple_name = Some(keyword.to_string());
    if receiver.is_some_and(is_qualified_name_receiver) {
        BaseMetadata::qualified_this_super_prefix(source_width, simple_name)
    } else {
        BaseMetadata::simple(source_width, simple_name)
    }
}

fn is_qualified_name_receiver(expression: &Expression) -> bool {
    match expression {
        Expression::FieldAccessExpression(field) => {
            field.name().is_some()
                && field.receiver().as_ref().is_some_and(|receiver| {
                    matches!(receiver, Expression::NameExpression(_))
                        || is_qualified_name_receiver(receiver)
                })
        }
        _ => false,
    }
}

fn collect_selector_chain_as_receiver(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    if let Expression::MethodInvocationExpression(invocation) = expression
        && invocation.receiver().is_none()
    {
        return collect_simple_method_invocation_chain(
            invocation,
            context,
            context
                .policy()
                .selector_chain_receiver_argument_indent_levels(),
        );
    }

    collect_selector_chain(expression, context)
}

pub(super) fn collect_cast_chain(
    cast: &jolt_java_syntax::CastExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    if let Some(chain) = peel_cast_operand_chain(cast, context)? {
        return Ok(chain);
    }

    let ty = cast
        .ty()
        .expect("parser-clean cast expression should have a type");
    let expression = cast
        .expression()
        .expect("parser-clean cast expression should have an expression");

    Ok(Chain::cast_primary_expression_base(
        format_cast_primary_base(&ty, format_expression(&expression, context)?, context)?,
        node_width(cast.code_text_range()),
    )
    .with_tail_range(cast.code_text_range()))
}

fn peel_cast_operand_chain(
    cast: &jolt_java_syntax::CastExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Option<Chain>> {
    let ty = cast
        .ty()
        .expect("parser-clean cast expression should have a type");
    let expression = cast
        .expression()
        .expect("parser-clean cast expression should have an expression");

    match expression {
        Expression::FieldAccessExpression(_) | Expression::MethodInvocationExpression(_) => {
            let mut chain = collect_selector_chain(&expression, context)?;
            let receiver_base = chain.base;
            chain.base = format_cast_primary_base(&ty, receiver_base, context)?;
            Ok(Some(
                Chain::with_base_metadata(
                    chain.base,
                    chain.members,
                    BaseMetadata::cast_primary_expression(node_width(cast.code_text_range())),
                )
                .with_tail_range(cast.code_text_range()),
            ))
        }
        _ => Ok(None),
    }
}

fn format_cast_primary_base(
    ty: &Type,
    receiver: Doc,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    Ok(java_expressions::cast_primary_base(
        format_type(ty, context)?,
        receiver,
        context.policy(),
    ))
}

pub(super) fn collect_field_access_chain(
    field: &jolt_java_syntax::FieldAccessExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    let receiver = field
        .receiver()
        .expect("parser-clean field access should have a receiver");
    let name = field
        .name()
        .expect("parser-clean field access should have a name");
    let type_arguments_node = field.type_arguments();
    let selector_width = name.text().chars().count()
        + type_arguments_node
            .as_ref()
            .map(|arguments| text_range_width(arguments.text_range()))
            .unwrap_or_default();
    let type_arguments = type_arguments_node
        .clone()
        .map(|arguments| format_type_argument_list(&arguments, context))
        .transpose()?;

    let mut chain = collect_selector_chain_as_receiver(&receiver, context)?;
    attach_selector_boundary_comments(
        &mut chain,
        member_start_range(&type_arguments_node, name.token_text_range()),
        context,
    );
    chain.push(ChainMember::field(
        concat([
            format_token(&name),
            type_arguments.unwrap_or_else(|| text("")),
        ]),
        selector_width,
        Some(name.text().to_string()),
    ));
    chain.set_tail_range(field.code_text_range());
    Ok(chain)
}

pub(super) fn collect_method_invocation_chain(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    let arguments_node = invocation
        .arguments()
        .expect("parser-clean method invocation should have arguments");
    let argument_count = arguments_node.arguments().count();

    if let Some(receiver) = invocation.receiver() {
        let name = invocation
            .name()
            .expect("parser-clean qualified method invocation should have a name");
        let type_arguments_node = invocation.type_arguments();
        let has_type_arguments = type_arguments_node.is_some();
        let selector_width = name.text().chars().count()
            + text_range_width(arguments_node.text_range())
            + type_arguments_node
                .as_ref()
                .map(|arguments| text_range_width(arguments.text_range()))
                .unwrap_or_default();
        let type_arguments_width = type_arguments_node
            .as_ref()
            .map(|arguments| text_range_width(arguments.text_range()))
            .unwrap_or_default();
        let selector_head_width = if argument_count > 0 {
            name.text().chars().count() + type_arguments_width + 1
        } else {
            selector_width
        };
        let (type_arguments, type_arguments_after_chain_break) =
            if let Some(arguments) = type_arguments_node.clone() {
                let (default, after_chain_break) =
                    format_selector_type_argument_list_variants(&arguments, context)?;
                (Some(default), Some(after_chain_break))
            } else {
                (None, None)
            };
        let mut chain = collect_selector_chain_as_receiver(&receiver, context)?;
        attach_selector_boundary_comments(
            &mut chain,
            member_start_range(&type_arguments_node, name.token_text_range()),
            context,
        );
        let can_build_receiver_head_arguments = context
            .unhandled_comment_trivia_in_range(arguments_node.text_range())
            .is_none();
        let arguments = format_argument_list(&arguments_node, context)?;
        let arguments_as_receiver_head = if can_build_receiver_head_arguments {
            Some(format_argument_list_with_continuation_indent(
                &arguments_node,
                context,
                context
                    .policy()
                    .selector_chain_receiver_argument_indent_levels(),
            )?)
        } else {
            None
        };
        let selector_head = java_chains::explicit_type_argument_invocation_selector_head(
            type_arguments.clone(),
            text(name.text()),
            context.policy(),
        );
        let member = java_chains::explicit_type_argument_invocation_selector(
            type_arguments,
            text(name.text()),
            arguments.clone(),
            context.policy(),
        );
        let member_after_chain_break =
            java_chains::explicit_type_argument_invocation_selector_after_chain_break(
                type_arguments_after_chain_break.clone(),
                text(name.text()),
                arguments.clone(),
                context.policy(),
            );
        let member_as_receiver_head_after_chain_break =
            arguments_as_receiver_head.map(|arguments| {
                java_chains::explicit_type_argument_invocation_selector_after_chain_break(
                    type_arguments_after_chain_break,
                    text(name.text()),
                    arguments,
                    context.policy(),
                )
            });
        chain.push(ChainMember::call(
            member,
            selector_head,
            arguments,
            Some(member_after_chain_break),
            member_as_receiver_head_after_chain_break,
            selector_width,
            selector_head_width,
            argument_count,
            has_type_arguments,
            Some(name.text().to_string()),
        ));
        chain.set_tail_range(invocation.code_text_range());
        return Ok(chain);
    }

    collect_simple_method_invocation_chain(
        invocation,
        context,
        context.policy().continuation_indent_levels(),
    )
}

fn collect_simple_method_invocation_chain(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
    context: &mut JavaFormatContext<'_>,
    argument_indent_levels: u16,
) -> FormatResult<Chain> {
    let arguments_node = invocation
        .arguments()
        .expect("parser-clean method invocation should have arguments");
    let name = invocation
        .simple_name()
        .expect("parser-clean simple method invocation should have a name");
    let arguments = format_argument_list_with_continuation_indent(
        &arguments_node,
        context,
        argument_indent_levels,
    )?;
    Ok(Chain::call_base(
        concat([text(name.text()), arguments]),
        node_width(invocation.code_text_range()),
    )
    .with_tail_range(invocation.code_text_range()))
}

pub(super) fn collect_array_access_chain(
    array_access: &jolt_java_syntax::ArrayAccessExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Chain> {
    let receiver = array_access
        .receiver()
        .expect("parser-clean array access should have a receiver");
    let l_bracket = array_access
        .l_bracket()
        .expect("parser-clean array access should have an opening bracket");
    let r_bracket = array_access
        .r_bracket()
        .expect("parser-clean array access should have a closing bracket");

    let mut chain = collect_selector_chain_as_receiver(&receiver, context)?;
    attach_selector_boundary_comments(&mut chain, l_bracket.token_text_range(), context);
    chain.push(ChainMember::array_access(
        format_array_access_selector(array_access, context)?,
        text_range_width(TextRange::new(
            l_bracket.token_text_range().start(),
            r_bracket.token_text_range().end(),
        )),
    ));
    chain.set_tail_range(array_access.code_text_range());
    Ok(chain)
}

fn attach_selector_boundary_comments(
    chain: &mut Chain,
    member_start_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) {
    let Some(tail_range) = chain.tail_range() else {
        return;
    };
    let member_start = member_start_range.start();
    if member_start < tail_range.end() {
        return;
    }

    let comments = take_trailing_line_comment_docs_in_range_as_own_line(
        context,
        tail_range,
        TextRange::new(tail_range.end(), member_start),
    );
    chain.push_trailing_comments_to_tail(comments);
}

fn member_start_range(
    type_arguments: &Option<jolt_java_syntax::TypeArgumentList>,
    name_range: TextRange,
) -> TextRange {
    type_arguments
        .as_ref()
        .map_or(name_range, jolt_java_syntax::TypeArgumentList::text_range)
}

fn node_width(range: Option<jolt_diagnostics::TextRange>) -> usize {
    range.map(text_range_width).unwrap_or_default()
}

fn text_range_width(range: jolt_diagnostics::TextRange) -> usize {
    range.end().get().saturating_sub(range.start().get())
}

pub(super) fn format_method_reference_expression(
    reference: &MethodReferenceExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let qualifier_type_arguments = reference
        .qualifier_type_arguments()
        .map(|arguments| format_type_argument_list(&arguments, context))
        .transpose()?;
    let dimensions = reference
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions, context))
        .transpose()?;

    let qualifier = if let Some(expression) = reference.expression_qualifier() {
        format_expression(&expression, context)?
    } else {
        let ty = reference
            .type_qualifier()
            .expect("validated method reference should have a qualifier");
        if context
            .policy()
            .method_reference_type_qualifier_uses_selector_chain()
            && qualifier_type_arguments.is_none()
            && dimensions.is_none()
        {
            format_method_reference_type_qualifier(&ty, context)?
        } else {
            format_simple_expression_type(&ty, context, "method reference qualifier")?
        }
    };
    let member_type_arguments = reference
        .member_type_arguments()
        .map(|arguments| format_type_argument_list(&arguments, context))
        .transpose()?;

    let member = if reference.is_constructor_reference() {
        text("new")
    } else {
        let name = reference
            .name()
            .expect("validated method reference should have a member name");
        format_token(&name)
    };

    let reference_tail = concat([
        text("::"),
        member_type_arguments.unwrap_or_else(|| text("")),
        member,
    ]);

    if !context
        .policy()
        .method_reference_type_qualifier_uses_selector_chain()
    {
        return Ok(concat([
            qualifier,
            qualifier_type_arguments.unwrap_or_else(|| text("")),
            dimensions.unwrap_or_else(|| text("")),
            reference_tail,
        ]));
    }

    Ok(group(concat([
        qualifier,
        qualifier_type_arguments.unwrap_or_else(|| text("")),
        dimensions.unwrap_or_else(|| text("")),
        indent_by(
            context.policy().continuation_indent_levels(),
            concat([soft_line(), reference_tail]),
        ),
    ])))
}

fn format_method_reference_type_qualifier(
    ty: &Type,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let Some(chain) = simple_type_qualifier_chain(ty) else {
        return format_simple_expression_type(ty, context, "method reference qualifier");
    };

    Ok(java_chains::selector_chain(
        chain,
        context.policy(),
        ChainRole::Default,
    ))
}

fn simple_type_qualifier_chain(ty: &Type) -> Option<Chain> {
    let parts = ty.simple_layout_parts()?;
    let mut names = Vec::new();
    let mut expect_identifier = true;
    for part in parts {
        let TypeLayoutPart::Token(token) = part else {
            return None;
        };
        match (expect_identifier, token.kind()) {
            (true, JavaSyntaxKind::Identifier) => {
                names.push(token.text().to_string());
                expect_identifier = false;
            }
            (false, JavaSyntaxKind::Dot) => {
                expect_identifier = true;
            }
            _ => return None,
        }
    }

    if expect_identifier || names.len() < 2 {
        return None;
    }

    let mut names = names.into_iter();
    let first = names.next()?;
    let mut chain = Chain::simple_base(text(first.clone()), first.len(), Some(first));
    for name in names {
        chain.push(ChainMember::field(
            text(name.clone()),
            name.len(),
            Some(name),
        ));
    }
    Some(chain)
}

pub(super) fn format_literal_expression(
    literal: &jolt_java_syntax::LiteralExpression,
    context: &JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let token = literal
        .token()
        .expect("parser-clean literal expression should have a literal token");
    if token.kind() == JavaSyntaxKind::TextBlockLiteral
        && context.policy().normalizes_text_block_indentation()
    {
        Ok(java_literals::text_block_literal(token.text()).into_doc())
    } else if token.kind() == JavaSyntaxKind::StringLiteral {
        let line_context = context.line_context_for_range(token.token_text_range());
        if line_context.starts_line
            && let Some(doc) = java_literals::string_literal_reflow(
                token.text(),
                line_context.start_column,
                line_context.trailing_width,
                context.policy(),
            )
        {
            return Ok(doc);
        }
        Ok(format_token(&token))
    } else if token.text().contains(is_line_terminator) {
        Ok(format_multiline_token(&token))
    } else {
        Ok(format_token(&token))
    }
}

pub(super) fn format_name_expression(
    name: &jolt_java_syntax::NameExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut parts = name
        .annotations()
        .map(|annotation| format_annotation(&annotation, context, "type-use"))
        .collect::<FormatResult<Vec<_>>>()?;
    let identifier = name
        .identifier()
        .expect("parser-clean name expression should have an identifier");
    parts.push(format_token(&identifier));
    Ok(wrap::space_separated(parts))
}

pub(super) fn format_this_expression(
    this: &jolt_java_syntax::ThisExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let token = this
        .token()
        .expect("parser-clean this expression should have `this`");
    if let Some(receiver) = this.receiver() {
        Ok(concat([
            format_expression(&receiver, context)?,
            text("."),
            format_token(&token),
        ]))
    } else {
        Ok(format_token(&token))
    }
}

pub(super) fn format_super_expression(
    super_expression: &jolt_java_syntax::SuperExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let token = super_expression
        .token()
        .expect("parser-clean super expression should have `super`");
    if let Some(receiver) = super_expression.receiver() {
        Ok(concat([
            format_expression(&receiver, context)?,
            text("."),
            format_token(&token),
        ]))
    } else {
        Ok(format_token(&token))
    }
}

pub(super) fn format_parenthesized_expression(
    parenthesized: &jolt_java_syntax::ParenthesizedExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_parenthesized_expression_with_chain_role(parenthesized, context, ChainRole::Default)
}

fn format_parenthesized_chain_base(
    parenthesized: &jolt_java_syntax::ParenthesizedExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = parenthesized
        .expression()
        .expect("parser-clean parenthesized expression should have an expression");
    Ok(java_expressions::flat_parenthesized_expression(
        format_expression(&expression, context)?,
    ))
}

fn format_parenthesized_expression_with_chain_role(
    parenthesized: &jolt_java_syntax::ParenthesizedExpression,
    context: &mut JavaFormatContext<'_>,
    chain_role: ChainRole,
) -> FormatResult<Doc> {
    let expression = parenthesized
        .expression()
        .expect("parser-clean parenthesized expression should have an expression");
    Ok(java_expressions::parenthesized_expression(
        format_expression_with_chain_role(&expression, context, chain_role)?,
        context.policy(),
    ))
}

pub(super) fn format_class_literal_expression(
    class_literal: &jolt_java_syntax::ClassLiteralExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let class_token = class_literal
        .class_token()
        .expect("parser-clean class literal should have `class`");

    let qualifier = if let Some(expression) = class_literal.expression() {
        format_expression(&expression, context)?
    } else if let Some(token) = class_literal.primitive_or_void_token() {
        format_token(&token)
    } else {
        let ty = class_literal
            .ty()
            .expect("parser-clean class literal should have a qualifier");
        format_type_layout_parts(&ty.layout_parts(), context)?
    };
    let dimensions = class_literal
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions, context))
        .transpose()?;

    Ok(concat([
        qualifier,
        dimensions.unwrap_or_else(|| text("")),
        text("."),
        format_token(&class_token),
    ]))
}

pub(super) fn format_array_dimensions(
    dimensions: &ArrayDimensions,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_layout_parts(&dimensions.layout_parts(), context)
}

pub(super) fn format_unary_expression(
    unary: &jolt_java_syntax::UnaryExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let operator = unary
        .operator()
        .expect("parser-clean unary expression should have an operator");
    let operand = unary
        .operand()
        .expect("parser-clean unary expression should have an operand");
    let separator = if unary_operator_needs_separator(&operator, &operand) {
        text(" ")
    } else {
        text("")
    };
    let operand = format_unary_operand(&operand, context)?;
    Ok(concat([format_token(&operator), separator, operand]))
}

fn format_unary_operand(
    operand: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let doc = format_expression(operand, context)?;
    if matches!(
        operand,
        Expression::AssignmentExpression(_) | Expression::BinaryExpression(_)
    ) {
        Ok(concat([text("("), doc, text(")")]))
    } else {
        Ok(doc)
    }
}

pub(super) fn format_postfix_expression(
    postfix: &jolt_java_syntax::PostfixExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let operand = postfix
        .operand()
        .expect("parser-clean postfix expression should have an operand");
    let operator = postfix
        .operator()
        .expect("parser-clean postfix expression should have an operator");
    Ok(concat([
        format_unary_operand(&operand, context)?,
        format_token(&operator),
    ]))
}

pub(super) fn format_assignment_expression(
    assignment: &jolt_java_syntax::AssignmentExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let left = assignment
        .left()
        .expect("parser-clean assignment expression should have a left side");
    let operator = assignment
        .operator()
        .expect("parser-clean assignment expression should have an operator");
    let right = assignment
        .right()
        .expect("parser-clean assignment expression should have a right side");
    let operator_range = operator.token_text_range();
    let right_range = right
        .code_text_range()
        .expect("parser-clean assignment right side should have a code range");
    let operator_doc = format_assignment_operator(&operator, right_range, context);
    let leading = take_expression_gap_comment_docs(
        context,
        TextRange::new(operator_range.end(), right_range.start()),
        right_range,
    )?;
    Ok(java_expressions::assignment_expression_from_parts(
        format_expression(&left, context)?,
        operator_doc,
        format_expression(&right, context)?,
        ExpressionLayout::for_expression(&right, context.policy()),
        leading,
        context.policy(),
    ))
}

fn format_array_access_selector(
    array_access: &jolt_java_syntax::ArrayAccessExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let index = array_access
        .index()
        .expect("parser-clean array access should have an index");

    Ok(group(concat([
        text("["),
        indent_by(
            context.policy().array_access_index_indent_levels(),
            concat([soft_line(), format_expression(&index, context)?]),
        ),
        text("]"),
    ])))
}

pub(super) fn format_argument_list(
    arguments: &jolt_java_syntax::ArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_argument_list_with_continuation_indent(
        arguments,
        context,
        context.policy().continuation_indent_levels(),
    )
}

fn format_argument_list_with_continuation_indent(
    arguments: &jolt_java_syntax::ArgumentList,
    context: &mut JavaFormatContext<'_>,
    continuation_indent_levels: u16,
) -> FormatResult<Doc> {
    use crate::analyzers::array_initializers::TabularEntry;

    assert!(
        !arguments.has_trailing_comma(),
        "parser-clean argument list should not have a trailing comma"
    );
    let list_range = arguments
        .code_text_range()
        .expect("parser-clean argument list should have a code range");
    let argument_expressions = arguments.arguments().collect::<Vec<_>>();
    let is_format_method = argument_expressions.len() >= 2
        && crate::analyzers::format_strings::is_format_string_expression(&argument_expressions[0]);
    let arguments =
        argument_expressions
            .into_iter()
            .map(|argument| {
                let range = argument
                    .code_text_range()
                    .expect("parser-clean argument expression should have a code range");
                let shape = argument_list_item_shape(&argument, context.policy());
                let has_inline_comments = context
                    .has_comment_in_bucket(range, JavaCommentBucket::InlineLeadingBlock)
                    || context.has_comment_in_bucket(range, JavaCommentBucket::InlineTrailingBlock);
                let tabular_entry = TabularEntry {
                    range,
                    kind: crate::analyzers::array_initializers::expression_parallel_kind(&argument),
                    row_weight: 1,
                    is_nested_array: false,
                };
                let argument = argument.clone();
                Ok(java_lists::ListItem::new(range, move |context| {
                    format_argument(&argument, context)
                })
                .with_shape(shape)
                .with_inline_comments(has_inline_comments)
                .with_tabular_entry(tabular_entry))
            })
            .collect::<FormatResult<Vec<_>>>()?;
    java_lists::argument_list_with_continuation_indent(
        arguments,
        list_range,
        is_format_method,
        continuation_indent_levels,
        context,
    )
}

fn argument_list_item_shape(
    argument: &Expression,
    policy: JavaFormatPolicy,
) -> java_lists::ListItemShape {
    match argument {
        Expression::LiteralExpression(_)
        | Expression::NameExpression(_)
        | Expression::ThisExpression(_)
        | Expression::SuperExpression(_)
        | Expression::ClassLiteralExpression(_) => java_lists::ListItemShape::Simple,
        Expression::FieldAccessExpression(_) => java_lists::ListItemShape::SelectorChain,
        Expression::MethodInvocationExpression(invocation) => {
            method_invocation_argument_shape(invocation, policy)
        }
        Expression::ObjectCreationExpression(creation) => {
            if creation.body().is_some() {
                java_lists::ListItemShape::AnonymousObjectCreationUnit
            } else {
                java_lists::ListItemShape::NestedArgumentUnit
            }
        }
        _ => java_lists::ListItemShape::Complex,
    }
}

fn method_invocation_argument_shape(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
    policy: JavaFormatPolicy,
) -> java_lists::ListItemShape {
    let Some(receiver) = invocation.receiver() else {
        return java_lists::ListItemShape::Call;
    };

    if is_single_unit_invocation_receiver(&receiver) {
        if qualified_invocation_head_width(invocation, &receiver)
            >= policy.argument_list_single_nested_invocation_head_min_width()
        {
            java_lists::ListItemShape::WideHeadNestedArgumentUnit
        } else {
            java_lists::ListItemShape::Call
        }
    } else {
        java_lists::ListItemShape::SelectorChain
    }
}

fn qualified_invocation_head_width(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
    receiver: &Expression,
) -> usize {
    let receiver_width = node_width(receiver.code_text_range());
    let name_width = invocation
        .name()
        .map(|name| name.text().chars().count())
        .unwrap_or_default();
    let type_arguments_width = invocation
        .type_arguments()
        .map(|arguments| text_range_width(arguments.text_range()))
        .unwrap_or_default();
    receiver_width + 1 + type_arguments_width + name_width + 1
}

fn is_single_unit_invocation_receiver(receiver: &Expression) -> bool {
    matches!(
        receiver,
        Expression::NameExpression(_)
            | Expression::ThisExpression(_)
            | Expression::SuperExpression(_)
            | Expression::ClassLiteralExpression(_)
    )
}

pub(super) fn format_argument(
    argument: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = argument
        .code_text_range()
        .expect("parser-clean argument expression should have a code range");
    let comments = take_inline_leading_block_comment_docs(context, code_range);
    let role = nested_argument_chain_role(context);
    let expression = context.in_nested_argument_scope(|context| {
        format_expression_with_chain_role(argument, context, role)
    })?;
    let trailing_comments = take_inline_trailing_block_comment_docs(context, code_range);

    let mut parts = Vec::new();
    if !comments.is_empty() {
        parts.push(join(text(" "), comments));
    }
    parts.push(expression);
    if !trailing_comments.is_empty() {
        parts.push(join(text(" "), trailing_comments));
    }

    Ok(wrap::space_separated(parts))
}

fn nested_argument_chain_role(context: &JavaFormatContext<'_>) -> ChainRole {
    if context.nested_argument_depth()
        < context
            .policy()
            .nested_argument_selector_chain_fit_depth_limit()
    {
        ChainRole::NestedArgumentFit
    } else {
        ChainRole::NestedArgument
    }
}

pub(super) fn format_cast_expression(
    cast: &jolt_java_syntax::CastExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Some(chain) = peel_cast_operand_chain(cast, context)? {
        return Ok(java_chains::selector_chain(
            chain,
            context.policy(),
            ChainRole::Default,
        ));
    }

    let ty = cast
        .ty()
        .expect("parser-clean cast expression should have a type");
    let expression = cast
        .expression()
        .expect("parser-clean cast expression should have an expression");

    Ok(group_cast_expression(
        format_type(&ty, context)?,
        format_expression(&expression, context)?,
    ))
}

pub(super) fn group_cast_expression(ty: Doc, expression: Doc) -> Doc {
    concat([text("("), ty, text(") "), expression])
}

pub(super) fn format_object_creation_expression(
    creation: &jolt_java_syntax::ObjectCreationExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = creation
        .ty()
        .expect("parser-clean object creation should have a type");
    let qualifier = creation
        .qualifier()
        .map(|qualifier| format_expression(&qualifier, context))
        .transpose()?;
    let type_arguments = creation
        .type_arguments()
        .map(|arguments| format_type_argument_list(&arguments, context))
        .transpose()?;
    let arguments = creation
        .arguments()
        .expect("parser-clean object creation should have arguments");
    let body = creation
        .body()
        .map(|body| format_class_body(&body, context))
        .transpose()?;

    let mut parts = Vec::new();
    if let Some(qualifier) = qualifier {
        parts.push(qualifier);
        parts.push(text("."));
    }
    parts.push(text("new "));
    if let Some(type_arguments) = type_arguments {
        parts.push(type_arguments);
    }
    parts.push(format_type(&ty, context)?);
    parts.push(format_argument_list(&arguments, context)?);
    if let Some(body) = body {
        parts.push(text(" "));
        parts.push(braced_type_body(body));
    }

    Ok(concat(parts))
}

pub(super) fn format_array_creation_expression(
    creation: &jolt_java_syntax::ArrayCreationExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = creation
        .ty()
        .expect("parser-clean array creation should have a type");

    let dimensions = creation
        .dimensions()
        .map(|dimension| format_dim_expression(&dimension, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let trailing_dimensions = creation
        .trailing_dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions, context))
        .transpose()?;
    let initializer = creation
        .initializer()
        .map(|initializer| format_array_initializer(&initializer, context, false))
        .transpose()?;

    assert!(
        !dimensions.is_empty() || initializer.is_some(),
        "parser-clean array creation should have dimensions or an initializer"
    );

    let type_doc = format_type(&ty, context)?;
    let dimension_count = dimensions.len();
    let mut head = vec![text("new "), type_doc];
    head.extend(
        dimensions
            .into_iter()
            .map(|dimension| concat([soft_line(), dimension])),
    );
    if let Some(trailing_dimensions) = trailing_dimensions {
        head.push(trailing_dimensions);
    }

    let head_doc = if dimension_count > 1 {
        group(indent_by(
            context.policy().continuation_indent_levels(),
            concat(head),
        ))
    } else {
        concat(head)
    };

    let mut parts = vec![head_doc];
    if let Some(initializer) = initializer {
        parts.push(text(" "));
        parts.push(initializer);
    }

    Ok(concat(parts))
}

pub(super) fn format_dim_expression(
    dimension: &jolt_java_syntax::DimExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let annotations = format_annotation_list(dimension.annotations(), context, "type-use")?;
    let expression = dimension
        .expression()
        .expect("parser-clean dimension expression should have an expression");

    let mut parts = Vec::new();
    if !annotations.is_empty() {
        parts.push(text(" "));
        parts.push(join(text(" "), annotations));
        parts.push(text(" "));
    }
    parts.push(text("["));
    parts.push(format_expression(&expression, context)?);
    parts.push(text("]"));

    Ok(concat(parts))
}

pub(super) fn format_array_initializer(
    initializer: &ArrayInitializer,
    context: &mut JavaFormatContext<'_>,
    _in_annotation: bool,
) -> FormatResult<Doc> {
    let has_trailing_comma = initializer.has_trailing_comma();
    let values = initializer.values().collect::<Vec<_>>();
    let list_range = initializer
        .code_text_range()
        .expect("parser-clean array initializer should have a source range");
    let list_items = values.iter().map(|value| {
        let range = value
            .code_text_range()
            .expect("parser-clean array initializer value should have a source range");
        let value = value.clone();
        java_lists::ListItem::new(range, move |context| {
            format_variable_initializer_value(&value, context)
        })
    });
    let list = java_lists::format_braced_list_items(list_items, list_range, context)?;
    let policy = context.policy();

    let entries = crate::analyzers::array_initializers::tabular_entries(&values);
    let layout = if let Some(tabular) =
        crate::analyzers::array_initializers::tabular_layout_for_entries(&entries, context)
    {
        let rows_nested = tabular
            .rows
            .iter()
            .map(|row| {
                crate::analyzers::array_initializers::row_opens_without_extra_indent(
                    &entries,
                    row,
                    tabular.cols,
                )
            })
            .collect();
        crate::helpers::array_initializers::InitializerLayout::Tabular {
            cols: tabular.cols,
            rows: tabular.rows,
            rows_nested,
        }
    } else {
        let short_items =
            crate::analyzers::array_initializers::has_only_short_items(&values, policy);
        let tight_fit = entries.len() >= policy.array_initializer_tight_fit_min_items()
            && entries
                .iter()
                .all(|entry| entry.kind.is_scalar_initializer() && entry.row_weight == 1);
        crate::helpers::array_initializers::InitializerLayout::Fill {
            short_items,
            tight_fit,
        }
    };

    Ok(
        crate::helpers::array_initializers::braced_initializer_block(
            list,
            layout,
            has_trailing_comma,
            policy,
        ),
    )
}

pub(super) fn format_variable_initializer_value(
    value: &VariableInitializerValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match value {
        VariableInitializerValue::LiteralExpression(literal) => {
            format_literal_expression(literal, context)
        }
        VariableInitializerValue::NameExpression(name) => format_name_expression(name, context),
        VariableInitializerValue::ThisExpression(this) => format_this_expression(this, context),
        VariableInitializerValue::SuperExpression(super_expression) => {
            format_super_expression(super_expression, context)
        }
        VariableInitializerValue::ParenthesizedExpression(parenthesized) => {
            format_parenthesized_expression(parenthesized, context)
        }
        VariableInitializerValue::ClassLiteralExpression(class_literal) => {
            format_class_literal_expression(class_literal, context)
        }
        VariableInitializerValue::FieldAccessExpression(_)
        | VariableInitializerValue::ArrayAccessExpression(_)
        | VariableInitializerValue::MethodInvocationExpression(_) => {
            let expression = match value {
                VariableInitializerValue::FieldAccessExpression(field) => {
                    Expression::FieldAccessExpression(field.clone())
                }
                VariableInitializerValue::ArrayAccessExpression(array_access) => {
                    Expression::ArrayAccessExpression(array_access.clone())
                }
                VariableInitializerValue::MethodInvocationExpression(invocation) => {
                    Expression::MethodInvocationExpression(invocation.clone())
                }
                _ => unreachable!("matched selector initializer values"),
            };
            format_selector_chain(&expression, context, ChainRole::Default)
        }
        VariableInitializerValue::MethodReferenceExpression(reference) => {
            format_method_reference_expression(reference, context)
        }
        VariableInitializerValue::ObjectCreationExpression(creation) => {
            format_object_creation_expression(creation, context)
        }
        VariableInitializerValue::ArrayCreationExpression(creation) => {
            format_array_creation_expression(creation, context)
        }
        VariableInitializerValue::AssignmentExpression(assignment) => {
            format_assignment_expression(assignment, context)
        }
        VariableInitializerValue::BinaryExpression(binary) => {
            format_binary_expression(binary, context)
        }
        VariableInitializerValue::UnaryExpression(unary) => format_unary_expression(unary, context),
        VariableInitializerValue::PostfixExpression(postfix) => {
            format_postfix_expression(postfix, context)
        }
        VariableInitializerValue::CastExpression(cast) => format_cast_expression(cast, context),
        VariableInitializerValue::LambdaExpression(lambda) => {
            format_lambda_expression(lambda, context)
        }
        VariableInitializerValue::SwitchExpression(switch) => {
            format_switch_expression(switch, context)
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(initializer, context, false)
        }
        VariableInitializerValue::ConditionalExpression(conditional) => {
            format_conditional_expression(conditional, context)
        }
        VariableInitializerValue::InstanceofExpression(instanceof) => {
            format_instanceof_expression(instanceof, context)
        }
    }
}

pub(super) fn format_lambda_expression(
    lambda: &jolt_java_syntax::LambdaExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let parameters = if lambda.has_empty_parameter_list() {
        text("()")
    } else if let Some(parameter) = lambda.single_parameter() {
        format_lambda_parameter(&parameter, context)?
    } else {
        let parameters = lambda
            .parameters()
            .expect("validated lambda expression should have parameters");
        let l_paren = lambda.l_paren();
        let r_paren = lambda.r_paren();
        format_lambda_parameter_list(&parameters, l_paren.as_ref(), r_paren.as_ref(), context)?
    };

    let body = if let Some(expression) = lambda.expression_body() {
        let body = format_lambda_expression_body(&expression, context)?;
        return Ok(java_lambdas::expression_lambda(
            parameters,
            body,
            context.policy(),
        ));
    } else {
        let block = lambda
            .block_body()
            .expect("parser-clean lambda expression should have a body");
        format_block(&block, context)?
    };

    Ok(java_lambdas::block_lambda(parameters, body))
}

fn format_lambda_expression_body(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Expression::BinaryExpression(binary) = expression {
        return format_binary_expression_with_layout(
            binary,
            context,
            java_expressions::BinaryExpressionLayout::LambdaBody,
        );
    }

    format_expression_with_chain_role(expression, context, ChainRole::LambdaBody)
}

pub(super) fn format_lambda_parameter_list(
    parameters: &jolt_java_syntax::LambdaParameterList,
    l_paren: Option<&JavaSyntaxToken>,
    r_paren: Option<&JavaSyntaxToken>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = if let (Some(l_paren), Some(r_paren)) = (l_paren, r_paren) {
        TextRange::new(
            l_paren.token_text_range().start(),
            r_paren.token_text_range().end(),
        )
    } else {
        parameters.text_range()
    };
    let open_range = l_paren.map(JavaSyntaxToken::token_text_range);
    let parameters = parameters
        .parameters()
        .map(|parameter| {
            let range = parameter
                .code_text_range()
                .expect("validated lambda parameter should have a code range");
            let parameter = parameter.clone();
            Ok(java_lambdas::LambdaParameterItem::new(
                range,
                move |context| format_lambda_parameter(&parameter, context),
            ))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lambdas::parenthesized_parameter_list(parameters, list_range, open_range, context)
}

pub(super) fn format_lambda_parameter(
    parameter: &jolt_java_syntax::LambdaParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let name = parameter
        .name()
        .expect("validated lambda parameter should have a name");

    let final_prefix = parameter.final_token().map_or_else(
        || text(""),
        |token| concat([format_token(&token), text(" ")]),
    );

    if let Some(ty) = parameter.ty() {
        let ty_range = ty
            .code_text_range()
            .expect("parser-clean lambda parameter type should have a code range");
        let prefix = if let Some(ellipsis) = parameter.ellipsis() {
            concat([
                final_prefix,
                format_simple_expression_type(&ty, context, "lambda parameter")?,
                format_token(&ellipsis),
            ])
        } else {
            concat([
                final_prefix,
                format_simple_expression_type(&ty, context, "lambda parameter")?,
            ])
        };
        let boundary = parameter
            .ellipsis()
            .map_or(ty_range, |ellipsis| ellipsis.token_text_range());
        return Ok(format_lambda_parameter_name_gap(
            context, prefix, boundary, &name,
        ));
    }

    if let Some(var) = parameter.var_token() {
        let prefix = concat([final_prefix, format_token(&var)]);
        return Ok(format_lambda_parameter_name_gap(
            context,
            prefix,
            var.token_text_range(),
            &name,
        ));
    }

    Ok(concat([final_prefix, format_token(&name)]))
}

fn format_lambda_parameter_name_gap(
    context: &mut JavaFormatContext<'_>,
    prefix: Doc,
    boundary: TextRange,
    name: &JavaSyntaxToken,
) -> Doc {
    let name_range = name.token_text_range();
    if boundary.end() >= name_range.start() {
        return concat([prefix, text(" "), format_token(name)]);
    }

    let owner_range = TextRange::new(boundary.end(), name_range.start());
    let inline_comments =
        take_inline_leading_block_comment_docs_in_range(context, owner_range, name_range);
    let trailing_comments =
        take_trailing_line_comment_docs_in_range_as_own_line(context, boundary, owner_range);
    let name = format_token(name);

    if !trailing_comments.is_empty() {
        let mut parts = vec![prefix, text(" "), join(hard_line(), trailing_comments)];
        let mut name_parts = Vec::new();
        if !inline_comments.is_empty() {
            name_parts.push(join(text(" "), inline_comments));
            name_parts.push(text(" "));
        }
        name_parts.push(name);
        parts.push(indent_by(
            context.policy().continuation_indent_levels(),
            concat([hard_line(), concat(name_parts)]),
        ));
        return concat(parts);
    }

    let mut parts = vec![prefix, text(" ")];
    if !inline_comments.is_empty() {
        parts.push(join(text(" "), inline_comments));
        parts.push(text(" "));
    }
    parts.push(name);
    concat(parts)
}

pub(super) fn format_simple_expression_type(
    ty: &Type,
    context: &mut JavaFormatContext<'_>,
    type_context: &str,
) -> FormatResult<Doc> {
    let _ = type_context;
    format_type_layout_parts(&ty.layout_parts(), context)
}

pub(super) fn format_binary_expression(
    binary: &jolt_java_syntax::BinaryExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_binary_expression_with_layout(
        binary,
        context,
        java_expressions::BinaryExpressionLayout::Default,
    )
}

fn format_binary_expression_with_layout(
    binary: &jolt_java_syntax::BinaryExpression,
    context: &mut JavaFormatContext<'_>,
    layout: java_expressions::BinaryExpressionLayout,
) -> FormatResult<Doc> {
    let policy = context.policy();
    let mut formatter = BinaryExpressionRuleFormatter { context };
    java_expressions::binary_expression(binary, layout, policy, &mut formatter)
}

struct BinaryExpressionRuleFormatter<'context, 'source> {
    context: &'context mut JavaFormatContext<'source>,
}

impl java_expressions::BinaryExpressionFormatter for BinaryExpressionRuleFormatter<'_, '_> {
    fn format_operand(
        &mut self,
        java_expressions::BinaryOperandSlot {
            operand,
            parent_precedence,
            side,
            previous_range,
            next_operator_range,
        }: java_expressions::BinaryOperandSlot<'_>,
    ) -> FormatResult<java_expressions::BinaryOperand> {
        format_binary_operand_with_comments(
            operand,
            parent_precedence,
            side,
            previous_range,
            next_operator_range,
            self.context,
        )
    }

    fn format_operator(
        &mut self,
        java_expressions::BinaryOperatorSlot {
            operator,
            next_operand_range,
        }: java_expressions::BinaryOperatorSlot<'_>,
    ) -> java_expressions::BinaryOperator {
        format_binary_operator(operator, next_operand_range, self.context)
    }
}

fn format_binary_operator(
    operator: &JavaSyntaxToken,
    next_operand_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> java_expressions::BinaryOperator {
    let operator_range = operator.token_text_range();
    let trailing = take_trailing_line_comment_docs_in_range_as_own_line(
        context,
        operator_range,
        TextRange::new(operator_range.end(), next_operand_range.start()),
    );
    java_expressions::binary_operator_with_trailing_comments(format_token(operator), trailing)
}

fn format_assignment_operator(
    operator: &JavaSyntaxToken,
    right_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> java_expressions::AssignmentOperator {
    let operator_range = operator.token_text_range();
    let boundary = TextRange::new(operator_range.end(), right_range.start());
    let trailing_line =
        take_trailing_line_comment_docs_in_range_as_own_line(context, operator_range, boundary);
    let trailing_block =
        take_same_line_trailing_block_comment_docs_in_range(context, operator_range, boundary);
    java_expressions::assignment_operator_with_trailing_comments(
        format_token(operator),
        trailing_line,
        trailing_block,
    )
}

fn take_expression_gap_comment_docs(
    context: &mut JavaFormatContext<'_>,
    owner_range: TextRange,
    code_range: TextRange,
) -> FormatResult<Vec<Doc>> {
    let mut comments = take_leading_comment_docs_in_range(context, owner_range, code_range)?;
    let inline = take_inline_leading_block_comment_docs_in_range(context, owner_range, code_range);
    comments.extend(inline);
    Ok(comments)
}

fn format_binary_operand_with_comments(
    operand: &Expression,
    parent_precedence: u8,
    side: BinarySide,
    previous_range: TextRange,
    next_operator_range: Option<TextRange>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<java_expressions::BinaryOperand> {
    let operand_range = operand
        .code_text_range()
        .expect("parser-clean binary operand should have a code range");
    let leading = if previous_range.end() < operand_range.start() {
        take_expression_gap_comment_docs(
            context,
            TextRange::new(previous_range.end(), operand_range.start()),
            operand_range,
        )?
    } else {
        Vec::new()
    };
    let trailing = if let Some(next_operator_range) = next_operator_range {
        take_trailing_line_comment_docs_in_range_as_own_line(
            context,
            operand_range,
            TextRange::new(operand_range.end(), next_operator_range.start()),
        )
    } else {
        Vec::new()
    };

    Ok(java_expressions::binary_operand_from_parts(
        format_binary_operand(operand, parent_precedence, side, context)?,
        ExpressionLayout::for_expression(operand, context.policy()),
        leading,
        trailing,
    ))
}

fn format_binary_operand(
    operand: &Expression,
    parent_precedence: u8,
    side: BinarySide,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let doc = format_expression(operand, context)?;
    if binary_analysis::operand_needs_parentheses(operand, parent_precedence, side) {
        Ok(concat([text("("), doc, text(")")]))
    } else {
        Ok(doc)
    }
}

pub(super) fn unary_operator_needs_separator(
    operator: &JavaSyntaxToken,
    operand: &Expression,
) -> bool {
    let Expression::UnaryExpression(operand) = operand else {
        return false;
    };
    let Some(operand_operator) = operand.operator() else {
        return false;
    };
    matches!(
        (operator.kind(), operand_operator.kind()),
        (
            JavaSyntaxKind::Plus,
            JavaSyntaxKind::Plus | JavaSyntaxKind::PlusPlus
        ) | (
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Minus | JavaSyntaxKind::MinusMinus
        )
    )
}

pub(super) const fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
