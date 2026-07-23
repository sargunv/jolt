use jolt_syntax::ParsedChildren;

use crate::JavaSyntaxKind;

fn java_contextual_text_is(input: ParsedChildren<'_>, index: usize, expected: &str) -> bool {
    input
        .token_text(index)
        .is_some_and(|text| crate::lexer::lexical_text_is(text, expected))
}

macro_rules! define_shapes {
    ($($schema:tt)*) => {
        jolt_syntax::__lower_syntax_schema!(JavaSyntaxKind; $($schema)*);
    };
}

java_syntax_schema!(define_shapes);

/// The single generated Java production syntax factory.
pub(crate) struct JavaSyntaxFactory;

macro_rules! java_matches {
    ($input:ident, $index:expr, (token $kind:ident)) => {
        $input.is_token($index) && $input.kind($index) == Some(JavaSyntaxKind::$kind.to_raw())
    };
    ($input:ident, $index:expr, (token_set [$($kind:ident),*])) => {
        $input.is_token($index)
            && matches!(
                $input.kind($index).and_then(JavaSyntaxKind::from_raw),
                Some($(JavaSyntaxKind::$kind)|*)
            )
    };
    ($input:ident, $index:expr, (element_set [$($kind:ident),*])) => {
        matches!(
            $input.kind($index).and_then(JavaSyntaxKind::from_raw),
            Some($(JavaSyntaxKind::$kind)|*)
        )
    };
    ($input:ident, $index:expr, (contextual $text:literal)) => {
        $input.is_token($index)
            && $input.kind($index) == Some(JavaSyntaxKind::Identifier.to_raw())
            && java_contextual_text_is($input, $index, $text)
    };
    ($input:ident, $index:expr, (node $kind:ident)) => {
        $input.is_node($index) && $input.kind($index) == Some(JavaSyntaxKind::$kind.to_raw())
    };
    ($input:ident, $index:expr, (constructed $kind:ident)) => {
        java_matches!($input, $index, (node $kind))
    };
    ($input:ident, $index:expr, (list $kind:ident)) => {
        java_matches!($input, $index, (node $kind))
    };
    ($input:ident, $index:expr, (node_set [$($kind:ident),*])) => {
        $input.is_node($index)
            && matches!(
                $input.kind($index).and_then(JavaSyntaxKind::from_raw),
                Some($(JavaSyntaxKind::$kind)|*)
            )
    };
    ($input:ident, $index:expr, (category $category:ident)) => {
        $input.is_node($index)
            && java_category_accepts(
                Category::$category,
                $input.kind($index).and_then(JavaSyntaxKind::from_raw),
            )
    };
    ($input:ident, $index:expr, (any_node)) => {
        $input.is_node($index)
    };
    ($input:ident, $index:expr, (any_element)) => {
        $index < $input.len()
    };
    ($input:ident, $index:expr, (choice [$($matcher:tt),*])) => {
        false $(|| java_matches!($input, $index, $matcher))*
    };
}

macro_rules! define_java_factory {
    ($($schema:tt)*) => {
        jolt_syntax::__define_syntax_factory!(
            JavaSyntaxKind, JavaSyntaxFactory, java_matches, |_, _| false,
            java_category_accepts; $($schema)*
        );
    };
}

java_syntax_schema!(define_java_factory);

#[cfg(test)]
macro_rules! java_audit_matches {
    ($slot:ident, (token $kind:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if token.kind() == JavaSyntaxKind::$kind)
    };
    ($slot:ident, (token_set [$($kind:ident),*])) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if matches!(token.kind(), $(JavaSyntaxKind::$kind)|*))
    };
    ($slot:ident, (element_set [$($kind:ident),*])) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Node(node) => matches!(node.kind(), $(JavaSyntaxKind::$kind)|*),
            jolt_syntax::SyntaxSlot::Token(token) => matches!(token.kind(), $(JavaSyntaxKind::$kind)|*),
            jolt_syntax::SyntaxSlot::Empty => false,
        }
    };
    ($slot:ident, (contextual $text:literal)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if token.kind() == JavaSyntaxKind::Identifier && crate::lexer::lexical_text_is(token.text(), $text))
    };
    ($slot:ident, (node $kind:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if node.kind() == JavaSyntaxKind::$kind)
    };
    ($slot:ident, (constructed $kind:ident)) => {
        java_audit_matches!($slot, (node $kind))
    };
    ($slot:ident, (list $kind:ident)) => {
        java_audit_matches!($slot, (node $kind))
    };
    ($slot:ident, (node_set [$($kind:ident),*])) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if matches!(node.kind(), $(JavaSyntaxKind::$kind)|*))
    };
    ($slot:ident, (category $category:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if java_category_accepts(Category::$category, Some(node.kind())))
    };
    ($slot:ident, (any_node)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(_))
    };
    ($slot:ident, (any_element)) => {
        !matches!($slot, jolt_syntax::SyntaxSlot::Empty)
    };
    ($slot:ident, (choice [$($matcher:tt),*])) => {
        false $(|| java_audit_matches!($slot, $matcher))*
    };
}

#[cfg(test)]
macro_rules! define_java_physical_audit {
    ($($schema:tt)*) => {
        jolt_test_support::__define_physical_schema_audit! {
            kind: JavaSyntaxKind,
            language: crate::JavaLanguage,
            matches: java_audit_matches,
            accepts_malformed: |_| false,
            visibility: pub(crate),
            $($schema)*
        }
    };
}

#[cfg(test)]
java_syntax_schema!(define_java_physical_audit);
