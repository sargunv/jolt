use super::{
    BodyItem, ConstructorInvocation, Doc, JavaSyntaxToken, LeadingTrivia, Range,
    format_argument_list, format_block_statement_item_or_recovered,
    format_construct_leading_comments, format_expression, format_name, format_removed_comments,
    format_statement_semicolon, format_token_after_construct_leading_comments,
    format_token_sequence, format_token_with_comments, format_type_argument_list,
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs, join_body_items,
    relative_token_range_between,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_constructor_body<'source>(
    body: &jolt_java_syntax::ConstructorBody<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let elements = constructor_body_elements(body);
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if ignored_ranges.is_empty() {
        let mut items = Vec::with_capacity(elements.len().saturating_add(2));
        items.extend(format_constructor_body_open_dangling_comments(
            doc,
            body.open_brace(),
        ));
        items.extend(
            elements
                .iter()
                .filter_map(|element| format_constructor_body_element(element, doc)),
        );
        items.extend(format_constructor_body_close_dangling_comments(
            doc,
            body.close_brace(),
        ));
        return (!items.is_empty()).then(|| join_body_items(doc, items));
    }
    let element_ranges = elements
        .iter()
        .map(|element| {
            constructor_body_element_token_range(element, body.text_range().start().get())
        })
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &element_ranges);
    let mut items = Vec::with_capacity(
        elements
            .len()
            .saturating_add(ignored_runs.len())
            .saturating_add(2),
    );
    items.extend(format_constructor_body_open_dangling_comments(
        doc,
        body.open_brace(),
    ));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (element_index, element) in elements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == element_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= element_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(element_index) {
            continue;
        }

        let Some(mut item) = format_constructor_body_element(element, doc) else {
            continue;
        };
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == element_index {
            item = item.without_blank_line_before();
        }
        items.push(item);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
        ignored_index += 1;
    }
    items.extend(format_constructor_body_close_dangling_comments(
        doc,
        body.close_brace(),
    ));

    (!items.is_empty()).then(|| join_body_items(doc, items))
}

fn format_constructor_body_open_dangling_comments<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    open: Option<JavaSyntaxToken<'source>>,
) -> Option<BodyItem<'source>> {
    format_removed_comments(doc, open?.trailing_comments())
        .map(|comments| BodyItem::new(comments, false))
}

fn format_constructor_body_close_dangling_comments<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    close: Option<JavaSyntaxToken<'source>>,
) -> Option<BodyItem<'source>> {
    format_removed_comments(doc, close?.leading_comments())
        .map(|comments| BodyItem::new(comments, false))
}

fn constructor_body_elements<'source>(
    body: &jolt_java_syntax::ConstructorBody<'source>,
) -> Vec<
    jolt_java_syntax::RecoveredSeparatedListEntry<
        'source,
        jolt_java_syntax::ConstructorBodyEntry<'source>,
    >,
> {
    body.entries_with_recovered().collect()
}

fn constructor_body_element_token_range(
    element: &jolt_java_syntax::RecoveredSeparatedListEntry<
        '_,
        jolt_java_syntax::ConstructorBodyEntry<'_>,
    >,
    body_start: usize,
) -> Option<Range<usize>> {
    match element {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
            Some(relative_token_range_between(
                &constructor_body_entry_first_token(entry)?,
                &constructor_body_entry_last_token(entry)?,
                body_start,
            ))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(relative_token_range_between(token, token, body_start))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(
            relative_token_range_between(&error.first_token()?, &error.last_token()?, body_start),
        ),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(
            relative_token_range_between(&node.first_token()?, &node.last_token()?, body_start),
        ),
    }
}

fn format_constructor_body_element<'source>(
    element: &jolt_java_syntax::RecoveredSeparatedListEntry<
        'source,
        jolt_java_syntax::ConstructorBodyEntry<'source>,
    >,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    match element {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => match entry {
            jolt_java_syntax::ConstructorBodyEntry::Invocation(invocation) => Some(BodyItem::new(
                format_constructor_invocation(invocation, doc),
                invocation.starts_after_blank_line(),
            )),
            jolt_java_syntax::ConstructorBodyEntry::BlockStatement(statement) => {
                format_block_statement_item_or_recovered(statement, doc)
            }
        },
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => Some(BodyItem::new(
            format_token_sequence(doc, std::iter::once(*token), LeadingTrivia::Preserve),
            false,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(BodyItem::new(
            format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
            false,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(BodyItem::new(
            format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
            false,
        )),
    }
}

fn format_constructor_invocation<'source>(
    invocation: &ConstructorInvocation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let invocation_first_token = invocation.first_token();
    doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, invocation_first_token.as_ref()),
            format_constructor_invocation_qualifier(invocation, doc),
            invocation
                .type_arguments()
                .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                    &arguments, doc
                ),),
            invocation.target().map_or_else(Doc::nil, |target| {
                format_token_after_construct_leading_comments(
                    doc,
                    &target,
                    invocation_first_token.as_ref(),
                )
            },),
            format_argument_list(invocation.arguments(), doc),
            format_statement_semicolon(invocation.semicolon(), doc),
        ]
    )
}

fn format_constructor_invocation_qualifier<'source>(
    invocation: &ConstructorInvocation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let dot = invocation
        .dot_token()
        .as_ref()
        .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token));
    if let Some(name) = invocation.qualifier_name() {
        return doc_concat!(doc, [format_name(&name, doc), dot]);
    }
    if let Some(expression) = invocation.qualifier_expression() {
        return doc_concat!(doc, [format_expression(&expression, doc), dot]);
    }
    dot
}

fn constructor_body_entry_first_token<'source>(
    entry: &jolt_java_syntax::ConstructorBodyEntry<'source>,
) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
    match entry {
        jolt_java_syntax::ConstructorBodyEntry::Invocation(invocation) => invocation.first_token(),
        jolt_java_syntax::ConstructorBodyEntry::BlockStatement(statement) => {
            statement.first_token()
        }
    }
}

fn constructor_body_entry_last_token<'source>(
    entry: &jolt_java_syntax::ConstructorBodyEntry<'source>,
) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
    match entry {
        jolt_java_syntax::ConstructorBodyEntry::Invocation(invocation) => invocation.last_token(),
        jolt_java_syntax::ConstructorBodyEntry::BlockStatement(statement) => statement.last_token(),
    }
}
