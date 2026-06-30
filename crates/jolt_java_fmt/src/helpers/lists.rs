use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{
    Doc, concat, fill, fill_entry, group, hard_line, hard_line_without_break_parent, indent,
    indent_by, join, line, soft_line, text,
};

use crate::comments::{
    format_own_line_comment_doc, reject_unhandled_comments_in_range, take_dangling_comment_docs,
    take_inline_leading_block_comment_docs, take_inline_trailing_block_comment_docs,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::helpers::separated;
use crate::policy::JavaFormatPolicy;

pub(crate) struct ListItem {
    range: TextRange,
    source_width: usize,
    shape: ListItemShape,
    format: Box<dyn for<'ctx> FnOnce(&mut JavaFormatContext<'ctx>) -> FormatResult<Doc>>,
}

impl ListItem {
    pub(crate) fn new(
        range: TextRange,
        format: impl for<'ctx> FnOnce(&mut JavaFormatContext<'ctx>) -> FormatResult<Doc> + 'static,
    ) -> Self {
        Self {
            range,
            source_width: text_range_width(range),
            shape: ListItemShape::Unknown,
            format: Box::new(format),
        }
    }

    pub(crate) fn with_shape(mut self, shape: ListItemShape) -> Self {
        self.shape = shape;
        self
    }

    pub(crate) fn doc(doc: Doc, range: TextRange) -> Self {
        Self::new(range, |_| Ok(doc))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ListItemShape {
    Simple,
    Call,
    Complex,
    Unknown,
}

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    separated::comma_list(items)
}

pub(crate) fn argument_list_docs(
    items: impl IntoIterator<Item = Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    separated::delimited_comma_list("(", ")", policy.continuation_indent_levels(), items)
}

pub(crate) fn formal_parameter_list_docs(
    items: impl IntoIterator<Item = Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    separated::delimited_comma_list_one_per_line(
        "(",
        ")",
        policy.continuation_indent_levels(),
        items,
    )
}

pub(crate) fn empty_argument_list(policy: JavaFormatPolicy) -> Doc {
    separated::delimited_comma_list(
        "(",
        ")",
        policy.continuation_indent_levels(),
        std::iter::empty(),
    )
}

pub(crate) fn empty_formal_parameter_list(policy: JavaFormatPolicy) -> Doc {
    separated::delimited_comma_list_one_per_line(
        "(",
        ")",
        policy.continuation_indent_levels(),
        std::iter::empty(),
    )
}

pub(crate) fn argument_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, list_range, ListCommentMode::Delimited, context)?;
    Ok(argument_list_with_policy(
        "(",
        ")",
        context.policy().continuation_indent_levels(),
        items,
    ))
}

pub(crate) fn formal_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, list_range, ListCommentMode::Delimited, context)?;
    Ok(delimited_comma_list_one_per_line_with_comments(
        "(",
        ")",
        context.policy().continuation_indent_levels(),
        items,
    ))
}

pub(crate) fn type_argument_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, list_range, ListCommentMode::Delimited, context)?;
    Ok(delimited_comma_list_with_comments(
        "<",
        ">",
        context.policy().type_argument_indent_levels(),
        items,
    ))
}

pub(crate) fn type_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, list_range, ListCommentMode::Delimited, context)?;
    Ok(delimited_comma_list_one_per_line_with_comments(
        "<",
        ">",
        context.policy().type_argument_indent_levels(),
        items,
    ))
}

pub(crate) fn keyword_prefixed_clause_list(
    keyword: &'static str,
    items: impl IntoIterator<Item = ListItem>,
    _clause_range: TextRange,
    ownership_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, ownership_range, ListCommentMode::Clause, context)?;
    if keyword == "throws" {
        return Ok(keyword_prefixed_comma_list_with_comments(
            keyword,
            context.policy().continuation_indent_levels(),
            items,
        ));
    }

    let docs = items.into_docs();
    Ok(concat([text(keyword), text(" "), comma_list(docs)]))
}

pub(crate) fn braced_comma_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    one_per_line: bool,
    has_trailing_comma: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, list_range, ListCommentMode::Delimited, context)?;
    Ok(braced_comma_list_with_comments(
        items,
        one_per_line,
        has_trailing_comma,
    ))
}

#[derive(Clone, Copy)]
enum ListCommentMode {
    Delimited,
    Clause,
}

fn format_list_items(
    items: impl IntoIterator<Item = ListItem>,
    ownership_range: TextRange,
    comment_mode: ListCommentMode,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<FormattedList> {
    let items = items.into_iter().collect::<Vec<_>>();
    let dangling_range = items
        .last()
        .map_or(ownership_range, |item| TextRange::new(item.range.end(), ownership_range.end()));
    let next_item_starts = items
        .iter()
        .skip(1)
        .map(|item| item.range.start())
        .chain(std::iter::once(ownership_range.end()))
        .collect::<Vec<_>>();
    let mut docs = Vec::new();

    for (item, next_item_start) in items.into_iter().zip(next_item_starts) {
        let leading = context
            .take_leading_comments_in_range(ownership_range, item.range)
            .into_iter()
            .map(|comment| format_own_line_comment_doc(context, &comment))
            .collect::<Vec<_>>();
        let inline_leading = take_inline_leading_block_comment_docs(context, item.range);
        let has_leading_comments = !leading.is_empty();
        let has_inline_leading_comments = !inline_leading.is_empty();
        reject_unhandled_comments_in_range(
            context,
            TextRange::new(ownership_range.start(), item.range.start()),
            "Java formatter does not support comments between list items yet",
        )?;

        let mut doc = (item.format)(context)?;

        if !inline_leading.is_empty() {
            doc = concat([join(text(" "), inline_leading), text(" "), doc]);
        }

        let inline_trailing = take_inline_trailing_block_comment_docs(context, item.range);
        let trailing_blocks = context
            .take_list_item_trailing_block_comments(
                item.range,
                TextRange::new(item.range.end(), next_item_start),
            )
            .into_iter()
            .map(|comment| text(context.raw_text(&comment)))
            .collect::<Vec<_>>();
        let trailing_line = context
            .take_list_item_trailing_line_comment(
                item.range,
                TextRange::new(item.range.end(), next_item_start),
            )
            .map(|comment| text(context.raw_text(&comment)));
        let has_comments = has_leading_comments
            || has_inline_leading_comments
            || !inline_trailing.is_empty()
            || !trailing_blocks.is_empty()
            || trailing_line.is_some();
        if !inline_trailing.is_empty() {
            doc = concat([doc, text(" "), join(text(" "), inline_trailing)]);
        }

        if !leading.is_empty() {
            doc = concat([join(hard_line(), leading), hard_line(), doc]);
        }

        docs.push(FormattedListItem {
            doc,
            shape: item.shape,
            source_width: item.source_width,
            has_comments,
            has_structural_comments: has_leading_comments
                || !trailing_blocks.is_empty()
                || trailing_line.is_some(),
            trailing_blocks,
            trailing_line,
        });
    }

    let before_close = match comment_mode {
        ListCommentMode::Delimited => take_dangling_comment_docs(context, dangling_range)?,
        ListCommentMode::Clause => Vec::new(),
    };
    reject_unhandled_comments_in_range(
        context,
        ownership_range,
        "Java formatter does not support dangling comments inside lists yet",
    )?;

    Ok(FormattedList {
        items: docs,
        before_close,
    })
}

struct FormattedList {
    items: Vec<FormattedListItem>,
    before_close: Vec<Doc>,
}

impl FormattedList {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn has_comments(&self) -> bool {
        !self.before_close.is_empty() || self.items.iter().any(|item| item.has_comments)
    }

    fn has_structural_comments(&self) -> bool {
        !self.before_close.is_empty() || self.items.iter().any(|item| item.has_structural_comments)
    }

    fn into_docs(self) -> Vec<Doc> {
        self.items
            .into_iter()
            .map(FormattedListItem::into_doc)
            .collect()
    }
}

struct FormattedListItem {
    doc: Doc,
    shape: ListItemShape,
    source_width: usize,
    has_comments: bool,
    has_structural_comments: bool,
    trailing_blocks: Vec<Doc>,
    trailing_line: Option<Doc>,
}

impl FormattedListItem {
    fn into_doc(self) -> Doc {
        self.doc
    }
}

fn argument_list_with_policy(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    items: FormattedList,
) -> Doc {
    if items.is_empty() {
        return delimited_comma_list_with_comments(open, close, indent_levels, items);
    }

    let all_known = items
        .items
        .iter()
        .all(|item| item.shape != ListItemShape::Unknown);
    let docs = items
        .items
        .iter()
        .map(|item| item.doc.clone())
        .collect::<Vec<_>>();

    if !items.has_comments()
        && all_known
        && items.items.len() >= 4
        && pairable_argument_items(&items.items)
    {
        return paired_delimited_comma_list(open, close, indent_levels, docs);
    }

    delimited_comma_list_with_comments(open, close, indent_levels, items)
}

fn pairable_argument_items(items: &[FormattedListItem]) -> bool {
    let call_count = items
        .iter()
        .filter(|item| item.shape == ListItemShape::Call)
        .count();
    let starts_with_call = items
        .first()
        .is_some_and(|item| item.shape == ListItemShape::Call);

    call_count <= 1
        && !starts_with_call
        && items.iter().all(|item| !item.has_comments)
        && items.iter().all(|item| item.source_width >= 20)
        && items
            .iter()
            .all(|item| matches!(item.shape, ListItemShape::Simple | ListItemShape::Call))
}

fn delimited_comma_list_with_comments(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    list: FormattedList,
) -> Doc {
    if !list.has_structural_comments() {
        return separated::delimited_comma_list(open, close, indent_levels, list.into_docs());
    }

    comment_delimited_comma_list(open, close, indent_levels, list)
}

fn delimited_comma_list_one_per_line_with_comments(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    list: FormattedList,
) -> Doc {
    if !list.has_structural_comments() {
        return separated::delimited_comma_list_one_per_line(
            open,
            close,
            indent_levels,
            list.into_docs(),
        );
    }

    comment_delimited_comma_list(open, close, indent_levels, list)
}

fn keyword_prefixed_comma_list_with_comments(
    keyword: &'static str,
    continuation_indent_levels: u16,
    list: FormattedList,
) -> Doc {
    if !list.has_structural_comments() {
        return separated::keyword_prefixed_comma_list(
            keyword,
            continuation_indent_levels,
            list.into_docs(),
        );
    }

    let docs = list.into_docs();
    if docs.is_empty() {
        return text(keyword);
    }

    group(concat([
        text(keyword),
        indent_by(
            continuation_indent_levels,
            concat([hard_line(), join(concat([text(","), hard_line()]), docs)]),
        ),
    ]))
}

fn comment_delimited_comma_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    list: FormattedList,
) -> Doc {
    let list_item_count = list.items.len();
    let has_before_close = !list.before_close.is_empty();
    let close_on_own_line = has_before_close
        || list
            .items
            .last()
            .is_some_and(|item| item.trailing_line.is_some());
    let mut parts = list
        .items
        .into_iter()
        .map(|item| (item.doc, item.trailing_blocks, item.trailing_line))
        .collect::<Vec<_>>();
    parts.extend(
        list.before_close
            .into_iter()
            .map(|comment| (comment, Vec::new(), None)),
    );

    if parts.is_empty() {
        return text(format!("{open}{close}"));
    }

    let mut body = Vec::new();
    let total_parts = parts.len();
    for (index, (part, trailing_blocks, trailing_line)) in parts.into_iter().enumerate() {
        body.push(part);
        if index + 1 < list_item_count {
            body.push(text(","));
        }
        for comment in trailing_blocks {
            body.push(text(" "));
            body.push(comment);
        }
        if let Some(comment) = trailing_line {
            body.push(text(" "));
            body.push(comment);
        }
        if index + 1 < total_parts {
            body.push(hard_line());
        }
    }

    if close_on_own_line {
        return group(concat([
            text(open),
            indent_by(indent_levels, concat([hard_line(), concat(body)])),
            hard_line(),
            text(close),
        ]));
    }

    group(concat([
        text(open),
        indent_by(
            indent_levels,
            concat([hard_line(), concat(body), text(close)]),
        ),
    ]))
}

fn braced_comma_list_with_comments(
    list: FormattedList,
    one_per_line: bool,
    has_trailing_comma: bool,
) -> Doc {
    if !list.has_structural_comments() {
        let mut docs = list.into_docs();
        if has_trailing_comma && let Some(last) = docs.last_mut() {
            *last = concat([last.clone(), text(",")]);
        }
        if one_per_line {
            return braced_comma_block_one_per_line(docs);
        }
        return braced_comma_block(docs);
    }

    comment_braced_comma_list(list, has_trailing_comma)
}

fn braced_comma_block(items: impl IntoIterator<Item = Doc>) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text("{}");
    }

    let last = items.pop().expect("non-empty items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    concat([
        text("{"),
        indent(concat([
            hard_line_without_break_parent(),
            fill(entries, last),
        ])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}

fn braced_comma_block_one_per_line(items: impl IntoIterator<Item = Doc>) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text("{}");
    }

    let last = items.pop().expect("non-empty items checked above");
    let mut body = items
        .into_iter()
        .flat_map(|item| [item, text(","), hard_line_without_break_parent()])
        .collect::<Vec<_>>();
    body.push(last);

    concat([
        text("{"),
        indent(concat([hard_line_without_break_parent(), concat(body)])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}

fn comment_braced_comma_list(list: FormattedList, has_trailing_comma: bool) -> Doc {
    let list_item_count = list.items.len();
    let mut parts = list
        .items
        .into_iter()
        .map(|item| (item.doc, item.trailing_blocks, item.trailing_line))
        .collect::<Vec<_>>();
    parts.extend(
        list.before_close
            .into_iter()
            .map(|comment| (comment, Vec::new(), None)),
    );

    if parts.is_empty() {
        return text("{}");
    }

    let mut body = Vec::new();
    let total_parts = parts.len();
    for (index, (part, trailing_blocks, trailing_line)) in parts.into_iter().enumerate() {
        body.push(part);
        if index + 1 < list_item_count || (has_trailing_comma && index + 1 == list_item_count) {
            body.push(text(","));
        }
        for comment in trailing_blocks {
            body.push(text(" "));
            body.push(comment);
        }
        if let Some(comment) = trailing_line {
            body.push(text(" "));
            body.push(comment);
        }
        if index + 1 < total_parts {
            body.push(hard_line_without_break_parent());
        }
    }

    concat([
        text("{"),
        indent(concat([hard_line_without_break_parent(), concat(body)])),
        hard_line_without_break_parent(),
        text("}"),
    ])
}

fn text_range_width(range: TextRange) -> usize {
    range.end().get().saturating_sub(range.start().get())
}

fn paired_delimited_comma_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    items: Vec<Doc>,
) -> Doc {
    let pairs = items
        .chunks(2)
        .map(|chunk| {
            if let [left, right] = chunk {
                concat([left.clone(), text(", "), right.clone()])
            } else {
                chunk[0].clone()
            }
        })
        .collect::<Vec<_>>();

    group(concat([
        text(open),
        indent_by(
            indent_levels,
            concat([soft_line(), join(concat([text(","), line()]), pairs)]),
        ),
        text(close),
    ]))
}
