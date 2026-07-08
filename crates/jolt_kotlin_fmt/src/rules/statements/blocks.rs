use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{Block, BlockItem, RecoveredSeparatedListEntry};
use jolt_syntax::source_gap_is_trivia;

use crate::helpers::blocks::{
    BodyItem, empty_source_braced_body, join_body_items, source_braced_body,
};
use crate::helpers::comments::{LeadingTrivia, format_dangling_comments, format_token_sequence};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};

use super::format_block_item;

pub(crate) fn format_block<'source>(block: &Block<'source>) -> Doc<'source> {
    if block.items().next().is_none() {
        if let Some(contents) = format_block_dangling_comments(block)
            .or_else(|| format_block_contents_from_recovered_entries(block))
        {
            return source_braced_body(
                block.open_brace().as_ref(),
                block.close_brace().as_ref(),
                Some(contents),
            );
        }
        return empty_source_braced_body(block.open_brace().as_ref(), block.close_brace().as_ref());
    }

    let body = format_block_contents(block);
    if body.is_none() && block_inner_is_whitespace(block) {
        return empty_source_braced_body(block.open_brace().as_ref(), block.close_brace().as_ref());
    }

    source_braced_body(
        block.open_brace().as_ref(),
        block.close_brace().as_ref(),
        body,
    )
}

fn format_block_contents<'source>(block: &Block<'source>) -> Option<Doc<'source>> {
    let entries = block.items_with_recovered().collect::<Vec<_>>();
    let items = entries
        .iter()
        .filter_map(|entry| match entry {
            RecoveredSeparatedListEntry::Entry(item) => Some(*item),
            RecoveredSeparatedListEntry::Token(_)
            | RecoveredSeparatedListEntry::Error(_)
            | RecoveredSeparatedListEntry::Node(_) => None,
        })
        .collect::<Vec<_>>();

    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_block_contents_with_ignored(block, &entries, &items, &ignored_ranges);
    }

    let docs = block_body_entries(block, &entries, &items);

    (!docs.is_empty()).then(|| join_body_items(docs))
}

fn format_block_contents_from_recovered_entries<'source>(
    block: &Block<'source>,
) -> Option<Doc<'source>> {
    let entries = block.items_with_recovered().collect::<Vec<_>>();
    let items = entries
        .iter()
        .filter_map(|entry| match entry {
            RecoveredSeparatedListEntry::Entry(item) => Some(*item),
            RecoveredSeparatedListEntry::Token(_)
            | RecoveredSeparatedListEntry::Error(_)
            | RecoveredSeparatedListEntry::Node(_) => None,
        })
        .collect::<Vec<_>>();
    let docs = block_body_entries(block, &entries, &items);

    (!docs.is_empty()).then(|| join_body_items(docs))
}

fn block_body_entries<'source>(
    block: &Block<'source>,
    entries: &[RecoveredSeparatedListEntry<'source, BlockItem<'source>>],
    items: &[BlockItem<'source>],
) -> Vec<BodyItem<'source>> {
    let mut docs = Vec::new();
    let mut recovered_docs = Vec::new();
    let mut item_index = 0;

    for entry in entries {
        match entry {
            RecoveredSeparatedListEntry::Entry(item) => {
                push_recovered_block_body_item(&mut docs, &mut recovered_docs);
                docs.push(block_body_item(block, items, item_index, item));
                item_index += 1;
            }
            RecoveredSeparatedListEntry::Token(token) => recovered_docs.push(
                format_token_sequence(std::iter::once(*token), LeadingTrivia::Preserve),
            ),
            RecoveredSeparatedListEntry::Error(error) => recovered_docs.push(
                format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
            ),
            RecoveredSeparatedListEntry::Node(node) => recovered_docs.push(format_token_sequence(
                node.token_iter(),
                LeadingTrivia::Preserve,
            )),
        }
    }

    push_recovered_block_body_item(&mut docs, &mut recovered_docs);
    docs
}

fn push_recovered_block_body_item<'source>(
    docs: &mut Vec<BodyItem<'source>>,
    recovered_docs: &mut Vec<Doc<'source>>,
) {
    if recovered_docs.is_empty() {
        return;
    }

    docs.push(BodyItem::new(concat(std::mem::take(recovered_docs)), false));
}

fn format_block_dangling_comments<'source>(block: &Block<'source>) -> Option<Doc<'source>> {
    let comments = block
        .open_brace()
        .into_iter()
        .flat_map(|token| token.trailing_comments())
        .chain(
            block
                .close_brace()
                .into_iter()
                .flat_map(|token| token.leading_comments()),
        )
        .collect::<Vec<_>>();

    (!comments.is_empty()).then(|| format_dangling_comments(comments))
}

fn format_block_contents_with_ignored<'source>(
    block: &Block<'source>,
    entries: &[RecoveredSeparatedListEntry<'source, BlockItem<'source>>],
    items: &[BlockItem<'source>],
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Option<Doc<'source>> {
    let block_start = block.text_range().start().get();
    let entry_ranges = entries
        .iter()
        .map(|entry| recovered_block_item_token_range(entry, block_start))
        .collect::<Vec<_>>();
    let mut ignored_runs = formatter_ignore_runs(ignored_ranges, &entry_ranges);
    for run in &mut ignored_runs {
        run.include_on_marker = true;
    }
    if ignored_runs.is_empty() {
        let docs = block_body_entries(block, entries, items);
        return (!docs.is_empty()).then(|| join_body_items(docs));
    }

    let mut docs = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    let mut item_index = 0;
    for (entry_index, entry) in entries.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == entry_index
        {
            docs.push(BodyItem::new(
                formatter_ignore_run_doc(&ignored_runs[ignored_index]),
                false,
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= entry_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(entry_index) {
            if matches!(entry, RecoveredSeparatedListEntry::Entry(_)) {
                item_index += 1;
            }
            continue;
        }

        let mut body_item = recovered_block_body_item(block, items, &mut item_index, entry);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == entry_index {
            body_item = body_item.without_blank_line_before();
        }
        docs.push(body_item);
    }

    while ignored_index < ignored_runs.len() {
        docs.push(BodyItem::new(
            formatter_ignore_run_doc(&ignored_runs[ignored_index]),
            false,
        ));
        ignored_index += 1;
    }

    (!docs.is_empty()).then(|| join_body_items(docs))
}

fn recovered_block_body_item<'source>(
    block: &Block<'source>,
    items: &[BlockItem<'source>],
    item_index: &mut usize,
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) -> BodyItem<'source> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => {
            let body_item = block_body_item(block, items, *item_index, item);
            *item_index += 1;
            body_item
        }
        RecoveredSeparatedListEntry::Token(token) => BodyItem::new(
            format_token_sequence(std::iter::once(*token), LeadingTrivia::Preserve),
            false,
        ),
        RecoveredSeparatedListEntry::Error(error) => BodyItem::new(
            format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
            false,
        ),
        RecoveredSeparatedListEntry::Node(node) => BodyItem::new(
            format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
            false,
        ),
    }
}

fn block_body_item<'source>(
    block: &Block<'source>,
    items: &[BlockItem<'source>],
    index: usize,
    item: &BlockItem<'source>,
) -> BodyItem<'source> {
    BodyItem::new(
        format_block_item(item),
        block_item_starts_after_blank_line(block, items, index),
    )
}

fn block_item_starts_after_blank_line(
    block: &Block<'_>,
    items: &[BlockItem<'_>],
    index: usize,
) -> bool {
    if index == 0 {
        return false;
    }
    let previous_end = items[index - 1].text_range().end().get();
    let current_start = items[index].text_range().start().get();
    gap_has_blank_line(
        block.source_text(),
        block.text_range().start().get(),
        previous_end,
        current_start,
    )
}

fn gap_has_blank_line(source: &str, block_start: usize, start: usize, end: usize) -> bool {
    let gap = &source[start - block_start..end - block_start];
    let mut line_breaks = 0;
    for byte in gap.bytes() {
        if byte == b'\n' {
            line_breaks += 1;
            if line_breaks >= 2 {
                return true;
            }
        }
    }
    false
}

fn block_inner_is_whitespace(block: &Block<'_>) -> bool {
    let Some(open) = block.open_brace() else {
        return false;
    };
    let Some(close) = block.close_brace() else {
        return false;
    };
    source_gap_is_trivia(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
        open.token_text_range().end().get(),
        close.token_text_range().start().get(),
    )
}

fn block_item_token_range(
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
    entry: &RecoveredSeparatedListEntry<'_, BlockItem<'_>>,
    block_start: usize,
) -> Option<std::ops::Range<usize>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => block_item_token_range(item, block_start),
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
