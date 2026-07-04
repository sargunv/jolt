use super::{
    Doc, JavaFormatter, LambdaExpression, LambdaParameter, comment_forces_line, concat,
    format_annotation, format_block, format_expression, format_leading_comments, format_token_text,
    format_token_with_comments, format_trailing_comments_before_line_break, format_type, hard_line,
    inline_modifier_prefix_from_docs, text, token_iter_has_comments,
};

pub(super) fn format_lambda_expression(
    expression: &LambdaExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_lambda_parameters(expression, formatter),
        format_lambda_arrow(expression),
        expression.expression_body().map_or_else(
            || {
                expression
                    .block_body()
                    .map_or_else(jolt_fmt_ir::nil, |block| format_block(&block, formatter))
            },
            |body| format_expression(&body, formatter),
        ),
    ])
}

fn format_lambda_arrow(expression: &LambdaExpression) -> Doc {
    let Some(arrow) = expression.arrow() else {
        return text(" -> ");
    };

    if arrow.leading_comments().is_empty() && arrow.trailing_comments().is_empty() {
        return text(" -> ");
    }

    let trailing_comments = arrow.trailing_comments();
    let forced_line = trailing_comments.iter().any(comment_forces_line);

    concat([
        text(" "),
        format_leading_comments(&arrow),
        format_token_text(arrow.text()),
        format_trailing_comments_before_line_break(&arrow),
        if forced_line { hard_line() } else { text(" ") },
    ])
}

fn format_lambda_parameters(expression: &LambdaExpression, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(parameter) = expression.concise_parameter()
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        if token_iter_has_comments(parameter.token_iter()) {
            return format_lambda_parameter(&parameter, formatter);
        }
        return parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));
    }

    let parameter_list = expression.parameters();
    let parameters = parameter_list
        .as_ref()
        .map(|parameters| parameters.parameters().collect::<Vec<_>>())
        .unwrap_or_default();

    if let [parameter] = parameters.as_slice()
        && is_simple_untyped_lambda_parameter(parameter)
    {
        if token_iter_has_comments(parameter.token_iter()) {
            return format_lambda_parameter(parameter, formatter);
        }
        return parameter
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));
    }

    let open = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::open_paren)
        .or_else(|| expression.open_paren());
    let close = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::close_paren)
        .or_else(|| expression.close_paren());

    if open.is_none() && close.is_none() && parameters.is_empty() {
        return text("()");
    }

    concat([
        open.as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        jolt_fmt_ir::join(
            text(", "),
            parameters
                .into_iter()
                .map(|parameter| format_lambda_parameter(&parameter, formatter)),
        ),
        close
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
    ])
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter) -> bool {
    parameter.ty().is_none()
        && parameter.var_token().is_none()
        && !parameter.is_variable_arity()
        && parameter.prefix_annotations().next().is_none()
        && parameter.varargs_annotations().next().is_none()
        && parameter.modifier_tokens().next().is_none()
}

fn format_lambda_parameter(parameter: &LambdaParameter, formatter: &JavaFormatter<'_>) -> Doc {
    let prefix_annotations = parameter
        .prefix_annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let modifier_tokens = parameter.modifier_tokens().collect::<Vec<_>>();
    let has_inline_prefix = !prefix_annotations.is_empty() || !modifier_tokens.is_empty();
    let prefix = inline_modifier_prefix_from_docs(prefix_annotations, modifier_tokens);
    let ty = parameter.ty();
    let var_token = parameter.var_token();
    let has_type_prefix = ty.is_some() || var_token.is_some();
    let varargs_annotations = parameter
        .varargs_annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let ty = ty.map_or_else(
        || var_token.map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
        |ty| format_type(&ty, formatter),
    );
    let name = parameter
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));

    if !has_inline_prefix && !has_type_prefix {
        return name;
    }
    if !has_type_prefix {
        return concat([prefix, name]);
    }

    concat([
        prefix,
        ty,
        if parameter.is_variable_arity() {
            if let Some(ellipsis) = parameter.ellipsis_token() {
                if varargs_annotations.is_empty() {
                    concat([format_token_with_comments(&ellipsis), text(" ")])
                } else {
                    concat([
                        text(" "),
                        inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                        format_token_with_comments(&ellipsis),
                        text(" "),
                    ])
                }
            } else {
                jolt_fmt_ir::nil()
            }
        } else {
            text(" ")
        },
        name,
    ])
}
