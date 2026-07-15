#![allow(dead_code)]

use jolt_syntax::{
    BuildSyntaxTreeError, FactoryNode, FactorySlot, ParsedChildren, RawSyntaxKind, SyntaxFactory,
    SyntaxTreeSink,
};

use crate::JavaSyntaxKind;

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
            && $input.token_text_is($index, $text)
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

macro_rules! java_fixed_slot {
    ($input:ident, $cursor:ident, required $matcher:tt) => {{
        if java_matches!($input, $cursor, $matcher) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else if java_is_recovery_item($input, $cursor) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            FactorySlot::Missing
        }
    }};
    ($input:ident, $cursor:ident, optional $matcher:tt) => {{
        if java_matches!($input, $cursor, $matcher) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            FactorySlot::Absent
        }
    }};
}

#[inline]
fn java_is_recovery_item(input: ParsedChildren<'_>, index: usize) -> bool {
    input.is_directly_malformed(index)
}

macro_rules! java_item_matches {
    ($input:ident, $index:expr, $matcher:tt) => {
        java_matches!($input, $index, $matcher) || java_is_recovery_item($input, $index)
    };
}

macro_rules! java_fixed_node {
    ($kind:ident, $input:ident, $sink:ident;
        $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        let slots = [$(java_fixed_slot!($input, cursor, $cardinality $matcher)),*];
        if cursor != $input.len() {
            Ok($sink.raw_malformed($kind.to_raw()))
        } else {
            Ok($sink.fixed($kind.to_raw(), slots))
        }
    }};
}

macro_rules! java_list_minimum {
    (many) => {
        0_usize
    };
    (one_or_more) => {
        1_usize
    };
}

macro_rules! java_list_node {
    ($kind:ident, $input:ident, $sink:ident,
        $cardinality:ident, $matcher:tt) => {{
        let minimum = java_list_minimum!($cardinality);
        if (0..$input.len()).all(|index| java_item_matches!($input, index, $matcher)) {
            if $input.len() >= minimum {
                Ok($sink.fixed($kind.to_raw(), (0..$input.len()).map(FactorySlot::Input)))
            } else {
                Ok($sink.fixed(
                    $kind.to_raw(),
                    std::iter::repeat_n(FactorySlot::Missing, minimum),
                ))
            }
        } else {
            Ok($sink.raw_malformed($kind.to_raw()))
        }
    }};
    ($kind:ident, $input:ident, $sink:ident,
        $cardinality:ident, $matcher:tt [disambiguate $policy:ident]) => {
        java_list_node!($kind, $input, $sink, $cardinality, $matcher)
    };
    ($kind:ident, $input:ident, $sink:ident,
        $cardinality:ident, $matcher:tt [
            separated $separator:tt,
            minimum $minimum:literal,
            trailing $trailing:ident,
            recovery bogus_owner
        ]) => {{
        java_separated_list!(
            $kind, $input, $sink, $matcher, $separator, $minimum, $trailing
        )
    }};
}

macro_rules! java_separated_list {
    ($kind:ident, $input:ident, $sink:ident, $matcher:tt, $separator:tt, $minimum:literal, $trailing:ident) => {{
        let minimum = $minimum as usize;
        let mut cursor = 0;
        let mut item_count = 0_usize;
        let mut expect_item = true;
        let mut recovered = false;
        while cursor < $input.len() {
            if expect_item {
                if java_item_matches!($input, cursor, $matcher) {
                    item_count += 1;
                    expect_item = false;
                    cursor += 1;
                } else if java_matches!($input, cursor, $separator) {
                    recovered = true;
                    expect_item = false;
                } else {
                    break;
                }
            } else if java_matches!($input, cursor, $separator) {
                expect_item = true;
                cursor += 1;
            } else if java_item_matches!($input, cursor, $matcher) {
                recovered = true;
                expect_item = true;
            } else {
                break;
            }
        }
        if cursor != $input.len() {
            Ok($sink.raw_malformed($kind.to_raw()))
        } else {
            recovered |= item_count < minimum;
            recovered |= java_trailing_needs_empty!($trailing, item_count, expect_item);
            if !recovered {
                Ok($sink.fixed($kind.to_raw(), (0..$input.len()).map(FactorySlot::Input)))
            } else {
                let mut slots = Vec::with_capacity($input.len().max(minimum * 2));
                let mut cursor = 0;
                let mut count = 0_usize;
                let mut expect_item = true;
                while cursor < $input.len() {
                    if expect_item {
                        if java_item_matches!($input, cursor, $matcher) {
                            slots.push(FactorySlot::Input(cursor));
                            count += 1;
                            expect_item = false;
                            cursor += 1;
                        } else {
                            slots.push(FactorySlot::Missing);
                            expect_item = false;
                        }
                    } else if java_matches!($input, cursor, $separator) {
                        slots.push(FactorySlot::Input(cursor));
                        expect_item = true;
                        cursor += 1;
                    } else {
                        slots.push(FactorySlot::Missing);
                        expect_item = true;
                    }
                }
                java_push_trailing_empty!($trailing, slots, count, expect_item);
                while count < minimum {
                    if count > 0 {
                        slots.push(FactorySlot::Missing);
                    }
                    slots.push(FactorySlot::Missing);
                    count += 1;
                }
                Ok($sink.fixed($kind.to_raw(), slots))
            }
        }
    }};
}

macro_rules! java_trailing_needs_empty {
    (forbidden, $count:ident, $expect_item:ident) => {
        $expect_item && $count > 0
    };
    (optional, $count:ident, $expect_item:ident) => {
        false
    };
    (required, $count:ident, $expect_item:ident) => {
        !$expect_item && $count > 0
    };
}

macro_rules! java_push_trailing_empty {
    (forbidden, $slots:ident, $count:ident, $expect_item:ident) => {
        if $expect_item && !$slots.is_empty() {
            $slots.push(FactorySlot::Missing);
        }
    };
    (optional, $slots:ident, $count:ident, $expect_item:ident) => {};
    (required, $slots:ident, $count:ident, $expect_item:ident) => {
        if !$expect_item && $count > 0 {
            $slots.push(FactorySlot::Missing);
        }
    };
}

macro_rules! java_factory_arm {
    ($kind:ident, $input:ident, $sink:ident, valid; $($fields:tt)*) => {
        java_fixed_node!($kind, $input, $sink; $($fields)*)
    };
    ($kind:ident, $input:ident, $sink:ident, constructed; $($fields:tt)*) => {
        java_fixed_node!($kind, $input, $sink; $($fields)*)
    };
    ($kind:ident, $input:ident, $sink:ident, list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        java_list_node!($kind, $input, $sink, $cardinality, $matcher $([$($policy)*])?)
    };
    ($kind:ident, $input:ident, $sink:ident, malformed; $($fields:tt)*) => {
        Ok($sink.raw_malformed($kind.to_raw()))
    };
}

macro_rules! define_java_factory {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $(
                $kind:ident => $wrapper:ident [$module:ident $class:ident] {
                    $($fields:tt)*
                }
            )*
        }
    ) => {
        fn java_category_accepts(category: Category, kind: Option<JavaSyntaxKind>) -> bool {
            match category {
                $(Category::$family => matches!(kind, Some(JavaSyntaxKind::$bogus $(| JavaSyntaxKind::$member)*)),)*
            }
        }

        impl SyntaxFactory for JavaSyntaxFactory {
            fn make_syntax(
                &self,
                raw_kind: RawSyntaxKind,
                input: ParsedChildren<'_>,
                sink: &mut SyntaxTreeSink<'_>,
            ) -> Result<FactoryNode, BuildSyntaxTreeError> {
                let kind = JavaSyntaxKind::from_raw(raw_kind)
                    .ok_or(BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind })?;
                match kind {
                    $(JavaSyntaxKind::$kind => java_factory_arm!(kind, input, sink, $class; $($fields)*),)*
                    $(JavaSyntaxKind::$bogus => Ok(sink.raw_malformed(raw_kind)),)*
                    _ => Err(BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind }),
                }
            }
        }
    };
}

java_syntax_schema!(define_java_factory);
