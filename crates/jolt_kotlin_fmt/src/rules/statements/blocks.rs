use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Block, BlockItem, BlockItemList, BlockItemListElement, BlockItemListElementSyntax,
    KotlinSyntaxListPart, KotlinSyntaxToken, boundary_separator_removal_claim,
};

use crate::helpers::blocks::{BodyItem, BodyItemSeparator, join_body_items};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_dangling_comments, format_removed_separator,
    token_has_comments,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, format_delimiter, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
};

use super::format_block_item_at_body_boundary;

pub(crate) fn format_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(block.open_brace(), doc);
    let close = resolve_required_delimiter(block.close_brace(), doc);
    let contents = format_block_contents(doc, block, open.source(), close.source());
    format_braced_body(doc, open, close, contents)
}

#[derive(Clone, Copy)]
enum BlockContents<'source> {
    Empty,
    Body(Doc<'source>),
}

fn format_block_contents<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> BlockContents<'source> {
    let items = match resolve_required_field(block.items(), doc) {
        KotlinFormatField::Present(items) => items,
        KotlinFormatField::Malformed(malformed) => {
            return BlockContents::Body(malformed);
        }
    };
    let parts = collect_block_parts(doc, &items);

    let container =
        formatter_ignore_content_range(items.text_range(), open.copied(), close.copied());
    let ignored_runs =
        doc.formatter_ignore_runs(container, parts.iter().map(block_part_ignore_range));
    let mut body_items = if ignored_runs.is_empty() {
        block_body_parts(doc, &parts, close)
    } else {
        block_body_parts_with_ignored(doc, &parts, &ignored_runs, close)
    };
    if let Some(comments) = format_open_dangling_comments(doc, open) {
        body_items.insert(0, BodyItem::new(comments, BodyItemSeparator::Line));
    }
    if let Some(comments) = format_close_dangling_comments(doc, close) {
        // The gap that opens the close brace's leading trivia belongs to that
        // token, so the separator in front of this run reads it from there.
        let separator = BodyItemSeparator::between(
            close.is_some_and(KotlinSyntaxToken::has_leading_blank_line),
        );
        body_items.push(BodyItem::new(comments, separator));
    }
    if body_items.is_empty() {
        BlockContents::Empty
    } else {
        BlockContents::Body(join_body_items(doc, body_items))
    }
}

fn format_open_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
) -> Option<Doc<'source>> {
    let comments = open?.trailing_comments().collect::<Vec<_>>();
    (!comments.is_empty()).then(|| format_dangling_comments(doc, comments))
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
            KotlinSyntaxListPart::Item(element) => {
                block_element_part(doc, &mut preceding_item, element)
            }
            KotlinSyntaxListPart::Separator(_)
            | KotlinSyntaxListPart::Missing(_)
            | KotlinSyntaxListPart::Malformed(_) => {
                preceding_item = None;
                doc.block_on_invariant("typed Kotlin block list exposed a non-item part");
                None
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

fn block_body_parts<'source>(
    doc: &mut DocBuilder<'source>,
    parts: &[BlockPart<'source>],
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Vec<BodyItem<'source>> {
    let mut body_items = Vec::with_capacity(parts.len());
    let mut previous = None;
    let successors = block_part_successors(parts, close);
    for (part, successor) in parts.iter().zip(successors) {
        body_items.push(block_body_part(doc, part, previous, successor.as_ref()));
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
    successor: Option<&KotlinSyntaxToken<'source>>,
) -> BodyItem<'source> {
    let part_doc = match part {
        BlockPart::Item(item) => format_block_item_at_body_boundary(doc, item, successor).doc,
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
    parts: &[BlockPart<'source>],
    ignored_runs: &[FormatterIgnoreRun<'source>],
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Vec<BodyItem<'source>> {
    if ignored_runs.is_empty() {
        return block_body_parts(doc, parts, close);
    }

    let mut body_items = Vec::with_capacity(parts.len().saturating_add(ignored_runs.len()));
    let mut previous = None;
    let successors = block_part_successors(parts, close);
    for_each_formatter_ignore_splice(parts.len(), ignored_runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            body_items.push(BodyItem::new(
                formatter_ignore_run_doc(run, doc),
                BodyItemSeparator::Line,
            ));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            let part = &parts[index];
            body_items.push(block_body_part(
                doc,
                part,
                previous,
                successors[index].as_ref(),
            ));
            if !matches!(part, BlockPart::Separator { visible: false, .. }) {
                previous = part.last_token();
            }
        }
        FormatterIgnoreSplice::End { .. } => {}
    });
    body_items
}

fn block_part_successors<'source>(
    parts: &[BlockPart<'source>],
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Vec<Option<KotlinSyntaxToken<'source>>> {
    let mut successor = close.copied();
    let mut successors = vec![None; parts.len()];
    for (index, part) in parts.iter().enumerate().rev() {
        successors[index] = successor;
        if let Some(first) = part.first_token() {
            successor = Some(first);
        }
    }
    successors
}

fn block_item_separator<'source>(
    previous: Option<KotlinSyntaxToken<'source>>,
    current: Option<KotlinSyntaxToken<'source>>,
) -> BodyItemSeparator {
    let Some(current) = previous.and(current) else {
        return BodyItemSeparator::Line;
    };
    BodyItemSeparator::between(current.has_leading_blank_line())
}

fn block_part_ignore_range(part: &BlockPart<'_>) -> Option<FormatterIgnoreItemRange> {
    Some(FormatterIgnoreItemRange::between(
        &part.first_token()?,
        &part.last_token()?,
    ))
}

fn format_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    contents: BlockContents<'source>,
) -> Doc<'source> {
    let has_close = close.is_visible();
    let open = format_delimiter(
        doc,
        open,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let contents = match contents {
        BlockContents::Empty => {
            let close = format_delimiter(
                doc,
                close,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            return doc.concat([open, close]);
        }
        BlockContents::Body(body) => {
            let line = doc.hard_line_boundary();
            let body = doc.concat([line, body]);
            let body = doc.indent(body);
            if has_close {
                let line = doc.hard_line_boundary();
                doc.concat([body, line])
            } else {
                body
            }
        }
    };
    let close = format_delimiter(
        doc,
        close,
        LeadingTrivia::SuppressAlreadyHandled,
        TrailingTrivia::Preserve,
    );
    doc.concat([open, contents, close])
}
