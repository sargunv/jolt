//! TEMPORARY: the stand-in for unported rules, deleted at integration.

use std::cell::RefCell;
use std::collections::BTreeSet;

use jolt_fmt::Doc;
use jolt_java_syntax::JavaSyntaxKind;
use jolt_syntax::SyntaxNode;

use crate::JavaFormatter;

thread_local! {
    static KINDS: RefCell<BTreeSet<&'static str>> = const { RefCell::new(BTreeSet::new()) };
}

/// Prints an unported node verbatim (`node()` already prints its leading and
/// trailing comments) and records its kind for `jref`.
pub(crate) fn placeholder<'a>(f: &mut JavaFormatter<'a>, node: SyntaxNode<'a>) -> Doc {
    KINDS.with(|kinds| {
        kinds
            .borrow_mut()
            .insert(JavaSyntaxKind::from_raw(node.kind()).name());
    });
    f.verbatim(node)
}

/// The placeholder kinds formatted on this thread since the last call.
#[must_use]
pub fn take_placeholder_kinds() -> Vec<&'static str> {
    KINDS.with(|kinds| {
        std::mem::take(&mut *kinds.borrow_mut())
            .into_iter()
            .collect()
    })
}
