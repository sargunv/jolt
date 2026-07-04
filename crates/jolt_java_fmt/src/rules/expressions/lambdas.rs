use super::{
    Doc, JavaFormatter, LambdaExpression, LambdaParameter, comment_forces_line, concat,
    format_annotation, format_block, format_expression, format_leading_comments,
    format_separator_with_comments, format_token_text, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type, hard_line,
    inline_modifier_prefix_from_docs, token_iter_has_comments,
};
use jolt_fmt_ir::space;

pub(super) fn format_lambda_expression<'source>(
    expression: &LambdaExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_lambda_arrow<'source>(expression: &LambdaExpression<'source>) -> Doc<'source> {
    let Some(arrow) = expression.arrow() else {
        return jolt_fmt_ir::nil();
    };

    if arrow.leading_comments().is_empty() && arrow.trailing_comments().is_empty() {
        return concat([space(), format_separator_with_comments(&arrow, space())]);
    }

    let forced_line = arrow
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));

    concat([
        space(),
        format_leading_comments(&arrow),
        format_token_text(arrow.text()),
        format_trailing_comments_before_line_break(&arrow),
        if forced_line { hard_line() } else { space() },
    ])
}

fn format_lambda_parameters<'source>(
    expression: &LambdaExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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
    if let Some(parameters) = parameter_list.as_ref() {
        let mut parameters = parameters.parameters();
        if let Some(parameter) = parameters.next()
            && parameters.next().is_none()
            && is_simple_untyped_lambda_parameter(&parameter)
        {
            if token_iter_has_comments(parameter.token_iter()) {
                return format_lambda_parameter(&parameter, formatter);
            }
            return parameter
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));
        }
    }

    let open = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::open_paren)
        .or_else(|| expression.open_paren());
    let close = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::close_paren)
        .or_else(|| expression.close_paren());
    let has_parameters = parameter_list
        .as_ref()
        .is_some_and(|parameters| parameters.parameters().next().is_some());

    if open.is_none() && close.is_none() && !has_parameters {
        return jolt_fmt_ir::nil();
    }

    concat([
        open.as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        parameter_list
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_lambda_parameter_entries(parameters, formatter)
            }),
        close
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
    ])
}

fn format_lambda_parameter_entries<'source>(
    parameters: &jolt_java_syntax::LambdaParameterList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut entries = parameters.entries().peekable();
    while let Some(entry) = entries.next() {
        docs.push(format_lambda_parameter(&entry.parameter, formatter));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, space()));
        } else if entries.peek().is_some() {
            docs.push(space());
        }
    }
    concat(docs)
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter<'_>) -> bool {
    parameter.ty().is_none()
        && parameter.var_token().is_none()
        && !parameter.is_variable_arity()
        && parameter.prefix_annotations().next().is_none()
        && parameter.varargs_annotations().next().is_none()
        && parameter.modifier_tokens().next().is_none()
}

fn format_lambda_parameter<'source>(
    parameter: &LambdaParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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
                    concat([format_token_with_comments(&ellipsis), space()])
                } else {
                    concat([
                        space(),
                        inline_modifier_prefix_from_docs(varargs_annotations, Vec::new()),
                        format_token_with_comments(&ellipsis),
                        space(),
                    ])
                }
            } else {
                jolt_fmt_ir::nil()
            }
        } else {
            space()
        },
        name,
    ])
}
