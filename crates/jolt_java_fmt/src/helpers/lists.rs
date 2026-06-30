use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{
    Doc, best_fitting, concat, fill, fill_entry, group, hard_line, hard_line_without_break_parent,
    indent, indent_by, join, line, soft_line, text,
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

pub(crate) fn statement_expression_list(
    items: impl IntoIterator<Item = ListItem>,
    ownership_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, ownership_range, ListCommentMode::Clause, context)?;
    if !items.has_structural_comments() {
        return Ok(comma_list(items.into_docs()));
    }

    Ok(comment_clause_comma_list(items))
}

pub(crate) fn argument_list_docs(
    items: impl IntoIterator<Item = Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    separated::delimited_comma_list("(", ")", policy.continuation_indent_levels(), items)
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
    is_format_method: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range: None,
        },
        context,
    )?;
    Ok(argument_list_with_policy(
        "(",
        ")",
        context.policy().continuation_indent_levels(),
        context.policy(),
        is_format_method,
        items,
    ))
}

pub(crate) fn formal_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    open_range: Option<TextRange>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range,
        },
        context,
    )?;
    Ok(delimited_comma_list_one_per_line_with_comments(
        "(",
        ")",
        context.policy().continuation_indent_levels(),
        items,
    ))
}

pub(crate) fn lambda_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    open_range: Option<TextRange>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range,
        },
        context,
    )?;
    Ok(delimited_comma_list_with_comments(
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
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "<",
            open_range: None,
        },
        context,
    )?;
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
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "<",
            open_range: None,
        },
        context,
    )?;
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
    type_clause_list(
        keyword,
        items,
        ownership_range,
        context.policy().continuation_indent_levels(),
        context,
    )
}

/// `extends` / `implements` / `permits` / `throws` keyword-prefixed type lists.
pub(crate) fn type_clause_list(
    keyword: &'static str,
    items: impl IntoIterator<Item = ListItem>,
    ownership_range: TextRange,
    continuation_indent_levels: u16,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(items, ownership_range, ListCommentMode::Clause, context)?;
    if keyword == "throws" {
        return Ok(keyword_prefixed_comma_list_with_comments(
            keyword,
            continuation_indent_levels,
            items,
        ));
    }

    if items.has_structural_comments() {
        return Ok(concat([
            text(keyword),
            text(" "),
            comment_clause_comma_list(items),
        ]));
    }

    Ok(keyword_type_list_clause(
        keyword,
        items.into_docs(),
        continuation_indent_levels,
    ))
}

fn keyword_type_list_clause(
    keyword: &'static str,
    type_docs: Vec<Doc>,
    continuation_indent_levels: u16,
) -> Doc {
    assert!(
        !type_docs.is_empty(),
        "parser-clean type clause should contain at least one type"
    );

    if type_docs.len() == 1 {
        return concat([
            text(keyword),
            text(" "),
            type_docs
                .into_iter()
                .next()
                .expect("one type checked above"),
        ]);
    }

    let mut type_docs = type_docs;
    let first = type_docs.remove(0);
    let rest = type_docs
        .into_iter()
        .flat_map(|doc| [text(","), line(), doc])
        .collect::<Vec<_>>();

    group(concat([
        text(keyword),
        text(" "),
        first,
        indent_by(continuation_indent_levels, concat(rest)),
    ]))
}

pub(crate) fn braced_comma_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    one_per_line: bool,
    has_trailing_comma: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "{",
            open_range: None,
        },
        context,
    )?;
    Ok(braced_comma_list_with_comments(
        items,
        one_per_line,
        has_trailing_comma,
    ))
}

#[derive(Clone, Copy)]
enum ListCommentMode {
    Delimited {
        open: &'static str,
        open_range: Option<TextRange>,
    },
    Clause,
}

fn format_list_items(
    items: impl IntoIterator<Item = ListItem>,
    ownership_range: TextRange,
    comment_mode: ListCommentMode,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<FormattedList> {
    let items = items.into_iter().collect::<Vec<_>>();
    let dangling_range = items.last().map_or(ownership_range, |item| {
        TextRange::new(item.range.end(), ownership_range.end())
    });
    let next_item_starts = items
        .iter()
        .skip(1)
        .map(|item| item.range.start())
        .chain(std::iter::once(ownership_range.end()))
        .collect::<Vec<_>>();
    let mut opening_comments = match comment_mode {
        ListCommentMode::Delimited {
            open, open_range, ..
        } => take_opening_delimiter_comments(
            context,
            open_range.unwrap_or_else(|| {
                TextRange::new(
                    ownership_range.start(),
                    ownership_range.start() + open.len().into(),
                )
            }),
        ),
        ListCommentMode::Clause => Vec::new(),
    };
    let mut docs = Vec::new();
    let mut previous_item_end = None;

    for (index, (item, next_item_start)) in items.into_iter().zip(next_item_starts).enumerate() {
        let gap_start = previous_item_end.unwrap_or_else(|| ownership_range.start());
        let mut leading = context
            .take_leading_comments_in_range(ownership_range, item.range)
            .into_iter()
            .map(|comment| format_own_line_comment_doc(context, &comment))
            .collect::<Vec<_>>();
        if let Some(previous_item_end) = previous_item_end {
            let separator_comments = context
                .take_list_separator_trailing_line_comments(TextRange::new(
                    previous_item_end,
                    item.range.start(),
                ))
                .into_iter()
                .map(|comment| format_own_line_comment_doc(context, &comment))
                .collect::<Vec<_>>();
            if !separator_comments.is_empty() {
                let mut comments = separator_comments;
                comments.extend(leading);
                leading = comments;
            }
        }
        if index == 0 && !opening_comments.is_empty() {
            let mut comments = Vec::new();
            comments.append(&mut opening_comments);
            comments.extend(leading);
            leading = comments;
        }
        let inline_leading = take_inline_leading_block_comment_docs(context, item.range);
        let has_leading_comments = !leading.is_empty();
        let has_inline_leading_comments = !inline_leading.is_empty();
        reject_unhandled_comments_in_range(
            context,
            TextRange::new(gap_start, item.range.start()),
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
        previous_item_end = Some(item.range.end());
    }

    let mut before_close = match comment_mode {
        ListCommentMode::Delimited { .. } => take_dangling_comment_docs(context, dangling_range)?,
        ListCommentMode::Clause => Vec::new(),
    };
    if docs.is_empty() && !opening_comments.is_empty() {
        opening_comments.extend(before_close);
        before_close = opening_comments;
    }
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

fn take_opening_delimiter_comments(
    context: &mut JavaFormatContext<'_>,
    delimiter_range: TextRange,
) -> Vec<Doc> {
    context
        .take_trailing_line_comment(delimiter_range)
        .into_iter()
        .map(|comment| format_own_line_comment_doc(context, &comment))
        .collect()
}

pub(crate) struct FormattedList {
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

    pub(crate) fn has_structural_comments(&self) -> bool {
        !self.before_close.is_empty() || self.items.iter().any(|item| item.has_structural_comments)
    }

    pub(crate) fn into_docs(self) -> Vec<Doc> {
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
    policy: JavaFormatPolicy,
    is_format_method: bool,
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

    if items.has_comments() {
        return delimited_comma_list_with_comments(open, close, indent_levels, items);
    }

    if is_format_method && items.items.len() >= 2 {
        return format_method_argument_list(open, close, indent_levels, docs);
    }

    let all_short = has_only_short_argument_items(&items.items, policy);
    let flat = separated::delimited_comma_list_flat(open, close, docs.clone());
    let broken = if all_short {
        separated::delimited_comma_list(open, close, indent_levels, docs)
    } else {
        separated::delimited_comma_list_one_per_line(open, close, indent_levels, docs)
    };
    best_fitting(flat, [broken])
}

/// google-java-format `isFormatMethod`: format string on its own continuation
/// line, remaining arguments filled (not one-per-line).
fn format_method_argument_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    mut docs: Vec<Doc>,
) -> Doc {
    let first = docs.remove(0);
    let flat = separated::delimited_comma_list_flat(
        open,
        close,
        std::iter::once(first.clone()).chain(docs.clone()),
    );
    let broken = format_method_argument_list_broken(open, close, indent_levels, first, docs);
    best_fitting(flat, [broken])
}

fn format_method_argument_list_broken(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    first: Doc,
    rest: Vec<Doc>,
) -> Doc {
    if rest.is_empty() {
        return group(concat([
            text(open),
            indent_by(
                indent_levels,
                concat([soft_line(), concat([first, text(close)])]),
            ),
        ]));
    }

    let mut rest = rest;
    let last = rest
        .pop()
        .expect("format method argument list has at least two items");
    let entries = rest
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    group(concat([
        text(open),
        indent_by(
            indent_levels,
            concat([
                soft_line(),
                concat([
                    first,
                    text(","),
                    line(),
                    fill(entries, concat([last, text(close)])),
                ]),
            ]),
        ),
    ]))
}

fn has_only_short_argument_items(items: &[FormattedListItem], policy: JavaFormatPolicy) -> bool {
    let max_length = policy.argument_list_max_item_length_for_filling();
    items
        .iter()
        .all(|item| !item.has_comments && item.source_width < max_length)
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

fn comment_clause_comma_list(list: FormattedList) -> Doc {
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

    concat(body)
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

pub(crate) fn format_braced_list_items(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<FormattedList> {
    format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "{",
            open_range: None,
        },
        context,
    )
}

pub(crate) fn comment_braced_comma_list(list: FormattedList, has_trailing_comma: bool) -> Doc {
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
