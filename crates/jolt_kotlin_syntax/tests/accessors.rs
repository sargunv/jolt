use jolt_kotlin_syntax::{
    Block, KotlinFileItem, KotlinSyntaxField, KotlinSyntaxListPart, parse_kotlin_file,
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
    let Ok(KotlinSyntaxField::Present(items)) = syntax.items() else {
        panic!("expected structured file items in {}", fixture.display());
    };
    let Some(function) = items.parts().find_map(|part| {
        let Ok(KotlinSyntaxListPart::Item(item)) = part else {
            return None;
        };
        match item.cast_family::<KotlinFileItem<'_>>() {
            Some(KotlinFileItem::FunctionDeclaration(function)) => Some(function),
            _ => None,
        }
    }) else {
        panic!("expected function declaration in {}", fixture.display());
    };
    let Ok(KotlinSyntaxField::Present(body)) = function.body() else {
        panic!("expected function body in {}", fixture.display());
    };
    let block = body
        .cast_node::<Block<'_>>()
        .unwrap_or_else(|| panic!("expected function body block in {}", fixture.display()));

    assert!(
        !block.inner_is_whitespace(),
        "a represented semicolon token adjacent to both braces is still block interior"
    );
}
