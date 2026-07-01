use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{
    Doc, FlatLine, GroupId, LevelBreakMode, break_level, break_level_with_indent, concat, fill,
    fill_entry, flat_text, group, group_id, hard_line, hard_line_without_break_parent, indent,
    indent_by, join, level_break, line, soft_line, text,
};

use crate::analyzers::array_initializers::TabularEntry;
use crate::comments::{
    format_inline_comment_doc, format_own_line_comment_doc, reject_unhandled_comments_in_range,
    take_dangling_comment_docs, take_inline_leading_block_comment_docs,
    take_inline_trailing_block_comment_docs,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::helpers::separated;
use crate::policy::JavaFormatPolicy;

pub(crate) const TYPE_DECLARATION_TYPE_PARAMETERS_GROUP_ID: GroupId = GroupId(1);
pub(crate) const SELECTOR_TYPE_ARGUMENTS_GROUP_ID: GroupId = GroupId(2);

pub(crate) struct ListItem {
    range: TextRange,
    source_width: usize,
    shape: ListItemShape,
    tabular_entry: Option<TabularEntry>,
    has_inline_comments: bool,
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
            tabular_entry: None,
            has_inline_comments: false,
            format: Box::new(format),
        }
    }

    pub(crate) fn with_shape(mut self, shape: ListItemShape) -> Self {
        self.shape = shape;
        self
    }

    pub(crate) fn with_tabular_entry(mut self, entry: TabularEntry) -> Self {
        self.tabular_entry = Some(entry);
        self
    }

    pub(crate) fn with_inline_comments(mut self, has_inline_comments: bool) -> Self {
        self.has_inline_comments = has_inline_comments;
        self
    }

    pub(crate) fn doc(doc: Doc, range: TextRange) -> Self {
        Self::new(range, |_| Ok(doc))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ListItemShape {
    Simple,
    SelectorChain,
    AnonymousObjectCreationUnit,
    NestedArgumentUnit,
    WideHeadNestedArgumentUnit,
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
    let items = format_list_items(
        items,
        ownership_range,
        ListCommentMode::Clause,
        ',',
        0,
        false,
        context,
    )?;
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
    argument_list_with_continuation_indent(
        items,
        list_range,
        is_format_method,
        context.policy().continuation_indent_levels(),
        context,
    )
}

pub(crate) fn argument_list_with_continuation_indent(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    is_format_method: bool,
    continuation_indent_levels: u16,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range: None,
        },
        ',',
        0,
        true,
        context,
    )?;
    argument_list_with_policy(
        "(",
        ")",
        continuation_indent_levels,
        context.policy(),
        is_format_method,
        items,
        context,
    )
}

pub(crate) fn formal_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    open_range: Option<TextRange>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    formal_parameter_list_with_indent(
        items,
        list_range,
        open_range,
        context.policy().continuation_indent_levels(),
        context,
    )
}

pub(crate) fn formal_parameter_list_with_indent(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    open_range: Option<TextRange>,
    continuation_indent_levels: u16,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let separator_leading_comment_indent_columns = if continuation_indent_levels == 0 {
        context.policy().continuation_indent_columns()
    } else {
        0
    };
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range,
        },
        ',',
        separator_leading_comment_indent_columns,
        false,
        context,
    )?;
    Ok(delimited_comma_list_one_per_line_with_comments(
        "(",
        ")",
        continuation_indent_levels,
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
        ',',
        0,
        false,
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
    type_argument_list_with_indent(
        items,
        list_range,
        context.policy().type_argument_indent_levels(),
        false,
        context,
    )
}

pub(crate) fn nested_type_argument_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    type_argument_list_with_indent(
        items,
        list_range,
        context.policy().nested_type_argument_indent_levels(),
        false,
        context,
    )
}

pub(crate) fn type_clause_type_argument_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    has_multiple_clause_types: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    type_argument_list_with_indent(
        items,
        list_range,
        context
            .policy()
            .type_clause_type_argument_indent_levels(has_multiple_clause_types),
        context
            .policy()
            .type_clause_type_arguments_one_per_line(has_multiple_clause_types),
        context,
    )
}

fn type_argument_list_with_indent(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    indent_levels: u16,
    one_per_line: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "<",
            open_range: None,
        },
        ',',
        0,
        false,
        context,
    )?;
    if one_per_line {
        return Ok(delimited_comma_list_one_per_line_with_comments(
            "<",
            ">",
            indent_levels,
            items,
        ));
    }

    if items.has_structural_comments() {
        return Ok(comment_delimited_comma_list("<", ">", indent_levels, items));
    }

    Ok(gjf_type_argument_list(indent_levels, items.into_docs()))
}

pub(crate) fn selector_type_argument_list_variants(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Doc)> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "<",
            open_range: None,
        },
        ',',
        0,
        false,
        context,
    )?;
    let default = selector_type_argument_list_doc(
        items.clone(),
        context.policy().type_argument_indent_levels(),
    );
    let after_chain_break = selector_type_argument_list_doc(
        items,
        context.policy().selector_type_argument_indent_levels(),
    );
    Ok((default, after_chain_break))
}

fn selector_type_argument_list_doc(items: FormattedList, indent_levels: u16) -> Doc {
    if items.has_structural_comments() {
        return group_id(
            SELECTOR_TYPE_ARGUMENTS_GROUP_ID,
            comment_delimited_comma_list("<", ">", indent_levels, items),
        );
    }

    group_id(
        SELECTOR_TYPE_ARGUMENTS_GROUP_ID,
        selector_type_argument_list_without_open_break(indent_levels, items.into_docs()),
    )
}

fn selector_type_argument_list_without_open_break(indent_levels: u16, mut docs: Vec<Doc>) -> Doc {
    if docs.is_empty() {
        return text("<>");
    }

    let last = docs.pop().expect("non-empty selector type arguments");
    let entries = docs
        .into_iter()
        .map(|doc| fill_entry(doc, concat([text(","), line()])));
    group(concat([
        text("<"),
        indent_by(indent_levels, fill(entries, concat([last, text(">")]))),
    ]))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeParameterListContext {
    TypeDeclaration { has_following_type_clauses: bool },
    CallableDeclaration,
}

pub(crate) fn type_parameter_list(
    items: impl IntoIterator<Item = ListItem>,
    list_range: TextRange,
    list_context: TypeParameterListContext,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        list_range,
        ListCommentMode::Delimited {
            open: "<",
            open_range: None,
        },
        ',',
        0,
        false,
        context,
    )?;
    let indent_levels = match list_context {
        TypeParameterListContext::TypeDeclaration {
            has_following_type_clauses,
        } => context
            .policy()
            .declaration_type_parameter_indent_levels(has_following_type_clauses),
        TypeParameterListContext::CallableDeclaration => {
            context.policy().callable_type_parameter_indent_levels()
        }
    };

    if matches!(
        list_context,
        TypeParameterListContext::TypeDeclaration { .. }
    ) && context.policy().declaration_type_parameters_fill()
        && items.items.len()
            <= context
                .policy()
                .declaration_type_parameters_fill_max_items()
        && !items.has_structural_comments()
        && items
            .items
            .iter()
            .all(|item| item.shape != ListItemShape::Complex)
    {
        let type_parameters = delimited_comma_list_with_group_id(
            "<",
            ">",
            indent_levels,
            TYPE_DECLARATION_TYPE_PARAMETERS_GROUP_ID,
            items.into_docs(),
        );
        return Ok(type_parameters);
    }

    let type_parameters =
        delimited_comma_list_one_per_line_with_comments("<", ">", indent_levels, items);
    if matches!(
        list_context,
        TypeParameterListContext::TypeDeclaration { .. }
    ) {
        Ok(group_id(
            TYPE_DECLARATION_TYPE_PARAMETERS_GROUP_ID,
            type_parameters,
        ))
    } else {
        Ok(type_parameters)
    }
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
    let items = format_list_items(
        items,
        ownership_range,
        ListCommentMode::Clause,
        ',',
        0,
        false,
        context,
    )?;
    if keyword == "throws" && items.has_structural_comments() {
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

pub(crate) fn resource_specification(
    items: impl IntoIterator<Item = ListItem>,
    specification_range: TextRange,
    has_trailing_semicolon: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = format_list_items(
        items,
        specification_range,
        ListCommentMode::Delimited {
            open: "(",
            open_range: None,
        },
        ';',
        0,
        false,
        context,
    )?;
    Ok(resource_specification_with_comments(
        items,
        has_trailing_semicolon,
        context.policy().continuation_indent_levels(),
    ))
}

fn resource_specification_with_comments(
    list: FormattedList,
    has_trailing_semicolon: bool,
    indent_levels: u16,
) -> Doc {
    if list.is_empty() {
        return text("()");
    }

    if list.has_structural_comments() {
        return comment_delimited_semicolon_list(
            "(",
            ")",
            indent_levels,
            list,
            has_trailing_semicolon,
        );
    }

    let mut docs = list.into_docs();
    if docs.len() == 1 {
        let resource = docs
            .pop()
            .expect("single resource checked above should have one doc");
        let trailing = if has_trailing_semicolon {
            text("; ")
        } else {
            text("")
        };
        return group(concat([text("("), resource, trailing, text(")")]));
    }

    semicolon_delimited_list("(", ")", indent_levels, docs, has_trailing_semicolon)
}

fn semicolon_delimited_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    mut items: Vec<Doc>,
    has_trailing_semicolon: bool,
) -> Doc {
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let last = items.pop().expect("non-empty items checked above");
    let trailing = if has_trailing_semicolon {
        text("; ")
    } else {
        text("")
    };
    let last_with_close = concat([last, trailing, text(close)]);

    if items.is_empty() {
        return group(concat([text(open), last_with_close]));
    }

    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(";"), line()])));

    group(concat([
        text(open),
        indent_by(
            indent_levels,
            concat([soft_line(), fill(entries, last_with_close)]),
        ),
    ]))
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
    separator: char,
    separator_leading_comment_indent_columns: usize,
    reattach_leading_parameter_comments: bool,
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
    let mut after_open = match comment_mode {
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

    for (item, next_item_start) in items.into_iter().zip(next_item_starts) {
        let gap_start = previous_item_end.unwrap_or_else(|| ownership_range.start());
        let mut has_separator_leading_comments = false;
        let mut inline_leading = if reattach_leading_parameter_comments {
            context
                .take_list_item_leading_block_comments(ownership_range, item.range)
                .into_iter()
                .map(|comment| format_inline_comment_doc(context, &comment))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut leading = Vec::new();
        for comment in context.take_leading_comments_in_range(ownership_range, item.range) {
            leading.push(format_own_line_comment_doc(context, &comment));
        }
        if let Some(previous_item_end) = previous_item_end {
            let separator_comments = context
                .take_list_separator_trailing_line_comments_with_separator(
                    TextRange::new(previous_item_end, item.range.start()),
                    separator,
                )
                .into_iter()
                .map(|comment| {
                    let comment = concat([
                        text(separator.to_string()),
                        text(" "),
                        format_own_line_comment_doc(context, &comment),
                    ]);
                    if separator_leading_comment_indent_columns == 0 {
                        comment
                    } else {
                        concat([
                            text(" ".repeat(separator_leading_comment_indent_columns)),
                            comment,
                        ])
                    }
                })
                .collect::<Vec<_>>();
            if !separator_comments.is_empty() {
                has_separator_leading_comments = true;
                let mut comments = separator_comments;
                comments.extend(leading);
                leading = comments;
            }
        }
        inline_leading.extend(take_inline_leading_block_comment_docs(context, item.range));
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
            .take_list_item_trailing_block_comments_with_separator(
                item.range,
                TextRange::new(item.range.end(), next_item_start),
                separator,
            )
            .into_iter()
            .map(|comment| text(context.raw_text(&comment)))
            .collect::<Vec<_>>();
        let trailing_line = context
            .take_list_item_trailing_line_comment_with_separator(
                item.range,
                TextRange::new(item.range.end(), next_item_start),
                separator,
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
            tabular_entry: item.tabular_entry,
            has_inline_comments: item.has_inline_comments || has_inline_leading_comments,
            has_comments,
            has_structural_comments: has_leading_comments
                || !trailing_blocks.is_empty()
                || trailing_line.is_some(),
            has_separator_leading_comments,
            trailing_blocks,
            trailing_line,
        });
        previous_item_end = Some(item.range.end());
    }

    let before_close = match comment_mode {
        ListCommentMode::Delimited { .. } => take_dangling_comment_docs(context, dangling_range)?,
        ListCommentMode::Clause => Vec::new(),
    };
    reject_unhandled_comments_in_range(
        context,
        ownership_range,
        "Java formatter does not support dangling comments inside lists yet",
    )?;

    Ok(FormattedList {
        after_open: std::mem::take(&mut after_open),
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
        .map(|comment| text(context.raw_text(&comment)))
        .collect()
}

#[derive(Clone)]
pub(crate) struct FormattedList {
    after_open: Vec<Doc>,
    items: Vec<FormattedListItem>,
    before_close: Vec<Doc>,
}

impl FormattedList {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn has_comments(&self) -> bool {
        !self.after_open.is_empty()
            || !self.before_close.is_empty()
            || self.items.iter().any(|item| item.has_comments)
    }

    pub(crate) fn has_structural_comments(&self) -> bool {
        !self.after_open.is_empty()
            || !self.before_close.is_empty()
            || self.items.iter().any(|item| item.has_structural_comments)
    }

    fn has_only_before_close_structural_comments(&self) -> bool {
        self.after_open.is_empty()
            && !self.before_close.is_empty()
            && self.items.iter().all(|item| {
                !item.has_comments
                    && !item.has_structural_comments
                    && item.trailing_blocks.is_empty()
                    && item.trailing_line.is_none()
                    && !item.has_separator_leading_comments
            })
    }

    fn has_inline_comments(&self) -> bool {
        self.items.iter().any(|item| item.has_inline_comments)
    }

    pub(crate) fn into_docs(self) -> Vec<Doc> {
        self.items
            .into_iter()
            .map(FormattedListItem::into_doc)
            .collect()
    }
}

#[derive(Clone)]
struct FormattedListItem {
    doc: Doc,
    shape: ListItemShape,
    source_width: usize,
    tabular_entry: Option<TabularEntry>,
    has_inline_comments: bool,
    has_comments: bool,
    has_structural_comments: bool,
    has_separator_leading_comments: bool,
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
    context: &JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if items.is_empty() {
        return Ok(delimited_comma_list_with_comments(
            open,
            close,
            indent_levels,
            items,
        ));
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
    let has_policy_inline_comments = policy
        .argument_list_breaks_inline_commented_items_one_per_line()
        && items.has_inline_comments();
    let has_policy_comments = items.has_comments() || has_policy_inline_comments;

    if !has_policy_comments
        && all_known
        && items.items.len() >= 4
        && items.items.len().is_multiple_of(2)
        && tabular_argument_columns(&items.items, context)? == Some(2)
    {
        return Ok(paired_delimited_comma_list(
            open,
            close,
            indent_levels,
            docs,
        ));
    }

    if has_policy_comments {
        if has_policy_inline_comments && !items.has_structural_comments() {
            return Ok(separated::delimited_comma_list_one_per_line(
                open,
                close,
                indent_levels,
                docs,
            ));
        }

        return Ok(delimited_comma_list_with_comments(
            open,
            close,
            indent_levels,
            items,
        ));
    }

    if is_format_method && items.items.len() >= 2 {
        let rest_fill_mode = if argument_items_use_fill_layout(&items.items[1..], policy, context) {
            LevelBreakMode::Independent
        } else {
            LevelBreakMode::Unified
        };
        return Ok(format_method_argument_list(
            open,
            close,
            indent_levels,
            docs,
            rest_fill_mode,
        ));
    }

    let fill_mode = if argument_items_use_fill_layout(&items.items, policy, context) {
        LevelBreakMode::Independent
    } else {
        LevelBreakMode::Unified
    };
    Ok(gjf_argument_list(
        open,
        close,
        indent_levels,
        docs,
        fill_mode,
    ))
}

/// google-java-format `addArguments` else branch: open, `(`, break, `argList`, `)`.
fn gjf_argument_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    docs: Vec<Doc>,
    fill_mode: LevelBreakMode,
) -> Doc {
    if docs.is_empty() {
        return text(format!("{open}{close}"));
    }

    let inner = gjf_delimited_comma_list_body(docs, close, fill_mode);
    break_level_with_indent(
        indent_levels as i16,
        [text(open), inner],
        [level_break(LevelBreakMode::Unified, FlatLine::Empty, 0)],
    )
    .expect("valid GJF argument list level")
}

/// google-java-format `visitParameterizedType`: `<`, break, zero-indent arg list, `>`.
fn gjf_type_argument_list(indent_levels: u16, docs: Vec<Doc>) -> Doc {
    if docs.is_empty() {
        return text("<>");
    }

    let inner = gjf_delimited_comma_list_body(docs, ">", LevelBreakMode::Independent);
    break_level_with_indent(
        indent_levels as i16,
        [text("<"), inner],
        [level_break(LevelBreakMode::Unified, FlatLine::Empty, 0)],
    )
    .expect("valid GJF type argument list level")
}

/// google-java-format `argList` at zero indent; trailing `close` sits on the last segment.
fn gjf_delimited_comma_list_body(
    mut docs: Vec<Doc>,
    close: &'static str,
    fill_mode: LevelBreakMode,
) -> Doc {
    let last = docs.pop().expect("non-empty argList");
    let last_with_close = concat([last, text(close)]);

    if docs.is_empty() {
        return last_with_close;
    }

    let comma_items = docs
        .into_iter()
        .map(|doc| concat([doc, text(",")]))
        .collect::<Vec<_>>();
    let breaks = vec![level_break(fill_mode, flat_text(" "), 0); comma_items.len()];
    break_level(
        comma_items
            .into_iter()
            .chain(std::iter::once(last_with_close)),
        breaks,
    )
    .expect("valid argList comma breaks")
}

/// google-java-format `isFormatMethod`: format string, then unified `argList` for the rest.
fn format_method_argument_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    mut docs: Vec<Doc>,
    rest_fill_mode: LevelBreakMode,
) -> Doc {
    let first = docs.remove(0);
    let inner = if docs.is_empty() {
        concat([first, text(close)])
    } else {
        let rest = gjf_delimited_comma_list_body(docs, close, rest_fill_mode);
        break_level(
            [concat([first, text(",")]), rest],
            [level_break(LevelBreakMode::Unified, FlatLine::Space, 0)],
        )
        .expect("valid format-method argument breaks")
    };

    break_level_with_indent(
        indent_levels as i16,
        [text(open), inner],
        [level_break(LevelBreakMode::Unified, FlatLine::Empty, 0)],
    )
    .expect("valid GJF format-method argument list level")
}

fn argument_items_use_fill_layout(
    items: &[FormattedListItem],
    policy: JavaFormatPolicy,
    context: &JavaFormatContext<'_>,
) -> bool {
    if context.nested_argument_depth() > 0
        && items.len() >= policy.argument_list_nested_fill_max_items()
    {
        return false;
    }

    items.iter().all(|item| {
        !item.has_comments && item.source_width < policy.argument_list_max_item_length_for_filling()
    })
}

fn tabular_argument_columns(
    items: &[FormattedListItem],
    context: &JavaFormatContext<'_>,
) -> FormatResult<Option<usize>> {
    if items.iter().any(|item| item.has_comments) {
        return Ok(None);
    }

    let entries = items
        .iter()
        .map(|item| item.tabular_entry)
        .collect::<Option<Vec<_>>>();
    let Some(entries) = entries else {
        return Ok(None);
    };

    Ok(
        crate::analyzers::array_initializers::tabular_layout_for_entries(&entries, context)
            .map(|layout| layout.cols),
    )
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

fn delimited_comma_list_with_group_id(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    id: GroupId,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let last = items.pop().expect("non-empty items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    group_id(
        id,
        concat([
            text(open),
            indent_by(
                indent_levels,
                concat([soft_line(), fill(entries, concat([last, text(close)]))]),
            ),
        ]),
    )
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
    let after_open = list.after_open;
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
        .map(|item| {
            (
                item.doc,
                item.trailing_blocks,
                item.trailing_line,
                item.has_separator_leading_comments,
            )
        })
        .collect::<Vec<_>>();
    parts.extend(
        list.before_close
            .into_iter()
            .map(|comment| (comment, Vec::new(), None, false)),
    );

    if parts.is_empty() {
        if !after_open.is_empty() {
            return group(concat([
                open_with_comments(open, after_open),
                hard_line(),
                text(close),
            ]));
        }
        return text(format!("{open}{close}"));
    }

    let mut body = Vec::new();
    let total_parts = parts.len();
    for (index, (part, trailing_blocks, trailing_line, _)) in parts.iter().cloned().enumerate() {
        body.push(part);
        if index + 1 < list_item_count && !parts[index + 1].3 {
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
            open_with_comments(open, after_open),
            indent_by(indent_levels, concat([hard_line(), concat(body)])),
            hard_line(),
            text(close),
        ]));
    }

    group(concat([
        open_with_comments(open, after_open),
        indent_by(
            indent_levels,
            concat([hard_line(), concat(body), text(close)]),
        ),
    ]))
}

fn comment_delimited_semicolon_list(
    open: &'static str,
    close: &'static str,
    indent_levels: u16,
    list: FormattedList,
    has_trailing_semicolon: bool,
) -> Doc {
    let after_open = list.after_open;
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
        if !after_open.is_empty() {
            return group(concat([
                open_with_comments(open, after_open),
                hard_line(),
                text(close),
            ]));
        }
        return text(format!("{open}{close}"));
    }

    let mut body = Vec::new();
    let total_parts = parts.len();
    for (index, (part, trailing_blocks, trailing_line)) in parts.into_iter().enumerate() {
        body.push(part);
        if index + 1 < list_item_count || (has_trailing_semicolon && index + 1 == list_item_count) {
            body.push(text(";"));
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
            open_with_comments(open, after_open),
            indent_by(indent_levels, concat([hard_line(), concat(body)])),
            hard_line(),
            text(close),
        ]));
    }

    group(concat([
        open_with_comments(open, after_open),
        indent_by(
            indent_levels,
            concat([hard_line(), concat(body), text(close)]),
        ),
    ]))
}

fn open_with_comments(open: &'static str, comments: Vec<Doc>) -> Doc {
    if comments.is_empty() {
        text(open)
    } else {
        concat([text(open), text(" "), join(text(" "), comments)])
    }
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
        ',',
        0,
        false,
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

pub(crate) fn tabular_braced_comma_list_with_before_close_comments(
    list: FormattedList,
    cols: usize,
    rows: Vec<Vec<usize>>,
    rows_nested: Vec<bool>,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Option<Doc> {
    if cols != 2 || !list.has_only_before_close_structural_comments() {
        return None;
    }

    let FormattedList {
        after_open: _,
        items,
        before_close,
    } = list;
    let docs = items
        .into_iter()
        .map(FormattedListItem::into_doc)
        .collect::<Vec<_>>();
    if docs.is_empty() {
        return None;
    }

    let row_count = rows.len();
    let mut body = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            body.push(hard_line_without_break_parent());
        }

        let nested = rows_nested.get(row_index).copied().unwrap_or(false);
        let is_last_row = row_index + 1 == row_count;
        let row_items = row
            .iter()
            .map(|&index| docs.get(index).cloned())
            .collect::<Option<Vec<_>>>()?;
        body.push(tabular_braced_comma_row(
            row_items,
            cols,
            nested,
            is_last_row,
            has_trailing_comma || !before_close.is_empty(),
            policy,
        ));
    }
    body.extend(
        before_close
            .into_iter()
            .flat_map(|comment| [hard_line_without_break_parent(), comment]),
    );

    Some(concat([
        text("{"),
        indent(concat([hard_line_without_break_parent(), concat(body)])),
        hard_line_without_break_parent(),
        text("}"),
    ]))
}

fn tabular_braced_comma_row(
    mut items: Vec<Doc>,
    cols: usize,
    nested_row: bool,
    is_last_row: bool,
    has_trailing_comma: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    if items.is_empty() {
        return text("");
    }

    let row_indent = if cols == 1 || nested_row {
        0
    } else {
        policy.continuation_indent_levels()
    };

    if items.len() == 1 {
        let item = items.into_iter().next().expect("one item");
        let item = if is_last_row && !has_trailing_comma {
            item
        } else {
            concat([item, text(",")])
        };
        return if row_indent == 0 {
            item
        } else {
            indent_by(row_indent, item)
        };
    }

    let last = items.pop().expect("multiple items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));
    let last_doc = if is_last_row && !has_trailing_comma {
        last
    } else {
        concat([last, text(",")])
    };
    let body = fill(entries, last_doc);

    if row_indent == 0 {
        group(body)
    } else {
        group(indent_by(row_indent, body))
    }
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
            concat([hard_line(), join(concat([text(","), hard_line()]), pairs)]),
        ),
        text(close),
    ]))
}
