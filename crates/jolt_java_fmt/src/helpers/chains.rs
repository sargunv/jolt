use jolt_fmt_ir::{Doc, concat, group, indent, soft_line};

pub(crate) fn member_chain(
    root: Doc,
    suffixes: Vec<Doc>,
    keep_first_suffix_with_root: bool,
) -> Doc {
    if suffixes.is_empty() {
        return root;
    }

    let mut suffixes = suffixes.into_iter();
    let head = if keep_first_suffix_with_root {
        suffixes
            .next()
            .map_or(root.clone(), |suffix| concat([root, suffix]))
    } else {
        root
    };
    let rest = suffixes
        .map(|suffix| concat([soft_line(), suffix]))
        .collect::<Vec<_>>();

    if rest.is_empty() {
        return group(head);
    }

    group(concat([head, indent(concat(rest))]))
}
