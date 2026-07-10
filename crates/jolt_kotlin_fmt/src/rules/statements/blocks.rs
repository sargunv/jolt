use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{Block, BlockItem, RecoveredSeparatedListEntry};

use crate::helpers::blocks::{
    BodyItem, empty_source_braced_body, join_body_items, source_braced_body,
};
use crate::helpers::comments::{LeadingTrivia, format_dangling_comments, format_token_sequence};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
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
    let mut items = Vec::with_capacity(entries.len());
    items.extend(entries.iter().filter_map(|entry| match entry {
        RecoveredSeparatedListEntry::Entry(item) => Some(*item),
        RecoveredSeparatedListEntry::Token(_)
        | RecoveredSeparatedListEntry::Error(_)
        | RecoveredSeparatedListEntry::Node(_) => None,
    }));

    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_block_contents_with_ignored(doc, block, &entries, &items, &ignored_ranges);
    }

    let docs = block_body_entries(doc, block, &entries, &items);

    (!docs.is_empty()).then(|| join_body_items(doc, docs))
}

fn format_block_contents_from_recovered_entries<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Option<Doc<'source>> {
    let entries = block.items_with_recovered().collect::<Vec<_>>();
    let mut items = Vec::with_capacity(entries.len());
    items.extend(entries.iter().filter_map(|entry| match entry {
        RecoveredSeparatedListEntry::Entry(item) => Some(*item),
        RecoveredSeparatedListEntry::Token(_)
        | RecoveredSeparatedListEntry::Error(_)
        | RecoveredSeparatedListEntry::Node(_) => None,
    }));
    let docs = block_body_entries(doc, block, &entries, &items);

    (!docs.is_empty()).then(|| join_body_items(doc, docs))
}

fn block_body_entries<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    entries: &[RecoveredSeparatedListEntry<'source, BlockItem<'source>>],
    items: &[BlockItem<'source>],
) -> Vec<BodyItem<'source>> {
    let mut body_items = Vec::with_capacity(entries.len());
    let mut item_index = 0;
    let mut entries = entries.iter().peekable();

    while let Some(entry) = entries.next() {
        match entry {
            RecoveredSeparatedListEntry::Entry(item) => {
                body_items.push(block_body_item(doc, block, items, item_index, item));
                item_index += 1;
            }
            recovered_entry => {
                let mut recovered_is_empty = true;
                let recovered = doc.concat_list(|recovered_docs| {
                    push_recovered_block_entry(recovered_docs, recovered_entry);
                    while entries.peek().is_some_and(|entry| {
                        !matches!(entry, RecoveredSeparatedListEntry::Entry(_))
                    }) {
                        let entry = entries.next().expect("peeked block body entry exists");
                        push_recovered_block_entry(recovered_docs, entry);
                    }
                    recovered_is_empty = recovered_docs.is_empty();
                });
                if !recovered_is_empty {
                    body_items.push(BodyItem::new(recovered, false));
                }
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
    items: &[BlockItem<'source>],
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
            && !gap_after_on_marker_contains_comment(doc, block.source_text(), run, &entry_ranges);
    }
    if ignored_runs.is_empty() {
        let docs = block_body_entries(doc, block, entries, items);
        return (!docs.is_empty()).then(|| join_body_items(doc, docs));
    }

    let mut body_items = Vec::with_capacity(entries.len().saturating_add(ignored_runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    let mut item_index = 0;
    for (entry_index, entry) in entries.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == entry_index
        {
            body_items.push(BodyItem::new(
                formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
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

        let mut body_item = recovered_block_body_item(doc, block, items, &mut item_index, entry);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == entry_index {
            body_item = body_item.without_blank_line_before();
        }
        body_items.push(body_item);
    }

    while ignored_index < ignored_runs.len() {
        body_items.push(BodyItem::new(
            formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
            false,
        ));
        ignored_index += 1;
    }

    (!body_items.is_empty()).then(|| join_body_items(doc, body_items))
}

fn gap_after_on_marker_contains_comment(
    _doc: &mut DocBuilder<'_>,
    source: &str,
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun<'_>,
    entry_ranges: &[Option<std::ops::Range<usize>>],
) -> bool {
    let Some(next_start) = entry_ranges
        .get(run.skip_end)
        .and_then(|range| range.as_ref())
        .map(|range| range.start)
    else {
        return false;
    };
    let on_line_end = run.range.interior.start + run.range.raw_text_with_on.len();
    if on_line_end >= next_start {
        return false;
    }

    let gap = &source[on_line_end..next_start];
    gap.contains("//") || gap.contains("/*")
}

fn recovered_block_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    items: &[BlockItem<'source>],
    item_index: &mut usize,
    entry: &RecoveredSeparatedListEntry<'source, BlockItem<'source>>,
) -> BodyItem<'source> {
    match entry {
        RecoveredSeparatedListEntry::Entry(item) => {
            let body_item = block_body_item(doc, block, items, *item_index, item);
            *item_index += 1;
            body_item
        }
        RecoveredSeparatedListEntry::Token(token) => BodyItem::new(
            format_token_sequence(doc, std::iter::once(*token), LeadingTrivia::Preserve),
            false,
        ),
        RecoveredSeparatedListEntry::Error(error) => BodyItem::new(
            format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
            false,
        ),
        RecoveredSeparatedListEntry::Node(node) => BodyItem::new(
            format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
            false,
        ),
    }
}

fn block_body_item<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    items: &[BlockItem<'source>],
    index: usize,
    item: &BlockItem<'source>,
) -> BodyItem<'source> {
    BodyItem::new(
        format_block_item(doc, item),
        block_item_starts_after_blank_line(doc, block, items, index),
    )
}

fn block_item_starts_after_blank_line(
    doc: &mut DocBuilder<'_>,
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
        doc,
        block.source_text(),
        block.text_range().start().get(),
        previous_end,
        current_start,
    )
}

fn gap_has_blank_line(
    _doc: &mut DocBuilder<'_>,
    source: &str,
    block_start: usize,
    start: usize,
    end: usize,
) -> bool {
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
