use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, empty_line, group, hard_line, indent, join, soft_line, text,
};

use crate::comments::{
    take_dangling_comment_docs, take_own_line_comment_docs_in_range,
    take_same_line_trailing_block_comment_docs_in_range,
    take_separator_leading_javadoc_comment_docs_in_range,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::layout as wrap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockLayoutOptions {
    pub collapse_if_empty: bool,
    pub preserve_leading_blank_line: bool,
    pub preserve_trailing_blank_line: bool,
}

impl Default for BlockLayoutOptions {
    fn default() -> Self {
        Self {
            collapse_if_empty: true,
            preserve_leading_blank_line: false,
            preserve_trailing_blank_line: false,
        }
    }
}

impl BlockLayoutOptions {
    pub(crate) const fn control_flow_body() -> Self {
        Self {
            collapse_if_empty: true,
            preserve_leading_blank_line: true,
            preserve_trailing_blank_line: false,
        }
    }

    pub(crate) const fn if_then_only_clause() -> Self {
        Self {
            collapse_if_empty: Self::control_flow_body().collapse_if_empty,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: false,
        }
    }

    pub(crate) const fn if_then_with_trailing_clauses() -> Self {
        Self {
            collapse_if_empty: false,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: true,
        }
    }

    pub(crate) const fn if_final_clause() -> Self {
        Self {
            collapse_if_empty: false,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: false,
        }
    }

    pub(crate) const fn do_body() -> Self {
        Self {
            collapse_if_empty: Self::control_flow_body().collapse_if_empty,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: true,
        }
    }

    pub(crate) const fn try_body_without_clauses() -> Self {
        Self {
            collapse_if_empty: Self::control_flow_body().collapse_if_empty,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: false,
        }
    }

    pub(crate) const fn try_body_with_clauses() -> Self {
        Self {
            collapse_if_empty: false,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: true,
        }
    }

    pub(crate) const fn try_final_clause_body() -> Self {
        Self {
            collapse_if_empty: false,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: false,
        }
    }

    pub(crate) const fn finally_body() -> Self {
        Self {
            collapse_if_empty: false,
            preserve_leading_blank_line: Self::control_flow_body().preserve_leading_blank_line,
            preserve_trailing_blank_line: false,
        }
    }
}

pub(crate) struct TypeBodyLayout {
    members: Vec<Doc>,
    separators: Vec<Doc>,
    before_close: Vec<Doc>,
    has_members: bool,
}

pub(crate) struct TypeBodyItemLayout {
    pub(crate) doc: Doc,
    pub(crate) range: TextRange,
    pub(crate) keep_adjacent_to_next: bool,
}

pub(crate) fn braced_type_body(body: TypeBodyLayout) -> Doc {
    let TypeBodyLayout {
        mut members,
        mut separators,
        before_close,
        has_members,
    } = body;
    if !has_members && members.is_empty() && before_close.is_empty() {
        return text("{}");
    }
    if !before_close.is_empty() {
        if !members.is_empty() {
            separators.push(empty_line());
        }
        members.extend(before_close);
    }
    if has_members {
        wrap::braced_block_with_separators(members, separators)
    } else {
        wrap::braced_block(members)
    }
}

pub(crate) fn type_body<Member>(
    body_range: TextRange,
    members: &[Member],
    context: &mut JavaFormatContext<'_>,
    range: impl Fn(&Member) -> Option<TextRange>,
    keep_adjacent: impl Fn(&Member, &Member) -> bool,
    mut format_member: impl FnMut(&Member, &mut JavaFormatContext<'_>) -> FormatResult<Doc>,
) -> FormatResult<TypeBodyLayout> {
    if members.is_empty() {
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, body_range)?,
            separators: Vec::new(),
            before_close: Vec::new(),
            has_members: false,
        });
    }

    let separators = type_body_member_separators(members, context, &range, keep_adjacent);
    let tail_start = members
        .iter()
        .filter_map(&range)
        .next_back()
        .unwrap_or(body_range);
    let before_close = take_body_tail_comment_docs(context, body_range, tail_start)?;
    let members = members
        .iter()
        .map(|member| format_member(member, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(TypeBodyLayout {
        members,
        separators,
        before_close,
        has_members: true,
    })
}

pub(crate) fn type_body_from_items_with_separators(
    body_range: TextRange,
    items: Vec<TypeBodyItemLayout>,
    separators: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<TypeBodyLayout> {
    if items.is_empty() {
        return Ok(TypeBodyLayout {
            members: take_dangling_comment_docs(context, body_range)?,
            separators: Vec::new(),
            before_close: Vec::new(),
            has_members: false,
        });
    }

    let tail_start = items.last().map_or(body_range, |item| item.range);
    let before_close = take_body_tail_comment_docs(context, body_range, tail_start)?;
    let members = items.into_iter().map(|item| item.doc).collect();
    Ok(TypeBodyLayout {
        members,
        separators,
        before_close,
        has_members: true,
    })
}

pub(crate) struct EnumBody {
    pub(crate) constants: Vec<Doc>,
    pub(crate) constant_separators: Vec<Doc>,
    pub(crate) semicolon: Option<Doc>,
    pub(crate) blank_line_before_members: bool,
    pub(crate) members: Vec<Doc>,
    pub(crate) before_close: Vec<Doc>,
}

pub(crate) fn enum_body(body: EnumBody) -> Doc {
    let EnumBody {
        constants,
        constant_separators,
        semicolon,
        blank_line_before_members,
        members,
        before_close,
    } = body;
    let constant_count = constants.len();
    let has_semicolon = semicolon.is_some();
    let member_count = members.len();
    let mut items = constants;
    items.extend(semicolon);
    items.extend(members);
    if before_close.is_empty() {
        return enum_body_items(
            items,
            constant_count,
            constant_separators,
            has_semicolon,
            blank_line_before_members,
            member_count,
        );
    }

    let item_count = items.len();
    items.push(join(hard_line(), before_close));
    let mut separators = enum_body_separators(
        constant_count,
        constant_separators,
        has_semicolon,
        blank_line_before_members,
        member_count,
    );
    if item_count > 0 {
        separators.push(empty_line());
    }
    wrap::braced_block_with_separators(items, separators)
}

fn enum_body_items(
    items: Vec<Doc>,
    constant_count: usize,
    constant_separators: Vec<Doc>,
    has_semicolon: bool,
    blank_line_before_members: bool,
    member_count: usize,
) -> Doc {
    if (!has_semicolon && !blank_line_before_members) || member_count == 0 {
        return wrap::braced_block(items);
    }

    wrap::braced_block_with_separators(
        items,
        enum_body_separators(
            constant_count,
            constant_separators,
            has_semicolon,
            blank_line_before_members,
            member_count,
        ),
    )
}

fn enum_body_separators(
    constant_count: usize,
    constant_separators: Vec<Doc>,
    has_semicolon: bool,
    blank_line_before_members: bool,
    member_count: usize,
) -> Vec<Doc> {
    let mut separators = Vec::new();
    separators.extend(constant_separators);
    if separators.len() < constant_count.saturating_sub(1) {
        separators
            .extend((separators.len()..constant_count.saturating_sub(1)).map(|_| hard_line()));
    }
    if has_semicolon && constant_count > 0 {
        separators.push(hard_line());
    }
    if member_count > 0 {
        separators.push(if blank_line_before_members {
            empty_line()
        } else {
            hard_line()
        });
        separators.extend((1..member_count).map(|_| hard_line()));
    }
    separators
}

pub(crate) fn statement_block<Statement>(
    container_range: TextRange,
    statements: &[Statement],
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
    range: impl Fn(&Statement) -> Option<TextRange>,
    format_statement: impl FnMut(&Statement, &mut JavaFormatContext<'_>) -> FormatResult<Doc>,
) -> FormatResult<Doc> {
    statement_block_with_opening_comments(
        container_range,
        statements,
        Vec::new(),
        context,
        options,
        range,
        format_statement,
    )
}

pub(crate) fn statement_block_with_opening_comments<Statement>(
    container_range: TextRange,
    statements: &[Statement],
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
    range: impl Fn(&Statement) -> Option<TextRange>,
    mut format_statement: impl FnMut(&Statement, &mut JavaFormatContext<'_>) -> FormatResult<Doc>,
) -> FormatResult<Doc> {
    if statements.is_empty() {
        let dangling = take_dangling_comment_docs(context, container_range)?;
        return Ok(empty_braced_block(
            context,
            container_range,
            options,
            opening_comments,
            dangling,
        ));
    }

    let first_range = statements.iter().find_map(&range);
    let last_range = statements.iter().filter_map(&range).next_back();
    let open_brace = container_range.start();
    let close_brace = container_range.end();
    let open_brace_end = (open_brace.get() + 1).into();
    let close_brace_start = close_brace.get().saturating_sub(1).into();
    let leading_blank = options.preserve_leading_blank_line
        && first_range.is_some_and(|first| {
            context.has_blank_line_between(
                TextRange::new(open_brace, open_brace_end),
                TextRange::new(first.start(), first.start()),
            )
        });
    let trailing_blank = options.preserve_trailing_blank_line
        && last_range.is_some_and(|last| {
            context.has_blank_line_between(last, TextRange::new(close_brace_start, close_brace))
        });

    let mut separators = statements
        .windows(2)
        .map(|window| {
            let left = range(&window[0]);
            let right = range(&window[1]);
            if let (Some(left), Some(right)) = (left, right)
                && context.has_blank_line_before(left, right)
            {
                return empty_line();
            }
            hard_line()
        })
        .collect::<Vec<_>>();

    let mut statement_docs = statements
        .iter()
        .map(|statement| format_statement(statement, context))
        .collect::<FormatResult<Vec<_>>>()?;
    if let Some(last_range) = statements.iter().filter_map(&range).next_back() {
        let tail = take_own_line_comment_docs_in_range(
            context,
            TextRange::new(last_range.end(), container_range.end()),
        )?;
        if !tail.is_empty() {
            separators.push(hard_line());
            statement_docs.push(join(hard_line(), tail));
        }
    }

    Ok(braced_body_with_opening_comments(
        opening_comments,
        statement_docs,
        separators,
        wrap::BracedBodyLayout {
            leading_blank_line: leading_blank,
            trailing_blank_line: trailing_blank,
        },
    ))
}

fn empty_braced_block(
    context: &JavaFormatContext<'_>,
    container_range: TextRange,
    options: BlockLayoutOptions,
    opening_comments: Vec<Doc>,
    dangling: Vec<Doc>,
) -> Doc {
    if options.collapse_if_empty && opening_comments.is_empty() && dangling.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    let open_brace = container_range.start();
    let close_brace = container_range.end();
    let open_brace_end = (open_brace.get() + 1).into();
    let close_brace_start = close_brace.get().saturating_sub(1).into();
    let interior_blank = (options.preserve_leading_blank_line
        || options.preserve_trailing_blank_line)
        && context.has_blank_line_between(
            TextRange::new(open_brace, open_brace_end),
            TextRange::new(close_brace_start, close_brace),
        );

    braced_body_with_opening_comments(
        opening_comments,
        dangling,
        Vec::new(),
        wrap::BracedBodyLayout {
            leading_blank_line: options.preserve_leading_blank_line && interior_blank,
            trailing_blank_line: options.preserve_trailing_blank_line && interior_blank,
        },
    )
}

pub(crate) fn constructor_body_with_opening_comments(
    body_range: TextRange,
    invocation: Option<Doc>,
    statements: Vec<Doc>,
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut items = Vec::new();
    items.extend(invocation);
    items.extend(statements);

    if opening_comments.is_empty() {
        if items.is_empty() {
            return Ok(wrap::braced_block(take_dangling_comment_docs(
                context, body_range,
            )?));
        }
        return Ok(wrap::braced_block(items));
    }

    if items.is_empty() {
        return Ok(braced_body_with_opening_comments(
            opening_comments,
            take_dangling_comment_docs(context, body_range)?,
            Vec::new(),
            wrap::BracedBodyLayout::default(),
        ));
    }

    Ok(braced_body_with_opening_comments(
        opening_comments,
        items,
        Vec::new(),
        wrap::BracedBodyLayout::default(),
    ))
}

fn braced_body_with_opening_comments(
    opening_comments: Vec<Doc>,
    items: Vec<Doc>,
    separators: Vec<Doc>,
    layout: wrap::BracedBodyLayout,
) -> Doc {
    if opening_comments.is_empty() {
        return wrap::braced_body(items, separators, layout);
    }

    let wrap::BracedBodyLayout {
        leading_blank_line,
        trailing_blank_line,
    } = layout;
    let mut body = Vec::new();
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        body.push(first);
    }
    for (separator, item) in separators.into_iter().zip(items) {
        body.push(separator);
        body.push(item);
    }

    let mut parts = vec![text("{"), concat(opening_comments)];
    if leading_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    if !body.is_empty() {
        parts.push(indent(concat([hard_line(), concat(body)])));
    }
    if trailing_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    parts.push(hard_line());
    parts.push(text("}"));
    concat(parts)
}

pub(crate) fn take_body_tail_comment_docs(
    context: &mut JavaFormatContext<'_>,
    body_range: TextRange,
    tail_start: TextRange,
) -> FormatResult<Vec<Doc>> {
    take_own_line_comment_docs_in_range(context, TextRange::new(tail_start.end(), body_range.end()))
}

fn type_body_member_separators<Member>(
    members: &[Member],
    context: &mut JavaFormatContext<'_>,
    range: impl Fn(&Member) -> Option<TextRange>,
    keep_adjacent: impl Fn(&Member, &Member) -> bool,
) -> Vec<Doc> {
    members
        .windows(2)
        .map(|window| {
            let left = range(&window[0]);
            let right = range(&window[1]);
            let separator = if let (Some(left), Some(right)) = (left, right)
                && (context.has_blank_line_before(left, right)
                    || context.has_leading_comments_before(right))
            {
                empty_line()
            } else if keep_adjacent(&window[0], &window[1]) {
                hard_line()
            } else {
                empty_line()
            };
            let (Some(left), Some(right)) = (left, right) else {
                return separator;
            };
            let boundary = TextRange::new(left.end(), right.start());
            let trailing_blocks =
                take_same_line_trailing_block_comment_docs_in_range(context, left, boundary);
            let leading_javadocs =
                take_separator_leading_javadoc_comment_docs_in_range(context, boundary, right);
            let mut separator_parts = Vec::new();
            if !trailing_blocks.is_empty() {
                separator_parts.extend([text(" "), join(hard_line(), trailing_blocks)]);
            }
            separator_parts.push(separator);
            if !leading_javadocs.is_empty() {
                separator_parts.extend([join(hard_line(), leading_javadocs), hard_line()]);
            }
            concat(separator_parts)
        })
        .collect()
}

pub(crate) fn type_body_item_separators(
    members: &[TypeBodyItemLayout],
    context: &mut JavaFormatContext<'_>,
) -> Vec<Doc> {
    members
        .windows(2)
        .map(|window| {
            let left = window[0].range;
            let right = window[1].range;
            let separator = if context.has_blank_line_before(left, right)
                || context.has_leading_comments_before(right)
            {
                empty_line()
            } else if window[0].keep_adjacent_to_next {
                hard_line()
            } else {
                empty_line()
            };
            let boundary = TextRange::new(left.end(), right.start());
            let trailing_blocks =
                take_same_line_trailing_block_comment_docs_in_range(context, left, boundary);
            let leading_javadocs =
                take_separator_leading_javadoc_comment_docs_in_range(context, boundary, right);
            let mut separator_parts = Vec::new();
            if !trailing_blocks.is_empty() {
                separator_parts.extend([text(" "), join(hard_line(), trailing_blocks)]);
            }
            separator_parts.push(separator);
            if !leading_javadocs.is_empty() {
                separator_parts.extend([join(hard_line(), leading_javadocs), hard_line()]);
            }
            concat(separator_parts)
        })
        .collect()
}
