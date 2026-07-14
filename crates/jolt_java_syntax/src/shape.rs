#![allow(dead_code)]

use crate::JavaSyntaxKind;

macro_rules! define_shapes {
    ($($schema:tt)*) => {
        jolt_syntax::__lower_syntax_schema!(JavaSyntaxKind; $($schema)*);
    };
}

java_syntax_schema!(define_shapes);
