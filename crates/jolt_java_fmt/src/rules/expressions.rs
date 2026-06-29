use super::{
    ArrayDimensions, ArrayInitializer, Doc, Expression, FormatResult, JavaFormatContext,
    JavaSyntaxKind, JavaSyntaxToken, MethodReferenceExpression, Type, TypeArgumentList,
    TypeLayoutPart, VariableInitializerValue, concat, format_annotation_list, format_block,
    format_switch_expression, format_token, format_type, hard_line, join, missing_layout,
    take_inline_leading_block_comment_docs, take_inline_trailing_block_comment_docs, text, wrap,
};

pub(super) fn format_expression(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match expression {
        Expression::LiteralExpression(literal) => format_literal_expression(literal),
        Expression::NameExpression(name) => format_name_expression(name),
        Expression::ThisExpression(this) => format_this_expression(this),
        Expression::SuperExpression(super_expression) => format_super_expression(super_expression),
        Expression::ParenthesizedExpression(parenthesized) => {
            format_parenthesized_expression(parenthesized, context)
        }
        Expression::ClassLiteralExpression(class_literal) => {
            format_class_literal_expression(class_literal, context)
        }
        Expression::FieldAccessExpression(_) | Expression::MethodInvocationExpression(_) => {
            format_selector_chain(expression, context)
        }
        Expression::ArrayAccessExpression(array_access) => {
            format_array_access_expression(array_access, context)
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
        Expression::SwitchExpression(switch) => format_switch_expression(switch, context),
        _ => Err(missing_layout(
            format!(
                "Java formatter does not support expression kind {:?} yet",
                expression.kind()
            ),
            expression.text_range(),
        )),
    }
}

pub(super) fn format_selector_chain(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let (base, selectors) = collect_selector_chain(expression, context)?;
    Ok(wrap::dot_chain(base, selectors))
}

pub(super) fn collect_selector_chain(
    expression: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Vec<Doc>)> {
    match expression {
        Expression::NameExpression(name) => Ok((format_name_expression(name)?, Vec::new())),
        Expression::ThisExpression(this) => Ok((format_this_expression(this)?, Vec::new())),
        Expression::SuperExpression(super_expression) => {
            Ok((format_super_expression(super_expression)?, Vec::new()))
        }
        Expression::ObjectCreationExpression(creation) => Ok((
            format_object_creation_expression(creation, context)?,
            Vec::new(),
        )),
        Expression::ArrayAccessExpression(array_access) => Ok((
            format_array_access_expression(array_access, context)?,
            Vec::new(),
        )),
        Expression::ClassLiteralExpression(class_literal) => Ok((
            format_class_literal_expression(class_literal, context)?,
            Vec::new(),
        )),
        Expression::ParenthesizedExpression(parenthesized) => Ok((
            format_parenthesized_expression(parenthesized, context)?,
            Vec::new(),
        )),
        Expression::FieldAccessExpression(field) => collect_field_access_chain(field, context),
        Expression::MethodInvocationExpression(invocation) => {
            collect_method_invocation_chain(invocation, context)
        }
        _ => Err(missing_layout(
            "Java formatter does not support this selector chain expression yet",
            expression.text_range(),
        )),
    }
}

pub(super) fn collect_field_access_chain(
    field: &jolt_java_syntax::FieldAccessExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Vec<Doc>)> {
    if !field.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this field access expression shape yet",
            field.text_range(),
        ));
    }
    let receiver = field.receiver().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field access expression without a receiver",
            field.text_range(),
        )
    })?;
    if !is_supported_selector_receiver(&receiver) {
        return Err(missing_layout(
            "Java formatter does not support this field access receiver yet",
            receiver.text_range(),
        ));
    }
    let name = field.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a field access expression without a name",
            field.text_range(),
        )
    })?;

    let (base, mut selectors) = collect_selector_chain(&receiver, context)?;
    selectors.push(text(name.text()));
    Ok((base, selectors))
}

pub(super) fn collect_method_invocation_chain(
    invocation: &jolt_java_syntax::MethodInvocationExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Vec<Doc>)> {
    if !invocation.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method invocation shape yet",
            invocation.text_range(),
        ));
    }

    let arguments = invocation.arguments().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method invocation without arguments",
            invocation.text_range(),
        )
    })?;
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method invocation argument shape yet",
            arguments.text_range(),
        ));
    }
    let arguments = format_argument_list(&arguments, context)?;

    if let Some(receiver) = invocation.receiver() {
        if !is_supported_selector_receiver(&receiver) {
            return Err(missing_layout(
                "Java formatter does not support this method invocation receiver yet",
                receiver.text_range(),
            ));
        }
        let name = invocation.name().ok_or_else(|| {
            missing_layout(
                "Java formatter found a qualified method invocation without a name",
                invocation.text_range(),
            )
        })?;
        let type_arguments = invocation
            .type_arguments()
            .map(|arguments| format_type_argument_list(&arguments))
            .transpose()?;
        let (base, mut selectors) = collect_selector_chain(&receiver, context)?;
        selectors.push(concat([
            type_arguments.unwrap_or_else(|| text("")),
            text(name.text()),
            arguments,
        ]));
        return Ok((base, selectors));
    }

    let name = invocation.simple_name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a method invocation without a simple name",
            invocation.text_range(),
        )
    })?;
    Ok((concat([text(name.text()), arguments]), Vec::new()))
}

pub(super) fn format_method_reference_expression(
    reference: &MethodReferenceExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !reference.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this method reference expression shape yet",
            reference.text_range(),
        ));
    }

    let qualifier = if let Some(expression) = reference.expression_qualifier() {
        format_expression(&expression, context)?
    } else {
        let ty = reference
            .type_qualifier()
            .expect("validated method reference should have a qualifier");
        format_simple_expression_type(&ty, "method reference qualifier")?
    };
    let dimensions = reference
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions))
        .transpose()?;
    let type_arguments = reference
        .type_arguments()
        .map(|arguments| format_type_argument_list(&arguments))
        .transpose()?;

    let member = if reference.is_constructor_reference() {
        text("new")
    } else {
        let name = reference
            .name()
            .expect("validated method reference should have a member name");
        format_token(&name)
    };

    Ok(concat([
        qualifier,
        dimensions.unwrap_or_else(|| text("")),
        text("::"),
        type_arguments.unwrap_or_else(|| text("")),
        member,
    ]))
}

pub(super) fn format_literal_expression(
    literal: &jolt_java_syntax::LiteralExpression,
) -> FormatResult<Doc> {
    let token = literal.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this literal expression shape yet",
            literal.text_range(),
        )
    })?;
    if token.text().contains(is_line_terminator) {
        return Err(missing_layout(
            "Java formatter does not support multiline literals yet",
            token.text_range(),
        ));
    }
    Ok(format_token(&token))
}

pub(super) fn format_name_expression(name: &jolt_java_syntax::NameExpression) -> FormatResult<Doc> {
    let identifier = name.identifier().ok_or_else(|| {
        missing_layout(
            "Java formatter only supports simple name expressions yet",
            name.text_range(),
        )
    })?;
    Ok(format_token(&identifier))
}

pub(super) fn format_this_expression(this: &jolt_java_syntax::ThisExpression) -> FormatResult<Doc> {
    let token = this.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this expression shape yet",
            this.text_range(),
        )
    })?;
    Ok(format_token(&token))
}

pub(super) fn format_super_expression(
    super_expression: &jolt_java_syntax::SuperExpression,
) -> FormatResult<Doc> {
    let token = super_expression.token().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support super expression shape yet",
            super_expression.text_range(),
        )
    })?;
    Ok(format_token(&token))
}

pub(super) fn format_parenthesized_expression(
    parenthesized: &jolt_java_syntax::ParenthesizedExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !parenthesized.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this parenthesized expression shape yet",
            parenthesized.text_range(),
        ));
    }
    let expression = parenthesized.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a parenthesized expression without an expression",
            parenthesized.text_range(),
        )
    })?;
    Ok(wrap::parenthesized_expression(format_expression(
        &expression,
        context,
    )?))
}

pub(super) fn format_class_literal_expression(
    class_literal: &jolt_java_syntax::ClassLiteralExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !class_literal.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this class literal expression shape yet",
            class_literal.text_range(),
        ));
    }

    let class_token = class_literal.class_token().ok_or_else(|| {
        missing_layout(
            "Java formatter found a class literal expression without `class`",
            class_literal.text_range(),
        )
    })?;

    let qualifier = if let Some(expression) = class_literal.expression() {
        format_expression(&expression, context)?
    } else if let Some(token) = class_literal.primitive_or_void_token() {
        format_token(&token)
    } else {
        let ty = class_literal.ty().ok_or_else(|| {
            missing_layout(
                "Java formatter found a class literal expression without a qualifier",
                class_literal.text_range(),
            )
        })?;
        let parts = ty.simple_layout_parts().ok_or_else(|| {
            missing_layout(
                "Java formatter does not support this class literal type qualifier yet",
                ty.text_range(),
            )
        })?;
        let mut docs = Vec::new();
        for part in parts {
            match part {
                TypeLayoutPart::Text(value) => docs.push(text(value)),
                TypeLayoutPart::Token(token) => docs.push(format_token(&token)),
                TypeLayoutPart::Annotation(annotation) => {
                    return Err(missing_layout(
                        "Java formatter does not support annotated class literal type qualifiers yet",
                        annotation.text_range(),
                    ));
                }
            }
        }
        concat(docs)
    };
    let dimensions = class_literal
        .dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions))
        .transpose()?;

    Ok(concat([
        qualifier,
        dimensions.unwrap_or_else(|| text("")),
        text("."),
        format_token(&class_token),
    ]))
}

pub(super) fn format_array_dimensions(dimensions: &ArrayDimensions) -> FormatResult<Doc> {
    let count = dimensions.simple_layout_count().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this array dimensions shape yet",
            dimensions.text_range(),
        )
    })?;

    Ok(concat(std::iter::repeat_n(text("[]"), count)))
}

pub(super) fn format_unary_expression(
    unary: &jolt_java_syntax::UnaryExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !unary.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this unary expression shape yet",
            unary.text_range(),
        ));
    }
    let operator = unary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a unary expression without an operator",
            unary.text_range(),
        )
    })?;
    let operand = unary.operand().ok_or_else(|| {
        missing_layout(
            "Java formatter found a unary expression without an operand",
            unary.text_range(),
        )
    })?;
    if matches!(
        operand,
        Expression::AssignmentExpression(_) | Expression::BinaryExpression(_)
    ) {
        return Err(missing_layout(
            "Java formatter does not support this unary operand without parentheses",
            operand.text_range(),
        ));
    }
    if matches!(
        operator.kind(),
        JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
    ) && !is_supported_assignment_left(&operand)
    {
        return Err(missing_layout(
            "Java formatter does not support this update operand yet",
            operand.text_range(),
        ));
    }
    let separator = if unary_operator_needs_separator(&operator, &operand) {
        text(" ")
    } else {
        text("")
    };
    Ok(concat([
        format_token(&operator),
        separator,
        format_expression(&operand, context)?,
    ]))
}

pub(super) fn format_postfix_expression(
    postfix: &jolt_java_syntax::PostfixExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !postfix.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this postfix expression shape yet",
            postfix.text_range(),
        ));
    }
    let operand = postfix.operand().ok_or_else(|| {
        missing_layout(
            "Java formatter found a postfix expression without an operand",
            postfix.text_range(),
        )
    })?;
    if matches!(
        operand,
        Expression::AssignmentExpression(_) | Expression::BinaryExpression(_)
    ) {
        return Err(missing_layout(
            "Java formatter does not support this postfix operand without parentheses",
            operand.text_range(),
        ));
    }
    if !is_supported_assignment_left(&operand) {
        return Err(missing_layout(
            "Java formatter does not support this postfix operand yet",
            operand.text_range(),
        ));
    }
    let operator = postfix.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a postfix expression without an operator",
            postfix.text_range(),
        )
    })?;
    Ok(concat([
        format_expression(&operand, context)?,
        format_token(&operator),
    ]))
}

pub(super) fn format_assignment_expression(
    assignment: &jolt_java_syntax::AssignmentExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !assignment.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this assignment expression shape yet",
            assignment.text_range(),
        ));
    }
    let left = assignment.left().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without a left side",
            assignment.text_range(),
        )
    })?;
    if !is_supported_assignment_left(&left) {
        return Err(missing_layout(
            "Java formatter does not support this assignment left side yet",
            left.text_range(),
        ));
    }
    let operator = assignment.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without an operator",
            assignment.text_range(),
        )
    })?;
    let right = assignment.right().ok_or_else(|| {
        missing_layout(
            "Java formatter found an assignment expression without a right side",
            assignment.text_range(),
        )
    })?;
    Ok(wrap::assignment_expression(
        format_expression(&left, context)?,
        format_token(&operator),
        format_expression(&right, context)?,
    ))
}

pub(super) fn format_array_access_expression(
    array_access: &jolt_java_syntax::ArrayAccessExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !array_access.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this array access expression shape yet",
            array_access.text_range(),
        ));
    }
    let receiver = array_access.receiver().ok_or_else(|| {
        missing_layout(
            "Java formatter found an array access expression without a receiver",
            array_access.text_range(),
        )
    })?;
    let index = array_access.index().ok_or_else(|| {
        missing_layout(
            "Java formatter found an array access expression without an index",
            array_access.text_range(),
        )
    })?;

    Ok(concat([
        format_expression(&receiver, context)?,
        text("["),
        format_expression(&index, context)?,
        text("]"),
    ]))
}

pub(super) fn format_argument_list(
    arguments: &jolt_java_syntax::ArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this argument list shape yet",
            arguments.text_range(),
        ));
    }
    let arguments = arguments
        .arguments()
        .map(|argument| format_argument(&argument, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(wrap::parenthesized_comma_list(arguments))
}

pub(super) fn format_argument(
    argument: &Expression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = argument.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty argument expression",
            argument.text_range(),
        )
    })?;
    let comments = take_inline_leading_block_comment_docs(context, code_range);
    let expression = format_expression(argument, context)?;
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

pub(super) fn format_cast_expression(
    cast: &jolt_java_syntax::CastExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = cast.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found a cast expression without a type",
            cast.text_range(),
        )
    })?;
    let expression = cast.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a cast expression without an operand",
            cast.text_range(),
        )
    })?;

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
    if !creation.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this object creation expression shape yet",
            creation.text_range(),
        ));
    }
    let ty = creation.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found an object creation expression without a type",
            creation.text_range(),
        )
    })?;
    let arguments = creation.arguments().ok_or_else(|| {
        missing_layout(
            "Java formatter found an object creation expression without arguments",
            creation.text_range(),
        )
    })?;

    Ok(concat([
        text("new "),
        format_type(&ty, context)?,
        format_argument_list(&arguments, context)?,
    ]))
}

pub(super) fn format_array_creation_expression(
    creation: &jolt_java_syntax::ArrayCreationExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = creation.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found an array creation expression without a type",
            creation.text_range(),
        )
    })?;

    let dimensions = creation
        .dimensions()
        .map(|dimension| format_dim_expression(&dimension, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let trailing_dimensions = creation
        .trailing_dimensions()
        .map(|dimensions| format_array_dimensions(&dimensions))
        .transpose()?;
    let initializer = creation
        .initializer()
        .map(|initializer| format_array_initializer(&initializer, context))
        .transpose()?;

    if dimensions.is_empty() && initializer.is_none() {
        return Err(missing_layout(
            "Java formatter found an array creation expression without dimensions or an initializer",
            creation.text_range(),
        ));
    }

    let mut parts = vec![text("new "), format_type(&ty, context)?];
    parts.extend(dimensions);
    if let Some(trailing_dimensions) = trailing_dimensions {
        parts.push(trailing_dimensions);
    }
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
    let expression = dimension.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found an array dimension without an expression",
            dimension.text_range(),
        )
    })?;

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
) -> FormatResult<Doc> {
    let values = initializer
        .values()
        .map(|value| format_variable_initializer_value(&value, context))
        .collect::<FormatResult<Vec<_>>>()?;

    if values.is_empty() {
        return Ok(text("{}"));
    }

    Ok(concat([
        text("{"),
        jolt_fmt_ir::indent(concat([
            hard_line(),
            join(concat([text(","), hard_line()]), values),
        ])),
        hard_line(),
        text("}"),
    ]))
}

pub(super) fn format_variable_initializer_value(
    value: &VariableInitializerValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match value {
        VariableInitializerValue::LiteralExpression(literal) => format_literal_expression(literal),
        VariableInitializerValue::NameExpression(name) => format_name_expression(name),
        VariableInitializerValue::ThisExpression(this) => format_this_expression(this),
        VariableInitializerValue::SuperExpression(super_expression) => {
            format_super_expression(super_expression)
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
            format_selector_chain(&expression, context)
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
            format_array_initializer(initializer, context)
        }
        VariableInitializerValue::ConditionalExpression(_)
        | VariableInitializerValue::InstanceofExpression(_) => Err(missing_layout(
            format!(
                "Java formatter does not support array initializer value kind {:?} yet",
                value.kind()
            ),
            value.text_range(),
        )),
    }
}

pub(super) fn format_lambda_expression(
    lambda: &jolt_java_syntax::LambdaExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !lambda.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this lambda expression shape yet",
            lambda.text_range(),
        ));
    }

    let parameters = if lambda.has_empty_parameter_list() {
        text("()")
    } else if let Some(parameter) = lambda.single_parameter() {
        format_lambda_parameter(&parameter)?
    } else {
        let parameters = lambda
            .parameters()
            .expect("validated lambda expression should have parameters");
        format_lambda_parameter_list(&parameters)?
    };

    let body = if let Some(expression) = lambda.expression_body() {
        format_expression(&expression, context)?
    } else {
        let block = lambda.block_body().ok_or_else(|| {
            missing_layout(
                "Java formatter found a lambda expression without a body",
                lambda.text_range(),
            )
        })?;
        format_block(&block, context)?
    };

    Ok(concat([parameters, text(" -> "), body]))
}

pub(super) fn format_lambda_parameter_list(
    parameters: &jolt_java_syntax::LambdaParameterList,
) -> FormatResult<Doc> {
    let parameters = parameters
        .parameters()
        .map(|parameter| format_lambda_parameter(&parameter))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(wrap::parenthesized_comma_list(parameters))
}

pub(super) fn format_lambda_parameter(
    parameter: &jolt_java_syntax::LambdaParameter,
) -> FormatResult<Doc> {
    let name = parameter
        .name()
        .expect("validated lambda parameter should have a name");

    let final_prefix = parameter.final_token().map_or_else(
        || text(""),
        |token| concat([format_token(&token), text(" ")]),
    );

    if let Some(ty) = parameter.ty() {
        let prefix = if let Some(ellipsis) = parameter.ellipsis() {
            concat([
                final_prefix,
                format_simple_expression_type(&ty, "lambda parameter")?,
                format_token(&ellipsis),
                text(" "),
            ])
        } else {
            concat([
                final_prefix,
                format_simple_expression_type(&ty, "lambda parameter")?,
                text(" "),
            ])
        };
        return Ok(concat([prefix, format_token(&name)]));
    }

    if let Some(var) = parameter.var_token() {
        return Ok(concat([
            final_prefix,
            format_token(&var),
            text(" "),
            format_token(&name),
        ]));
    }

    Ok(concat([final_prefix, format_token(&name)]))
}

pub(super) fn format_simple_expression_type(ty: &Type, context: &str) -> FormatResult<Doc> {
    let parts = ty.simple_layout_parts().ok_or_else(|| {
        missing_layout(
            format!("Java formatter does not support this {context} type shape yet"),
            ty.text_range(),
        )
    })?;
    format_simple_type_layout_parts(parts, context)
}

pub(super) fn format_type_argument_list(arguments: &TypeArgumentList) -> FormatResult<Doc> {
    let parts = arguments
        .simple_layout_parts()
        .expect("validated method invocation should have supported type arguments");
    format_simple_type_layout_parts(parts, "method invocation type argument")
}

pub(super) fn format_simple_type_layout_parts(
    parts: Vec<TypeLayoutPart>,
    context: &str,
) -> FormatResult<Doc> {
    let mut docs = Vec::new();
    for part in parts {
        match part {
            TypeLayoutPart::Text(value) => docs.push(text(value)),
            TypeLayoutPart::Token(token) => docs.push(format_token(&token)),
            TypeLayoutPart::Annotation(annotation) => {
                return Err(missing_layout(
                    format!("Java formatter does not support annotated {context} types yet"),
                    annotation.text_range(),
                ));
            }
        }
    }

    Ok(concat(docs))
}

pub(super) fn format_binary_expression(
    binary: &jolt_java_syntax::BinaryExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !binary.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this binary expression shape yet",
            binary.text_range(),
        ));
    }
    let operator = binary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without an operator",
            binary.text_range(),
        )
    })?;
    let precedence = binary_precedence(operator.kind()).ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this binary operator yet",
            operator.text_range(),
        )
    })?;
    let left = binary.left().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a left side",
            binary.text_range(),
        )
    })?;
    let right = binary.right().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a right side",
            binary.text_range(),
        )
    })?;

    let mut first = None;
    let mut rest = Vec::new();
    collect_binary_left_chain(&left, precedence, &mut first, &mut rest, context)?;
    rest.push((
        format_token(&operator),
        format_binary_operand(&right, precedence, BinarySide::Right, context)?,
    ));

    let first = first.ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without a left chain",
            binary.text_range(),
        )
    })?;
    Ok(wrap::binary_chain(first, rest))
}

#[derive(Clone, Copy)]
enum BinarySide {
    Left,
    Right,
}

pub(super) fn collect_binary_left_chain(
    expression: &Expression,
    parent_precedence: u8,
    first: &mut Option<Doc>,
    rest: &mut Vec<(Doc, Doc)>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<()> {
    if let Expression::BinaryExpression(binary) = expression
        && binary.has_supported_layout_shape()
    {
        let operator = binary.operator().ok_or_else(|| {
            missing_layout(
                "Java formatter found a binary expression without an operator",
                binary.text_range(),
            )
        })?;
        let child_precedence = binary_precedence(operator.kind()).ok_or_else(|| {
            missing_layout(
                "Java formatter does not support this binary operator yet",
                operator.text_range(),
            )
        })?;
        if child_precedence == parent_precedence {
            let left = binary.left().ok_or_else(|| {
                missing_layout(
                    "Java formatter found a binary expression without a left side",
                    binary.text_range(),
                )
            })?;
            let right = binary.right().ok_or_else(|| {
                missing_layout(
                    "Java formatter found a binary expression without a right side",
                    binary.text_range(),
                )
            })?;

            collect_binary_left_chain(&left, parent_precedence, first, rest, context)?;
            rest.push((
                format_token(&operator),
                format_binary_operand(&right, parent_precedence, BinarySide::Right, context)?,
            ));
            return Ok(());
        }
    }

    *first = Some(format_binary_operand(
        expression,
        parent_precedence,
        BinarySide::Left,
        context,
    )?);
    Ok(())
}

fn format_binary_operand(
    operand: &Expression,
    parent_precedence: u8,
    side: BinarySide,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let doc = format_expression(operand, context)?;
    let Expression::BinaryExpression(binary) = operand else {
        return Ok(doc);
    };
    let operator = binary.operator().ok_or_else(|| {
        missing_layout(
            "Java formatter found a binary expression without an operator",
            binary.text_range(),
        )
    })?;
    let child_precedence = binary_precedence(operator.kind()).ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this binary operator yet",
            operator.text_range(),
        )
    })?;
    let needs_parentheses = child_precedence < parent_precedence
        || (child_precedence == parent_precedence && matches!(side, BinarySide::Right));
    if needs_parentheses {
        Ok(concat([text("("), doc, text(")")]))
    } else {
        Ok(doc)
    }
}

pub(super) fn binary_precedence(kind: JavaSyntaxKind) -> Option<u8> {
    match kind {
        JavaSyntaxKind::OrOr => Some(3),
        JavaSyntaxKind::AndAnd => Some(4),
        JavaSyntaxKind::Bar => Some(5),
        JavaSyntaxKind::Caret => Some(6),
        JavaSyntaxKind::Amp => Some(7),
        JavaSyntaxKind::EqEq | JavaSyntaxKind::BangEq => Some(8),
        JavaSyntaxKind::Lt | JavaSyntaxKind::Gt | JavaSyntaxKind::LtEq | JavaSyntaxKind::GtEq => {
            Some(9)
        }
        JavaSyntaxKind::LShift | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => {
            Some(10)
        }
        JavaSyntaxKind::Plus | JavaSyntaxKind::Minus => Some(11),
        JavaSyntaxKind::Star | JavaSyntaxKind::Slash | JavaSyntaxKind::Percent => Some(12),
        _ => None,
    }
}

pub(super) fn is_supported_selector_receiver(expression: &Expression) -> bool {
    match expression {
        Expression::NameExpression(_)
        | Expression::ThisExpression(_)
        | Expression::SuperExpression(_)
        | Expression::ClassLiteralExpression(_)
        | Expression::FieldAccessExpression(_)
        | Expression::MethodInvocationExpression(_)
        | Expression::ArrayAccessExpression(_)
        | Expression::ObjectCreationExpression(_) => true,
        Expression::ParenthesizedExpression(parenthesized) => parenthesized
            .expression()
            .is_some_and(|inner| is_supported_selector_receiver(&inner)),
        _ => false,
    }
}

pub(super) fn is_supported_assignment_left(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::NameExpression(_)
            | Expression::FieldAccessExpression(_)
            | Expression::ArrayAccessExpression(_)
    )
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
