use jolt_syntax::{
    BuildSyntaxTreeError, FactoryNode, FactorySlot, ParsedChildren, RawSyntaxKind, SyntaxFactory,
    SyntaxTreeSink,
};

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
macro_rules! java_audit_fixed_field {
    ($slot:ident, $missing:ident, required, $matcher:tt) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Empty => $missing = true,
            jolt_syntax::SyntaxSlot::Node(node) if node.is_directly_malformed() => {}
            _ if java_audit_matches!($slot, $matcher) => {}
            _ => return jolt_test_support::PhysicalNodeAudit::Unexpected,
        }
    };
    ($slot:ident, $missing:ident, optional, $matcher:tt) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Empty => {}
            _ if java_audit_matches!($slot, $matcher) => {}
            _ => return jolt_test_support::PhysicalNodeAudit::Unexpected,
        }
    };
}

#[cfg(test)]
macro_rules! java_audit_node {
    ($node:ident, valid; $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        #[allow(unused_mut)]
        let mut missing = false;
        $(
            let Some(slot) = $node.slot_at(cursor) else {
                return jolt_test_support::PhysicalNodeAudit::Unexpected;
            };
            java_audit_fixed_field!(slot, missing, $cardinality, $matcher);
            cursor += 1;
        )*
        if cursor != $node.slot_count() {
            jolt_test_support::PhysicalNodeAudit::Unexpected
        } else if missing {
            jolt_test_support::PhysicalNodeAudit::MissingRequired
        } else {
            jolt_test_support::PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, constructed; $($fields:tt)*) => {
        java_audit_node!($node, valid; $($fields)*)
    };
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)?;) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical list slot");
            match slot {
                jolt_syntax::SyntaxSlot::Empty => missing = true,
                jolt_syntax::SyntaxSlot::Node(child) if child.is_directly_malformed() => {}
                _ if java_audit_matches!(slot, $matcher) => {}
                _ => return jolt_test_support::PhysicalNodeAudit::Unexpected,
            }
        }
        if missing {
            jolt_test_support::PhysicalNodeAudit::MissingRequired
        } else {
            jolt_test_support::PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? [disambiguate $policy:ident];) => {
        java_audit_node!($node, list; $field: $cardinality $matcher $(=> $role)?;)
    };
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? [separated $separator:tt, minimum $minimum:literal, trailing $trailing:ident, recovery bogus_owner];) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical separated-list slot");
            if matches!(slot, jolt_syntax::SyntaxSlot::Empty) {
                missing = true;
            } else if index % 2 == 0 {
                if !matches!(slot, jolt_syntax::SyntaxSlot::Node(child) if child.is_directly_malformed())
                    && !java_audit_matches!(slot, $matcher)
                {
                    return jolt_test_support::PhysicalNodeAudit::Unexpected;
                }
            } else if !java_audit_matches!(slot, $separator) {
                return jolt_test_support::PhysicalNodeAudit::Unexpected;
            }
        }
        if missing {
            jolt_test_support::PhysicalNodeAudit::MissingRequired
        } else {
            jolt_test_support::PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, malformed; $($fields:tt)*) => {
        jolt_test_support::PhysicalNodeAudit::Malformed
    };
}

#[cfg(test)]
macro_rules! define_java_physical_audit {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })*
        }
    ) => {
        pub(crate) fn audit_physical_node(
            node: jolt_syntax::SyntaxNode<'_, crate::JavaLanguage>,
        ) -> jolt_test_support::PhysicalNodeAudit {
            match node.kind() {
                $(JavaSyntaxKind::$kind => java_audit_node!(node, $class; $($fields)*),)*
                $(JavaSyntaxKind::$bogus => jolt_test_support::PhysicalNodeAudit::Malformed,)*
                _ => jolt_test_support::PhysicalNodeAudit::Unexpected,
            }
        }
    };
}

#[cfg(test)]
java_syntax_schema!(define_java_physical_audit);
