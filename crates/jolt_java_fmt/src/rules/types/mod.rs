//! Area D, types and declaration parts: every rule entry point of the area. Unported kinds call
//! the temporary `placeholder` (grep `placeholder(`).

mod class_types;
mod modifiers;
mod names;
mod variables;

use jolt_fmt::Doc;
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementList,
    AnnotationElementValue, AnnotationElementValuePair, ArrayDimension, ArrayDimensions, ArrayType,
    ComponentPattern, IntersectionType, MatchAllPattern, NonSealedModifier, ReceiverParameter,
    RecordPattern, TypeArgument, TypeArgumentList, TypeBoundList, TypeParameter, TypeParameterList,
    TypePattern, UnionType, WildcardType,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::placeholder::placeholder;

pub(crate) use class_types::{class_type, class_type_segment_node, primitive_type, void_type};
pub(crate) use modifiers::{annotation_list, modifier_list, parameter_modifier_list};
pub(crate) use names::{name, qualified_name, qualified_name_segment_node};
pub(crate) use variables::{
    formal_parameter, variable_declarator, variable_declarator_list, variable_initializer,
};

pub(crate) fn non_sealed_modifier<'a>(
    f: &mut JavaFormatter<'a>,
    node: NonSealedModifier<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation<'a>(f: &mut JavaFormatter<'a>, node: Annotation<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_argument_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationArgumentList<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_element_value<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementValue<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_element_value_pair<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementValuePair<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_element_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementList<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn annotation_array_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationArrayInitializer<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_parameter_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: TypeParameterList<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_parameter<'a>(f: &mut JavaFormatter<'a>, node: TypeParameter<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_bound_list<'a>(f: &mut JavaFormatter<'a>, node: TypeBoundList<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_type<'a>(f: &mut JavaFormatter<'a>, node: ArrayType<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn intersection_type<'a>(f: &mut JavaFormatter<'a>, node: IntersectionType<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn union_type<'a>(f: &mut JavaFormatter<'a>, node: UnionType<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_argument_list<'a>(f: &mut JavaFormatter<'a>, node: TypeArgumentList<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_argument<'a>(f: &mut JavaFormatter<'a>, node: TypeArgument<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn wildcard_type<'a>(f: &mut JavaFormatter<'a>, node: WildcardType<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_dimensions<'a>(f: &mut JavaFormatter<'a>, node: ArrayDimensions<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_dimension<'a>(f: &mut JavaFormatter<'a>, node: ArrayDimension<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn receiver_parameter<'a>(
    f: &mut JavaFormatter<'a>,
    node: ReceiverParameter<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn type_pattern<'a>(f: &mut JavaFormatter<'a>, node: TypePattern<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn record_pattern<'a>(f: &mut JavaFormatter<'a>, node: RecordPattern<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn component_pattern<'a>(f: &mut JavaFormatter<'a>, node: ComponentPattern<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn match_all_pattern<'a>(f: &mut JavaFormatter<'a>, node: MatchAllPattern<'a>) -> Doc {
    placeholder(f, node.syntax())
}
