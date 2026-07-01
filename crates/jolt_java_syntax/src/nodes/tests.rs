use jolt_syntax::RawSyntaxKind;
use jolt_text::{TextRange, TextSize};

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

fn semicolon_trailing_comment(semicolon: Option<JavaSyntaxToken>) -> String {
    semicolon
        .expect("semicolon")
        .trailing_comments()
        .first()
        .expect("semicolon trailing comment")
        .text()
        .to_owned()
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
fn token_comments_expose_source_ranges() {
    let syntax = parse_clean("class A {\n  // leading\n  int x; /* trailing */\n}\n");
    let field = descendants::<FieldDeclaration>(&syntax)
        .into_iter()
        .next()
        .expect("field declaration");
    let int_token = field.tokens().into_iter().next().expect("field type token");
    let semicolon = field
        .tokens()
        .into_iter()
        .find(|token| token.kind() == JavaSyntaxKind::Semicolon)
        .expect("field semicolon");

    let leading = int_token.leading_comments();
    assert_eq!(leading[0].text(), "// leading");
    assert_eq!(
        leading[0].text_range(),
        TextRange::new(TextSize::new(12), TextSize::new(22))
    );

    let trailing = semicolon.trailing_comments();
    assert_eq!(trailing[0].text(), "/* trailing */");
    assert_eq!(
        trailing[0].text_range(),
        TextRange::new(TextSize::new(32), TextSize::new(46))
    );
}

#[test]
fn leading_comments_do_not_count_as_blank_lines() {
    let syntax = parse_clean(
        r"
                class A {
                  void run() {
                    first();
                    // keep with second
                    second();

                    // separated from third
                    third();
                  }
                }
            ",
    );
    let statements = descendants::<BlockStatement>(&syntax);
    let second = statements
        .iter()
        .find(|statement| statement.source_text().contains("second"))
        .expect("second statement");
    let third = statements
        .iter()
        .find(|statement| statement.source_text().contains("third"))
        .expect("third statement");

    assert!(!second.starts_after_blank_line());
    assert!(third.starts_after_blank_line());
}

#[test]
fn variable_declarator_lists_expose_entries_with_commas() {
    let syntax = parse_clean("class A { int first /* a */, /* b */ second = 2, third; }\n");
    let list = descendants::<VariableDeclaratorList>(&syntax)
        .into_iter()
        .next()
        .expect("variable declarator list");

    let entries = list.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 3);
    assert_eq!(
        entries[0].declarator.name().expect("first name").text(),
        "first"
    );
    assert_eq!(
        entries[0]
            .declarator
            .name()
            .expect("first name")
            .trailing_comments()[0]
            .text(),
        "/* a */"
    );
    assert!(entries[0].comma.is_some());
    assert_eq!(
        entries[1].declarator.name().expect("second name").text(),
        "second"
    );
    assert_eq!(
        entries[0]
            .comma
            .as_ref()
            .expect("first comma")
            .trailing_comments()[0]
            .text(),
        "/* b */"
    );
    assert!(entries[2].comma.is_none());
}

#[test]
fn compilation_unit_items_expose_ordered_top_level_roles() {
    let syntax = parse_clean(
        r"
                package example.order;

                import java.util.List;

                ;
                class A {}
                ;
                interface B {}
            ",
    );

    let item_kinds = syntax
        .items()
        .map(|item| match item {
            CompilationUnitItem::Package(package) => {
                format!(
                    "package:{}",
                    package.name().expect("package name").compact_text()
                )
            }
            CompilationUnitItem::Import(import) => {
                format!("import:{}", import.import_path().expect("import path"))
            }
            CompilationUnitItem::Module(module) => {
                format!(
                    "module:{}",
                    module.name().expect("module name").compact_text()
                )
            }
            CompilationUnitItem::Type(declaration) => {
                format!("type:{:?}", declaration.kind())
            }
            CompilationUnitItem::EmptyDeclaration(_) => "empty".to_owned(),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        item_kinds,
        [
            "package:example.order".to_owned(),
            "import:java.util.List".to_owned(),
            "empty".to_owned(),
            "type:ClassDeclaration".to_owned(),
            "empty".to_owned(),
            "type:InterfaceDeclaration".to_owned(),
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
    let directives = module.directives().collect::<Vec<_>>();
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
    assert!(module.is_open());
    assert_eq!(
        module.name().expect("module name").source_text().trim(),
        "example.module"
    );

    let ModuleDirective::RequiresDirective(requires) = &directives[0] else {
        panic!("expected requires directive");
    };
    assert!(requires.has_static_modifier());
    assert!(requires.has_transitive_modifier());

    let directive_names = directives
        .iter()
        .map(|directive| {
            directive
                .names()
                .map(|name| name.source_text().trim().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        directive_names,
        [
            vec!["java.sql".to_owned()],
            vec!["example.api".to_owned(), "friend.module".to_owned()],
            vec!["example.internal".to_owned(), "friend.module".to_owned()],
            vec!["example.Service".to_owned()],
            vec![
                "example.Service".to_owned(),
                "example.ServiceImpl".to_owned()
            ],
        ]
    );
}

#[test]
fn module_directives_expose_structured_roles() {
    let syntax = parse_clean(
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
    let module = syntax
        .module_declaration()
        .expect("module source should expose module declaration");
    let directives = module.directives().collect::<Vec<_>>();
    let directive_roles = directives
        .iter()
        .map(|directive| {
            directive
                .directive_role()
                .map(|role| match role {
                    ModuleDirectiveRole::Requires {
                        module,
                        is_static,
                        is_transitive,
                    } => format!(
                        "requires:{}:{is_static}:{is_transitive}",
                        module.compact_text()
                    ),
                    ModuleDirectiveRole::Exports { package, targets } => format!(
                        "exports:{}:{}",
                        package.compact_text(),
                        targets
                            .iter()
                            .map(NameSyntax::compact_text)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    ModuleDirectiveRole::Opens { package, targets } => format!(
                        "opens:{}:{}",
                        package.compact_text(),
                        targets
                            .iter()
                            .map(NameSyntax::compact_text)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    ModuleDirectiveRole::Uses { service } => {
                        format!("uses:{}", service.compact_text())
                    }
                    ModuleDirectiveRole::Provides {
                        service,
                        implementations,
                    } => format!(
                        "provides:{}:{}",
                        service.compact_text(),
                        implementations
                            .iter()
                            .map(NameSyntax::compact_text)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                })
                .expect("module directive role")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        directive_roles,
        [
            "requires:java.sql:true:true".to_owned(),
            "exports:example.api:friend.module".to_owned(),
            "opens:example.internal:friend.module".to_owned(),
            "uses:example.Service".to_owned(),
            "provides:example.Service:example.ServiceImpl".to_owned(),
        ]
    );
}

#[test]
fn module_directive_name_lists_expose_entries_with_commas() {
    let syntax = parse_clean(
        r"
                module example.module {
                    exports example.api to friend.one, // first export
                        friend.two;
                    opens example.internal to friend.three, // first open
                        friend.four;
                    provides example.Service with example.ImplOne, // first impl
                        example.ImplTwo;
                }
            ",
    );
    let module = syntax
        .module_declaration()
        .expect("module source should expose module declaration");
    let directives = module.directives().collect::<Vec<_>>();

    let ModuleDirective::ExportsDirective(exports) = &directives[0] else {
        panic!("expected exports directive");
    };
    let export_targets = exports.target_entries().collect::<Vec<_>>();
    assert_eq!(export_targets.len(), 2);
    assert_eq!(export_targets[0].name.source_text().trim(), "friend.one");
    assert_eq!(
        semicolon_trailing_comment(export_targets[0].comma.clone()),
        "// first export"
    );
    assert_eq!(export_targets[1].name.source_text().trim(), "friend.two");

    let ModuleDirective::OpensDirective(opens) = &directives[1] else {
        panic!("expected opens directive");
    };
    let open_targets = opens.target_entries().collect::<Vec<_>>();
    assert_eq!(open_targets.len(), 2);
    assert_eq!(open_targets[0].name.source_text().trim(), "friend.three");
    assert_eq!(
        semicolon_trailing_comment(open_targets[0].comma.clone()),
        "// first open"
    );
    assert_eq!(open_targets[1].name.source_text().trim(), "friend.four");

    let ModuleDirective::ProvidesDirective(provides) = &directives[2] else {
        panic!("expected provides directive");
    };
    let implementations = provides.implementation_entries().collect::<Vec<_>>();
    assert_eq!(implementations.len(), 2);
    assert_eq!(
        implementations[0].name.source_text().trim(),
        "example.ImplOne"
    );
    assert_eq!(
        semicolon_trailing_comment(implementations[0].comma.clone()),
        "// first impl"
    );
    assert_eq!(
        implementations[1].name.source_text().trim(),
        "example.ImplTwo"
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
fn statement_accessors_expose_terminal_semicolons() {
    let syntax = parse_clean(
        r"
                class Accessors extends Base {
                    Accessors() {
                        super(); // constructor
                        int local = 0; // local
                        local++; // expression
                        assert local > 0; // assert
                        if (local == 0) return; // return
                        if (local == 1) throw problem; // throw
                        label: while (ready) {
                            break label; // break
                            continue label; // continue
                        }
                        do local++; while (ready); // do
                    }

                    int value(int input) {
                        return switch (input) {
                            default -> {
                                yield input; // yield
                            }
                        };
                    }
                }
            ",
    );

    let constructor_invocation = descendants::<ConstructorInvocation>(&syntax)
        .into_iter()
        .next()
        .expect("constructor invocation");
    assert_eq!(
        semicolon_trailing_comment(constructor_invocation.semicolon()),
        "// constructor"
    );

    let local_statement = descendants::<BlockStatement>(&syntax)
        .into_iter()
        .find(|statement| statement.source_text().contains("int local"))
        .expect("local variable block statement");
    assert_eq!(
        semicolon_trailing_comment(local_statement.semicolon()),
        "// local"
    );

    assert_eq!(
        semicolon_trailing_comment(descendants::<ExpressionStatement>(&syntax)[0].semicolon()),
        "// expression"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<AssertStatement>(&syntax)[0].semicolon()),
        "// assert"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<ReturnStatement>(&syntax)[0].semicolon()),
        "// return"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<ThrowStatement>(&syntax)[0].semicolon()),
        "// throw"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<BreakStatement>(&syntax)[0].semicolon()),
        "// break"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<ContinueStatement>(&syntax)[0].semicolon()),
        "// continue"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<DoStatement>(&syntax)[0].semicolon()),
        "// do"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<YieldStatement>(&syntax)[0].semicolon()),
        "// yield"
    );
}

#[test]
fn declaration_accessors_expose_terminal_semicolons() {
    let syntax = parse_clean(
        r"
                class Accessors {
                    String field; // field
                }

                @interface Contract {
                    String value(); // element
                }

                interface Api {
                    void call(); // method
                }
            ",
    );

    assert_eq!(
        semicolon_trailing_comment(descendants::<FieldDeclaration>(&syntax)[0].semicolon()),
        "// field"
    );
    assert_eq!(
        semicolon_trailing_comment(
            descendants::<AnnotationElementDeclaration>(&syntax)[0].semicolon()
        ),
        "// element"
    );
    assert_eq!(
        semicolon_trailing_comment(descendants::<MethodDeclaration>(&syntax)[0].semicolon()),
        "// method"
    );
}

#[test]
fn resource_lists_expose_entries_with_semicolons() {
    let syntax = parse_clean(
        r"
                class Accessors {
                    void run() throws Exception {
                        try (
                            var declared = open(); // declared
                            existing; // optional trailing
                        ) {
                        }
                    }
                }
            ",
    );
    let specification = descendants::<ResourceSpecification>(&syntax)
        .into_iter()
        .next()
        .expect("resource specification");
    let list = specification.list().expect("resource list");
    let entries = list.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert!(entries[0].resource.declaration().is_some());
    assert_eq!(
        semicolon_trailing_comment(entries[0].separator.clone()),
        "// declared"
    );
    assert!(entries[1].resource.variable_access().is_some());
    assert!(entries[1].separator.is_none());
    assert_eq!(
        semicolon_trailing_comment(specification.trailing_semicolon()),
        "// optional trailing"
    );
}

#[test]
fn resource_lists_without_trailing_semicolon_expose_no_optional_separator() {
    let syntax = parse_clean(
        r"
                class Accessors {
                    void run() throws Exception {
                        try (
                            var declared = open();
                            existing
                            // before close
                        ) {
                        }
                    }
                }
            ",
    );
    let specification = descendants::<ResourceSpecification>(&syntax)
        .into_iter()
        .next()
        .expect("resource specification");
    let list = specification.list().expect("resource list");
    let entries = list.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert!(entries[0].resource.declaration().is_some());
    assert!(entries[0].separator.is_some());
    assert!(entries[1].resource.variable_access().is_some());
    assert!(entries[1].separator.is_none());
    assert!(specification.trailing_semicolon().is_none());
    assert_eq!(
        specification
            .close_paren()
            .expect("resource close paren")
            .leading_comments()[0]
            .text(),
        "// before close"
    );
}

#[test]
fn array_initializers_expose_entries_with_commas_and_braces() {
    let syntax = parse_clean(
        r"
                class Accessors {
                    int[] values = {/* start */ 1, // one
                        2, /* two */ 3, // trailing
                    };
                }
            ",
    );
    let initializer = descendants::<ArrayInitializer>(&syntax)
        .into_iter()
        .next()
        .expect("array initializer");
    let entries = initializer.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 3);
    assert_eq!(
        initializer
            .open_brace()
            .expect("initializer open brace")
            .trailing_comments()[0]
            .text(),
        "/* start */"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// one"
    );
    assert_eq!(
        entries[1]
            .comma
            .as_ref()
            .expect("second comma")
            .trailing_comments()[0]
            .text(),
        "/* two */"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[2].comma.clone()),
        "// trailing"
    );
    assert!(initializer.close_brace().is_some());
}

#[test]
fn import_declarations_expose_structured_names() {
    let syntax = parse_clean(
        r"
                import java.util.List;
                import java.util.*;
                import static java.util.Collections.emptyList;
                import static java.util.Collections.*;
                import module.foo.Bar;
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
            (JavaSyntaxKind::QualifiedName, "module.foo.Bar".to_owned()),
            (JavaSyntaxKind::QualifiedName, "java.base".to_owned()),
        ]
    );

    let import_roles = syntax
        .imports()
        .map(|import| {
            (
                import.is_static(),
                import.is_star(),
                import.is_module(),
                import.import_path().expect("import path"),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        import_roles,
        [
            (false, false, false, "java.util.List".to_owned()),
            (false, true, false, "java.util.*".to_owned()),
            (
                true,
                false,
                false,
                "java.util.Collections.emptyList".to_owned()
            ),
            (true, true, false, "java.util.Collections.*".to_owned()),
            (false, false, false, "module.foo.Bar".to_owned()),
            (false, false, true, "java.base".to_owned()),
        ]
    );

    let import_kinds = syntax
        .imports()
        .map(|import| {
            import
                .import_kind()
                .map(|kind| match kind {
                    ImportKind::SingleType(name) => ("single-type", name.compact_text()),
                    ImportKind::TypeOnDemand(name) => ("type-on-demand", name.compact_text()),
                    ImportKind::SingleStatic(name) => ("single-static", name.compact_text()),
                    ImportKind::StaticOnDemand(name) => ("static-on-demand", name.compact_text()),
                    ImportKind::SingleModule(name) => ("single-module", name.compact_text()),
                })
                .expect("import kind")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        import_kinds,
        [
            ("single-type", "java.util.List".to_owned()),
            ("type-on-demand", "java.util".to_owned()),
            (
                "single-static",
                "java.util.Collections.emptyList".to_owned()
            ),
            ("static-on-demand", "java.util.Collections".to_owned()),
            ("single-type", "module.foo.Bar".to_owned()),
            ("single-module", "java.base".to_owned()),
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
#[allow(clippy::too_many_lines)]
fn statement_body_accessors_expose_body_kind() {
    let syntax = parse_clean(
        r"
                class Bodies {
                    void body(boolean ready) {
                        if (ready) {
                            run();
                        } else;
                        while (ready);
                        do run(); while (ready);
                        for (;;);
                        for (String name : names()) run(name);
                    }
                }
            ",
    );

    let if_statement = descendants::<IfStatement>(&syntax)
        .into_iter()
        .next()
        .expect("if statement");
    assert_eq!(
        if_statement.keyword().expect("if keyword").kind(),
        JavaSyntaxKind::IfKw
    );
    assert_eq!(
        if_statement.else_keyword().expect("else keyword").kind(),
        JavaSyntaxKind::ElseKw
    );
    assert!(matches!(
        if_statement.then_body(),
        Some(StatementBody::Block(_))
    ));
    let Some(StatementBody::Block(then_block)) = if_statement.then_body() else {
        panic!("expected then block");
    };
    assert_eq!(
        then_block.open_brace().expect("block open brace").kind(),
        JavaSyntaxKind::LBrace
    );
    assert_eq!(
        then_block.close_brace().expect("block close brace").kind(),
        JavaSyntaxKind::RBrace
    );
    assert!(matches!(
        if_statement.else_body(),
        Some(StatementBody::Empty(_))
    ));

    let while_statement = descendants::<WhileStatement>(&syntax)
        .into_iter()
        .next()
        .expect("while statement");
    assert_eq!(
        while_statement.keyword().expect("while keyword").kind(),
        JavaSyntaxKind::WhileKw
    );
    assert!(matches!(
        while_statement.statement_body(),
        Some(StatementBody::Empty(_))
    ));

    let do_statement = descendants::<DoStatement>(&syntax)
        .into_iter()
        .next()
        .expect("do statement");
    assert_eq!(
        do_statement.keyword().expect("do keyword").kind(),
        JavaSyntaxKind::DoKw
    );
    assert_eq!(
        do_statement
            .while_keyword()
            .expect("do-while keyword")
            .kind(),
        JavaSyntaxKind::WhileKw
    );
    assert!(matches!(
        do_statement.statement_body(),
        Some(StatementBody::Unbraced(Statement::ExpressionStatement(_)))
    ));

    let basic_for = descendants::<ForStatement>(&syntax)
        .into_iter()
        .filter_map(|for_statement| for_statement.basic())
        .find(|basic| {
            basic.initializer().is_none() && basic.condition().is_none() && basic.update().is_none()
        })
        .expect("basic for statement");
    assert_eq!(
        basic_for.keyword().expect("basic for keyword").kind(),
        JavaSyntaxKind::ForKw
    );
    assert!(matches!(
        basic_for.statement_body(),
        Some(StatementBody::Empty(_))
    ));

    let enhanced_for = descendants::<ForStatement>(&syntax)
        .into_iter()
        .find_map(|for_statement| for_statement.enhanced())
        .expect("enhanced for statement");
    assert_eq!(
        enhanced_for.keyword().expect("enhanced for keyword").kind(),
        JavaSyntaxKind::ForKw
    );
    assert!(matches!(
        enhanced_for.statement_body(),
        Some(StatementBody::Unbraced(Statement::ExpressionStatement(_)))
    ));
}

#[test]
fn switch_label_case_entries_expose_commas() {
    let syntax = parse_clean(
        r"
                class Example {
                    int classify(Object value) {
                        return switch (value) {
                            case null, // null arm
                                default -> 0;
                            case String s -> 1;
                        };
                    }
                }
            ",
    );
    let label = descendants::<SwitchLabel>(&syntax)
        .into_iter()
        .find(|label| label.source_text().contains("default"))
        .expect("case null, default label");

    let entries = label.case_entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        match &entries[0].item {
            SwitchLabelCaseItem::Constant(constant) => constant.source_text().trim().to_owned(),
            _ => panic!("expected null constant"),
        },
        "null"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// null arm"
    );
    assert!(matches!(entries[1].item, SwitchLabelCaseItem::Default(_)));
    assert!(entries[1].comma.is_none());
}

#[test]
fn wildcard_and_unnamed_accessors_expose_roles() {
    let syntax = parse_clean(
        r"
                record Pair(Object left, Object right) {
                }

                class Roles {
                    AutoCloseable open() {
                        return null;
                    }

                    void method(Object _, Object value) throws Exception {
                        var _ = value;
                        java.util.List<? extends Number> upper = null;
                        java.util.List<? super Integer> lower = null;
                        try (AutoCloseable _ = open()) {
                        } catch (Exception _) {
                        }
                        java.util.function.IntUnaryOperator zero = (int _) -> 0;
                        if (value instanceof Pair(Object left, _)) {
                        }
                    }
                }
            ",
    );

    let unnamed_formal = descendants::<FormalParameter>(&syntax)
        .into_iter()
        .find(|parameter| parameter.source_text().trim() == "Object _")
        .expect("unnamed formal parameter");
    assert_eq!(unnamed_formal.name().expect("formal name").text(), "_");
    assert!(unnamed_formal.is_unnamed());

    let unnamed_declarators = descendants::<VariableDeclarator>(&syntax)
        .into_iter()
        .filter(VariableDeclarator::is_unnamed)
        .collect::<Vec<_>>();
    assert_eq!(unnamed_declarators.len(), 2);
    assert!(
        unnamed_declarators.iter().all(|declarator| declarator
            .name()
            .expect("declarator name")
            .text()
            == "_")
    );

    let catch_parameter = descendants::<CatchParameter>(&syntax)
        .into_iter()
        .next()
        .expect("catch parameter");
    assert_eq!(catch_parameter.name().expect("catch name").text(), "_");
    assert!(catch_parameter.is_unnamed());

    let lambda_parameter = descendants::<LambdaParameter>(&syntax)
        .into_iter()
        .next()
        .expect("lambda parameter");
    assert_eq!(lambda_parameter.name().expect("lambda name").text(), "_");
    assert!(lambda_parameter.is_unnamed());

    let wildcard_bounds = descendants::<WildcardType>(&syntax)
        .into_iter()
        .filter_map(|wildcard| wildcard.bound_clause())
        .collect::<Vec<_>>();
    assert!(matches!(
        wildcard_bounds.as_slice(),
        [
            WildcardBound::Extends(extends_bound),
            WildcardBound::Super(super_bound),
        ] if extends_bound.source_text().trim() == "Number"
            && super_bound.source_text().trim() == "Integer"
    ));

    let match_all = descendants::<MatchAllPattern>(&syntax)
        .into_iter()
        .next()
        .expect("match-all pattern");
    assert_eq!(match_all.underscore().expect("match-all token").text(), "_");
    assert!(match_all.is_unnamed());
}

#[test]
fn record_patterns_expose_component_entries_with_commas_and_parens() {
    let syntax = parse_clean(
        r"
                record Pair(int left, int right) {
                }

                class Patterns {
                    int read(Object value) {
                        return switch (value) {
                            case Pair(/* open */ int left, // left
                                _) -> left;
                            default -> 0;
                        };
                    }
                }
            ",
    );
    let pattern = descendants::<RecordPattern>(&syntax)
        .into_iter()
        .next()
        .expect("record pattern");

    assert_eq!(
        pattern
            .open_paren()
            .expect("pattern open paren")
            .trailing_comments()[0]
            .text(),
        "/* open */"
    );
    assert_eq!(
        pattern.close_paren().expect("pattern close paren").text(),
        ")"
    );

    let entries = pattern.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].component.source_text().trim(), "int left");
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// left"
    );
    assert_eq!(entries[1].component.source_text().trim(), "_");
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

    let foo_arguments = foo.arguments().expect("foo arguments");
    assert_eq!(foo_arguments.source_text(), "(1, bar())");
    assert_eq!(
        foo_arguments.open_paren().expect("foo open paren").kind(),
        JavaSyntaxKind::LParen
    );
    assert_eq!(
        foo_arguments.close_paren().expect("foo close paren").kind(),
        JavaSyntaxKind::RParen
    );
    let entries = foo_arguments.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].argument.source_text(), "1");
    assert_eq!(
        entries[0].comma.as_ref().expect("first comma").kind(),
        JavaSyntaxKind::Comma
    );
    assert_eq!(entries[1].argument.source_text(), "bar()");
    assert!(entries[1].comma.is_none());
    assert_eq!(bar.arguments().expect("bar arguments").source_text(), "()");
    assert_eq!(baz.arguments().expect("baz arguments").source_text(), "()");
}

#[test]
fn declaration_parameter_lists_expose_entries_with_commas_and_parens() {
    let syntax = parse_clean(
        r"
                record Person(/* components */ String name, // name
                    int age
                    // before component close
                ) {
                    void run(/* params */ String name, // name
                        int count
                        // before parameter close
                    ) {
                    }
                }
            ",
    );

    let components = descendants::<RecordComponentList>(&syntax)
        .into_iter()
        .next()
        .expect("record component list");
    let component_entries = components.entries().collect::<Vec<_>>();
    assert_eq!(component_entries.len(), 2);
    assert_eq!(
        components
            .open_paren()
            .expect("component open paren")
            .trailing_comments()[0]
            .text(),
        "/* components */"
    );
    assert_eq!(
        semicolon_trailing_comment(component_entries[0].comma.clone()),
        "// name"
    );
    assert_eq!(
        components
            .close_paren()
            .expect("component close paren")
            .leading_comments()[0]
            .text(),
        "// before component close"
    );

    let parameters = descendants::<FormalParameterList>(&syntax)
        .into_iter()
        .next()
        .expect("formal parameter list");
    let parameter_entries = parameters.entries().collect::<Vec<_>>();
    assert_eq!(parameter_entries.len(), 2);
    assert_eq!(
        parameters
            .open_paren()
            .expect("parameter open paren")
            .trailing_comments()[0]
            .text(),
        "/* params */"
    );
    assert_eq!(
        semicolon_trailing_comment(parameter_entries[0].comma.clone()),
        "// name"
    );
    assert_eq!(
        parameters
            .close_paren()
            .expect("parameter close paren")
            .leading_comments()[0]
            .text(),
        "// before parameter close"
    );
}

#[test]
fn throws_clauses_expose_entries_with_commas_and_keyword() {
    let syntax = parse_clean(
        r"
                class Accessors {
                    void run() throws /* checked */ IOException, // io
                        TimeoutException {
                    }
                }
            ",
    );
    let throws = descendants::<ThrowsClause>(&syntax)
        .into_iter()
        .next()
        .expect("throws clause");
    let entries = throws.entries().collect::<Vec<_>>();

    assert_eq!(
        throws
            .keyword()
            .expect("throws keyword")
            .trailing_comments()[0]
            .text(),
        "/* checked */"
    );
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].exception.source_text().trim(), "IOException");
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// io"
    );
    assert_eq!(
        entries[1].exception.source_text().trim(),
        "TimeoutException"
    );
    assert!(entries[1].comma.is_none());
}

#[test]
fn catch_type_lists_expose_union_entries() {
    let syntax = parse_clean(
        r"
                class Catches {
                    void run() {
                        try {
                            work();
                        } catch (java.io.IOException | RuntimeException ex) {
                            recover(ex);
                        }
                    }
                }
            ",
    );

    let parameter = descendants::<CatchParameter>(&syntax)
        .into_iter()
        .next()
        .expect("catch parameter");
    let types = parameter.types().expect("catch types");
    let entries = types.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].ty.source_text().trim(), "java.io.IOException");
    assert_eq!(
        entries[0].separator.as_ref().expect("pipe").kind(),
        JavaSyntaxKind::Bar
    );
    assert_eq!(entries[1].ty.source_text().trim(), "RuntimeException");
    assert!(entries[1].separator.is_none());
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
fn type_parameter_and_argument_lists_expose_entries_with_commas_and_angles() {
    let syntax = parse_clean(
        r"
                class Accessors</* params */ T, // type
                    U
                    // before parameter close
                > {
                    java.util.Map</* args */ String, // key
                        ? super U
                        // before argument close
                    > values;
                }
            ",
    );

    let parameters = descendants::<TypeParameterList>(&syntax)
        .into_iter()
        .next()
        .expect("type parameter list");
    let parameter_entries = parameters.entries().collect::<Vec<_>>();
    assert_eq!(parameter_entries.len(), 2);
    assert_eq!(
        parameters
            .open_angle()
            .expect("parameter open angle")
            .trailing_comments()[0]
            .text(),
        "/* params */"
    );
    assert_eq!(
        semicolon_trailing_comment(parameter_entries[0].comma.clone()),
        "// type"
    );
    assert_eq!(
        parameters
            .close_angle()
            .expect("parameter close angle")
            .leading_comments()[0]
            .text(),
        "// before parameter close"
    );

    let arguments = descendants::<TypeArgumentList>(&syntax)
        .into_iter()
        .find(|arguments| arguments.source_text().contains("String"))
        .expect("type argument list");
    let argument_entries = arguments.entries().collect::<Vec<_>>();
    assert_eq!(argument_entries.len(), 2);
    assert_eq!(
        arguments
            .open_angle()
            .expect("argument open angle")
            .trailing_comments()[0]
            .text(),
        "/* args */"
    );
    assert_eq!(
        semicolon_trailing_comment(argument_entries[0].comma.clone()),
        "// key"
    );
    assert_eq!(
        arguments
            .close_angle()
            .expect("argument close angle")
            .leading_comments()[0]
            .text(),
        "// before argument close"
    );
}

#[test]
fn type_intersections_expose_entries_with_ampersands() {
    let syntax = parse_clean(
        r"
                class Accessors<T extends Number & // numeric
                    Comparable<T>> {
                    Object value(Object value) {
                        return (Runnable & // runnable
                            AutoCloseable) value;
                    }
                }
            ",
    );

    let parameter = descendants::<TypeParameter>(&syntax)
        .into_iter()
        .next()
        .expect("type parameter");
    let bounds = parameter.bounds().expect("type bounds");
    let bound_entries = bounds.entries().collect::<Vec<_>>();
    assert_eq!(bound_entries.len(), 2);
    assert_eq!(bound_entries[0].ty.source_text().trim(), "Number");
    assert_eq!(
        semicolon_trailing_comment(bound_entries[0].separator.clone()),
        "// numeric"
    );
    assert_eq!(bound_entries[1].ty.source_text().trim(), "Comparable<T>");

    let intersection = descendants::<IntersectionType>(&syntax)
        .into_iter()
        .find(|intersection| intersection.source_text().contains("Runnable"))
        .expect("intersection cast type");
    let intersection_entries = intersection.entries().collect::<Vec<_>>();
    assert_eq!(intersection_entries.len(), 2);
    assert_eq!(intersection_entries[0].ty.source_text().trim(), "Runnable");
    assert_eq!(
        semicolon_trailing_comment(intersection_entries[0].separator.clone()),
        "// runnable"
    );
    assert_eq!(
        intersection_entries[1].ty.source_text().trim(),
        "AutoCloseable"
    );
}

#[test]
fn type_header_clauses_expose_entries_with_commas_and_keywords() {
    let syntax = parse_clean(
        r"
                class Derived extends /* base */ Base implements First, // first
                    Second permits Child, // child
                    Other {
                }
            ",
    );
    let class = descendants::<ClassDeclaration>(&syntax)
        .into_iter()
        .next()
        .expect("class declaration");

    let extends = class.extends_clause().expect("extends clause");
    assert_eq!(
        extends
            .keyword()
            .expect("extends keyword")
            .trailing_comments()[0]
            .text(),
        "/* base */"
    );
    let extends_entries = extends.entries().collect::<Vec<_>>();
    assert_eq!(extends_entries.len(), 1);
    assert_eq!(extends_entries[0].ty.source_text().trim(), "Base");

    let implements = class.implements_clause().expect("implements clause");
    assert_eq!(
        implements.keyword().expect("implements keyword").text(),
        "implements"
    );
    let implements_entries = implements.entries().collect::<Vec<_>>();
    assert_eq!(implements_entries.len(), 2);
    assert_eq!(implements_entries[0].ty.source_text().trim(), "First");
    assert_eq!(
        semicolon_trailing_comment(implements_entries[0].comma.clone()),
        "// first"
    );
    assert_eq!(implements_entries[1].ty.source_text().trim(), "Second");

    let permits = class.permits_clause().expect("permits clause");
    assert_eq!(
        permits.keyword().expect("permits keyword").text(),
        "permits"
    );
    let permits_entries = permits.entries().collect::<Vec<_>>();
    assert_eq!(permits_entries.len(), 2);
    assert_eq!(permits_entries[0].name.source_text().trim(), "Child");
    assert_eq!(
        semicolon_trailing_comment(permits_entries[0].comma.clone()),
        "// child"
    );
    assert_eq!(permits_entries[1].name.source_text().trim(), "Other");
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

#[test]
fn annotation_argument_lists_expose_entries_with_commas_and_parens() {
    let syntax = parse_clean(
        r#"
                @Anno(/* start */ value = "x", // value
                    count = 2
                    // before close
                )
                class Annotated {}
            "#,
    );
    let arguments = descendants::<AnnotationArgumentList>(&syntax)
        .into_iter()
        .next()
        .expect("annotation argument list");
    let entries = arguments.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert_eq!(
        arguments
            .open_paren()
            .expect("annotation open paren")
            .trailing_comments()[0]
            .text(),
        "/* start */"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// value"
    );
    assert!(entries[1].comma.is_none());
    assert_eq!(
        arguments
            .close_paren()
            .expect("annotation close paren")
            .leading_comments()[0]
            .text(),
        "// before close"
    );
}

#[test]
fn annotation_array_initializers_expose_entries_with_commas_and_braces() {
    let syntax = parse_clean(
        r"
                @interface Accessors {
                    int[] values() default {/* start */ 1, // one
                        2, /* two */ 3, // trailing
                    };
                }
            ",
    );
    let initializer = descendants::<AnnotationArrayInitializer>(&syntax)
        .into_iter()
        .next()
        .expect("annotation array initializer");
    let entries = initializer.entries().collect::<Vec<_>>();

    assert_eq!(entries.len(), 3);
    assert_eq!(
        initializer
            .open_brace()
            .expect("annotation initializer open brace")
            .trailing_comments()[0]
            .text(),
        "/* start */"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[0].comma.clone()),
        "// one"
    );
    assert_eq!(
        entries[1]
            .comma
            .as_ref()
            .expect("second comma")
            .trailing_comments()[0]
            .text(),
        "/* two */"
    );
    assert_eq!(
        semicolon_trailing_comment(entries[2].comma.clone()),
        "// trailing"
    );
    assert!(initializer.close_brace().is_some());
}

#[test]
#[allow(clippy::too_many_lines)]
fn declaration_accessors_expose_formatter_facing_structure() {
    let syntax = parse_clean(
        r#"
                @Deprecated
                public class Accessors<T> extends Base implements Runnable permits Other {
                    ;
                    static {}
                    {}
                    int first = 1, second[];
                    Accessors(String name) throws Exception {}
                    String method(int count) throws Exception { return ""; }
                    class Nested {}
                }

                record Data(int value, String... names) implements Runnable {
                    Data {}
                }

                enum Choice {
                    ONE(1) { void hook() {} },
                    TWO;

                    int code;
                }
            "#,
    );

    let class = syntax
        .type_declarations()
        .find_map(|declaration| match declaration {
            TypeDeclaration::ClassDeclaration(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");

    let modifiers = class.modifiers().expect("class modifiers");
    assert_eq!(modifiers.annotations().count(), 1);
    assert_eq!(
        modifiers
            .modifier_tokens()
            .map(|token| token.kind())
            .collect::<Vec<_>>(),
        [JavaSyntaxKind::PublicKw]
    );
    assert_eq!(class.name().expect("class name").text(), "Accessors");
    assert_eq!(
        class
            .type_parameters()
            .expect("type parameters")
            .parameters()
            .count(),
        1
    );
    assert!(class.extends_clause().is_some());
    assert!(class.implements_clause().is_some());
    assert!(class.permits_clause().is_some());

    let class_body = class.body().expect("class body");
    let member_kinds = class_body
        .members()
        .map(|member| member.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        member_kinds,
        [
            JavaSyntaxKind::EmptyDeclaration,
            JavaSyntaxKind::StaticInitializer,
            JavaSyntaxKind::InstanceInitializer,
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::ConstructorDeclaration,
            JavaSyntaxKind::MethodDeclaration,
            JavaSyntaxKind::ClassDeclaration,
        ]
    );

    let field = descendants::<FieldDeclaration>(&syntax)
        .into_iter()
        .find(|field| field.source_text().contains("first"))
        .expect("field declaration");
    let declarators = field.declarators().expect("field declarators");
    let declarator_names = declarators
        .declarators()
        .map(|declarator| {
            declarator
                .name()
                .expect("declarator name")
                .text()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(declarator_names, ["first", "second"]);

    let constructor = descendants::<ConstructorDeclaration>(&syntax)
        .into_iter()
        .find(|constructor| {
            constructor
                .name()
                .is_some_and(|name| name.text() == "Accessors")
        })
        .expect("constructor declaration");
    assert_eq!(
        constructor
            .parameters()
            .expect("parameters")
            .parameters()
            .count(),
        1
    );
    assert!(constructor.throws_clause().is_some());
    assert_eq!(
        constructor
            .throws_clause()
            .expect("constructor throws")
            .exceptions()
            .map(|exception| exception.source_text().trim().to_owned())
            .collect::<Vec<_>>(),
        ["Exception"]
    );
    assert!(constructor.body().is_some());

    let method = descendants::<MethodDeclaration>(&syntax)
        .into_iter()
        .find(|method| method.name().is_some_and(|name| name.text() == "method"))
        .expect("method declaration");
    assert_eq!(
        method
            .return_type()
            .expect("return type")
            .source_text()
            .trim(),
        "String"
    );
    assert_eq!(
        method
            .parameters()
            .expect("parameters")
            .parameters()
            .count(),
        1
    );
    assert!(method.throws_clause().is_some());
    assert_eq!(
        method
            .throws_clause()
            .expect("method throws")
            .exceptions()
            .map(|exception| exception.source_text().trim().to_owned())
            .collect::<Vec<_>>(),
        ["Exception"]
    );
    assert!(method.body().is_some());

    let record = syntax
        .type_declarations()
        .find_map(|declaration| match declaration {
            TypeDeclaration::RecordDeclaration(record) => Some(record),
            _ => None,
        })
        .expect("record declaration");
    let component_names = record
        .components()
        .expect("record components")
        .components()
        .map(|component| component.name().expect("component name").text().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(component_names, ["value", "names"]);
    assert!(record.implements_clause().is_some());
    assert_eq!(
        record
            .body()
            .expect("record body")
            .members()
            .map(|member| member.kind())
            .collect::<Vec<_>>(),
        [JavaSyntaxKind::CompactConstructorDeclaration]
    );

    let enum_ = syntax
        .type_declarations()
        .find_map(|declaration| match declaration {
            TypeDeclaration::EnumDeclaration(enum_) => Some(enum_),
            _ => None,
        })
        .expect("enum declaration");
    let enum_body = enum_.body().expect("enum body");
    let constants = enum_body.constants().expect("enum constants");
    assert_eq!(
        constants
            .constants()
            .map(|constant| constant.name().expect("constant name").text().to_owned())
            .collect::<Vec<_>>(),
        ["ONE", "TWO"]
    );
    assert_eq!(enum_body.members().count(), 1);
}

#[test]
fn interface_and_annotation_body_accessors_expose_members() {
    let syntax = parse_clean(
        r#"
                interface Api {
                    int VALUE = 1;
                    void call();
                    class Nested {}
                }

                @interface Anno {
                    String value() default "x";
                    int COUNT = 1;
                    class Nested {}
                    ;
                }
            "#,
    );

    let interface = syntax
        .type_declarations()
        .find_map(|declaration| match declaration {
            TypeDeclaration::InterfaceDeclaration(interface) => Some(interface),
            _ => None,
        })
        .expect("interface declaration");
    assert_eq!(
        interface
            .body()
            .expect("interface body")
            .members()
            .map(|member| member.kind())
            .collect::<Vec<_>>(),
        [
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::MethodDeclaration,
            JavaSyntaxKind::ClassDeclaration,
        ]
    );

    let annotation = syntax
        .type_declarations()
        .find_map(|declaration| match declaration {
            TypeDeclaration::AnnotationInterfaceDeclaration(annotation) => Some(annotation),
            _ => None,
        })
        .expect("annotation interface declaration");
    assert_eq!(
        annotation
            .body()
            .expect("annotation interface body")
            .members()
            .map(|member| member.kind())
            .collect::<Vec<_>>(),
        [
            JavaSyntaxKind::AnnotationElementDeclaration,
            JavaSyntaxKind::FieldDeclaration,
            JavaSyntaxKind::ClassDeclaration,
            JavaSyntaxKind::EmptyDeclaration,
        ]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn expression_and_statement_accessors_expose_layout_roles() {
    let syntax = parse_clean(
        r"
                class Expressions {
                    void test(int a, int b, int c, boolean ready) {
                        int value = (a + b) * -c;
                        value += ready ? call(1, 2) : new int[] { 3 };
                        java.util.function.Supplier<Expressions> supplier = Expressions::new;
                        builder.add(a).add(b).build();
                        this.field = builder.value;
                        for (value = 0, a = 0; value < 3; value++, a++) value += a;
                        for (int i = 0; i < 3; i++) value += i;
                        for (String item : names()) call(item);
                        while (ready) value++;
                        do value--; while (ready);
                        synchronized (this) { call(value); }
                        return;
                    }
                }
            ",
    );

    let assignment = descendants::<AssignmentExpression>(&syntax)
        .into_iter()
        .find(|assignment| assignment.source_text().contains("+="))
        .expect("compound assignment");
    assert_eq!(
        assignment.operator().expect("assignment operator").kind(),
        JavaSyntaxKind::PlusEq
    );
    assert_eq!(
        assignment
            .left()
            .expect("assignment lhs")
            .source_text()
            .trim(),
        "value"
    );
    assert!(matches!(
        assignment.right().expect("assignment rhs"),
        Expression::ConditionalExpression(_)
    ));

    let conditional = descendants::<ConditionalExpression>(&syntax)
        .into_iter()
        .next()
        .expect("conditional expression");
    assert_eq!(
        conditional.question_token().expect("question token").kind(),
        JavaSyntaxKind::Question
    );
    assert_eq!(
        conditional.colon_token().expect("colon token").kind(),
        JavaSyntaxKind::Colon
    );
    assert_eq!(
        conditional
            .condition()
            .expect("condition")
            .source_text()
            .trim(),
        "ready"
    );
    assert_eq!(
        conditional
            .true_expression()
            .expect("true expression")
            .source_text()
            .trim(),
        "call(1, 2)"
    );
    assert!(matches!(
        conditional.false_expression().expect("false expression"),
        Expression::ArrayCreationExpression(_)
    ));

    let binary = descendants::<BinaryExpression>(&syntax)
        .into_iter()
        .find(|binary| binary.source_text().contains('*'))
        .expect("binary expression");
    assert_eq!(
        binary.operator().expect("binary operator").kind(),
        JavaSyntaxKind::Star
    );
    assert!(matches!(
        binary.left().expect("binary lhs"),
        Expression::ParenthesizedExpression(_)
    ));
    assert!(matches!(
        binary.right().expect("binary rhs"),
        Expression::UnaryExpression(_)
    ));

    let invocation = descendants::<MethodInvocationExpression>(&syntax)
        .into_iter()
        .find(|invocation| invocation.source_text().trim() == "call(1, 2)")
        .expect("method invocation");
    assert_eq!(
        invocation
            .arguments()
            .expect("arguments")
            .arguments()
            .count(),
        2
    );

    let method_reference = descendants::<MethodReferenceExpression>(&syntax)
        .into_iter()
        .next()
        .expect("method reference");
    assert_eq!(
        method_reference
            .double_colon()
            .expect("double colon")
            .kind(),
        JavaSyntaxKind::DoubleColon
    );
    assert!(method_reference.is_constructor_reference());
    assert_eq!(
        method_reference.new_token().expect("new token").kind(),
        JavaSyntaxKind::NewKw
    );

    let statement_expression_list = descendants::<StatementExpressionList>(&syntax)
        .into_iter()
        .find(|list| list.source_text().contains("value = 0"))
        .expect("statement expression list");
    let entries = statement_expression_list.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].expression.source_text().trim(), "value = 0");
    assert_eq!(
        entries[0].comma.as_ref().expect("first comma").kind(),
        JavaSyntaxKind::Comma
    );
    assert_eq!(entries[1].expression.source_text().trim(), "a = 0");
    assert!(entries[1].comma.is_none());

    let chain_invocation = descendants::<MethodInvocationExpression>(&syntax)
        .into_iter()
        .find(|invocation| invocation.source_text().trim() == "builder.add(a).add(b).build()")
        .expect("chain invocation");
    let chain = Expression::from(chain_invocation)
        .member_chain()
        .expect("member chain");
    assert_eq!(chain.root().source_text().trim(), "builder");
    assert_eq!(chain.suffixes().len(), 3);
    assert!(matches!(
        chain.suffixes(),
        [
            MemberChainSuffix::MethodInvocation(_),
            MemberChainSuffix::MethodInvocation(_),
            MemberChainSuffix::MethodInvocation(_),
        ]
    ));

    let field_chain = descendants::<FieldAccessExpression>(&syntax)
        .into_iter()
        .find(|access| access.source_text().trim() == "builder.value")
        .expect("field chain");
    let field_chain = Expression::from(field_chain)
        .member_chain()
        .expect("field member chain");
    assert_eq!(field_chain.root().source_text().trim(), "builder");
    assert!(matches!(
        field_chain.suffixes(),
        [MemberChainSuffix::FieldAccess(_)]
    ));

    let basic_for = descendants::<ForStatement>(&syntax)
        .into_iter()
        .filter_map(|for_statement| for_statement.basic())
        .find(|basic| {
            basic
                .condition()
                .is_some_and(|condition| condition.source_text().trim() == "i < 3")
        })
        .expect("basic for statement");
    assert!(basic_for.initializer().is_some());
    assert_eq!(
        basic_for
            .condition()
            .expect("for condition")
            .source_text()
            .trim(),
        "i < 3"
    );
    assert!(basic_for.update().is_some());
    assert!(basic_for.body().is_some());

    let enhanced_for = descendants::<ForStatement>(&syntax)
        .into_iter()
        .find_map(|for_statement| for_statement.enhanced())
        .expect("enhanced for statement");
    assert!(enhanced_for.variable().is_some());
    assert_eq!(
        enhanced_for
            .iterable()
            .expect("iterable")
            .source_text()
            .trim(),
        "names()"
    );
    assert!(enhanced_for.body().is_some());

    let while_statement = descendants::<WhileStatement>(&syntax)
        .into_iter()
        .next()
        .expect("while statement");
    assert_eq!(
        while_statement
            .condition()
            .expect("condition")
            .source_text()
            .trim(),
        "ready"
    );
    assert!(while_statement.body().is_some());

    let synchronized = descendants::<SynchronizedStatement>(&syntax)
        .into_iter()
        .next()
        .expect("synchronized statement");
    assert_eq!(
        synchronized
            .expression()
            .expect("expression")
            .source_text()
            .trim(),
        "this"
    );
    assert!(synchronized.body().is_some());
}

#[test]
#[allow(clippy::too_many_lines)]
fn expressions_expose_parent_layout_roles() {
    let syntax = parse_clean(
        r"
                class ParentRoles {
                    Object field;

                    Object method(boolean ready, Object value, Object[] array, int index) {
                        Object local = ready ? array[index] : call(value);
                        if (value instanceof String s) {
                            return this.field;
                        }
                        while (ready) {
                            value = builder.add(value);
                        }
                        for (; ready; value = next()) {
                            synchronized (this) {
                                throw fail();
                            }
                        }
                        return (Object) array[index];
                    }
                }
            ",
    );

    let initializer = descendants::<VariableInitializer>(&syntax)
        .into_iter()
        .find(|initializer| initializer.source_text().contains('?'))
        .expect("conditional initializer");
    let conditional = initializer
        .value()
        .and_then(|value| Expression::cast(value.syntax().clone()))
        .expect("initializer expression");
    assert_eq!(
        conditional.parent_role(),
        Some(ExpressionParentRole::VariableInitializer)
    );

    let conditional = match conditional {
        Expression::ConditionalExpression(conditional) => conditional,
        other => panic!("expected conditional expression, got {:?}", other.kind()),
    };
    assert_eq!(
        conditional
            .condition()
            .expect("conditional condition")
            .parent_role(),
        Some(ExpressionParentRole::ConditionalCondition)
    );
    assert_eq!(
        conditional
            .true_expression()
            .expect("true expression")
            .parent_role(),
        Some(ExpressionParentRole::ConditionalTrueExpression)
    );
    assert_eq!(
        conditional
            .false_expression()
            .expect("false expression")
            .parent_role(),
        Some(ExpressionParentRole::ConditionalFalseExpression)
    );

    let array_access = descendants::<ArrayAccessExpression>(&syntax)
        .into_iter()
        .find(|access| access.source_text().trim() == "array[index]")
        .expect("array access");
    assert_eq!(
        array_access.array().expect("array").parent_role(),
        Some(ExpressionParentRole::ArrayAccessArray)
    );
    assert_eq!(
        array_access.index().expect("index").parent_role(),
        Some(ExpressionParentRole::ArrayAccessIndex)
    );

    let call = descendants::<MethodInvocationExpression>(&syntax)
        .into_iter()
        .find(|invocation| invocation.source_text().trim() == "call(value)")
        .expect("call expression");
    let argument = call
        .arguments()
        .expect("arguments")
        .arguments()
        .next()
        .expect("argument");
    assert_eq!(argument.parent_role(), Some(ExpressionParentRole::Argument));

    let if_statement = descendants::<IfStatement>(&syntax)
        .into_iter()
        .next()
        .expect("if statement");
    let instanceof = if_statement.condition().expect("if condition");
    assert_eq!(
        instanceof.parent_role(),
        Some(ExpressionParentRole::IfCondition)
    );
    let instanceof = match instanceof {
        Expression::InstanceofExpression(instanceof) => instanceof,
        other => panic!("expected instanceof expression, got {:?}", other.kind()),
    };
    assert_eq!(
        instanceof
            .expression()
            .expect("instanceof operand")
            .parent_role(),
        Some(ExpressionParentRole::InstanceofOperand)
    );

    let field_access = descendants::<FieldAccessExpression>(&syntax)
        .into_iter()
        .find(|access| access.source_text().trim() == "this.field")
        .expect("field access");
    assert_eq!(
        field_access
            .receiver()
            .expect("field receiver")
            .parent_role(),
        Some(ExpressionParentRole::FieldAccessReceiver)
    );

    let while_statement = descendants::<WhileStatement>(&syntax)
        .into_iter()
        .next()
        .expect("while statement");
    assert_eq!(
        while_statement
            .condition()
            .expect("while condition")
            .parent_role(),
        Some(ExpressionParentRole::WhileCondition)
    );

    let assignment = descendants::<AssignmentExpression>(&syntax)
        .into_iter()
        .find(|assignment| assignment.source_text().contains("builder.add"))
        .expect("assignment expression");
    assert_eq!(
        assignment.left().expect("assignment lhs").parent_role(),
        Some(ExpressionParentRole::AssignmentLeft)
    );
    assert_eq!(
        assignment.right().expect("assignment rhs").parent_role(),
        Some(ExpressionParentRole::AssignmentRight)
    );

    let builder_call = descendants::<MethodInvocationExpression>(&syntax)
        .into_iter()
        .find(|invocation| invocation.source_text().trim() == "builder.add(value)")
        .expect("builder call");
    assert_eq!(
        builder_call
            .qualifier()
            .expect("method qualifier")
            .parent_role(),
        Some(ExpressionParentRole::MethodInvocationQualifier)
    );

    let basic_for = descendants::<ForStatement>(&syntax)
        .into_iter()
        .filter_map(|for_statement| for_statement.basic())
        .find(|basic| basic.update().is_some())
        .expect("basic for");
    assert_eq!(
        basic_for.condition().expect("for condition").parent_role(),
        Some(ExpressionParentRole::BasicForCondition)
    );

    let synchronized = descendants::<SynchronizedStatement>(&syntax)
        .into_iter()
        .next()
        .expect("synchronized statement");
    assert_eq!(
        synchronized
            .expression()
            .expect("sync expression")
            .parent_role(),
        Some(ExpressionParentRole::SynchronizedExpression)
    );

    let throw_statement = descendants::<ThrowStatement>(&syntax)
        .into_iter()
        .next()
        .expect("throw statement");
    assert_eq!(
        throw_statement
            .expression()
            .expect("throw value")
            .parent_role(),
        Some(ExpressionParentRole::ThrowValue)
    );

    let cast = descendants::<CastExpression>(&syntax)
        .into_iter()
        .next()
        .expect("cast expression");
    assert_eq!(
        cast.expression().expect("cast operand").parent_role(),
        Some(ExpressionParentRole::CastOperand)
    );

    let return_statement = descendants::<ReturnStatement>(&syntax)
        .into_iter()
        .find(|statement| statement.source_text().contains("(Object)"))
        .expect("return statement");
    assert_eq!(
        return_statement
            .expression()
            .expect("return value")
            .parent_role(),
        Some(ExpressionParentRole::ReturnValue)
    );
}
