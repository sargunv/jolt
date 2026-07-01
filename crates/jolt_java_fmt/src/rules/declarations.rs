use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, group, hard_line, line, soft_line, text};
use jolt_java_syntax::{
    AnnotationElementDeclaration, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    BlockStatement, ClassBody, ClassBodyMember, ClassDeclaration, ConstructorInvocation,
    EnumConstant, EnumDeclaration, ExtendsClause, FormalParameterList, ImplementsClause,
    InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaSyntaxToken, MethodDeclaration,
    ModifierList, PermitsClause, RecordBody, RecordDeclaration, Type, TypeDeclaration,
};

use crate::helpers::blocks::{BodyItem, braced_body, join_body_items};
use crate::helpers::comments::{
    format_dangling_comments, format_leading_comments, format_token_sequence,
    format_trailing_comments,
};
use crate::helpers::declarations::{declaration_with_body, declaration_without_body};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_control_marker,
};
use crate::rules::annotations::format_annotation_element_value;
use crate::rules::expressions::{format_argument_list, format_expression};
use crate::rules::modifiers::{format_modifier_prefix, format_modifier_prefix_from_parts};
use crate::rules::names::format_name;
use crate::rules::statements::{format_block, format_block_body, format_block_item};
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_argument_list, format_type_parameter_list,
    format_type_without_leading_comments,
};
use crate::rules::variables::{
    format_field_declaration, format_formal_parameter, format_record_component,
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
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
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
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
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
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
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
                .constants()
                .map(|constant| format_enum_constant(&constant))
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
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
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
                .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
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
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - body_start
            ..last.token_text_range().end().get() - body_start,
    )
}

fn interface_member_token_range(
    member: &InterfaceBodyMember,
    body_start: usize,
) -> Option<Range<usize>> {
    let tokens = member.tokens();
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - body_start
            ..last.token_text_range().end().get() - body_start,
    )
}

fn annotation_member_token_range(
    member: &AnnotationInterfaceBodyMember,
    body_start: usize,
) -> Option<Range<usize>> {
    let tokens = member.tokens();
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - body_start
            ..last.token_text_range().end().get() - body_start,
    )
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

fn format_enum_body_contents(
    constants: Vec<Doc>,
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

    let constants_doc = (!constants.is_empty()).then(|| {
        let constants_len = constants.len();
        join_docs(
            constants
                .into_iter()
                .enumerate()
                .map(|(index, constant)| {
                    let separator = if !has_body_declarations || index + 1 < constants_len {
                        ","
                    } else {
                        ";"
                    };
                    concat([constant, text(separator)])
                })
                .collect(),
            &hard_line(),
        )
    });

    let members_doc = format_class_member_body(
        &body.source_text(),
        body.text_range().start().get(),
        &members,
        open_comments,
        close_comments,
    );

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

fn format_enum_constant(constant: &EnumConstant) -> Doc {
    let tokens = constant.tokens();
    let Some(name) = constant.name() else {
        return format_token_sequence(&tokens);
    };

    concat([
        format_modifier_prefix_from_parts(constant.annotations().collect(), Vec::new()),
        format_leading_comments(&name),
        text(name.text().to_owned()),
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
    let comments = tokens
        .iter()
        .flat_map(|token| {
            let mut comments = token.leading_comments();
            comments.extend(token.trailing_comments());
            comments
        })
        .filter(|comment| !is_formatter_control_marker(comment.text()))
        .collect::<Vec<_>>();

    (!comments.is_empty()).then(|| format_dangling_comments(comments))
}

fn format_body_open_dangling_comments(open: Option<JavaSyntaxToken>) -> Option<FormattedMember> {
    let comments = non_formatter_control_comments(open?.trailing_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

fn format_body_close_dangling_comments(close: Option<JavaSyntaxToken>) -> Option<FormattedMember> {
    let comments = non_formatter_control_comments(close?.leading_comments());
    (!comments.is_empty()).then(|| FormattedMember::comment(format_dangling_comments(comments)))
}

fn non_formatter_control_comments(
    comments: Vec<jolt_java_syntax::JavaComment>,
) -> Vec<jolt_java_syntax::JavaComment> {
    comments
        .into_iter()
        .filter(|comment| !is_formatter_control_marker(comment.text()))
        .collect()
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

fn format_record_components(components: Option<jolt_java_syntax::RecordComponentList>) -> Doc {
    let Some(components) = components else {
        return text("()");
    };
    parenthesized_comma_list(
        components
            .components()
            .map(|component| format_record_component(&component))
            .collect(),
    )
}

fn format_extends_clause(clause: Option<ExtendsClause>) -> Doc {
    format_type_list_clause(
        "extends",
        clause.map(|clause| clause.types().collect::<Vec<_>>()),
    )
}

fn format_implements_clause(clause: Option<ImplementsClause>) -> Doc {
    format_type_list_clause(
        "implements",
        clause.map(|clause| clause.types().collect::<Vec<_>>()),
    )
}

fn format_permits_clause(clause: Option<PermitsClause>) -> Doc {
    format_type_clause(
        "permits",
        clause.map(|clause| {
            clause
                .names()
                .map(|name| format_name(&name))
                .collect::<Vec<_>>()
        }),
    )
}

fn format_type_list_clause(keyword: &'static str, items: Option<Vec<Type>>) -> Doc {
    let Some(items) = items else {
        return jolt_fmt_ir::nil();
    };
    if items.is_empty() {
        return jolt_fmt_ir::nil();
    }

    if items.iter().any(type_has_leading_comments) {
        let items = items
            .into_iter()
            .map(|ty| {
                concat([
                    format_construct_leading_comments(&ty.tokens()),
                    format_type_without_leading_comments(&ty),
                ])
            })
            .collect::<Vec<_>>();
        return concat([
            jolt_fmt_ir::indent(line()),
            text(keyword),
            jolt_fmt_ir::indent(concat([
                line(),
                join_docs(items, &concat([text(","), line()])),
            ])),
        ]);
    }

    format_type_clause(
        keyword,
        Some(items.into_iter().map(|ty| format_type(&ty)).collect()),
    )
}

fn format_type_clause(keyword: &'static str, items: Option<Vec<Doc>>) -> Doc {
    let Some(items) = items else {
        return jolt_fmt_ir::nil();
    };
    if items.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        jolt_fmt_ir::indent(line()),
        text(keyword),
        text(" "),
        jolt_fmt_ir::join(text(", "), items),
    ])
}

fn type_has_leading_comments(ty: &Type) -> bool {
    ty.tokens()
        .first()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn join_member_docs(members: Vec<FormattedMember>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_category = None;
    let mut previous_was_neutral = false;

    for member in members {
        if !joined.is_empty() {
            joined.push(member_separator(
                previous_category,
                member.category,
                member.starts_after_blank_line,
                previous_was_neutral,
            ));
        }
        previous_was_neutral = member.category.is_none();
        if let Some(category) = member.category {
            previous_category = Some(category);
        }
        joined.push(member.doc);
    }

    concat(joined)
}

fn member_separator(
    previous_category: Option<MemberCategory>,
    current_category: Option<MemberCategory>,
    starts_after_blank_line: bool,
    previous_was_neutral: bool,
) -> Doc {
    if previous_was_neutral {
        return hard_line();
    }
    if starts_after_blank_line {
        return jolt_fmt_ir::empty_line();
    }

    match (previous_category, current_category) {
        (Some(MemberCategory::Field), Some(MemberCategory::Field))
        | (None, Some(_))
        | (_, None) => hard_line(),
        _ => jolt_fmt_ir::empty_line(),
    }
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum MemberCategory {
    Field,
    Constructor,
    Method,
    Initializer,
    Type,
}

struct FormattedMember {
    category: Option<MemberCategory>,
    starts_after_blank_line: bool,
    doc: Doc,
}

impl FormattedMember {
    fn comment(doc: Doc) -> Self {
        Self {
            category: None,
            starts_after_blank_line: false,
            doc,
        }
    }

    fn ignored(doc: Doc, category: MemberCategory) -> Self {
        Self {
            category: Some(category),
            starts_after_blank_line: false,
            doc,
        }
    }

    fn without_blank_line_before(self) -> Self {
        Self {
            starts_after_blank_line: false,
            ..self
        }
    }

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
        text(name.text().to_owned()),
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
        .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned()));

    match constructor.body() {
        Some(body) => declaration_with_body(prefix, header, format_constructor_body(&body)),
        None => declaration_without_body(prefix, header),
    }
}

fn format_method_declaration(method: &MethodDeclaration) -> Doc {
    let Some(name) = method.name() else {
        return format_token_sequence(&method.tokens());
    };
    let prefix = concat([
        format_construct_leading_comments(&method.tokens()),
        format_modifier_prefix(method.modifiers()),
    ]);
    let throws = method.throws_clause();
    let has_throws = throws
        .as_ref()
        .is_some_and(|throws| throws.exceptions().next().is_some());
    let header = concat([
        format_type_parameter_list(method.type_parameters()),
        method
            .return_type()
            .map_or_else(jolt_fmt_ir::nil, |return_type| {
                concat([
                    format_type_without_leading_comments(&return_type),
                    text(" "),
                ])
            }),
        text(name.text().to_owned()),
        format_parameters(method.parameters()),
        format_throws_clause(throws),
    ]);

    match method.body() {
        Some(body) if has_throws => declaration_with_body(prefix, header, format_block_body(&body)),
        Some(body) => callable_declaration_with_body(prefix, header, format_block_body(&body)),
        None => declaration_without_body(prefix, header),
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
            text(name.text().to_owned()),
            text("()"),
            element
                .dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions)
                }),
            format_annotation_element_default(element.default_value()),
        ])),
        text(";"),
    ])
}

fn format_annotation_element_default(default: Option<jolt_java_syntax::DefaultValue>) -> Doc {
    default.map_or_else(jolt_fmt_ir::nil, |default| {
        concat([
            line(),
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
    parenthesized_comma_list(
        parameters
            .parameters()
            .map(|parameter| format_formal_parameter(&parameter))
            .collect(),
    )
}

fn format_construct_leading_comments(tokens: &[jolt_java_syntax::JavaSyntaxToken]) -> Doc {
    tokens
        .first()
        .map_or_else(jolt_fmt_ir::nil, format_leading_comments)
}

fn callable_declaration_with_body(prefix: Doc, header: Doc, body: Option<Doc>) -> Doc {
    concat([prefix, group(header), text(" "), braced_body(body)])
}

fn parenthesized_comma_list(items: Vec<Doc>) -> Doc {
    if items.is_empty() {
        return text("()");
    }

    group(concat([
        text("("),
        jolt_fmt_ir::indent(concat([
            soft_line(),
            join_docs(items, &concat([text(","), line()])),
        ])),
        soft_line(),
        text(")"),
    ]))
}

fn format_throws_clause(throws: Option<jolt_java_syntax::ThrowsClause>) -> Doc {
    let Some(throws) = throws else {
        return jolt_fmt_ir::nil();
    };
    let exceptions = throws
        .exceptions()
        .map(|exception| format_type(&exception))
        .collect::<Vec<_>>();
    if exceptions.is_empty() {
        return jolt_fmt_ir::nil();
    }

    let docs = vec![
        line(),
        text("throws "),
        join_docs(exceptions, &concat([text(","), line()])),
    ];
    jolt_fmt_ir::indent(concat(docs))
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
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - body_start
            ..last.token_text_range().end().get() - body_start,
    )
}

fn format_constructor_body_element(element: &ConstructorBodyElement) -> Option<BodyItem> {
    match element {
        ConstructorBodyElement::Invocation(invocation) => Some(BodyItem::new(
            format_constructor_invocation(invocation),
            invocation.starts_after_blank_line(),
        )),
        ConstructorBodyElement::BlockStatement(statement) => {
            statement.item().and_then(format_block_item)
        }
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
            .map_or_else(jolt_fmt_ir::nil, |target| text(target.text().to_owned())),
        format_argument_list(invocation.arguments()),
        text(";"),
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
