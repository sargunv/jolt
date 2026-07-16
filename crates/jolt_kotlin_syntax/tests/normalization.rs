use jolt_kotlin_syntax::{
    BinaryExpression, Expression, ImportDirective, KotlinNode, KotlinRoleElement,
    KotlinSyntaxListPart, KotlinSyntaxNode, KotlinSyntaxView, TerminatorList, parse_kotlin_file,
};

#[test]
fn separator_removal_requires_a_clean_owning_list() {
    let clean_parse = parse_kotlin_file("fun first() { value; }\nfun second() { other; }\n");
    let root = clean_parse
        .syntax()
        .expect("represented Kotlin file")
        .syntax_node()
        .expect("physical Kotlin file");
    let lists = find_nodes::<TerminatorList<'_>>(root);
    let first_token = terminator(&lists[0]);
    let second_token = terminator(&lists[1]);

    assert!(lists[0].separator_removal_claim(first_token).is_some());
    assert!(lists[0].separator_removal_claim(second_token).is_none());
}

#[test]
fn precedence_parentheses_require_a_recovery_free_expression() {
    let valid_parse = parse_kotlin_file("val value = 1 + 2\n");
    let valid = Expression::BinaryExpression(
        find_nodes::<BinaryExpression<'_>>(
            valid_parse
                .syntax()
                .expect("represented Kotlin file")
                .syntax_node()
                .expect("physical Kotlin file"),
        )[0],
    );
    assert!(valid.precedence_parenthesis_claims().is_some());

    let recovered_parse = parse_kotlin_file("val value = 1 +\n");
    let recovered = Expression::BinaryExpression(
        find_nodes::<BinaryExpression<'_>>(
            recovered_parse
                .syntax()
                .expect("represented Kotlin file")
                .syntax_node()
                .expect("physical Kotlin file"),
        )[0],
    );
    assert!(recovered.precedence_parenthesis_claims().is_none());
}

#[test]
fn import_reordering_requires_a_recovery_free_directive() {
    let valid_parse = parse_kotlin_file("import z.B\nclass C\n");
    let valid = find_nodes::<ImportDirective<'_>>(
        valid_parse
            .syntax()
            .expect("represented Kotlin file")
            .syntax_node()
            .expect("physical Kotlin file"),
    )[0];
    assert!(valid.canonical_reorder_claim().is_some());

    let malformed_parse = parse_kotlin_file("import z.B unexpected\nclass C\n");
    let malformed = find_nodes::<ImportDirective<'_>>(
        malformed_parse
            .syntax()
            .expect("represented Kotlin file")
            .syntax_node()
            .expect("physical Kotlin file"),
    )[0];
    assert!(malformed.canonical_reorder_claim().is_none());
}

fn terminator<'source>(
    list: &TerminatorList<'source>,
) -> jolt_kotlin_syntax::KotlinSyntaxToken<'source> {
    list.parts()
        .find_map(|part| match part.ok()? {
            KotlinSyntaxListPart::Item(KotlinRoleElement::Token(token)) => Some(token),
            KotlinSyntaxListPart::Item(KotlinRoleElement::Node(_))
            | KotlinSyntaxListPart::Separator(_)
            | KotlinSyntaxListPart::Missing(_)
            | KotlinSyntaxListPart::Malformed(_) => None,
        })
        .expect("represented block terminator")
}

fn find_nodes<'source, N: KotlinNode<'source>>(root: KotlinSyntaxNode<'source>) -> Vec<N> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some(node) = N::cast(node) {
            found.push(node);
        }
        stack.extend(node.children());
    }
    found
}
