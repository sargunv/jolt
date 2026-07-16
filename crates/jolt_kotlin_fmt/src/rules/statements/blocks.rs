use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Block, BlockItem, BlockItemList, BlockItemListElement, BlockItemListElementSyntax,
    KotlinCommentKind, KotlinSyntaxInvariantError, KotlinSyntaxListPart, KotlinSyntaxToken,
    boundary_separator_removal_claim,
};
use jolt_syntax::tokens_have_blank_line_between;

use crate::helpers::blocks::{BodyItem, BodyItemSeparator, join_body_items};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_dangling_comments, format_removed_separator,
    format_token, token_has_comments,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, resolve_required_delimiter, resolve_required_field,
};

use super::format_block_item;

pub(crate) fn format_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(block.open_brace(), doc);
    let close = resolve_required_delimiter(block.close_brace(), doc);
    let contents = format_block_contents(doc, block, close.source());
    format_braced_body(doc, open, close, contents.doc, contents.empty)
}

struct BlockContents<'source> {
    doc: Option<Doc<'source>>,
    empty: bool,
}

fn format_block_contents<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> BlockContents<'source> {
    let items = match resolve_required_field(block.items(), doc) {
        KotlinFormatField::Present(items) => items,
        KotlinFormatField::Malformed(malformed) => {
            return BlockContents {
                doc: Some(malformed),
                empty: false,
            };
        }
    };
    let parts = collect_block_parts(doc, &items);

    let ignored_ranges = formatter_ignore_ranges(
        block.source_text(),
        block.text_range().start().get(),
        block.token_iter(),
    );
    let mut body_items = if ignored_ranges.is_empty() {
        block_body_parts(doc, &parts)
    } else {
        block_body_parts_with_ignored(doc, block, &parts, &ignored_ranges)
    };
    if let Some(comments) = format_close_dangling_comments(doc, close) {
        body_items.push(BodyItem::new(comments, BodyItemSeparator::Line));
    }
    let empty = body_items.is_empty();
    BlockContents {
        empty,
        doc: (!body_items.is_empty()).then(|| join_body_items(doc, body_items)),
    }
}

fn format_close_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Option<Doc<'source>> {
    let comments = close?.leading_comments().collect::<Vec<_>>();
    (!comments.is_empty()).then(|| format_dangling_comments(doc, comments))
}

enum BlockPart<'source> {
    Item(BlockItem<'source>),
    Separator {
        token: KotlinSyntaxToken<'source>,
        removed: Doc<'source>,
        visible: bool,
    },
}

impl<'source> BlockPart<'source> {
    fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Item(item) => item.first_token(),
            Self::Separator { token, .. } => Some(*token),
        }
    }

    fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Item(item) => item.last_token(),
            Self::Separator { token, .. } => Some(*token),
        }
    }
}

fn collect_block_parts<'source>(
    doc: &mut DocBuilder<'source>,
    items: &BlockItemList<'source>,
) -> Vec<BlockPart<'source>> {
    let mut parts = Vec::new();
    let mut preceding_item = None;
    for part in items.parts() {
        let part = match part {
            Ok(KotlinSyntaxListPart::Item(element)) => {
                block_element_part(doc, &mut preceding_item, element)
            }
            Ok(
                KotlinSyntaxListPart::Separator(_)
                | KotlinSyntaxListPart::Missing(_)
                | KotlinSyntaxListPart::Malformed(_),
            ) => {
                preceding_item = None;
                doc.block_on_invariant("typed Kotlin block list exposed a non-item part");
                None
            }
            Err(error) => {
                preceding_item = None;
                invariant_block_part(doc, error)
            }
        };
        if let Some(part) = part {
            parts.push(part);
        }
    }
    parts
}

fn block_element_part<'source>(
    doc: &mut DocBuilder<'source>,
    preceding_item: &mut Option<BlockItem<'source>>,
    element: BlockItemListElement<'source>,
) -> Option<BlockPart<'source>> {
    match element.classify() {
        Ok(BlockItemListElementSyntax::Item(item)) => {
            *preceding_item = Some(item);
            Some(BlockPart::Item(item))
        }
        Ok(BlockItemListElementSyntax::Terminator(token)) => {
            Some(separator_part(doc, preceding_item.as_ref(), token))
        }
        Err(error) => {
            *preceding_item = None;
            doc.block_on_invariant(error.to_string());
            None
        }
    }
}

fn separator_part<'source>(
    doc: &mut DocBuilder<'source>,
    preceding_item: Option<&BlockItem<'source>>,
    token: KotlinSyntaxToken<'source>,
) -> BlockPart<'source> {
    let claim = preceding_item.and_then(|owner| boundary_separator_removal_claim(owner, token));
    let removed = format_removed_separator(doc, &token, claim, false);
    BlockPart::Separator {
        token,
        removed,
        visible: token_has_comments(&token),
    }
}

fn invariant_block_part<'source>(
    doc: &mut DocBuilder<'source>,
    error: KotlinSyntaxInvariantError,
) -> Option<BlockPart<'source>> {
    doc.block_on_invariant(error.to_string());
    None
}

fn block_body_parts<'source>(
    doc: &mut DocBuilder<'source>,
    parts: &[BlockPart<'source>],
) -> Vec<BodyItem<'source>> {
    let mut body_items = Vec::with_capacity(parts.len());
    let mut previous = None;
    for part in parts {
        body_items.push(block_body_part(doc, part, previous));
        if !matches!(part, BlockPart::Separator { visible: false, .. }) {
            previous = part.last_token();
        }
    }
    body_items
}

fn block_body_part<'source>(
    doc: &mut DocBuilder<'source>,
    part: &BlockPart<'source>,
    previous: Option<KotlinSyntaxToken<'source>>,
) -> BodyItem<'source> {
    let part_doc = match part {
        BlockPart::Item(item) => format_block_item(doc, item),
        BlockPart::Separator {
            removed, visible, ..
        } => {
            if !visible {
                return BodyItem::invisible(*removed);
            }
            *removed
        }
    };
    BodyItem::new(part_doc, block_item_separator(previous, part.first_token()))
}

fn block_body_parts_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    parts: &[BlockPart<'source>],
    ignored_ranges: &[FormatterIgnoreRange<'source>],
) -> Vec<BodyItem<'source>> {
    let block_start = block.text_range().start().get();
    let part_ranges = parts
        .iter()
        .map(|part| block_part_token_range(part, block_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &part_ranges);
    if ignored_runs.is_empty() {
        return block_body_parts(doc, parts);
    }

    let mut body_items = Vec::with_capacity(parts.len().saturating_add(ignored_runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    let mut previous = None;
    for (part_index, part) in parts.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == part_index
        {
            body_items.push(BodyItem::new(
                formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
                BodyItemSeparator::Line,
            ));
            ignored_index += 1;
        }
        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= part_index {
            skip_index += 1;
        }
        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(part_index) {
            continue;
        }

        let mut item = block_body_part(doc, part, previous);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == part_index {
            item = item.without_blank_line_before();
        }
        body_items.push(item);
        if !matches!(part, BlockPart::Separator { visible: false, .. }) {
            previous = part.last_token();
        }
    }
    while ignored_index < ignored_runs.len() {
        body_items.push(BodyItem::new(
            formatter_ignore_run_doc(&ignored_runs[ignored_index], doc),
            BodyItemSeparator::Line,
        ));
        ignored_index += 1;
    }
    body_items
}

fn block_item_separator<'source>(
    previous: Option<KotlinSyntaxToken<'source>>,
    current: Option<KotlinSyntaxToken<'source>>,
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

fn block_part_token_range(
    part: &BlockPart<'_>,
    block_start: usize,
) -> Option<std::ops::Range<usize>> {
    Some(relative_token_range_between(
        &part.first_token()?,
        &part.last_token()?,
        block_start,
    ))
}

fn format_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    body: Option<Doc<'source>>,
    empty: bool,
) -> Doc<'source> {
    let has_close = close.source().is_some();
    let open = format_delimiter(doc, open, LeadingTrivia::Preserve, TrailingTrivia::Preserve);
    if empty {
        let close = format_delimiter(
            doc,
            close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        return doc.concat([open, close]);
    }
    let contents = if let Some(body) = body {
        let line = doc.hard_line();
        let body = doc.concat([line, body]);
        let body = doc.indent(body);
        if has_close {
            let line = doc.hard_line();
            doc.concat([body, line])
        } else {
            body
        }
    } else {
        doc.hard_line()
    };
    let close = format_delimiter(
        doc,
        close,
        LeadingTrivia::SuppressAlreadyHandled,
        TrailingTrivia::Preserve,
    );
    doc.concat([open, contents, close])
}

fn format_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    delimiter: KotlinFormatDelimiter<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(token) => format_token(doc, &token, leading, trailing),
        KotlinFormatDelimiter::Recovery(recovery) => recovery,
    }
}
