use jolt_syntax::ParsedChildren;

use crate::KotlinSyntaxKind;

macro_rules! define_shapes {
    ($($schema:tt)*) => {
        jolt_syntax::__lower_syntax_schema!(KotlinSyntaxKind; $($schema)*);
    };
}

kotlin_syntax_schema!(define_shapes);

/// The single generated Kotlin production syntax factory.
pub(crate) struct KotlinSyntaxFactory;

macro_rules! kotlin_matches {
    ($input:ident, $index:expr, (token $kind:ident)) => {
        $input.is_token($index) && $input.kind($index) == Some(KotlinSyntaxKind::$kind.to_raw())
    };
    ($input:ident, $index:expr, (token_set [$($kind:ident),*])) => {
        $input.is_token($index)
            && matches!(
                $input.kind($index).and_then(KotlinSyntaxKind::from_raw),
                Some($(KotlinSyntaxKind::$kind)|*)
            )
    };
    ($input:ident, $index:expr, (element_set [$($kind:ident),*])) => {
        matches!(
            $input.kind($index).and_then(KotlinSyntaxKind::from_raw),
            Some($(KotlinSyntaxKind::$kind)|*)
        )
    };
    ($input:ident, $index:expr, (contextual $text:literal)) => {
        $input.is_token($index)
            && $input.kind($index) == Some(KotlinSyntaxKind::Identifier.to_raw())
            && $input.token_text_is($index, $text)
    };
    ($input:ident, $index:expr, (node $kind:ident)) => {
        $input.is_node($index) && $input.kind($index) == Some(KotlinSyntaxKind::$kind.to_raw())
    };
    ($input:ident, $index:expr, (constructed $kind:ident)) => {
        kotlin_matches!($input, $index, (node $kind))
    };
    ($input:ident, $index:expr, (list $kind:ident)) => {
        kotlin_matches!($input, $index, (node $kind))
    };
    ($input:ident, $index:expr, (node_set [$($kind:ident),*])) => {
        $input.is_node($index)
            && matches!(
                $input.kind($index).and_then(KotlinSyntaxKind::from_raw),
                Some($(KotlinSyntaxKind::$kind)|*)
            )
    };
    ($input:ident, $index:expr, (category $category:ident)) => {
        $input.is_node($index)
            && kotlin_category_accepts(
                Category::$category,
                $input.kind($index).and_then(KotlinSyntaxKind::from_raw),
            )
    };
    ($input:ident, $index:expr, (any_node)) => {
        $input.is_node($index)
    };
    ($input:ident, $index:expr, (any_element)) => {
        $index < $input.len()
    };
    ($input:ident, $index:expr, (choice [$($matcher:tt),*])) => {
        false $(|| kotlin_matches!($input, $index, $matcher))*
    };
}

#[inline]
fn kotlin_is_recovery_item(input: ParsedChildren<'_>, index: usize) -> bool {
    input.is_directly_malformed(index)
}
macro_rules! define_kotlin_factory {
    ($($schema:tt)*) => {
        jolt_syntax::__define_syntax_factory!(
            KotlinSyntaxKind, KotlinSyntaxFactory, kotlin_matches, kotlin_is_recovery_item,
            kotlin_category_accepts; $($schema)*
        );
    };
}

kotlin_syntax_schema!(define_kotlin_factory);
