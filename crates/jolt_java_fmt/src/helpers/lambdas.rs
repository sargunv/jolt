use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{Doc, best_fitting, concat, group, hard_line, indent_by, join, text};
use jolt_java_syntax::{BinaryExpression, Expression};

use crate::comments::{
    format_own_line_comment_doc, reject_unhandled_comments_in_range,
    take_inline_leading_block_comment_docs, take_inline_trailing_block_comment_docs,
    take_leading_comment_docs_in_range,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::helpers::expressions as java_expressions;
use crate::helpers::lists as java_lists;
use crate::policy::JavaFormatPolicy;

pub(crate) struct LambdaParameterItem {
    range: TextRange,
    format: Box<dyn for<'ctx> FnOnce(&mut JavaFormatContext<'ctx>) -> FormatResult<Doc>>,
}

impl LambdaParameterItem {
    pub(crate) fn new(
        range: TextRange,
        format: impl for<'ctx> FnOnce(&mut JavaFormatContext<'ctx>) -> FormatResult<Doc> + 'static,
    ) -> Self {
        Self {
            range,
            format: Box::new(format),
        }
    }
}

pub(crate) enum LambdaBody {
    Expression(Doc),
    Block(Doc),
}

pub(crate) trait LambdaBodyExpressionFormatter {
    fn format_binary_expression_body(
        &mut self,
        binary: &BinaryExpression,
        layout: java_expressions::BinaryExpressionLayout,
    ) -> FormatResult<Doc>;

    fn format_expression_body(&mut self, expression: &Expression) -> FormatResult<Doc>;
}

pub(crate) fn expression_body(
    expression: &Expression,
    formatter: &mut impl LambdaBodyExpressionFormatter,
) -> FormatResult<Doc> {
    if let Expression::BinaryExpression(binary) = expression {
        return formatter.format_binary_expression_body(
            binary,
            java_expressions::BinaryExpressionLayout::LambdaBody,
        );
    }

    formatter.format_expression_body(expression)
}

pub(crate) fn lambda_expression(
    parameters: Doc,
    body: LambdaBody,
    policy: JavaFormatPolicy,
) -> Doc {
    match body {
        LambdaBody::Expression(body) => expression_lambda(parameters, body, policy),
        LambdaBody::Block(body) => block_lambda(parameters, body),
    }
}

fn expression_lambda(parameters: Doc, body: Doc, policy: JavaFormatPolicy) -> Doc {
    if !policy.lambda_expression_body_breaks_after_arrow() {
        return concat([parameters, text(" -> "), body]);
    }

    let flat = concat([parameters.clone(), text(" -> "), body.clone()]);
    let broken = concat([
        parameters,
        text(" ->"),
        indent_by(
            policy.lambda_expression_body_indent_levels(),
            concat([hard_line(), body]),
        ),
    ]);
    best_fitting(flat, [broken])
}

fn block_lambda(parameters: Doc, body: Doc) -> Doc {
    concat([parameters, text(" -> "), body])
}

pub(crate) fn parenthesized_parameter_list(
    items: Vec<LambdaParameterItem>,
    list_range: TextRange,
    open_range: Option<TextRange>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if items.len() == 1 {
        let open_range = open_range.expect("parenthesized lambda parameters should have '('");
        return single_parenthesized_parameter(items, list_range, open_range, context);
    }

    let items = items
        .into_iter()
        .map(|item| java_lists::ListItem::new(item.range, item.format))
        .collect::<Vec<_>>();
    java_lists::lambda_parameter_list(items, list_range, open_range, context)
}

fn single_parenthesized_parameter(
    mut items: Vec<LambdaParameterItem>,
    list_range: TextRange,
    open_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let item = items
        .pop()
        .expect("single parenthesized lambda parameter should have one item");
    let opening_comment = context
        .take_trailing_line_comment(open_range)
        .map(|comment| format_own_line_comment_doc(context, &comment));
    let leading_comments = take_leading_comment_docs_in_range(context, list_range, item.range)?;
    let inline_leading = take_inline_leading_block_comment_docs(context, item.range);
    reject_unhandled_comments_in_range(
        context,
        TextRange::new(list_range.start(), item.range.start()),
        "Java formatter does not support comments before lambda parameters yet",
    )?;

    let mut parameter = (item.format)(context)?;
    if !inline_leading.is_empty() {
        parameter = concat([join(text(" "), inline_leading), text(" "), parameter]);
    }
    let inline_trailing = take_inline_trailing_block_comment_docs(context, item.range);
    if !inline_trailing.is_empty() {
        parameter = concat([parameter, text(" "), join(text(" "), inline_trailing)]);
    }

    let doc = match (opening_comment, leading_comments.is_empty()) {
        (Some(comment), _) => {
            let mut body = vec![hard_line()];
            if !leading_comments.is_empty() {
                body.push(join(hard_line(), leading_comments));
                body.push(hard_line());
            }
            body.push(parameter);
            concat([
                text("("),
                text(" "),
                comment,
                indent_by(context.policy().continuation_indent_levels(), concat(body)),
                text(")"),
            ])
        }
        (None, false) => concat([
            text("("),
            indent_by(
                context.policy().continuation_indent_levels(),
                concat([
                    hard_line(),
                    join(hard_line(), leading_comments),
                    hard_line(),
                    parameter,
                ]),
            ),
            text(")"),
        ]),
        (None, true) => concat([
            text("("),
            indent_by(context.policy().continuation_indent_levels(), parameter),
            text(")"),
        ]),
    };

    reject_unhandled_comments_in_range(
        context,
        list_range,
        "Java formatter does not support dangling comments inside lambda parameters yet",
    )?;
    Ok(group(doc))
}
