use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, group, hard_line, line, text};
use jolt_java_syntax::{
    AnnotationElementDeclaration, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    BlockStatement, ClassBody, ClassBodyMember, ClassDeclaration, ConstructorInvocation,
    EnumConstant, EnumConstantListEntry, EnumDeclaration, ExtendsClause, FormalParameterList,
    ImplementsClause, InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaSyntaxToken,
    MethodDeclaration, ModifierList, PermitsClause, PermitsClauseEntry, RecordBody,
    RecordComponentList, RecordDeclaration, ThrowsClause, ThrowsClauseEntry, Type, TypeClauseEntry,
    TypeDeclaration,
};

use crate::helpers::blocks::{BodyItem, braced_body, join_body_items};
use crate::helpers::comments::{
    comment_forces_line, comment_is_star_block, format_comment, format_construct_leading_comments,
    format_dangling_comments, format_leading_comments, format_removed_token_comments,
    format_token_sequence, format_token_text, format_trailing_comments,
    format_trailing_comments_before_line_break, non_formatter_control_comments, token_has_comments,
};
use crate::helpers::declarations::{declaration_with_body, declaration_without_body};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_control_marker, relative_token_range,
};
use crate::helpers::lists::{CommaListItem, parenthesized_list};
use crate::helpers::member_body::{
    MemberBodyCategory as MemberCategory, MemberBodyItem as FormattedMember,
    join_member_body as join_member_docs,
};
use crate::rules::annotations::format_annotation_element_value;
use crate::rules::expressions::{format_argument_list, format_expression};
use crate::rules::modifiers::{
    format_modifier_prefix, format_modifier_prefix_from_parts, format_typed_modifier_prefix,
};
use crate::rules::names::format_name;
use crate::rules::statements::{
    format_block, format_block_body, format_block_statement_item, format_statement_semicolon,
};
use crate::rules::types::{
    format_array_dimensions, format_inline_annotations, format_type, format_type_argument_list,
    format_type_parameter_list, format_type_without_leading_comments,
};
use crate::rules::variables::{
    format_field_declaration, format_formal_parameter, format_receiver_parameter,
    format_record_component,
};

pub(crate) fn format_type_declaration(declaration: &TypeDeclaration) -> Doc {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class),
        TypeDeclaration::InterfaceDeclaration(interface) => format_interface_declaration(interface),
        TypeDeclaration::RecordDeclaration(record) => format_record_declaration(record),
        TypeDeclaration::EnumDeclaration(enum_) => format_enum_declaration(enum_),
        TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation)
        }
    }
}

pub(crate) fn format_anonymous_class_body(body: &ClassBody) -> Doc {
    braced_body(format_class_body(body))
}

fn format_class_declaration(class: &ClassDeclaration) -> Doc {
    format_type_declaration_with_body(
        &class.tokens(),
        class.modifiers(),
        concat([
            text("class "),
            class
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(class.type_parameters()),
            format_extends_clause(class.extends_clause()),
            format_implements_clause(class.implements_clause()),
            format_permits_clause(class.permits_clause()),
        ]),
        class.body().and_then(|body| format_class_body(&body)),
    )
}

fn format_interface_declaration(interface: &InterfaceDeclaration) -> Doc {
    format_type_declaration_with_body(
        &interface.tokens(),
        interface.modifiers(),
        concat([
            text("interface "),
            interface
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(interface.type_parameters()),
            format_extends_clause(interface.extends_clause()),
            format_permits_clause(interface.permits_clause()),
        ]),
        interface
            .body()
            .and_then(|body| format_interface_body(&body)),
    )
}

fn format_record_declaration(record: &RecordDeclaration) -> Doc {
    format_type_declaration_with_body(
        &record.tokens(),
        record.modifiers(),
        group(concat([
            text("record "),
            record
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(record.type_parameters()),
            format_record_components(record.components()),
            format_implements_clause(record.implements_clause()),
        ])),
        record.body().and_then(|body| format_record_body(&body)),
    )
}

fn format_enum_declaration(enum_: &EnumDeclaration) -> Doc {
    let constants = enum_
        .body()
        .and_then(|body| body.constants())
        .map(|constants| {
            constants
                .entries()
                .map(format_enum_constant_entry)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let body_doc = enum_
        .body()
        .and_then(|body| format_enum_body_contents(constants, &body));

    format_type_declaration_with_body(
        &enum_.tokens(),
        enum_.modifiers(),
        concat([
            text("enum "),
            enum_
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_implements_clause(enum_.implements_clause()),
        ]),
        body_doc,
    )
}

fn format_annotation_interface_declaration(annotation: &AnnotationInterfaceDeclaration) -> Doc {
    format_type_declaration_with_body(
        &annotation.tokens(),
        annotation.modifiers(),
        concat([
            text("@interface "),
            annotation
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
        ]),
        annotation
            .body()
            .and_then(|body| format_annotation_interface_body(&body)),
    )
}

fn format_class_body(body: &ClassBody) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    format_class_member_body(
        &body.source_text(),
        body.text_range().start().get(),
        &members,
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
    )
}

fn format_record_body(body: &RecordBody) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    format_class_member_body(
        &body.source_text(),
        body.text_range().start().get(),
        &members,
        format_body_open_dangling_comments(body.open_brace()),
        format_body_close_dangling_comments(body.close_brace()),
    )
}

fn format_interface_body(body: &InterfaceBody) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    let effective_members = printable_interface_members(&members);
    let member_ranges = effective_members
        .iter()
        .map(|member| interface_member_token_range(member, body.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(&body.source_text());
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted = Vec::new();
    formatted.extend(format_body_open_dangling_comments(body.open_brace()));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in effective_members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted.push(FormattedMember::ignored(
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

        let mut formatted_member = FormattedMember::from_interface_member(member);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_interface_member_category(run, &effective_members),
        ));
        ignored_index += 1;
    }
    formatted.extend(format_body_close_dangling_comments(body.close_brace()));

    (!formatted.is_empty()).then(|| join_member_docs(formatted))
}

fn format_annotation_interface_body(
    body: &jolt_java_syntax::AnnotationInterfaceBody,
) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    let members = printable_annotation_members(&members);
    let member_ranges = members
        .iter()
        .map(|member| annotation_member_token_range(member, body.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(&body.source_text());
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted = Vec::new();
    formatted.extend(format_body_open_dangling_comments(body.open_brace()));
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted.push(FormattedMember::ignored(
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

        let mut formatted_member = FormattedMember::from_annotation_member(member);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_annotation_member_category(run, &members),
        ));
        ignored_index += 1;
    }
    formatted.extend(format_body_close_dangling_comments(body.close_brace()));

    (!formatted.is_empty()).then(|| join_member_docs(formatted))
}

fn format_class_member_body(
    source: &str,
    body_start: usize,
    members: &[ClassBodyMember],
    open_dangling_comments: Option<FormattedMember>,
    close_dangling_comments: Option<FormattedMember>,
) -> Option<Doc> {
    let effective_members = effective_members(members);
    let ignored_ranges = formatter_ignore_ranges(source);
    let member_ranges = effective_members
        .iter()
        .map(|member| class_member_token_range(member, body_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &member_ranges);
    let mut formatted = Vec::new();
    formatted.extend(open_dangling_comments);
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (member_index, member) in effective_members.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == member_index
        {
            let run = &ignored_runs[ignored_index];
            formatted.push(FormattedMember::ignored(
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

        let mut formatted_member = FormattedMember::from_member(member);
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == member_index {
            formatted_member = formatted_member.without_blank_line_before();
        }
        formatted.push(formatted_member);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        formatted.push(FormattedMember::ignored(
            formatter_ignore_run_doc(run),
            ignored_member_category(run, &effective_members),
        ));
        ignored_index += 1;
    }
    formatted.extend(close_dangling_comments);

    (!formatted.is_empty()).then(|| join_member_docs(formatted))
}

fn class_member_token_range(member: &ClassBodyMember, body_start: usize) -> Option<Range<usize>> {
    let tokens = member.tokens();
    relative_token_range(&tokens, body_start)
}

fn interface_member_token_range(
    member: &InterfaceBodyMember,
    body_start: usize,
) -> Option<Range<usize>> {
    let tokens = member.tokens();
    relative_token_range(&tokens, body_start)
}

fn annotation_member_token_range(
    member: &AnnotationInterfaceBodyMember,
    body_start: usize,
) -> Option<Range<usize>> {
    let tokens = member.tokens();
    relative_token_range(&tokens, body_start)
}

fn ignored_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[ClassBodyMember],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, member_category)
}

fn ignored_interface_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[InterfaceBodyMember],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, interface_member_category)
}

fn ignored_annotation_member_category(
    run: &crate::helpers::formatter_ignore::FormatterIgnoreRun,
    members: &[AnnotationInterfaceBodyMember],
) -> MemberCategory {
    members
        .get(run.skip_start)
        .map_or(MemberCategory::Type, annotation_member_category)
}

fn format_type_declaration_with_body(
    tokens: &[jolt_java_syntax::JavaSyntaxToken],
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    body: Option<Doc>,
) -> Doc {
    declaration_with_body(
        concat([
            format_construct_leading_comments(tokens),
            format_modifier_prefix(modifiers),
        ]),
        header_tail,
        body,
    )
}

struct FormattedEnumConstant {
    doc: Doc,
    comma: Option<JavaSyntaxToken>,
}

fn format_enum_body_contents(
    constants: Vec<FormattedEnumConstant>,
    body: &jolt_java_syntax::EnumBody,
) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    let enum_semicolons = body.semicolon_tokens().collect::<Vec<_>>();
    let effective_members = effective_members(&members);
    let has_body_declarations = effective_members
        .iter()
        .any(|member| !matches!(member, ClassBodyMember::EmptyDeclaration(_)));
    let open_comments = combine_comment_members(
        combine_comment_members(
            format_body_open_dangling_comments(body.open_brace()),
            format_enum_body_semicolon_comments(&enum_semicolons),
        ),
        format_empty_enum_constant_list_comments(body.constants()),
    );
    let close_comments = format_body_close_dangling_comments(body.close_brace());
    if constants.is_empty()
        && effective_members.is_empty()
        && open_comments.is_none()
        && close_comments.is_none()
    {
        return None;
    }

    let mut moved_member_comments = Vec::new();
    let constants_doc = (!constants.is_empty()).then(|| {
        let constants_len = constants.len();
        let mut pending_constant_comments = Vec::new();
        let mut constant_lines = Vec::new();
        for (index, entry) in constants.into_iter().enumerate() {
            if !pending_constant_comments.is_empty() {
                constant_lines.push(format_dangling_comments(std::mem::take(
                    &mut pending_constant_comments,
                )));
            }

            let is_last_constant = index + 1 == constants_len;
            let separator = if !has_body_declarations || !is_last_constant {
                ","
            } else {
                ";"
            };
            let moved_comments = entry.comma.as_ref().map_or_else(Vec::new, |comma| {
                enum_separator_moved_comments(comma, has_body_declarations && is_last_constant)
            });
            if has_body_declarations && is_last_constant {
                moved_member_comments.extend(moved_comments);
            } else {
                pending_constant_comments.extend(moved_comments);
            }

            constant_lines.push(concat([
                entry.doc,
                format_enum_constant_separator(
                    entry.comma.as_ref(),
                    separator,
                    !has_body_declarations || !is_last_constant,
                ),
            ]));
        }

        if !pending_constant_comments.is_empty() {
            constant_lines.push(format_dangling_comments(pending_constant_comments));
        }

        join_docs(constant_lines, &hard_line())
    });

    let moved_member_comments = (!moved_member_comments.is_empty())
        .then(|| format_dangling_comments(moved_member_comments));
    let members_doc = format_class_member_body(
        &body.source_text(),
        body.text_range().start().get(),
        &members,
        open_comments,
        close_comments,
    );
    let members_doc = match (moved_member_comments, members_doc) {
        (Some(comments), Some(members)) => Some(concat([comments, hard_line(), members])),
        (Some(comments), None) => Some(comments),
        (None, members) => members,
    };

    match (constants_doc, members_doc) {
        (Some(constants), Some(members)) => {
            Some(concat([constants, jolt_fmt_ir::empty_line(), members]))
        }
        (Some(constants), None) => Some(constants),
        (None, Some(members)) if has_body_declarations => {
            Some(concat([text(";"), jolt_fmt_ir::empty_line(), members]))
        }
        (None, Some(members)) => Some(members),
        (None, None) => None,
    }
}

fn format_enum_constant_entry(entry: EnumConstantListEntry) -> FormattedEnumConstant {
    FormattedEnumConstant {
        doc: format_enum_constant(&entry.constant),
        comma: entry.comma,
    }
}

fn format_enum_constant_separator(
    comma: Option<&JavaSyntaxToken>,
    separator: &'static str,
    include_trailing_comments: bool,
) -> Doc {
    comma.map_or_else(
        || text(separator),
        |comma| {
            concat([
                format_leading_comments(comma),
                text(separator),
                if include_trailing_comments {
                    format_enum_separator_inline_trailing_comments(comma)
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        },
    )
}

fn format_enum_separator_inline_trailing_comments(comma: &JavaSyntaxToken) -> Doc {
    let comments = comma
        .trailing_comments()
        .into_iter()
        .filter(|comment| !enum_separator_comment_moves(comment))
        .collect::<Vec<_>>();

    let mut docs = Vec::new();
    for comment in comments {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

fn enum_separator_moved_comments(
    comma: &JavaSyntaxToken,
    move_all_trailing_comments: bool,
) -> Vec<jolt_java_syntax::JavaComment> {
    comma
        .trailing_comments()
        .into_iter()
        .filter(|comment| {
            !is_formatter_control_marker(comment.text())
                && (move_all_trailing_comments || enum_separator_comment_moves(comment))
        })
        .collect()
}

fn enum_separator_comment_moves(comment: &jolt_java_syntax::JavaComment) -> bool {
    comment.kind() != jolt_java_syntax::JavaCommentKind::Line
        && (comment_forces_line(comment) || comment_is_star_block(comment))
}

fn format_enum_constant(constant: &EnumConstant) -> Doc {
    let tokens = constant.tokens();
    let Some(name) = constant.name() else {
        return format_token_sequence(&tokens);
    };

    concat([
        format_modifier_prefix_from_parts(constant.annotations().collect(), Vec::new()),
        format_leading_comments(&name),
        format_token_text(name.text()),
        format_trailing_comments(&name),
        constant
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_argument_list(Some(arguments))
            }),
        constant.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([text(" "), braced_body(format_class_body(&body))])
        }),
    ])
}

fn effective_members(members: &[ClassBodyMember]) -> Vec<ClassBodyMember> {
    printable_class_members(members)
}

fn printable_class_members(members: &[ClassBodyMember]) -> Vec<ClassBodyMember> {
    members
        .iter()
        .filter(|member| is_printable_class_member(member))
        .cloned()
        .collect()
}

fn printable_interface_members(members: &[InterfaceBodyMember]) -> Vec<InterfaceBodyMember> {
    members
        .iter()
        .filter(|member| is_printable_interface_member(member))
        .cloned()
        .collect()
}

fn printable_annotation_members(
    members: &[AnnotationInterfaceBodyMember],
) -> Vec<AnnotationInterfaceBodyMember> {
    members
        .iter()
        .filter(|member| is_printable_annotation_member(member))
        .cloned()
        .collect()
}

fn is_printable_class_member(member: &ClassBodyMember) -> bool {
    !matches!(member, ClassBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration(member.tokens().as_slice()).is_some()
}

fn is_printable_interface_member(member: &InterfaceBodyMember) -> bool {
    !matches!(member, InterfaceBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration(member.tokens().as_slice()).is_some()
}

fn is_printable_annotation_member(member: &AnnotationInterfaceBodyMember) -> bool {
    !matches!(member, AnnotationInterfaceBodyMember::EmptyDeclaration(_))
        || format_removed_empty_declaration(member.tokens().as_slice()).is_some()
}

fn format_removed_empty_declaration(tokens: &[JavaSyntaxToken]) -> Option<Doc> {
    format_removed_token_comments(tokens)
}

fn format_body_open_dangling_comments(open: Option<JavaSyntaxToken>) -> Option<FormattedMember> {
    let comments = non_formatter_control_comments(open?.trailing_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

fn format_body_close_dangling_comments(close: Option<JavaSyntaxToken>) -> Option<FormattedMember> {
    let comments = non_formatter_control_comments(close?.leading_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

fn format_empty_enum_constant_list_comments(
    constants: Option<jolt_java_syntax::EnumConstantList>,
) -> Option<FormattedMember> {
    let constants = constants?;
    if constants.constants().next().is_some() {
        return None;
    }

    format_removed_empty_declaration(constants.tokens().as_slice()).map(FormattedMember::comment)
}

fn format_enum_body_semicolon_comments(semicolons: &[JavaSyntaxToken]) -> Option<FormattedMember> {
    format_removed_empty_declaration(semicolons).map(FormattedMember::comment)
}

fn combine_comment_members(
    first: Option<FormattedMember>,
    second: Option<FormattedMember>,
) -> Option<FormattedMember> {
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

fn member_category(member: &ClassBodyMember) -> MemberCategory {
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

fn interface_member_category(member: &InterfaceBodyMember) -> MemberCategory {
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

fn annotation_member_category(member: &AnnotationInterfaceBodyMember) -> MemberCategory {
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

fn format_record_components(components: Option<RecordComponentList>) -> Doc {
    let Some(components) = components else {
        return text("()");
    };
    let open = components.open_paren();
    let close = components.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        components
            .entries()
            .map(|entry| CommaListItem {
                doc: format_record_component(&entry.component),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_extends_clause(clause: Option<ExtendsClause>) -> Doc {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "extends",
        clause.entries().collect::<Vec<_>>(),
    )
}

fn format_implements_clause(clause: Option<ImplementsClause>) -> Doc {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "implements",
        clause.entries().collect::<Vec<_>>(),
    )
}

fn format_permits_clause(clause: Option<PermitsClause>) -> Doc {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_permits_header_clause(
        keyword.as_ref(),
        "permits",
        clause.entries().collect::<Vec<_>>(),
    )
}

fn format_type_header_clause(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    entries: Vec<TypeClauseEntry>,
) -> Doc {
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    let should_break = header_keyword_forces_line(keyword)
        || entries.iter().any(|entry| {
            type_has_leading_comments(&entry.ty)
                || entry.comma.as_ref().is_some_and(token_has_comments)
        });

    if should_break {
        return jolt_fmt_ir::indent(concat([
            line(),
            format_header_clause_keyword(keyword, fallback),
            jolt_fmt_ir::indent(concat([
                format_header_clause_keyword_break(keyword),
                format_type_clause_entries_broken(entries),
            ])),
        ]));
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_header_clause_keyword(keyword, fallback),
        text(" "),
        format_type_clause_entries_inline(entries),
    ]))
}

fn format_permits_header_clause(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    entries: Vec<PermitsClauseEntry>,
) -> Doc {
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    let should_break = header_keyword_forces_line(keyword)
        || entries.iter().any(|entry| {
            name_has_leading_comments(&entry.name)
                || entry.comma.as_ref().is_some_and(token_has_comments)
        });

    if should_break {
        return jolt_fmt_ir::indent(concat([
            line(),
            format_header_clause_keyword(keyword, fallback),
            jolt_fmt_ir::indent(concat([
                format_header_clause_keyword_break(keyword),
                format_permits_clause_entries_broken(entries),
            ])),
        ]));
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_header_clause_keyword(keyword, fallback),
        text(" "),
        format_permits_clause_entries_inline(entries),
    ]))
}

fn format_header_clause_keyword(keyword: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                format_token_text(keyword.text()),
                format_trailing_comments_before_line_break(keyword),
            ])
        },
    )
}

fn format_header_clause_keyword_break(keyword: Option<&JavaSyntaxToken>) -> Doc {
    if header_keyword_forces_line(keyword) {
        hard_line()
    } else {
        line()
    }
}

fn header_keyword_forces_line(keyword: Option<&JavaSyntaxToken>) -> bool {
    keyword.is_some_and(|keyword| keyword.trailing_comments().iter().any(comment_forces_line))
}

fn format_type_clause_entries_inline(entries: Vec<TypeClauseEntry>) -> Doc {
    let mut docs = Vec::new();

    for entry in entries {
        docs.push(format_type(&entry.ty));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_inline(&comma));
        }
    }

    concat(docs)
}

fn format_type_clause_entries_broken(entries: Vec<TypeClauseEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(&entry.ty.tokens()),
            format_type_without_leading_comments(&entry.ty),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_broken(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_permits_clause_entries_inline(entries: Vec<PermitsClauseEntry>) -> Doc {
    let mut docs = Vec::new();

    for entry in entries {
        docs.push(format_name(&entry.name));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_inline(&comma));
        }
    }

    concat(docs)
}

fn format_permits_clause_entries_broken(entries: Vec<PermitsClauseEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(&entry.name.tokens()),
            format_name(&entry.name),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_broken(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_header_clause_separator_inline(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        text(" "),
    ])
}

fn format_header_clause_separator_broken(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn type_has_leading_comments(ty: &Type) -> bool {
    ty.tokens()
        .first()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn name_has_leading_comments(name: &jolt_java_syntax::NameSyntax) -> bool {
    name.tokens()
        .first()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn join_docs(docs: Vec<Doc>, separator: &Doc) -> Doc {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(separator.clone());
        }
        joined.push(doc);
    }
    concat(joined)
}

impl FormattedMember {
    fn from_member(member: &ClassBodyMember) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            ClassBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field),
            },
            ClassBodyMember::ConstructorDeclaration(constructor) => Self {
                category: Some(MemberCategory::Constructor),
                starts_after_blank_line,
                doc: format_constructor_declaration(constructor),
            },
            ClassBodyMember::CompactConstructorDeclaration(constructor) => Self {
                category: Some(MemberCategory::Constructor),
                starts_after_blank_line,
                doc: format_compact_constructor_declaration(constructor),
            },
            ClassBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method),
            },
            ClassBodyMember::StaticInitializer(member) => Self {
                category: Some(MemberCategory::Initializer),
                starts_after_blank_line,
                doc: concat([
                    text("static "),
                    member
                        .body()
                        .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body)),
                ]),
            },
            ClassBodyMember::InstanceInitializer(member) => Self {
                category: Some(MemberCategory::Initializer),
                starts_after_blank_line,
                doc: member
                    .body()
                    .map_or_else(jolt_fmt_ir::nil, |body| format_block(&body)),
            },
            ClassBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class),
            },
            ClassBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record),
            },
            ClassBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_),
            },
            ClassBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface),
            },
            ClassBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation),
            },
            ClassBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_empty_declaration(member.tokens().as_slice())
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }

    fn from_interface_member(member: &InterfaceBodyMember) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            InterfaceBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field),
            },
            InterfaceBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method),
            },
            InterfaceBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class),
            },
            InterfaceBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record),
            },
            InterfaceBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_),
            },
            InterfaceBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface),
            },
            InterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation),
            },
            InterfaceBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_empty_declaration(member.tokens().as_slice())
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }

    fn from_annotation_member(member: &AnnotationInterfaceBodyMember) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            AnnotationInterfaceBodyMember::FieldDeclaration(field) => Self {
                category: Some(MemberCategory::Field),
                starts_after_blank_line,
                doc: format_field_declaration(field),
            },
            AnnotationInterfaceBodyMember::MethodDeclaration(method) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_method_declaration(method),
            },
            AnnotationInterfaceBodyMember::AnnotationElementDeclaration(member) => Self {
                category: Some(MemberCategory::Method),
                starts_after_blank_line,
                doc: format_annotation_element_declaration(member),
            },
            AnnotationInterfaceBodyMember::ClassDeclaration(class) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_class_declaration(class),
            },
            AnnotationInterfaceBodyMember::RecordDeclaration(record) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_record_declaration(record),
            },
            AnnotationInterfaceBodyMember::EnumDeclaration(enum_) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_enum_declaration(enum_),
            },
            AnnotationInterfaceBodyMember::InterfaceDeclaration(interface) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_interface_declaration(interface),
            },
            AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(annotation) => Self {
                category: Some(MemberCategory::Type),
                starts_after_blank_line,
                doc: format_annotation_interface_declaration(annotation),
            },
            AnnotationInterfaceBodyMember::EmptyDeclaration(_) => Self {
                category: None,
                starts_after_blank_line,
                doc: format_removed_empty_declaration(member.tokens().as_slice())
                    .unwrap_or_else(jolt_fmt_ir::nil),
            },
        }
    }
}

fn format_constructor_declaration(constructor: &jolt_java_syntax::ConstructorDeclaration) -> Doc {
    let Some(name) = constructor.name() else {
        return format_token_sequence(&constructor.tokens());
    };
    let prefix = concat([
        format_construct_leading_comments(&constructor.tokens()),
        format_modifier_prefix(constructor.modifiers()),
    ]);
    let throws = constructor.throws_clause();
    let has_throws = throws
        .as_ref()
        .is_some_and(|throws| throws.exceptions().next().is_some());
    let header = concat([
        format_type_parameter_list(constructor.type_parameters()),
        format_token_text(name.text()),
        format_parameters(constructor.parameters()),
        format_throws_clause(throws),
    ]);

    match constructor.body() {
        Some(body) if has_throws => {
            declaration_with_body(prefix, header, format_constructor_body(&body))
        }
        Some(body) => {
            callable_declaration_with_body(prefix, header, format_constructor_body(&body))
        }
        None => declaration_without_body(prefix, header),
    }
}

fn format_compact_constructor_declaration(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration,
) -> Doc {
    let prefix = format_modifier_prefix(constructor.modifiers());
    let header = constructor
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text()));

    match constructor.body() {
        Some(body) => declaration_with_body(prefix, header, format_constructor_body(&body)),
        None => declaration_without_body(prefix, header),
    }
}

fn format_method_declaration(method: &MethodDeclaration) -> Doc {
    let Some(name) = method.name() else {
        return format_token_sequence(&method.tokens());
    };
    let modifiers = format_typed_modifier_prefix(method.modifiers());
    let prefix = concat([
        format_construct_leading_comments(&method.tokens()),
        modifiers.declaration_prefix,
    ]);
    let throws = method.throws_clause();
    let has_throws = throws
        .as_ref()
        .is_some_and(|throws| throws.exceptions().next().is_some());
    let header = concat([
        format_type_parameter_list(method.type_parameters()),
        modifiers.type_use_prefix,
        format_inline_annotations(method.return_type_annotations().collect()),
        method
            .return_type()
            .map_or_else(jolt_fmt_ir::nil, |return_type| {
                concat([
                    format_type_without_leading_comments(&return_type),
                    text(" "),
                ])
            }),
        format_token_text(name.text()),
        format_parameters(method.parameters()),
        format_throws_clause(throws),
    ]);

    match method.body() {
        Some(body) if has_throws => declaration_with_body(prefix, header, format_block_body(&body)),
        Some(body) => callable_declaration_with_body(prefix, header, format_block_body(&body)),
        None => concat([
            prefix,
            group(header),
            format_statement_semicolon(method.semicolon()),
        ]),
    }
}

fn format_annotation_element_declaration(element: &AnnotationElementDeclaration) -> Doc {
    let Some(name) = element.name() else {
        return format_token_sequence(&element.tokens());
    };

    concat([
        group(concat([
            format_modifier_prefix(element.modifiers()),
            element
                .ty()
                .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
            text(" "),
            format_token_text(name.text()),
            text("()"),
            element
                .dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions)
                }),
            format_annotation_element_default(element.default_value()),
        ])),
        format_statement_semicolon(element.semicolon()),
    ])
}

fn format_annotation_element_default(default: Option<jolt_java_syntax::DefaultValue>) -> Doc {
    default.map_or_else(jolt_fmt_ir::nil, |default| {
        concat([
            text(" "),
            text("default "),
            default.value().map_or_else(jolt_fmt_ir::nil, |value| {
                format_annotation_element_value(&value)
            }),
        ])
    })
}

fn format_parameters(parameters: Option<FormalParameterList>) -> Doc {
    let Some(parameters) = parameters else {
        return text("()");
    };
    let open = parameters.open_paren();
    let close = parameters.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        parameters
            .entries()
            .map(|entry| CommaListItem {
                doc: match entry.item {
                    jolt_java_syntax::FormalParameterListItem::ReceiverParameter(parameter) => {
                        format_receiver_parameter(&parameter)
                    }
                    jolt_java_syntax::FormalParameterListItem::FormalParameter(parameter) => {
                        format_formal_parameter(&parameter)
                    }
                },
                comma: entry.comma,
            })
            .collect(),
    )
}

fn callable_declaration_with_body(prefix: Doc, header: Doc, body: Option<Doc>) -> Doc {
    concat([prefix, group(header), text(" "), braced_body(body)])
}

fn format_throws_clause(throws: Option<ThrowsClause>) -> Doc {
    let Some(throws) = throws else {
        return jolt_fmt_ir::nil();
    };
    let entries = throws.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_throws_keyword(&throws),
        format_throws_keyword_spacing(&throws),
        format_throws_entries(entries),
    ]))
}

fn format_throws_keyword(throws: &ThrowsClause) -> Doc {
    throws.keyword().map_or_else(
        || text("throws"),
        |keyword| {
            concat([
                format_leading_comments(&keyword),
                text("throws"),
                format_trailing_comments_before_line_break(&keyword),
            ])
        },
    )
}

fn format_throws_keyword_spacing(throws: &ThrowsClause) -> Doc {
    if throws
        .keyword()
        .is_some_and(|keyword| keyword.trailing_comments().iter().any(comment_forces_line))
    {
        hard_line()
    } else {
        text(" ")
    }
}

fn format_throws_entries(entries: Vec<ThrowsClauseEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_type(&entry.exception));
        if let Some(comma) = entry.comma {
            docs.push(format_throws_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_throws_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn format_constructor_body(body: &jolt_java_syntax::ConstructorBody) -> Option<Doc> {
    let elements = constructor_body_elements(body);
    let element_ranges = elements
        .iter()
        .map(|element| {
            constructor_body_element_token_range(element, body.text_range().start().get())
        })
        .collect::<Vec<_>>();
    let ignored_ranges = formatter_ignore_ranges(&body.source_text());
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &element_ranges);
    let mut items = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (element_index, element) in elements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == element_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= element_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(element_index) {
            continue;
        }

        let Some(mut item) = format_constructor_body_element(element) else {
            continue;
        };
        if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == element_index {
            item = item.without_blank_line_before();
        }
        items.push(item);
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
        ignored_index += 1;
    }

    (!items.is_empty()).then(|| join_body_items(items))
}

fn constructor_body_elements(
    body: &jolt_java_syntax::ConstructorBody,
) -> Vec<ConstructorBodyElement> {
    body.invocation()
        .into_iter()
        .map(ConstructorBodyElement::Invocation)
        .chain(
            body.block_statements()
                .map(ConstructorBodyElement::BlockStatement),
        )
        .collect()
}

fn constructor_body_element_token_range(
    element: &ConstructorBodyElement,
    body_start: usize,
) -> Option<Range<usize>> {
    let tokens = element.tokens();
    relative_token_range(&tokens, body_start)
}

fn format_constructor_body_element(element: &ConstructorBodyElement) -> Option<BodyItem> {
    match element {
        ConstructorBodyElement::Invocation(invocation) => Some(BodyItem::new(
            format_constructor_invocation(invocation),
            invocation.starts_after_blank_line(),
        )),
        ConstructorBodyElement::BlockStatement(statement) => format_block_statement_item(statement),
    }
}

fn format_constructor_invocation(invocation: &ConstructorInvocation) -> Doc {
    concat([
        format_construct_leading_comments(&invocation.tokens()),
        format_constructor_invocation_qualifier(invocation),
        invocation
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments)
            }),
        invocation
            .target()
            .map_or_else(jolt_fmt_ir::nil, |target| format_token_text(target.text())),
        format_argument_list(invocation.arguments()),
        format_statement_semicolon(invocation.semicolon()),
    ])
}

fn format_constructor_invocation_qualifier(invocation: &ConstructorInvocation) -> Doc {
    if let Some(name) = invocation.qualifier_name() {
        return concat([format_name(&name), text(".")]);
    }
    invocation
        .qualifier_expression()
        .map_or_else(jolt_fmt_ir::nil, |expression| {
            concat([format_expression(&expression), text(".")])
        })
}

enum ConstructorBodyElement {
    Invocation(ConstructorInvocation),
    BlockStatement(BlockStatement),
}

impl ConstructorBodyElement {
    fn tokens(&self) -> Vec<jolt_java_syntax::JavaSyntaxToken> {
        match self {
            Self::Invocation(invocation) => invocation.tokens(),
            Self::BlockStatement(statement) => statement.tokens(),
        }
    }
}
