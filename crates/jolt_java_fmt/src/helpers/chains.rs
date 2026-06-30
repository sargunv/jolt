use jolt_fmt_ir::{
    Doc, best_fitting, concat, fill, fill_entry, group, hard_line, indent_by, soft_line, text,
};

use crate::analyzers::chains::{Chain, ChainMember};
use crate::policy::JavaFormatPolicy;

const CONTINUATION_INDENT_LEVELS: u16 = 2;

pub(crate) fn selector_chain(chain: Chain, policy: JavaFormatPolicy) -> Doc {
    let groups = chain.groups();
    let Chain {
        base,
        mut members,
        metadata,
    } = chain;

    if members.is_empty() {
        return base;
    }

    if groups.all_fields(members.len()) {
        return field_selector_chain(base, members);
    }

    let field_prefix_len = groups.field_prefix_len();
    if field_prefix_len >= 2 {
        let remaining = members.split_off(field_prefix_len);
        let receiver_width = metadata.base.source_width
            + members
                .iter()
                .map(|member| member.source_width + 1)
                .sum::<usize>();
        let receiver = field_selector_chain(base, members);
        return selector_chain(
            Chain::with_base_metadata(
                receiver,
                remaining,
                crate::analyzers::chains::BaseMetadata::complex(receiver_width),
            ),
            policy,
        );
    }

    let flat_chain = concat(
        std::iter::once(base.clone()).chain(
            members
                .iter()
                .flat_map(|member| [text("."), member.doc.clone()]),
        ),
    );

    let leading_type_argument_call_len = groups.leading_type_argument_call_len();
    if leading_type_argument_call_len > 0 {
        let broken_chain =
            selector_chain_with_cohesive_head(base, members, leading_type_argument_call_len);
        return best_fitting(flat_chain, [broken_chain]);
    }

    if keeps_simple_receiver_call_head(&groups, metadata, policy) {
        let broken_chain = selector_chain_with_cohesive_head(base, members, 1);
        return best_fitting(flat_chain, [broken_chain]);
    }

    if metadata.call_count >= 10 || breaks_before_first_selector(metadata, policy) {
        let broken_chain = break_before_first_selector_chain(base, members, true);
        return best_fitting(flat_chain, [broken_chain]);
    }

    let first_member = members.remove(0);
    let base = concat([base, text("."), first_member.doc]);
    if members.is_empty() {
        return best_fitting(flat_chain, [base]);
    }

    let broken_chain = group(concat([
        base,
        continuation_indent(concat(
            members
                .into_iter()
                .map(|member| concat([soft_line(), text("."), member.doc])),
        )),
    ]));
    best_fitting(flat_chain, [broken_chain])
}

fn keeps_simple_receiver_call_head(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
) -> bool {
    if !(groups.starts_with_call_run()
        && !metadata.base.is_complex
        && metadata.base.call_count == 0
        && metadata.base.source_width > 0
        && metadata.first_member_is_call
        && metadata.call_count >= 2)
    {
        return false;
    }

    if metadata.first_call_argument_count == 0 {
        return !policy.selector_chain_breaks_before_first_selector();
    }

    metadata.base.source_width <= 3 && metadata.first_call_argument_count <= 3
}

fn selector_chain_with_cohesive_head(
    base: Doc,
    mut members: Vec<ChainMember>,
    head_len: usize,
) -> Doc {
    let tail = members.split_off(head_len);
    let head = concat(
        std::iter::once(base).chain(
            members
                .into_iter()
                .flat_map(|member| [text("."), member.doc]),
        ),
    );

    if tail.is_empty() {
        return head;
    }

    group(concat([
        head,
        continuation_indent(concat(
            tail.into_iter()
                .map(|member| concat([soft_line(), text("."), member.doc])),
        )),
    ]))
}

fn breaks_before_first_selector(
    metadata: crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
) -> bool {
    if !policy.selector_chain_breaks_before_first_selector() {
        return false;
    }

    if metadata.base.is_complex && metadata.call_count > 0 {
        return true;
    }

    if metadata.base.call_count > 0
        && metadata.first_member_is_call
        && metadata.first_call_argument_count >= 3
    {
        return true;
    }

    metadata.first_member_is_call
        && metadata.total_call_count >= 2
        && metadata.base.source_width + metadata.first_member_width
            >= policy.selector_chain_long_receiver_width()
}

fn break_before_first_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    force_break: bool,
) -> Doc {
    let separator = if force_break {
        hard_line()
    } else {
        soft_line()
    };
    group(concat([
        base,
        continuation_indent(concat(
            members
                .into_iter()
                .map(|member| concat([separator.clone(), text("."), member.doc])),
        )),
    ]))
}

fn field_selector_chain(base: Doc, members: Vec<ChainMember>) -> Doc {
    let mut segments = members
        .into_iter()
        .map(|member| member.doc)
        .collect::<Vec<_>>();
    let last = segments.pop().expect("non-empty members checked above");
    if segments.is_empty() {
        let flat = concat([base.clone(), text("."), last.clone()]);
        let broken = group(concat([
            base,
            continuation_indent(concat([soft_line(), text("."), last])),
        ]));
        return best_fitting(flat, [broken]);
    }

    let entries = std::iter::once(base)
        .chain(segments)
        .map(|segment| fill_entry(segment, concat([soft_line(), text(".")])));

    group(continuation_indent(fill(entries, last)))
}

fn continuation_indent(doc: Doc) -> Doc {
    indent_by(CONTINUATION_INDENT_LEVELS, doc)
}
