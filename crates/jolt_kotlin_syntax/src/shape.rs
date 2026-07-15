use jolt_syntax::{
    BuildSyntaxTreeError, FactoryNode, FactorySlot, ParsedChildren, RawSyntaxKind, SyntaxFactory,
    SyntaxTreeSink,
};

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

macro_rules! kotlin_fixed_slot {
    ($input:ident, $cursor:ident, required $matcher:tt) => {{
        if kotlin_matches!($input, $cursor, $matcher) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else if kotlin_is_recovery_item($input, $cursor) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            FactorySlot::Missing
        }
    }};
    ($input:ident, $cursor:ident, optional $matcher:tt) => {{
        if kotlin_matches!($input, $cursor, $matcher) {
            let slot = FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            FactorySlot::Absent
        }
    }};
}

#[inline]
fn kotlin_is_recovery_item(input: ParsedChildren<'_>, index: usize) -> bool {
    input.is_directly_malformed(index)
}

macro_rules! kotlin_item_matches {
    ($input:ident, $index:expr, $matcher:tt) => {
        kotlin_matches!($input, $index, $matcher) || kotlin_is_recovery_item($input, $index)
    };
}

macro_rules! kotlin_fixed_node {
    ($kind:ident, $input:ident, $sink:ident;
        $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        let slots = [$(kotlin_fixed_slot!($input, cursor, $cardinality $matcher)),*];
        if cursor != $input.len() {
            Ok($sink.raw_malformed($kind.to_raw()))
        } else {
            Ok($sink.fixed($kind.to_raw(), slots))
        }
    }};
}

macro_rules! kotlin_list_minimum {
    (many) => {
        0_usize
    };
    (one_or_more) => {
        1_usize
    };
}

macro_rules! kotlin_list_node {
    ($kind:ident, $input:ident, $sink:ident,
        $cardinality:ident, $matcher:tt) => {{
        let minimum = kotlin_list_minimum!($cardinality);
        if (0..$input.len()).all(|index| kotlin_item_matches!($input, index, $matcher)) {
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
        kotlin_list_node!($kind, $input, $sink, $cardinality, $matcher)
    };
    ($kind:ident, $input:ident, $sink:ident,
        $cardinality:ident, $matcher:tt [
            separated $separator:tt,
            minimum $minimum:literal,
            trailing $trailing:ident,
            recovery bogus_owner
        ]) => {{
        kotlin_separated_list!(
            $kind, $input, $sink, $matcher, $separator, $minimum, $trailing
        )
    }};
}

macro_rules! kotlin_separated_list {
    ($kind:ident, $input:ident, $sink:ident, $matcher:tt, $separator:tt, $minimum:literal, $trailing:ident) => {{
        let minimum = $minimum as usize;
        let mut cursor = 0;
        let mut item_count = 0_usize;
        let mut expect_item = true;
        let mut recovered = false;
        while cursor < $input.len() {
            if expect_item {
                if kotlin_item_matches!($input, cursor, $matcher) {
                    item_count += 1;
                    expect_item = false;
                    cursor += 1;
                } else if kotlin_matches!($input, cursor, $separator) {
                    recovered = true;
                    expect_item = false;
                } else {
                    break;
                }
            } else if kotlin_matches!($input, cursor, $separator) {
                expect_item = true;
                cursor += 1;
            } else if kotlin_item_matches!($input, cursor, $matcher) {
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
            recovered |= kotlin_trailing_needs_empty!($trailing, item_count, expect_item);
            if !recovered {
                Ok($sink.fixed($kind.to_raw(), (0..$input.len()).map(FactorySlot::Input)))
            } else {
                let mut slots = Vec::with_capacity($input.len().max(minimum * 2));
                let mut cursor = 0;
                let mut count = 0_usize;
                let mut expect_item = true;
                while cursor < $input.len() {
                    if expect_item {
                        if kotlin_item_matches!($input, cursor, $matcher) {
                            slots.push(FactorySlot::Input(cursor));
                            count += 1;
                            expect_item = false;
                            cursor += 1;
                        } else {
                            slots.push(FactorySlot::Missing);
                            expect_item = false;
                        }
                    } else if kotlin_matches!($input, cursor, $separator) {
                        slots.push(FactorySlot::Input(cursor));
                        expect_item = true;
                        cursor += 1;
                    } else {
                        slots.push(FactorySlot::Missing);
                        expect_item = true;
                    }
                }
                kotlin_push_trailing_empty!($trailing, slots, count, expect_item);
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

macro_rules! kotlin_trailing_needs_empty {
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

macro_rules! kotlin_push_trailing_empty {
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

macro_rules! kotlin_factory_arm {
    ($kind:ident, $input:ident, $sink:ident, valid; $($fields:tt)*) => {
        kotlin_fixed_node!($kind, $input, $sink; $($fields)*)
    };
    ($kind:ident, $input:ident, $sink:ident, constructed; $($fields:tt)*) => {
        kotlin_fixed_node!($kind, $input, $sink; $($fields)*)
    };
    ($kind:ident, $input:ident, $sink:ident, list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        kotlin_list_node!($kind, $input, $sink, $cardinality, $matcher $([$($policy)*])?)
    };
    ($kind:ident, $input:ident, $sink:ident, malformed; $($fields:tt)*) => {
        Ok($sink.raw_malformed($kind.to_raw()))
    };
}

macro_rules! define_kotlin_factory {
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
        fn kotlin_category_accepts(category: Category, kind: Option<KotlinSyntaxKind>) -> bool {
            match category {
                $(Category::$family => matches!(kind, Some(KotlinSyntaxKind::$bogus $(| KotlinSyntaxKind::$member)*)),)*
            }
        }

        impl SyntaxFactory for KotlinSyntaxFactory {
            fn make_syntax(
                &self,
                raw_kind: RawSyntaxKind,
                input: ParsedChildren<'_>,
                sink: &mut SyntaxTreeSink<'_>,
            ) -> Result<FactoryNode, BuildSyntaxTreeError> {
                let kind = KotlinSyntaxKind::from_raw(raw_kind)
                    .ok_or(BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind })?;
                match kind {
                    $(KotlinSyntaxKind::$kind => kotlin_factory_arm!(kind, input, sink, $class; $($fields)*),)*
                    $(KotlinSyntaxKind::$bogus => Ok(sink.raw_malformed(raw_kind)),)*
                    _ => Err(BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind }),
                }
            }
        }
    };
}

kotlin_syntax_schema!(define_kotlin_factory);
