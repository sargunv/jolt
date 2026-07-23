use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Block, BlockItem, BlockItemList, BlockItemListElement, BlockItemListElementSyntax,
    KotlinCommentKind, KotlinSyntaxListPart, KotlinSyntaxToken, boundary_separator_removal_claim,
};
use jolt_syntax::tokens_have_blank_line_between;

use crate::helpers::blocks::{BodyItem, BodyItemSeparator, join_body_items};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_dangling_comments, format_removed_separator,
    format_token, token_has_comments,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, resolve_required_delimiter, resolve_required_field,
};
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
};

use super::format_block_item;

pub(crate) fn format_block<'source>(
    doc: &mut DocBuilder<'source>,
    block: &Block<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(block.open_brace(), doc);
    let close = resolve_required_delimiter(block.close_brace(), doc);
    let contents = format_block_contents(doc, block, open.source(), close.source());
    format_braced_body(doc, open, close, contents.doc, contents.empty)
}

struct BlockContents<'source> {
    doc: Option<Doc<'source>>,
    empty: bool,
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
            return BlockContents {
                doc: Some(malformed),
                empty: false,
            };
        }
    };
    let parts = collect_block_parts(doc, &items);

    let container =
        formatter_ignore_content_range(items.text_range(), open.copied(), close.copied());
    let ignored_runs =
        doc.formatter_ignore_runs(container, parts.iter().map(block_part_ignore_range));
    let mut body_items = if ignored_runs.is_empty() {
        block_body_parts(doc, &parts)
    } else {
        block_body_parts_with_ignored(doc, &parts, &ignored_runs)
    };
    if let Some(comments) = format_open_dangling_comments(doc, open) {
        body_items.insert(0, BodyItem::new(comments, BodyItemSeparator::Line));
    }
    if let Some(comments) = format_close_dangling_comments(doc, close) {
        body_items.push(BodyItem::new(comments, BodyItemSeparator::Line));
    }
    let empty = body_items.is_empty();
    BlockContents {
        empty,
        doc: (!body_items.is_empty()).then(|| join_body_items(doc, body_items)),
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
    parts: &[BlockPart<'source>],
    ignored_runs: &[FormatterIgnoreRun<'source>],
) -> Vec<BodyItem<'source>> {
    if ignored_runs.is_empty() {
        return block_body_parts(doc, parts);
    }

    let mut body_items = Vec::with_capacity(parts.len().saturating_add(ignored_runs.len()));
    let mut previous = None;
    for_each_formatter_ignore_splice(parts.len(), ignored_runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            body_items.push(BodyItem::new(
                formatter_ignore_run_doc(run, doc),
                BodyItemSeparator::Line,
            ));
        }
        FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        } => {
            let part = &parts[index];
            let mut item = block_body_part(doc, part, previous);
            if clear_blank_line_before {
                item = item.without_blank_line_before();
            }
            body_items.push(item);
            if !matches!(part, BlockPart::Separator { visible: false, .. }) {
                previous = part.last_token();
            }
        }
    });
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
    body: Option<Doc<'source>>,
    empty: bool,
) -> Doc<'source> {
    let has_close = close.source().is_some();
    let open = format_delimiter(
        doc,
        open,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
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
