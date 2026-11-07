//! Area A, declarations: every rule entry point of the area. Unported kinds call
//! the temporary `placeholder` (grep `placeholder(`).

mod classes;
mod compilation_unit;
mod members;

use jolt_fmt::Doc;
use jolt_java_syntax::{
    AnnotationElementDeclaration, AnnotationInterfaceBody, AnnotationInterfaceDeclaration,
    CompactConstructorDeclaration, ConstructorBody, ConstructorDeclaration, ConstructorInvocation,
    DefaultValue, EmptyDeclaration, EnumBody, EnumConstant, EnumDeclaration, ExportsDirective,
    ExtendsClause, ImplementsClause, InstanceInitializer, InterfaceBody, InterfaceDeclaration,
    ModuleDeclaration, ModuleImplementationClause, ModuleTargetClause, OpensDirective,
    PermitsClause, ProvidesDirective, RecordBody, RecordComponent, RecordDeclaration,
    RequiresDirective, StaticInitializer, UsesDirective,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::placeholder::placeholder;

pub(crate) use classes::{class_body, class_declaration};
pub(crate) use compilation_unit::{compilation_unit, import_declaration, package_declaration};
pub(crate) use members::{field_declaration, method_declaration, throws_clause};

pub(crate) fn module_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn requires_directive<'a>(
    f: &mut JavaFormatter<'a>,
    node: RequiresDirective<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn exports_directive<'a>(f: &mut JavaFormatter<'a>, node: ExportsDirective<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn opens_directive<'a>(f: &mut JavaFormatter<'a>, node: OpensDirective<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn uses_directive<'a>(f: &mut JavaFormatter<'a>, node: UsesDirective<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn provides_directive<'a>(
    f: &mut JavaFormatter<'a>,
    node: ProvidesDirective<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn module_target_clause<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleTargetClause<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn module_implementation_clause<'a>(
    f: &mut JavaFormatter<'a>,
    node: ModuleImplementationClause<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn record_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: RecordDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn enum_declaration<'a>(f: &mut JavaFormatter<'a>, node: EnumDeclaration<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: InterfaceDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn extends_clause<'a>(f: &mut JavaFormatter<'a>, node: ExtendsClause<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn implements_clause<'a>(f: &mut JavaFormatter<'a>, node: ImplementsClause<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn permits_clause<'a>(f: &mut JavaFormatter<'a>, node: PermitsClause<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn interface_body<'a>(f: &mut JavaFormatter<'a>, node: InterfaceBody<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn enum_body<'a>(f: &mut JavaFormatter<'a>, node: EnumBody<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn record_body<'a>(f: &mut JavaFormatter<'a>, node: RecordBody<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_interface_body<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceBody<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn empty_declaration<'a>(f: &mut JavaFormatter<'a>, node: EmptyDeclaration<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn enum_constant<'a>(f: &mut JavaFormatter<'a>, node: EnumConstant<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn record_component<'a>(f: &mut JavaFormatter<'a>, node: RecordComponent<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn constructor_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConstructorDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn constructor_body<'a>(f: &mut JavaFormatter<'a>, node: ConstructorBody<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn constructor_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConstructorInvocation<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn compact_constructor_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: CompactConstructorDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn static_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: StaticInitializer<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn instance_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: InstanceInitializer<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_element_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn default_value<'a>(f: &mut JavaFormatter<'a>, node: DefaultValue<'a>) -> Doc {
    placeholder(f, node.syntax())
}
