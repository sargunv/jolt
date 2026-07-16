use jolt_java_syntax::{
    AnnotationArrayInitializer, ArrayInitializer, EmptyDeclaration, EmptyStatement, Guard,
    IfStatement, ImportDeclaration, JavaNode, JavaSyntaxNode, JavaSyntaxView, ModifierList,
    ModuleDirective, ParameterModifierList, RequiresModifierList, UsesDirective, WhileStatement,
    parse_compilation_unit,
};

#[test]
fn trailing_comma_synthesis_requires_a_represented_value() {
    let empty_array_parse = parse_compilation_unit("class C { int[] values = {}; }");
    let empty_array = find_node::<ArrayInitializer<'_>>(
        empty_array_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    );
    assert!(empty_array.trailing_comma_claim().is_none());

    let populated_array_parse = parse_compilation_unit("class C { int[] values = {1}; }");
    let populated_array = find_node::<ArrayInitializer<'_>>(
        populated_array_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    );
    assert!(populated_array.trailing_comma_claim().is_some());

    let empty_annotation_parse =
        parse_compilation_unit("@interface A { int[] value() default {}; }");
    let empty_annotation = find_node::<AnnotationArrayInitializer<'_>>(
        empty_annotation_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    );
    assert!(empty_annotation.trailing_comma_claim().is_none());

    let populated_annotation_parse =
        parse_compilation_unit("@interface A { int[] value() default {1}; }");
    let populated_annotation = find_node::<AnnotationArrayInitializer<'_>>(
        populated_annotation_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    );
    assert!(populated_annotation.trailing_comma_claim().is_some());
}

#[test]
fn guard_parenthesis_removal_is_paired_and_recovery_free() {
    let paired_parse = parse_compilation_unit(
        "class C { void f(Object value) { switch (value) { case String s when (s.isBlank()) -> {} } } }",
    );
    let paired = find_node::<Guard<'_>>(
        paired_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    )
    .redundant_parenthesis_removal_claims();
    assert!(paired.open.is_some() && paired.close.is_some());

    let unpaired_parse = parse_compilation_unit(
        "class C { void f(Object value) { switch (value) { case String s when (s.isBlank() -> {} } } }",
    );
    let unpaired = find_node::<Guard<'_>>(
        unpaired_parse
            .syntax()
            .expect("represented compilation unit")
            .syntax_node()
            .expect("physical compilation unit"),
    )
    .redundant_parenthesis_removal_claims();
    assert!(unpaired.open.is_none() && unpaired.close.is_none());
}

#[test]
fn valid_empty_syntax_authorizes_only_its_own_separator() {
    let parse = parse_compilation_unit("; class C { void f() { ; } }");
    let root = parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    let declaration = find_node::<EmptyDeclaration<'_>>(root);
    let statement = find_node::<EmptyStatement<'_>>(root);

    assert!(declaration.separator_removal_claim().is_some());
    assert!(statement.separator_removal_claim().is_some());
}

#[test]
fn block_brace_synthesis_requires_the_exact_recovery_free_control_owner() {
    let valid_parse = parse_compilation_unit(
        "class C { void f(boolean value) { if (value) ; while (value) ; } }",
    );
    let valid_root = valid_parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    assert!(
        find_node::<IfStatement<'_>>(valid_root)
            .then_block_brace_claims()
            .is_some()
    );
    assert!(
        find_node::<WhileStatement<'_>>(valid_root)
            .body_block_brace_claims()
            .is_some()
    );

    let recovered_parse = parse_compilation_unit("class C { void f() { if () ; while () ; } }");
    let recovered_root = recovered_parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    assert!(
        find_node::<IfStatement<'_>>(recovered_root)
            .then_block_brace_claims()
            .is_none()
    );
    assert!(
        find_node::<WhileStatement<'_>>(recovered_root)
            .body_block_brace_claims()
            .is_none()
    );
}

#[test]
fn import_and_modifier_reordering_require_recovery_free_owners() {
    let valid_import_parse = parse_compilation_unit("import z.B; class C {}");
    assert!(
        find_node::<ImportDeclaration<'_>>(
            valid_import_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_some()
    );

    let malformed_import_parse = parse_compilation_unit("import z.B unexpected; class C {}");
    assert!(
        find_node::<ImportDeclaration<'_>>(
            malformed_import_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_none()
    );

    let valid_modifier_parse = parse_compilation_unit("public final class C {}");
    assert!(
        find_node::<ModifierList<'_>>(
            valid_modifier_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_some()
    );

    let malformed_modifier_parse = parse_compilation_unit("class C { transient void method() {} }");
    assert!(
        find_node::<ModifierList<'_>>(
            malformed_modifier_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_none()
    );

    let valid_parameter_parse = parse_compilation_unit("class C { void f(final String value) {} }");
    assert!(
        find_node::<ParameterModifierList<'_>>(
            valid_parameter_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_some()
    );

    let malformed_parameter_parse =
        parse_compilation_unit("class C { void f(transient String value) {} }");
    assert!(
        find_node::<ParameterModifierList<'_>>(
            malformed_parameter_parse
                .syntax()
                .expect("represented compilation unit")
                .syntax_node()
                .expect("physical compilation unit"),
        )
        .canonical_reorder_claim()
        .is_none()
    );
}

#[test]
fn module_reordering_uses_the_smallest_complete_local_owner() {
    let valid_module_parse =
        parse_compilation_unit("module m { requires transitive static a.b; uses z.B; }");
    let valid_module_root = valid_module_parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    assert!(
        ModuleDirective::UsesDirective(find_node::<UsesDirective<'_>>(valid_module_root))
            .canonical_reorder_claim()
            .is_some()
    );
    assert!(
        find_node::<RequiresModifierList<'_>>(valid_module_root)
            .canonical_reorder_claim()
            .is_some()
    );

    let malformed_module_parse =
        parse_compilation_unit("module m { requires transitive + a.b; uses z.B; }");
    let malformed_module_root = malformed_module_parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    assert!(
        ModuleDirective::UsesDirective(find_node::<UsesDirective<'_>>(malformed_module_root))
            .canonical_reorder_claim()
            .is_some()
    );
    assert!(
        find_node::<RequiresModifierList<'_>>(malformed_module_root)
            .canonical_reorder_claim()
            .is_some()
    );
}

fn find_node<'source, N: JavaNode<'source>>(root: JavaSyntaxNode<'source>) -> N {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some(node) = N::cast(node) {
            return node;
        }
        stack.extend(node.children());
    }
    panic!("expected {}", std::any::type_name::<N>());
}
