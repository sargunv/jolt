use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, join, text};

use crate::comments::{
    take_dangling_comment_docs, take_own_line_comment_docs_in_range,
    take_same_line_trailing_block_comment_docs_in_range,
    take_separator_leading_javadoc_comment_docs_in_range,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::layout as wrap;

pub(crate) struct TypeBodyLayout {
    members: Vec<Doc>,
    separators: Vec<Doc>,
    before_close: Vec<Doc>,
    has_members: bool,
}

pub(crate) fn braced_type_body(body: TypeBodyLayout) -> Doc {
    let TypeBodyLayout {
        mut members,
        mut separators,
        before_close,
        has_members,
    } = body;
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

pub(crate) struct EnumBody {
    pub(crate) constants: Vec<Doc>,
    pub(crate) semicolon: Option<Doc>,
    pub(crate) members: Vec<Doc>,
    pub(crate) before_close: Vec<Doc>,
}

pub(crate) fn enum_body(body: EnumBody) -> Doc {
    let EnumBody {
        constants,
        semicolon,
        members,
        before_close,
    } = body;
    let mut items = constants;
    items.extend(semicolon);
    items.extend(members);
    if before_close.is_empty() {
        return wrap::braced_block(items);
    }

    let item_count = items.len();
    items.push(join(hard_line(), before_close));
    let mut separators = vec![hard_line(); item_count.saturating_sub(1)];
    if item_count > 0 {
        separators.push(empty_line());
    }
    wrap::braced_block_with_separators(items, separators)
}

pub(crate) fn statement_block<Statement>(
    container_range: TextRange,
    statements: &[Statement],
    context: &mut JavaFormatContext<'_>,
    range: impl Fn(&Statement) -> Option<TextRange>,
    mut format_statement: impl FnMut(&Statement, &mut JavaFormatContext<'_>) -> FormatResult<Doc>,
) -> FormatResult<Doc> {
    if statements.is_empty() {
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context,
            container_range,
        )?));
    }

    let mut separators = statements
        .windows(2)
        .map(|window| {
            let left = range(&window[0]);
            let right = range(&window[1]);
            if let (Some(left), Some(right)) = (left, right)
                && context.has_blank_line_between(left, right)
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

    Ok(wrap::braced_block_with_separators(
        statement_docs,
        separators,
    ))
}

pub(crate) fn constructor_body(
    body_range: TextRange,
    invocation: Option<Doc>,
    statements: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut items = Vec::new();
    items.extend(invocation);
    items.extend(statements);

    if items.is_empty() {
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context, body_range,
        )?));
    }

    Ok(wrap::braced_block(items))
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
                && context.has_blank_line_between(left, right)
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
