use jolt_fmt_ir::{
    Doc, best_fitting, concat, fill, fill_entry, group, hard_line, indent_by, soft_line, text,
};

use crate::analyzers::chains::{
    Chain, ChainMember, ChainMemberKind, ChainRole, classified_prefix_member_end_index,
    single_invocation_coalesced_prefix_len,
};
use crate::policy::JavaFormatPolicy;

pub(crate) fn selector_chain(chain: Chain, policy: JavaFormatPolicy, role: ChainRole) -> Doc {
    let groups = chain.groups();
    let Chain {
        base,
        base_trailing_comments,
        mut members,
        metadata,
        ..
    } = chain;

    if members.is_empty() {
        return append_trailing_comments(base, base_trailing_comments);
    }

    if has_trailing_comments(&base_trailing_comments, &members) {
        return commented_selector_chain(base, base_trailing_comments, members, policy);
    }

    let flat_chain =
        concat(std::iter::once(base.clone()).chain(members.iter().map(prefixed_member_doc)));

    if groups.all_fields(members.len()) {
        return field_selector_chain(base, members, policy, role);
    }

    let field_prefix_len = groups.field_prefix_len();
    if field_prefix_len >= 2
        && crate::analyzers::type_names::type_name_prefix_member_end_index(
            metadata.base.simple_name.as_deref(),
            &members,
        )
        .is_none()
    {
        let remaining = members.split_off(field_prefix_len);
        let receiver_width = metadata.base.source_width
            + members
                .iter()
                .map(|member| member.source_width + 1)
                .sum::<usize>();
        let receiver = field_selector_chain(base, members, policy, role);
        return selector_chain(
            Chain::with_base_metadata(
                receiver,
                remaining,
                crate::analyzers::chains::BaseMetadata::complex(receiver_width),
            )
            .with_tail_range(None),
            policy,
            role,
        );
    }

    let leading_type_argument_call_len = groups.leading_type_argument_call_len();
    if leading_type_argument_call_len > 0 {
        let broken_chain = selector_chain_with_cohesive_head(
            base,
            members,
            leading_type_argument_call_len,
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    if keeps_simple_receiver_call_head(&groups, &metadata, policy, role) {
        let broken_chain = selector_chain_with_cohesive_head(base, members, 1, policy);
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    let coalesced_prefix_len = single_invocation_coalesced_prefix_len(&members);
    if coalesced_prefix_len > 0 {
        let broken_chain =
            selector_chain_with_cohesive_head(base, members, coalesced_prefix_len, policy);
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    if let Some(stream_end) =
        crate::analyzers::chains::stream_suffix_prefix_member_end_index(&members)
        && stream_end + 1 < members.len()
    {
        let broken_chain = selector_chain_with_cohesive_head(base, members, stream_end + 1, policy);
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    if metadata.base.forces_break_before_first_selector {
        let broken_chain = break_before_first_selector_chain(base, members, false, policy);
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    if let Some(prefix_end) = classified_prefix_member_end_index(&metadata.base, &members)
        && prefix_end + 1 < members.len()
        && members
            .get(prefix_end)
            .is_some_and(super::super::analyzers::chains::ChainMember::is_call)
    {
        let broken_chain = selector_chain_with_prefix_group(
            base,
            members,
            prefix_end,
            metadata.base.source_width,
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    if metadata.call_count >= 10 || breaks_before_first_selector(&metadata, policy, role) {
        let broken_chain = break_before_first_selector_chain(base, members, true, policy);
        return chain_layout_preference(flat_chain, broken_chain, role);
    }

    let first_member = members.remove(0);
    let base =
        append_leading_array_accesses(concat([base, member_doc(first_member)]), &mut members);
    if members.is_empty() {
        return chain_layout_preference(flat_chain, base, role);
    }

    let broken_chain = group(concat([
        base,
        continuation_indent(concat(member_docs_after_line(soft_line(), members)), policy),
    ]));
    chain_layout_preference(flat_chain, broken_chain, role)
}

/// Nested chain arguments are embedded in outer chain member docs. Avoid
/// `best_fitting` there: each fit trial walks the full subtree and deeply
/// nested call trees (e.g. `B24909927.java`) blow up exponentially.
fn chain_layout_preference(flat: Doc, broken: Doc, role: ChainRole) -> Doc {
    if matches!(role, ChainRole::NestedArgument) {
        broken
    } else {
        best_fitting(flat, [broken])
    }
}

fn keeps_simple_receiver_call_head(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: &crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
    role: ChainRole,
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

    if policy.selector_chain_preserves_nested_argument_head(role) {
        return true;
    }

    if metadata.first_call_argument_count == 0 {
        return !policy.selector_chain_breaks_before_first_selector_for_role(role);
    }

    metadata.base.source_width <= 3 && metadata.first_call_argument_count <= 3
}

fn selector_chain_with_cohesive_head(
    base: Doc,
    mut members: Vec<ChainMember>,
    head_len: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    let head_len = head_len_with_array_access_suffix(&members, head_len);
    let tail = members.split_off(head_len);
    let head = concat(std::iter::once(base).chain(members.into_iter().map(member_doc)));

    if tail.is_empty() {
        return head;
    }

    group(concat([
        head,
        continuation_indent(concat(member_docs_after_line(soft_line(), tail)), policy),
    ]))
}

fn breaks_before_first_selector(
    metadata: &crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
    role: ChainRole,
) -> bool {
    if policy.selector_chain_role_breaks_before_first_selector(
        role,
        metadata.base.kind,
        metadata.first_member_is_call,
    ) {
        return true;
    }

    if !policy.selector_chain_breaks_before_first_selector_for_role(role) {
        return false;
    }

    if metadata.base.is_complex && metadata.call_count > 0 {
        return true;
    }

    if metadata.base.forces_break_before_first_selector && metadata.first_member_is_call {
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

fn selector_chain_with_prefix_group(
    base: Doc,
    mut members: Vec<ChainMember>,
    prefix_end: usize,
    base_width: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    let prefix_end = head_len_with_array_access_suffix(&members, prefix_end + 1) - 1;
    let tail = members.split_off(prefix_end + 1);
    let prefix_members = members;
    let min_length = policy.selector_chain_min_receiver_length_before_break();
    let prefix = dot_fill_selector_segments(base, prefix_members, base_width, min_length, policy);

    if tail.is_empty() {
        return prefix;
    }

    group(concat([
        prefix,
        continuation_indent(concat(member_docs_after_line(soft_line(), tail)), policy),
    ]))
}

fn dot_fill_selector_segments(
    base: Doc,
    members: Vec<ChainMember>,
    mut accumulated_width: usize,
    min_length: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    if members.is_empty() {
        return base;
    }

    if members.len() == 1 {
        let member = members.into_iter().next().expect("one prefix member");
        let break_before_dot = accumulated_width > min_length;
        return if break_before_dot {
            group(concat([
                base,
                continuation_indent(concat([soft_line(), member_doc(member)]), policy),
            ]))
        } else {
            concat([base, member_doc(member)])
        };
    }

    let mut segments = members
        .into_iter()
        .map(|member| (member.source_width, member.doc))
        .collect::<Vec<_>>();
    let (last_width, last) = segments
        .pop()
        .expect("multiple prefix members checked above");
    let entries = std::iter::once((accumulated_width, base))
        .chain(segments)
        .map(|(width, segment)| {
            accumulated_width += width + 1;
            let separator = if accumulated_width > min_length {
                concat([soft_line(), text(".")])
            } else {
                text(".")
            };
            fill_entry(segment, separator)
        });
    let _ = last_width;

    group(continuation_indent(
        fill(entries, concat([text("."), last])),
        policy,
    ))
}

fn break_before_first_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    force_break: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    let separator = if force_break {
        hard_line()
    } else {
        soft_line()
    };
    group(concat([
        base,
        continuation_indent(concat(member_docs_after_line(separator, members)), policy),
    ]))
}

fn commented_selector_chain(
    base: Doc,
    base_trailing_comments: Vec<Doc>,
    members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
) -> Doc {
    group(concat([
        append_trailing_comments(base, base_trailing_comments),
        continuation_indent(
            concat(members.into_iter().map(|member| {
                let kind = member.kind;
                let doc = append_trailing_comments(member.doc, member.trailing_comments);
                concat([hard_line(), member_doc_with_kind(kind, doc)])
            })),
            policy,
        ),
    ]))
}

fn head_len_with_array_access_suffix(members: &[ChainMember], head_len: usize) -> usize {
    let mut len = head_len;
    while members
        .get(len)
        .is_some_and(|member| matches!(member.kind, ChainMemberKind::ArrayAccess))
    {
        len += 1;
    }
    len
}

fn append_leading_array_accesses(mut base: Doc, members: &mut Vec<ChainMember>) -> Doc {
    while members
        .first()
        .is_some_and(|member| matches!(member.kind, ChainMemberKind::ArrayAccess))
    {
        let member = members.remove(0);
        base = concat([base, member.doc]);
    }
    base
}

fn field_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
    role: ChainRole,
) -> Doc {
    let mut segments = members
        .into_iter()
        .map(|member| member.doc)
        .collect::<Vec<_>>();
    let last = segments.pop().expect("non-empty members checked above");
    if segments.is_empty() {
        let flat = concat([base.clone(), text("."), last.clone()]);
        let broken = group(concat([
            base,
            continuation_indent(concat([soft_line(), text("."), last]), policy),
        ]));
        return chain_layout_preference(flat, broken, role);
    }

    let entries = std::iter::once(base)
        .chain(segments)
        .map(|segment| fill_entry(segment, concat([soft_line(), text(".")])));

    group(continuation_indent(fill(entries, last), policy))
}

fn has_trailing_comments(base_trailing_comments: &[Doc], members: &[ChainMember]) -> bool {
    !base_trailing_comments.is_empty()
        || members
            .iter()
            .any(|member| !member.trailing_comments.is_empty())
}

fn append_trailing_comments(doc: Doc, comments: Vec<Doc>) -> Doc {
    if comments.is_empty() {
        return doc;
    }

    concat(
        std::iter::once(doc).chain(
            comments
                .into_iter()
                .flat_map(|comment| [text(" "), comment]),
        ),
    )
}

fn prefixed_member_doc(member: &ChainMember) -> Doc {
    match member.kind {
        ChainMemberKind::Field | ChainMemberKind::Call { .. } => {
            concat([text("."), member.doc.clone()])
        }
        ChainMemberKind::ArrayAccess => member.doc.clone(),
    }
}

fn member_doc(member: ChainMember) -> Doc {
    member_doc_with_kind(member.kind, member.doc)
}

fn member_doc_with_kind(kind: ChainMemberKind, doc: Doc) -> Doc {
    match kind {
        ChainMemberKind::Field | ChainMemberKind::Call { .. } => concat([text("."), doc]),
        ChainMemberKind::ArrayAccess => doc,
    }
}

fn member_docs_after_line(line: Doc, members: Vec<ChainMember>) -> Vec<Doc> {
    let mut docs = Vec::new();
    for member in members {
        match member.kind {
            ChainMemberKind::Field | ChainMemberKind::Call { .. } => {
                docs.push(concat([line.clone(), text("."), member.doc]));
            }
            ChainMemberKind::ArrayAccess => {
                if let Some(last) = docs.last_mut() {
                    *last = concat([last.clone(), member.doc]);
                } else {
                    docs.push(concat([line.clone(), member.doc]));
                }
            }
        }
    }
    docs
}

fn continuation_indent(doc: Doc, policy: JavaFormatPolicy) -> Doc {
    indent_by(policy.continuation_indent_levels(), doc)
}
