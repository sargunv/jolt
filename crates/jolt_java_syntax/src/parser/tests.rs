// Java SE 26 grammar and syntax specification:
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-2.html
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-19.html
//
// Java parser focused-test bar. Focused tests should cover:
//
// - every Java syntax grammar declaration, using small representative programs
//   rather than every possible production combination;
// - every known parser ambiguity, including contextual keywords, type-vs-
//   expression boundaries, lambda parameters, casts, patterns, switch labels,
//   and greater-than token splitting in type contexts;
// - error recovery shapes when a diagnostic or recovery boundary is part of
//   parser behavior the formatter depends on.
// - regression tests grounded in actual bugs we have written
//
// Focused tests should not try to enumerate the combinatorial product of the
// grammar. Each test should make one source-shape claim obvious.

use jolt_diagnostics::{DiagnosticStage, Severity, SyntaxOutcome};
use jolt_syntax::Event;

use super::{finish_parse, parse_compilation_unit, source::ParseEvents};
use crate::JavaSyntaxKind;

#[test]
fn parser_shell_wraps_source_in_compilation_unit() {
    let parse = parse_compilation_unit("package a;\nclass A {}\n");
    let syntax = parse.syntax().expect("clean parse should produce syntax");

    assert_eq!(syntax.kind(), JavaSyntaxKind::CompilationUnit);
    assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
    assert!(parse.diagnostics().is_empty());
}

#[test]
fn parser_shell_preserves_source_text() {
    let source = "class A {\n  // hello\n}\n";
    let parse = parse_compilation_unit(source);
    let syntax = parse.syntax().expect("clean parse should produce syntax");

    assert_eq!(syntax.source_text(), source);
}

#[test]
fn invalid_event_stream_aborts_without_syntax() {
    let mut diagnostics = Vec::new();
    let parse = finish_parse(
        String::new(),
        ParseEvents {
            events: vec![Event::Token],
            tokens: Vec::new(),
            trivia: Vec::new(),
            diagnostics: Vec::new(),
        },
        &mut diagnostics,
    );

    assert_eq!(parse.outcome(), SyntaxOutcome::Aborted);
    assert!(parse.syntax().is_none());
    assert_eq!(parse.diagnostics().len(), 1);

    let diagnostic = &parse.diagnostics()[0];
    assert_eq!(
        diagnostic.code.as_str(),
        "internal.syntax.invalid_event_stream"
    );
    assert_eq!(diagnostic.severity, Severity::InternalError);
    assert_eq!(diagnostic.stage, DiagnosticStage::Parser);
    assert_eq!(diagnostic.range, None);
}

#[test]
fn parses_ordinary_compilation_unit_package_imports_and_top_level_types() {
    // Spec: JLS 19 CompilationUnit, OrdinaryCompilationUnit, PackageDeclaration,
    // ImportDeclaration, and TopLevelClassOrInterfaceDeclaration.
    assert_parse_snapshot(
        "parses_ordinary_compilation_unit_package_imports_and_top_level_types",
        r"
            @Deprecated
            package example.parser;

            import java.util.List;
            import java.util.*;
            import static java.util.Collections.emptyList;
            import static java.util.Collections.*;
            import module java.base;

            class A {}
            interface B {}
        ",
    );
}

#[test]
fn parses_single_module_import_declaration() {
    // Spec: JLS 19 SingleModuleImportDeclaration is `import module ModuleName;`.
    assert_parse_snapshot(
        "parses_single_module_import_declaration",
        r"
            import module java.base;

            class ModuleImport {}
        ",
    );
}

#[test]
fn parses_non_module_import_declaration_forms() {
    // Spec: JLS 19 ImportDeclaration includes single type, type-on-demand,
    // single static, and static-on-demand imports.
    assert_parse_snapshot(
        "parses_non_module_import_declaration_forms",
        r"
            import java.util.List;
            import java.util.*;
            import static java.util.Collections.emptyList;
            import static java.util.Collections.*;

            class Imports {}
        ",
    );
}

#[test]
fn ordinary_import_declarations_expose_structured_names() {
    // Spec: JLS 19 SingleTypeImportDeclaration is `import TypeName;`.
    assert_parse_snapshot(
        "ordinary_import_declarations_expose_structured_names",
        r"
            import java.util.List;

            class OrdinaryImport {}
        ",
    );
}

#[test]
fn type_on_demand_import_declarations_expose_package_prefix_names() {
    // Spec: JLS 19 TypeImportOnDemandDeclaration is `import PackageOrTypeName.*;`.
    assert_parse_snapshot(
        "type_on_demand_import_declarations_expose_package_prefix_names",
        r"
            import java.util.*;

            class OnDemandImport {}
        ",
    );
}

#[test]
fn static_import_declarations_expose_structured_names() {
    // Spec: JLS 19 static imports include single-static and static-on-demand forms.
    assert_parse_snapshot(
        "static_import_declarations_expose_structured_names",
        r"
            import static java.util.Collections.emptyList;
            import static java.util.Collections.*;

            class StaticImports {}
        ",
    );
}

#[test]
fn module_import_declarations_expose_structured_module_names() {
    // Spec: JLS 19 SingleModuleImportDeclaration is `import module ModuleName;`.
    assert_parse_snapshot(
        "module_import_declarations_expose_structured_module_names",
        r"
            import module java.base;

            class ModuleImportName {}
        ",
    );
}

#[test]
fn parses_empty_declaration_alternatives() {
    // Spec: JLS 19 permits `;` as an empty declaration at top level and in
    // class, interface, and annotation-interface member positions.
    assert_parse_snapshot(
        "parses_empty_declaration_alternatives",
        r"
            ;

            class EmptyDeclarations {
                ;
            }

            interface EmptyInterface {
                ;
            }

            @interface EmptyAnnotation {
                ;
            }
        ",
    );
}

#[test]
fn parses_compact_compilation_unit_with_imports_and_methods() {
    // Spec: JLS 19 CompilationUnit and CompactCompilationUnit.
    assert_parse_snapshot(
        "parses_compact_compilation_unit_with_imports_and_methods",
        r"
            import java.util.List;

            void main() {
                System.out.println(List.of());
            }
        ",
    );
}

#[test]
fn parses_modular_compilation_unit_and_module_directives() {
    // Spec: JLS 19 ModularCompilationUnit, ModuleDeclaration, and ModuleDirective.
    assert_parse_snapshot(
        "parses_modular_compilation_unit_and_module_directives",
        r"
            import java.lang.annotation.Native;

            @Deprecated
            open module example.module {
                requires transitive static java.sql;
                exports example.api to friend.module;
                opens example.internal to friend.module;
                uses example.Service;
                provides example.Service with example.ServiceImpl;
            }
        ",
    );
}

#[test]
fn parses_normal_class_declaration_clauses_and_members() {
    // Spec: JLS 19 NormalClassDeclaration, class clauses, class body
    // declarations, fields, methods, constructors, and initializers.
    assert_parse_snapshot(
        "parses_normal_class_declaration_clauses_and_members",
        r"
            sealed class A<T extends B & C> extends B implements I permits D {
                @Deprecated
                private int first = 1, second[] = {2};

                static {}
                {}

                A() {
                    this(0);
                }

                A(int value) {}

                <U> void method(final int x, String... rest) throws E {}
                void receiver(A this) {}
                class Inner {
                    void receiver(A A.this) {}
                }
            }

            final class D extends A {}
            class B {}
            interface C {}
            interface I {}
            class E extends Exception {}
        ",
    );
}

#[test]
fn diagnoses_missing_class_body() {
    // Spec: JLS 19 NormalClassDeclaration requires a ClassBody.
    assert_parse_snapshot("diagnoses_missing_class_body", "class A;");
}

#[test]
fn parses_sealed_non_sealed_and_permits_contextual_keywords() {
    // Spec: JLS 3.9 and JLS 19 recognize `sealed`, `non-sealed`, and
    // `permits` contextually in class/interface declarations.
    assert_parse_snapshot(
        "parses_sealed_non_sealed_and_permits_contextual_keywords",
        r"
            sealed class SealedClass permits FinalClass, OpenClass {}
            final class FinalClass extends SealedClass {}
            non-sealed class OpenClass extends SealedClass {}

            sealed interface SealedInterface permits OpenInterface {}
            non-sealed interface OpenInterface extends SealedInterface {}
        ",
    );
}

#[test]
fn parses_context_specific_modifier_productions() {
    // Spec: JLS 19 has context-specific modifier productions for constructors,
    // enum constants, interface constants, and annotation-interface elements.
    assert_parse_snapshot(
        "parses_context_specific_modifier_productions",
        r"
            enum ModifiedEnum {
                @Marker VALUE
            }

            class ModifiedConstructor {
                public ModifiedConstructor() {}
            }

            interface ModifiedInterface {
                public static final int X = 1;
            }

            @interface ModifiedAnnotation {
                @Marker String value();
            }

            @interface Marker {}
        ",
    );
}

#[test]
fn parses_trailing_dims_on_method_and_annotation_element_declarators() {
    // Spec: JLS 19 MethodDeclarator and AnnotationInterfaceElementDeclaration
    // permit trailing Dims after the parameter list.
    assert_parse_snapshot(
        "parses_trailing_dims_on_method_and_annotation_element_declarators",
        r"
            class TrailingMethodDims {
                int values()[];
            }

            @interface TrailingAnnotationElementDims {
                int value()[];
            }
        ",
    );
}

#[test]
fn trailing_method_and_annotation_element_dims_have_per_dimension_nodes() {
    // Spec: JLS 19 MethodDeclarator and AnnotationInterfaceElementDeclaration
    // use Dims, which permits annotations before each individual `[]`.
    assert_parse_snapshot(
        "trailing_method_and_annotation_element_dims_have_per_dimension_nodes",
        r"
            abstract class TrailingMethodPerDimensionDims {
                abstract int values() @A [] @B [];
            }

            @interface TrailingAnnotationElementPerDimensionDims {
                int values() @A [] @B [];
            }

            @interface A {}
            @interface B {}
        ",
    );
}

#[test]
fn parses_explicit_constructor_invocation_forms() {
    // Spec: JLS 19 ExplicitConstructorInvocation includes `this`, `super`,
    // ExpressionName-qualified `super`, and Primary-qualified `super` forms.
    assert_parse_snapshot(
        "parses_explicit_constructor_invocation_forms",
        r#"
            class ConstructorInvocations extends Base {
                ConstructorInvocations() {
                    this(0);
                }

                ConstructorInvocations(String value) {
                    <String>this(value, 0);
                }

                <T> ConstructorInvocations(T value, int marker) {
                    super(value);
                }

                ConstructorInvocations(int value) {
                    super(value);
                }

                class Inner extends Base {
                    Inner(ConstructorInvocations outer) {
                        outer.super(0);
                    }

                    Inner(ConstructorInvocations outer, String value) {
                        outer.<String>super(value);
                    }

                    Inner() {
                        (new ConstructorInvocations()).super(0);
                    }

                    Inner(String value) {
                        (new ConstructorInvocations()).<String>super(value);
                    }
                }

                class GenericSuper extends Base {
                    GenericSuper() {
                        <String>super("value");
                    }
                }
            }

            class Base {
                Base() {}
                Base(int value) {}
                <T> Base(T value) {}
            }
        "#,
    );
}

#[test]
fn parses_constructor_declaration_body_as_constructor_body() {
    // Spec: JLS 19 ConstructorDeclaration contains ConstructorBody, whose
    // optional ExplicitConstructorInvocation is distinct from ordinary blocks.
    assert_parse_snapshot(
        "parses_constructor_declaration_body_as_constructor_body",
        r"
            class ConstructorBodyShape {
                ConstructorBodyShape() {
                    this(0);
                }

                ConstructorBodyShape(int value) {}
            }
        ",
    );
}

#[test]
fn parses_explicit_constructor_invocation_arguments_as_argument_lists() {
    // Spec: JLS 19 ExplicitConstructorInvocation forms carry ArgumentList, and
    // type-argument forms carry TypeArguments before `this` or `super`.
    assert_parse_snapshot(
        "parses_explicit_constructor_invocation_arguments_as_argument_lists",
        r"
            class ConstructorInvocationArguments extends Base {
                ConstructorInvocationArguments() {
                    this(0);
                }

                ConstructorInvocationArguments(String value) {
                    <String>this(value, 0);
                }

                <T> ConstructorInvocationArguments(T value, int marker) {
                    super(value);
                }

                class Inner extends Base {
                    Inner(ConstructorInvocationArguments outer) {
                        outer.super(0);
                    }

                    Inner(ConstructorInvocationArguments outer, String value) {
                        outer.<String>super(value);
                    }

                    Inner() {
                        (new ConstructorInvocationArguments()).super(0);
                    }

                    Inner(String value) {
                        (new ConstructorInvocationArguments()).<String>super(value);
                    }
                }
            }

            class Base {
                Base() {}
                Base(int value) {}
                <T> Base(T value) {}
            }
        ",
    );
}

#[test]
fn parses_method_invocation_primary_qualified_super_constructor_invocations() {
    // Spec: JLS 19 ExplicitConstructorInvocation permits
    // `Primary . [TypeArguments] super ( [ArgumentList] ) ;`, and
    // PrimaryNoNewArray includes MethodInvocation.
    assert_parse_snapshot(
        "parses_method_invocation_primary_qualified_super_constructor_invocations",
        r"
            class MethodInvocationQualifiedSuper {
                Inner makeOuter() {
                    return null;
                }

                class Inner extends Base {
                    Inner() {
                        makeOuter().super(0);
                    }

                    Inner(String value) {
                        makeOuter().<String>super(value);
                    }
                }
            }

            class Base {
                Base() {}
                Base(int value) {}
                <T> Base(T value) {}
            }
        ",
    );
}

#[test]
fn diagnoses_misplaced_explicit_constructor_invocations() {
    // Spec: JLS 19 ConstructorBody permits at most one
    // ExplicitConstructorInvocation, and only before BlockStatements.
    assert_parse_snapshot(
        "diagnoses_misplaced_explicit_constructor_invocations",
        r"
            class MisplacedConstructorInvocations {
                MisplacedConstructorInvocations() {
                    int value = 0;
                    this();
                    super();
                }
            }
        ",
    );
}

#[test]
fn recovers_from_invalid_void_field() {
    // Spec: JLS 19 FieldDeclaration uses UnannType, not `void`.
    assert_parse_snapshot(
        "recovers_from_invalid_void_field",
        "class InvalidVoidField { void x; }",
    );
}

#[test]
fn recovers_missing_declaration_names() {
    assert_parse_snapshot(
        "recovers_missing_declaration_names",
        r"
            class MissingDeclarationNames {
                <T>() {}
                void () {}
            }

            @interface MissingAnnotationElementName {
                int ();
            }

            enum MissingEnumConstantName { , }
        ",
    );
}

#[test]
fn parses_enum_declaration_constants_and_body_declarations() {
    // Spec: JLS 19 EnumDeclaration, EnumBody, EnumConstantList,
    // EnumConstant, and EnumBodyDeclarations.
    assert_parse_snapshot(
        "parses_enum_declaration_constants_and_body_declarations",
        r"
            enum Planet {
                MERCURY(1),
                VENUS(2) {
                    void override() {}
                };

                final int order;

                Planet(int order) {
                    this.order = order;
                }

                void override() {}
            }
        ",
    );
}

#[test]
fn parses_enum_constant_arguments_as_structured_argument_lists() {
    // Spec: JLS 19 EnumConstant permits Arguments, whose ArgumentList contains
    // expressions rather than raw balanced tokens.
    assert_parse_snapshot(
        "parses_enum_constant_arguments_as_structured_argument_lists",
        r#"
            enum StructuredEnumArguments {
                VALUE(1 + helper("x"), new Box())
            }
        "#,
    );
}

#[test]
fn parses_enum_constant_arguments_before_class_body_as_structured_argument_lists() {
    // Spec: JLS 19 EnumConstant permits Arguments followed by an optional
    // ClassBody.
    assert_parse_snapshot(
        "parses_enum_constant_arguments_before_class_body_as_structured_argument_lists",
        r"
            enum StructuredEnumClassBodyArguments {
                SPECIAL(helper(1)) {
                    void run() {}
                }
            }
        ",
    );
}

#[test]
fn parses_record_declaration_header_body_and_compact_constructor() {
    // Spec: JLS 19 RecordDeclaration, RecordHeader, RecordComponentList,
    // RecordComponent, RecordBody, and CompactConstructorDeclaration.
    assert_parse_snapshot(
        "parses_record_declaration_header_body_and_compact_constructor",
        r"
            record Point(@Deprecated int x, String... labels) implements Named {
                Point {
                    labels = labels.clone();
                }

                public String name() {
                    return labels[0];
                }
            }

            interface Named {}
        ",
    );
}

#[test]
fn parses_annotated_record_components() {
    // Spec: JLS 19 RecordComponentModifier permits annotations, and
    // VariableArityRecordComponent permits annotations before `...`.
    assert_parse_snapshot(
        "parses_annotated_record_components",
        r"
            record AnnotatedRecordComponents(@Marker int x, String @Marker ... labels) {}

            @interface Marker {}
        ",
    );
}

#[test]
fn parses_interface_and_annotation_interface_declarations() {
    // Spec: JLS 19 InterfaceDeclaration, InterfaceExtends,
    // InterfacePermits, ConstantDeclaration, InterfaceMethodDeclaration,
    // AnnotationInterfaceDeclaration, AnnotationInterfaceElementDeclaration,
    // and DefaultValue.
    assert_parse_snapshot(
        "parses_interface_and_annotation_interface_declarations",
        r#"
            sealed interface Shape extends Drawable permits Circle {
                int SIDES = 0;

                private static void helper() {}

                void draw();
            }

            non-sealed interface Circle extends Shape {}
            interface Drawable {}

            @interface JsonName {
                String value() default "";
                int[] flags() default {1, 2};
            }
        "#,
    );
}

#[test]
fn parses_annotation_forms_and_element_values() {
    // Spec: JLS 19 Annotation, NormalAnnotation, MarkerAnnotation,
    // SingleElementAnnotation, ElementValuePairList, ElementValuePair,
    // ElementValue, and ElementValueArrayInitializer.
    assert_parse_snapshot(
        "parses_annotation_forms_and_element_values",
        r#"
            @Marker
            @Single("value")
            @Normal(name = "test", nested = @Marker, values = {1, 2, 3})
            class Annotated {}
        "#,
    );
}

#[test]
fn annotation_interface_members_are_annotation_element_declarations() {
    // Spec: JLS 19 AnnotationInterfaceElementDeclaration is a declaration
    // inside an annotation interface, distinct from annotation-use values.
    assert_parse_snapshot(
        "annotation_interface_members_are_annotation_element_declarations",
        r#"
            @interface Contract {
                String value() default "x";
                int[] flags() default {1, 2};
                Nested nested() default @Nested;
            }

            @interface Nested {}
        "#,
    );
}

#[test]
fn normal_annotation_pairs_have_structured_element_value_pairs() {
    // Spec: JLS 19 NormalAnnotation uses ElementValuePair nodes for each
    // `Identifier = ElementValue` entry, and ElementValue preserves nested
    // annotations and array initializers structurally.
    assert_parse_snapshot(
        "normal_annotation_pairs_have_structured_element_value_pairs",
        r#"
            @Normal(name = "test", nested = @Marker, values = {1, 2})
            class Annotated {}

            @interface Normal {
                String name();
                Marker nested();
                int[] values();
            }

            @interface Marker {}
        "#,
    );
}

#[test]
fn single_element_annotations_and_defaults_have_structured_element_values() {
    // Spec: JLS 19 SingleElementAnnotation and DefaultValue both contain
    // ElementValue, which may be an expression, nested annotation, or
    // ElementValueArrayInitializer.
    assert_parse_snapshot(
        "single_element_annotations_and_defaults_have_structured_element_values",
        r#"
            @StringSingle("test")
            @NestedSingle(@Marker)
            @ArraySingle({@Marker, "x"})
            class Annotated {}

            @interface StringSingle {
                String value() default "fallback";
            }

            @interface NestedSingle {
                Marker value() default @Marker;
            }

            @interface ArraySingle {
                Object[] value() default {@Marker, "y"};
            }

            @interface Marker {}
        "#,
    );
}

#[test]
fn parses_dangling_else_with_nearest_if_binding() {
    // Spec: JLS 19 StatementNoShortIf and IfThenElseStatementNoShortIf split
    // the dangling-else ambiguity so `else` binds to the nearest eligible `if`.
    assert_parse_snapshot(
        "parses_dangling_else_with_nearest_if_binding",
        r"
            class DanglingElse {
                void method(boolean first, boolean second) {
                    if (first)
                        if (second)
                            winner();
                        else
                            loser();
                }

                void winner() {}
                void loser() {}
            }
        ",
    );
}

#[test]
fn parses_type_shapes_and_type_arguments() {
    // Spec: JLS 19 Type, PrimitiveType, ReferenceType,
    // ClassOrInterfaceType, TypeArguments, TypeArgument, Wildcard,
    // WildcardBounds, ArrayType, and Dims.
    assert_parse_snapshot(
        "parses_type_shapes_and_type_arguments",
        r"
            class Types<T extends Number & Comparable<T>> {
                int primitive;
                double floating;
                java.util.Map<String, ? extends Number> upper;
                java.util.List<? super T>[] lower;
                T[][] matrix;
            }
        ",
    );
}

#[test]
fn void_method_results_and_class_literals_have_void_type_nodes() {
    // Spec: JLS 19 Result and ClassLiteral both use the `void` terminal as a
    // distinct type shape rather than an identifier or ordinary keyword bucket.
    assert_parse_snapshot(
        "void_method_results_and_class_literals_have_void_type_nodes",
        r"
            class VoidTypeShapes {
                void method() {
                    Object literal = void.class;
                }
            }
        ",
    );
}

#[test]
fn type_bounds_and_casts_have_intersection_type_nodes() {
    // Spec: JLS 19 TypeBound and CastExpression both represent `A & B` with
    // AdditionalBound; the CST should expose that full intersection as a type
    // node for formatter traversal.
    assert_parse_snapshot(
        "type_bounds_and_casts_have_intersection_type_nodes",
        r"
            interface A {}
            interface B {}

            class IntersectionTypeShapes<T extends A & B> {
                Object method(Object value) {
                    return (A & B) value;
                }
            }
        ",
    );
}

#[test]
fn parses_floating_point_type() {
    // Spec: JLS 19 FloatingPointType includes `float` and `double`.
    assert_parse_snapshot(
        "parses_floating_point_type",
        r"
            class FloatingPointTypes {
                float single;
                double wide;
            }
        ",
    );
}

#[test]
fn parses_qualified_receiver_parameter() {
    // Spec: JLS 19 ReceiverParameter permits `UnannType Identifier . this`.
    assert_parse_snapshot(
        "parses_qualified_receiver_parameter",
        r"
            class ReceiverOuter {
                class ReceiverInner {
                    void method(ReceiverOuter ReceiverOuter.this) {}
                }
            }
        ",
    );
}

#[test]
fn parses_constructor_receiver_parameter() {
    // Spec: JLS 19 ConstructorDeclarator permits a leading ReceiverParameter.
    assert_parse_snapshot(
        "parses_constructor_receiver_parameter",
        r"
            class ConstructorReceiverOuter {
                class ConstructorReceiverInner {
                    ConstructorReceiverInner(ConstructorReceiverOuter ConstructorReceiverOuter.this) {}
                }
            }
        ",
    );
}

#[test]
fn diagnoses_misplaced_receiver_parameter() {
    // Spec: JLS 19 ReceiverParameter is an optional leading parameter.
    assert_parse_snapshot(
        "diagnoses_misplaced_receiver_parameter",
        r"
            class MisplacedReceiverOuter {
                class MisplacedReceiverInner {
                    void method(String value, MisplacedReceiverOuter MisplacedReceiverOuter.this) {}
                }
            }
        ",
    );
}

#[test]
fn parses_annotated_array_dimensions() {
    // Spec: JLS 19 Dims permits annotations before each `[]`.
    assert_parse_snapshot(
        "parses_annotated_array_dimensions",
        r"
            class AnnotatedDims {
                String @Marker [] @Marker [] names;
            }

            @interface Marker {}
        ",
    );
}

#[test]
fn array_type_dims_have_per_dimension_nodes() {
    // Spec: JLS 19 ArrayType uses Dims, which permits annotations before each
    // individual `[]`.
    assert_parse_snapshot(
        "array_type_dims_have_per_dimension_nodes",
        r"
            class ArrayTypePerDimensionDims {
                String @A [] @B [] names;
            }

            @interface A {}
            @interface B {}
        ",
    );
}

#[test]
fn parses_annotated_type_parameter_modifier() {
    // Spec: JLS 19 TypeParameterModifier permits annotations on type
    // parameters.
    assert_parse_snapshot(
        "parses_annotated_type_parameter_modifier",
        r"
            class AnnotatedTypeParameter<@Marker T> {}

            @interface Marker {}
        ",
    );
}

#[test]
fn parses_block_local_declarations_and_statement_forms() {
    // Spec: JLS 19 Block, BlockStatement, LocalClassOrInterfaceDeclaration,
    // LocalVariableDeclarationStatement, LocalVariableDeclaration, EmptyStatement,
    // LabeledStatement, ExpressionStatement, IfThenStatement,
    // IfThenElseStatement, and AssertStatement.
    assert_parse_snapshot(
        "parses_block_local_declarations_and_statement_forms",
        r"
            class Statements {
                void method(Object o) {
                    ;
                    class LocalClass {}
                    interface LocalInterface {}
                    final var local = o;
                    label: if (local == null) ;
                    if (local instanceof String s) s.trim(); else local.toString();
                    assert local != null : local;
                }
            }
        ",
    );
}

#[test]
fn parses_var_local_variable_type() {
    // Spec: JLS 3.9 recognizes `var` contextually as LocalVariableType.
    assert_parse_snapshot(
        "parses_var_local_variable_type",
        r"
            class VarLocalVariable {
                void method(Object value) {
                    final var local = value;
                }
            }
        ",
    );
}

#[test]
fn parses_qualified_contextual_keyword_local_types() {
    // Spec: JLS 3.9 restricts `var` and `yield` as TypeIdentifier, but
    // contextual spellings can still start a qualified class type.
    assert_parse_snapshot(
        "parses_qualified_contextual_keyword_local_types",
        r"
            class QualifiedContextualKeywordLocalTypes {
                void method() {
                    var.Type x;
                    yield.Type y;
                }
            }
        ",
    );
}

#[test]
fn parses_class_instance_creation_statement_expression() {
    // Spec: JLS 19 StatementExpression includes ClassInstanceCreationExpression.
    assert_parse_snapshot(
        "parses_class_instance_creation_statement_expression",
        r"
            class CreationStatementExpression {
                void method() {
                    new Object();
                    new Runnable() {
                        public void run() {}
                    };
                }
            }
        ",
    );
}

#[test]
fn diagnoses_invalid_statement_expressions() {
    // Spec: JLS 19 StatementExpression is limited to assignment, pre/post
    // increment/decrement, method invocation, and class instance creation.
    assert_parse_snapshot(
        "diagnoses_invalid_statement_expressions",
        r"
            class InvalidStatementExpressions {
                void method(int i, int j) {
                    1 + 2;
                    i;
                    for (i + 1; i < 10; j + 1) {}
                }
            }
        ",
    );
}

#[test]
fn diagnoses_invalid_expression_forms() {
    assert_parse_snapshot(
        "diagnoses_invalid_expression_forms",
        r"
            class InvalidExpressions {
                void method(Object x, C c) {
                    (f)();
                    this();
                    new C()();
                    1 = x;
                    a + b = c;
                    (a) = b;
                    new C;
                    new C {};
                    new int();
                    Object invalidQualifiedCreation = new Outer<String>.Inner();
                    Object validQualifiedCreation = new Outer.Inner<String>();
                    int[] xs = new int[][3];
                    int[] ys = new int[3] {1, 2};
                    boolean primitiveInstanceof = x instanceof int;
                }
            }
        ",
    );
}

#[test]
fn diagnoses_invalid_declaration_contexts() {
    assert_parse_snapshot(
        "diagnoses_invalid_declaration_contexts",
        r"
            class InvalidDeclarationContexts<T extends int> {
                void method(Object x, java.util.List<String> values) throws int {
                    for (String value = null : values) {}
                    for (String first, second : values) {}
                    try (AutoCloseable missing, second = open()) {}
                    try (AutoCloseable first = open(), second = open()) {}
                    try {
                        risky();
                    } catch (int ex) {
                    }
                }

                AutoCloseable open() { return null; }
                void risky() throws Exception {}
                transient void transientMethod() {}
                volatile InvalidDeclarationContexts() {}
                synchronized int synchronizedField;
            }
        ",
    );
}

#[test]
fn parses_switch_statement_rules_groups_labels_and_guards() {
    // Spec: JLS 19 SwitchStatement, SwitchBlock, SwitchRule,
    // SwitchBlockStatementGroup, SwitchLabel, CaseConstant, CasePattern,
    // and Guard.
    assert_parse_snapshot(
        "parses_switch_statement_rules_groups_labels_and_guards",
        r"
            class Switches {
                void statement(Object value, int count) {
                    switch (value) {
                        case null, default -> {}
                        case String s when s.isEmpty() -> s.trim();
                        case Integer i -> {}
                    }

                    switch (count) {
                        case 1:
                        case 2:
                            break;
                        default:
                            break;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_switch_block_statement_group() {
    // Spec: JLS 19 SwitchBlockStatementGroup preserves colon-form switch
    // labels with the block statements they introduce.
    assert_parse_snapshot(
        "parses_switch_block_statement_group",
        r"
            class SwitchGroups {
                void method(int value) {
                    switch (value) {
                        case 1:
                        case 2:
                            value++;
                            break;
                    }
                }
            }
        ",
    );
}

#[test]
fn switch_case_constant_label_items_are_structured() {
    // Spec: JLS 19 SwitchLabel case constants are a comma-separated list of
    // CaseConstant elements, including signed integer constants and names.
    assert_parse_snapshot(
        "switch_case_constant_label_items_are_structured",
        r"
            class SwitchCaseConstants {
                static final int NAME = 3;

                void method(int value) {
                    switch (value) {
                        case 1, -2, NAME -> value++;
                    }
                }
            }
        ",
    );
}

#[test]
fn switch_case_constant_can_be_a_conditional_expression() {
    // Spec: JLS 19 CaseConstant is a ConditionalExpression, so a valid
    // expression item must remain one CaseConstant instead of being split at
    // expression operators.
    assert_parse_snapshot(
        "switch_case_constant_can_be_a_conditional_expression",
        r"
            class SwitchCaseConstantExpression {
                static final int OFFSET = 1;

                void method(int value) {
                    switch (value) {
                        case 1 + OFFSET -> value++;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_case_null_default_switch_label() {
    // Spec: JLS 19 SwitchLabel has a special `case null, default` alternative.
    assert_parse_snapshot(
        "parses_case_null_default_switch_label",
        r"
            class NullDefaultSwitchLabel {
                int method(Object value) {
                    return switch (value) {
                        case null, default -> 0;
                    };
                }
            }
        ",
    );
}

#[test]
fn switch_case_pattern_label_items_are_structured_with_guards() {
    // Spec: JLS 19 SwitchLabel permits `case CasePattern Guard`.
    assert_parse_snapshot(
        "switch_case_pattern_label_items_are_structured_with_guards",
        r"
            class SwitchCasePatternGuard {
                void method(Object value) {
                    switch (value) {
                        case String s when s.isEmpty() -> s.trim();
                        case String s when (s.isBlank()) -> s.trim();
                    }
                }
            }
        ",
    );
}

#[test]
fn disambiguates_when_in_switch_labels() {
    // Spec: JLS 19 `when` starts a Guard only after a CasePattern.
    assert_parse_snapshot(
        "disambiguates_when_in_switch_labels",
        r"
            class SwitchWhenIdentifier {
                void method(int when) {
                    switch (when) {
                        case when:
                            break;
                    }
                }
            }
        ",
    );
    assert_parse_snapshot(
        "disambiguates_when_in_switch_labels__invalid_guard",
        r"
            class SwitchInvalidGuard {
                void method(int value) {
                    switch (value) {
                        case 1 when true:
                            break;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_switch_rule_with_throw_statement() {
    // Spec: JLS 19 SwitchRule permits `SwitchLabel -> ThrowStatement`.
    assert_parse_snapshot(
        "parses_switch_rule_with_throw_statement",
        r"
            class ThrowSwitchRule {
                int method(Object value) {
                    return switch (value) {
                        case null -> throw new IllegalArgumentException();
                        default -> 0;
                    };
                }
            }
        ",
    );
}

#[test]
fn parses_loop_jump_synchronized_and_try_statements() {
    // Spec: JLS 19 WhileStatement, DoStatement, BasicForStatement,
    // EnhancedForStatement, BreakStatement, ContinueStatement, ReturnStatement,
    // ThrowStatement, SynchronizedStatement, TryStatement, Catches,
    // CatchClause, CatchType, Finally, TryWithResourcesStatement,
    // ResourceSpecification, ResourceList, Resource, and VariableAccess.
    assert_parse_snapshot(
        "parses_loop_jump_synchronized_and_try_statements",
        r"
            class Flow {
                int method(java.util.List<String> values) throws Exception {
                    while (values.isEmpty()) continue;
                    do { break; } while (false);
                    for (int i = 0; i < 10; i++) {}
                    for (String value : values) {}
                    for (SomeClass<?> value : values) {}
                    synchronized (this) {}
                    try {
                        throw new Exception();
                    } catch (java.io.IOException | RuntimeException ex) {
                        return 1;
                    } finally {
                        values.clear();
                    }
                    try (var ignored = open(); existing) {}
                    return 0;
                }

                AutoCloseable open() { return null; }
                AutoCloseable existing;
            }
        ",
    );
    assert_parse_snapshot(
        "parses_loop_jump_synchronized_and_try_statements__resources",
        r"
            class Resources {
                void method() throws Exception {
                    try (var declared = open(); existing) {}
                }

                AutoCloseable open() { return null; }
                AutoCloseable existing;
            }
        ",
    );
}

#[test]
fn try_with_resources_has_resource_specification_boundary() {
    // Spec: JLS 19 TryWithResourcesStatement contains a ResourceSpecification
    // that wraps the parenthesized resource list and optional trailing `;`.
    assert_parse_snapshot(
        "try_with_resources_has_resource_specification_boundary",
        r"
            class ResourceSpecificationShape {
                void method() throws Exception {
                    try (var declared = open(); existing;) {
                    }
                }

                AutoCloseable open() { return null; }
                AutoCloseable existing;
            }
        ",
    );
}

#[test]
fn parses_basic_for_with_conditional_initializer() {
    // Spec: JLS 19 BasicForStatement permits a statement-expression list as
    // the initializer; `? :` inside that initializer is not an enhanced-for
    // colon.
    assert_parse_snapshot(
        "parses_basic_for_with_conditional_initializer",
        r"
            class BasicForConditionalInitializer {
                void method(boolean flag, int n) {
                    int i;
                    for (i = flag ? 1 : 2; i < n; i++) {}
                }
            }
        ",
    );
}

#[test]
fn diagnoses_expression_try_with_resource() {
    // Spec: JLS 19 Resource is a local variable declaration or VariableAccess,
    // not an arbitrary method invocation expression.
    assert_parse_snapshot(
        "diagnoses_expression_try_with_resource",
        r"
            class ExpressionTryWithResource {
                void method() throws Exception {
                    try (open()) {}
                }

                AutoCloseable open() { return null; }
            }
        ",
    );
}

#[test]
fn catch_clause_has_catch_parameter_boundary() {
    // Spec: JLS 19 CatchFormalParameter is catch-specific: it combines
    // variable modifiers, CatchType, and VariableDeclaratorId.
    assert_parse_snapshot(
        "catch_clause_has_catch_parameter_boundary",
        r"
            class CatchParameterShape {
                void method() {
                    try {
                        risky();
                    } catch (final java.io.IOException | RuntimeException ex) {
                    }
                }

                void risky() throws java.io.IOException {}
            }
        ",
    );
}

#[test]
fn catch_union_types_have_union_type_nodes() {
    // Spec: JLS 19 CatchType permits a union of ClassType alternatives joined
    // by `|`; the CST should expose the full union type, not only the catch
    // parameter or surrounding catch-type list.
    assert_parse_snapshot(
        "catch_union_types_have_union_type_nodes",
        r"
            class CatchUnionTypeShape {
                void method() {
                    try {
                        risky();
                    } catch (java.io.IOException | RuntimeException ex) {
                    }
                }

                void risky() throws java.io.IOException {}
            }
        ",
    );
}

#[test]
fn diagnoses_bare_try_statement() {
    // Spec: JLS 19 TryStatement requires catches, finally, or resources.
    assert_parse_snapshot(
        "diagnoses_bare_try_statement",
        r"
            class BareTry {
                void method() {
                    try {}
                }
            }
        ",
    );
}

#[test]
fn parses_optional_basic_for_condition_and_bare_return() {
    // Spec: JLS 19 BasicForStatement has an optional Expression; ReturnStatement
    // has an optional Expression. Empty forms must not produce recovery nodes.
    assert_parse_snapshot(
        "parses_optional_basic_for_condition_and_bare_return",
        r"
            class OptionalFlowExpressions {
                void method() {
                    for (;;) {
                        return;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_field_access_resource_variable_access() {
    // Spec: JLS 19 VariableAccess in try-with-resources permits FieldAccess as
    // well as ExpressionName.
    assert_parse_snapshot(
        "parses_field_access_resource_variable_access",
        r"
            class FieldAccessResource {
                AutoCloseable existing;

                void method() throws Exception {
                    try (this.existing) {}
                }
            }
        ",
    );
}

#[test]
fn parses_unnamed_variable_declarator_ids() {
    // Spec: JLS 19 VariableDeclaratorId permits `_` as an unnamed variable,
    // including locals, formals, catch parameters, resources, and normal
    // lambda parameters.
    assert_parse_snapshot(
        "parses_unnamed_variable_declarator_ids",
        r"
            class UnnamedVariableDeclarators {
                void method(Object _, java.util.List<String> values) throws Exception {
                    var _ = values;
                    try (AutoCloseable _ = open()) {
                    } catch (Exception _) {
                    }
                    java.util.function.IntUnaryOperator zero = (int _) -> 0;
                }

                AutoCloseable open() {
                    return null;
                }
            }
        ",
    );
}

#[test]
fn parses_for_statement_expression_lists() {
    // Spec: JLS 19 StatementExpressionList is used by both statement-expression
    // ForInit and ForUpdate alternatives.
    assert_parse_snapshot(
        "parses_for_statement_expression_lists",
        r"
            class ForExpressionLists {
                void method() {
                    int i;
                    int j;
                    for (i = 0, j = 0; i < 10; i++, j++) {}
                }
            }
        ",
    );
}

#[test]
fn parses_array_initializers_and_array_creation_forms() {
    // Spec: JLS 19 ArrayInitializer, VariableInitializerList,
    // ArrayCreationExpression, DimExprs, DimExpr, and Dims.
    assert_parse_snapshot(
        "parses_array_initializers_and_array_creation_forms",
        r#"
            class Arrays {
                void method(int n) {
                    int[] literal = {1, 2, 3};
                    int[][] sized = new int[n][n];
                    String[] named = new String[] {""};
                }
            }
        "#,
    );
}

#[test]
fn parses_annotated_dim_expression() {
    // Spec: JLS 19 DimExpr permits annotations before a sized array dimension.
    assert_parse_snapshot(
        "parses_annotated_dim_expression",
        r"
            class AnnotatedDimExpression {
                void method(int n) {
                    int[] values = new int @Marker [n];
                }
            }

            @interface Marker {}
        ",
    );
}

#[test]
fn array_creation_unsized_trailing_dims_have_per_dimension_nodes() {
    // Spec: JLS 19 ArrayCreationExpression permits sized DimExprs followed by
    // unsized Dims, whose annotations belong to each individual `[]`.
    assert_parse_snapshot(
        "array_creation_unsized_trailing_dims_have_per_dimension_nodes",
        r"
            class ArrayCreationPerDimensionDims {
                void method() {
                    Object values = new String[1] @A [] @B [];
                }
            }

            @interface A {}
            @interface B {}
        ",
    );
}

#[test]
fn parses_primary_access_invocation_creation_and_references() {
    // Spec: JLS 19 Primary, PrimaryNoNewArray, ClassLiteral,
    // ClassInstanceCreationExpression, ArrayAccess, FieldAccess,
    // MethodInvocation, ArgumentList, and MethodReference.
    assert_parse_snapshot(
        "parses_primary_access_invocation_creation_and_references",
        r"
            class Expressions extends Base {
                Object field;

                void method(String[] values) {
                    Object classLiteral = String[][].class;
                    Object created = new Outer<String>().new Inner(1);
                    Object anonymous = new Runnable() {
                        public void run() {}
                    };
                    Object literal = null;
                    Object parenthesized = (created);
                    Object fieldAccess = this.field;
                    Object superAccess = super.toString();
                    Object arrayAccess = values[0];
                    Object invocation = created.toString();
                    java.util.function.Function<String, Integer> ref = String::length;
                }
            }

            class Base {}

            class Outer<T> {
                class Inner {
                    Inner(int value) {}
                }
            }
        ",
    );
}

#[test]
fn parses_super_field_access_forms() {
    // Spec: JLS 19 FieldAccess includes `super.Identifier` and
    // `TypeName.super.Identifier` forms in addition to primary field access.
    assert_parse_snapshot(
        "parses_super_field_access_forms",
        r"
            class SuperFieldAccess extends Base {
                int field;

                class Inner extends Base {
                    int method() {
                        return super.field + SuperFieldAccess.super.field;
                    }
                }
            }

            class Base {
                int field;
            }
        ",
    );
}

#[test]
fn parses_primitive_and_void_class_literals() {
    // Spec: JLS 19 ClassLiteral includes primitive, boolean, and void
    // alternatives in addition to reference types.
    assert_parse_snapshot(
        "parses_primitive_and_void_class_literals",
        r"
            class PrimitiveClassLiterals {
                void method() {
                    Object intClass = int.class;
                    Object booleanClass = boolean.class;
                    Object voidClass = void.class;
                }
            }
        ",
    );
}

#[test]
fn parses_qualified_this_expression() {
    // Spec: JLS 19 PrimaryNoNewArray includes `TypeName . this`.
    assert_parse_snapshot(
        "parses_qualified_this_expression",
        r"
            class QualifiedThis {
                class Inner {
                    Object method() {
                        return QualifiedThis.this;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_array_access_forms() {
    // Spec: JLS 19 ArrayAccess includes ExpressionName, PrimaryNoNewArray, and
    // ArrayCreationExpressionWithInitializer bases.
    assert_parse_snapshot(
        "parses_array_access_forms",
        r"
            class ArrayAccessForms {
                void method(int[] values) {
                    int byName = values[0];
                    int byPrimary = (values)[0];
                    int byCreation = new int[] {1}[0];
                }
            }
        ",
    );
}

#[test]
fn parses_diamond_class_instance_creation() {
    // Spec: JLS 19 TypeArgumentsOrDiamond permits `<>` in class instance
    // creation, independently from explicit type arguments.
    assert_parse_snapshot(
        "parses_diamond_class_instance_creation",
        r"
            class DiamondCreation {
                void method() {
                    Object list = new java.util.ArrayList<>();
                }
            }
        ",
    );
}

#[test]
fn parses_expression_name_qualified_class_instance_creation() {
    // Spec: JLS 19 ClassInstanceCreationExpression includes
    // ExpressionName-qualified `new` forms.
    assert_parse_snapshot(
        "parses_expression_name_qualified_class_instance_creation",
        r"
            class QualifiedCreation {
                Outer outer;

                void method() {
                    Object inner = outer.new Inner();
                }
            }

            class Outer {
                class Inner {}
            }
        ",
    );
}

#[test]
fn parses_primary_qualified_class_instance_creation() {
    // Spec: JLS 19 ClassInstanceCreationExpression includes Primary-qualified
    // `new` forms.
    assert_parse_snapshot(
        "parses_primary_qualified_class_instance_creation",
        r"
            class PrimaryQualifiedCreation {
                void method() {
                    Object inner = new Outer().new Inner();
                }
            }

            class Outer {
                class Inner {}
            }
        ",
    );
}

#[test]
fn parses_class_instance_creation_with_constructor_type_arguments() {
    // Spec: JLS 19 UnqualifiedClassInstanceCreationExpression permits
    // constructor type arguments immediately after `new`.
    assert_parse_snapshot(
        "parses_class_instance_creation_with_constructor_type_arguments",
        r#"
            class ConstructorTypeArgumentsCreation {
                void method() {
                    Object box = new <String> Box("value");
                }
            }

            class Box<T> {
                <U> Box(U value) {}
            }
        "#,
    );
}

#[test]
fn parses_method_reference_forms() {
    // Spec: JLS 19 MethodReference has distinct expression-name, primary,
    // reference-type, super, qualified-super, class-constructor, and
    // array-constructor forms.
    assert_parse_snapshot(
        "parses_method_reference_forms",
        r"
            class MethodReferences extends Base {
                <T> T id(T value) { return value; }

                void method(MethodReferences target, String value) {
                    java.util.function.Supplier<Integer> expressionName = value::length;
                    java.util.function.Supplier<String> primary = (value)::trim;
                    java.util.function.Function<String, Integer> referenceType = String::length;
                    java.util.function.Supplier<String> superReference = super::toString;
                    java.util.function.Supplier<String> qualifiedSuper = MethodReferences.super::toString;
                    java.util.function.Supplier<MethodReferences> constructor = MethodReferences::new;
                    java.util.function.IntFunction<String[]> arrayConstructor = String[]::new;
                    java.util.function.Function<String, String> expressionNameGeneric = target::<String>id;
                    java.util.function.Function<String, String> primaryGeneric = this::<String>id;
                    java.util.function.Function<String, String> referenceTypeGeneric = MethodReferences::<String>staticId;
                    java.util.function.Function<String, String> superGeneric = super::<String>baseId;
                    java.util.function.Function<String, String> qualifiedSuperGeneric = MethodReferences.super::<String>baseId;
                    java.util.function.Supplier<java.util.ArrayList<String>> constructorGeneric = java.util.ArrayList<String>::<String>new;
                }

                static <T> T staticId(T value) { return value; }
            }

            class Base {
                <T> T baseId(T value) { return value; }
            }
        ",
    );
}

#[test]
fn parses_primitive_array_method_reference() {
    // Spec: JLS 19 MethodReference permits array-constructor references whose
    // ReferenceType is a primitive array type.
    assert_parse_snapshot(
        "parses_primitive_array_method_reference",
        r"
            class PrimitiveArrayMethodReference {
                void method() {
                    java.util.function.IntFunction<int[]> factory = int[]::new;
                }
            }
        ",
    );
}

#[test]
fn parses_annotated_expression_type_uses() {
    // Spec: JLS 19 permits type-use annotations in casts, constructor type
    // arguments, and method reference reference types/type arguments.
    assert_parse_snapshot(
        "parses_annotated_expression_type_uses",
        r"
            class AnnotatedExpressionTypeUses {
                void method(Object o) {
                    new @A(0x44) ArrayList<>();
                    java.util.function.Supplier<java.util.List<?>> a = @A(0x45) ArrayList::new;
                    java.util.function.Supplier<java.util.List<?>> b = @A(0x46) ImmutableList::of;
                    String s = (@A(0x47) String) o;
                    java.util.List<?> xs = new ArrayList<@A(0x48) String>();
                    xs = ImmutableList.<@A(0x49) String>of();
                    java.util.function.Supplier<java.util.List<?>> c = ArrayList<@A(0x4A) String>::new;
                    java.util.function.Supplier<java.util.List<?>> d = ImmutableList::<@A(0x4B) String>of;
                }
            }

            @interface A {
                int value();
            }
        ",
    );
}

#[test]
fn parses_method_invocation_forms() {
    // Spec: JLS 19 MethodInvocation has simple-name, type-name,
    // expression-name, primary, super, and TypeName.super forms.
    assert_parse_snapshot(
        "parses_method_invocation_forms",
        r#"
            class MethodInvocations extends Base {
                static void staticMethod() {}
                static <T> void staticGeneric(T value) {}
                void simple() {}
                void instance() {}
                <T> void generic(T value) {}

                void method(MethodInvocations target) {
                    simple();
                    MethodInvocations.staticMethod();
                    target.instance();
                    (target).instance();
                    super.baseMethod();
                    MethodInvocations.super.baseMethod();
                    this.<String>generic("value");
                    target.<String>generic("value");
                    MethodInvocations.<String>staticGeneric("value");
                    super.<String>baseGeneric("value");
                    MethodInvocations.super.<String>baseGeneric("value");
                }
            }

            class Base {
                void baseMethod() {}
                <T> void baseGeneric(T value) {}
            }
        "#,
    );
}

#[test]
fn parses_anonymous_class_instance_creation_body() {
    // Spec: JLS 19 UnqualifiedClassInstanceCreationExpression permits an
    // optional ClassBody for anonymous class creation.
    assert_parse_snapshot(
        "parses_anonymous_class_instance_creation_body",
        r"
            class AnonymousCreation {
                Runnable runnable = new Runnable() {
                    public void run() {}
                };
            }
        ",
    );
}

#[test]
fn recovers_invalid_syntax_with_error_nodes_and_diagnostics() {
    // Spec: formatter parser behavior for malformed input. The JLS grammar
    // defines valid syntax; the formatter parser still needs lossless recovery
    // boundaries for invalid syntax.
    assert_parse_snapshot(
        "recovers_invalid_syntax_with_error_nodes_and_diagnostics",
        r"
            class Broken {
                void method( {
                    int value = ;
                }
            }
        ",
    );
}

#[test]
fn parses_lambda_assignment_conditional_and_operator_expressions() {
    // Spec: JLS 19 LambdaExpression, LambdaParameters, LambdaParameterList,
    // LambdaParameter, AssignmentExpression, Assignment, LeftHandSide,
    // ConditionalExpression, and binary expression precedence productions.
    assert_parse_snapshot(
        "parses_lambda_assignment_conditional_and_operator_expressions",
        r"
            class Operators {
                void method() {
                    java.util.function.Function<String, String> trim = (String s) -> s.trim();
                    java.util.function.IntUnaryOperator inc = x -> x + 1;
                    int a = 1, b = 2, c = 3;
                    a = b > c ? b + c * 2 : b | c ^ a & b;
                }
            }
        ",
    );
}

#[test]
fn parses_compound_assignment_operators() {
    // Spec: JLS 19 AssignmentOperator includes compound assignment operators.
    assert_parse_snapshot(
        "parses_compound_assignment_operators",
        r"
            class CompoundAssignments {
                void method() {
                    int value = 1;
                    value += 2;
                    value <<= 1;
                    value >>>= 1;
                }
            }
        ",
    );
}

#[test]
fn parses_assignment_left_hand_side_forms() {
    // Spec: JLS 19 LeftHandSide permits ExpressionName, FieldAccess, and
    // ArrayAccess assignments.
    assert_parse_snapshot(
        "parses_assignment_left_hand_side_forms",
        r"
            class AssignmentLeftHandSides {
                int field;

                void method(int[] values) {
                    int local;
                    local = 1;
                    this.field = 2;
                    values[0] = 3;
                }
            }
        ",
    );
}

#[test]
fn parses_logical_and_shift_binary_expression_precedence() {
    // Spec: JLS 19 binary expression precedence productions, including
    // conditional-or, conditional-and, inclusive-or, exclusive-or, and shift.
    assert_parse_snapshot(
        "parses_logical_and_shift_binary_expression_precedence",
        r"
            class BinaryPrecedence {
                void method(int a, int b, int c) {
                    boolean logical = a < b && b < c || c == a;
                    int arithmetic = a + b * c - a / b % c;
                    int left = a << 1;
                    int right = b >> 1;
                    int unsigned = c >>> 1;
                    int bits = a | b ^ c & a;
                }
            }
        ",
    );
}

#[test]
fn parses_unary_postfix_cast_and_switch_expressions() {
    // Spec: JLS 19 UnaryExpression, PreIncrementExpression,
    // PreDecrementExpression, UnaryExpressionNotPlusMinus,
    // PostfixExpression, PostIncrementExpression, PostDecrementExpression,
    // CastExpression, and SwitchExpression.
    assert_parse_snapshot(
        "parses_unary_postfix_cast_and_switch_expressions",
        r"
            class ExpressionEdges {
                int method(Object value) {
                    int i = 0;
                    ++i;
                    --i;
                    i++;
                    i--;
                    int cast = (int) value;
                    return switch (value) {
                        case Integer n -> {
                            yield n;
                        }
                        default -> -i;
                    };
                }
            }
        ",
    );
}

#[test]
fn parses_record_type_and_match_all_patterns() {
    // Spec: JLS 19 Pattern, TypePattern, RecordPattern,
    // ComponentPatternList, ComponentPattern, and MatchAllPattern.
    assert_parse_snapshot(
        "parses_record_type_and_match_all_patterns",
        r"
            record Pair(int left, int right) {}

            class Patterns {
                int method(Object value) {
                    if (value instanceof Pair(int left, _)) {
                        return left;
                    }
                    return switch (value) {
                        case Pair(int left, int right) -> left + right;
                        case String text when !text.isEmpty() -> text.length();
                        default -> 0;
                    };
                }
            }
        ",
    );
}

#[test]
fn parses_instanceof_reference_type_and_pattern_forms() {
    // Spec: JLS 19 RelationalExpression permits both `instanceof ReferenceType`
    // and `instanceof Pattern`.
    assert_parse_snapshot(
        "parses_instanceof_reference_type_and_pattern_forms",
        r"
            class InstanceofForms {
                boolean method(Object value) {
                    return value instanceof java.util.List<?> || value instanceof String text;
                }
            }
        ",
    );
}

#[test]
fn instanceof_forms_have_dedicated_expression_nodes() {
    // Spec: JLS 19 RelationalExpression gives `instanceof` special RHS
    // grammar for both ReferenceType and Pattern; the CST should expose that
    // grammar boundary directly rather than only as a generic binary operator.
    assert_parse_snapshot(
        "instanceof_forms_have_dedicated_expression_nodes",
        r"
            class InstanceofExpressionShape {
                boolean method(Object value) {
                    return value instanceof java.util.List<?> || value instanceof String text;
                }
            }
        ",
    );
}

#[test]
fn parses_contextual_keyword_and_type_expression_ambiguities() {
    // Spec: JLS 3.9 contextual keywords plus JLS 19 ambiguity boundaries:
    // TypeIdentifier, UnqualifiedMethodIdentifier, LocalVariableDeclaration,
    // MethodInvocation, LambdaExpression, CastExpression, and nested generic
    // `>` token splitting in type contexts.
    assert_parse_snapshot(
        "parses_contextual_keyword_and_type_expression_ambiguities",
        r#"
            record Box<T>(T value) {}

            class Ambiguities<T extends java.util.Map<String, java.util.List<Integer>>> {
                Object field;

                void method(Object value) {
                    var local = (String) value;
                    java.util.function.Function<String, String> lambda = (String s) -> s;
                    this.yield();
                    Object access = this.field;
                    java.util.Map<String, java.util.List<Integer>> nested = null;
                    nested.get("key");
                }

                void yield() {}
            }
        "#,
    );
}

#[test]
fn parses_contextual_yield_as_method_invocation() {
    // Spec: JLS 3.9 contextual keywords and JLS 19 MethodInvocation. `yield`
    // remains an ordinary method name outside YieldStatement context.
    assert_parse_snapshot(
        "parses_contextual_yield_as_method_invocation",
        r"
            class ContextualYield {
                void method() {
                    this.yield();
                }

                void yield() {}
            }
        ",
    );
}

#[test]
fn parses_yield_identifier_statement_expressions() {
    // Spec: JLS 3.9 and JLS 19: `yield` is a YieldStatement only in the
    // statement form `yield Expression ;`; otherwise these are ordinary
    // expression statements.
    assert_parse_snapshot(
        "parses_yield_identifier_statement_expressions",
        r"
            class YieldIdentifierStatementExpressions {
                void method() {
                    yield = 1;
                    yield += 2;
                    yield[0] = 3;
                    yield++;
                    yield.foo();
                }
            }
        ",
    );
}

#[test]
fn parses_contextual_keywords_as_identifiers_outside_keyword_contexts() {
    // Spec: JLS 3.9 contextual keywords reduce to identifiers outside their
    // recognized syntactic contexts, including module directive words and
    // `when` outside a switch guard.
    assert_parse_snapshot(
        "parses_contextual_keywords_as_identifiers_outside_keyword_contexts",
        r#"
            class ContextualKeywordIdentifiers {
                int module;
                int open;
                int opens;
                int requires;
                int transitive;
                int exports;
                int to;
                int uses;
                int provides;
                int with;
                int when;
                int permits;
                int sealed;
                int record;
                int var;

                void when() {}
                void requires() {}
                void method() {
                    int when = this.when;
                    record = "name";
                    this.requires();
                }
            }
        "#,
    );
}

#[test]
fn parses_contextual_keyword_adjacency_as_ordinary_tokens() {
    // Spec: JLS 3.9 prevents contextual keyword recognition when the following
    // input character is a JavaLetterOrDigit, including `varfilename` and
    // `non-sealedclass`.
    assert_parse_snapshot(
        "parses_contextual_keyword_adjacency_as_ordinary_tokens",
        r"
            class varfilename {}

            class ContextualKeywordAdjacency {
                void method(int non, int sealedclass) {
                    varfilename value = null;
                    int difference = non-sealedclass;
                }
            }
        ",
    );
}

#[test]
fn recovers_contextual_keyword_missing_space_before_class() {
    // Spec: JLS 3.9 tokenizes `non-sealedclass` as ordinary tokens, so it is
    // not a valid `non-sealed class` declaration without whitespace.
    assert_parse_snapshot(
        "recovers_contextual_keyword_missing_space_before_class",
        r"
            non-sealedclass MissingSpace {}
        ",
    );
}

#[test]
fn parses_nested_generic_type_arguments_closed_by_shift_token() {
    // Spec: JLS 3.5 contextual tokenization and JLS 19 TypeArguments.
    // In type context, adjacent `>` characters from a `>>` token must close
    // nested type argument lists rather than stay a shift operator.
    assert_parse_snapshot(
        "parses_nested_generic_type_arguments_closed_by_shift_token",
        r"
            class NestedGenericClose {
                java.util.Map<String, java.util.List<Integer>> value;
            }
        ",
    );
}

#[test]
fn parses_deeply_nested_generic_type_arguments_closed_by_shift_tokens() {
    // Spec: JLS 3.5 contextual tokenization calls out two, three, and
    // four-or-more adjacent `>` characters in type contexts.
    assert_parse_snapshot(
        "parses_deeply_nested_generic_type_arguments_closed_by_shift_tokens",
        r"
            class DeepNestedGenericClose {
                java.util.Map<String, java.util.Map<Integer, java.util.List<Long>>> triple;
                java.util.Map<String, java.util.Map<Integer, java.util.Map<Long, java.util.List<Double>>>> quadruple;
            }
        ",
    );
}

#[test]
fn parses_class_type_type_arguments_outside_name_nodes() {
    // Regression: ClassType names should not absorb following TypeArgumentList
    // nodes; each generic segment keeps its type arguments as siblings.
    assert_parse_snapshot(
        "parses_class_type_type_arguments_outside_name_nodes",
        r"
            class ClassTypeTypeArgumentsOutsideNameNodes {
                java.util.Map<String>.Entry<Integer> value;
            }
        ",
    );
}

#[test]
fn recovers_malformed_nested_generic_member() {
    // Regression coverage for recovery after a nested generic type closes with
    // adjacent `>` characters but the member declaration is still malformed.
    assert_parse_snapshot(
        "recovers_malformed_nested_generic_member",
        r"
            class MalformedGenericClose {
                java.util.Map<String, java.util.List<Integer>>;
                int after;
            }
        ",
    );
}

#[test]
fn parses_mixed_generic_close_and_relational_greater_than() {
    // Spec: JLS 3.5 contextual tokenization lets adjacent `>` characters close
    // generic types and still participate in the surrounding expression.
    assert_parse_snapshot(
        "parses_mixed_generic_close_and_relational_greater_than",
        r"
            class MixedGenericCloseAndGreaterThan {
                boolean method(Object value, Object limit) {
                    return value instanceof Box<String>> limit;
                }
            }
        ",
    );
}

#[test]
fn parses_mixed_deep_generic_close_and_relational_greater_than() {
    // Spec: JLS 3.5 contextual tokenization lets two adjacent `>` characters
    // close nested type arguments while the next remains relational.
    assert_parse_snapshot(
        "parses_mixed_deep_generic_close_and_relational_greater_than",
        r"
            class MixedDeepGenericCloseAndGreaterThan {
                boolean method(Object value, Object limit) {
                    return value instanceof Box<List<String>>> limit;
                }
            }
        ",
    );
}

#[test]
fn recovers_restricted_type_identifiers() {
    // Spec: JLS 3.8 excludes contextual keywords such as `permits`, `record`,
    // `sealed`, `var`, and `yield` from TypeIdentifier.
    assert_parse_snapshot(
        "recovers_restricted_type_identifiers",
        r"
            class var {}
            class record {}
            class permits {}
            record sealed(int value) {}
            interface yield {}
        ",
    );
}

#[test]
fn diagnoses_literals_and_underscore_used_as_general_identifiers() {
    // Spec: JLS 3.8 Identifier excludes boolean literals, null literal, and
    // `_`; unnamed variables/patterns are handled by dedicated grammar paths.
    assert_parse_snapshot(
        "diagnoses_literals_and_underscore_used_as_general_identifiers",
        r"
            package true;

            class InvalidIdentifiers {
                int false;
                int _;
                true value;
                void null() {}
            }
        ",
    );
}

#[test]
fn recovers_unqualified_yield_method_invocation() {
    // Spec: JLS 3.8 excludes `yield` from UnqualifiedMethodIdentifier; method
    // invocations must qualify it, as covered by `this.yield()`.
    assert_parse_snapshot(
        "recovers_unqualified_yield_method_invocation",
        r"
            class UnqualifiedYield {
                void method() {
                    yield();
                }

                void yield() {}
            }
        ",
    );
}

#[test]
fn diagnoses_decimal_integer_boundary_literals_outside_unary_minus_operand() {
    // Spec: JLS 3.10.1 permits decimal `2147483648` and
    // `9223372036854775808L` only as the direct operand of unary minus.
    assert_parse_snapshot(
        "diagnoses_decimal_integer_boundary_literals_outside_unary_minus_operand",
        r"
            class IntegerLiteralBoundaries {
                void method() {
                    int min = -2_147_483_648;
                    int badInt = 2_147_483_648;
                    int badParenthesizedInt = -(2147483648);
                    long minLong = -9_223_372_036_854_775_808L;
                    long badLong = 9_223_372_036_854_775_808L;
                    long badPlusLong = +9223372036854775808L;
                    switch (badInt) {
                        case -2147483648:
                            break;
                        case 2147483648:
                            break;
                    }
                }
            }
        ",
    );
}

#[test]
fn parses_conditional_expression_with_lambda_third_operand() {
    // Spec: JLS 19 ConditionalExpression permits `? Expression :
    // LambdaExpression`, a lambda-specific ambiguity boundary.
    assert_parse_snapshot(
        "parses_conditional_expression_with_lambda_third_operand",
        r"
            class ConditionalLambda {
                void method(boolean flag) {
                    java.util.function.Function<Integer, Integer> existing = null;
                    java.util.function.Function<Integer, Integer> chosen =
                        flag ? existing : x -> x;
                }
            }
        ",
    );
}

#[test]
fn parses_var_lambda_parameter() {
    // Spec: JLS 19 LambdaParameterType includes `var`.
    assert_parse_snapshot(
        "parses_var_lambda_parameter",
        r"
            class VarLambdaParameter {
                void method() {
                    java.util.function.IntUnaryOperator identity = (var x) -> x;
                }
            }
        ",
    );
}

#[test]
fn parses_parenthesized_concise_lambda_parameter_list() {
    // Spec: JLS 19 LambdaParameterList permits comma-separated
    // ConciseLambdaParameter entries inside parentheses.
    assert_parse_snapshot(
        "parses_parenthesized_concise_lambda_parameter_list",
        r"
            class ConciseLambdaParameterList {
                void method() {
                    java.util.function.BinaryOperator<Integer> add = (x, y) -> x + y;
                }
            }
        ",
    );
}

#[test]
fn parses_variable_arity_lambda_parameter() {
    // Spec: JLS 19 NormalLambdaParameter includes VariableArityParameter.
    assert_parse_snapshot(
        "parses_variable_arity_lambda_parameter",
        r"
            class VarargsLambdaParameter {
                void method() {
                    java.util.function.Function<String[], Integer> lengths =
                        (String... values) -> values.length;
                }
            }
        ",
    );
}

#[test]
fn diagnoses_invalid_lambda_parameters() {
    assert_parse_snapshot(
        "diagnoses_invalid_lambda_parameters",
        r"
            class InvalidLambdaParameters {
                void method() {
                    java.util.function.BiFunction<Integer, Integer, Integer> mixedImplicit =
                        (x, int y) -> y;
                    java.util.function.BiFunction<Integer, Integer, Integer> mixedVar =
                        (var x, y) -> y;
                    java.util.function.BiFunction<String[], String, Integer> trailingVarargs =
                        (String... values, String suffix) -> values.length;
                    java.util.function.Function<Integer, Integer> finalImplicit =
                        (final x) -> x;
                    java.util.function.Function<Integer, Integer> annotatedImplicit =
                        (@Deprecated x) -> x;
                }
            }
        ",
    );
}

#[test]
fn parses_unnamed_lambda_parameter() {
    // Spec: JLS 19 LambdaParameter permits `_` as an unnamed parameter.
    assert_parse_snapshot(
        "parses_unnamed_lambda_parameter",
        r"
            class UnnamedLambdaParameter {
                void method() {
                    java.util.function.IntUnaryOperator zero = _ -> 0;
                }
            }
        ",
    );
}

#[test]
fn parses_cast_expression_whose_operand_is_lambda() {
    // Spec: JLS 19 CastExpression permits casts whose operand is a
    // LambdaExpression.
    assert_parse_snapshot(
        "parses_cast_expression_whose_operand_is_lambda",
        r"
            class CastLambda {
                void method() {
                    Runnable runnable = (Runnable) () -> {};
                }
            }
        ",
    );
}

#[test]
fn disambiguates_parenthesized_expression_from_reference_cast() {
    // Spec: JLS 19 reference CastExpression operands use
    // UnaryExpressionNotPlusMinus, so `(x) - y` is not a cast.
    assert_parse_snapshot(
        "disambiguates_parenthesized_expression_from_reference_cast",
        r"
            class CastAmbiguity {
                void method(int x, int y) {
                    int value = (x) - y;
                }
            }
        ",
    );
}

#[test]
fn parses_primitive_cast_inside_binary_expression() {
    // Spec: JLS 19 primitive CastExpression operand is a UnaryExpression, so
    // `(int) x + 1` is a binary expression whose left operand is `(int) x`.
    assert_parse_snapshot(
        "parses_primitive_cast_inside_binary_expression",
        r"
            class PrimitiveCastBinaryExpression {
                int method(Object x) {
                    return (int) x + 1;
                }
            }
        ",
    );
}

#[test]
fn parses_intersection_cast_expression() {
    // Spec: JLS 19 CastExpression includes intersection reference-type casts
    // using AdditionalBound.
    assert_parse_snapshot(
        "parses_intersection_cast_expression",
        r"
            class IntersectionCast {
                void method(Object value) {
                    Runnable runnable = (Runnable & AutoCloseable) value;
                }
            }
        ",
    );
}

#[test]
fn parses_intersection_cast_whose_operand_is_lambda() {
    // Spec: JLS 19 CastExpression permits an intersection reference-type cast
    // whose operand is a LambdaExpression.
    assert_parse_snapshot(
        "parses_intersection_cast_whose_operand_is_lambda",
        r"
            interface CloseRunnable extends Runnable, AutoCloseable {}

            class IntersectionCastLambda {
                void method() {
                    CloseRunnable runnable = (Runnable & AutoCloseable) () -> {};
                }
            }
        ",
    );
}

fn assert_parse_snapshot(snapshot_name: &str, source: &str) {
    let parse = parse_compilation_unit(source);
    let syntax = parse.syntax().expect("parse should produce syntax");

    assert_eq!(
        syntax.source_text(),
        source,
        "parser reconstruction changed source"
    );
    insta::assert_debug_snapshot!(snapshot_name, &parse);
}
