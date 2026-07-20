use std::fmt::{Debug, Write};
use std::marker::PhantomData;

use jolt_syntax::{Language, SyntaxElement, SyntaxNode, SyntaxSlot};

/// Result of checking a represented node against the physical slots generated
/// from its language schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalNodeAudit {
    Exact,
    MissingRequired,
    Unexpected,
    Malformed,
}

/// Generates the mechanical physical-slot audit from a language schema.
/// Languages provide only their matcher and malformed-child policy.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_physical_schema_audit {
    (
        kind: $kind:ident,
        language: $language:path,
        matches: $matches:ident,
        accepts_malformed: $accepts_malformed:expr,
        visibility: $visibility:vis,
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($node_kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        $visibility fn audit_physical_node(
            node: $crate::__private::SyntaxNode<'_, $language>,
        ) -> $crate::PhysicalNodeAudit {
            match node.kind() {
                $($kind::$node_kind => $crate::__physical_node_audit!(
                    $matches, $accepts_malformed, node, $class; $($fields)*
                ),)*
                $($kind::$bogus => $crate::PhysicalNodeAudit::Malformed,)*
                _ => $crate::PhysicalNodeAudit::Unexpected,
            }
        }

        $visibility fn is_required_slot(
            node: $crate::__private::SyntaxNode<'_, $language>,
            slot: usize,
        ) -> bool {
            match node.kind() {
                $($kind::$node_kind => $crate::__physical_required_slot!(slot, $class; $($fields)*),)*
                $($kind::$bogus => false,)*
                _ => false,
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __physical_node_audit {
    ($matches:ident, $accepts_malformed:expr, $node:ident, valid;
        $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        #[allow(unused_mut)]
        let mut missing = false;
        $(
            let Some(slot) = $node.slot_at(cursor) else {
                return $crate::PhysicalNodeAudit::Unexpected;
            };
            $crate::__physical_fixed_field!(
                $matches, $accepts_malformed, slot, missing, $cardinality, $matcher
            );
            cursor += 1;
        )*
        if cursor != $node.slot_count() {
            $crate::PhysicalNodeAudit::Unexpected
        } else if missing {
            $crate::PhysicalNodeAudit::MissingRequired
        } else {
            $crate::PhysicalNodeAudit::Exact
        }
    }};
    ($matches:ident, $accepts_malformed:expr, $node:ident, constructed; $($fields:tt)*) => {
        $crate::__physical_node_audit!(
            $matches, $accepts_malformed, $node, valid; $($fields)*
        )
    };
    ($matches:ident, $accepts_malformed:expr, $node:ident, list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)?;) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical list slot");
            match slot {
                $crate::__private::SyntaxSlot::Empty => missing = true,
                _ if ($accepts_malformed)(slot) || $matches!(slot, $matcher) => {}
                _ => return $crate::PhysicalNodeAudit::Unexpected,
            }
        }
        if missing {
            $crate::PhysicalNodeAudit::MissingRequired
        } else {
            $crate::PhysicalNodeAudit::Exact
        }
    }};
    ($matches:ident, $accepts_malformed:expr, $node:ident, list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? [disambiguate $policy:ident];) => {
        $crate::__physical_node_audit!(
            $matches, $accepts_malformed, $node, list;
            $field: $cardinality $matcher $(=> $role)?;
        )
    };
    ($matches:ident, $accepts_malformed:expr, $node:ident, list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)?
        [separated $separator:tt, minimum $minimum:literal, trailing $trailing:ident, recovery bogus_owner];) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical separated-list slot");
            if matches!(slot, $crate::__private::SyntaxSlot::Empty) {
                missing = true;
            } else if index % 2 == 0 {
                if !($accepts_malformed)(slot) && !$matches!(slot, $matcher) {
                    return $crate::PhysicalNodeAudit::Unexpected;
                }
            } else if !$matches!(slot, $separator) {
                return $crate::PhysicalNodeAudit::Unexpected;
            }
        }
        if missing {
            $crate::PhysicalNodeAudit::MissingRequired
        } else {
            $crate::PhysicalNodeAudit::Exact
        }
    }};
    ($matches:ident, $accepts_malformed:expr, $node:ident, malformed; $($fields:tt)*) => {
        $crate::PhysicalNodeAudit::Malformed
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __physical_fixed_field {
    ($matches:ident, $accepts_malformed:expr, $slot:ident, $missing:ident, required, $matcher:tt) => {
        match $slot {
            $crate::__private::SyntaxSlot::Empty => $missing = true,
            _ if ($accepts_malformed)($slot) || $matches!($slot, $matcher) => {}
            _ => return $crate::PhysicalNodeAudit::Unexpected,
        }
    };
    ($matches:ident, $accepts_malformed:expr, $slot:ident, $missing:ident, optional, $matcher:tt) => {
        match $slot {
            $crate::__private::SyntaxSlot::Empty => {}
            _ if $matches!($slot, $matcher) => {}
            _ => return $crate::PhysicalNodeAudit::Unexpected,
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __physical_required_slot {
    ($slot:ident, valid;
        $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {
        [$(matches!(stringify!($cardinality), "required")),*]
            .get($slot)
            .copied()
            .unwrap_or(false)
    };
    ($slot:ident, constructed; $($fields:tt)*) => {
        $crate::__physical_required_slot!($slot, valid; $($fields)*)
    };
    ($slot:ident, list; $($fields:tt)*) => { true };
    ($slot:ident, malformed; $($fields:tt)*) => { false };
}

/// Corpus audit of the production physical tree. Language crates generate the
/// node check directly from the same schema invocation as their factory and
/// typed accessors; there is no second schema interpreter or reconstructed
/// child stream.
pub struct SchemaAudit<K> {
    language: &'static str,
    clean_missing: Vec<String>,
    diagnostic_missing: Vec<String>,
    clean_unexpected: Vec<String>,
    diagnostic_unexpected: Vec<String>,
    kind: PhantomData<K>,
}

impl<K: Copy + Eq + Debug> SchemaAudit<K> {
    #[must_use]
    pub fn new(language: &'static str) -> Self {
        Self {
            language,
            clean_missing: Vec::new(),
            diagnostic_missing: Vec::new(),
            clean_unexpected: Vec::new(),
            diagnostic_unexpected: Vec::new(),
            kind: PhantomData,
        }
    }

    pub fn visit<L>(
        &mut self,
        label: &str,
        root: SyntaxNode<'_, L>,
        has_diagnostics: bool,
        audit_node: impl Copy + Fn(SyntaxNode<'_, L>) -> PhysicalNodeAudit,
    ) where
        L: Language<Kind = K>,
    {
        self.visit_node(label, root, has_diagnostics, &mut Vec::new(), audit_node);
    }

    fn visit_node<L>(
        &mut self,
        label: &str,
        node: SyntaxNode<'_, L>,
        has_diagnostics: bool,
        occurrences: &mut Vec<(K, usize)>,
        audit_node: impl Copy + Fn(SyntaxNode<'_, L>) -> PhysicalNodeAudit,
    ) where
        L: Language<Kind = K>,
    {
        let occurrence = if let Some((_, count)) = occurrences
            .iter_mut()
            .find(|(kind, _)| *kind == node.kind())
        {
            *count += 1;
            *count
        } else {
            occurrences.push((node.kind(), 1));
            1
        };
        let node_label = format!("{label}: {:?} occurrence {occurrence}", node.kind());
        for index in 0..node.slot_count() {
            if let Some(SyntaxSlot::Node(child)) = node.slot_at(index) {
                assert_eq!(
                    child.parent(),
                    Some(node),
                    "incorrect physical parent for {node_label} slot {index}"
                );
                assert_eq!(
                    child.index(),
                    index,
                    "incorrect physical parent slot for {node_label} slot {index}"
                );
            }
        }
        let children = node.children_with_tokens().collect::<Vec<_>>();
        let expected = audit_node(node);
        let directly_malformed = node.is_directly_malformed();
        let declared_malformed = expected == PhysicalNodeAudit::Malformed;
        let result = match (directly_malformed, declared_malformed) {
            (false, false) => Some(expected),
            (true, _) => Some(PhysicalNodeAudit::Malformed),
            (false, true) => {
                unexpected_for(self, has_diagnostics).push(format!(
                    "{node_label} has malformed kind without direct malformed ownership ({})",
                    render_children(&children)
                ));
                None
            }
        };

        match result {
            Some(PhysicalNodeAudit::Exact | PhysicalNodeAudit::Malformed) | None => {}
            Some(PhysicalNodeAudit::MissingRequired) => {
                missing_for(self, has_diagnostics)
                    .push(format!("{node_label} ({})", render_children(&children)));
            }
            Some(PhysicalNodeAudit::Unexpected) => {
                unexpected_for(self, has_diagnostics)
                    .push(format!("{node_label} ({})", render_children(&children)));
            }
        }
        for child in node.children() {
            self.visit_node(label, child, has_diagnostics, occurrences, audit_node);
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        assert!(
            self.clean_missing.is_empty() && self.clean_unexpected.is_empty(),
            "{} clean corpus violates its declared schema:\nmissing:\n{}\nunexpected:\n{}",
            self.language,
            self.clean_missing.join("\n"),
            self.clean_unexpected.join("\n"),
        );
        assert!(
            self.diagnostic_unexpected.is_empty(),
            "{} diagnostic corpus has unexpected physical shapes:\n{}",
            self.language,
            self.diagnostic_unexpected.join("\n"),
        );

        let mut output = String::new();
        for item in &self.diagnostic_missing {
            writeln!(output, "{item}").unwrap();
        }
        output
    }
}

fn missing_for<K>(audit: &mut SchemaAudit<K>, has_diagnostics: bool) -> &mut Vec<String> {
    if has_diagnostics {
        &mut audit.diagnostic_missing
    } else {
        &mut audit.clean_missing
    }
}

fn unexpected_for<K>(audit: &mut SchemaAudit<K>, has_diagnostics: bool) -> &mut Vec<String> {
    if has_diagnostics {
        &mut audit.diagnostic_unexpected
    } else {
        &mut audit.clean_unexpected
    }
}

fn render_children<L: Language>(children: &[SyntaxElement<'_, L>]) -> String
where
    L::Kind: Debug,
{
    children
        .iter()
        .map(|child| match child {
            SyntaxElement::Node(node) => format!("{:?}", node.kind()),
            SyntaxElement::Token(token) => format!("{:?}({:?})", token.kind(), token.text()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
