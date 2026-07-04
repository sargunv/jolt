use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, group, hard_line, line, text};
use jolt_java_syntax::{
    AnnotationElementDeclaration, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    BlockStatement, ClassBody, ClassBodyMember, ClassDeclaration, ConstructorInvocation,
    EnumConstant, EnumDeclaration, ExtendsClause, FormalParameterList, ImplementsClause,
    InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaSyntaxToken, MethodDeclaration,
    ModifierList, PermitsClause, PermitsClauseEntry, RecordBody, RecordDeclaration, ThrowsClause,
    ThrowsClauseEntry, TypeClauseEntry, TypeDeclaration,
};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{BodyItem, braced_body, braced_body_tail, join_body_items};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, comment_is_star_block,
    comments_from_tokens, format_comment, format_construct_leading_comments,
    format_dangling_comments, format_leading_comment_list, format_removed_comments,
    format_separator_with_comments, format_token, format_token_after_construct_leading_comments,
    format_token_with_comments, has_removed_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_control_marker, relative_token_range_between,
};
use crate::helpers::lists::{CommaListItem, comma_list, parenthesized_list};
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
    format_block, format_block_statement_item, format_statement_semicolon,
};
use crate::rules::types::{
    format_array_dimensions, format_inline_annotations, format_type, format_type_argument_list,
    format_type_parameter_list, format_type_without_leading_comments,
};
use crate::rules::variables::{
    format_field_declaration, format_formal_parameter, format_receiver_parameter,
    format_record_component,
};

mod callables;
mod constructor_bodies;
mod enums;
mod member_bodies;
mod type_declarations;

use callables::{
    format_annotation_element_declaration, format_compact_constructor_declaration,
    format_constructor_declaration, format_method_declaration,
};
use constructor_bodies::format_constructor_body;
use enums::format_enum_body_contents;
use member_bodies::{
    format_annotation_interface_body, format_class_body, format_interface_body, format_record_body,
};
use type_declarations::{
    format_annotation_interface_declaration, format_class_declaration, format_enum_declaration,
    format_interface_declaration, format_record_declaration,
};

pub(crate) fn format_type_declaration<'source>(
    declaration: &TypeDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match declaration {
        TypeDeclaration::ClassDeclaration(class) => format_class_declaration(class, formatter),
        TypeDeclaration::InterfaceDeclaration(interface) => {
            format_interface_declaration(interface, formatter)
        }
        TypeDeclaration::RecordDeclaration(record) => format_record_declaration(record, formatter),
        TypeDeclaration::EnumDeclaration(enum_) => format_enum_declaration(enum_, formatter),
        TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => {
            format_annotation_interface_declaration(annotation, formatter)
        }
    }
}

pub(crate) fn format_anonymous_class_body<'source>(
    body: &ClassBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    braced_body(format_class_body(body, formatter))
}
