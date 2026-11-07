//! Area A, declarations: the compilation unit, packages, imports, modules,
//! type declarations, bodies and members.

mod bodies;
mod compilation_unit;
mod members;
mod modules;
mod types;

pub(crate) use bodies::{
    annotation_interface_body, class_body, empty_declaration, enum_body, enum_constant,
    interface_body, record_body,
};
pub(crate) use compilation_unit::{compilation_unit, import_declaration, package_declaration};
pub(crate) use members::{
    annotation_element_declaration, compact_constructor_declaration, constructor_body,
    constructor_declaration, constructor_invocation, default_value, field_declaration,
    instance_initializer, method_declaration, record_component, static_initializer,
};
pub(crate) use modules::{
    exports_directive, module_declaration, module_implementation_clause, module_target_clause,
    opens_directive, provides_directive, requires_directive, uses_directive,
};
pub(crate) use types::{
    annotation_interface_declaration, class_declaration, enum_declaration, extends_clause,
    implements_clause, interface_declaration, permits_clause, record_declaration, throws_clause,
};
