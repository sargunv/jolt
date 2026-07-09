use super::{
    Block, BlockItem, BlockStatement, BodyItem, Doc, FormatterIgnoreRange, JavaSyntaxToken, Range,
    TrailingTrivia, comments_from_tokens, format_dangling_comments,
    format_local_variable_declaration, format_removed_comments, format_statement,
    format_statement_semicolon, format_type_declaration, formatter_ignore_ranges,
    formatter_ignore_run_doc, formatter_ignore_runs, join_body_items, relative_token_range_between,
};
use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, format_token_after_relocated_leading_comments,
    format_token_sequence, format_token_with_inline_leading_comments,
};
use jolt_fmt_ir::DocBuilder;

pub(crate) fn format_block<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = match block.open_brace().as_ref() {
        Some(open) => format_block_open_brace(open, doc),
        None => Doc::nil(),
    };
    let body = match format_block_statements_body(block, doc) {
        Some(body) => {
            let body = doc_concat!(doc, [doc.hard_line(), body]);
            doc_concat!(doc, [doc_indent!(doc, body), doc.hard_line()])
        }
        None => doc.hard_line(),
    };
    let close = match block.close_brace().as_ref() {
        Some(close) => {
            format_token_after_relocated_leading_comments(doc, close, TrailingTrivia::Preserve)
        }
        None => Doc::nil(),
    };

    doc_concat!(doc, [open, body, close])
}

fn format_block_open_brace<'source>(
    open: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        doc,
        open,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

fn format_block_statements_body<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let block_start = block.text_range().start().get();
    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    let entries = block.block_statements_with_recovered().collect::<Vec<_>>();
    let mut items = Vec::with_capacity(entries.len().saturating_add(2));
    items.extend(format_block_open_dangling_comments(block, doc));
    if ignored_ranges.is_empty() {
        items.extend(format_block_statement_items_with_recovered(entries, doc));
    } else {
        items.extend(format_block_statement_items_with_ignored(
            entries,
            block_start,
            &ignored_ranges,
            doc,
        ));
    }
    items.extend(format_block_close_dangling_comments(block, doc));
    (!items.is_empty()).then(|| join_body_items(doc, items))
}

fn format_block_statement_items_with_recovered<'source, 'fmt, Statements>(
    statements: Statements,
    doc: &'fmt mut DocBuilder<'source>,
) -> impl Iterator<Item = BodyItem<'source>> + use<'source, 'fmt, Statements>
where
    Statements: IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, BlockStatement<'source>>,
    >,
{
    statements.into_iter().filter_map(move |entry| match entry {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(statement) => {
            format_block_statement_item_or_recovered(&statement, doc)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => Some(BodyItem::new(
            format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve),
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
    })
}

fn format_block_statement_items_with_ignored<'source>(
    entries: Vec<jolt_java_syntax::RecoveredSeparatedListEntry<'source, BlockStatement<'source>>>,
    block_start: usize,
    ignored_ranges: &[FormatterIgnoreRange<'source>],
    doc: &mut DocBuilder<'source>,
) -> Vec<BodyItem<'source>> {
    let entry_ranges = entries
        .iter()
        .map(|entry| recovered_block_statement_entry_token_range(entry, block_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &entry_ranges);

    let mut items = Vec::with_capacity(entries.len().saturating_add(ignored_runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (entry_index, entry) in entries.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == entry_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= entry_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(entry_index) {
            continue;
        }

        let Some(mut item) = format_recovered_block_statement_entry(entry, doc) else {
            continue;
        };
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == entry_index {
            item = item.without_blank_line_before();
        }
        items.push(item);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
        ignored_index += 1;
    }

    items
}

fn format_recovered_block_statement_entry<'source>(
    entry: jolt_java_syntax::RecoveredSeparatedListEntry<'source, BlockStatement<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    match entry {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(statement) => {
            format_block_statement_item_or_recovered(&statement, doc)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => Some(BodyItem::new(
            format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve),
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

fn format_block_open_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let comments = block.open_brace()?.trailing_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn format_block_close_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let comments = block.close_brace()?.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn recovered_block_statement_entry_token_range(
    entry: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, BlockStatement<'_>>,
    block_start: usize,
) -> Option<Range<usize>> {
    match entry {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(statement) => {
            block_statement_token_range(statement, block_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            let range = token.token_text_range();
            Some(range.start().get() - block_start..range.end().get() - block_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(
            relative_token_range_between(&error.first_token()?, &error.last_token()?, block_start),
        ),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(
            relative_token_range_between(&node.first_token()?, &node.last_token()?, block_start),
        ),
    }
}

pub(crate) fn format_block_statement_item<'source>(
    statement: &BlockStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let starts_after_blank_line = statement.starts_after_blank_line();
    let doc = format_block_item_doc(statement.item()?, statement.semicolon(), doc)?;
    Some(BodyItem::new(doc, starts_after_blank_line))
}

pub(crate) fn format_block_statement_item_or_recovered<'source>(
    statement: &BlockStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    if statement.item().is_some() {
        return format_block_statement_item(statement, doc);
    }

    Some(BodyItem::new(
        format_token_sequence(doc, statement.token_iter(), LeadingTrivia::Preserve),
        false,
    ))
}

fn format_block_item_doc<'source>(
    item: BlockItem<'source>,
    semicolon: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let doc = match item {
        BlockItem::EmptyStatement(statement) => format_removed_empty_statement(&statement, doc),
        BlockItem::LocalVariableDeclaration(declaration) => Some(doc_concat!(
            doc,
            [
                format_local_variable_declaration(&declaration, doc),
                format_statement_semicolon(semicolon, doc),
            ]
        )),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => declaration
            .declaration()
            .map(|declaration| format_type_declaration(&declaration, doc)),
        BlockItem::Block(block) => Some(format_block(&block, doc)),
        BlockItem::LabeledStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::ExpressionStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::IfStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::AssertStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::SwitchStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::WhileStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::DoStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::ForStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::BreakStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::YieldStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::ContinueStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::ReturnStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::ThrowStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::SynchronizedStatement(statement) => {
            Some(format_statement(&statement.into(), doc))
        }
        BlockItem::TryStatement(statement) => Some(format_statement(&statement.into(), doc)),
        BlockItem::TryWithResourcesStatement(statement) => {
            Some(format_statement(&statement.into(), doc))
        }
    }?;
    Some(doc)
}

fn format_removed_empty_statement<'source>(
    statement: &jolt_java_syntax::EmptyStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    format_removed_comments(doc, comments_from_tokens(statement.token_iter()))
}

fn block_statement_token_range(
    statement: &BlockStatement<'_>,
    block_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &statement.first_token()?,
        &statement.last_token()?,
        block_start,
    ))
}
