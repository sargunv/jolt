macro_rules! define_kotlin_syntax_kind {
    ($($schema:tt)*) => {
        jolt_syntax::__define_syntax_kind!(
            KotlinSyntaxKind, "A Kotlin token or syntax node kind."; $($schema)*
        );
    };
}

kotlin_syntax_schema!(define_kotlin_syntax_kind);
