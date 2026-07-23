use jolt_kotlin_syntax::{
    BinaryExpression, DeclarationBody, KotlinFileItem, KotlinNode, KotlinSyntaxField,
    KotlinSyntaxKind, KotlinSyntaxListPart, KotlinSyntaxNode, KotlinSyntaxView,
    UserTypeSegmentList, ValueParameterSeparatedList, parse_kotlin_file,
};
use jolt_test_support::{kotlin_fixture_root, read_to_string};

#[test]
fn block_inner_is_whitespace_rejects_adjacent_interior_tokens() {
    let fixture = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"))
        .join("syntax/parser/block-empty-statement-adjacent-braces.kt");
    let source = read_to_string(&fixture);
    let parse = parse_kotlin_file(&source);
    let syntax = parse
        .syntax()
        .unwrap_or_else(|| panic!("parser aborted in {}", fixture.display()));
    let KotlinSyntaxField::Present(items) = syntax.items() else {
        panic!("expected structured file items in {}", fixture.display());
    };
    let Some(function) = items.parts().find_map(|part| {
        let KotlinSyntaxListPart::Item(item) = part else {
            return None;
        };
        match item.cast_family::<KotlinFileItem<'_>>() {
            Some(KotlinFileItem::FunctionDeclaration(function)) => Some(function),
            _ => None,
        }
    }) else {
        panic!("expected function declaration in {}", fixture.display());
    };
    let KotlinSyntaxField::Present(body) = function.body() else {
        panic!("expected function body in {}", fixture.display());
    };
    let DeclarationBody::BlockBody(body) = body else {
        panic!("expected function body block in {}", fixture.display());
    };
    let KotlinSyntaxField::Present(block) = body.block() else {
        panic!("expected represented block in {}", fixture.display());
    };

    assert!(
        !block.inner_is_whitespace(),
        "a represented semicolon token adjacent to both braces is still block interior"
    );
}

#[test]
fn directly_malformed_valid_and_list_nodes_have_no_typed_wrapper() {
    let binary_parse = parse_kotlin_file("fun f() = 1 2\n");
    let binary_root = binary_parse
        .syntax()
        .expect("malformed Kotlin source must retain a typed root")
        .syntax_node()
        .expect("typed Kotlin root must have physical syntax");
    let binary = find_node(binary_root, KotlinSyntaxKind::BinaryExpression);
    assert!(binary.is_directly_malformed());
    assert!(BinaryExpression::cast(binary).is_none());

    let list_parse = parse_kotlin_file("typealias T = A..B\n");
    let list_root = list_parse
        .syntax()
        .expect("malformed Kotlin source must retain a typed root")
        .syntax_node()
        .expect("typed Kotlin root must have physical syntax");
    let list = find_node(list_root, KotlinSyntaxKind::UserTypeSegmentList);
    assert!(list.is_directly_malformed());
    assert!(UserTypeSegmentList::cast(list).is_none());
}

#[test]
fn list_parts_expose_directly_malformed_items() {
    let parse = parse_kotlin_file("fun f(first: Int 1) {}\n");
    let root = parse
        .syntax()
        .expect("malformed Kotlin source must retain a typed root")
        .syntax_node()
        .expect("typed Kotlin root must have physical syntax");
    let raw_list = find_node(root, KotlinSyntaxKind::ValueParameterSeparatedList);
    let list = ValueParameterSeparatedList::cast(raw_list)
        .expect("non-malformed parameter list must retain its typed wrapper");
    let malformed = list.parts().find_map(|part| match part {
        KotlinSyntaxListPart::Malformed(malformed) => Some(malformed),
        KotlinSyntaxListPart::Item(_)
        | KotlinSyntaxListPart::Separator(_)
        | KotlinSyntaxListPart::Missing(_) => None,
    });
    let malformed = malformed.expect("recovered parameter must remain a malformed list part");
    assert!(
        malformed
            .syntax_node()
            .expect("malformed parameter must have physical syntax")
            .is_directly_malformed()
    );
}

fn find_node(root: KotlinSyntaxNode<'_>, kind: KotlinSyntaxKind) -> KotlinSyntaxNode<'_> {
    let mut nodes = vec![root];
    let mut seen = Vec::new();
    while let Some(node) = nodes.pop() {
        if node.kind() == kind {
            return node;
        }
        seen.push(node.kind());
        nodes.extend(node.children());
    }
    panic!("expected {kind:?} in represented Kotlin tree; saw {seen:?}");
}
