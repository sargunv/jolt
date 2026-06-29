use crate::comments::{
    reject_unhandled_comments_before_end, reject_unhandled_comments_before_start,
    take_dangling_comment_docs, take_inline_leading_block_comment_docs,
    take_inline_trailing_block_comment_docs, take_leading_comment_docs, with_attached_comments,
    with_leading_and_trailing_comments,
};
use crate::context::JavaFormatContext;
use crate::diagnostics::{FormatResult, missing_layout};
use crate::layout as wrap;
use jolt_fmt_ir::{Doc, concat, hard_line, join, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationElementDeclaration, AnnotationElementValue,
    AnnotationElementValuePair, AnnotationInterfaceBody, AnnotationInterfaceBodyMember,
    AnnotationInterfaceDeclaration, ArrayDimensions, ArrayInitializer, BasicForStatement, Block,
    BlockItem, BlockStatement, BreakStatement, CatchClause, CatchParameter, ClassBody,
    ClassBodyMember, ClassDeclaration, CompilationUnit, CompilationUnitMember, ConstructorBody,
    ConstructorDeclaration, ContinueStatement, DefaultValue, DoStatement, EmptyStatement,
    EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList, EnumDeclaration, Expression,
    ExtendsClause, FieldDeclaration, FinallyClause, ForInitializer, ForStatement, ForUpdate,
    FormalParameter, FormalParameterList, FormalParameterModifier, IfStatement, ImplementsClause,
    ImportDeclaration, InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaSyntaxKind,
    JavaSyntaxToken, LabeledStatement, LocalVariableDeclaration, MethodDeclaration,
    MethodReferenceExpression, ModifierList, NameSyntax, PackageDeclaration, PermitsClause,
    ReturnStatement, Statement, StatementExpressionList, SwitchBlock, SwitchBlockItem,
    SwitchBlockStatementGroup, SwitchLabel, SwitchRule, SwitchRuleBody, SwitchStatement,
    ThrowStatement, ThrowsClause, TryStatement, Type, TypeArgumentList, TypeDeclaration,
    TypeLayoutPart, TypeParameterList, VariableDeclarator, VariableInitializerValue,
    WhileStatement, YieldStatement,
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
use declarations::{format_field_declaration, format_method_declaration, format_type_declaration};
use expressions::{format_argument_list, format_array_dimensions, format_expression};
use names::format_name;
use statements::{
    format_block, format_constructor_body, format_switch_expression,
    format_variable_declarator_list,
};
use tokens::format_token;
use types::format_type;

pub(crate) use compilation_unit::format_compilation_unit;
