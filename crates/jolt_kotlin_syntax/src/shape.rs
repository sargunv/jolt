#![allow(dead_code)]

use crate::KotlinSyntaxKind;

macro_rules! define_shapes {
    ($($schema:tt)*) => {
        jolt_syntax::__lower_syntax_schema!(KotlinSyntaxKind; $($schema)*);
    };
}

kotlin_syntax_schema!(define_shapes);
