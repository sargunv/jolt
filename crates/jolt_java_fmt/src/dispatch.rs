//! The total dispatch from node kind to rule function.

use jolt_fmt::Doc;
use jolt_java_syntax::JavaSyntaxKind as K;
use jolt_syntax::{AstNode, SyntaxNode};

use crate::JavaFormatter;
use crate::helpers::lists::unexpected_list;
use crate::rules::{declarations, expressions, statements, types};

fn cast<'a, N: AstNode<'a>>(node: SyntaxNode<'a>) -> N {
    N::cast(node).expect("dispatch matched the node kind")
}

type Rule = for<'a, 'f> fn(&'f mut JavaFormatter<'a>, SyntaxNode<'a>) -> Doc;

pub(crate) fn format_node<'a>(f: &mut JavaFormatter<'a>, node: SyntaxNode<'a>) -> Doc {
    rule(K::from_raw(node.kind()))(f, node)
}

/// The rule for a node kind. Each arm is its own small function, so deep
/// nesting does not pay for one large frame per level (debug builds).
#[allow(clippy::too_many_lines)]
fn rule(kind: K) -> Rule {
    match kind {
        K::CompilationUnit => |f, node| declarations::compilation_unit(f, cast(node)),
        K::PackageDeclaration => |f, node| declarations::package_declaration(f, cast(node)),
        K::ImportDeclaration => |f, node| declarations::import_declaration(f, cast(node)),
        K::ModuleDeclaration => |f, node| declarations::module_declaration(f, cast(node)),
        K::RequiresDirective => |f, node| declarations::requires_directive(f, cast(node)),
        K::ExportsDirective => |f, node| declarations::exports_directive(f, cast(node)),
        K::OpensDirective => |f, node| declarations::opens_directive(f, cast(node)),
        K::UsesDirective => |f, node| declarations::uses_directive(f, cast(node)),
        K::ProvidesDirective => |f, node| declarations::provides_directive(f, cast(node)),
        K::ModuleTargetClause => |f, node| declarations::module_target_clause(f, cast(node)),
        K::ModuleImplementationClause => {
            |f, node| declarations::module_implementation_clause(f, cast(node))
        }
        K::ModifierList => |f, node| types::modifier_list(f, cast(node)),
        K::Annotation => |f, node| types::annotation(f, cast(node)),
        K::AnnotationArgumentList => |f, node| types::annotation_argument_list(f, cast(node)),
        K::AnnotationElementDeclaration => {
            |f, node| declarations::annotation_element_declaration(f, cast(node))
        }
        K::AnnotationElementValue => |f, node| types::annotation_element_value(f, cast(node)),
        K::AnnotationElementValuePair => {
            |f, node| types::annotation_element_value_pair(f, cast(node))
        }
        K::AnnotationElementList => |f, node| types::annotation_element_list(f, cast(node)),
        K::AnnotationArrayInitializer => {
            |f, node| types::annotation_array_initializer(f, cast(node))
        }
        K::DefaultValue => |f, node| declarations::default_value(f, cast(node)),
        K::ClassDeclaration => |f, node| declarations::class_declaration(f, cast(node)),
        K::RecordDeclaration => |f, node| declarations::record_declaration(f, cast(node)),
        K::EnumDeclaration => |f, node| declarations::enum_declaration(f, cast(node)),
        K::InterfaceDeclaration => |f, node| declarations::interface_declaration(f, cast(node)),
        K::AnnotationInterfaceDeclaration => {
            |f, node| declarations::annotation_interface_declaration(f, cast(node))
        }
        K::TypeParameterList => |f, node| types::type_parameter_list(f, cast(node)),
        K::TypeParameter => |f, node| types::type_parameter(f, cast(node)),
        K::TypeBoundList => |f, node| types::type_bound_list(f, cast(node)),
        K::ExtendsClause => |f, node| declarations::extends_clause(f, cast(node)),
        K::ImplementsClause => |f, node| declarations::implements_clause(f, cast(node)),
        K::PermitsClause => |f, node| declarations::permits_clause(f, cast(node)),
        K::ClassBody => |f, node| declarations::class_body(f, cast(node)),
        K::EmptyDeclaration => |f, node| declarations::empty_declaration(f, cast(node)),
        K::RecordBody => |f, node| declarations::record_body(f, cast(node)),
        K::InterfaceBody => |f, node| declarations::interface_body(f, cast(node)),
        K::AnnotationInterfaceBody => {
            |f, node| declarations::annotation_interface_body(f, cast(node))
        }
        K::EnumBody => |f, node| declarations::enum_body(f, cast(node)),
        K::EnumConstant => |f, node| declarations::enum_constant(f, cast(node)),
        K::RecordComponent => |f, node| declarations::record_component(f, cast(node)),
        K::FieldDeclaration => |f, node| declarations::field_declaration(f, cast(node)),
        K::MethodDeclaration => |f, node| declarations::method_declaration(f, cast(node)),
        K::ConstructorDeclaration => |f, node| declarations::constructor_declaration(f, cast(node)),
        K::ConstructorBody => |f, node| declarations::constructor_body(f, cast(node)),
        K::ConstructorInvocation => |f, node| declarations::constructor_invocation(f, cast(node)),
        K::CompactConstructorDeclaration => {
            |f, node| declarations::compact_constructor_declaration(f, cast(node))
        }
        K::StaticInitializer => |f, node| declarations::static_initializer(f, cast(node)),
        K::InstanceInitializer => |f, node| declarations::instance_initializer(f, cast(node)),
        K::FormalParameter => |f, node| types::formal_parameter(f, cast(node)),
        K::ReceiverParameter => |f, node| types::receiver_parameter(f, cast(node)),
        K::ThrowsClause => |f, node| declarations::throws_clause(f, cast(node)),
        K::VariableDeclaratorList => |f, node| types::variable_declarator_list(f, cast(node)),
        K::VariableDeclarator => |f, node| types::variable_declarator(f, cast(node)),
        K::VariableInitializer => |f, node| types::variable_initializer(f, cast(node)),
        K::Block => |f, node| statements::block(f, cast(node)),
        K::BlockStatement => |f, node| statements::block_statement(f, cast(node)),
        K::LocalVariableDeclaration => {
            |f, node| statements::local_variable_declaration(f, cast(node))
        }
        K::LocalClassOrInterfaceDeclaration => {
            |f, node| statements::local_class_or_interface_declaration(f, cast(node))
        }
        K::EmptyStatement => |f, node| statements::empty_statement(f, cast(node)),
        K::LabeledStatement => |f, node| statements::labeled_statement(f, cast(node)),
        K::ExpressionStatement => |f, node| statements::expression_statement(f, cast(node)),
        K::IfStatement => |f, node| statements::if_statement(f, cast(node)),
        K::AssertStatement => |f, node| statements::assert_statement(f, cast(node)),
        K::SwitchStatement => |f, node| statements::switch_statement(f, cast(node)),
        K::SwitchBlock => |f, node| statements::switch_block(f, cast(node)),
        K::SwitchBlockStatementGroup => {
            |f, node| statements::switch_block_statement_group(f, cast(node))
        }
        K::SwitchRule => |f, node| statements::switch_rule(f, cast(node)),
        K::SwitchLabel => |f, node| statements::switch_label(f, cast(node)),
        K::CaseConstant => |f, node| statements::case_constant(f, cast(node)),
        K::CasePattern => |f, node| statements::case_pattern(f, cast(node)),
        K::Guard => |f, node| statements::guard(f, cast(node)),
        K::WhileStatement => |f, node| statements::while_statement(f, cast(node)),
        K::DoStatement => |f, node| statements::do_statement(f, cast(node)),
        K::ForStatement => |f, node| statements::for_statement(f, cast(node)),
        K::BasicForStatement => |f, node| statements::basic_for_statement(f, cast(node)),
        K::EnhancedForStatement => |f, node| statements::enhanced_for_statement(f, cast(node)),
        K::EnhancedForVariable => |f, node| statements::enhanced_for_variable(f, cast(node)),
        K::ForInitializer => |f, node| statements::for_initializer(f, cast(node)),
        K::ForUpdate => |f, node| statements::for_update(f, cast(node)),
        K::BreakStatement => |f, node| statements::break_statement(f, cast(node)),
        K::YieldStatement => |f, node| statements::yield_statement(f, cast(node)),
        K::ContinueStatement => |f, node| statements::continue_statement(f, cast(node)),
        K::ReturnStatement => |f, node| statements::return_statement(f, cast(node)),
        K::ThrowStatement => |f, node| statements::throw_statement(f, cast(node)),
        K::SynchronizedStatement => |f, node| statements::synchronized_statement(f, cast(node)),
        K::TryStatement => |f, node| statements::try_statement(f, cast(node)),
        K::TryWithResourcesStatement => {
            |f, node| statements::try_with_resources_statement(f, cast(node))
        }
        K::CatchClause => |f, node| statements::catch_clause(f, cast(node)),
        K::CatchParameter => |f, node| statements::catch_parameter(f, cast(node)),
        K::CatchTypeList => |f, node| statements::catch_type_list(f, cast(node)),
        K::FinallyClause => |f, node| statements::finally_clause(f, cast(node)),
        K::ResourceSpecification => |f, node| statements::resource_specification(f, cast(node)),
        K::Resource => |f, node| statements::resource(f, cast(node)),
        K::ResourceVariableDeclaration => {
            |f, node| statements::resource_variable_declaration(f, cast(node))
        }
        K::VariableAccess => |f, node| statements::variable_access(f, cast(node)),
        K::PrimitiveType => |f, node| types::primitive_type(f, cast(node)),
        K::VoidType => |f, node| types::void_type(f, cast(node)),
        K::ClassType => |f, node| types::class_type(f, cast(node)),
        K::ArrayType => |f, node| types::array_type(f, cast(node)),
        K::IntersectionType => |f, node| types::intersection_type(f, cast(node)),
        K::UnionType => |f, node| types::union_type(f, cast(node)),
        K::TypeArgumentList => |f, node| types::type_argument_list(f, cast(node)),
        K::TypeArgument => |f, node| types::type_argument(f, cast(node)),
        K::WildcardType => |f, node| types::wildcard_type(f, cast(node)),
        K::ArrayDimensions => |f, node| types::array_dimensions(f, cast(node)),
        K::ArrayDimension => |f, node| types::array_dimension(f, cast(node)),
        K::Name => |f, node| types::name(f, cast(node)),
        K::QualifiedName => |f, node| types::qualified_name(f, cast(node)),
        K::LiteralExpression => |f, node| expressions::literal_expression(f, cast(node)),
        K::NameExpression => |f, node| expressions::name_expression(f, cast(node)),
        K::ThisExpression => |f, node| expressions::this_expression(f, cast(node)),
        K::SuperExpression => |f, node| expressions::super_expression(f, cast(node)),
        K::ParenthesizedExpression => {
            |f, node| expressions::parenthesized_expression(f, cast(node))
        }
        K::ClassLiteralExpression => |f, node| expressions::class_literal_expression(f, cast(node)),
        K::FieldAccessExpression => |f, node| expressions::field_access_expression(f, cast(node)),
        K::ArrayAccessExpression => |f, node| expressions::array_access_expression(f, cast(node)),
        K::MethodInvocationExpression => {
            |f, node| expressions::method_invocation_expression(f, cast(node))
        }
        K::MethodReferenceExpression => {
            |f, node| expressions::method_reference_expression(f, cast(node))
        }
        K::ObjectCreationExpression => {
            |f, node| expressions::object_creation_expression(f, cast(node))
        }
        K::ArrayCreationExpression => {
            |f, node| expressions::array_creation_expression(f, cast(node))
        }
        K::DimExpression => |f, node| expressions::dim_expression(f, cast(node)),
        K::ArrayInitializer => |f, node| expressions::array_initializer(f, cast(node)),
        K::AssignmentExpression => |f, node| expressions::assignment_expression(f, cast(node)),
        K::ConditionalExpression => |f, node| expressions::conditional_expression(f, cast(node)),
        K::InstanceofExpression => |f, node| expressions::instanceof_expression(f, cast(node)),
        K::BinaryExpression => |f, node| expressions::binary_expression(f, cast(node)),
        K::UnaryExpression => |f, node| expressions::unary_expression(f, cast(node)),
        K::PostfixExpression => |f, node| expressions::postfix_expression(f, cast(node)),
        K::CastExpression => |f, node| expressions::cast_expression(f, cast(node)),
        K::LambdaExpression => |f, node| expressions::lambda_expression(f, cast(node)),
        K::LambdaParameter => |f, node| expressions::lambda_parameter(f, cast(node)),
        K::SwitchExpression => |f, node| statements::switch_expression(f, cast(node)),
        K::ArgumentList => |f, node| expressions::argument_list(f, cast(node)),
        K::TypePattern => |f, node| types::type_pattern(f, cast(node)),
        K::RecordPattern => |f, node| types::record_pattern(f, cast(node)),
        K::ComponentPattern => |f, node| types::component_pattern(f, cast(node)),
        K::MatchAllPattern => |f, node| types::match_all_pattern(f, cast(node)),
        K::NonSealedModifier => |f, node| types::non_sealed_modifier(f, cast(node)),
        K::ClassTypeSegmentNode => |f, node| types::class_type_segment_node(f, cast(node)),
        K::QualifiedNameSegmentNode => |f, node| types::qualified_name_segment_node(f, cast(node)),
        K::QualifiedMethodInvocation => {
            |f, node| expressions::qualified_method_invocation(f, cast(node))
        }
        K::UnqualifiedMethodInvocation => {
            |f, node| expressions::unqualified_method_invocation(f, cast(node))
        }
        K::AnnotationList => |f, node| types::annotation_list(f, cast(node)),
        K::ParameterModifierList => |f, node| types::parameter_modifier_list(f, cast(node)),
        K::EnumConstantList
        | K::RecordComponentList
        | K::FormalParameterList
        | K::StatementExpressionList
        | K::ResourceList
        | K::LambdaParameterList
        | K::CompilationUnitItemList
        | K::ModuleDirectiveList
        | K::RequiresModifierList
        | K::NameList
        | K::ModuleNameList
        | K::AnnotationElementArgumentList
        | K::AnnotationInterfaceBodyMemberList
        | K::AnnotationElementValueList
        | K::TypeParameterSeparatedList
        | K::TypeList
        | K::ClassBodyMemberList
        | K::RecordBodyMemberList
        | K::InterfaceBodyMemberList
        | K::ConstructorBodyEntryList
        | K::BlockStatementList
        | K::SwitchEntryList
        | K::SwitchLabelColonList
        | K::SwitchLabelItemList
        | K::CatchClauseList
        | K::ClassTypeSegmentList
        | K::TypeAmpList
        | K::TypeBarList
        | K::TypeArgumentSeparatedList
        | K::NameSegmentDotList
        | K::DimExpressionList
        | K::VariableInitializerList
        | K::LambdaModifierList
        | K::ExpressionList
        | K::ComponentPatternList => unexpected_list,
        K::Bogus => |_, _| unreachable!("a bogus node is malformed and printed verbatim"),
        _ => |_, node| unreachable!("{node:?} is a token, not a node"),
    }
}
