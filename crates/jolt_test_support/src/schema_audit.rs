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

/// Corpus audit of the production physical tree. Language crates generate the
/// node check directly from the same schema invocation as their factory and
/// typed accessors; there is no second schema interpreter or reconstructed
/// child stream.
pub struct SchemaAudit<K> {
    language: &'static str,
    files: usize,
    diagnostic_files: usize,
    nodes: usize,
    exact: usize,
    malformed: usize,
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
            files: 0,
            diagnostic_files: 0,
            nodes: 0,
            exact: 0,
            malformed: 0,
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
        self.files += 1;
        self.diagnostic_files += usize::from(has_diagnostics);
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
        self.nodes += 1;
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
            (false, false) | (true, true) => Some(expected),
            (true, false) => {
                let range = node.text_range();
                unexpected_for(self, has_diagnostics).push(format!(
                    "{node_label} has valid kind with direct malformed ownership \
                     [bytes={}, tokens={}, children={}] ({})",
                    range.len().get(),
                    node.tokens().count(),
                    children.len(),
                    render_children(&children)
                ));
                None
            }
            (false, true) => {
                unexpected_for(self, has_diagnostics).push(format!(
                    "{node_label} has malformed kind without direct malformed ownership ({})",
                    render_children(&children)
                ));
                None
            }
        };

        match result {
            Some(PhysicalNodeAudit::Exact) => self.exact += 1,
            Some(PhysicalNodeAudit::Malformed) => self.malformed += 1,
            Some(PhysicalNodeAudit::MissingRequired) => {
                missing_for(self, has_diagnostics)
                    .push(format!("{node_label} ({})", render_children(&children)));
            }
            Some(PhysicalNodeAudit::Unexpected) => {
                unexpected_for(self, has_diagnostics)
                    .push(format!("{node_label} ({})", render_children(&children)));
            }
            None => {}
        }
        for child in node.children() {
            self.visit_node(label, child, has_diagnostics, occurrences, audit_node);
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(output, "language = {}", self.language).unwrap();
        for (name, count) in [
            ("fixture_files", self.files),
            ("diagnostic_fixture_files", self.diagnostic_files),
            ("audited_nodes", self.nodes),
            ("exact_valid_shapes", self.exact),
            ("malformed_nodes", self.malformed),
            ("clean_missing_required_shapes", self.clean_missing.len()),
            (
                "diagnostic_missing_required_shapes",
                self.diagnostic_missing.len(),
            ),
            ("clean_unexpected_shapes", self.clean_unexpected.len()),
            (
                "diagnostic_unexpected_shapes",
                self.diagnostic_unexpected.len(),
            ),
        ] {
            writeln!(output, "{name} = {count}").unwrap();
        }
        for (section, items) in [
            ("clean_missing_required", &self.clean_missing),
            ("diagnostic_missing_required", &self.diagnostic_missing),
            ("clean_unexpected", &self.clean_unexpected),
            ("diagnostic_unexpected", &self.diagnostic_unexpected),
        ] {
            writeln!(output, "\n[{section}]").unwrap();
            for item in items {
                writeln!(output, "{item}").unwrap();
            }
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
