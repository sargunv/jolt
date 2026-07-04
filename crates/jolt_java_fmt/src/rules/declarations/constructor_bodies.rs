use super::{
    BlockStatement, BodyItem, ConstructorInvocation, Doc, JavaFormatter, JavaSyntaxToken, Range,
    concat, format_argument_list, format_block_statement_item, format_construct_leading_comments,
    format_dangling_comments, format_expression, format_name, format_statement_semicolon,
    format_token_after_construct_leading_comments, format_token_with_comments,
    format_type_argument_list, formatter_ignore_ranges, formatter_ignore_run_doc,
    formatter_ignore_runs, join_body_items, non_formatter_control_comments,
    relative_token_range_between,
};

pub(super) fn format_constructor_body<'source>(
    body: &jolt_java_syntax::ConstructorBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let elements = constructor_body_elements(body);
    let element_ranges = elements
        .iter()
        .map(|element| {
            constructor_body_element_token_range(element, body.text_range().start().get())
        })
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &element_ranges);
    let mut items = Vec::new();
    items.extend(format_constructor_body_open_dangling_comments(
        body.open_brace(),
    ));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (element_index, element) in elements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == element_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= element_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(element_index) {
            continue;
        }

        let Some(mut item) = format_constructor_body_element(element, formatter) else {
            continue;
        };
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == element_index {
            item = item.without_blank_line_before();
        }
        items.push(item);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
        ignored_index += 1;
    }
    items.extend(format_constructor_body_close_dangling_comments(
        body.close_brace(),
    ));

    (!items.is_empty()).then(|| join_body_items(items))
}

fn format_constructor_body_open_dangling_comments(
    open: Option<JavaSyntaxToken<'_>>,
) -> Option<BodyItem<'_>> {
    let comments = non_formatter_control_comments(open?.trailing_comments());
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn format_constructor_body_close_dangling_comments(
    close: Option<JavaSyntaxToken<'_>>,
) -> Option<BodyItem<'_>> {
    let comments = non_formatter_control_comments(close?.leading_comments());
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn constructor_body_elements<'source>(
    body: &jolt_java_syntax::ConstructorBody<'source>,
) -> Vec<ConstructorBodyElement<'source>> {
    body.invocation()
        .into_iter()
        .map(ConstructorBodyElement::Invocation)
        .chain(
            body.block_statements()
                .map(ConstructorBodyElement::BlockStatement),
        )
        .collect()
}

fn constructor_body_element_token_range(
    element: &ConstructorBodyElement<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &element.first_token()?,
        &element.last_token()?,
        body_start,
    ))
}

fn format_constructor_body_element<'source>(
    element: &ConstructorBodyElement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<BodyItem<'source>> {
    match element {
        ConstructorBodyElement::Invocation(invocation) => Some(BodyItem::new(
            format_constructor_invocation(invocation, formatter),
            invocation.starts_after_blank_line(),
        )),
        ConstructorBodyElement::BlockStatement(statement) => {
            format_block_statement_item(statement, formatter)
        }
    }
}

fn format_constructor_invocation<'source>(
    invocation: &ConstructorInvocation<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let invocation_first_token = invocation.first_token();
    concat([
        format_construct_leading_comments(invocation_first_token.as_ref()),
        format_constructor_invocation_qualifier(invocation, formatter),
        invocation
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
        invocation.target().map_or_else(jolt_fmt_ir::nil, |target| {
            format_token_after_construct_leading_comments(&target, invocation_first_token.as_ref())
        }),
        format_argument_list(invocation.arguments(), formatter),
        format_statement_semicolon(invocation.semicolon()),
    ])
}

fn format_constructor_invocation_qualifier<'source>(
    invocation: &ConstructorInvocation<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(name) = invocation.qualifier_name() {
        return concat([
            format_name(&name),
            invocation
                .dot_token()
                .as_ref()
                .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        ]);
    }
    invocation
        .qualifier_expression()
        .map_or_else(jolt_fmt_ir::nil, |expression| {
            concat([
                format_expression(&expression, formatter),
                invocation
                    .dot_token()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
            ])
        })
}

enum ConstructorBodyElement<'source> {
    Invocation(ConstructorInvocation<'source>),
    BlockStatement(BlockStatement<'source>),
}

impl<'source> ConstructorBodyElement<'source> {
    fn first_token(&self) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
        match self {
            Self::Invocation(invocation) => invocation.first_token(),
            Self::BlockStatement(statement) => statement.first_token(),
        }
    }

    fn last_token(&self) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
        match self {
            Self::Invocation(invocation) => invocation.last_token(),
            Self::BlockStatement(statement) => statement.last_token(),
        }
    }
}
