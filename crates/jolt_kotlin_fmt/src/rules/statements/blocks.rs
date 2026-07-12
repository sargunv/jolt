use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{Block, BlockItem, KotlinCommentKind, RecoveredSeparatedListEntry};
use jolt_syntax::tokens_have_blank_line_between;

use crate::helpers::blocks::{
    BodyItem, BodyItemSeparator, empty_source_braced_body, join_body_items, source_braced_body,
};
use crate::helpers::comments::{LeadingTrivia, format_dangling_comments, format_token_sequence};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_on_marker, relative_token_range_between,
};

use super::format_block_item;

pub(crate) fn format_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Doc<'source> {
    if block.items().next().is_none() {
        if let Some(contents) = format_block_dangling_comments(doc, block)
            .or_else(|| format_block_contents_from_recovered_entries(doc, block))
        {
            return source_braced_body(
                doc,
                block.open_brace().as_ref(),
                block.close_brace().as_ref(),
                Some(contents),
            );
        }
        return empty_source_braced_body(
            doc,
            block.open_brace().as_ref(),
            block.close_brace().as_ref(),
        );
    }

    let body = format_block_contents(doc, block);
    if body.is_none() && block.inner_is_whitespace() {
        return empty_source_braced_body(
            doc,
            block.open_brace().as_ref(),
            block.close_brace().as_ref(),
        );
    }

    source_braced_body(
        doc,
        block.open_brace().as_ref(),
        block.close_brace().as_ref(),
        body,
    )
}

fn format_block_contents<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Option<Doc<'source>> {
    let entries = block.items_with_recovered().collect::<Vec<_>>();

    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_block_contents_with_ignored(doc, block, &entries, &ignored_ranges);
    }

    let docs = block_body_entries(doc, &entries);

    (!docs.is_empty()).then(|| join_body_items(doc, docs))
}

fn format_block_contents_from_recovered_entries<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Option<Doc<'source>> {
    let entries = block.items_with_recovered().collect::<Vec<_>>();
    let docs = block_body_entries(doc, &entries);

    (!docs.is_empty()).then(|| join_body_items(doc, docs))
}

fn block_body_entries<'source>(
    doc: &mut DocBuilder<'source>,
    entries: &[RecoveredSeparatedListEntry<'source, BlockItem<'source>>],
) -> Vec<BodyItem<'source>> {
    let mut body_items = Vec::with_capacity(entries.len());
    let mut entries = entries.iter().peekable();
    let mut previous_token = None;

    while let Some(entry) = entries.next() {
        match entry {
            RecoveredSeparatedListEntry::Entry(item) => {
                body_items.push(block_body_item(doc, item, previous_token));
                previous_token = item.last_token();
            }
            recovered_entry => {
                let mut recovered_is_empty = true;
                let mut recovered_last_token = recovered_entry_last_token(recovered_entry);
                let separator = block_item_separator(
                    previous_token,
                    recovered_block_entry_first_token(recovered_entry),
                );
                let recovered = doc.concat_list(|recovered_docs| {
                    push_recovered_block_entry(recovered_docs, recovered_entry);
                    while entries.peek().is_some_and(|entry| {
                        !matches!(entry, RecoveredSeparatedListEntry::Entry(_))
                    }) {
                        let entry = entries.next().expect("peeked block body entry exists");
                        push_recovered_block_entry(recovered_docs, entry);
                        recovered_last_token = recovered_entry_last_token(entry);
                    }
                    recovered_is_empty = recovered_docs.is_empty();
                });
                if !recovered_is_empty {
                    body_items.push(BodyItem::new(recovered, separator));
                }
                previous_token = recovered_last_token;
            }
        }
    }

    body_items
}

fn push_recovered_block_entry<'source>(
    recovered_docs: &mut ConcatBuilder<'_, 'source>,
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) {
    let recovered = match entry {
        RecoveredSeparatedListEntry::Entry(_) => return,
        RecoveredSeparatedListEntry::Token(token) => format_token_sequence(
            recovered_docs,
            std::iter::once(*token),
            LeadingTrivia::Preserve,
        ),
        RecoveredSeparatedListEntry::Error(error) => {
            format_token_sequence(recovered_docs, error.token_iter(), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Node(node) => {
            format_token_sequence(recovered_docs, node.token_iter(), LeadingTrivia::Preserve)
        }
    };
    recovered_docs.push(recovered);
}

fn format_block_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Option<Doc<'source>> {
    let mut comments = block
        .open_brace()
        .into_iter()
        .flat_map(|token| token.trailing_comments())
        .chain(
            block
                .close_brace()
                .into_iter()
                .flat_map(|token| token.leading_comments()),
        )
        .peekable();

    comments
        .peek()
        .is_some()
        .then(|| format_dangling_comments(doc, comments))
}

fn format_block_contents_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    entries: &[RecoveredSeparatedListEntry<'source, BlockItem<'source>>],
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Option<Doc<'source>> {
    let block_start = block.text_range().start().get();
    let entry_ranges = entries
        .iter()
        .map(|entry| recovered_block_item_token_range(doc, entry, block_start))
        .collect::<Vec<_>>();
    let mut ignored_runs = formatter_ignore_runs(ignored_ranges, &entry_ranges);
    for run in &mut ignored_runs {
        run.include_on_marker = entries
            .get(run.skip_end)
            .is_some_and(|entry| matches!(entry, RecoveredSeparatedListEntry::Entry(_)))
            && entries
                .get(run.skip_end)
                .and_then(recovered_block_entry_first_token)
                .is_none_or(|token| !has_comment_after_formatter_on(&token));
    }
    if ignored_runs.is_empty() {
        let docs = block_body_entries(doc, entries);
        return (!docs.is_empty()).then(|| join_body_items(doc, docs));
    }

    let mut body_items = Vec::with_capacity(entries.len().saturating_add(ignored_runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    let mut previous_token = None;
    for (entry_index, entry) in entries.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == entry_index
        {
            body_items.push(BodyItem::new(
                formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
                BodyItemSeparator::Line,
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= entry_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(entry_index) {
            continue;
        }

        let mut body_item = recovered_block_body_item(doc, entry, previous_token);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == entry_index {
            body_item = body_item.without_blank_line_before();
        }
        body_items.push(body_item);
        previous_token = recovered_entry_last_token(entry);
    }

    while ignored_index < ignored_runs.len() {
        body_items.push(BodyItem::new(
            formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
            BodyItemSeparator::Line,
        ));
        ignored_index += 1;
    }

    (!body_items.is_empty()).then(|| join_body_items(doc, body_items))
}

fn recovered_block_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
    previous_token: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> BodyItem<'source> {
    if let RecoveredSeparatedListEntry::Entry(item) = entry {
        return block_body_item(doc, item, previous_token);
    }

    let entry_doc = match entry {
        RecoveredSeparatedListEntry::Entry(_) => unreachable!("handled above"),
        RecoveredSeparatedListEntry::Token(token) => {
            format_token_sequence(doc, std::iter::once(*token), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Error(error) => {
            format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Node(node) => {
            format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve)
        }
    };
    let separator = block_item_separator(previous_token, recovered_block_entry_first_token(entry));
    BodyItem::new(entry_doc, separator)
}

fn block_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &BlockItem<'source>,
    previous_token: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> BodyItem<'source> {
    BodyItem::new(
        format_block_item(doc, item),
        block_item_separator(previous_token, item.first_token()),
    )
}

fn block_item_separator<'source>(
    previous: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
    current: Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> BodyItemSeparator {
    let Some((previous, current)) = previous.zip(current) else {
        return BodyItemSeparator::Line;
    };
    let has_blank_line = tokens_have_blank_line_between(&previous, &current);
    let previous_forces_line = previous
        .trailing_comments()
        .any(|comment| comment.kind() == KotlinCommentKind::Line);
    match (has_blank_line, previous_forces_line) {
        (false, true) => BodyItemSeparator::None,
        (true, true) | (false, false) => BodyItemSeparator::Line,
        (true, false) => BodyItemSeparator::EmptyLine,
    }
}

fn recovered_block_entry_first_token<'source>(
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) -> Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => item.first_token(),
        RecoveredSeparatedListEntry::Token(token) => Some(*token),
        RecoveredSeparatedListEntry::Error(error) => error.first_token(),
        RecoveredSeparatedListEntry::Node(node) => node.first_token(),
    }
}

fn recovered_entry_last_token<'source>(
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) -> Option<jolt_kotlin_syntax::KotlinSyntaxToken<'source>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => item.last_token(),
        RecoveredSeparatedListEntry::Token(token) => Some(*token),
        RecoveredSeparatedListEntry::Error(error) => error.last_token(),
        RecoveredSeparatedListEntry::Node(node) => node.last_token(),
    }
}

fn has_comment_after_formatter_on(token: &jolt_kotlin_syntax::KotlinSyntaxToken<'_>) -> bool {
    let mut saw_on_marker = false;
    for comment in token.leading_comments() {
        if saw_on_marker {
            return true;
        }
        saw_on_marker = is_formatter_on_marker(comment.text());
    }
    false
}

fn block_item_token_range(
    _doc: &mut DocBuilder<'_>,
    item: &BlockItem<'_>,
    block_start: usize,
) -> Option<std::ops::Range<usize>> {
    Some(relative_token_range_between(
        &item.first_token()?,
        &item.last_token()?,
        block_start,
    ))
}

fn recovered_block_item_token_range(
    doc: &mut DocBuilder<'_>,
    entry: &RecoveredSeparatedListEntry<'_, BlockItem<'_>>,
    block_start: usize,
) -> Option<std::ops::Range<usize>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => block_item_token_range(doc, item, block_start),
        RecoveredSeparatedListEntry::Token(token) => {
            let range = token.token_text_range();
            Some(range.start().get() - block_start..range.end().get() - block_start)
        }
        RecoveredSeparatedListEntry::Error(error) => Some(relative_token_range_between(
            &error.first_token()?,
            &error.last_token()?,
            block_start,
        )),
        RecoveredSeparatedListEntry::Node(node) => Some(relative_token_range_between(
            &node.first_token()?,
            &node.last_token()?,
            block_start,
        )),
    }
}
