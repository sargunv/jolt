use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, indent_by, join, line, soft_line, text};

use crate::layout as wrap;
use crate::policy::JavaFormatPolicy;

pub(crate) fn switch_construct(selector: Doc, block: Doc) -> Doc {
    concat([
        text("switch "),
        wrap::parenthesized_expression(selector),
        text(" "),
        block,
    ])
}

pub(crate) fn switch_block(
    mut items: Vec<Doc>,
    mut separators: Vec<Doc>,
    before_first: Vec<Doc>,
    after_last: Vec<Doc>,
) -> Doc {
    if items.is_empty() {
        return wrap::braced_block(Vec::<Doc>::new());
    }

    if let Some(first) = items.first_mut()
        && !before_first.is_empty()
    {
        *first = concat([join(hard_line(), before_first), hard_line(), first.clone()]);
    }

    if !after_last.is_empty() {
        separators.push(hard_line());
        items.push(join(hard_line(), after_last));
    }

    wrap::braced_block_with_separators(items, separators)
}

pub(crate) fn switch_block_item_separator(boundary_comments: Vec<Doc>) -> Doc {
    if boundary_comments.is_empty() {
        hard_line()
    } else {
        concat([
            hard_line(),
            join(hard_line(), boundary_comments),
            hard_line(),
        ])
    }
}

pub(crate) fn switch_statement_group(
    labels: Vec<Doc>,
    body_comments: Vec<Doc>,
    statements: Vec<Doc>,
) -> Doc {
    if statements.is_empty() {
        return join(hard_line(), labels);
    }

    let statements = if body_comments.is_empty() {
        join(hard_line(), statements)
    } else {
        concat([
            join(hard_line(), body_comments),
            hard_line(),
            join(hard_line(), statements),
        ])
    };

    concat([
        join(hard_line(), labels),
        indent(concat([hard_line(), statements])),
    ])
}

pub(crate) fn case_label(items: impl IntoIterator<Item = Doc>, policy: JavaFormatPolicy) -> Doc {
    concat([text("case "), case_label_item_list(items, policy)])
}

pub(crate) fn guarded_pattern(base: Doc, guard: Doc, policy: JavaFormatPolicy) -> Doc {
    group(concat([
        base,
        indent_by(
            policy.continuation_indent_levels(),
            concat([line(), text("when "), guard]),
        ),
    ]))
}

pub(crate) fn switch_rule(
    label: Doc,
    arrow: Doc,
    body: Doc,
    body_comments: Vec<Doc>,
    body_is_block: bool,
    arrow_has_trailing_comment: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    let has_body_comments = !body_comments.is_empty();
    let body = if body_comments.is_empty() {
        body
    } else {
        concat([join(hard_line(), body_comments), hard_line(), body])
    };

    if body_is_block && !has_body_comments && !arrow_has_trailing_comment {
        return concat([label, arrow, text(" "), body]);
    }

    let separator = if arrow_has_trailing_comment {
        hard_line()
    } else {
        line()
    };
    group(concat([
        label,
        arrow,
        indent_by(
            policy.continuation_indent_levels(),
            concat([separator, body]),
        ),
    ]))
}

pub(crate) fn switch_record_pattern_components(
    ty: Doc,
    components: impl IntoIterator<Item = Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let mut components = components.into_iter().collect::<Vec<_>>();
    if components.is_empty() {
        return concat([ty, text("()")]);
    }
    if components.len() == 1 {
        let component = components
            .pop()
            .expect("single component length checked above");
        return concat([ty, text("("), component, text(")")]);
    }

    let last = components
        .pop()
        .expect("non-empty components checked above");
    let mut body = components
        .into_iter()
        .flat_map(|component| [component, text(","), line()])
        .collect::<Vec<_>>();
    body.push(last);
    body.push(text(")"));

    group(concat([
        ty,
        text("("),
        indent_by(
            policy.switch_record_pattern_component_indent_levels(),
            concat([soft_line(), concat(body)]),
        ),
    ]))
}

fn case_label_item_list(items: impl IntoIterator<Item = Doc>, policy: JavaFormatPolicy) -> Doc {
    let mut items = items.into_iter();
    let Some(first) = items.next() else {
        return text("");
    };

    group(concat([
        first,
        indent_by(
            policy.continuation_indent_levels(),
            concat(items.map(|item| concat([text(","), line(), item]))),
        ),
    ]))
}
