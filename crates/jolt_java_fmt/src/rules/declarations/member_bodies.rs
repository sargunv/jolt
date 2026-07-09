use super::{
    AnnotationInterfaceBodyMember, ClassBody, ClassBodyMember, Doc, FormattedMember, InterfaceBody,
    InterfaceBodyMember, JavaFormatter, JavaSyntaxToken, MemberCategory, Range, RecordBody,
    comments_from_tokens, concat, format_annotation_element_declaration,
    format_annotation_interface_declaration, format_block, format_class_declaration,
    format_compact_constructor_declaration, format_constructor_declaration,
    format_enum_declaration, format_field_declaration, format_interface_declaration,
    format_method_declaration, format_record_declaration, format_removed_comments,
    format_token_sequence, format_token_with_comments, formatter_ignore_ranges,
    formatter_ignore_run_doc, formatter_ignore_runs, hard_line, has_removed_comments,
    join_member_docs, relative_token_range_between,
};
use jolt_fmt_ir::space;

pub(super) fn format_class_body<'source>(
    body: &ClassBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if ignored_ranges.is_empty() {
        return format_class_member_docs_with_recovered(
            format_body_open_dangling_comments(body.open_brace()),
            body.members_with_recovered(),
            format_body_close_dangling_comments(body.close_brace()),
            formatter,
        );
    }

    format_class_member_docs_with_recovered_and_ignored(
        body.text_range().start().get(),
        &ignored_ranges,
        body.members_with_recovered(),
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_record_body<'source>(
    body: &RecordBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if ignored_ranges.is_empty() {
        return format_class_member_docs_with_recovered(
            format_body_open_dangling_comments(body.open_brace()),
            body.members_with_recovered(),
            format_body_close_dangling_comments(body.close_brace()),
            formatter,
        );
    }

    format_class_member_docs_with_recovered_and_ignored(
        body.text_range().start().get(),
        &ignored_ranges,
        body.members_with_recovered(),
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_interface_body<'source>(
    body: &InterfaceBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if ignored_ranges.is_empty() {
        return format_interface_member_docs_with_recovered(
            format_body_open_dangling_comments(body.open_brace()),
            body.members_with_recovered(),
            format_body_close_dangling_comments(body.close_brace()),
            formatter,
        );
    }
    format_interface_member_docs_with_recovered_and_ignored(
        body.text_range().start().get(),
        &ignored_ranges,
        format_body_open_dangling_comments(body.open_brace()),
        body.members_with_recovered(),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_annotation_interface_body<'source>(
    body: &jolt_java_syntax::AnnotationInterfaceBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    if ignored_ranges.is_empty() {
        return format_annotation_member_docs_with_recovered(
            format_body_open_dangling_comments(body.open_brace()),
            body.members_with_recovered(),
            format_body_close_dangling_comments(body.close_brace()),
            formatter,
        );
    }
    format_annotation_member_docs_with_recovered_and_ignored(
        body.text_range().start().get(),
        &ignored_ranges,
        format_body_open_dangling_comments(body.open_brace()),
        body.members_with_recovered(),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_class_member_body<'source>(
    body_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, ClassBodyMember<'source>>,
    >,
    open_dangling_comments: Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter();
    if ignored_ranges.is_empty() {
        return format_class_member_docs_with_recovered(
            open_dangling_comments,
            members,
            close_dangling_comments,
            formatter,
        );
    }

    let members = members.collect::<Vec<_>>();
    let member_ranges = members
        .iter()
        .map(|member| recovered_class_member_token_range(member, body_start))
        .collect::<Vec<_>>();
    format_members_with_ignored(
        &members,
        &formatter_ignore_runs(ignored_ranges, &member_ranges),
        open_dangling_comments,
        |run, members| ignored_recovered_class_member_category(run, members),
        |member| format_recovered_class_member(member, formatter),
        close_dangling_comments,
    )
}

fn format_members_with_ignored<'source, Member>(
    members: &[Member],
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
    open_dangling_comments: Option<FormattedMember<'source>>,
    mut ignored_category: impl FnMut(
        &crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>,
        &[Member],
    ) -> MemberCategory,
    mut format_member: impl FnMut(&Member) -> Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
) -> Option<Doc<'source>> {
    let mut formatted_members = Vec::with_capacity(
        members
            .len()
            .saturating_add(ignored_runs.len())
            .saturating_add(2),
    );
    formatted_members.extend(open_dangling_comments);
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted_members.push(FormattedMember::ignored(
                formatter_ignore_run_doc(run),
                ignored_category(run, members),
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= member_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(member_index) {
            continue;
        }

        if let Some(mut formatted_member) = format_member(member) {
            if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
                formatted_member = formatted_member.without_blank_line_before();
            }
            formatted_members.push(formatted_member);
        }
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted_members.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_category(run, members),
        ));
        ignored_index += 1;
    }
    formatted_members.extend(close_dangling_comments);

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

fn format_class_member_docs_with_recovered<'source>(
    open_dangling_comments: Option<FormattedMember<'source>>,
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, ClassBodyMember<'source>>,
    >,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter();
    let (lower, _) = members.size_hint();
    let mut formatted_members = Vec::with_capacity(lower.saturating_add(2));
    formatted_members.extend(open_dangling_comments);

    for entry in members {
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
                if is_printable_class_member(&member) {
                    formatted_members.push(FormattedMember::from_member(&member, formatter));
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    std::iter::once(token),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    error.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    node.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
        }
    }
    formatted_members.extend(close_dangling_comments);

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

fn format_class_member_docs_with_recovered_and_ignored<'source>(
    source_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, ClassBodyMember<'source>>,
    >,
    open_dangling_comments: Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter().collect::<Vec<_>>();
    let member_ranges = members
        .iter()
        .map(|member| recovered_class_member_token_range(member, source_start))
        .collect::<Vec<_>>();
    format_members_with_ignored(
        &members,
        &formatter_ignore_runs(ignored_ranges, &member_ranges),
        open_dangling_comments,
        |run, members| ignored_recovered_class_member_category(run, members),
        |member| format_recovered_class_member(member, formatter),
        close_dangling_comments,
    )
}

fn format_interface_member_docs_with_recovered<'source>(
    open_dangling_comments: Option<FormattedMember<'source>>,
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, InterfaceBodyMember<'source>>,
    >,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter();
    let (lower, _) = members.size_hint();
    let mut formatted_members = Vec::with_capacity(lower.saturating_add(2));
    formatted_members.extend(open_dangling_comments);

    for entry in members {
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
                if is_printable_interface_member(&member) {
                    formatted_members
                        .push(FormattedMember::from_interface_member(&member, formatter));
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    std::iter::once(token),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    error.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    node.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
        }
    }
    formatted_members.extend(close_dangling_comments);

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

fn format_interface_member_docs_with_recovered_and_ignored<'source>(
    source_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    open_dangling_comments: Option<FormattedMember<'source>>,
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<'source, InterfaceBodyMember<'source>>,
    >,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter().collect::<Vec<_>>();
    let member_ranges = members
        .iter()
        .map(|member| recovered_interface_member_token_range(member, source_start))
        .collect::<Vec<_>>();
    format_members_with_ignored(
        &members,
        &formatter_ignore_runs(ignored_ranges, &member_ranges),
        open_dangling_comments,
        |run, members| ignored_recovered_interface_member_category(run, members),
        |member| format_recovered_interface_member(member, formatter),
        close_dangling_comments,
    )
}

fn format_annotation_member_docs_with_recovered<'source>(
    open_dangling_comments: Option<FormattedMember<'source>>,
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<
            'source,
            AnnotationInterfaceBodyMember<'source>,
        >,
    >,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter();
    let (lower, _) = members.size_hint();
    let mut formatted_members = Vec::with_capacity(lower.saturating_add(2));
    formatted_members.extend(open_dangling_comments);

    for entry in members {
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
                if is_printable_annotation_member(&member) {
                    formatted_members
                        .push(FormattedMember::from_annotation_member(&member, formatter));
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    std::iter::once(token),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    error.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                formatted_members.push(FormattedMember::comment(format_token_sequence(
                    node.token_iter(),
                    crate::helpers::comments::LeadingTrivia::Preserve,
                )));
            }
        }
    }
    formatted_members.extend(close_dangling_comments);

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

fn format_annotation_member_docs_with_recovered_and_ignored<'source>(
    source_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    open_dangling_comments: Option<FormattedMember<'source>>,
    members: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<
            'source,
            AnnotationInterfaceBodyMember<'source>,
        >,
    >,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = members.into_iter().collect::<Vec<_>>();
    let member_ranges = members
        .iter()
        .map(|member| recovered_annotation_member_token_range(member, source_start))
        .collect::<Vec<_>>();
    format_members_with_ignored(
        &members,
        &formatter_ignore_runs(ignored_ranges, &member_ranges),
        open_dangling_comments,
        |run, members| ignored_recovered_annotation_member_category(run, members),
        |member| format_recovered_annotation_member(member, formatter),
        close_dangling_comments,
    )
}

fn class_member_token_range(
    member: &ClassBodyMember<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &member.first_token()?,
        &member.last_token()?,
        body_start,
    ))
}

fn interface_member_token_range(
    member: &InterfaceBodyMember<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &member.first_token()?,
        &member.last_token()?,
        body_start,
    ))
}

fn annotation_member_token_range(
    member: &AnnotationInterfaceBodyMember<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &member.first_token()?,
        &member.last_token()?,
        body_start,
    ))
}

fn recovered_class_member_token_range(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, ClassBodyMember<'_>>,
    body_start: usize,
) -> Option<Range<usize>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            class_member_token_range(member, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(relative_token_range_between(token, token, body_start))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            recovered_error_token_range(error, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            recovered_node_token_range(node, body_start)
        }
    }
}

fn recovered_interface_member_token_range(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, InterfaceBodyMember<'_>>,
    body_start: usize,
) -> Option<Range<usize>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            interface_member_token_range(member, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(relative_token_range_between(token, token, body_start))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            recovered_error_token_range(error, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            recovered_node_token_range(node, body_start)
        }
    }
}

fn recovered_annotation_member_token_range(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, AnnotationInterfaceBodyMember<'_>>,
    body_start: usize,
) -> Option<Range<usize>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            annotation_member_token_range(member, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(relative_token_range_between(token, token, body_start))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            recovered_error_token_range(error, body_start)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            recovered_node_token_range(node, body_start)
        }
    }
}

fn recovered_error_token_range(
    error: &jolt_java_syntax::ErrorNode<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &error.first_token()?,
        &error.last_token()?,
        body_start,
    ))
}

fn recovered_node_token_range(
    node: &jolt_java_syntax::RecoveredNode<'_>,
    body_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &node.first_token()?,
        &node.last_token()?,
        body_start,
    ))
}

fn ignored_recovered_class_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[jolt_java_syntax::RecoveredSeparatedListEntry<'_, ClassBodyMember<'_>>],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, recovered_class_member_category)
}

fn ignored_recovered_interface_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[jolt_java_syntax::RecoveredSeparatedListEntry<'_, InterfaceBodyMember<'_>>],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, recovered_interface_member_category)
}

fn ignored_recovered_annotation_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[jolt_java_syntax::RecoveredSeparatedListEntry<
        '_,
        AnnotationInterfaceBodyMember<'_>,
    >],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, recovered_annotation_member_category)
}

fn recovered_class_member_category(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, ClassBodyMember<'_>>,
) -> MemberCategory {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => member_category(member),
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Error(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Node(_) => MemberCategory::Type,
    }
}

fn recovered_interface_member_category(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, InterfaceBodyMember<'_>>,
) -> MemberCategory {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            interface_member_category(member)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Error(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Node(_) => MemberCategory::Type,
    }
}

fn recovered_annotation_member_category(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, AnnotationInterfaceBodyMember<'_>>,
) -> MemberCategory {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            annotation_member_category(member)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Error(_)
        | jolt_java_syntax::RecoveredSeparatedListEntry::Node(_) => MemberCategory::Type,
    }
}

fn format_recovered_class_member<'source>(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'source, ClassBodyMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<FormattedMember<'source>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            is_printable_class_member(member)
                .then(|| FormattedMember::from_member(member, formatter))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(FormattedMember::comment(format_token_sequence(
                std::iter::once(*token),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            Some(FormattedMember::comment(format_token_sequence(
                error.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            Some(FormattedMember::comment(format_token_sequence(
                node.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
    }
}

fn format_recovered_interface_member<'source>(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<'source, InterfaceBodyMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<FormattedMember<'source>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            is_printable_interface_member(member)
                .then(|| FormattedMember::from_interface_member(member, formatter))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(FormattedMember::comment(format_token_sequence(
                std::iter::once(*token),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            Some(FormattedMember::comment(format_token_sequence(
                error.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            Some(FormattedMember::comment(format_token_sequence(
                node.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
    }
}

fn format_recovered_annotation_member<'source>(
    member: &jolt_java_syntax::RecoveredSeparatedListEntry<
        'source,
        AnnotationInterfaceBodyMember<'source>,
    >,
    formatter: &JavaFormatter<'_>,
) -> Option<FormattedMember<'source>> {
    match member {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(member) => {
            is_printable_annotation_member(member)
                .then(|| FormattedMember::from_annotation_member(member, formatter))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(FormattedMember::comment(format_token_sequence(
                std::iter::once(*token),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
            Some(FormattedMember::comment(format_token_sequence(
                error.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
            Some(FormattedMember::comment(format_token_sequence(
                node.token_iter(),
                crate::helpers::comments::LeadingTrivia::Preserve,
            )))
        }
    }
}

fn is_printable_class_member(member: &ClassBodyMember<'_>) -> bool {
    !matches!(member, ClassBodyMember::EmptyDeclaration(_))
        || has_removed_comments(comments_from_tokens(member.token_iter()))
}

fn is_printable_interface_member(member: &InterfaceBodyMember<'_>) -> bool {
    !matches!(member, InterfaceBodyMember::EmptyDeclaration(_))
        || has_removed_comments(comments_from_tokens(member.token_iter()))
}

fn is_printable_annotation_member(member: &AnnotationInterfaceBodyMember<'_>) -> bool {
    !matches!(member, AnnotationInterfaceBodyMember::EmptyDeclaration(_))
        || has_removed_comments(comments_from_tokens(member.token_iter()))
}

pub(super) fn format_body_open_dangling_comments(
    open: Option<JavaSyntaxToken<'_>>,
) -> Option<FormattedMember<'_>> {
    format_removed_comments(open?.trailing_comments()).map(FormattedMember::comment)
}

pub(super) fn format_body_close_dangling_comments(
    close: Option<JavaSyntaxToken<'_>>,
) -> Option<FormattedMember<'_>> {
    format_removed_comments(close?.leading_comments()).map(FormattedMember::comment)
}

pub(super) fn format_empty_enum_constant_list_comments(
    constants: Option<jolt_java_syntax::EnumConstantList<'_>>,
) -> Option<FormattedMember<'_>> {
    let constants = constants?;
    if constants.constants().next().is_some() {
        return None;
    }

    format_removed_comments(comments_from_tokens(constants.token_iter()))
        .map(FormattedMember::comment)
}

pub(super) fn combine_comment_members<'source>(
    first: Option<FormattedMember<'source>>,
    second: Option<FormattedMember<'source>>,
) -> Option<FormattedMember<'source>> {
    match (first, second) {
        (Some(first), Some(second)) => Some(FormattedMember::comment(concat([
            first.doc,
            hard_line(),
            second.doc,
        ]))),
        (Some(member), None) | (None, Some(member)) => Some(member),
        (None, None) => None,
    }
}

fn member_category(member: &ClassBodyMember<'_>) -> MemberCategory {
    match member {
        ClassBodyMember::FieldDeclaration(_) => MemberCategory::Field,
        ClassBodyMember::ConstructorDeclaration(_)
        | ClassBodyMember::CompactConstructorDeclaration(_) => MemberCategory::Constructor,
        ClassBodyMember::MethodDeclaration(_) => MemberCategory::Method,
        ClassBodyMember::StaticInitializer(_) | ClassBodyMember::InstanceInitializer(_) => {
            MemberCategory::Initializer
        }
        ClassBodyMember::ClassDeclaration(_)
        | ClassBodyMember::RecordDeclaration(_)
        | ClassBodyMember::EnumDeclaration(_)
        | ClassBodyMember::InterfaceDeclaration(_)
        | ClassBodyMember::AnnotationInterfaceDeclaration(_)
        | ClassBodyMember::EmptyDeclaration(_) => MemberCategory::Type,
    }
}

fn interface_member_category(member: &InterfaceBodyMember<'_>) -> MemberCategory {
    match member {
        InterfaceBodyMember::FieldDeclaration(_) => MemberCategory::Field,
        InterfaceBodyMember::MethodDeclaration(_) => MemberCategory::Method,
        InterfaceBodyMember::ClassDeclaration(_)
        | InterfaceBodyMember::RecordDeclaration(_)
        | InterfaceBodyMember::EnumDeclaration(_)
        | InterfaceBodyMember::InterfaceDeclaration(_)
        | InterfaceBodyMember::AnnotationInterfaceDeclaration(_)
        | InterfaceBodyMember::EmptyDeclaration(_) => MemberCategory::Type,
    }
}

fn annotation_member_category(member: &AnnotationInterfaceBodyMember<'_>) -> MemberCategory {
    match member {
        AnnotationInterfaceBodyMember::FieldDeclaration(_) => MemberCategory::Field,
        AnnotationInterfaceBodyMember::MethodDeclaration(_)
        | AnnotationInterfaceBodyMember::AnnotationElementDeclaration(_) => MemberCategory::Method,
        AnnotationInterfaceBodyMember::ClassDeclaration(_)
        | AnnotationInterfaceBodyMember::RecordDeclaration(_)
        | AnnotationInterfaceBodyMember::EnumDeclaration(_)
        | AnnotationInterfaceBodyMember::InterfaceDeclaration(_)
        | AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(_)
        | AnnotationInterfaceBodyMember::EmptyDeclaration(_) => MemberCategory::Type,
    }
}

impl<'source> FormattedMember<'source> {
    fn from_member(member: &ClassBodyMember<'source>, formatter: &JavaFormatter<'_>) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            ClassBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field, formatter),
            },
            ClassBodyMember::ConstructorDeclaration(constructor) => Self {
                category: Some(MemberCategory::Constructor),
                starts_after_blank_line,
                doc: format_constructor_declaration(constructor, formatter),
            },
            ClassBodyMember::CompactConstructorDeclaration(constructor) => Self {
                category: Some(MemberCategory::Constructor),
                starts_after_blank_line,
                doc: format_compact_constructor_declaration(constructor, formatter),
            },
            ClassBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method, formatter),
            },
            ClassBodyMember::StaticInitializer(member) => Self {
                category: Some(MemberCategory::Initializer),
                starts_after_blank_line,
                doc: concat([
                    member
                        .static_token()
                        .as_ref()
                        .map_or_else(jolt_fmt_ir::nil, |token| {
                            concat([format_token_with_comments(token), space()])
                        }),
                    member
                        .body()
                        .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body, formatter)),
                ]),
            },
            ClassBodyMember::InstanceInitializer(member) => Self {
                category: Some(MemberCategory::Initializer),
                starts_after_blank_line,
                doc: member
                    .body()
                    .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body, formatter)),
            },
            ClassBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class, formatter),
            },
            ClassBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record, formatter),
            },
            ClassBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_, formatter),
            },
            ClassBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface, formatter),
            },
            ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation, formatter),
            },
            ClassBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_comments(comments_from_tokens(member.token_iter()))
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }

    fn from_interface_member(
        member: &InterfaceBodyMember<'source>,
        formatter: &JavaFormatter<'_>,
    ) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            InterfaceBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field, formatter),
            },
            InterfaceBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method, formatter),
            },
            InterfaceBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class, formatter),
            },
            InterfaceBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record, formatter),
            },
            InterfaceBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_, formatter),
            },
            InterfaceBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface, formatter),
            },
            InterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation, formatter),
            },
            InterfaceBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_comments(comments_from_tokens(member.token_iter()))
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }

    fn from_annotation_member(
        member: &AnnotationInterfaceBodyMember<'source>,
        formatter: &JavaFormatter<'_>,
    ) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            AnnotationInterfaceBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field, formatter),
            },
            AnnotationInterfaceBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method, formatter),
            },
            AnnotationInterfaceBodyMember::AnnotationElementDeclaration(member) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_annotation_element_declaration(member, formatter),
            },
            AnnotationInterfaceBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class, formatter),
            },
            AnnotationInterfaceBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record, formatter),
            },
            AnnotationInterfaceBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_, formatter),
            },
            AnnotationInterfaceBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface, formatter),
            },
            AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation, formatter),
            },
            AnnotationInterfaceBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_comments(comments_from_tokens(member.token_iter()))
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }
}
