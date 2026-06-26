use jolt_syntax::RawSyntaxKind;

use super::*;
use crate::{SyntaxOutcome, parse_compilation_unit};

fn parse_clean(source: &str) -> CompilationUnit {
    let parse = parse_compilation_unit(source);
    let syntax = parse
        .syntax()
        .expect("clean parse should produce syntax")
        .clone();

    assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
    assert!(parse.diagnostics().is_empty());

    syntax
}

fn descendants<N: JavaNode>(syntax: &CompilationUnit) -> Vec<N> {
    syntax.syntax.descendants().filter_map(N::cast).collect()
}

#[test]
fn every_java_node_kind_has_exactly_one_wrapper() {
    let expected = (u16::from(JavaSyntaxKind::ErrorNode)
        ..=u16::from(JavaSyntaxKind::MatchAllPattern))
        .map(|raw| {
            JavaSyntaxKind::from_raw(RawSyntaxKind::new(raw))
                .expect("node-kind range should be valid")
        })
        .collect::<Vec<_>>();

    assert_eq!(ALL_NODE_KINDS, expected.as_slice());

    for kind in expected {
        let casts = node_casts_for_kind(kind, test_syntax(kind));
        assert_eq!(
            casts.len(),
            1,
            "{kind:?} should cast to exactly one wrapper, got {casts:?}"
        );
    }
}

#[test]
fn every_concrete_wrapper_casts_its_declared_kind() {
    assert_node_wrappers_cast_their_declared_kind();
}

#[test]
fn wrappers_and_families_reject_token_kinds() {
    let syntax = test_syntax(JavaSyntaxKind::Identifier);

    assert!(node_casts_for_kind(JavaSyntaxKind::Identifier, syntax.clone()).is_empty());
    assert!(family_casts_for_kind(JavaSyntaxKind::Identifier, syntax).is_empty());
}

#[test]
fn family_enums_cast_exactly_their_declared_variants() {
    for (family, variants) in family_variant_kinds() {
        for &kind in ALL_NODE_KINDS {
            let syntax = test_syntax(kind);
            let casts = family_casts_for_kind(kind, syntax);
            let should_cast = variants.contains(&kind);
            assert_eq!(
                casts.contains(&family),
                should_cast,
                "{family} cast mismatch for {kind:?}; casts={casts:?}"
            );
        }
    }
}

#[test]
fn family_conversions_preserve_variant_kind() {
    assert_family_conversions_compile_and_preserve_kind();
}

#[test]
fn compilation_unit_accessors_traverse_real_parser_output() {
    let parse = parse_compilation_unit(
        r"
                package example.accessors;

                import java.util.List;
                import static java.util.Collections.emptyList;

                class A {}
                interface B {}
            ",
    );
    let syntax = parse.syntax().expect("clean parse should produce syntax");

    assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
    assert!(parse.diagnostics().is_empty());
    assert!(syntax.package_declaration().is_some());
    assert_eq!(syntax.imports().count(), 2);
    assert!(syntax.module_declaration().is_none());

    let type_kinds = syntax
        .type_declarations()
        .map(|declaration| declaration.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        type_kinds,
        [
            JavaSyntaxKind::ClassDeclaration,
            JavaSyntaxKind::InterfaceDeclaration
        ]
    );
}

#[test]
fn module_declaration_directives_traverse_real_parser_output() {
    let parse = parse_compilation_unit(
        r"
                open module example.module {
                    requires transitive static java.sql;
                    exports example.api to friend.module;
                    opens example.internal to friend.module;
                    uses example.Service;
                    provides example.Service with example.ServiceImpl;
                }
            ",
    );
    let syntax = parse.syntax().expect("clean parse should produce syntax");
    let module = syntax
        .module_declaration()
        .expect("module source should expose module declaration");

    assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
    assert!(parse.diagnostics().is_empty());

    let directive_kinds = module
        .directives()
        .map(|directive| directive.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        directive_kinds,
        [
            JavaSyntaxKind::RequiresDirective,
            JavaSyntaxKind::ExportsDirective,
            JavaSyntaxKind::OpensDirective,
            JavaSyntaxKind::UsesDirective,
            JavaSyntaxKind::ProvidesDirective,
        ]
    );
}

#[test]
fn block_accessors_unwrap_parser_block_statement_items() {
    let parse = parse_compilation_unit(
        r"
                class Accessors {
                    void method(Object value) {
                        ;
                        class Local {}
                        var local = value;
                        value.toString();
                        if (value == null) return;
                    }
                }
            ",
    );
    let syntax = parse.syntax().expect("clean parse should produce syntax");
    let block = syntax
        .syntax
        .descendants()
        .find_map(Block::cast)
        .expect("method body should contain a block");

    assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
    assert!(parse.diagnostics().is_empty());

    let item_kinds = block.items().map(|item| item.kind()).collect::<Vec<_>>();
    assert_eq!(
        item_kinds,
        [
            JavaSyntaxKind::EmptyStatement,
            JavaSyntaxKind::LocalClassOrInterfaceDeclaration,
            JavaSyntaxKind::LocalVariableDeclaration,
            JavaSyntaxKind::ExpressionStatement,
            JavaSyntaxKind::IfStatement,
        ]
    );

    let statement_kinds = block
        .statements()
        .map(|statement| statement.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        statement_kinds,
        [
            JavaSyntaxKind::EmptyStatement,
            JavaSyntaxKind::ExpressionStatement,
            JavaSyntaxKind::IfStatement,
        ]
    );
}

#[test]
fn import_declarations_expose_structured_names() {
    let syntax = parse_clean(
        r"
                import java.util.List;
                import java.util.*;
                import static java.util.Collections.emptyList;
                import static java.util.Collections.*;
                import module java.base;

                class Imports {}
            ",
    );

    let names = syntax
        .imports()
        .map(|import| {
            let name = import.name().expect("import should expose its parsed name");
            (name.kind(), name.source_text())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        [
            (JavaSyntaxKind::QualifiedName, "java.util.List".to_owned()),
            (JavaSyntaxKind::QualifiedName, "java.util".to_owned()),
            (
                JavaSyntaxKind::QualifiedName,
                "java.util.Collections.emptyList".to_owned()
            ),
            (
                JavaSyntaxKind::QualifiedName,
                "java.util.Collections".to_owned()
            ),
            (JavaSyntaxKind::QualifiedName, "java.base".to_owned()),
        ]
    );
}

#[test]
fn type_declarations_expose_names_and_bodies() {
    let syntax = parse_clean(
        r"
                class ClassName {}
                record RecordName(int value) {}
                enum EnumName { VALUE }
                interface InterfaceName {}
                @interface AnnotationName {}
            ",
    );

    let declarations = syntax.type_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 5);

    let TypeDeclaration::ClassDeclaration(class) = &declarations[0] else {
        panic!("expected class declaration");
    };
    assert_eq!(class.name().expect("class name").text(), "ClassName");
    assert_eq!(
        class.body().expect("class body").kind(),
        JavaSyntaxKind::ClassBody
    );

    let TypeDeclaration::RecordDeclaration(record) = &declarations[1] else {
        panic!("expected record declaration");
    };
    assert_eq!(record.name().expect("record name").text(), "RecordName");
    assert_eq!(
        record.body().expect("record body").kind(),
        JavaSyntaxKind::RecordBody
    );

    let TypeDeclaration::EnumDeclaration(enum_) = &declarations[2] else {
        panic!("expected enum declaration");
    };
    assert_eq!(enum_.name().expect("enum name").text(), "EnumName");
    assert_eq!(
        enum_.body().expect("enum body").kind(),
        JavaSyntaxKind::EnumBody
    );

    let TypeDeclaration::InterfaceDeclaration(interface) = &declarations[3] else {
        panic!("expected interface declaration");
    };
    assert_eq!(
        interface.name().expect("interface name").text(),
        "InterfaceName"
    );
    assert_eq!(
        interface.body().expect("interface body").kind(),
        JavaSyntaxKind::InterfaceBody
    );

    let TypeDeclaration::AnnotationInterfaceDeclaration(annotation) = &declarations[4] else {
        panic!("expected annotation interface declaration");
    };
    assert_eq!(
        annotation.name().expect("annotation interface name").text(),
        "AnnotationName"
    );
    assert_eq!(
        annotation.body().expect("annotation interface body").kind(),
        JavaSyntaxKind::AnnotationInterfaceBody
    );
}

#[test]
fn method_declarations_expose_names_and_parameter_lists() {
    let syntax = parse_clean(
        r"
                class Methods {
                    String compute(String name, int count) {
                        return name;
                    }

                    void empty() {}
                }
            ",
    );

    let methods = descendants::<MethodDeclaration>(&syntax);
    assert_eq!(methods.len(), 2);

    assert_eq!(methods[0].name().expect("method name").text(), "compute");
    assert_eq!(
        methods[0]
            .parameters()
            .expect("non-empty parameter list")
            .source_text(),
        "String name, int count"
    );

    assert_eq!(methods[1].name().expect("method name").text(), "empty");
    assert!(methods[1].parameters().is_none());
}

#[test]
fn if_statements_expose_condition_then_and_else_children() {
    let syntax = parse_clean(
        r"
                class Branches {
                    void branch(boolean ready) {
                        if (ready && check()) {
                            run();
                        } else if (!ready) {
                            return;
                        }
                    }
                }
            ",
    );

    let ifs = descendants::<IfStatement>(&syntax);
    assert_eq!(ifs.len(), 2);

    assert_eq!(
        ifs[0].condition().expect("outer condition").source_text(),
        "ready && check()"
    );
    assert_eq!(
        ifs[0].then_statement().expect("outer then").kind(),
        JavaSyntaxKind::Block
    );
    assert_eq!(
        ifs[0].else_statement().expect("outer else").kind(),
        JavaSyntaxKind::IfStatement
    );

    assert_eq!(
        ifs[1].condition().expect("inner condition").source_text(),
        "!ready"
    );
    assert_eq!(
        ifs[1].then_statement().expect("inner then").kind(),
        JavaSyntaxKind::Block
    );
    assert!(ifs[1].else_statement().is_none());
}

#[test]
fn method_invocations_expose_argument_lists() {
    let syntax = parse_clean(
        r"
                class Calls {
                    void call(Target target) {
                        target.foo(1, bar()).baz();
                    }
                }
            ",
    );

    let invocations = descendants::<MethodInvocationExpression>(&syntax);
    let foo = invocations
        .iter()
        .find(|invocation| {
            let text = invocation.source_text();
            let text = text.trim();
            text.contains(".foo(") && !text.contains(".baz")
        })
        .expect("expected target.foo invocation");
    let bar = invocations
        .iter()
        .find(|invocation| invocation.source_text().trim() == "bar()")
        .expect("expected bar invocation");
    let baz = invocations
        .iter()
        .find(|invocation| invocation.source_text().trim().ends_with(".baz()"))
        .expect("expected chained baz invocation");

    assert_eq!(
        foo.arguments().expect("foo arguments").source_text(),
        "(1, bar())"
    );
    assert_eq!(bar.arguments().expect("bar arguments").source_text(), "()");
    assert_eq!(baz.arguments().expect("baz arguments").source_text(), "()");
}

#[test]
fn array_types_expose_dimensions() {
    let syntax = parse_clean(
        r"
                class Arrays {
                    java.util.List<String[][]>[] names;
                }
            ",
    );

    let array_types = descendants::<ArrayType>(&syntax);
    let outer_array_type = array_types
        .iter()
        .find(|array_type| array_type.source_text().contains("List"))
        .expect("expected outer array type");
    let inner_array_type = array_types
        .iter()
        .find(|array_type| array_type.source_text().trim() == "String[][]")
        .expect("expected inner array type");

    assert_eq!(
        outer_array_type
            .dimensions()
            .expect("outer array dimensions")
            .source_text()
            .trim(),
        "[]"
    );
    assert_eq!(
        inner_array_type
            .dimensions()
            .expect("inner array dimensions")
            .source_text()
            .trim(),
        "[][]"
    );
}

#[test]
fn annotations_expose_argument_lists() {
    let syntax = parse_clean(
        r#"
                @Anno(value = "x", count = 2)
                @Marker
                class Annotated {}
            "#,
    );

    let annotations = descendants::<Annotation>(&syntax);
    let anno = annotations
        .iter()
        .find(|annotation| annotation.source_text().trim_start().starts_with("@Anno"))
        .expect("expected annotation with arguments");
    let marker = annotations
        .iter()
        .find(|annotation| annotation.source_text().trim() == "@Marker")
        .expect("expected marker annotation");

    assert_eq!(
        anno.arguments()
            .expect("annotation arguments")
            .source_text(),
        r#"(value = "x", count = 2)"#
    );
    assert!(marker.arguments().is_none());
}
