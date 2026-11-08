//! Area D, types and declaration parts: modifiers and annotations, types,
//! names, parameters, variable declarators, and patterns.

mod annotations;
mod basic;
mod generics;
mod modifiers;
mod names;
mod patterns;
mod variables;

pub(crate) use annotations::{
    annotation, annotation_argument_list, annotation_array_initializer, annotation_element_value,
    annotation_element_value_pair,
};
pub(crate) use basic::{
    array_dimension, array_dimensions, array_type, class_type, class_type_segment_node,
    primitive_type, void_type,
};
pub(crate) use generics::{
    intersection_type, type_argument, type_argument_list, type_bound_list, type_parameter,
    type_parameter_list, union_type, wildcard_type,
};
pub(crate) use modifiers::{
    annotation_lines, annotation_list, inline_modifiers, modifier_list, non_sealed_modifier,
    parameter_modifier_list,
};
pub(crate) use names::{name, name_key, qualified_name, qualified_name_segment_node};
pub(crate) use patterns::{component_pattern, match_all_pattern, record_pattern, type_pattern};
pub(crate) use variables::{
    formal_parameter, receiver_parameter, record_component, variable_declarator,
    variable_declarator_list, variable_initializer,
};
