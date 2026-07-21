use super::{
    AnnotationInterfaceBodyMember, ClassBody, ClassBodyMember, Doc, FormattedMember,
    FormatterIgnoreSplice, InterfaceBody, InterfaceBodyMember, JavaSyntaxToken, MemberCategory,
    Range, RecordBody, comments_from_tokens, for_each_formatter_ignore_splice,
    format_annotation_element_declaration, format_annotation_interface_declaration, format_block,
    format_class_declaration, format_compact_constructor_declaration,
    format_constructor_declaration, format_dangling_comments, format_enum_declaration,
    format_field_declaration, format_interface_declaration, format_method_declaration,
    format_record_declaration, format_removed_comments, format_token_with_comments,
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs, has_removed_comments,
    join_member_docs, relative_token_range_between,
};
use crate::helpers::blocks::BodyContent;
use crate::helpers::comments::format_token_removal;
use crate::helpers::recovery::{
    JavaFormatField, format_malformed, resolve_optional_field, resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    AnnotationInterfaceBodyMemberList, JavaSyntaxInvariantError, JavaSyntaxListPart, JavaSyntaxView,
};

type PartResult<'source, T> = Result<JavaSyntaxListPart<'source, T>, JavaSyntaxInvariantError>;

macro_rules! standard_member_body {
    ($name:ident, $body:ty, $category:path, $format:path) => {
        pub(super) fn $name<'source>(
            body: &$body,
            doc: &mut DocBuilder<'source>,
        ) -> BodyContent<'source> {
            let open = present_token(body.open_brace());
            let close = present_token(body.close_brace());
            let open_comments = format_body_open_dangling_comments(open, doc);
            let close_comments = format_body_close_dangling_comments(close, doc);
            let members = match resolve_required_field(body.members(), doc) {
                JavaFormatField::Present(members) => members,
                JavaFormatField::Malformed(malformed) => {
                    return recovered_member_body(open_comments, malformed, close_comments, doc);
                }
            };
            let body_start = body.text_range().start().get();
            let ignored =
                formatter_ignore_ranges(body.source_text(), body_start, body.token_iter());
            format_member_parts(
                body_start,
                &ignored,
                members.parts(),
                open_comments,
                close_comments,
                |member| family_token_range(member, body_start),
                $category,
                |member, doc| Some($format(member, doc)),
                doc,
            )
        }
    };
}

standard_member_body!(
    format_class_body,
    ClassBody<'source>,
    member_category,
    FormattedMember::from_member
);
standard_member_body!(
    format_record_body,
    RecordBody<'source>,
    member_category,
    FormattedMember::from_member
);
standard_member_body!(
    format_interface_body,
    InterfaceBody<'source>,
    interface_member_category,
    FormattedMember::from_interface_member
);

pub(super) fn format_annotation_interface_body<'source>(
    body: &jolt_java_syntax::AnnotationInterfaceBody<'source>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    let open = present_token(body.open_brace());
    let close = present_token(body.close_brace());
    let open_comments = format_body_open_dangling_comments(open, doc);
    let close_comments = format_body_close_dangling_comments(close, doc);
    let elements = match resolve_optional_field(body.elements(), doc) {
        JavaFormatField::Present(elements) => elements,
        JavaFormatField::Malformed(malformed) => {
            return recovered_member_body(open_comments, malformed, close_comments, doc);
        }
    };
    let Some(elements) = elements else {
        return combine_comment_members(doc, open_comments, close_comments)
            .map(|member| member.doc)
            .into();
    };
    let declarations: AnnotationInterfaceBodyMemberList<'source> =
        match resolve_required_field(elements.declarations(), doc) {
            JavaFormatField::Present(declarations) => declarations,
            JavaFormatField::Malformed(malformed) => {
                return recovered_member_body(open_comments, malformed, close_comments, doc);
            }
        };
    let ignored = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    format_member_parts(
        body.text_range().start().get(),
        &ignored,
        declarations.parts(),
        open_comments,
        close_comments,
        |member| family_token_range(member, body.text_range().start().get()),
        annotation_member_category,
        |member, doc| Some(FormattedMember::from_annotation_member(member, doc)),
        doc,
    )
}

fn recovered_member_body<'source>(
    open: Option<FormattedMember<'source>>,
    malformed: Doc<'source>,
    close: Option<FormattedMember<'source>>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    let contents = doc_concat!(
        doc,
        [
            open.map_or_else(Doc::nil, |member| member.doc),
            malformed,
            close.map_or_else(Doc::nil, |member| member.doc)
        ]
    );
    BodyContent::new(contents, true, true)
}

fn present_token<'source>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
        JavaSyntaxInvariantError,
    >,
) -> Option<JavaSyntaxToken<'source>> {
    match field {
        Ok(jolt_java_syntax::JavaSyntaxField::Present(token)) => Some(token),
        Ok(
            jolt_java_syntax::JavaSyntaxField::Missing(_)
            | jolt_java_syntax::JavaSyntaxField::Malformed(_),
        )
        | Err(_) => None,
    }
}

pub(super) fn format_class_member_body<'source>(
    body_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    members: impl IntoIterator<Item = PartResult<'source, ClassBodyMember<'source>>>,
    open_dangling_comments: Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    format_member_parts(
        body_start,
        ignored_ranges,
        members,
        open_dangling_comments,
        close_dangling_comments,
        |member| family_token_range(member, body_start),
        member_category,
        |member, doc| Some(FormattedMember::from_member(member, doc)),
        doc,
    )
}

#[allow(clippy::too_many_arguments)]
fn format_member_parts<'source, T: Copy>(
    body_start: usize,
    ignored_ranges: &[crate::helpers::formatter_ignore::FormatterIgnoreRange<'source>],
    members: impl IntoIterator<Item = PartResult<'source, T>>,
    open_dangling_comments: Option<FormattedMember<'source>>,
    close_dangling_comments: Option<FormattedMember<'source>>,
    item_range: impl Fn(&T) -> Option<Range<usize>>,
    item_category: impl Fn(&T) -> MemberCategory,
    mut format_item: impl FnMut(&T, &mut DocBuilder<'source>) -> Option<FormattedMember<'source>>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    let members = members.into_iter();
    if ignored_ranges.is_empty() {
        let (lower, _) = members.size_hint();
        let mut formatted = Vec::with_capacity(lower.saturating_add(2));
        formatted.extend(open_dangling_comments);
        for member in members {
            if let Some(member) = format_part(&member, &mut format_item, doc) {
                formatted.push(member);
            }
        }
        formatted.extend(close_dangling_comments);
        let present = !formatted.is_empty();
        let visible = formatted.iter().any(|member| member.visible);
        let contents = if present {
            join_member_docs(doc, formatted)
        } else {
            Doc::nil()
        };
        return BodyContent::new(contents, present, visible);
    }

    let members = members.collect::<Vec<_>>();
    let ranges = members
        .iter()
        .map(|part| part_token_range(part, body_start, &item_range))
        .collect::<Vec<_>>();
    let runs = formatter_ignore_runs(ignored_ranges, &ranges);
    let mut formatted =
        Vec::with_capacity(members.len().saturating_add(runs.len()).saturating_add(2));
    formatted.extend(open_dangling_comments);
    for_each_formatter_ignore_splice(members.len(), &runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            let category = members
                .get(run.skip_start)
                .map_or(MemberCategory::Type, |part| {
                    part_category(part, &item_category)
                });
            formatted.push(FormattedMember::ignored(
                formatter_ignore_run_doc(run, doc),
                category,
            ));
        }
        FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        } => {
            if let Some(mut member) = format_part(&members[index], &mut format_item, doc) {
                if clear_blank_line_before {
                    member = member.without_blank_line_before();
                }
                formatted.push(member);
            }
        }
    });
    formatted.extend(close_dangling_comments);
    let present = !formatted.is_empty();
    let visible = formatted.iter().any(|member| member.visible);
    let contents = if present {
        join_member_docs(doc, formatted)
    } else {
        Doc::nil()
    };
    BodyContent::new(contents, present, visible)
}

fn format_part<'source, T>(
    part: &PartResult<'source, T>,
    format_item: &mut impl FnMut(&T, &mut DocBuilder<'source>) -> Option<FormattedMember<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<FormattedMember<'source>> {
    match part {
        Ok(JavaSyntaxListPart::Item(item)) => format_item(item, doc),
        Ok(JavaSyntaxListPart::Malformed(malformed)) => {
            Some(FormattedMember::comment(format_malformed(malformed, doc)))
        }
        Ok(JavaSyntaxListPart::Missing(missing)) => Some(FormattedMember::comment(
            crate::helpers::recovery::format_missing(missing, doc),
        )),
        Ok(JavaSyntaxListPart::Separator(token)) => {
            doc.block_on_invariant("unseparated Java member list contained a separator");
            Some(FormattedMember::comment(format_token_with_comments(
                doc, token,
            )))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            None
        }
    }
}

fn part_token_range<T>(
    part: &PartResult<'_, T>,
    body_start: usize,
    item_range: &impl Fn(&T) -> Option<Range<usize>>,
) -> Option<Range<usize>> {
    match part {
        Ok(JavaSyntaxListPart::Item(item)) => item_range(item),
        Ok(JavaSyntaxListPart::Separator(token)) => {
            Some(relative_token_range_between(token, token, body_start))
        }
        Ok(JavaSyntaxListPart::Malformed(malformed)) => {
            let syntax = malformed.syntax_node()?;
            Some(relative_token_range_between(
                &syntax.first_token()?,
                &syntax.last_token()?,
                body_start,
            ))
        }
        Ok(JavaSyntaxListPart::Missing(_)) | Err(_) => None,
    }
}

fn part_category<T>(
    part: &PartResult<'_, T>,
    item_category: &impl Fn(&T) -> MemberCategory,
) -> MemberCategory {
    match part {
        Ok(JavaSyntaxListPart::Item(item)) => item_category(item),
        _ => MemberCategory::Type,
    }
}

fn family_token_range<'source>(
    member: &impl JavaSyntaxView<'source>,
    body_start: usize,
) -> Option<Range<usize>> {
    let syntax = member.syntax_node()?;
    Some(relative_token_range_between(
        &syntax.first_token()?,
        &syntax.last_token()?,
        body_start,
    ))
}

pub(super) fn format_body_open_dangling_comments<'source>(
    open: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<FormattedMember<'source>> {
    format_removed_comments(doc, open?.trailing_comments()).map(FormattedMember::comment)
}

pub(super) fn format_body_close_dangling_comments<'source>(
    close: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<FormattedMember<'source>> {
    let comments = close?.leading_comments().collect::<Vec<_>>();
    (!comments.is_empty())
        .then(|| FormattedMember::comment(format_dangling_comments(doc, comments)))
}

pub(super) fn format_empty_enum_constant_list_comments<'source>(
    constants: Option<jolt_java_syntax::EnumConstantList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<FormattedMember<'source>> {
    let constants = constants?;
    if constants.parts().next().is_some() {
        return None;
    }
    format_removed_comments(doc, comments_from_tokens(constants.token_iter()))
        .map(FormattedMember::comment)
}

pub(super) fn combine_comment_members<'source>(
    doc: &mut DocBuilder<'source>,
    first: Option<FormattedMember<'source>>,
    second: Option<FormattedMember<'source>>,
) -> Option<FormattedMember<'source>> {
    match (first, second) {
        (Some(first), Some(second)) => Some(FormattedMember::comment(doc_concat!(
            doc,
            [first.doc, doc.hard_line(), second.doc,]
        ))),
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
        | ClassBodyMember::EmptyDeclaration(_)
        | ClassBodyMember::BogusClassBodyMember(_) => MemberCategory::Type,
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
        | InterfaceBodyMember::EmptyDeclaration(_)
        | InterfaceBodyMember::BogusInterfaceBodyMember(_) => MemberCategory::Type,
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
        | AnnotationInterfaceBodyMember::EmptyDeclaration(_)
        | AnnotationInterfaceBodyMember::BogusAnnotationInterfaceBodyMember(_) => {
            MemberCategory::Type
        }
    }
}

impl<'source> FormattedMember<'source> {
    fn from_member(member: &ClassBodyMember<'source>, doc: &mut DocBuilder<'source>) -> Self {
        let starts_after_blank_line = member.starts_after_blank_line();
        match member {
            ClassBodyMember::FieldDeclaration(field) => Self::formatted(
                MemberCategory::Field,
                starts_after_blank_line,
                format_field_declaration(field, doc),
            ),
            ClassBodyMember::ConstructorDeclaration(value) => Self::formatted(
                MemberCategory::Constructor,
                starts_after_blank_line,
                format_constructor_declaration(value, doc),
            ),
            ClassBodyMember::CompactConstructorDeclaration(value) => Self::formatted(
                MemberCategory::Constructor,
                starts_after_blank_line,
                format_compact_constructor_declaration(value, doc),
            ),
            ClassBodyMember::MethodDeclaration(value) => Self::formatted(
                MemberCategory::Method,
                starts_after_blank_line,
                format_method_declaration(value, doc),
            ),
            ClassBodyMember::StaticInitializer(value) => {
                let static_token = crate::helpers::recovery::format_required_field(
                    value.static_keyword(),
                    doc,
                    |token, doc| format_token_with_comments(doc, &token),
                );
                let body = crate::helpers::recovery::format_required_field(
                    value.body(),
                    doc,
                    |body, doc| format_block(&body, doc),
                );
                Self::formatted(
                    MemberCategory::Initializer,
                    starts_after_blank_line,
                    doc_concat!(doc, [static_token, doc.space(), body]),
                )
            }
            ClassBodyMember::InstanceInitializer(value) => {
                let body = crate::helpers::recovery::format_required_field(
                    value.body(),
                    doc,
                    |body, doc| format_block(&body, doc),
                );
                Self::formatted(MemberCategory::Initializer, starts_after_blank_line, body)
            }
            ClassBodyMember::ClassDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                starts_after_blank_line,
                format_class_declaration(value, doc),
            ),
            ClassBodyMember::RecordDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                starts_after_blank_line,
                format_record_declaration(value, doc),
            ),
            ClassBodyMember::EnumDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                starts_after_blank_line,
                format_enum_declaration(value, doc),
            ),
            ClassBodyMember::InterfaceDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                starts_after_blank_line,
                format_interface_declaration(value, doc),
            ),
            ClassBodyMember::AnnotationInterfaceDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                starts_after_blank_line,
                format_annotation_interface_declaration(value, doc),
            ),
            ClassBodyMember::EmptyDeclaration(empty) => {
                format_empty_member(empty, starts_after_blank_line, doc)
            }
            ClassBodyMember::BogusClassBodyMember(value) => {
                Self::comment(format_malformed(value, doc))
            }
        }
    }

    fn from_interface_member(
        member: &InterfaceBodyMember<'source>,
        doc: &mut DocBuilder<'source>,
    ) -> Self {
        let blank = member.starts_after_blank_line();
        match member {
            InterfaceBodyMember::FieldDeclaration(value) => Self::formatted(
                MemberCategory::Field,
                blank,
                format_field_declaration(value, doc),
            ),
            InterfaceBodyMember::MethodDeclaration(value) => Self::formatted(
                MemberCategory::Method,
                blank,
                format_method_declaration(value, doc),
            ),
            InterfaceBodyMember::ClassDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_class_declaration(value, doc),
            ),
            InterfaceBodyMember::RecordDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_record_declaration(value, doc),
            ),
            InterfaceBodyMember::EnumDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_enum_declaration(value, doc),
            ),
            InterfaceBodyMember::InterfaceDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_interface_declaration(value, doc),
            ),
            InterfaceBodyMember::AnnotationInterfaceDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_annotation_interface_declaration(value, doc),
            ),
            InterfaceBodyMember::EmptyDeclaration(empty) => format_empty_member(empty, blank, doc),
            InterfaceBodyMember::BogusInterfaceBodyMember(value) => {
                Self::comment(format_malformed(value, doc))
            }
        }
    }

    fn from_annotation_member(
        member: &AnnotationInterfaceBodyMember<'source>,
        doc: &mut DocBuilder<'source>,
    ) -> Self {
        let blank = member.starts_after_blank_line();
        match member {
            AnnotationInterfaceBodyMember::FieldDeclaration(value) => Self::formatted(
                MemberCategory::Field,
                blank,
                format_field_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::MethodDeclaration(value) => Self::formatted(
                MemberCategory::Method,
                blank,
                format_method_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::AnnotationElementDeclaration(value) => Self::formatted(
                MemberCategory::Method,
                blank,
                format_annotation_element_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::ClassDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_class_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::RecordDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_record_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::EnumDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_enum_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::InterfaceDeclaration(value) => Self::formatted(
                MemberCategory::Type,
                blank,
                format_interface_declaration(value, doc),
            ),
            AnnotationInterfaceBodyMember::AnnotationInterfaceDeclaration(value) => {
                Self::formatted(
                    MemberCategory::Type,
                    blank,
                    format_annotation_interface_declaration(value, doc),
                )
            }
            AnnotationInterfaceBodyMember::EmptyDeclaration(empty) => {
                format_empty_member(empty, blank, doc)
            }
            AnnotationInterfaceBodyMember::BogusAnnotationInterfaceBodyMember(value) => {
                Self::comment(format_malformed(value, doc))
            }
        }
    }

    fn formatted(
        category: MemberCategory,
        starts_after_blank_line: bool,
        doc: Doc<'source>,
    ) -> Self {
        Self {
            category: Some(category),
            starts_after_blank_line,
            doc,
            visible: true,
        }
    }
}

fn format_empty_member<'source>(
    empty: &jolt_java_syntax::EmptyDeclaration<'source>,
    starts_after_blank_line: bool,
    doc: &mut DocBuilder<'source>,
) -> FormattedMember<'source> {
    let has_comments = has_removed_comments(comments_from_tokens(empty.token_iter()));
    let semicolon = match resolve_required_field(empty.semicolon(), doc) {
        JavaFormatField::Present(semicolon) => semicolon,
        JavaFormatField::Malformed(recovery) => return FormattedMember::comment(recovery),
    };
    let (member_doc, removed) =
        format_token_removal(doc, &semicolon, empty.separator_removal_claim());
    let visible = has_comments || !removed;
    if visible {
        FormattedMember {
            category: None,
            starts_after_blank_line,
            doc: member_doc,
            visible: true,
        }
    } else {
        FormattedMember::invisible(member_doc)
    }
}
