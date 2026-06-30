//! Java syntax rule layer.
//!
//! Formatter rules own the syntax-to-document contract for parser-clean Java:
//! identify the CST wrapper's source range and grammar slots, collect comments
//! associated with the node or slot, format children through rule functions or
//! Java helper modules, emit leading and trailing comments through shared
//! wrappers, explicitly place or reject dangling and inline comments, and
//! return a real document for accepted syntax.

use crate::comments::{
    reject_unhandled_comments_before_end, reject_unhandled_comments_before_start,
    take_dangling_comment_docs, take_inline_leading_block_comment_docs,
    take_inline_trailing_block_comment_docs, take_leading_comment_docs, with_attached_comments,
    with_leading_and_trailing_comments,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::FormatResult;
use crate::helpers::lists as java_lists;
use crate::layout as wrap;
use jolt_fmt_ir::{Doc, concat, hard_line, join, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementDeclaration,
    AnnotationElementListItem, AnnotationElementValue, AnnotationElementValuePair,
    AnnotationInterfaceBody, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    ArrayDimensions, ArrayInitializer, BasicForStatement, Block, BlockItem, BlockStatement,
    BreakStatement, CatchClause, CatchParameter, ClassBody, ClassBodyMember, ClassDeclaration,
    CompactConstructorDeclaration, CompilationUnit, CompilationUnitMember, ConstructorBody,
    ConstructorDeclaration, ConstructorInvocation, ContinueStatement, DefaultValue, DoStatement,
    EmptyStatement, EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList,
    EnumDeclaration, Expression, ExtendsClause, FieldDeclaration, FinallyClause, ForInitializer,
    ForStatement, ForUpdate, FormalParameter, FormalParameterList, FormalParameterModifier,
    IfStatement, ImplementsClause, ImportDeclaration, InterfaceBody, InterfaceBodyMember,
    InterfaceDeclaration, JavaSyntaxKind, JavaSyntaxToken, LabeledStatement,
    LocalVariableDeclaration, MethodDeclaration, MethodReferenceExpression, ModifierList,
    ModuleDeclaration, ModuleDirective, NameSyntax, PackageDeclaration, Pattern, PermitsClause,
    ReceiverParameter, RecordComponent, RecordComponentList, RecordDeclaration, Resource,
    ResourceSpecification, ReturnStatement, Statement, StatementExpressionList, SwitchBlock,
    SwitchBlockItem, SwitchBlockStatementGroup, SwitchLabel, SwitchLabelItem, SwitchRule,
    SwitchRuleBody, SwitchStatement, ThrowStatement, ThrowsClause, TryStatement,
    TryWithResourcesStatement, Type, TypeArgumentList, TypeBoundList, TypeDeclaration,
    TypeLayoutPart, TypeParameterList, VariableAccess, VariableDeclarator,
    VariableInitializerValue, WhileStatement, YieldStatement,
};

mod annotations;
mod compilation_unit;
mod declarations;
mod expressions;
mod names;
mod statements;
#[cfg(test)]
mod tests;
mod tokens;
mod types;

use annotations::{
    format_annotation, format_annotation_element_value, format_annotation_list,
    format_modifier_list, with_vertical_annotations,
};
use declarations::{
    braced_type_body, format_class_body, format_field_declaration, format_method_declaration,
    format_type_declaration,
};
use expressions::{
    format_argument_list, format_array_dimensions, format_expression, format_pattern,
    format_variable_initializer_value,
};
use names::format_name;
use statements::{
    format_block, format_constructor_body, format_local_variable_declaration_header,
    format_switch_expression, format_variable_declarator_list,
};
use tokens::{format_multiline_token, format_token};
use types::{format_type, format_type_argument_list, format_type_layout_parts};

pub(crate) use compilation_unit::format_compilation_unit;
