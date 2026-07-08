use jolt_fmt_ir::Doc;
use jolt_kotlin_syntax::{Block, BlockItem, KotlinSyntaxToken};

use crate::helpers::blocks::{
    BodyItem, empty_source_braced_body, join_body_items, source_braced_body,
};
use crate::helpers::comments::{LeadingTrivia, format_dangling_comments, format_token_sequence};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::source::source_gap_is_trivia;

use super::format_block_item;

pub(crate) fn format_block<'source>(block: &Block<'source>) -> Doc<'source> {
    if block.items().next().is_none() {
        if let Some(contents) = format_block_dangling_comments(block)
            .or_else(|| format_block_contents_with_recovered_tokens(block, &[]))
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
    let items = block.items().collect::<Vec<_>>();
    if !items_cover_block_contents(block, &items) {
        return format_block_contents_with_recovered_tokens(block, &items);
    }

    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    if !ignored_ranges.is_empty() {
        return format_block_contents_with_ignored(block, &items, &ignored_ranges);
    }

    let docs = block_body_items(block, &items);

    (!docs.is_empty()).then(|| join_body_items(docs))
}

fn format_block_contents_with_recovered_tokens<'source>(
    block: &Block<'source>,
    items: &[BlockItem<'source>],
) -> Option<Doc<'source>> {
    let body_start = block.open_brace().map_or_else(
        || block.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let body_end = block.close_brace().map_or_else(
        || block.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    let source = block.source_text();
    let tokens = block.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut docs = Vec::new();
    let mut cursor = body_start;

    for (index, item) in items.iter().enumerate() {
        push_uncovered_block_tokens(
            &mut docs,
            block,
            &tokens,
            &mut token_cursor,
            cursor,
            item.text_range().start().get(),
        );

        let starts_after_blank_line = if docs.is_empty() {
            false
        } else if source_gap_is_trivia(
            source,
            block.text_range().start().get(),
            block.token_iter(),
            cursor,
            item.text_range().start().get(),
        ) {
            block_item_starts_after_blank_line(block, items, index)
        } else {
            false
        };
        docs.push(BodyItem::new(
            format_block_item(item),
            starts_after_blank_line,
        ));
        cursor = item.text_range().end().get();
    }

    push_uncovered_block_tokens(
        &mut docs,
        block,
        &tokens,
        &mut token_cursor,
        cursor,
        body_end,
    );

    (!docs.is_empty()).then(|| join_body_items(docs))
}

fn push_uncovered_block_tokens<'source>(
    docs: &mut Vec<BodyItem<'source>>,
    block: &Block<'source>,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) {
    let block_start = block.text_range().start().get();
    let source = block.source_text();
    if source_gap_is_trivia(source, block_start, tokens.iter().copied(), start, end) {
        return;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return;
    }

    docs.push(BodyItem::new(
        format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        false,
    ));
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
    items: &[BlockItem<'source>],
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Option<Doc<'source>> {
    let block_start = block.text_range().start().get();
    let item_ranges = items
        .iter()
        .map(|item| block_item_token_range(item, block_start))
        .collect::<Vec<_>>();
    let mut ignored_runs = formatter_ignore_runs(ignored_ranges, &item_ranges);
    for run in &mut ignored_runs {
        run.include_on_marker = true;
    }
    if ignored_runs.is_empty() {
        let docs = block_body_items(block, items);
        return (!docs.is_empty()).then(|| join_body_items(docs));
    }

    let mut docs = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (item_index, item) in items.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            docs.push(BodyItem::new(
                formatter_ignore_run_doc(&ignored_runs[ignored_index]),
                false,
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= item_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(item_index) {
            continue;
        }

        let mut body_item = block_body_item(block, items, item_index, item);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == item_index {
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

fn block_body_items<'source>(
    block: &Block<'source>,
    items: &[BlockItem<'source>],
) -> Vec<BodyItem<'source>> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| block_body_item(block, items, index, item))
        .collect()
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

fn items_cover_block_contents(block: &Block<'_>, items: &[BlockItem<'_>]) -> bool {
    let Some(open) = block.open_brace() else {
        return false;
    };
    let Some(close) = block.close_brace() else {
        return false;
    };
    let block_start = block.text_range().start().get();
    let source = block.source_text();
    let mut covered_until = open.token_text_range().end().get();

    for item in items {
        let item_start = item.text_range().start().get();
        if !source_gap_is_trivia(
            source,
            block_start,
            block.token_iter(),
            covered_until,
            item_start,
        ) {
            return false;
        }
        covered_until = item.text_range().end().get();
    }

    source_gap_is_trivia(
        source,
        block_start,
        block.token_iter(),
        covered_until,
        close.token_text_range().start().get(),
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
