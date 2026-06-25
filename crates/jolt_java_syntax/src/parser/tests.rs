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

use jolt_syntax::green_text;

use super::parse_compilation_unit;
use crate::{JavaSyntaxKind, JavaSyntaxNode};

#[test]
fn parser_shell_wraps_source_in_compilation_unit() {
    let parse = parse_compilation_unit("package a;\nclass A {}\n");

    assert_eq!(parse.syntax().kind(), JavaSyntaxKind::CompilationUnit);
    assert!(parse.diagnostics().is_empty());
    assert!(parse.lexer_diagnostics().is_empty());
}

#[test]
fn parser_shell_preserves_source_text() {
    let source = "class A {\n  // hello\n}\n";
    let parse = parse_compilation_unit(source);

    assert_eq!(green_text(parse.syntax().green()), source);
}

#[test]
fn parses_ordinary_compilation_unit_package_imports_and_top_level_types() {
    // Spec: JLS 19 CompilationUnit, OrdinaryCompilationUnit, PackageDeclaration,
    // ImportDeclaration, and TopLevelClassOrInterfaceDeclaration.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::CompilationUnit,
            JavaSyntaxKind::PackageDeclaration,
            JavaSyntaxKind::Annotation,
            JavaSyntaxKind::ImportDeclaration,
            JavaSyntaxKind::ClassDeclaration,
            JavaSyntaxKind::InterfaceDeclaration,
        ],
    );
}

#[test]
fn parses_single_module_import_declaration() {
    // Spec: JLS 19 SingleModuleImportDeclaration is `import module ModuleName;`.
    let parse = assert_valid_parse(
        r"
            import module java.base;

            class ModuleImport {}
        ",
    );
    let import_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ImportDeclaration);

    assert!(
        import_texts
            .iter()
            .any(|text| text == "import module java.base;"),
        "expected single module import declaration; actual imports: {import_texts:#?}"
    );
}

#[test]
fn parses_non_module_import_declaration_forms() {
    // Spec: JLS 19 ImportDeclaration includes single type, type-on-demand,
    // single static, and static-on-demand imports.
    let parse = assert_valid_parse(
        r"
            import java.util.List;
            import java.util.*;
            import static java.util.Collections.emptyList;
            import static java.util.Collections.*;

            class Imports {}
        ",
    );
    let import_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ImportDeclaration);

    for expected in [
        "import java.util.List;",
        "import java.util.*;",
        "import static java.util.Collections.emptyList;",
        "import static java.util.Collections.*;",
    ] {
        assert!(
            import_texts.iter().any(|text| text == expected),
            "expected import declaration `{expected}`; actual imports: {import_texts:#?}"
        );
    }
}

#[test]
fn parses_empty_declaration_alternatives() {
    // Spec: JLS 19 permits `;` as an empty declaration at top level and in
    // class, interface, and annotation-interface member positions.
    let parse = assert_valid_parse(
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
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::EmptyDeclaration) >= 4,
        "expected top-level, class, interface, and annotation empty declarations; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::EmptyDeclaration)
            .iter()
            .all(|text| text == ";"),
        "expected each EmptyDeclaration to contain only `;`"
    );
}

#[test]
fn parses_compact_compilation_unit_with_imports_and_methods() {
    // Spec: JLS 19 CompilationUnit and CompactCompilationUnit.
    assert_parse_contains(
        r"
            import java.util.List;

            void main() {
                System.out.println(List.of());
            }
        ",
        &[
            JavaSyntaxKind::CompilationUnit,
            JavaSyntaxKind::ImportDeclaration,
            JavaSyntaxKind::MethodDeclaration,
            JavaSyntaxKind::Block,
            JavaSyntaxKind::MethodInvocationExpression,
            JavaSyntaxKind::ArgumentList,
        ],
    );
}

#[test]
fn parses_modular_compilation_unit_and_module_directives() {
    // Spec: JLS 19 ModularCompilationUnit, ModuleDeclaration, and ModuleDirective.
    let parse = assert_valid_parse(
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
    let module_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ModuleDeclaration);
    let requires_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::RequiresDirective);
    let exports_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ExportsDirective);
    let opens_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::OpensDirective);
    let uses_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::UsesDirective);
    let provides_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ProvidesDirective);

    assert!(
        module_texts
            .iter()
            .any(|text| text.starts_with("@Deprecated open module example.module")),
        "expected annotated open ModuleDeclaration; actual modules: {module_texts:#?}"
    );
    assert!(
        requires_texts
            .iter()
            .any(|text| text == "requires transitive static java.sql;"),
        "expected requires directive with transitive/static modifiers; actual requires: {requires_texts:#?}"
    );
    assert!(
        exports_texts
            .iter()
            .any(|text| text == "exports example.api to friend.module;"),
        "expected exports-to directive; actual exports: {exports_texts:#?}"
    );
    assert!(
        opens_texts
            .iter()
            .any(|text| text == "opens example.internal to friend.module;"),
        "expected opens-to directive; actual opens: {opens_texts:#?}"
    );
    assert!(
        uses_texts
            .iter()
            .any(|text| text == "uses example.Service;"),
        "expected uses directive; actual uses: {uses_texts:#?}"
    );
    assert!(
        provides_texts
            .iter()
            .any(|text| text == "provides example.Service with example.ServiceImpl;"),
        "expected provides-with directive; actual provides: {provides_texts:#?}"
    );
}

#[test]
fn parses_normal_class_declaration_clauses_and_members() {
    // Spec: JLS 19 NormalClassDeclaration, class clauses, class body
    // declarations, fields, methods, constructors, and initializers.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::ClassDeclaration,
            JavaSyntaxKind::ModifierList,
            JavaSyntaxKind::TypeParameterList,
            JavaSyntaxKind::TypeParameter,
            JavaSyntaxKind::TypeBoundList,
            JavaSyntaxKind::ExtendsClause,
            JavaSyntaxKind::ImplementsClause,
            JavaSyntaxKind::PermitsClause,
            JavaSyntaxKind::ClassBody,
            JavaSyntaxKind::ClassBodyDeclaration,
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::VariableDeclaratorList,
            JavaSyntaxKind::VariableDeclarator,
            JavaSyntaxKind::VariableInitializer,
            JavaSyntaxKind::StaticInitializer,
            JavaSyntaxKind::InstanceInitializer,
            JavaSyntaxKind::ConstructorDeclaration,
            JavaSyntaxKind::ConstructorInvocation,
            JavaSyntaxKind::MethodDeclaration,
            JavaSyntaxKind::FormalParameterList,
            JavaSyntaxKind::FormalParameter,
            JavaSyntaxKind::ReceiverParameter,
            JavaSyntaxKind::ThrowsClause,
        ],
    );
}

#[test]
fn parses_sealed_non_sealed_and_permits_contextual_keywords() {
    // Spec: JLS 3.9 and JLS 19 recognize `sealed`, `non-sealed`, and
    // `permits` contextually in class/interface declarations.
    let parse = assert_valid_parse(
        r"
            sealed class SealedClass permits FinalClass, OpenClass {}
            final class FinalClass extends SealedClass {}
            non-sealed class OpenClass extends SealedClass {}

            sealed interface SealedInterface permits OpenInterface {}
            non-sealed interface OpenInterface extends SealedInterface {}
        ",
    );
    let class_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ClassDeclaration);
    let interface_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::InterfaceDeclaration);

    for expected in [
        "sealed class SealedClass permits FinalClass, OpenClass {}",
        "non-sealed class OpenClass extends SealedClass {}",
    ] {
        assert!(
            class_texts.iter().any(|text| text == expected),
            "expected contextual-keyword class declaration `{expected}`; actual classes: {class_texts:#?}"
        );
    }
    for expected in [
        "sealed interface SealedInterface permits OpenInterface {}",
        "non-sealed interface OpenInterface extends SealedInterface {}",
    ] {
        assert!(
            interface_texts.iter().any(|text| text == expected),
            "expected contextual-keyword interface declaration `{expected}`; actual interfaces: {interface_texts:#?}"
        );
    }
}

#[test]
fn parses_context_specific_modifier_productions() {
    // Spec: JLS 19 has context-specific modifier productions for constructors,
    // enum constants, interface constants, and annotation-interface elements.
    let parse = assert_valid_parse(
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
    let enum_constant_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::EnumConstant);
    let constructor_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ConstructorDeclaration);
    let field_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FieldDeclaration);
    let annotation_element_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::AnnotationElement);

    assert!(
        enum_constant_texts
            .iter()
            .any(|text| text == "@Marker VALUE"),
        "expected annotated EnumConstant; actual enum constants: {enum_constant_texts:#?}"
    );
    assert!(
        constructor_texts
            .iter()
            .any(|text| text == "public ModifiedConstructor() {}"),
        "expected public ConstructorDeclaration; actual constructors: {constructor_texts:#?}"
    );
    assert!(
        field_texts
            .iter()
            .any(|text| text == "public static final int X = 1;"),
        "expected modified interface constant FieldDeclaration; actual fields: {field_texts:#?}"
    );
    assert!(
        annotation_element_texts
            .iter()
            .any(|text| text == "@Marker String value();"),
        "expected annotated annotation-interface element; actual elements: {annotation_element_texts:#?}"
    );
}

#[test]
fn parses_trailing_dims_on_method_and_annotation_element_declarators() {
    // Spec: JLS 19 MethodDeclarator and AnnotationInterfaceElementDeclaration
    // permit trailing Dims after the parameter list.
    let parse = assert_valid_parse(
        r"
            class TrailingMethodDims {
                int values()[];
            }

            @interface TrailingAnnotationElementDims {
                int value()[];
            }
        ",
    );
    let method_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::MethodDeclaration);
    let annotation_element_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::AnnotationElement);

    assert!(
        method_texts.iter().any(|text| text == "int values()[];"),
        "expected trailing dims MethodDeclaration; actual methods: {method_texts:#?}"
    );
    assert!(
        annotation_element_texts
            .iter()
            .any(|text| text == "int value()[];"),
        "expected trailing dims AnnotationElement; actual elements: {annotation_element_texts:#?}"
    );
}

#[test]
fn parses_explicit_constructor_invocation_forms() {
    // Spec: JLS 19 ExplicitConstructorInvocation includes `this`, `super`,
    // ExpressionName-qualified `super`, and Primary-qualified `super` forms.
    let parse = assert_valid_parse(
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
    let invocation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ConstructorInvocation);

    for expected in [
        "this(0);",
        "<String>this(value, 0);",
        "super(value);",
        "outer.super(0);",
        "outer.<String>super(value);",
        "(new ConstructorInvocations()).super(0);",
        "(new ConstructorInvocations()).<String>super(value);",
        "<String>super(\"value\");",
    ] {
        assert!(
            invocation_texts.iter().any(|text| text == expected),
            "expected ConstructorInvocation `{expected}`; actual constructor invocations: {invocation_texts:#?}"
        );
    }
}

#[test]
fn parses_enum_declaration_constants_and_body_declarations() {
    // Spec: JLS 19 EnumDeclaration, EnumBody, EnumConstantList,
    // EnumConstant, and EnumBodyDeclarations.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::EnumDeclaration,
            JavaSyntaxKind::EnumBody,
            JavaSyntaxKind::EnumConstantList,
            JavaSyntaxKind::EnumConstant,
            JavaSyntaxKind::ArgumentList,
            JavaSyntaxKind::ClassBody,
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::ConstructorDeclaration,
            JavaSyntaxKind::MethodDeclaration,
        ],
    );
}

#[test]
fn parses_record_declaration_header_body_and_compact_constructor() {
    // Spec: JLS 19 RecordDeclaration, RecordHeader, RecordComponentList,
    // RecordComponent, RecordBody, and CompactConstructorDeclaration.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::RecordDeclaration,
            JavaSyntaxKind::RecordComponentList,
            JavaSyntaxKind::RecordComponent,
            JavaSyntaxKind::Annotation,
            JavaSyntaxKind::ImplementsClause,
            JavaSyntaxKind::RecordBody,
            JavaSyntaxKind::CompactConstructorDeclaration,
            JavaSyntaxKind::MethodDeclaration,
        ],
    );
}

#[test]
fn parses_annotated_record_components() {
    // Spec: JLS 19 RecordComponentModifier permits annotations, and
    // VariableArityRecordComponent permits annotations before `...`.
    let parse = assert_valid_parse(
        r"
            record AnnotatedRecordComponents(@Marker int x, String @Marker ... labels) {}

            @interface Marker {}
        ",
    );
    let component_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::RecordComponent);

    for expected in ["@Marker int x", "String @Marker ... labels"] {
        assert!(
            component_texts.iter().any(|text| text == expected),
            "expected RecordComponent `{expected}`; actual record components: {component_texts:#?}"
        );
    }
}

#[test]
fn parses_interface_and_annotation_interface_declarations() {
    // Spec: JLS 19 InterfaceDeclaration, InterfaceExtends,
    // InterfacePermits, ConstantDeclaration, InterfaceMethodDeclaration,
    // AnnotationInterfaceDeclaration, AnnotationInterfaceElementDeclaration,
    // and DefaultValue.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::InterfaceDeclaration,
            JavaSyntaxKind::InterfaceBody,
            JavaSyntaxKind::ExtendsClause,
            JavaSyntaxKind::PermitsClause,
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::MethodDeclaration,
            JavaSyntaxKind::AnnotationInterfaceDeclaration,
            JavaSyntaxKind::AnnotationInterfaceBody,
            JavaSyntaxKind::AnnotationElementList,
            JavaSyntaxKind::AnnotationElement,
            JavaSyntaxKind::DefaultValue,
            JavaSyntaxKind::AnnotationArrayInitializer,
        ],
    );
}

#[test]
fn parses_annotation_forms_and_element_values() {
    // Spec: JLS 19 Annotation, NormalAnnotation, MarkerAnnotation,
    // SingleElementAnnotation, ElementValuePairList, ElementValuePair,
    // ElementValue, and ElementValueArrayInitializer.
    assert_parse_contains(
        r#"
            @Marker
            @Single("value")
            @Normal(name = "test", nested = @Marker, values = {1, 2, 3})
            class Annotated {}
        "#,
        &[
            JavaSyntaxKind::Annotation,
            JavaSyntaxKind::AnnotationArgumentList,
            JavaSyntaxKind::AnnotationElementList,
            JavaSyntaxKind::AnnotationElement,
            JavaSyntaxKind::AnnotationArrayInitializer,
            JavaSyntaxKind::ClassDeclaration,
        ],
    );
}

#[test]
fn parses_dangling_else_with_nearest_if_binding() {
    // Spec: JLS 19 StatementNoShortIf and IfThenElseStatementNoShortIf split
    // the dangling-else ambiguity so `else` binds to the nearest eligible `if`.
    let parse = assert_valid_parse(
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
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::IfStatement) >= 2,
        "expected nested IfStatement nodes; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        node_texts_of_kind(parse.syntax(), JavaSyntaxKind::IfStatement)
            .iter()
            .any(|text| normalize_whitespace(text) == "if (second) winner(); else loser();"),
        "expected inner if to own the else branch"
    );
}

#[test]
fn parses_type_shapes_and_type_arguments() {
    // Spec: JLS 19 Type, PrimitiveType, ReferenceType,
    // ClassOrInterfaceType, TypeArguments, TypeArgument, Wildcard,
    // WildcardBounds, ArrayType, and Dims.
    assert_parse_contains(
        r"
            class Types<T extends Number & Comparable<T>> {
                int primitive;
                double floating;
                java.util.Map<String, ? extends Number> upper;
                java.util.List<? super T>[] lower;
                T[][] matrix;
            }
        ",
        &[
            JavaSyntaxKind::ClassDeclaration,
            JavaSyntaxKind::TypeParameterList,
            JavaSyntaxKind::TypeBoundList,
            JavaSyntaxKind::PrimitiveType,
            JavaSyntaxKind::ClassType,
            JavaSyntaxKind::TypeArgumentList,
            JavaSyntaxKind::TypeArgument,
            JavaSyntaxKind::WildcardType,
            JavaSyntaxKind::ArrayType,
            JavaSyntaxKind::ArrayDimensions,
            JavaSyntaxKind::Name,
            JavaSyntaxKind::QualifiedName,
        ],
    );
}

#[test]
fn parses_floating_point_type() {
    // Spec: JLS 19 FloatingPointType includes `float` and `double`.
    let parse = assert_valid_parse(
        r"
            class FloatingPointTypes {
                float single;
                double wide;
            }
        ",
    );
    let field_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FieldDeclaration);

    for expected in ["float single;", "double wide;"] {
        assert!(
            field_texts.iter().any(|text| text == expected),
            "expected floating-point field `{expected}`; actual fields: {field_texts:#?}"
        );
    }
}

#[test]
fn parses_qualified_receiver_parameter() {
    // Spec: JLS 19 ReceiverParameter permits `UnannType Identifier . this`.
    let parse = assert_valid_parse(
        r"
            class ReceiverOuter {
                class ReceiverInner {
                    void method(ReceiverOuter ReceiverOuter.this) {}
                }
            }
        ",
    );
    let receiver_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ReceiverParameter);

    assert!(
        receiver_texts
            .iter()
            .any(|text| text == "ReceiverOuter ReceiverOuter.this"),
        "expected qualified ReceiverParameter; actual receivers: {receiver_texts:#?}"
    );
}

#[test]
fn parses_constructor_receiver_parameter() {
    // Spec: JLS 19 ConstructorDeclarator permits a leading ReceiverParameter.
    let parse = assert_valid_parse(
        r"
            class ConstructorReceiverOuter {
                class ConstructorReceiverInner {
                    ConstructorReceiverInner(ConstructorReceiverOuter ConstructorReceiverOuter.this) {}
                }
            }
        ",
    );
    let receiver_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ReceiverParameter);

    assert!(
        receiver_texts
            .iter()
            .any(|text| text == "ConstructorReceiverOuter ConstructorReceiverOuter.this"),
        "expected constructor ReceiverParameter; actual receivers: {receiver_texts:#?}"
    );
}

#[test]
fn parses_annotated_array_dimensions() {
    // Spec: JLS 19 Dims permits annotations before each `[]`.
    let parse = assert_valid_parse(
        r"
            class AnnotatedDims {
                String @Marker [] @Marker [] names;
            }

            @interface Marker {}
        ",
    );
    let dimensions_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ArrayDimensions);

    assert!(
        dimensions_texts
            .iter()
            .any(|text| text == "@Marker [] @Marker []"),
        "expected annotated ArrayDimensions; actual dimensions: {dimensions_texts:#?}"
    );
}

#[test]
fn parses_annotated_type_parameter_modifier() {
    // Spec: JLS 19 TypeParameterModifier permits annotations on type
    // parameters.
    let parse = assert_valid_parse(
        r"
            class AnnotatedTypeParameter<@Marker T> {}

            @interface Marker {}
        ",
    );
    let parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::TypeParameter);

    assert!(
        parameter_texts.iter().any(|text| text == "@Marker T"),
        "expected annotated TypeParameter; actual type parameters: {parameter_texts:#?}"
    );
}

#[test]
fn parses_block_local_declarations_and_statement_forms() {
    // Spec: JLS 19 Block, BlockStatement, LocalClassOrInterfaceDeclaration,
    // LocalVariableDeclarationStatement, LocalVariableDeclaration, EmptyStatement,
    // LabeledStatement, ExpressionStatement, IfThenStatement,
    // IfThenElseStatement, and AssertStatement.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::Block,
            JavaSyntaxKind::BlockStatement,
            JavaSyntaxKind::LocalClassOrInterfaceDeclaration,
            JavaSyntaxKind::LocalVariableDeclaration,
            JavaSyntaxKind::EmptyStatement,
            JavaSyntaxKind::LabeledStatement,
            JavaSyntaxKind::IfStatement,
            JavaSyntaxKind::ExpressionStatement,
            JavaSyntaxKind::AssertStatement,
            JavaSyntaxKind::TypePattern,
        ],
    );
}

#[test]
fn parses_var_local_variable_type() {
    // Spec: JLS 3.9 recognizes `var` contextually as LocalVariableType.
    let parse = assert_valid_parse(
        r"
            class VarLocalVariable {
                void method(Object value) {
                    final var local = value;
                }
            }
        ",
    );
    let local_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LocalVariableDeclaration);

    assert!(
        local_texts
            .iter()
            .any(|text| text == "final var local = value"),
        "expected var LocalVariableDeclaration; actual locals: {local_texts:#?}"
    );
}

#[test]
fn parses_class_instance_creation_statement_expression() {
    // Spec: JLS 19 StatementExpression includes ClassInstanceCreationExpression.
    let parse = assert_valid_parse(
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
    let statement_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ExpressionStatement);
    let creation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression);

    for expected in ["new Object();", "new Runnable() { public void run() {} };"] {
        assert!(
            statement_texts.iter().any(|text| text == expected),
            "expected creation ExpressionStatement `{expected}`; actual statements: {statement_texts:#?}"
        );
    }
    for expected in ["new Object()", "new Runnable() { public void run() {} }"] {
        assert!(
            creation_texts.iter().any(|text| text == expected),
            "expected ObjectCreationExpression `{expected}`; actual creations: {creation_texts:#?}"
        );
    }
}

#[test]
fn parses_switch_statement_rules_groups_labels_and_guards() {
    // Spec: JLS 19 SwitchStatement, SwitchBlock, SwitchRule,
    // SwitchBlockStatementGroup, SwitchLabel, CaseConstant, CasePattern,
    // and Guard.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::SwitchStatement,
            JavaSyntaxKind::SwitchBlock,
            JavaSyntaxKind::SwitchBlockStatementGroup,
            JavaSyntaxKind::SwitchRule,
            JavaSyntaxKind::SwitchLabel,
            JavaSyntaxKind::Guard,
            JavaSyntaxKind::TypePattern,
            JavaSyntaxKind::LiteralExpression,
            JavaSyntaxKind::BreakStatement,
        ],
    );
}

#[test]
fn parses_switch_block_statement_group() {
    // Spec: JLS 19 SwitchBlockStatementGroup preserves colon-form switch
    // labels with the block statements they introduce.
    let parse = assert_valid_parse(
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
    let group_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::SwitchBlockStatementGroup);

    assert!(
        group_texts
            .iter()
            .any(|text| text == "case 1: case 2: value++; break;"),
        "expected colon switch labels and statements in one SwitchBlockStatementGroup; actual groups: {group_texts:#?}"
    );
}

#[test]
fn parses_case_null_default_switch_label() {
    // Spec: JLS 19 SwitchLabel has a special `case null, default` alternative.
    let parse = assert_valid_parse(
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
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        actual_kinds.contains(&JavaSyntaxKind::SwitchExpression),
        "expected SwitchExpression in parse tree; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::SwitchRule),
        "expected SwitchRule in parse tree; actual kinds: {actual_kinds:#?}"
    );
    assert_eq!(
        count_kind(&actual_kinds, JavaSyntaxKind::SwitchLabel),
        1,
        "expected one SwitchLabel for `case null, default`; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        node_texts_of_kind(parse.syntax(), JavaSyntaxKind::SwitchLabel)
            .iter()
            .any(|text| normalize_whitespace(text) == "case null, default"),
        "expected `case null, default` to be preserved as one SwitchLabel"
    );
}

#[test]
fn parses_switch_rule_with_throw_statement() {
    // Spec: JLS 19 SwitchRule permits `SwitchLabel -> ThrowStatement`.
    let parse = assert_valid_parse(
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
    let switch_rule_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::SwitchRule);
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ThrowStatement),
        "expected ThrowStatement in arrow switch rule; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        switch_rule_texts
            .iter()
            .any(|text| text == "case null -> throw new IllegalArgumentException();"),
        "expected switch rule with arrow throw statement; actual switch rules: {switch_rule_texts:#?}"
    );
}

#[test]
fn parses_loop_jump_synchronized_and_try_statements() {
    // Spec: JLS 19 WhileStatement, DoStatement, BasicForStatement,
    // EnhancedForStatement, BreakStatement, ContinueStatement, ReturnStatement,
    // ThrowStatement, SynchronizedStatement, TryStatement, Catches,
    // CatchClause, CatchType, Finally, TryWithResourcesStatement,
    // ResourceSpecification, ResourceList, Resource, and VariableAccess.
    assert_parse_contains(
        r"
            class Flow {
                int method(java.util.List<String> values) throws Exception {
                    while (values.isEmpty()) continue;
                    do { break; } while (false);
                    for (int i = 0; i < 10; i++) {}
                    for (String value : values) {}
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
        &[
            JavaSyntaxKind::WhileStatement,
            JavaSyntaxKind::DoStatement,
            JavaSyntaxKind::ForStatement,
            JavaSyntaxKind::BasicForStatement,
            JavaSyntaxKind::EnhancedForStatement,
            JavaSyntaxKind::ForInitializer,
            JavaSyntaxKind::ForUpdate,
            JavaSyntaxKind::StatementExpressionList,
            JavaSyntaxKind::BreakStatement,
            JavaSyntaxKind::ContinueStatement,
            JavaSyntaxKind::ReturnStatement,
            JavaSyntaxKind::ThrowStatement,
            JavaSyntaxKind::SynchronizedStatement,
            JavaSyntaxKind::TryStatement,
            JavaSyntaxKind::TryWithResourcesStatement,
            JavaSyntaxKind::CatchClause,
            JavaSyntaxKind::CatchTypeList,
            JavaSyntaxKind::FinallyClause,
            JavaSyntaxKind::ResourceList,
            JavaSyntaxKind::Resource,
            JavaSyntaxKind::VariableAccess,
        ],
    );

    let parse = assert_valid_parse(
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
    let actual_kinds = syntax_kinds(parse.syntax());
    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::Resource) >= 2,
        "expected both local-variable and variable-access Resource nodes; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::VariableAccess),
        "expected VariableAccess for bare resource name; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        node_texts_of_kind(parse.syntax(), JavaSyntaxKind::Resource)
            .iter()
            .any(|text| normalize_whitespace(text) == "existing"),
        "expected bare resource name to be preserved as its own Resource"
    );
}

#[test]
fn parses_field_access_resource_variable_access() {
    // Spec: JLS 19 VariableAccess in try-with-resources permits FieldAccess as
    // well as ExpressionName.
    let parse = assert_valid_parse(
        r"
            class FieldAccessResource {
                AutoCloseable existing;

                void method() throws Exception {
                    try (this.existing) {}
                }
            }
        ",
    );
    let resource_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::Resource);
    let access_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::VariableAccess);

    assert!(
        resource_texts.iter().any(|text| text == "this.existing"),
        "expected field-access resource; actual resources: {resource_texts:#?}"
    );
    assert!(
        access_texts.iter().any(|text| text == "this.existing"),
        "expected field-access VariableAccess; actual variable accesses: {access_texts:#?}"
    );
}

#[test]
fn parses_unnamed_variable_declarator_ids() {
    // Spec: JLS 19 VariableDeclaratorId permits `_` as an unnamed variable,
    // including locals, formals, catch parameters, resources, and normal
    // lambda parameters.
    let parse = assert_valid_parse(
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
    let declarator_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::VariableDeclarator);
    let parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FormalParameter);
    let resource_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::Resource);
    let lambda_parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaParameter);

    assert!(
        declarator_texts.iter().any(|text| text == "_ = values"),
        "expected unnamed local VariableDeclarator; actual declarators: {declarator_texts:#?}"
    );
    assert!(
        parameter_texts.iter().any(|text| text == "Object _")
            && parameter_texts.iter().any(|text| text == "Exception _"),
        "expected unnamed formal and catch parameters; actual formal parameters: {parameter_texts:#?}"
    );
    assert!(
        resource_texts
            .iter()
            .any(|text| text == "AutoCloseable _ = open()"),
        "expected unnamed resource variable; actual resources: {resource_texts:#?}"
    );
    assert!(
        lambda_parameter_texts.iter().any(|text| text == "int _"),
        "expected unnamed normal lambda parameter; actual lambda parameters: {lambda_parameter_texts:#?}"
    );
}

#[test]
fn parses_for_statement_expression_lists() {
    // Spec: JLS 19 StatementExpressionList is used by both statement-expression
    // ForInit and ForUpdate alternatives.
    let parse = assert_valid_parse(
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
    let list_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::StatementExpressionList);

    for expected in ["i = 0, j = 0", "i++, j++"] {
        assert!(
            list_texts.iter().any(|text| text == expected),
            "expected StatementExpressionList `{expected}`; actual lists: {list_texts:#?}"
        );
    }
}

#[test]
fn parses_array_initializers_and_array_creation_forms() {
    // Spec: JLS 19 ArrayInitializer, VariableInitializerList,
    // ArrayCreationExpression, DimExprs, DimExpr, and Dims.
    assert_parse_contains(
        r#"
            class Arrays {
                void method(int n) {
                    int[] literal = {1, 2, 3};
                    int[][] sized = new int[n][n];
                    String[] named = new String[] {""};
                }
            }
        "#,
        &[
            JavaSyntaxKind::ArrayInitializer,
            JavaSyntaxKind::ArrayCreationExpression,
            JavaSyntaxKind::DimExpression,
            JavaSyntaxKind::ArrayDimensions,
            JavaSyntaxKind::VariableInitializer,
        ],
    );
}

#[test]
fn parses_annotated_dim_expression() {
    // Spec: JLS 19 DimExpr permits annotations before a sized array dimension.
    let parse = assert_valid_parse(
        r"
            class AnnotatedDimExpression {
                void method(int n) {
                    int[] values = new int @Marker [n];
                }
            }

            @interface Marker {}
        ",
    );
    let dim_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::DimExpression);

    assert!(
        dim_texts.iter().any(|text| text == "@Marker [n]"),
        "expected annotated DimExpression; actual dim expressions: {dim_texts:#?}"
    );
}

#[test]
fn parses_primary_access_invocation_creation_and_references() {
    // Spec: JLS 19 Primary, PrimaryNoNewArray, ClassLiteral,
    // ClassInstanceCreationExpression, ArrayAccess, FieldAccess,
    // MethodInvocation, ArgumentList, and MethodReference.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::ClassLiteralExpression,
            JavaSyntaxKind::ObjectCreationExpression,
            JavaSyntaxKind::ClassBody,
            JavaSyntaxKind::LiteralExpression,
            JavaSyntaxKind::NameExpression,
            JavaSyntaxKind::ParenthesizedExpression,
            JavaSyntaxKind::FieldAccessExpression,
            JavaSyntaxKind::ArrayAccessExpression,
            JavaSyntaxKind::MethodInvocationExpression,
            JavaSyntaxKind::MethodReferenceExpression,
            JavaSyntaxKind::ArgumentList,
            JavaSyntaxKind::ThisExpression,
            JavaSyntaxKind::SuperExpression,
        ],
    );
}

#[test]
fn parses_super_field_access_forms() {
    // Spec: JLS 19 FieldAccess includes `super.Identifier` and
    // `TypeName.super.Identifier` forms in addition to primary field access.
    let parse = assert_valid_parse(
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
    let access_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FieldAccessExpression);

    for expected in ["super.field", "SuperFieldAccess.super.field"] {
        assert!(
            access_texts.iter().any(|text| text == expected),
            "expected FieldAccessExpression `{expected}`; actual field accesses: {access_texts:#?}"
        );
    }
}

#[test]
fn parses_primitive_and_void_class_literals() {
    // Spec: JLS 19 ClassLiteral includes primitive, boolean, and void
    // alternatives in addition to reference types.
    let parse = assert_valid_parse(
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
    let literal_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ClassLiteralExpression);

    for expected in ["int.class", "boolean.class", "void.class"] {
        assert!(
            literal_texts.iter().any(|text| text == expected),
            "expected ClassLiteralExpression `{expected}`; actual class literals: {literal_texts:#?}"
        );
    }
}

#[test]
fn parses_qualified_this_expression() {
    // Spec: JLS 19 PrimaryNoNewArray includes `TypeName . this`.
    let parse = assert_valid_parse(
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
    let this_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ThisExpression);

    assert!(
        this_texts.iter().any(|text| text == "QualifiedThis.this"),
        "expected qualified this expression; actual this expressions: {this_texts:#?}"
    );
}

#[test]
fn parses_array_access_forms() {
    // Spec: JLS 19 ArrayAccess includes ExpressionName, PrimaryNoNewArray, and
    // ArrayCreationExpressionWithInitializer bases.
    let parse = assert_valid_parse(
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
    let access_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ArrayAccessExpression);

    for expected in ["values[0]", "(values)[0]", "new int[] {1}[0]"] {
        assert!(
            access_texts.iter().any(|text| text == expected),
            "expected ArrayAccessExpression `{expected}`; actual array accesses: {access_texts:#?}"
        );
    }
}

#[test]
fn parses_diamond_class_instance_creation() {
    // Spec: JLS 19 TypeArgumentsOrDiamond permits `<>` in class instance
    // creation, independently from explicit type arguments.
    let parse = assert_valid_parse(
        r"
            class DiamondCreation {
                void method() {
                    Object list = new java.util.ArrayList<>();
                }
            }
        ",
    );
    let creation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression);

    assert!(
        creation_texts
            .iter()
            .any(|text| text == "new java.util.ArrayList<>()"),
        "expected diamond ObjectCreationExpression; actual creations: {creation_texts:#?}"
    );
}

#[test]
fn parses_expression_name_qualified_class_instance_creation() {
    // Spec: JLS 19 ClassInstanceCreationExpression includes
    // ExpressionName-qualified `new` forms.
    let parse = assert_valid_parse(
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
    let creation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression);

    assert!(
        creation_texts
            .iter()
            .any(|text| text == "outer.new Inner()"),
        "expected ExpressionName-qualified class instance creation; actual creations: {creation_texts:#?}"
    );
}

#[test]
fn parses_primary_qualified_class_instance_creation() {
    // Spec: JLS 19 ClassInstanceCreationExpression includes Primary-qualified
    // `new` forms.
    let parse = assert_valid_parse(
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
    let creation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression);

    assert!(
        creation_texts
            .iter()
            .any(|text| text == "new Outer().new Inner()"),
        "expected Primary-qualified class instance creation; actual creations: {creation_texts:#?}"
    );
}

#[test]
fn parses_class_instance_creation_with_constructor_type_arguments() {
    // Spec: JLS 19 UnqualifiedClassInstanceCreationExpression permits
    // constructor type arguments immediately after `new`.
    let parse = assert_valid_parse(
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
    let creation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression);

    assert!(
        creation_texts
            .iter()
            .any(|text| text == "new <String> Box(\"value\")"),
        "expected constructor type arguments in ObjectCreationExpression; actual creations: {creation_texts:#?}"
    );
}

#[test]
fn parses_method_reference_forms() {
    // Spec: JLS 19 MethodReference has distinct expression-name, primary,
    // reference-type, super, qualified-super, class-constructor, and
    // array-constructor forms.
    let parse = assert_valid_parse(
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
    let reference_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::MethodReferenceExpression);

    for expected in [
        "value::length",
        "(value)::trim",
        "String::length",
        "super::toString",
        "MethodReferences.super::toString",
        "MethodReferences::new",
        "String[]::new",
        "target::<String>id",
        "this::<String>id",
        "MethodReferences::<String>staticId",
        "super::<String>baseId",
        "MethodReferences.super::<String>baseId",
        "java.util.ArrayList<String>::<String>new",
    ] {
        assert!(
            reference_texts.iter().any(|text| text == expected),
            "expected MethodReferenceExpression `{expected}`; actual method references: {reference_texts:#?}"
        );
    }
}

#[test]
fn parses_method_invocation_forms() {
    // Spec: JLS 19 MethodInvocation has simple-name, type-name,
    // expression-name, primary, super, and TypeName.super forms.
    let parse = assert_valid_parse(
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
    let invocation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::MethodInvocationExpression);

    for expected in [
        "simple()",
        "MethodInvocations.staticMethod()",
        "target.instance()",
        "(target).instance()",
        "super.baseMethod()",
        "MethodInvocations.super.baseMethod()",
        "this.<String>generic(\"value\")",
        "target.<String>generic(\"value\")",
        "MethodInvocations.<String>staticGeneric(\"value\")",
        "super.<String>baseGeneric(\"value\")",
        "MethodInvocations.super.<String>baseGeneric(\"value\")",
    ] {
        assert!(
            invocation_texts.iter().any(|text| text == expected),
            "expected MethodInvocationExpression `{expected}`; actual invocations: {invocation_texts:#?}"
        );
    }
}

#[test]
fn parses_anonymous_class_instance_creation_body() {
    // Spec: JLS 19 UnqualifiedClassInstanceCreationExpression permits an
    // optional ClassBody for anonymous class creation.
    let parse = assert_valid_parse(
        r"
            class AnonymousCreation {
                Runnable runnable = new Runnable() {
                    public void run() {}
                };
            }
        ",
    );
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ObjectCreationExpression),
        "expected ObjectCreationExpression in parse tree; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::ClassBody) >= 2,
        "expected enclosing and anonymous ClassBody nodes; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ObjectCreationExpression)
            .iter()
            .any(|text| normalize_whitespace(text).starts_with("new Runnable() {")),
        "expected anonymous class body inside ObjectCreationExpression"
    );
}

#[test]
fn recovers_invalid_syntax_with_error_nodes_and_diagnostics() {
    // Spec: formatter parser behavior for malformed input. The JLS grammar
    // defines valid syntax; the formatter parser still needs lossless recovery
    // boundaries for invalid syntax.
    let parse = parse_compilation_unit(
        r"
            class Broken {
                void method( {
                    int value = ;
                }
            }
        ",
    );

    assert!(
        parse.lexer_diagnostics().is_empty(),
        "lexer diagnostic(s): {:#?}",
        parse.lexer_diagnostics()
    );
    assert!(
        !parse.diagnostics().is_empty(),
        "expected parser diagnostics for invalid syntax"
    );

    let actual_kinds = syntax_kinds(parse.syntax());
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ErrorNode),
        "expected ErrorNode in parse tree; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn parses_lambda_assignment_conditional_and_operator_expressions() {
    // Spec: JLS 19 LambdaExpression, LambdaParameters, LambdaParameterList,
    // LambdaParameter, AssignmentExpression, Assignment, LeftHandSide,
    // ConditionalExpression, and binary expression precedence productions.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::LambdaExpression,
            JavaSyntaxKind::LambdaParameterList,
            JavaSyntaxKind::LambdaParameter,
            JavaSyntaxKind::AssignmentExpression,
            JavaSyntaxKind::ConditionalExpression,
            JavaSyntaxKind::BinaryExpression,
            JavaSyntaxKind::MethodInvocationExpression,
        ],
    );
}

#[test]
fn parses_compound_assignment_operators() {
    // Spec: JLS 19 AssignmentOperator includes compound assignment operators.
    let parse = assert_valid_parse(
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
    let assignment_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::AssignmentExpression);

    for expected in ["value += 2", "value <<= 1", "value >>>= 1"] {
        assert!(
            assignment_texts.iter().any(|text| text == expected),
            "expected compound AssignmentExpression `{expected}`; actual assignments: {assignment_texts:#?}"
        );
    }
}

#[test]
fn parses_assignment_left_hand_side_forms() {
    // Spec: JLS 19 LeftHandSide permits ExpressionName, FieldAccess, and
    // ArrayAccess assignments.
    let parse = assert_valid_parse(
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
    let assignment_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::AssignmentExpression);

    for expected in ["local = 1", "this.field = 2", "values[0] = 3"] {
        assert!(
            assignment_texts.iter().any(|text| text == expected),
            "expected AssignmentExpression `{expected}`; actual assignments: {assignment_texts:#?}"
        );
    }
}

#[test]
fn parses_logical_and_shift_binary_expression_precedence() {
    // Spec: JLS 19 binary expression precedence productions, including
    // conditional-or, conditional-and, inclusive-or, exclusive-or, and shift.
    let parse = assert_valid_parse(
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
    let binary_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::BinaryExpression);

    for expected in [
        "a < b && b < c || c == a",
        "a << 1",
        "b >> 1",
        "c >>> 1",
        "a | b ^ c & a",
        "a < b && b < c",
        "a < b",
        "b < c",
        "c == a",
        "b ^ c & a",
        "c & a",
        "a + b * c - a / b % c",
        "a + b * c",
        "b * c",
        "a / b % c",
        "a / b",
    ] {
        assert!(
            binary_texts.iter().any(|text| text == expected),
            "expected BinaryExpression `{expected}`; actual binary expressions: {binary_texts:#?}"
        );
    }
}

#[test]
fn parses_unary_postfix_cast_and_switch_expressions() {
    // Spec: JLS 19 UnaryExpression, PreIncrementExpression,
    // PreDecrementExpression, UnaryExpressionNotPlusMinus,
    // PostfixExpression, PostIncrementExpression, PostDecrementExpression,
    // CastExpression, and SwitchExpression.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::UnaryExpression,
            JavaSyntaxKind::PostfixExpression,
            JavaSyntaxKind::CastExpression,
            JavaSyntaxKind::SwitchExpression,
            JavaSyntaxKind::SwitchRule,
            JavaSyntaxKind::YieldStatement,
            JavaSyntaxKind::TypePattern,
        ],
    );
}

#[test]
fn parses_record_type_and_match_all_patterns() {
    // Spec: JLS 19 Pattern, TypePattern, RecordPattern,
    // ComponentPatternList, ComponentPattern, and MatchAllPattern.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::RecordDeclaration,
            JavaSyntaxKind::TypePattern,
            JavaSyntaxKind::RecordPattern,
            JavaSyntaxKind::ComponentPattern,
            JavaSyntaxKind::MatchAllPattern,
            JavaSyntaxKind::SwitchExpression,
            JavaSyntaxKind::Guard,
        ],
    );
}

#[test]
fn parses_instanceof_reference_type_and_pattern_forms() {
    // Spec: JLS 19 RelationalExpression permits both `instanceof ReferenceType`
    // and `instanceof Pattern`.
    let parse = assert_valid_parse(
        r"
            class InstanceofForms {
                boolean method(Object value) {
                    return value instanceof java.util.List<?> || value instanceof String text;
                }
            }
        ",
    );
    let binary_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::BinaryExpression);
    let pattern_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::TypePattern);

    assert!(
        binary_texts
            .iter()
            .any(|text| text == "value instanceof java.util.List<?>"),
        "expected instanceof ReferenceType expression; actual binary expressions: {binary_texts:#?}"
    );
    assert!(
        pattern_texts.iter().any(|text| text == "String text"),
        "expected instanceof TypePattern; actual patterns: {pattern_texts:#?}"
    );
}

#[test]
fn parses_contextual_keyword_and_type_expression_ambiguities() {
    // Spec: JLS 3.9 contextual keywords plus JLS 19 ambiguity boundaries:
    // TypeIdentifier, UnqualifiedMethodIdentifier, LocalVariableDeclaration,
    // MethodInvocation, LambdaExpression, CastExpression, and nested generic
    // `>` token splitting in type contexts.
    assert_parse_contains(
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
        &[
            JavaSyntaxKind::RecordDeclaration,
            JavaSyntaxKind::LocalVariableDeclaration,
            JavaSyntaxKind::CastExpression,
            JavaSyntaxKind::LambdaExpression,
            JavaSyntaxKind::MethodInvocationExpression,
            JavaSyntaxKind::TypeArgumentList,
            JavaSyntaxKind::ClassType,
            JavaSyntaxKind::FieldAccessExpression,
        ],
    );
}

#[test]
fn parses_contextual_yield_as_method_invocation() {
    // Spec: JLS 3.9 contextual keywords and JLS 19 MethodInvocation. `yield`
    // remains an ordinary method name outside YieldStatement context.
    assert_parse_contains(
        r"
            class ContextualYield {
                void method() {
                    this.yield();
                }

                void yield() {}
            }
        ",
        &[JavaSyntaxKind::MethodInvocationExpression],
    );
}

#[test]
fn parses_contextual_keywords_as_identifiers_outside_keyword_contexts() {
    // Spec: JLS 3.9 contextual keywords reduce to identifiers outside their
    // recognized syntactic contexts, including module directive words and
    // `when` outside a switch guard.
    let parse = assert_valid_parse(
        r"
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
                    this.requires();
                }
            }
        ",
    );
    let field_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FieldDeclaration);
    let method_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::MethodDeclaration);
    let local_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LocalVariableDeclaration);
    let field_access_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::FieldAccessExpression);
    let invocation_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::MethodInvocationExpression);

    for expected in [
        "int module;",
        "int open;",
        "int opens;",
        "int requires;",
        "int transitive;",
        "int exports;",
        "int to;",
        "int uses;",
        "int provides;",
        "int with;",
        "int when;",
        "int permits;",
        "int sealed;",
        "int record;",
        "int var;",
    ] {
        assert!(
            field_texts.iter().any(|text| text == expected),
            "expected contextual keyword field `{expected}`; actual fields: {field_texts:#?}"
        );
    }
    for expected in ["void when() {}", "void requires() {}"] {
        assert!(
            method_texts.iter().any(|text| text == expected),
            "expected contextual keyword method `{expected}`; actual methods: {method_texts:#?}"
        );
    }
    assert!(
        local_texts
            .iter()
            .any(|text| text == "int when = this.when"),
        "expected contextual keyword local variable; actual locals: {local_texts:#?}"
    );
    assert!(
        field_access_texts.iter().any(|text| text == "this.when"),
        "expected contextual keyword field access; actual field accesses: {field_access_texts:#?}"
    );
    assert!(
        invocation_texts
            .iter()
            .any(|text| text == "this.requires()"),
        "expected contextual keyword method invocation; actual invocations: {invocation_texts:#?}"
    );
}

#[test]
fn parses_contextual_keyword_adjacency_as_ordinary_tokens() {
    // Spec: JLS 3.9 prevents contextual keyword recognition when the following
    // input character is a JavaLetterOrDigit, including `varfilename` and
    // `non-sealedclass`.
    let parse = assert_valid_parse(
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
    let class_type_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ClassType);
    let binary_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::BinaryExpression);

    assert!(
        class_type_texts.iter().any(|text| text == "varfilename"),
        "expected `varfilename` to remain one type name; actual class types: {class_type_texts:#?}"
    );
    assert!(
        binary_texts.iter().any(|text| text == "non-sealedclass"),
        "expected `non-sealedclass` to parse as identifier-minus-identifier; actual binary expressions: {binary_texts:#?}"
    );
}

#[test]
fn recovers_contextual_keyword_missing_space_before_class() {
    // Spec: JLS 3.9 tokenizes `non-sealedclass` as ordinary tokens, so it is
    // not a valid `non-sealed class` declaration without whitespace.
    let parse = parse_compilation_unit(
        r"
            non-sealedclass MissingSpace {}
        ",
    );

    assert!(
        parse.lexer_diagnostics().is_empty(),
        "lexer diagnostic(s): {:#?}",
        parse.lexer_diagnostics()
    );
    assert!(
        !parse.diagnostics().is_empty(),
        "expected parser diagnostics for missing space after contextual keyword"
    );

    let actual_kinds = syntax_kinds(parse.syntax());
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ErrorNode),
        "expected ErrorNode for missing space after contextual keyword; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn parses_nested_generic_type_arguments_closed_by_shift_token() {
    // Spec: JLS 3.5 contextual tokenization and JLS 19 TypeArguments.
    // In type context, adjacent `>` characters from a `>>` token must close
    // nested type argument lists rather than stay a shift operator.
    let parse = assert_valid_parse(
        r"
            class NestedGenericClose {
                java.util.Map<String, java.util.List<Integer>> value;
            }
        ",
    );
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ClassType),
        "expected ClassType in parse tree; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::TypeArgumentList) >= 2,
        "expected nested TypeArgumentList nodes closed by `>>`; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn parses_deeply_nested_generic_type_arguments_closed_by_shift_tokens() {
    // Spec: JLS 3.5 contextual tokenization calls out two, three, and
    // four-or-more adjacent `>` characters in type contexts.
    let parse = assert_valid_parse(
        r"
            class DeepNestedGenericClose {
                java.util.Map<String, java.util.Map<Integer, java.util.List<Long>>> triple;
                java.util.Map<String, java.util.Map<Integer, java.util.Map<Long, java.util.List<Double>>>> quadruple;
            }
        ",
    );
    let actual_kinds = syntax_kinds(parse.syntax());

    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ClassType),
        "expected ClassType in parse tree; actual kinds: {actual_kinds:#?}"
    );
    assert!(
        count_kind(&actual_kinds, JavaSyntaxKind::TypeArgumentList) >= 7,
        "expected nested TypeArgumentList nodes closed by `>>>` and `>>>>`; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn recovers_restricted_type_identifiers() {
    // Spec: JLS 3.8 excludes contextual keywords such as `permits`, `record`,
    // `sealed`, `var`, and `yield` from TypeIdentifier.
    let parse = parse_compilation_unit(
        r"
            class var {}
            class record {}
            class permits {}
            record sealed(int value) {}
            interface yield {}
        ",
    );

    assert!(
        parse.lexer_diagnostics().is_empty(),
        "lexer diagnostic(s): {:#?}",
        parse.lexer_diagnostics()
    );
    assert!(
        !parse.diagnostics().is_empty(),
        "expected parser diagnostics for restricted TypeIdentifier usage"
    );

    let actual_kinds = syntax_kinds(parse.syntax());
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ErrorNode),
        "expected ErrorNode for restricted TypeIdentifier usage; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn recovers_unqualified_yield_method_invocation() {
    // Spec: JLS 3.8 excludes `yield` from UnqualifiedMethodIdentifier; method
    // invocations must qualify it, as covered by `this.yield()`.
    let parse = parse_compilation_unit(
        r"
            class UnqualifiedYield {
                void method() {
                    yield();
                }

                void yield() {}
            }
        ",
    );

    assert!(
        parse.lexer_diagnostics().is_empty(),
        "lexer diagnostic(s): {:#?}",
        parse.lexer_diagnostics()
    );
    assert!(
        !parse.diagnostics().is_empty(),
        "expected parser diagnostics for unqualified yield method invocation"
    );

    let actual_kinds = syntax_kinds(parse.syntax());
    assert!(
        actual_kinds.contains(&JavaSyntaxKind::ErrorNode),
        "expected ErrorNode for unqualified yield method invocation; actual kinds: {actual_kinds:#?}"
    );
}

#[test]
fn parses_conditional_expression_with_lambda_third_operand() {
    // Spec: JLS 19 ConditionalExpression permits `? Expression :
    // LambdaExpression`, a lambda-specific ambiguity boundary.
    let parse = assert_valid_parse(
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
    let conditional_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::ConditionalExpression);
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);

    assert!(
        conditional_texts
            .iter()
            .any(|text| text == "flag ? existing : x -> x"),
        "expected conditional expression with lambda third operand; actual conditional expressions: {conditional_texts:#?}"
    );
    assert!(
        lambda_texts.iter().any(|text| text == "x -> x"),
        "expected lambda expression in conditional third operand; actual lambda expressions: {lambda_texts:#?}"
    );
}

#[test]
fn parses_var_lambda_parameter() {
    // Spec: JLS 19 LambdaParameterType includes `var`.
    let parse = assert_valid_parse(
        r"
            class VarLambdaParameter {
                void method() {
                    java.util.function.IntUnaryOperator identity = (var x) -> x;
                }
            }
        ",
    );
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);
    let parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaParameter);

    assert!(
        lambda_texts.iter().any(|text| text == "(var x) -> x"),
        "expected var lambda parameter expression; actual lambda expressions: {lambda_texts:#?}"
    );
    assert!(
        parameter_texts.iter().any(|text| text == "var x"),
        "expected `var x` LambdaParameter; actual lambda parameters: {parameter_texts:#?}"
    );
}

#[test]
fn parses_parenthesized_concise_lambda_parameter_list() {
    // Spec: JLS 19 LambdaParameterList permits comma-separated
    // ConciseLambdaParameter entries inside parentheses.
    let parse = assert_valid_parse(
        r"
            class ConciseLambdaParameterList {
                void method() {
                    java.util.function.BinaryOperator<Integer> add = (x, y) -> x + y;
                }
            }
        ",
    );
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);
    let parameter_list_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaParameterList);

    assert!(
        lambda_texts.iter().any(|text| text == "(x, y) -> x + y"),
        "expected parenthesized concise lambda expression; actual lambda expressions: {lambda_texts:#?}"
    );
    assert!(
        parameter_list_texts.iter().any(|text| text == "x, y"),
        "expected concise LambdaParameterList `x, y`; actual lambda parameter lists: {parameter_list_texts:#?}"
    );
}

#[test]
fn parses_variable_arity_lambda_parameter() {
    // Spec: JLS 19 NormalLambdaParameter includes VariableArityParameter.
    let parse = assert_valid_parse(
        r"
            class VarargsLambdaParameter {
                void method() {
                    java.util.function.Function<String[], Integer> lengths =
                        (String... values) -> values.length;
                }
            }
        ",
    );
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);
    let parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaParameter);

    assert!(
        lambda_texts
            .iter()
            .any(|text| text == "(String... values) -> values.length"),
        "expected variable-arity lambda expression; actual lambdas: {lambda_texts:#?}"
    );
    assert!(
        parameter_texts
            .iter()
            .any(|text| text == "String... values"),
        "expected variable-arity LambdaParameter; actual lambda parameters: {parameter_texts:#?}"
    );
}

#[test]
fn parses_unnamed_lambda_parameter() {
    // Spec: JLS 19 LambdaParameter permits `_` as an unnamed parameter.
    let parse = assert_valid_parse(
        r"
            class UnnamedLambdaParameter {
                void method() {
                    java.util.function.IntUnaryOperator zero = _ -> 0;
                }
            }
        ",
    );
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);
    let parameter_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaParameter);

    assert!(
        lambda_texts.iter().any(|text| text == "_ -> 0"),
        "expected unnamed lambda parameter expression; actual lambda expressions: {lambda_texts:#?}"
    );
    assert!(
        parameter_texts.iter().any(|text| text == "_"),
        "expected `_` LambdaParameter; actual lambda parameters: {parameter_texts:#?}"
    );
}

#[test]
fn parses_cast_expression_whose_operand_is_lambda() {
    // Spec: JLS 19 CastExpression permits casts whose operand is a
    // LambdaExpression.
    assert_parse_contains(
        r"
            class CastLambda {
                void method() {
                    Runnable runnable = (Runnable) () -> {};
                }
            }
        ",
        &[
            JavaSyntaxKind::CastExpression,
            JavaSyntaxKind::LambdaExpression,
            JavaSyntaxKind::LambdaParameterList,
            JavaSyntaxKind::Block,
        ],
    );
}

#[test]
fn parses_intersection_cast_expression() {
    // Spec: JLS 19 CastExpression includes intersection reference-type casts
    // using AdditionalBound.
    let parse = assert_valid_parse(
        r"
            class IntersectionCast {
                void method(Object value) {
                    Runnable runnable = (Runnable & AutoCloseable) value;
                }
            }
        ",
    );
    let cast_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::CastExpression);

    assert!(
        cast_texts
            .iter()
            .any(|text| text == "(Runnable & AutoCloseable) value"),
        "expected intersection CastExpression; actual casts: {cast_texts:#?}"
    );
}

#[test]
fn parses_intersection_cast_whose_operand_is_lambda() {
    // Spec: JLS 19 CastExpression permits an intersection reference-type cast
    // whose operand is a LambdaExpression.
    let parse = assert_valid_parse(
        r"
            interface CloseRunnable extends Runnable, AutoCloseable {}

            class IntersectionCastLambda {
                void method() {
                    CloseRunnable runnable = (Runnable & AutoCloseable) () -> {};
                }
            }
        ",
    );
    let cast_texts = normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::CastExpression);
    let lambda_texts =
        normalized_node_texts_of_kind(parse.syntax(), JavaSyntaxKind::LambdaExpression);

    assert!(
        cast_texts
            .iter()
            .any(|text| text == "(Runnable & AutoCloseable) () -> {}"),
        "expected intersection cast with lambda operand; actual casts: {cast_texts:#?}"
    );
    assert!(
        lambda_texts.iter().any(|text| text == "() -> {}"),
        "expected lambda operand in intersection cast; actual lambdas: {lambda_texts:#?}"
    );
}

fn assert_parse_contains(source: &str, expected_kinds: &[JavaSyntaxKind]) {
    let parse = assert_valid_parse(source);
    let actual_kinds = syntax_kinds(parse.syntax());
    for expected in expected_kinds {
        assert!(
            actual_kinds.contains(expected),
            "expected syntax kind {expected:?} in parse tree; actual kinds: {actual_kinds:#?}"
        );
    }
}

fn assert_valid_parse(source: &str) -> super::JavaParse {
    let parse = parse_compilation_unit(source);

    assert!(
        parse.lexer_diagnostics().is_empty(),
        "lexer diagnostic(s): {:#?}",
        parse.lexer_diagnostics()
    );
    assert!(
        parse.diagnostics().is_empty(),
        "parser diagnostic(s): {:#?}",
        parse.diagnostics()
    );

    parse
}

fn syntax_kinds(root: &JavaSyntaxNode) -> Vec<JavaSyntaxKind> {
    let mut kinds = vec![root.kind()];
    kinds.extend(root.descendants().map(|node| node.kind()));
    kinds
}

fn node_texts_of_kind(root: &JavaSyntaxNode, expected: JavaSyntaxKind) -> Vec<String> {
    root.descendants()
        .filter(|node| node.kind() == expected)
        .map(|node| green_text(node.green()))
        .collect()
}

fn normalized_node_texts_of_kind(root: &JavaSyntaxNode, expected: JavaSyntaxKind) -> Vec<String> {
    node_texts_of_kind(root, expected)
        .iter()
        .map(|text| normalize_whitespace(text))
        .collect()
}

fn count_kind(kinds: &[JavaSyntaxKind], expected: JavaSyntaxKind) -> usize {
    kinds.iter().filter(|kind| **kind == expected).count()
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
