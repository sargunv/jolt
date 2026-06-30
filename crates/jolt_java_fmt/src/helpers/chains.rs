use jolt_fmt_ir::{
    Doc, best_fitting, concat, fill, fill_entry, group, hard_line, if_group_breaks, indent_by,
    soft_line, text,
};

use crate::analyzers::chains::{
    BaseMetadata, Chain, ChainBaseKind, ChainMember, ChainMemberKind, ChainMetadata, ChainRole,
    classified_prefix_member_end_index, is_gjf_log_statement_chain,
    single_invocation_coalesced_prefix_len,
};
use crate::helpers::lists::SELECTOR_TYPE_ARGUMENTS_GROUP_ID;
use crate::policy::JavaFormatPolicy;

pub(crate) fn explicit_type_argument_invocation_selector(
    type_arguments: Option<Doc>,
    name: Doc,
    arguments: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    let Some(type_arguments) = type_arguments else {
        return concat([name, arguments]);
    };

    concat([
        type_arguments,
        if_group_breaks(
            SELECTOR_TYPE_ARGUMENTS_GROUP_ID,
            indent_by(
                policy.type_argument_indent_levels(),
                concat([hard_line(), name.clone(), arguments.clone()]),
            ),
            concat([name, arguments]),
        ),
    ])
}

pub(crate) fn explicit_type_argument_invocation_selector_head(
    type_arguments: Option<Doc>,
    name: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    let Some(type_arguments) = type_arguments else {
        return name;
    };

    concat([
        type_arguments,
        if_group_breaks(
            SELECTOR_TYPE_ARGUMENTS_GROUP_ID,
            indent_by(
                policy.type_argument_indent_levels(),
                concat([hard_line(), name.clone()]),
            ),
            name,
        ),
    ])
}

pub(crate) fn explicit_type_argument_invocation_selector_after_chain_break(
    type_arguments: Option<Doc>,
    name: Doc,
    arguments: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    let Some(type_arguments) = type_arguments else {
        return concat([name, arguments]);
    };

    concat([
        type_arguments,
        if_group_breaks(
            SELECTOR_TYPE_ARGUMENTS_GROUP_ID,
            indent_by(
                policy.selector_invocation_head_indent_levels(),
                concat([hard_line(), name.clone(), arguments.clone()]),
            ),
            concat([name, arguments]),
        ),
    ])
}

pub(crate) fn selector_chain(chain: Chain, policy: JavaFormatPolicy, role: ChainRole) -> Doc {
    let groups = chain.groups();
    let Chain {
        base,
        base_trailing_comments,
        members,
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

    if is_gjf_log_statement_chain(&metadata.base, &members) {
        let broken_chain =
            gjf_log_statement_chain(base, base_trailing_comments, members, &metadata, policy);
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if matches!(metadata.base.kind, ChainBaseKind::PrimaryExpression)
        && members
            .first()
            .is_some_and(|member| matches!(member.kind, ChainMemberKind::Field))
        && !members
            .first()
            .is_some_and(|member| member.has_type_arguments)
    {
        let broken_chain = visit_regular_dot_chain_after_primary_receiver(base, members, policy);
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if groups.all_fields(members.len()) {
        if is_nested_argument_role(role) {
            return field_selector_chain(base, members, policy, role);
        }
        let min_length = policy.selector_chain_min_receiver_length_before_break();
        let broken_chain = field_dot_fill_selector_segments(
            base,
            members,
            metadata.base.source_width,
            min_length,
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    let leading_type_argument_call_len = groups.leading_type_argument_call_len();
    if leading_type_argument_call_len > 0
        && matches!(role, ChainRole::Default)
        && metadata.base.source_width >= policy.selector_chain_long_receiver_width()
        && breaks_long_simple_receiver_call_head(&metadata, &members, policy)
    {
        let broken_chain = break_before_first_selector_chain(
            base,
            members,
            true,
            policy.continuation_indent_levels(),
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if leading_type_argument_call_len > 0 {
        let broken_chain = explicit_type_argument_selector_chain(
            base,
            members,
            leading_type_argument_call_len,
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if keeps_nested_argument_call_head(&groups, &metadata, policy, role) {
        let broken_chain = selector_chain_with_cohesive_head(base, members, 1, policy, false);
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if keeps_simple_receiver_call_run_head(&groups, &metadata, &members, policy, role) {
        let broken_chain =
            simple_receiver_call_run_chain(base, members, metadata.base.source_width, policy);
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if keeps_tiny_simple_receiver_call_head(&groups, &metadata, policy, role) {
        let broken_chain = selector_chain_with_cohesive_head(
            base,
            members,
            1,
            policy,
            matches!(role, ChainRole::Default),
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    let coalesced_prefix_len = single_invocation_coalesced_prefix_len(&members);
    if coalesced_prefix_len > 0 {
        let call_index = coalesced_prefix_len - 1;
        let broken_chain = selector_chain_with_single_invocation_field_prefix(
            base,
            members,
            call_index,
            metadata.base.source_width,
            metadata.base.forces_break_before_first_selector,
            policy,
        );
        if metadata.base.forces_break_before_first_selector {
            return broken_chain;
        }
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if let Some(stream_end) =
        crate::analyzers::chains::stream_suffix_prefix_member_end_index(&members)
        && stream_end + 1 < members.len()
    {
        let broken_chain =
            selector_chain_with_cohesive_head(base, members, stream_end + 1, policy, false);
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if metadata.base.forces_break_before_first_selector {
        let broken_chain = break_before_first_selector_chain(
            base,
            members,
            false,
            policy.selector_chain_primary_selector_indent_levels(metadata.base.kind),
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if let Some(type_prefix_end) = crate::analyzers::type_names::type_name_prefix_member_end_index(
        metadata.base.simple_name.as_deref(),
        &members,
    ) && type_prefix_end + 1 < members.len()
        && !members
            .get(type_prefix_end)
            .is_some_and(ChainMember::is_call)
    {
        let broken_chain = selector_chain_with_prefix_group_at_min_length(
            base,
            members,
            type_prefix_end,
            metadata.base.source_width,
            policy.max_line_length(),
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if let Some(prefix_end) = classified_prefix_member_end_index(&metadata.base, &members)
        && prefix_end + 1 < members.len()
        && members.get(prefix_end).is_some_and(ChainMember::is_call)
    {
        let broken_chain = selector_chain_with_prefix_group(
            base,
            members,
            prefix_end,
            metadata.base.source_width,
            policy,
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    if metadata.call_count >= 10 || breaks_before_first_selector(&metadata, &members, policy, role)
    {
        let broken_chain = break_before_first_selector_chain(
            base,
            members,
            true,
            policy.continuation_indent_levels(),
        );
        return chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata));
    }

    let broken_chain = visit_regular_dot_chain(base, members, policy, &metadata.base);
    chain_layout_preference(flat_chain, broken_chain, role, policy, Some(&metadata))
}
/// Nested chain arguments are embedded in outer chain member docs. Avoid
/// `best_fitting` there: each fit trial walks the full subtree and deeply
/// nested call trees (e.g. `B24909927.java`) blow up exponentially.
fn chain_layout_preference(
    flat: Doc,
    broken: Doc,
    role: ChainRole,
    policy: JavaFormatPolicy,
    metadata: Option<&ChainMetadata>,
) -> Doc {
    match role {
        ChainRole::NestedArgument => broken,
        ChainRole::NestedArgumentFit
            if metadata.is_some_and(|metadata| {
                metadata.total_call_count > policy.nested_argument_selector_chain_fit_call_limit()
            }) =>
        {
            broken
        }
        _ => best_fitting(flat, [broken]),
    }
}

fn is_nested_argument_role(role: ChainRole) -> bool {
    matches!(
        role,
        ChainRole::NestedArgument | ChainRole::NestedArgumentFit
    )
}

fn keeps_nested_argument_call_head(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: &crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
    role: ChainRole,
) -> bool {
    starts_with_simple_receiver_call_run(groups, metadata)
        && policy.selector_chain_preserves_nested_argument_head(role)
}

fn keeps_simple_receiver_call_run_head(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: &crate::analyzers::chains::ChainMetadata,
    members: &[ChainMember],
    policy: JavaFormatPolicy,
    role: ChainRole,
) -> bool {
    starts_with_simple_receiver_call_run(groups, metadata)
        && policy.selector_chain_coalesces_simple_receiver_call_run(role)
        && !matches!(metadata.base.simple_name.as_deref(), Some("this" | "super"))
        && metadata.base.source_width
            <= policy.selector_chain_simple_receiver_call_run_max_base_width()
        && starts_with_simple_zero_arg_call_run(members)
}

fn keeps_tiny_simple_receiver_call_head(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: &crate::analyzers::chains::ChainMetadata,
    policy: JavaFormatPolicy,
    role: ChainRole,
) -> bool {
    if !starts_with_simple_receiver_call_run(groups, metadata) {
        return false;
    }

    if metadata.first_call_argument_count == 0 {
        return !policy.selector_chain_breaks_before_first_selector_for_role(role);
    }

    metadata.base.source_width <= 3 && metadata.first_call_argument_count <= 3
}

fn starts_with_simple_receiver_call_run(
    groups: &crate::analyzers::chains::ChainGroups,
    metadata: &crate::analyzers::chains::ChainMetadata,
) -> bool {
    groups.starts_with_call_run()
        && !metadata.base.is_complex
        && metadata.base.call_count == 0
        && metadata.base.source_width > 0
        && metadata.first_member_is_call
        && metadata.call_count >= 2
}

fn starts_with_simple_zero_arg_call_run(members: &[ChainMember]) -> bool {
    members
        .first()
        .is_some_and(|member| matches!(member.kind, ChainMemberKind::Call { argument_count: 0 }))
        && leading_simple_zero_arg_call_run_len(members) > 0
}

fn leading_simple_zero_arg_call_run_len(members: &[ChainMember]) -> usize {
    members
        .iter()
        .take_while(|member| {
            matches!(member.kind, ChainMemberKind::Call { argument_count: 0 })
                && !member.has_type_arguments
        })
        .count()
}

fn simple_receiver_call_run_chain(
    base: Doc,
    members: Vec<ChainMember>,
    base_width: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    regular_dot_line_limit_call_run_chain(
        base,
        members,
        base_width,
        policy.max_line_length(),
        policy,
    )
}

/// google-java-format's `visitDotWithPrefix` still emits a break opportunity
/// before explicit type-argument invocation selectors; the selector's `<...>`
/// and method name own a separate unified break.
fn explicit_type_argument_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    leading_type_argument_call_len: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    let breaks_type_argument_head = members
        .first()
        .is_some_and(|member| member.selector_head_width > policy.max_line_length());
    let cohesive = selector_chain_with_cohesive_head(
        base.clone(),
        members.clone(),
        leading_type_argument_call_len,
        policy,
        false,
    );
    if !breaks_type_argument_head || !policy.selector_chain_breaks_before_first_selector() {
        return cohesive;
    }

    let break_before_selector = break_before_first_selector_chain(
        base,
        members,
        false,
        policy.continuation_indent_levels(),
    );
    best_fitting(cohesive, [break_before_selector])
}

fn gjf_log_statement_chain(
    base: Doc,
    base_trailing_comments: Vec<Doc>,
    mut members: Vec<ChainMember>,
    metadata: &ChainMetadata,
    policy: JavaFormatPolicy,
) -> Doc {
    if has_trailing_comments(&base_trailing_comments, &members) {
        return commented_selector_chain(base, base_trailing_comments, members, policy);
    }

    let Some(last) = members.pop() else {
        return base;
    };

    let prefix_width = members
        .iter()
        .fold(metadata.base.source_width, |width, member| {
            width + 1 + member.selector_head_width
        })
        + 1
        + last.selector_head_width;
    let prefix_flat = concat(
        std::iter::once(base.clone())
            .chain(members.iter().map(prefixed_member_doc))
            .chain(std::iter::once(concat([
                text("."),
                last.selector_head_doc.clone(),
            ]))),
    );
    if prefix_width <= policy.max_line_length() {
        return concat([prefix_flat, last.selector_suffix_doc]);
    }

    let mut prefix_members = members;
    prefix_members.push(selector_head_member(last.clone()));
    let prefix_broken = regular_dot_line_limit_call_run_chain(
        base,
        prefix_members,
        metadata.base.source_width,
        policy.max_line_length(),
        policy,
    );
    concat([prefix_broken, last.selector_suffix_doc])
}

fn selector_head_member(mut member: ChainMember) -> ChainMember {
    member.doc = member.selector_head_doc.clone();
    member.doc_after_chain_break = None;
    member.doc_as_receiver_head_after_chain_break = None;
    member.selector_suffix_doc = text("");
    member.source_width = member.selector_head_width;
    member
}

fn selector_chain_with_cohesive_head(
    base: Doc,
    mut members: Vec<ChainMember>,
    head_len: usize,
    policy: JavaFormatPolicy,
    use_receiver_head_docs: bool,
) -> Doc {
    let head_len = head_len_with_array_access_suffix(&members, head_len);
    let tail = members.split_off(head_len);
    let has_tail = !tail.is_empty();
    let head_member_count = members.len();
    let head = concat(
        std::iter::once(base).chain(members.into_iter().enumerate().map(|(index, member)| {
            if use_receiver_head_docs && has_tail && index + 1 == head_member_count {
                member_doc_as_receiver_head(member)
            } else {
                member_doc(member)
            }
        })),
    );

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
    members: &[ChainMember],
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
        && ((metadata.total_call_count >= 2
            && metadata.base.source_width + metadata.first_member_width
                >= policy.selector_chain_long_receiver_width())
            || (matches!(role, ChainRole::Default)
                && breaks_long_simple_receiver_call_head(metadata, members, policy)))
}

fn breaks_long_simple_receiver_call_head(
    metadata: &crate::analyzers::chains::ChainMetadata,
    members: &[ChainMember],
    policy: JavaFormatPolicy,
) -> bool {
    let type_static_prefix = crate::analyzers::type_names::type_name_prefix_member_end_index(
        metadata.base.simple_name.as_deref(),
        members,
    )
    .is_some();

    metadata.first_member_is_call
        && !metadata.base.is_complex
        && metadata.base.call_count == 0
        && !matches!(metadata.base.simple_name.as_deref(), Some("this" | "super"))
        && !type_static_prefix
        && (!members
            .first()
            .is_some_and(|member| member.has_type_arguments)
            || metadata.base.source_width >= policy.selector_chain_long_receiver_width())
        && (metadata.base.source_width + 1 + metadata.first_member_head_width
            >= policy.selector_chain_long_receiver_width()
            || metadata.first_member_head_width >= policy.selector_chain_long_receiver_width())
}

fn selector_chain_with_single_invocation_field_prefix(
    base: Doc,
    mut members: Vec<ChainMember>,
    call_index: usize,
    base_width: usize,
    use_receiver_head_call: bool,
    policy: JavaFormatPolicy,
) -> Doc {
    let call_index = head_len_with_array_access_suffix(&members, call_index);
    let tail = members.split_off(call_index + 1);
    let call_member = members
        .pop()
        .expect("call member checked by coalesced prefix");
    let field_members = members;
    let min_length = policy.selector_chain_min_receiver_length_before_break();
    let call = if use_receiver_head_call {
        member_doc_as_receiver_head(call_member)
    } else {
        member_doc(call_member)
    };
    let receiver = if field_members.is_empty() {
        base
    } else {
        field_dot_fill_selector_segments(base, field_members, base_width, min_length, policy)
    };
    let head = concat([receiver, call]);

    if tail.is_empty() {
        return group(head);
    }

    group(concat([
        head,
        continuation_indent(concat(member_docs_after_line(soft_line(), tail)), policy),
    ]))
}

fn selector_chain_with_prefix_group(
    base: Doc,
    members: Vec<ChainMember>,
    prefix_end: usize,
    base_width: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    selector_chain_with_prefix_group_at_min_length(
        base,
        members,
        prefix_end,
        base_width,
        policy.selector_chain_min_receiver_length_before_break(),
        policy,
    )
}

fn selector_chain_with_prefix_group_at_min_length(
    base: Doc,
    mut members: Vec<ChainMember>,
    prefix_end: usize,
    base_width: usize,
    min_length: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    let prefix_end = head_len_with_array_access_suffix(&members, prefix_end + 1) - 1;
    let tail = members.split_off(prefix_end + 1);
    let prefix = dot_fill_prefix_segments(base, members, base_width, min_length, policy);

    if tail.is_empty() {
        return prefix;
    }

    group(concat([
        prefix,
        continuation_indent(concat(member_docs_after_line(soft_line(), tail)), policy),
    ]))
}

fn dot_fill_prefix_segments(
    base: Doc,
    members: Vec<ChainMember>,
    base_width: usize,
    min_length: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    dot_fill_terminal_call_prefix_segment(base, members, base_width, min_length, policy)
        .unwrap_or_else(|(base, members)| {
            dot_fill_selector_segments(base, members, base_width, min_length, policy)
        })
}

fn dot_fill_terminal_call_prefix_segment(
    base: Doc,
    mut members: Vec<ChainMember>,
    base_width: usize,
    min_length: usize,
    policy: JavaFormatPolicy,
) -> Result<Doc, (Doc, Vec<ChainMember>)> {
    let Some(last) = members.pop() else {
        return Err((base, members));
    };
    if !last.is_call() {
        members.push(last);
        return Err((base, members));
    }
    let Some(previous) = members.last_mut() else {
        members.push(last);
        return Err((base, members));
    };
    if !matches!(previous.kind, ChainMemberKind::Field) {
        members.push(last);
        return Err((base, members));
    }

    previous.doc = concat([
        previous.doc.clone(),
        text("."),
        last.selector_head_doc.clone(),
    ]);
    previous.doc_as_receiver_head_after_chain_break = last
        .doc_as_receiver_head_after_chain_break
        .clone()
        .map(|doc| concat([previous.doc.clone(), text("."), doc]));
    previous.doc_after_chain_break = last
        .doc_after_chain_break
        .clone()
        .map(|doc| concat([previous.doc.clone(), text("."), doc]));
    previous.source_width += 1 + last.selector_head_width;
    previous.selector_head_width += 1 + last.selector_head_width;
    previous.kind = ChainMemberKind::Call {
        argument_count: match last.kind {
            ChainMemberKind::Call { argument_count } => argument_count,
            _ => 0,
        },
    };
    previous.has_type_arguments |= last.has_type_arguments;
    previous.simple_name = last.simple_name.clone();

    Ok(concat([
        dot_fill_selector_segments(base, members, base_width, min_length, policy),
        continuation_indent(last.selector_suffix_doc, policy),
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

    group(continuation_indent(fill(entries, last), policy))
}

fn regular_dot_line_limit_call_run_chain(
    base: Doc,
    mut members: Vec<ChainMember>,
    base_width: usize,
    max_width: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    if members.is_empty() {
        return group(base);
    }

    if members.len() == 1 {
        let member = members.into_iter().next().expect("one selector member");
        let separator = regular_dot_line_limit_separator(
            base_width,
            selector_member_head_width(&member),
            max_width,
        );
        return group(concat([
            base,
            continuation_indent(concat([separator, member.doc]), policy),
        ]));
    }

    let last = members.pop().expect("non-empty members checked above");
    let mut length = base_width;
    let mut entries = Vec::with_capacity(members.len() + 1);
    let first_width = selector_member_head_width(
        members
            .first()
            .expect("member remains after popping last selector"),
    );
    let (separator, next_length) =
        regular_dot_line_limit_separator_and_width(length, first_width, max_width);
    length = next_length;
    entries.push(fill_entry(base, separator));

    for member in members {
        let next_width = selector_member_head_width(&member);
        let (separator, next_length) =
            regular_dot_line_limit_separator_and_width(length, next_width, max_width);
        length = next_length;
        entries.push(fill_entry(member.doc, separator));
    }

    group(continuation_indent(fill(entries, last.doc), policy))
}

fn regular_dot_line_limit_separator(
    accumulated_width: usize,
    next_width: usize,
    max_width: usize,
) -> Doc {
    regular_dot_line_limit_separator_and_width(accumulated_width, next_width, max_width).0
}

fn regular_dot_line_limit_separator_and_width(
    accumulated_width: usize,
    next_width: usize,
    max_width: usize,
) -> (Doc, usize) {
    let next_line_width = accumulated_width + 1 + next_width;
    if next_line_width > max_width {
        (concat([soft_line(), text(".")]), 1 + next_width)
    } else {
        (text("."), next_line_width)
    }
}

fn selector_member_head_width(member: &ChainMember) -> usize {
    member.selector_head_width
}

fn field_dot_fill_selector_segments(
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
        let member = members.into_iter().next().expect("one field member");
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
        .expect("multiple field members checked above");
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

    group(continuation_indent(fill(entries, last), policy))
}

/// google-java-format's primary-expression receiver path opens an indented
/// regular-dot chain with `needDot=true` after scanning the receiver.
fn visit_regular_dot_chain_after_primary_receiver(
    base: Doc,
    mut members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
) -> Doc {
    if members.is_empty() {
        return group(base);
    }

    let base = append_leading_array_accesses(base, &mut members);
    if members.is_empty() {
        return group(base);
    }

    if let Some(first) = members.first()
        && fits_fill_first_argument(first, &members[1..])
    {
        let first = members.remove(0);
        let head = concat([base, soft_line(), member_doc(first)]);
        let head = append_leading_array_accesses(head, &mut members);
        if members.is_empty() {
            return group(continuation_indent(head, policy));
        }
        return group(continuation_indent(
            regular_dot_fill_from_head(head, members, 0, policy),
            policy,
        ));
    }

    group(continuation_indent(
        regular_dot_fill_after_needed_dot(base, members, policy),
        policy,
    ))
}

fn regular_dot_fill_after_needed_dot(
    base: Doc,
    mut members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
) -> Doc {
    let first = members.remove(0);
    let first_width = first.source_width;
    let head = concat([base, soft_line(), member_doc(first)]);
    regular_dot_fill_from_head(
        head,
        members,
        policy
            .selector_chain_min_receiver_length_before_break()
            .saturating_add(1)
            .saturating_add(first_width),
        policy,
    )
}

fn regular_dot_fill_from_head(
    head: Doc,
    members: Vec<ChainMember>,
    mut accumulated_width: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    if members.is_empty() {
        return head;
    }

    let min_length = policy.selector_chain_min_receiver_length_before_break();
    let mut segments = members
        .into_iter()
        .map(|member| {
            let doc = member.doc_after_chain_break.unwrap_or(member.doc);
            (member.source_width, doc)
        })
        .collect::<Vec<_>>();
    let (_last_width, last) = segments
        .pop()
        .expect("non-empty tail checked by regular dot caller");
    let entries = std::iter::once((0, head))
        .chain(segments)
        .map(|(width, segment)| {
            accumulated_width = accumulated_width.saturating_add(1).saturating_add(width);
            let separator = if accumulated_width > min_length {
                concat([soft_line(), text(".")])
            } else {
                text(".")
            };
            fill_entry(segment, separator)
        });

    fill(entries, last)
}

/// google-java-format `visitRegularDot` fallback when no prefix/cohesive route applies.
fn visit_regular_dot_chain(
    base: Doc,
    mut members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
    base_metadata: &BaseMetadata,
) -> Doc {
    if members.is_empty() {
        return group(base);
    }

    if let Some(first) = members.first()
        && fits_fill_first_argument(first, &members[1..])
    {
        let first = members.remove(0);
        let merged_base =
            append_leading_array_accesses(concat([base, member_doc(first)]), &mut members);
        if members.is_empty() {
            return group(merged_base);
        }
        return group(concat([
            merged_base,
            continuation_indent(concat(member_docs_after_line(soft_line(), members)), policy),
        ]));
    }

    let _ = base_metadata;
    let first_member = members.remove(0);
    let base =
        append_leading_array_accesses(concat([base, member_doc(first_member)]), &mut members);
    if members.is_empty() {
        return group(base);
    }

    group(concat([
        base,
        continuation_indent(concat(member_docs_after_line(soft_line(), members)), policy),
    ]))
}

fn fits_fill_first_argument(first: &ChainMember, trailing: &[ChainMember]) -> bool {
    if trailing.is_empty() {
        return false;
    }

    match first.kind {
        ChainMemberKind::Call { argument_count: 1 } => {
            !first.has_type_arguments
                && first
                    .simple_name
                    .as_ref()
                    .is_some_and(|name| name.len() <= 4)
        }
        _ => false,
    }
}

fn break_before_first_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    force_break: bool,
    indent_levels: u16,
) -> Doc {
    let separator = if force_break {
        hard_line()
    } else {
        soft_line()
    };
    group(concat([
        base,
        indent_by(
            indent_levels,
            concat(member_docs_after_line(separator, members)),
        ),
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
                let doc = member.doc_after_chain_break.unwrap_or(member.doc);
                let doc = append_trailing_comments(doc, member.trailing_comments);
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
        return chain_layout_preference(flat, broken, role, policy, None);
    }

    let entries = std::iter::once(base)
        .chain(segments)
        .map(|segment| fill_entry(segment, concat([soft_line(), text(".")])));

    let _ = role;
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

fn member_doc_as_receiver_head(member: ChainMember) -> Doc {
    let kind = member.kind;
    let doc = member
        .doc_as_receiver_head_after_chain_break
        .unwrap_or(member.doc);
    member_doc_with_kind(kind, doc)
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
                let doc = member.doc_after_chain_break.unwrap_or(member.doc);
                docs.push(concat([line.clone(), text("."), doc]));
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
