//! Shared lowering for language syntax schemas.
//!
//! The production language crates consume their declarative schema directly.
//! This macro emits only the category discriminants and physical slot indices
//! shared by their generated factory and typed accessors.

#[doc(hidden)]
#[macro_export]
macro_rules! __lower_syntax_schema {
    (
        $syntax_kind:ident;
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $(
                $kind:ident => $wrapper:ident [$module:ident $class:ident] {
                    $(
                        $field:ident: $cardinality:ident $matcher:tt
                        $(=> $role:ident)?
                        $([$($policy:tt)*])?;
                    )*
                }
            )*
        }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum Category {
            $($family,)*
        }

        $(
            #[allow(non_camel_case_types)]
            pub(crate) mod $module {
                #[repr(usize)]
                pub(crate) enum Slot {
                    $($field,)*
                    __Len,
                }
            }
        )*
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_fixed_slot {
    ($matches:ident, $input:ident, $cursor:ident, required $matcher:tt, $recovery:expr) => {{
        if $matches!($input, $cursor, $matcher) || ($recovery)($input, $cursor) {
            let slot = $crate::FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            $crate::FactorySlot::Missing
        }
    }};
    ($matches:ident, $input:ident, $cursor:ident, optional $matcher:tt, $recovery:expr) => {{
        if $matches!($input, $cursor, $matcher) {
            let slot = $crate::FactorySlot::Input($cursor);
            $cursor += 1;
            slot
        } else {
            $crate::FactorySlot::Absent
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_fixed_node {
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr;
        $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        let slots = [$($crate::__syntax_fixed_slot!(
            $matches, $input, cursor, $cardinality $matcher, $recovery
        )),*];
        if cursor != $input.len() {
            Ok($sink.raw_malformed($kind.to_raw()))
        } else {
            Ok($sink.fixed($kind.to_raw(), slots))
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_list_minimum {
    (many) => {
        0_usize
    };
    (one_or_more) => {
        1_usize
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_list_node {
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        $cardinality:ident, $matcher:tt) => {{
        let minimum = $crate::__syntax_list_minimum!($cardinality);
        let item_matches = |index| $matches!($input, index, $matcher) || ($recovery)($input, index);
        if (0..$input.len()).all(item_matches) {
            if $input.len() >= minimum {
                Ok($sink.fixed(
                    $kind.to_raw(),
                    (0..$input.len()).map($crate::FactorySlot::Input),
                ))
            } else {
                Ok($sink.fixed(
                    $kind.to_raw(),
                    std::iter::repeat_n($crate::FactorySlot::Missing, minimum),
                ))
            }
        } else {
            Ok($sink.raw_malformed($kind.to_raw()))
        }
    }};
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        $cardinality:ident, $matcher:tt [disambiguate $policy:ident]) => {
        $crate::__syntax_list_node!(
            $matches,
            $kind,
            $input,
            $sink,
            $recovery,
            $cardinality,
            $matcher
        )
    };
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        $cardinality:ident, $matcher:tt [
            separated $separator:tt,
            minimum $minimum:literal,
            trailing $trailing:ident,
            recovery bogus_owner
        ]) => {
        $crate::__syntax_separated_list!(
            $matches, $kind, $input, $sink, $recovery, $matcher, $separator, $minimum, $trailing
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_separated_list {
    (
        $matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        $matcher:tt, $separator:tt, $minimum:literal, $trailing:ident
    ) => {{
        let minimum = $minimum as usize;
        let item_matches = |index| $matches!($input, index, $matcher) || ($recovery)($input, index);
        let mut cursor = 0;
        let mut item_count = 0_usize;
        let mut expect_item = true;
        let mut recovered = false;
        while cursor < $input.len() {
            if expect_item {
                if item_matches(cursor) {
                    item_count += 1;
                    expect_item = false;
                    cursor += 1;
                } else if $matches!($input, cursor, $separator) {
                    recovered = true;
                    expect_item = false;
                } else {
                    break;
                }
            } else if $matches!($input, cursor, $separator) {
                expect_item = true;
                cursor += 1;
            } else if item_matches(cursor) {
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
            recovered |= $crate::__syntax_trailing_needs_empty!($trailing, item_count, expect_item);
            if !recovered {
                Ok($sink.fixed(
                    $kind.to_raw(),
                    (0..$input.len()).map($crate::FactorySlot::Input),
                ))
            } else {
                let mut slots = Vec::with_capacity($input.len().max(minimum * 2));
                let mut cursor = 0;
                let mut count = 0_usize;
                let mut expect_item = true;
                while cursor < $input.len() {
                    if expect_item {
                        if item_matches(cursor) {
                            slots.push($crate::FactorySlot::Input(cursor));
                            count += 1;
                            expect_item = false;
                            cursor += 1;
                        } else {
                            slots.push($crate::FactorySlot::Missing);
                            expect_item = false;
                        }
                    } else if $matches!($input, cursor, $separator) {
                        slots.push($crate::FactorySlot::Input(cursor));
                        expect_item = true;
                        cursor += 1;
                    } else {
                        slots.push($crate::FactorySlot::Missing);
                        expect_item = true;
                    }
                }
                $crate::__syntax_push_trailing_empty!($trailing, slots, count, expect_item);
                while count < minimum {
                    if count > 0 {
                        slots.push($crate::FactorySlot::Missing);
                    }
                    slots.push($crate::FactorySlot::Missing);
                    count += 1;
                }
                Ok($sink.fixed($kind.to_raw(), slots))
            }
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_trailing_needs_empty {
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

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_push_trailing_empty {
    (forbidden, $slots:ident, $count:ident, $expect_item:ident) => {
        if $expect_item && !$slots.is_empty() {
            $slots.push($crate::FactorySlot::Missing);
        }
    };
    (optional, $slots:ident, $count:ident, $expect_item:ident) => {};
    (required, $slots:ident, $count:ident, $expect_item:ident) => {
        if !$expect_item && $count > 0 {
            $slots.push($crate::FactorySlot::Missing);
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __syntax_factory_arm {
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        valid; $($fields:tt)*) => {
        $crate::__syntax_fixed_node!(
            $matches, $kind, $input, $sink, $recovery; $($fields)*
        )
    };
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        constructed; $($fields:tt)*) => {
        $crate::__syntax_fixed_node!(
            $matches, $kind, $input, $sink, $recovery; $($fields)*
        )
    };
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        list; $field:ident: $cardinality:ident $matcher:tt
        $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        $crate::__syntax_list_node!(
            $matches, $kind, $input, $sink, $recovery,
            $cardinality, $matcher $([$($policy)*])?
        )
    };
    ($matches:ident, $kind:ident, $input:ident, $sink:ident, $recovery:expr,
        malformed; $($fields:tt)*) => {
        Ok($sink.raw_malformed($kind.to_raw()))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_syntax_factory {
    (
        $syntax_kind:ident, $factory:ident, $matches:ident, $recovery:expr,
        $category_accepts:ident;
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
        fn $category_accepts(category: Category, kind: Option<$syntax_kind>) -> bool {
            match category {
                $(Category::$family => matches!(
                    kind,
                    Some($syntax_kind::$bogus $(| $syntax_kind::$member)*)
                ),)*
            }
        }

        impl $crate::SyntaxFactory for $factory {
            fn make_syntax(
                &self,
                raw_kind: $crate::RawSyntaxKind,
                input: $crate::ParsedChildren<'_>,
                sink: &mut $crate::SyntaxTreeSink<'_>,
            ) -> Result<$crate::FactoryNode, $crate::BuildSyntaxTreeError> {
                let kind = $syntax_kind::from_raw(raw_kind).ok_or(
                    $crate::BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind },
                )?;
                match kind {
                    $($syntax_kind::$kind => $crate::__syntax_factory_arm!(
                        $matches, kind, input, sink, $recovery, $class; $($fields)*
                    ),)*
                    $($syntax_kind::$bogus => Ok(sink.raw_malformed(raw_kind)),)*
                    _ => Err($crate::BuildSyntaxTreeError::FactoryMismatch { kind: raw_kind }),
                }
            }
        }
    };
}
