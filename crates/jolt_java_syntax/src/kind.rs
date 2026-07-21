macro_rules! define_java_syntax_kind {
    ($($schema:tt)*) => {
        jolt_syntax::__define_syntax_kind!(
            JavaSyntaxKind, "A Java token or syntax node kind."; $($schema)*
        );
    };
}

java_syntax_schema!(define_java_syntax_kind);
