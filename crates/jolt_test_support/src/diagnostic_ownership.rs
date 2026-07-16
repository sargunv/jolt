use std::collections::HashMap;
use std::fmt::{Debug, Display};

use jolt_diagnostics::Diagnostic;
use jolt_syntax::{Language, SyntaxDiagnosticOwner, SyntaxNode, SyntaxNodeId, SyntaxSlot};

/// Proves bidirectional ownership between structural diagnostics and invalid
/// nodes: every owner targets a declared invalid shape, and every directly
/// malformed or required-incomplete node has an exact, non-duplicate owner.
pub fn assert_exact_structural_ownership<L>(
    root: SyntaxNode<'_, L>,
    diagnostics: &[Diagnostic],
    owners: &[Option<SyntaxDiagnosticOwner>],
    is_required_slot: impl Copy + Fn(SyntaxNode<'_, L>, usize) -> bool,
    context: impl Display,
) where
    L: Language,
    L::Kind: Debug,
{
    assert_exact_structural_ownership_requiring(
        root,
        diagnostics,
        owners,
        is_required_slot,
        |_| false,
        context,
    );
}

/// Extends [`assert_exact_structural_ownership`] by requiring every diagnostic
/// selected by `requires_owner` to name an exact structural owner.
pub fn assert_exact_structural_ownership_requiring<L>(
    root: SyntaxNode<'_, L>,
    diagnostics: &[Diagnostic],
    owners: &[Option<SyntaxDiagnosticOwner>],
    is_required_slot: impl Copy + Fn(SyntaxNode<'_, L>, usize) -> bool,
    requires_owner: impl Copy + Fn(&Diagnostic) -> bool,
    context: impl Display,
) where
    L: Language,
    L::Kind: Debug,
{
    assert_eq!(
        owners.len(),
        diagnostics.len(),
        "diagnostic owner count changed in {context}"
    );
    for (diagnostic, owner) in diagnostics.iter().zip(owners) {
        assert!(
            !requires_owner(diagnostic) || owner.is_some(),
            "structural diagnostic has no exact owner in {context}: {diagnostic:#?}"
        );
    }
    let mut owner_counts = HashMap::<(SyntaxNodeId, Option<u16>), usize>::new();
    for owner in owners.iter().flatten() {
        *owner_counts
            .entry((owner.node(), owner.slot()))
            .or_default() += 1;
    }

    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let mut has_required_empty = false;
        let mut has_required_empty_owner = false;
        for slot in 0..node.slot_count() {
            if is_required_slot(node, slot) && matches!(node.slot_at(slot), Some(SyntaxSlot::Empty))
            {
                has_required_empty = true;
                let slot = u16::try_from(slot).expect("syntax slot must fit in u16");
                if let Some(count) = owner_counts.remove(&(node.id(), Some(slot))) {
                    assert_eq!(
                        count, 1,
                        "schema-required empty slot has duplicate diagnostic owners in \
                         {context}: {node:#?}; slot={slot}"
                    );
                    has_required_empty_owner = true;
                }
            }
        }
        let has_node_owner = if node.is_directly_malformed() {
            let has_node_owner = owner_counts
                .remove(&(node.id(), None))
                .is_some_and(|count| count > 0);
            assert!(
                has_node_owner || has_required_empty_owner,
                "directly malformed node must have an exact node or required-empty-slot \
                 diagnostic owner in {context}: {node:#?}"
            );
            has_node_owner
        } else {
            false
        };
        assert!(
            !has_required_empty || has_required_empty_owner || has_node_owner,
            "node with schema-required empty shape has no exact diagnostic owner in \
             {context}: {node:#?}; owners={:?}; slots={:?}",
            owner_counts
                .iter()
                .filter(|((owner, _), _)| *owner == node.id())
                .collect::<Vec<_>>(),
            (0..node.slot_count())
                .map(|slot| {
                    (
                        slot,
                        is_required_slot(node, slot),
                        matches!(node.slot_at(slot), Some(SyntaxSlot::Empty)),
                    )
                })
                .collect::<Vec<_>>()
        );
        nodes.extend(node.children());
    }
    assert!(
        owner_counts.is_empty(),
        "structural diagnostic owner does not match a directly malformed node or \
         schema-required empty slot in {context}: {owner_counts:?}"
    );
}
