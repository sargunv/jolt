use jolt_fmt_ir::{Doc, DocBuilder, LayoutDoc};
use jolt_kotlin_syntax::{
    ClassBody, ClassMember, ClassMemberDeclaration, ClassMemberList, Declaration,
    KotlinRoleElement, KotlinSyntaxListPart, KotlinSyntaxToken, KotlinSyntaxView, StatementSyntax,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_dangling_comments, format_token,
};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, format_delimiter_with_preserved_trailing,
    format_malformed, format_missing, format_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
};

use super::{
    format_declaration, format_explicit_backing_field, format_function_declaration,
    format_initializer_block, format_property_accessor, format_property_declaration,
    format_secondary_constructor, format_type_alias_declaration,
};

pub(super) fn format_class_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: ClassBody<'source>,
) -> Doc<'source> {
    let has_close = match body.close_brace() {
        jolt_kotlin_syntax::KotlinSyntaxField::Present(_) => true,
        jolt_kotlin_syntax::KotlinSyntaxField::Malformed(malformed) => {
            malformed.first_token().is_some()
        }
        jolt_kotlin_syntax::KotlinSyntaxField::Missing(_) => false,
    };
    let open = resolve_required_delimiter(body.open_brace(), doc);
    let close = resolve_required_delimiter(body.close_brace(), doc);
    let contents = format_class_body_contents(doc, &body, open.source(), close.source());
    let space = doc.space();
    let body = format_class_braced_body(doc, open, close, contents, has_close);
    doc.concat([space, body])
}

fn format_class_body_contents<'source>(
    doc: &mut DocBuilder<'source>,
    body: &ClassBody<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Option<LayoutDoc<'source>> {
    let members = body.members();
    let malformed_is_visible = matches!(
        &members,
        jolt_kotlin_syntax::KotlinSyntaxField::Malformed(malformed)
            if malformed.first_token().is_some()
    );
    let members = match resolve_required_field(members, doc) {
        KotlinFormatField::Present(members) => members,
        KotlinFormatField::Malformed(malformed) => {
            return Some(LayoutDoc::from_visibility(malformed, malformed_is_visible));
        }
    };
    let parts = collect_class_body_parts(doc, &members);
    let container =
        formatter_ignore_content_range(members.text_range(), open.copied(), close.copied());
    let ignored_runs =
        doc.formatter_ignore_runs(container, parts.iter().map(class_body_part_ignore_range));
    let mut sections = if ignored_runs.is_empty() {
        class_body_sections(doc, &parts)
    } else {
        class_body_sections_with_ignored(doc, &parts, &ignored_runs)
    };
    if let Some(close) = close
        && !close.leading_comments().is_empty()
    {
        sections.push(ClassBodySection::new(
            LayoutDoc::Visible(format_dangling_comments(doc, close.leading_comments())),
            false,
        ));
    }
    (!sections.is_empty()).then(|| join_class_body_sections(doc, sections))
}

enum ClassBodyPart<'source> {
    Member(ClassMember<'source>),
    Token(KotlinSyntaxToken<'source>),
    Recovery {
        // Keep the per-part vector packed; LayoutDoc would add an aligned enum
        // tag to every ordinary class-body part.
        doc: Doc<'source>,
        first: Option<KotlinSyntaxToken<'source>>,
        last: Option<KotlinSyntaxToken<'source>>,
        visible: bool,
    },
}

impl<'source> ClassBodyPart<'source> {
    fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Member(member) => member.first_token(),
            Self::Token(token) => Some(*token),
            Self::Recovery { first, .. } => *first,
        }
    }

    fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Member(member) => member.last_token(),
            Self::Token(token) => Some(*token),
            Self::Recovery { last, .. } => *last,
        }
    }
}

fn collect_class_body_parts<'source>(
    doc: &mut DocBuilder<'source>,
    members: &ClassMemberList<'source>,
) -> Vec<ClassBodyPart<'source>> {
    members
        .parts()
        .map(|part| match part {
            KotlinSyntaxListPart::Item(element) => class_body_element(doc, element),
            KotlinSyntaxListPart::Separator(token) => ClassBodyPart::Token(token),
            KotlinSyntaxListPart::Missing(missing) => ClassBodyPart::Recovery {
                doc: format_missing(&missing, doc),
                first: None,
                last: None,
                visible: false,
            },
            KotlinSyntaxListPart::Malformed(malformed) => {
                let first = malformed.first_token();
                let last = malformed
                    .syntax_node()
                    .and_then(|syntax| syntax.last_token());
                ClassBodyPart::Recovery {
                    doc: format_malformed(&malformed, doc),
                    first,
                    last,
                    visible: first.is_some(),
                }
            }
        })
        .collect()
}

fn class_body_element<'source>(
    doc: &mut DocBuilder<'source>,
    element: KotlinRoleElement<'source>,
) -> ClassBodyPart<'source> {
    if let Some(member) = element.cast_family::<ClassMember<'source>>() {
        ClassBodyPart::Member(member)
    } else if let Some(token) = element.token() {
        ClassBodyPart::Token(token)
    } else {
        doc.block_on_invariant("Kotlin class member list contained an unsupported element");
        ClassBodyPart::Recovery {
            doc: Doc::nil(),
            first: None,
            last: None,
            visible: false,
        }
    }
}

fn class_body_sections<'source>(
    doc: &mut DocBuilder<'source>,
    parts: &[ClassBodyPart<'source>],
) -> Vec<ClassBodySection<'source>> {
    let mut sections = Vec::with_capacity(parts.len());
    let mut previous_had_comments = false;
    for part in parts {
        push_class_body_part(doc, &mut sections, part, &mut previous_had_comments);
    }
    sections
}

fn push_class_body_part<'source>(
    doc: &mut DocBuilder<'source>,
    sections: &mut Vec<ClassBodySection<'source>>,
    part: &ClassBodyPart<'source>,
    previous_had_comments: &mut bool,
) {
    let physical = match part {
        ClassBodyPart::Member(member) => {
            *previous_had_comments = member
                .last_token()
                .is_some_and(|token| !token.trailing_comments().is_empty());
            sections.push(ClassBodySection::new(
                LayoutDoc::Visible(format_class_member(doc, member)),
                enum_entry_continues(member),
            ));
            return;
        }
        ClassBodyPart::Token(token) => format_token(
            doc,
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        ),
        ClassBodyPart::Recovery {
            doc: claim,
            visible: false,
            ..
        } => {
            *previous_had_comments = false;
            sections.push(ClassBodySection::new(LayoutDoc::ClaimOnly(*claim), false));
            return;
        }
        ClassBodyPart::Recovery {
            doc: recovery,
            last,
            visible: true,
            ..
        } => {
            push_class_body_physical_doc(doc, sections, *recovery, *previous_had_comments);
            *previous_had_comments = last
                .as_ref()
                .is_some_and(|token| !token.trailing_comments().is_empty());
            return;
        }
    };
    push_class_body_physical_doc(doc, sections, physical, *previous_had_comments);
}

fn enum_entry_continues(member: &ClassMember<'_>) -> bool {
    let ClassMember::EnumEntry(entry) = member else {
        return false;
    };
    matches!(
        entry.comma(),
        jolt_kotlin_syntax::KotlinSyntaxField::Present(_)
    )
}

fn class_body_sections_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    parts: &[ClassBodyPart<'source>],
    ignored_runs: &[FormatterIgnoreRun<'source>],
) -> Vec<ClassBodySection<'source>> {
    let mut sections = Vec::with_capacity(parts.len().saturating_add(ignored_runs.len()));
    let mut previous_had_comments = false;
    for_each_formatter_ignore_splice(parts.len(), ignored_runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            sections.push(ClassBodySection::new(
                LayoutDoc::Visible(formatter_ignore_run_doc(run, doc)),
                !run.ends_with_on_marker(),
            ));
        }
        FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        } => {
            // Skipped parts still advance the trailing-comment state that the
            // next physical part reads. When this item immediately follows an
            // ignore run, recover that state from the last skipped part.
            if clear_blank_line_before {
                previous_had_comments = parts[index - 1]
                    .last_token()
                    .is_some_and(|token| !token.trailing_comments().is_empty());
            }
            push_class_body_part(
                doc,
                &mut sections,
                &parts[index],
                &mut previous_had_comments,
            );
        }
    });
    sections
}

fn push_class_body_physical_doc<'source>(
    doc: &mut DocBuilder<'source>,
    sections: &mut Vec<ClassBodySection<'source>>,
    physical: Doc<'source>,
    previous_had_comments: bool,
) {
    if previous_had_comments {
        let line = doc.hard_line();
        sections.push(ClassBodySection::new(
            LayoutDoc::Visible(doc.concat([line, physical])),
            false,
        ));
    } else if sections
        .last()
        .is_some_and(|previous| previous.hard_line_after)
    {
        sections.push(ClassBodySection::new(LayoutDoc::Visible(physical), false));
    } else if let Some(previous) = sections.last_mut() {
        previous.doc = doc.concat([previous.doc, physical]);
        previous.visible = true;
    } else {
        sections.push(ClassBodySection::new(LayoutDoc::Visible(physical), false));
    }
}

fn format_class_member<'source>(
    doc: &mut DocBuilder<'source>,
    member: &ClassMember<'source>,
) -> Doc<'source> {
    match member {
        ClassMember::ClassMemberDeclaration(member) => format_class_member_declaration(doc, member),
        ClassMember::ClassDeclaration(value) => {
            format_declaration(doc, &Declaration::ClassDeclaration(*value))
        }
        ClassMember::InterfaceDeclaration(value) => {
            format_declaration(doc, &Declaration::InterfaceDeclaration(*value))
        }
        ClassMember::ObjectDeclaration(value) => {
            format_declaration(doc, &Declaration::ObjectDeclaration(*value))
        }
        ClassMember::CompanionObject(value) => {
            format_declaration(doc, &Declaration::CompanionObject(*value))
        }
        ClassMember::EnumEntry(value) => format_declaration(doc, &Declaration::EnumEntry(*value)),
        ClassMember::FunctionDeclaration(value) => format_function_declaration(doc, value),
        ClassMember::PropertyDeclaration(value) => format_property_declaration(doc, value),
        ClassMember::TypeAliasDeclaration(value) => format_type_alias_declaration(doc, value),
        ClassMember::SecondaryConstructor(value) => format_secondary_constructor(doc, value),
        ClassMember::InitializerBlock(value) => format_initializer_block(doc, value),
        ClassMember::PropertyAccessor(value) => format_property_accessor(doc, value),
        ClassMember::ExplicitBackingField(value) => format_explicit_backing_field(doc, value),
        ClassMember::Statement(value) => format_class_statement(doc, value),
        ClassMember::BogusClassMember(value) => format_malformed(value, doc),
    }
}

fn format_class_member_declaration<'source>(
    doc: &mut DocBuilder<'source>,
    member: &ClassMemberDeclaration<'source>,
) -> Doc<'source> {
    let contents = match resolve_required_field(member.member(), doc) {
        KotlinFormatField::Present(element) => format_class_member_element(doc, element),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let comma = format_optional_field(member.comma(), doc, |comma, doc| {
        format_token(
            doc,
            &comma,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([contents, comma])
}

fn format_class_member_element<'source>(
    doc: &mut DocBuilder<'source>,
    element: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(declaration) = element.cast_family::<Declaration<'source>>() {
        return format_declaration(doc, &declaration);
    }
    if let Some(statement) = element.cast_node::<jolt_kotlin_syntax::Statement<'source>>() {
        return format_class_statement(doc, &statement);
    }
    doc.block_on_invariant("Kotlin class member wrapper contained an unsupported element");
    Doc::nil()
}

fn format_class_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &jolt_kotlin_syntax::Statement<'source>,
) -> Doc<'source> {
    crate::rules::statements::format_statement_syntax_with_leading(
        doc,
        &StatementSyntax::Statement(*statement),
    )
}

fn class_body_part_ignore_range(part: &ClassBodyPart<'_>) -> Option<FormatterIgnoreItemRange> {
    Some(FormatterIgnoreItemRange::between(
        &part.first_token()?,
        &part.last_token()?,
    ))
}

fn format_class_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    body: Option<LayoutDoc<'source>>,
    has_close: bool,
) -> Doc<'source> {
    let open = format_delimiter_with_preserved_trailing(doc, open, LeadingTrivia::Preserve);
    let contents = match body {
        Some(LayoutDoc::Visible(body)) => {
            let line = doc.hard_line();
            let body = doc.concat([line, body]);
            let body = doc.indent(body);
            if has_close {
                let line = doc.hard_line();
                doc.concat([body, line])
            } else {
                body
            }
        }
        Some(LayoutDoc::ClaimOnly(claim)) => {
            let line = doc.hard_line();
            doc.concat([claim, line])
        }
        None => doc.hard_line(),
    };
    let close =
        format_delimiter_with_preserved_trailing(doc, close, LeadingTrivia::SuppressAlreadyHandled);
    doc.concat([open, contents, close])
}

fn join_class_body_sections<'source>(
    doc: &mut DocBuilder<'source>,
    sections: Vec<ClassBodySection<'source>>,
) -> LayoutDoc<'source> {
    let mut previous_hard_line_after = false;
    let mut has_visible = false;
    let joined = doc.concat_list(|joined| {
        for section in sections {
            if section.visible && has_visible {
                let separator = if previous_hard_line_after {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section.doc);
            if section.visible {
                has_visible = true;
                previous_hard_line_after = section.hard_line_after;
            } else {
                previous_hard_line_after = false;
            }
        }
    });
    LayoutDoc::from_visibility(joined, has_visible)
}

struct ClassBodySection<'source> {
    // Project LayoutDoc into the existing flag storage instead of retaining its
    // enum tag in this per-member vector.
    doc: Doc<'source>,
    hard_line_after: bool,
    visible: bool,
}

impl<'source> ClassBodySection<'source> {
    fn new(layout: LayoutDoc<'source>, hard_line_after: bool) -> Self {
        Self {
            doc: layout.doc(),
            hard_line_after,
            visible: layout.is_visible(),
        }
    }
}
