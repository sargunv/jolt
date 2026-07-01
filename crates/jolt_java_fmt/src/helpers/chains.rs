use jolt_fmt_ir::{
    Doc, LevelBreak, LevelBreakMode, break_level_with_indent, concat, flat_text, group, hard_line,
    if_group_breaks, indent_by, level_break_with_prefix, soft_line, text,
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

    if is_gjf_log_statement_chain(&metadata.base, &members) {
        return gjf_log_statement_chain(base, base_trailing_comments, members, &metadata, policy);
    }

    if matches!(metadata.base.kind, ChainBaseKind::PrimaryExpression)
        && members
            .first()
            .is_some_and(|member| matches!(member.kind, ChainMemberKind::Field))
        && !members
            .first()
            .is_some_and(|member| member.has_type_arguments)
    {
        return visit_regular_dot_chain_after_primary_receiver(base, members, policy);
    }

    if groups.all_fields(members.len()) {
        if is_nested_argument_role(role) {
            return field_selector_chain(base, members, policy);
        }
        let min_length = policy.selector_chain_min_receiver_length_before_break();
        return field_regular_dot_selector_segments(
            base,
            members,
            metadata.base.source_width,
            min_length,
            policy,
        );
    }

    let leading_type_argument_call_len = groups.leading_type_argument_call_len();
    if leading_type_argument_call_len > 0
        && matches!(role, ChainRole::Default)
        && metadata.base.source_width >= policy.selector_chain_long_receiver_width()
        && breaks_long_simple_receiver_call_head(&metadata, &members, policy)
    {
        return break_before_first_selector_chain(
            base,
            members,
            true,
            policy.continuation_indent_levels(),
        );
    }

    if leading_type_argument_call_len > 0 {
        if keeps_this_super_explicit_type_argument_selector(&metadata.base, policy) {
            return this_super_explicit_type_argument_selector_chain(base, members, policy);
        }
        return explicit_type_argument_selector_chain(
            base,
            members,
            &metadata.base,
            leading_type_argument_call_len,
            policy,
        );
    }

    if keeps_nested_argument_call_head(&groups, &metadata, policy, role) {
        return selector_chain_with_cohesive_head(
            base,
            members,
            1,
            policy,
            false,
            LevelBreakMode::Independent,
        );
    }

    if keeps_simple_receiver_call_run_head(&groups, &metadata, &members, policy, role) {
        return simple_receiver_call_run_chain(base, members, metadata.base.source_width, policy);
    }

    if keeps_tiny_simple_receiver_call_head(&groups, &metadata, policy, role) {
        return selector_chain_with_cohesive_head(
            base,
            members,
            1,
            policy,
            matches!(role, ChainRole::Default),
            LevelBreakMode::Independent,
        );
    }

    if metadata.base.forces_break_before_first_selector {
        return break_before_first_selector_chain(
            base,
            members,
            false,
            policy.selector_chain_primary_selector_indent_levels(metadata.base.kind),
        );
    }

    if let Some(prefix_end) = classified_prefix_member_end_index(&metadata.base, &members) {
        let fill_mode = prefix_fill_mode(&members);
        return visit_dot_with_prefix(base, members, prefix_end, fill_mode, policy, &metadata.base);
    }

    if breaks_before_first_selector(&metadata, &members, policy, role) {
        return break_before_first_selector_chain(
            base,
            members,
            true,
            policy.continuation_indent_levels(),
        );
    }

    visit_regular_dot_chain(base, members, policy, &metadata.base)
}

fn keeps_this_super_explicit_type_argument_selector(
    base: &BaseMetadata,
    policy: JavaFormatPolicy,
) -> bool {
    (base.is_qualified_this_super_prefix
        || matches!(base.simple_name.as_deref(), Some("this" | "super")))
        && base.source_width <= policy.selector_chain_long_receiver_width()
}

fn this_super_explicit_type_argument_selector_chain(
    base: Doc,
    mut members: Vec<ChainMember>,
    policy: JavaFormatPolicy,
) -> Doc {
    let Some(first) = members.first().cloned() else {
        return group(base);
    };
    members.remove(0);
    let base = concat([base, text("."), member_chain_segment(&first)]);
    if members.is_empty() {
        return group(base);
    }
    prefix_dot_break_level_chain(
        base,
        &members.iter().map(member_chain_segment).collect::<Vec<_>>(),
        0,
        LevelBreakMode::Unified,
        policy.continuation_indent_levels(),
    )
}

fn dot_level_break(mode: LevelBreakMode) -> LevelBreak {
    level_break_with_prefix(mode, flat_text("."), text("."), 0)
}

fn member_regular_dot_segment(member: &ChainMember) -> Doc {
    member.doc.clone()
}

/// google-java-format `visitRegularDot`: optional dot breaks once accumulated
/// receiver length exceeds `min_length`.
fn regular_dot_min_length_chain(
    base: Doc,
    members: &[ChainMember],
    initial_length: usize,
    min_length: usize,
    indent_levels: u16,
    tail_break_mode: LevelBreakMode,
) -> Doc {
    group(regular_dot_min_length_chain_doc(
        base,
        members,
        initial_length,
        min_length,
        indent_levels,
        tail_break_mode,
    ))
}

/// Emits a flat prefix while `length <= min_length`, then a unified dot level
/// for the tail. Matches GJF inserting `breakOp(UNIFIED)` only once the running
/// receiver length exceeds `indentMultiplier * 4`.
fn regular_dot_min_length_chain_doc(
    base: Doc,
    members: &[ChainMember],
    mut length: usize,
    min_length: usize,
    indent_levels: u16,
    tail_break_mode: LevelBreakMode,
) -> Doc {
    if members.is_empty() {
        return base;
    }

    let mut flat_parts: Vec<Doc> = vec![base];
    let mut tail: Vec<Doc> = Vec::new();

    for member in members {
        let doc = member_regular_dot_segment(member);
        if tail.is_empty() {
            if length > min_length {
                tail.push(doc);
            } else {
                flat_parts.push(text("."));
                flat_parts.push(doc);
            }
            length = length.saturating_add(1);
        } else {
            tail.push(doc);
        }
        length = length.saturating_add(member.source_width);
    }

    let flat = concat(flat_parts);
    if tail.is_empty() {
        return flat;
    }

    let mut segments = vec![flat];
    segments.extend(tail.iter().cloned());

    let breaks = vec![dot_level_break(tail_break_mode); tail.len()];
    break_level_with_indent(indent_levels as i16, segments, breaks)
        .expect("valid regular dot min-length chain")
}

/// google-java-format `visitDotWithPrefix`: outer plusFour, prefix dots through
/// `prefix_end` use `prefix_fill_mode`, trailing dots use unified breaks.
fn visit_dot_with_prefix(
    base: Doc,
    members: Vec<ChainMember>,
    prefix_end: usize,
    prefix_fill_mode: LevelBreakMode,
    policy: JavaFormatPolicy,
    base_metadata: &BaseMetadata,
) -> Doc {
    let single_invocation_terminal =
        single_invocation_coalesced_prefix_len(base_metadata, &members) == members.len();
    let prefix_end = head_len_with_array_access_suffix(&members, prefix_end + 1) - 1;
    let trailing_dereferences = prefix_end + 1 < members.len();

    let leading_field_prefix_len = members
        .iter()
        .take(members.len().saturating_sub(1))
        .filter(|member| matches!(member.kind, ChainMemberKind::Field))
        .count();
    let terminal_call_can_own_args = single_invocation_terminal
        && members.get(prefix_end).is_some_and(ChainMember::is_call)
        && !trailing_dereferences
        && (leading_field_prefix_len <= 1
            || prefix_source_width(&members[..prefix_end])
                <= policy.selector_chain_long_receiver_width());
    if terminal_call_can_own_args || (single_invocation_terminal && leading_field_prefix_len <= 1) {
        let mut parts: Vec<Doc> = vec![base];
        for member in &members {
            parts.push(text("."));
            parts.push(member_regular_dot_segment(member));
        }
        return group(concat(parts));
    }

    if single_invocation_terminal && !trailing_dereferences {
        return regular_dot_min_length_chain(
            base,
            &members,
            base_metadata.source_width,
            policy.selector_chain_min_receiver_length_before_break(),
            policy.continuation_indent_levels(),
            LevelBreakMode::Unified,
        );
    }

    let member_segments: Vec<Doc> = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            if single_invocation_terminal
                && index == prefix_end
                && member.is_call()
                && !trailing_dereferences
            {
                member_regular_dot_segment(member)
            } else {
                member_chain_segment(member)
            }
        })
        .collect();
    prefix_dot_break_level_chain(
        base,
        &member_segments,
        prefix_end,
        prefix_fill_mode,
        policy.continuation_indent_levels(),
    )
}

fn prefix_source_width(members: &[ChainMember]) -> usize {
    members.iter().fold(0usize, |width, member| {
        width.saturating_add(member.source_width.saturating_add(1))
    })
}

fn member_chain_segment(member: &ChainMember) -> Doc {
    member
        .doc_after_chain_break
        .clone()
        .unwrap_or_else(|| member.doc.clone())
}

fn prefix_fill_mode(members: &[ChainMember]) -> LevelBreakMode {
    if crate::analyzers::chains::stream_suffix_prefix_member_end_index(members).is_some() {
        LevelBreakMode::Unified
    } else {
        LevelBreakMode::Independent
    }
}

/// google-java-format `visitDotWithPrefix`: prefix dots use `prefix_fill_mode`,
/// trailing selector dots use unified breaks.
fn prefix_dot_break_level_chain(
    base: Doc,
    member_segments: &[Doc],
    prefix_end: usize,
    prefix_fill_mode: LevelBreakMode,
    indent_levels: u16,
) -> Doc {
    if member_segments.is_empty() {
        return group(base);
    }

    let segments: Vec<Doc> = std::iter::once(base.clone())
        .chain(member_segments.iter().cloned())
        .collect();
    let breaks: Vec<LevelBreak> = (0..member_segments.len())
        .map(|index| {
            dot_level_break(if index <= prefix_end {
                prefix_fill_mode
            } else {
                LevelBreakMode::Unified
            })
        })
        .collect();
    group(
        break_level_with_indent(indent_levels as i16, segments, breaks)
            .expect("valid prefix dot break level chain"),
    )
}

/// Field/call-only chains on the hot expression path: one level with dot breaks.
fn field_call_dot_break_level_chain(
    base: Doc,
    members: &[ChainMember],
    mode: LevelBreakMode,
    indent_levels: u16,
) -> Doc {
    let member_segments: Vec<Doc> = members.iter().map(|member| member.doc.clone()).collect();
    prefix_dot_break_level_chain(
        base,
        &member_segments,
        members.len().saturating_sub(1),
        mode,
        indent_levels,
    )
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
    regular_dot_min_length_chain(
        base,
        &members,
        base_width,
        policy.selector_chain_min_receiver_length_before_break(),
        policy.continuation_indent_levels(),
        LevelBreakMode::Unified,
    )
}

/// google-java-format's `visitDotWithPrefix` still emits a break opportunity
/// before explicit type-argument invocation selectors; the selector's `<...>`
/// and method name own a separate unified break.
fn explicit_type_argument_selector_chain(
    base: Doc,
    members: Vec<ChainMember>,
    base_metadata: &BaseMetadata,
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
        LevelBreakMode::Independent,
    );

    if base_metadata.is_qualified_this_super_prefix
        || matches!(base_metadata.simple_name.as_deref(), Some("this" | "super"))
        || !breaks_type_argument_head
        || !policy.selector_chain_breaks_before_first_selector()
    {
        return cohesive;
    }

    // The explicit type-argument head alone exceeds the line width, so the
    // cohesive layout cannot fit flat; break before the first selector.
    break_before_first_selector_chain(base, members, false, policy.continuation_indent_levels())
}

fn gjf_log_statement_chain(
    base: Doc,
    base_trailing_comments: Vec<Doc>,
    members: Vec<ChainMember>,
    metadata: &ChainMetadata,
    policy: JavaFormatPolicy,
) -> Doc {
    if has_trailing_comments(&base_trailing_comments, &members) {
        return commented_selector_chain(base, base_trailing_comments, members, policy);
    }

    let _ = metadata;
    field_call_dot_break_level_chain(
        append_trailing_comments(base, base_trailing_comments),
        &members,
        LevelBreakMode::Independent,
        policy.continuation_indent_levels(),
    )
}

fn selector_chain_with_cohesive_head(
    base: Doc,
    members: Vec<ChainMember>,
    head_len: usize,
    policy: JavaFormatPolicy,
    use_receiver_head_docs: bool,
    prefix_fill_mode: LevelBreakMode,
) -> Doc {
    let head_len = head_len_with_array_access_suffix(&members, head_len);
    let has_tail = head_len < members.len();
    let member_segments: Vec<Doc> = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            if use_receiver_head_docs && has_tail && index + 1 == head_len {
                member
                    .doc_as_receiver_head_after_chain_break
                    .clone()
                    .unwrap_or_else(|| member.doc.clone())
            } else {
                member_chain_segment(member)
            }
        })
        .collect();
    prefix_dot_break_level_chain(
        base,
        &member_segments,
        head_len.saturating_sub(1),
        prefix_fill_mode,
        policy.continuation_indent_levels(),
    )
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

    if matches!(metadata.base.kind, ChainBaseKind::ObjectCreation) {
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

fn field_regular_dot_selector_segments(
    base: Doc,
    members: Vec<ChainMember>,
    base_width: usize,
    min_length: usize,
    policy: JavaFormatPolicy,
) -> Doc {
    regular_dot_min_length_chain(
        base,
        &members,
        base_width,
        min_length,
        policy.continuation_indent_levels(),
        LevelBreakMode::Independent,
    )
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

    let min_length = policy.selector_chain_min_receiver_length_before_break();

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
            regular_dot_min_length_chain_doc(
                head,
                &members,
                0,
                min_length,
                0,
                LevelBreakMode::Unified,
            ),
            policy,
        ));
    }

    group(continuation_indent(
        concat([
            base,
            soft_line(),
            regular_dot_min_length_chain_doc(
                text(""),
                &members,
                min_length,
                min_length,
                0,
                LevelBreakMode::Unified,
            ),
        ]),
        policy,
    ))
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
        && !matches!(base_metadata.kind, ChainBaseKind::ObjectCreation)
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
            continuation_indent(
                regular_dot_min_length_chain_doc(
                    text(""),
                    &members,
                    policy.selector_chain_min_receiver_length_before_break(),
                    policy.selector_chain_min_receiver_length_before_break(),
                    0,
                    LevelBreakMode::Unified,
                ),
                policy,
            ),
        ]));
    }

    let base = append_leading_array_accesses(base, &mut members);
    let min_length = policy.selector_chain_min_receiver_length_before_break();

    if matches!(base_metadata.kind, ChainBaseKind::ObjectCreation) {
        if members.is_empty() {
            return group(base);
        }
        return group(concat([
            base,
            continuation_indent(
                concat([
                    soft_line(),
                    regular_dot_min_length_chain_doc(
                        text(""),
                        &members,
                        min_length,
                        min_length,
                        0,
                        LevelBreakMode::Unified,
                    ),
                ]),
                policy,
            ),
        ]));
    }

    regular_dot_min_length_chain(
        base,
        &members,
        base_metadata.source_width,
        min_length,
        policy.continuation_indent_levels(),
        LevelBreakMode::Unified,
    )
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

fn field_selector_chain(base: Doc, members: Vec<ChainMember>, policy: JavaFormatPolicy) -> Doc {
    field_call_dot_break_level_chain(
        base,
        &members,
        LevelBreakMode::Independent,
        policy.continuation_indent_levels(),
    )
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
