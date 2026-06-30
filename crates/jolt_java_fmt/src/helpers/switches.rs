use jolt_fmt_ir::{Doc, concat, group, hard_line, indent_by, line, soft_line, text};

use crate::policy::JavaFormatPolicy;

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

pub(crate) fn switch_rule_with_block(label: Doc, arrow: Doc, body: Doc) -> Doc {
    concat([label, arrow, text(" "), body])
}

pub(crate) fn switch_rule_with_expression(
    label: Doc,
    arrow: Doc,
    body: Doc,
    policy: JavaFormatPolicy,
    force_break_after_arrow: bool,
) -> Doc {
    let separator = if force_break_after_arrow {
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
