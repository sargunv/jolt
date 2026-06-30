use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{Doc, concat, group, hard_line, indent_by, join, line, soft_line, text};

use crate::comments::{
    format_own_line_comment_doc, reject_unhandled_comments_in_range,
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
    let items = format_list_items(items, list_range, context)?;
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
    let items = format_list_items(items, list_range, context)?
        .into_iter()
        .map(FormattedListItem::into_doc);
    Ok(separated::delimited_comma_list_one_per_line(
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
    let items = format_list_items(items, list_range, context)?
        .into_iter()
        .map(FormattedListItem::into_doc);
    Ok(separated::delimited_comma_list(
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
    let items = format_list_items(items, list_range, context)?
        .into_iter()
        .map(FormattedListItem::into_doc);
    Ok(separated::delimited_comma_list_one_per_line(
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
    let items = format_list_items(items, ownership_range, context)?
        .into_iter()
        .map(FormattedListItem::into_doc);
    if keyword == "throws" {
        return Ok(separated::keyword_prefixed_comma_list(
            keyword,
            context.policy().continuation_indent_levels(),
            items,
        ));
    }

    Ok(concat([text(keyword), text(" "), comma_list(items)]))
}

fn format_list_items(
    items: impl IntoIterator<Item = ListItem>,
    ownership_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Vec<FormattedListItem>> {
    let mut docs = Vec::new();

    for item in items {
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
        let has_comments =
            has_leading_comments || has_inline_leading_comments || !inline_trailing.is_empty();
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
        });
    }

    reject_unhandled_comments_in_range(
        context,
        ownership_range,
        "Java formatter does not support dangling comments inside lists yet",
    )?;

    Ok(docs)
}

struct FormattedListItem {
    doc: Doc,
    shape: ListItemShape,
    source_width: usize,
    has_comments: bool,
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
    items: Vec<FormattedListItem>,
) -> Doc {
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let all_known = items
        .iter()
        .all(|item| item.shape != ListItemShape::Unknown);
    let docs = items
        .iter()
        .map(|item| item.doc.clone())
        .collect::<Vec<_>>();

    if all_known && items.len() >= 4 && pairable_argument_items(&items) {
        return paired_delimited_comma_list(open, close, indent_levels, docs);
    }

    separated::delimited_comma_list(open, close, indent_levels, docs)
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
