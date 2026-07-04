use super::{
    AnnotationInterfaceBodyMember, ClassBody, ClassBodyMember, Doc, FormattedMember, InterfaceBody,
    InterfaceBodyMember, JavaFormatter, JavaSyntaxToken, MemberCategory, Range, RecordBody,
    comments_from_tokens, concat, format_annotation_element_declaration,
    format_annotation_interface_declaration, format_block, format_class_declaration,
    format_compact_constructor_declaration, format_constructor_declaration,
    format_dangling_comments, format_enum_declaration, format_field_declaration,
    format_interface_declaration, format_method_declaration, format_record_declaration,
    format_removed_comments, format_removed_token_comments, format_token_with_comments,
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs, hard_line,
    join_member_docs, non_formatter_control_comments, relative_token_range_between, text,
};

pub(super) fn format_class_body<'source>(
    body: &ClassBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = body.members().collect::<Vec<_>>();
    format_class_member_body(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
        &members,
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_record_body<'source>(
    body: &RecordBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = body.members().collect::<Vec<_>>();
    format_class_member_body(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
        &members,
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
        formatter,
    )
}

pub(super) fn format_interface_body<'source>(
    body: &InterfaceBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = body.members().collect::<Vec<_>>();
    let effective_members = printable_interface_members(&members);
    let member_ranges = effective_members
        .iter()
        .map(|member| interface_member_token_range(member, body.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted_members = Vec::new();
    formatted_members.extend(format_body_open_dangling_comments(body.open_brace()));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in effective_members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted_members.push(FormattedMember::ignored(
                formatter_ignore_run_doc(run),
                ignored_interface_member_category(run, &effective_members),
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= member_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(member_index) {
            continue;
        }

        let mut formatted_member = FormattedMember::from_interface_member(member, formatter);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted_members.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted_members.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_interface_member_category(run, &effective_members),
        ));
        ignored_index += 1;
    }
    formatted_members.extend(format_body_close_dangling_comments(body.close_brace()));

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

pub(super) fn format_annotation_interface_body<'source>(
    body: &jolt_java_syntax::AnnotationInterfaceBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let members = body.members().collect::<Vec<_>>();
    let members = printable_annotation_members(&members);
    let member_ranges = members
        .iter()
        .map(|member| annotation_member_token_range(member, body.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted_members = Vec::new();
    formatted_members.extend(format_body_open_dangling_comments(body.open_brace()));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted_members.push(FormattedMember::ignored(
                formatter_ignore_run_doc(run),
                ignored_annotation_member_category(run, &members),
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= member_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(member_index) {
            continue;
        }

        let mut formatted_member = FormattedMember::from_annotation_member(member, formatter);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted_members.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted_members.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_annotation_member_category(run, &members),
        ));
        ignored_index += 1;
    }
    formatted_members.extend(format_body_close_dangling_comments(body.close_brace()));

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
}

pub(super) fn format_class_member_body<'source>(
    source: &'source str,
    body_start: usize,
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
    members: &[ClassBodyMember<'source>],
    open_dangling_comments: Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let effective_members = effective_members(members);
    let ignored_ranges = formatter_ignore_ranges(source, body_start, tokens);
    let member_ranges = effective_members
        .iter()
        .map(|member| class_member_token_range(member, body_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted_members = Vec::new();
    formatted_members.extend(open_dangling_comments);
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in effective_members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted_members.push(FormattedMember::ignored(
                formatter_ignore_run_doc(run),
                ignored_member_category(run, &effective_members),
            ));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= member_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(member_index) {
            continue;
        }

        let mut formatted_member = FormattedMember::from_member(member, formatter);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted_members.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted_members.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_member_category(run, &effective_members),
        ));
        ignored_index += 1;
    }
    formatted_members.extend(close_dangling_comments);

    (!formatted_members.is_empty()).then(|| join_member_docs(formatted_members))
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

fn ignored_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[ClassBodyMember<'_>],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, member_category)
}

fn ignored_interface_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[InterfaceBodyMember<'_>],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, interface_member_category)
}

fn ignored_annotation_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[AnnotationInterfaceBodyMember<'_>],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, annotation_member_category)
}

pub(super) fn effective_members<'source>(
    members: &[ClassBodyMember<'source>],
) -> Vec<ClassBodyMember<'source>> {
    printable_class_members(members)
}

fn printable_class_members<'source>(
    members: &[ClassBodyMember<'source>],
) -> Vec<ClassBodyMember<'source>> {
    members
        .iter()
        .filter(|member| is_printable_class_member(member))
        .copied()
        .collect()
}

fn printable_interface_members<'source>(
    members: &[InterfaceBodyMember<'source>],
) -> Vec<InterfaceBodyMember<'source>> {
    members
        .iter()
        .filter(|member| is_printable_interface_member(member))
        .copied()
        .collect()
}

fn printable_annotation_members<'source>(
    members: &[AnnotationInterfaceBodyMember<'source>],
) -> Vec<AnnotationInterfaceBodyMember<'source>> {
    members
        .iter()
        .filter(|member| is_printable_annotation_member(member))
        .copied()
        .collect()
}

fn is_printable_class_member(member: &ClassBodyMember<'_>) -> bool {
    !matches!(member, ClassBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration_comments(comments_from_tokens(member.token_iter()))
            .is_some()
}

fn is_printable_interface_member(member: &InterfaceBodyMember<'_>) -> bool {
    !matches!(member, InterfaceBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration_comments(comments_from_tokens(member.token_iter()))
            .is_some()
}

fn is_printable_annotation_member(member: &AnnotationInterfaceBodyMember<'_>) -> bool {
    !matches!(member, AnnotationInterfaceBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration_comments(comments_from_tokens(member.token_iter()))
            .is_some()
}

pub(super) fn format_removed_empty_declaration<'source>(
    tokens: &[JavaSyntaxToken<'source>],
) -> Option<Doc<'source>> {
    format_removed_token_comments(tokens)
}

fn format_removed_empty_declaration_comments(
    comments: Vec<jolt_java_syntax::JavaComment<'_>>,
) -> Option<Doc<'_>> {
    format_removed_comments(comments)
}

pub(super) fn format_body_open_dangling_comments(
    open: Option<JavaSyntaxToken<'_>>,
) -> Option<FormattedMember<'_>> {
    let comments = non_formatter_control_comments(open?.trailing_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

pub(super) fn format_body_close_dangling_comments(
    close: Option<JavaSyntaxToken<'_>>,
) -> Option<FormattedMember<'_>> {
    let comments = non_formatter_control_comments(close?.leading_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

pub(super) fn format_empty_enum_constant_list_comments(
    constants: Option<jolt_java_syntax::EnumConstantList<'_>>,
) -> Option<FormattedMember<'_>> {
    let constants = constants?;
    if constants.constants().next().is_some() {
        return None;
    }

    format_removed_empty_declaration_comments(comments_from_tokens(constants.token_iter()))
        .map(FormattedMember::comment)
}

pub(super) fn format_enum_body_semicolon_comments<'source>(
    semicolons: &[JavaSyntaxToken<'source>],
) -> Option<FormattedMember<'source>> {
    format_removed_empty_declaration(semicolons).map(FormattedMember::comment)
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
                            concat([format_token_with_comments(token), text(" ")])
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
                doc: format_removed_empty_declaration_comments(comments_from_tokens(
                    member.token_iter(),
                ))
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
                doc: format_removed_empty_declaration_comments(comments_from_tokens(
                    member.token_iter(),
                ))
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
                doc: format_removed_empty_declaration_comments(comments_from_tokens(
                    member.token_iter(),
                ))
                .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }
}
