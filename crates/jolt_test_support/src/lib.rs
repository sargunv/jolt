#![allow(clippy::missing_panics_doc)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Write as _};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId};
use jolt_fmt_ir::{FormatOptions, FormatSinkResult, RenderControl, RenderSink};
use jolt_syntax::{
    CommentKind, Language, SyntaxDiagnosticOwner, SyntaxNode, SyntaxSlot, SyntaxToken,
};
use unicode_width::UnicodeWidthStr;

mod diagnostic_ownership;
mod schema_audit;

pub use diagnostic_ownership::assert_exact_structural_ownership_requiring;
pub use schema_audit::{PhysicalNodeAudit, SchemaAudit};

#[doc(hidden)]
pub mod __private {
    pub use jolt_syntax::{SyntaxNode, SyntaxSlot};
}

/// Inventories parser diagnostic classification without unstable source ranges.
#[must_use]
pub fn diagnostic_inventory(diagnostics: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut inventory = BTreeMap::new();
    for diagnostic in diagnostics {
        let key = format!(
            "{:?}:{:?}:{}:{}",
            diagnostic.stage,
            diagnostic.severity,
            diagnostic.code.as_str(),
            diagnostic.message
        );
        *inventory.entry(key).or_default() += 1;
    }
    inventory
}

pub fn assert_bidirectional_diagnostic_ownership<L>(
    root: SyntaxNode<'_, L>,
    diagnostics: &[Diagnostic],
    owners: &[Option<SyntaxDiagnosticOwner>],
    requires_owner: impl Fn(&Diagnostic) -> bool,
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
    let mut nodes = vec![root];
    let mut cursor = 0;
    while let Some(node) = nodes.get(cursor).copied() {
        nodes.extend(node.children());
        cursor += 1;
    }
    let nodes_by_id = nodes
        .iter()
        .copied()
        .map(|node| (node.id(), node))
        .collect::<HashMap<_, _>>();
    let mut owned_nodes = HashSet::new();
    for (diagnostic, owner) in diagnostics.iter().zip(owners) {
        let Some(owner) = owner else {
            assert!(
                !requires_owner(diagnostic),
                "unowned structural diagnostic in {context}: {diagnostic:?}"
            );
            continue;
        };
        let node = nodes_by_id
            .get(&owner.node())
            .unwrap_or_else(|| panic!("unreachable diagnostic owner in {context}: {diagnostic:?}"));
        if let Some(slot) = owner.slot() {
            assert!(
                matches!(node.slot_at(slot as usize), Some(SyntaxSlot::Empty)),
                "diagnostic owner is not an empty slot in {context}: {diagnostic:?}; owner={owner:?}; node={node:#?}"
            );
        }
        owned_nodes.insert(owner.node());
    }
    for node in nodes {
        if node.is_directly_malformed() {
            assert!(
                owned_nodes.contains(&node.id()),
                "directly malformed node has no diagnostic owner in {context}: {node:#?}"
            );
        }
    }
}

pub fn assert_exact_diagnostic_owner<L>(
    root: SyntaxNode<'_, L>,
    diagnostics: &[Diagnostic],
    owners: &[Option<SyntaxDiagnosticOwner>],
    code: DiagnosticCodeId,
    message: &str,
    kind: L::Kind,
    slot: Option<u16>,
) where
    L: Language,
    L::Kind: Debug,
{
    assert_eq!(owners.len(), diagnostics.len());
    let (index, diagnostic) = diagnostics
        .iter()
        .enumerate()
        .find(|(_, diagnostic)| diagnostic.code == code && diagnostic.message == message)
        .unwrap_or_else(|| panic!("missing diagnostic {code} {message:?}"));
    let owner = owners[index].unwrap_or_else(|| panic!("unowned diagnostic: {diagnostic:?}"));
    let mut nodes = vec![root];
    let mut cursor = 0;
    while let Some(node) = nodes.get(cursor).copied() {
        nodes.extend(node.children());
        cursor += 1;
    }
    let node = nodes
        .into_iter()
        .find(|node| node.id() == owner.node())
        .unwrap_or_else(|| panic!("owner node is not reachable: {diagnostic:?}"));
    assert_eq!((node.kind(), owner.slot()), (kind, slot));
    if let Some(slot) = slot {
        assert!(matches!(
            node.slot_at(slot as usize),
            Some(SyntaxSlot::Empty)
        ));
    }
}

#[derive(Default)]
pub struct StringSink {
    text: String,
}

impl StringSink {
    #[must_use]
    pub fn into_string(self) -> String {
        self.text
    }
}

impl RenderSink for StringSink {
    fn write_str(&mut self, text: &str) -> RenderControl {
        self.text.push_str(text);
        RenderControl::Continue
    }
}

#[derive(Default)]
pub struct SnapshotBuilder {
    output: String,
}

impl SnapshotBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn section(mut self, name: &str, content: impl AsRef<str>) -> Self {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        writeln!(&mut self.output, "{name}:").expect("write snapshot section header");
        let content = content.as_ref();
        if content.is_empty() {
            self.output.push_str("<empty>\n");
        } else {
            self.output.push_str(content);
            if !content.ends_with('\n') {
                self.output.push('\n');
            }
        }
        self
    }

    #[must_use]
    pub fn finish(self) -> String {
        self.output
    }
}

#[must_use]
pub fn workspace_root(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir)
        .ancestors()
        .nth(2)
        .expect("crate manifest dir should be under workspace crates directory")
        .to_path_buf()
}

#[must_use]
pub fn java_fixture_root(manifest_dir: &str) -> PathBuf {
    workspace_root(manifest_dir).join("fixtures/java")
}

#[must_use]
pub fn kotlin_fixture_root(manifest_dir: &str) -> PathBuf {
    workspace_root(manifest_dir).join("fixtures/kotlin")
}

#[must_use]
pub fn collect_java_files(root: &Path) -> Vec<PathBuf> {
    collect_fixture_files(root, &["java"], "Java")
}

#[must_use]
pub fn collect_kotlin_files(root: &Path) -> Vec<PathBuf> {
    collect_fixture_files(root, &["kt", "kts"], "Kotlin")
}

#[must_use]
pub fn collect_fixture_files(root: &Path, extensions: &[&str], language: &str) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required {language} fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_fixture_files_into(root, extensions, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one {language} fixture under {}",
        root.display()
    );
    files
}

fn collect_fixture_files_into(root: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_fixture_files_into(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            files.push(path);
        }
    }
}

#[must_use]
pub fn fixture_snapshot_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or_else(|error| {
        panic!(
            "{} should be under {}: {error}",
            path.display(),
            root.display()
        )
    });
    let without_extension = relative.with_extension("");
    without_extension
        .components()
        .map(|component| component.as_os_str().to_string_lossy().replace('-', "_"))
        .collect::<Vec<_>>()
        .join("__")
}

#[must_use]
pub fn fixture_manifest(root: &Path, paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[must_use]
pub fn render_diagnostics(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "(none)\n".to_owned();
    }

    let mut output = String::new();
    for diagnostic in diagnostics {
        writeln!(
            &mut output,
            "code={} severity={:?} stage={:?} range={:?} message={}",
            diagnostic.code.as_str(),
            diagnostic.severity,
            diagnostic.stage,
            diagnostic.range,
            diagnostic.message
        )
        .expect("write diagnostics");
    }
    output
}

/// Collects `JOLT-TRIVIA:`-prefixed markers from `source` so fixture-driven
/// trivia conservation tests can compare counts before and after formatting.
#[must_use]
pub fn trivia_markers(source: &str) -> BTreeMap<String, usize> {
    let mut markers = BTreeMap::new();
    for (start, _) in source.match_indices("JOLT-TRIVIA:") {
        let marker = source[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
            .collect::<String>();
        *markers.entry(marker).or_insert(0) += 1;
    }
    markers
}

/// Inventories every represented comment by kind and a single global canonical
/// body that ignores formatter-controlled interior whitespace.
#[must_use]
pub fn represented_comment_inventory<'source, L>(
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
) -> BTreeMap<String, usize>
where
    L: Language,
{
    let mut comments = BTreeMap::new();
    for token in tokens {
        for comment in token.leading_comments().chain(token.trailing_comments()) {
            let key = comment_inventory_key(comment.kind(), comment.text());
            *comments.entry(key).or_default() += 1;
        }
    }
    comments
}

fn comment_inventory_key(kind: CommentKind, text: &str) -> String {
    format!("{kind:?}:{}", canonical_comment_text(text))
}

fn universal_lines(mut text: &str) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        if text.is_empty() {
            return None;
        }

        let Some(boundary) = text.bytes().position(|byte| matches!(byte, b'\r' | b'\n')) else {
            return Some(std::mem::take(&mut text));
        };
        let line = &text[..boundary];
        let boundary_len = if text.as_bytes()[boundary] == b'\r'
            && text.as_bytes().get(boundary + 1) == Some(&b'\n')
        {
            2
        } else {
            1
        };
        text = &text[boundary + boundary_len..];
        Some(line)
    })
}

fn canonical_comment_text(text: &str) -> String {
    let body = text
        .strip_prefix("//")
        .or_else(|| {
            text.strip_prefix("/**")
                .and_then(|text| text.strip_suffix("*/"))
        })
        .or_else(|| {
            text.strip_prefix("/*")
                .and_then(|text| text.strip_suffix("*/"))
        })
        .unwrap_or(text);
    let multiline = body.contains(['\r', '\n']);
    let mut canonical = String::new();
    for word in universal_lines(body)
        .flat_map(|line| {
            let line = line.trim();
            let line = if multiline {
                line.strip_prefix('*')
                    .map_or(line, |line| line.strip_prefix(' ').unwrap_or(line))
            } else {
                line
            };
            line.split_whitespace()
        })
        .filter(|word| !word.is_empty())
    {
        if !canonical.is_empty() {
            canonical.push(' ');
        }
        canonical.push_str(word);
    }
    canonical
}

#[cfg(test)]
mod tests {
    use super::comment_inventory_key;
    use jolt_syntax::CommentKind;

    #[test]
    fn canonical_comment_inventory_preserves_meaningful_stars_and_kind() {
        assert_ne!(
            comment_inventory_key(CommentKind::Block, "/* *bold* */"),
            comment_inventory_key(CommentKind::Block, "/* bold */")
        );
        assert_ne!(
            comment_inventory_key(CommentKind::Block, "/* same */"),
            comment_inventory_key(CommentKind::Doc, "/** same */")
        );
    }

    #[test]
    fn canonical_comment_inventory_ignores_multiline_decoration_and_whitespace() {
        assert_eq!(
            comment_inventory_key(CommentKind::Doc, "/**\n * hello   world\n */"),
            comment_inventory_key(CommentKind::Doc, "/** hello world */")
        );
    }

    #[test]
    fn canonical_comment_inventory_uses_universal_logical_lines() {
        let normalized = comment_inventory_key(
            CommentKind::Doc,
            "/**\n * hello\n *\n * universal world\n */",
        );
        assert_eq!(
            comment_inventory_key(
                CommentKind::Doc,
                "/**\r\n * hello\r *\r\n * universal world\n */",
            ),
            normalized
        );
        assert_eq!(
            comment_inventory_key(
                CommentKind::Doc,
                "/**\r * hello\r *\r * universal world\r */",
            ),
            normalized
        );
    }

    #[test]
    fn canonical_comment_inventory_preserves_doubled_stars_after_decoration() {
        assert_ne!(
            comment_inventory_key(CommentKind::Doc, "/**\r\n ** meaningful\r\n */"),
            comment_inventory_key(CommentKind::Doc, "/**\n * meaningful\n */")
        );
    }
}

/// Formats `source` with `format`, panicking on halt/block like corpus tests.
pub fn format_source_or_panic(
    format: impl FnOnce(&str, &FormatOptions, &mut StringSink) -> FormatSinkResult,
    source: &str,
    options: &FormatOptions,
    label: &str,
) -> String {
    let mut sink = StringSink::default();
    match format(source, options, &mut sink) {
        FormatSinkResult::Complete => sink.into_string(),
        FormatSinkResult::Halted => {
            panic!("formatter unexpectedly halted with StringSink for {label}")
        }
        FormatSinkResult::Blocked { diagnostic } => {
            panic!("formatter blocked for {label}: {diagnostic:#?}")
        }
    }
}

/// Runs the shared trivia conservation assertion flow over `files`:
/// each fixture must contain at least one `JOLT-TRIVIA:` marker, must parse
/// cleanly via `parse`, and must format idempotently while conserving markers
/// via `format`. `parse` and `format` should panic on diagnostic failure,
/// matching the per-crate test expectations.
pub fn assert_trivia_markers_conserved(
    files: &[PathBuf],
    parse: impl Fn(&str, &Path),
    format: impl Fn(&str, &Path) -> String,
) {
    for path in files {
        let source = read_to_string(path);
        let expected = trivia_markers(&source);
        assert!(
            !expected.is_empty(),
            "expected trivia fixture to contain at least one marker: {}",
            path.display()
        );
        parse(&source, path);
        let formatted = format(&source, path);
        assert_eq!(
            trivia_markers(&formatted),
            expected,
            "formatter must conserve trivia markers in {}",
            path.display()
        );
        let formatted_again = format(&formatted, path);
        assert_eq!(
            formatted_again,
            formatted,
            "formatter output must be idempotent for {}",
            path.display()
        );
    }
}

/// Owned parse facts for the shared formatter corpus harness.
///
/// Built inside each language's parse scope so the harness never holds borrowed
/// tokens across a second parse of formatted output.
#[derive(Clone, Debug)]
pub struct CorpusParseFacts {
    pub has_tree: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub comment_inventory: BTreeMap<String, usize>,
    pub structure: String,
}

/// Builds owned corpus facts from a language parse while its buffers are live.
#[must_use]
pub fn corpus_parse_facts<L>(
    root: Option<SyntaxNode<'_, L>>,
    diagnostics: &[Diagnostic],
    policy: &StructurePolicy<L::Kind>,
) -> CorpusParseFacts
where
    L: Language,
    L::Kind: Debug,
{
    CorpusParseFacts {
        has_tree: root.is_some(),
        diagnostics: diagnostics.to_vec(),
        comment_inventory: root
            .map(|root| represented_comment_inventory(root.tokens()))
            .unwrap_or_default(),
        structure: root
            .map(|root| structure_fingerprint(root, policy))
            .unwrap_or_default(),
    }
}

/// Which formatter-authorized normalizations a language's structure fingerprint
/// must tolerate.
///
/// The formatter is allowed to change a parse tree only through the closed
/// normalization vocabulary in `jolt_syntax` (`NormalizedToken`, `RemovalReason`,
/// `ReorderReason`). Everything a language actually normalizes is declared here so
/// the fingerprint stays blind to exactly those edits and sensitive to every other
/// tree change.
pub struct StructurePolicy<K: 'static> {
    /// Punctuation the formatter may insert or drop outright, mirroring
    /// `NormalizedToken::text` and `RemovalReason::Redundant*`. Structure still
    /// comes from the enclosing nodes, so eliding these tokens costs no coverage.
    pub normalizable_punctuation: &'static [K],
    /// Nodes whose child sequence is order-insensitive, covering
    /// `ReorderReason::ModuleDirectives` and Kotlin's sorted import list. Only
    /// recovery-free children are canonicalized, mirroring the authorization
    /// `NormalizationOwner` grants; a child carrying recovery keeps its position, so
    /// moving it past a sortable neighbour still fails.
    pub unordered_nodes: &'static [K],
    /// Nodes whose keyword tokens are order-insensitive, covering
    /// `ReorderReason::Modifiers`. Sorted keywords are emitted ahead of the child
    /// nodes so that moving a keyword across an annotation is tolerated. Child
    /// nodes keep their relative order except for the kinds
    /// `unordered_keywords_ordered_children` lists.
    pub unordered_keywords: &'static [K],
    /// Child kinds of an `unordered_keywords` node that may cross the node's
    /// other children but keep declaration order among themselves, covering an
    /// annotation the formatter moves across a node-shaped modifier like
    /// `non-sealed`. Canonicalization is a stable partition that emits these
    /// children ahead of the rest, never a sort by rendering, so a swap of two
    /// of them -- repeated annotations, which reflection exposes in declaration
    /// order -- still fails. A child carrying recovery stays with the node's
    /// other children, mirroring the authorization `NormalizationOwner` grants.
    pub unordered_keywords_ordered_children: &'static [K],
    /// Child kinds the formatter may reorder among their own positions inside an
    /// otherwise order-sensitive parent, covering `ReorderReason::Imports` where
    /// imports share a list with other declarations. As with `unordered_nodes`, a
    /// child carrying recovery is never canonicalized.
    pub reorderable_children: &'static [K],
    /// Single-child wrappers that are pure plumbing: they carry no source delimiter of
    /// their own, so eliding them cannot hide a lost boundary.
    pub elidable_wrappers: &'static [K],
    /// The wrapper a promoted statement body is placed in, covering
    /// `NormalizedToken::OpenBlockBrace`. Transparent *only* directly under
    /// `brace_promoting_parents`, so a block represented anywhere else stays visible
    /// and dropping its real braces still fails.
    pub promoted_body_wrapper: Option<K>,
    /// Parents whose body the formatter may promote from a bare statement to a block.
    pub brace_promoting_parents: &'static [K],
    /// Nodes that carry no meaning and may be dropped, covering
    /// `RemovalReason::RedundantSeparator` for empty statements.
    pub elidable_nodes: &'static [K],
}

/// Reports where two structure fingerprints first diverge.
///
/// Whole fingerprints are far too large to read, so this trims the shared prefix
/// and suffix and shows a window around the first difference.
fn describe_structure_divergence(expected: &str, actual: &str) -> String {
    const WINDOW: usize = 160;

    let prefix = expected
        .char_indices()
        .zip(actual.chars())
        .find(|((_, left), right)| left != right)
        .map_or_else(|| expected.len().min(actual.len()), |((index, _), _)| index);
    let window = |text: &str| {
        let start = text
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|&index| index <= prefix.saturating_sub(WINDOW / 2))
            .last()
            .unwrap_or(0);
        let end = text
            .char_indices()
            .map(|(index, _)| index)
            .find(|&index| index >= prefix + WINDOW)
            .unwrap_or(text.len());
        format!(
            "{}{}{}",
            if start > 0 { "..." } else { "" },
            &text[start..end],
            if end < text.len() { "..." } else { "" }
        )
    };
    format!(
        "first divergence at byte {prefix}\nexpected: {}\nactual:   {}",
        window(expected),
        window(actual)
    )
}

/// Renders a canonical, trivia-insensitive fingerprint of one parse tree.
///
/// Two parses share a fingerprint exactly when they have the same node kinds,
/// nesting, significant tokens, and empty slots, up to the normalizations `policy`
/// declares. Comparing a fixture's fingerprint against its formatted output's
/// fingerprint asserts that formatting never changed what the program means.
#[must_use]
pub fn structure_fingerprint<L>(
    root: SyntaxNode<'_, L>,
    policy: &StructurePolicy<L::Kind>,
) -> String
where
    L: Language,
    L::Kind: Debug,
{
    let mut out = String::new();
    write_node_structure(&mut out, root, None, false, policy);
    out
}

/// True when `node` stands for no source delimiter of its own, so collapsing it to a
/// lone child cannot hide a boundary the formatter dropped.
fn is_transparent_wrapper<K: Copy + PartialEq>(
    kind: K,
    parent: Option<K>,
    policy: &StructurePolicy<K>,
) -> bool {
    policy.elidable_wrappers.contains(&kind)
        || policy.promoted_body_wrapper == Some(kind)
            && parent.is_some_and(|parent| policy.brace_promoting_parents.contains(&parent))
}

/// True when any direct child is a kind the formatter may reorder. Only inspects child
/// kinds and recovery ownership, so it never walks a subtree.
fn has_reorderable_child<L>(node: SyntaxNode<'_, L>, policy: &StructurePolicy<L::Kind>) -> bool
where
    L: Language,
{
    (0..node.slot_count()).any(|index| {
        matches!(node.slot_at(index), Some(SyntaxSlot::Node(child))
            if is_canonicalizable_child(child, policy.reorderable_children))
    })
}

/// True when `child` is one of `kinds` and carries no recovery.
///
/// A reorder claim is authorized only for a recovery-free owner, so an entry the parser
/// recovered inside stays where the source put it. Canonicalizing it too would let the
/// fingerprint tolerate moving a malformed entry past the sortable ones around it.
fn is_canonicalizable_child<L>(child: SyntaxNode<'_, L>, kinds: &[L::Kind]) -> bool
where
    L: Language,
{
    kinds.contains(&child.kind()) && child.is_recovery_free()
}

/// True when a node cannot be emitted until all of its children are known, because it
/// reorders them or may collapse to a lone child.
fn needs_child_buffer<L>(
    node: SyntaxNode<'_, L>,
    parent: Option<L::Kind>,
    policy: &StructurePolicy<L::Kind>,
) -> bool
where
    L: Language,
{
    let kind = node.kind();
    is_transparent_wrapper(kind, parent, policy)
        || policy.unordered_nodes.contains(&kind)
        || policy.unordered_keywords.contains(&kind)
        || has_reorderable_child(node, policy)
}

enum StructureStep<'tree, L: Language> {
    Enter {
        node: SyntaxNode<'tree, L>,
        parent: Option<L::Kind>,
        separated: bool,
    },
    Token(SyntaxToken<'tree, L>),
    Close,
}

/// Streams `node` into `out` with an explicit work stack.
///
/// Postfix chains and operator chains are built by parser loops, so their trees nest as
/// deeply as the source is long and a recursive walk overflows the stack. Only nodes
/// that reorder or collapse their children recurse, via `render_buffered_node`, and the
/// parser bounds how deeply those nest with `excessive_syntax_nesting`.
fn write_node_structure<L>(
    out: &mut String,
    node: SyntaxNode<'_, L>,
    parent: Option<L::Kind>,
    separated: bool,
    policy: &StructurePolicy<L::Kind>,
) where
    L: Language,
    L::Kind: Debug,
{
    let mut steps = vec![StructureStep::Enter {
        node,
        parent,
        separated,
    }];
    while let Some(step) = steps.pop() {
        match step {
            StructureStep::Close => out.push(')'),
            StructureStep::Token(token) => {
                write!(out, " {:?}={}", token.kind(), token.text()).expect("write token kind");
            }
            StructureStep::Enter {
                node,
                parent,
                separated,
            } => {
                let mark = out.len();
                if separated {
                    out.push(' ');
                }
                if needs_child_buffer(node, parent, policy) {
                    let rendered = render_buffered_node(node, parent, policy);
                    // A transparent wrapper left holding nothing is what a dropped
                    // redundant separator looks like, so drop its separator too.
                    if rendered.is_empty() {
                        out.truncate(mark);
                    } else {
                        out.push_str(&rendered);
                    }
                    continue;
                }

                let kind = node.kind();
                out.push('(');
                write!(out, "{kind:?}").expect("write node kind");
                steps.push(StructureStep::Close);
                for index in (0..node.slot_count()).rev() {
                    match node.slot_at(index) {
                        Some(SyntaxSlot::Node(child)) => {
                            if policy.elidable_nodes.contains(&child.kind()) {
                                continue;
                            }
                            steps.push(StructureStep::Enter {
                                node: child,
                                parent: Some(kind),
                                separated: true,
                            });
                        }
                        Some(SyntaxSlot::Token(token)) => {
                            if policy.normalizable_punctuation.contains(&token.kind()) {
                                continue;
                            }
                            steps.push(StructureStep::Token(token));
                        }
                        // An empty slot is indistinguishable from a slot holding
                        // punctuation the policy elides, so recording it would make the
                        // two spellings disagree. Dropping every empty slot keeps the
                        // fingerprint slot-count insensitive; a subtree the formatter
                        // loses still disappears from its parent.
                        Some(SyntaxSlot::Empty) | None => {}
                    }
                }
            }
        }
    }
}

/// Renders one node whose children must be reordered or collapsed.
fn render_buffered_node<L>(
    node: SyntaxNode<'_, L>,
    parent: Option<L::Kind>,
    policy: &StructurePolicy<L::Kind>,
) -> String
where
    L: Language,
    L::Kind: Debug,
{
    let kind = node.kind();
    let transparent = is_transparent_wrapper(kind, parent, policy);
    let unordered = policy.unordered_nodes.contains(&kind);
    let unordered_keywords = policy.unordered_keywords.contains(&kind);

    let mut keywords: Vec<String> = Vec::new();
    let mut slots: Vec<String> = Vec::new();
    // Parallel to `slots` in `unordered_keywords` mode, where only node
    // children reach `slots`: whether the child joins the declaration-order
    // partition (`unordered_keywords_ordered_children`).
    let mut slot_ordered: Vec<bool> = Vec::new();
    let mut reorderable: Vec<usize> = Vec::new();
    for index in 0..node.slot_count() {
        match node.slot_at(index) {
            Some(SyntaxSlot::Node(child)) => {
                if policy.elidable_nodes.contains(&child.kind()) {
                    continue;
                }
                let mut rendered = String::new();
                write_node_structure(&mut rendered, child, Some(kind), false, policy);
                if rendered.is_empty() {
                    continue;
                }
                let canonicalizable = if unordered {
                    child.is_recovery_free()
                } else {
                    is_canonicalizable_child(child, policy.reorderable_children)
                };
                if canonicalizable {
                    reorderable.push(slots.len());
                }
                slot_ordered.push(
                    unordered_keywords
                        && is_canonicalizable_child(
                            child,
                            policy.unordered_keywords_ordered_children,
                        ),
                );
                slots.push(rendered);
            }
            Some(SyntaxSlot::Token(token)) => {
                if policy.normalizable_punctuation.contains(&token.kind()) {
                    continue;
                }
                let rendered = format!("{:?}={}", token.kind(), token.text());
                if unordered_keywords {
                    keywords.push(rendered);
                } else {
                    // A token carries no recovery of its own, so an unordered list
                    // canonicalizes it alongside its recovery-free node children.
                    if unordered {
                        reorderable.push(slots.len());
                    }
                    slots.push(rendered);
                }
            }
            Some(SyntaxSlot::Empty) => {}
            None => break,
        }
    }

    if unordered_keywords {
        keywords.sort_unstable();
        // A listed child may cross the node's other children but keeps
        // declaration order among its own kind, so canonicalize with a stable
        // partition rather than a sort by rendering: two of them swapping
        // places still changes the fingerprint.
        let mut partitioned: Vec<(bool, String)> = slot_ordered.into_iter().zip(slots).collect();
        partitioned.sort_by_key(|&(ordered, _)| !ordered);
        slots = partitioned
            .into_iter()
            .map(|(_, rendered)| rendered)
            .collect();
        keywords.append(&mut slots);
        slots = keywords;
    } else if reorderable.len() > 1 {
        // Canonicalize the reorderable entries among the positions they already
        // occupy. Every other slot -- punctuation, an unrelated declaration, or an
        // entry the parser recovered inside -- keeps its index, so a formatter that
        // moved one of those still changes the fingerprint.
        let mut moved = reorderable
            .iter()
            .map(|&index| slots[index].clone())
            .collect::<Vec<_>>();
        moved.sort_unstable();
        for (&index, rendered) in reorderable.iter().zip(moved) {
            slots[index] = rendered;
        }
    }

    // A wrapper the formatter may synthesize around a lone child renders as that child,
    // so braced and unbraced spellings agree, and renders as nothing at all when it is
    // empty so the caller can drop it.
    if transparent && slots.len() <= 1 {
        return slots.into_iter().next().unwrap_or_default();
    }

    let mut out = String::new();
    out.push('(');
    write!(&mut out, "{kind:?}").expect("write node kind");
    for slot in slots {
        out.push(' ');
        out.push_str(&slot);
    }
    out.push(')');
    out
}

/// Language bindings for the shared formatter corpus / recovery harness.
///
/// The harness owns the fixture walk, audit-vs-format routing, conservation
/// checks, and snapshot orchestration. Implementors supply owned parse facts,
/// the format function, and language-specific classification policy.
pub trait CorpusLanguage {
    /// Human-readable language name used in harness assertion messages.
    fn language_name(&self) -> &'static str;

    /// End offsets of the represented source tokens, in source order, keeping
    /// only the positions where trivia may legally attach.
    ///
    /// A position inside a string literal is excluded: text inserted there is
    /// literal content, not a comment, so probing it tests nothing about comment
    /// handling while producing malformed source that can drive the parser into
    /// an allocation large enough to abort the process -- which no in-process
    /// check can recover from. Excluding those positions is what keeps the sweep
    /// bounded by construction.
    fn token_end_offsets(&self, source: &str) -> Vec<usize>;

    /// Parses one fixture source into owned conservation facts.
    fn parse_facts(&self, source: &str) -> CorpusParseFacts;

    /// Formats `source`, panicking on halt/block like the corpus tests expect.
    fn format(&self, source: &str, label: &str) -> String;

    /// Formats `source` at `options`, reporting a halt or block instead of
    /// panicking. The corpus sweep records those as failures rather than
    /// aborting the run, and needs the width to vary.
    ///
    /// # Errors
    ///
    /// Returns the reason the formatter halted or refused the source.
    fn try_format(&self, source: &str, options: &FormatOptions) -> Result<String, String>;

    /// True when a fixture at `relative` is expected to carry parser
    /// diagnostics, routing it through the audit path instead of the format
    /// snapshot path.
    fn expects_parser_diagnostics(&self, relative: &str) -> bool;
}

/// Which trivia slot a probe insertion lands in.
///
/// This follows from the lines the insertion occupies, and it is what decides
/// which formatter path the probe reaches. Text that stays on the boundary's own
/// line becomes trailing trivia of the token before it; text on a line of its
/// own becomes leading trivia of the token after it. A sweep of only the first
/// kind therefore cannot observe a leading-comment bug at all, however many
/// boundaries it covers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProbePosition {
    /// Trailing trivia of the token before the boundary.
    Trailing,
    /// Leading trivia of the token after the boundary.
    Leading,
}

/// One shape of comment the conservation sweep injects at a token boundary.
pub struct ProbeShape {
    /// Category prefix, and the name `<path>:<name><n>` uses to replay a probe.
    pub name: &'static str,
    /// Text inserted at the boundary.
    pub insert: &'static str,
    /// The trivia slot the insertion lands in.
    pub position: ProbePosition,
    /// Distinguishes this shape's sampled boundaries from the other shapes'.
    pub seed: usize,
}

/// The comment shapes the conservation sweep injects at token boundaries.
///
/// Both [`ProbePosition`] variants appear, in both comment spellings, because
/// only a line comment forces the line that follows it.
pub const PROBE_SHAPES: &[ProbeShape] = &[
    ProbeShape {
        name: "comment",
        insert: " /*hunt*/ ",
        position: ProbePosition::Trailing,
        seed: 2_654_435_761,
    },
    ProbeShape {
        name: "linecomment",
        insert: " //hunt\n",
        position: ProbePosition::Trailing,
        seed: 2_246_822_519,
    },
    ProbeShape {
        name: "ownline",
        insert: "\n/*hunt*/\n",
        position: ProbePosition::Leading,
        seed: 40_503,
    },
    ProbeShape {
        name: "ownlinecomment",
        insert: "\n//hunt\n",
        position: ProbePosition::Leading,
        seed: 374_761_393,
    },
];

/// What a corpus sweep run needs beyond its [`CorpusLanguage`].
pub struct CorpusSweep<'a> {
    /// Directories under `tools/import/.imports` to walk.
    pub suites: &'a [&'a str],
    /// File extensions to sweep.
    pub extensions: &'a [&'a str],
    /// Paths to skip, matched as a suffix.
    pub skip_suffixes: &'a [&'a str],
    /// Directory under `target/hunt` for the report and repros.
    pub out_dir: &'a str,
    /// Failure categories this corpus is already known to produce.
    pub known_open: &'a [&'a str],
}

/// Runs the conservation sweep over an imported corpus and writes a bounded
/// report, returning the process exit code.
///
/// Exits non-zero when a failure category is outside `known_open`, or when a
/// `known_open` entry no longer fires: a stale entry would let the bug it
/// records be forgotten.
///
/// # Panics
///
/// Panics if the output directory cannot be created or the report cannot be
/// written.
pub fn run_corpus_sweep<L: CorpusLanguage + Sync>(
    lang: &L,
    repo_root: &Path,
    sweep: &CorpusSweep<'_>,
) -> i32 {
    // Panics are caught per file and recorded; keep the output readable.
    std::panic::set_hook(Box::new(|_| {}));

    let imports = repo_root.join("tools/import/.imports");
    let mut files = Vec::new();
    for suite in sweep.suites {
        collect_corpus_files(&imports.join(suite), sweep.extensions, &mut files);
    }
    files.sort();
    files.retain(|path| {
        !sweep
            .skip_suffixes
            .iter()
            .any(|suffix| path.ends_with(suffix))
    });

    let out_dir = repo_root.join("target/hunt").join(sweep.out_dir);
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("create hunt output directory");

    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk_size = files.len().div_ceil(threads).max(1);
    let files: &[PathBuf] = &files;
    let (mut findings, mut slow) = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    let mut findings = SweepFindings::default();
                    let mut slow = Vec::new();
                    for path in chunk {
                        let started = std::time::Instant::now();
                        for failure in sweep_file(lang, path) {
                            findings.push(path, failure);
                        }
                        let elapsed = started.elapsed().as_millis();
                        if elapsed > 1000 {
                            slow.push((path.clone(), elapsed));
                        }
                    }
                    (findings, slow)
                })
            })
            .collect();
        let mut all = SweepFindings::default();
        let mut all_slow = Vec::new();
        for handle in handles {
            let (findings, slow) = handle.join().expect("sweep worker panicked");
            all.merge(findings);
            all_slow.extend(slow);
        }
        (all, all_slow)
    });

    slow.sort_by_key(|(_, ms)| std::cmp::Reverse(*ms));
    let mut report = render_sweep_report(&mut findings, &slow, &out_dir);

    let verdict = check_known_open(
        findings.counts().keys().map(String::as_str),
        sweep.known_open,
    );
    let rendered = verdict.render(sweep.known_open.len());
    report.push_str(&rendered);
    fs::write(out_dir.join("report.txt"), &report).expect("write report");
    println!(
        "checked {} files, {} failures: {:?}",
        files.len(),
        findings.total(),
        findings.counts()
    );
    println!("report: {}", out_dir.join("report.txt").display());
    print!("{rendered}");
    i32::from(!verdict.is_clean())
}

/// Writes one repro file per retained sample and renders the report body: the
/// samples, what the per-key cap elided, the totals, and the slowest files.
fn render_sweep_report(
    findings: &mut SweepFindings,
    slow: &[(PathBuf, u128)],
    out_dir: &Path,
) -> String {
    let mut report = String::new();
    for (path, failure) in findings.samples() {
        let repro_dir = out_dir.join(failure.summary_key().replace(':', "-"));
        fs::create_dir_all(&repro_dir).expect("create repro directory");
        if let Ok(source) = fs::read_to_string(path) {
            let name = path.file_name().expect("corpus file name");
            fs::write(
                repro_dir.join(name),
                format!(
                    "// repro from: {} ({})\n{source}",
                    path.display(),
                    failure.category
                ),
            )
            .expect("write repro");
        }
        writeln!(
            report,
            "[{}] {}\n    {}",
            failure.category,
            path.display(),
            failure.detail.replace('\n', "\n    ")
        )
        .expect("writing to a String cannot fail");
    }
    for (key, count) in findings.counts().clone() {
        let elided = findings.elided(&key);
        if elided > 0 {
            writeln!(
                report,
                "\nelided {elided} further [{key}] sample(s) of {count} (cap {MAX_SAMPLES_PER_KEY} per key)",
            )
            .expect("writing to a String cannot fail");
        }
    }
    writeln!(report, "\nsummary: {:?}", findings.counts())
        .expect("writing to a String cannot fail");
    for (path, ms) in slow.iter().take(20) {
        writeln!(report, "slow: {ms}ms {}", path.display())
            .expect("writing to a String cannot fail");
    }
    report
}

fn collect_corpus_files(dir: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_corpus_files(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            files.push(path);
        }
    }
}

/// Sweeps one corpus file, turning a panic into a `panic` category rather than
/// aborting the whole run.
fn sweep_file<L: CorpusLanguage>(lang: &L, path: &Path) -> Vec<SweepFailure> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return vec![SweepFailure {
                category: "read-error".to_owned(),
                detail: error.to_string(),
            }];
        }
    };
    catch_unwind(AssertUnwindSafe(|| sweep_source(lang, &source))).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic>");
        vec![SweepFailure {
            category: "panic".to_owned(),
            detail: message.to_owned(),
        }]
    })
}

/// Compares a sweep's failure categories against the ones already known to be
/// open, so the sweep is a real signal while carrying existing debt.
///
/// A category absent from `known_open` is a regression the sweep must fail on. A
/// `known_open` entry that no longer fires is stale and must be deleted, which
/// also keeps the list from quietly outliving the bug it records.
pub struct KnownOpenVerdict {
    /// Categories that fired and are not in `known_open`.
    pub unexpected: Vec<String>,
    /// `known_open` entries that did not fire.
    pub stale: Vec<String>,
}

impl KnownOpenVerdict {
    /// True when the sweep matched the known-open list exactly.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.unexpected.is_empty() && self.stale.is_empty()
    }

    /// Renders the verdict so a new failure category, or one that has been fixed
    /// without pruning the list, is the thing that stands out.
    #[must_use]
    pub fn render(&self, known_open: usize) -> String {
        if self.is_clean() {
            return format!("\nsweep matches the {known_open} known-open categories\n");
        }
        let mut rendered = String::new();
        if !self.unexpected.is_empty() {
            let _ = writeln!(
                rendered,
                "\nREGRESSION: {} failure category/categories outside the known-open list: {:?}",
                self.unexpected.len(),
                self.unexpected
            );
        }
        if !self.stale.is_empty() {
            let _ = writeln!(
                rendered,
                "\nFIXED: {} known-open entry/entries no longer fire and must be deleted: {:?}",
                self.stale.len(),
                self.stale
            );
        }
        rendered
    }
}

/// Splits `observed` failure categories against `known_open`.
#[must_use]
pub fn check_known_open<'a>(
    observed: impl IntoIterator<Item = &'a str>,
    known_open: &[&str],
) -> KnownOpenVerdict {
    let observed: HashSet<&str> = observed.into_iter().collect();
    KnownOpenVerdict {
        unexpected: observed
            .iter()
            .filter(|category| !known_open.contains(*category))
            .map(|category| (*category).to_owned())
            .collect(),
        stale: known_open
            .iter()
            .filter(|category| !observed.contains(*category))
            .map(|category| (*category).to_owned())
            .collect(),
    }
}

/// Boundary count up to which a shape probes every boundary rather than
/// sampling. Above it the sweep would cost more than the corpus size warrants.
pub const SWEEP_EXHAUSTIVE_BOUNDARY_LIMIT: usize = 120;

/// Boundaries each shape samples in a file above that limit.
pub const SWEEP_SAMPLED_PROBES: usize = 3;

/// The line widths the sweep formats every file at.
pub const SWEEP_WIDTHS: &[u16] = &[40, 80, 120];

/// The boundary indices `shape` probes in a file with `boundaries` token
/// boundaries, in probe order.
///
/// Both the sweep and the single-file replay read placement from here, so a
/// reported `<path>:<shape><n>` always names the boundary the sweep used.
#[must_use]
pub fn probe_boundaries(shape: &ProbeShape, source: &str, boundaries: usize) -> Vec<usize> {
    if boundaries == 0 {
        return Vec::new();
    }
    if boundaries <= SWEEP_EXHAUSTIVE_BOUNDARY_LIMIT {
        return (0..boundaries).collect();
    }
    (0..SWEEP_SAMPLED_PROBES)
        .map(|probe| {
            (probe + 1).wrapping_mul(shape.seed.wrapping_mul(source.len() + 17)) % boundaries
        })
        .collect()
}

/// Looks a probe shape up by the name a `<path>:<shape><n>` argument carries,
/// returning the shape and the probe index.
#[must_use]
pub fn parse_probe_argument(probe: &str) -> Option<(&'static ProbeShape, usize)> {
    // Longest name first, so `ownlinecomment` is not read as `ownline` with a
    // number that fails to parse.
    let mut shapes: Vec<&ProbeShape> = PROBE_SHAPES.iter().collect();
    shapes.sort_by_key(|shape| std::cmp::Reverse(shape.name.len()));
    shapes.into_iter().find_map(|shape| {
        let index = probe.strip_prefix(shape.name)?.parse().ok()?;
        Some((shape, index))
    })
}

/// One failure the conservation sweep found, labelled by probe and check.
pub struct SweepFailure {
    /// `<probe label>:<check>`, e.g. `ownline12:not-idempotent`.
    pub category: String,
    /// Human-readable evidence, empty when the category says everything.
    pub detail: String,
}

impl SweepFailure {
    /// The category with the probe index stripped, so `ownline12:not-idempotent`
    /// and `ownline37:not-idempotent` share one summary key.
    #[must_use]
    pub fn summary_key(&self) -> String {
        let (probe, check) = self
            .category
            .split_once(':')
            .unwrap_or((self.category.as_str(), ""));
        let shape = probe.trim_end_matches(|character: char| character.is_ascii_digit());
        format!("{shape}:{check}")
    }
}

/// Samples kept per summary key, in the report and on disk.
///
/// A check that fails on a large share of the corpus would otherwise bury the
/// report under near-identical entries, and holding every failure's evidence --
/// two full formatted outputs for a non-idempotence -- exhausts memory well
/// before the sweep finishes. Counting is unbounded; retention is not. The
/// elided totals are reported rather than silently dropped.
pub const MAX_SAMPLES_PER_KEY: usize = 20;

/// Failure counts for a whole sweep, with a bounded sample of evidence.
#[derive(Default)]
pub struct SweepFindings {
    /// Total failures per summary key, uncapped.
    counts: BTreeMap<String, usize>,
    /// Retained samples per summary key, capped at [`MAX_SAMPLES_PER_KEY`].
    samples: Vec<(PathBuf, SweepFailure)>,
    retained: BTreeMap<String, usize>,
}

impl SweepFindings {
    /// Counts `failure`, keeping its evidence only while under the per-key cap.
    pub fn push(&mut self, path: &Path, failure: SweepFailure) {
        let key = failure.summary_key();
        *self.counts.entry(key.clone()).or_default() += 1;
        let retained = self.retained.entry(key).or_default();
        if *retained < MAX_SAMPLES_PER_KEY {
            *retained += 1;
            self.samples.push((path.to_path_buf(), failure));
        }
    }

    /// Folds another sweep's findings in, re-applying the per-key cap.
    pub fn merge(&mut self, other: Self) {
        for (key, count) in other.counts {
            *self.counts.entry(key).or_default() += count;
        }
        for (path, failure) in other.samples {
            let key = failure.summary_key();
            let retained = self.retained.entry(key).or_default();
            if *retained < MAX_SAMPLES_PER_KEY {
                *retained += 1;
                self.samples.push((path, failure));
            }
        }
    }

    /// Total failures across every key.
    #[must_use]
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    /// Failure counts per summary key.
    #[must_use]
    pub fn counts(&self) -> &BTreeMap<String, usize> {
        &self.counts
    }

    /// The retained samples, sorted by category then path.
    #[must_use]
    pub fn samples(&mut self) -> &[(PathBuf, SweepFailure)] {
        self.samples
            .sort_by(|(left_path, left), (right_path, right)| {
                (&left.category, left_path).cmp(&(&right.category, right_path))
            });
        &self.samples
    }

    /// How many failures of `key` were counted but not retained.
    #[must_use]
    pub fn elided(&self, key: &str) -> usize {
        self.counts
            .get(key)
            .copied()
            .unwrap_or_default()
            .saturating_sub(self.retained.get(key).copied().unwrap_or_default())
    }
}

/// Runs every sweep check on `source` at every [`SWEEP_WIDTHS`] width, then at
/// the default width with each [`PROBE_SHAPES`] probe injected.
#[must_use]
pub fn sweep_source<L: CorpusLanguage>(lang: &L, source: &str) -> Vec<SweepFailure> {
    let mut failures = Vec::new();
    for &width in SWEEP_WIDTHS {
        let options = FormatOptions {
            line_width: width,
            ..FormatOptions::default()
        };
        failures.extend(sweep_variant(lang, source, &options, &format!("w{width}")));
    }

    let boundaries = lang.token_end_offsets(source);
    let options = FormatOptions::default();
    let source_parses_cleanly = lang.parse_facts(source).diagnostics.is_empty();
    for shape in PROBE_SHAPES {
        for (probe, &boundary) in probe_boundaries(shape, source, boundaries.len())
            .iter()
            .enumerate()
        {
            let probed = probed_source(source, boundaries[boundary], shape);
            // A boundary offset can fall inside a string literal, where the
            // insertion is content rather than trivia: it breaks the literal
            // instead of modelling a comment. Such a probe tests nothing this
            // sweep is about, and the malformed source it produces can drive the
            // formatter into allocations large enough to abort the whole run,
            // which no in-process check can recover from. Require a probe to
            // keep clean source clean, so the sweep stays bounded by
            // construction rather than by luck.
            if source_parses_cleanly && !lang.parse_facts(&probed).diagnostics.is_empty() {
                continue;
            }
            failures.extend(sweep_variant(
                lang,
                &probed,
                &options,
                &format!("{}{probe}", shape.name),
            ));
        }
    }
    failures
}

/// Inserts `shape` at the byte offset `boundary`.
#[must_use]
pub fn probed_source(source: &str, boundary: usize, shape: &ProbeShape) -> String {
    let mut probed = String::with_capacity(source.len() + shape.insert.len());
    probed.push_str(&source[..boundary]);
    probed.push_str(shape.insert);
    probed.push_str(&source[boundary..]);
    probed
}

/// Checks one already-probed source at one width: clean reparse, diagnostic and
/// structure and comment conservation, and idempotence.
#[must_use]
pub fn sweep_variant<L: CorpusLanguage>(
    lang: &L,
    source: &str,
    options: &FormatOptions,
    label: &str,
) -> Vec<SweepFailure> {
    let mut failures = Vec::new();
    let mut fail = |check: &str, detail: String| {
        failures.push(SweepFailure {
            category: format!("{label}:{check}"),
            detail,
        });
    };

    let input = lang.parse_facts(source);
    if !input.has_tree {
        fail("no-tree", format!("diagnostics: {:?}", input.diagnostics));
        return failures;
    }
    let clean_input = input.diagnostics.is_empty();

    let formatted = match lang.try_format(source, options) {
        Ok(formatted) => formatted,
        Err(detail) => {
            fail("format-blocked", detail);
            return failures;
        }
    };
    let after = lang.parse_facts(&formatted);
    if !after.has_tree {
        fail("reparse-no-tree", String::new());
        return failures;
    }
    if clean_input && !after.diagnostics.is_empty() {
        fail(
            "reparse-dirty",
            format!("formatted output diagnostics: {:?}", after.diagnostics),
        );
    }
    if !clean_input
        && diagnostic_inventory(&input.diagnostics) != diagnostic_inventory(&after.diagnostics)
    {
        fail("diag-inventory-changed", String::new());
    }
    if clean_input && input.structure != after.structure {
        fail("structure-changed", String::new());
    }
    if input.comment_inventory != after.comment_inventory {
        fail(
            "comments-changed",
            format!(
                "expected: {:?}\nactual: {:?}",
                input.comment_inventory, after.comment_inventory
            ),
        );
    }
    match lang.try_format(&formatted, options) {
        Ok(repeated) if repeated != formatted => {
            fail(
                "not-idempotent",
                format!("--- first\n{formatted}\n--- second\n{repeated}"),
            );
        }
        Err(detail) => fail("reformat-blocked", detail),
        Ok(_) => {}
    }
    failures
}

/// Drives the shared formatter corpus loop over `files`.
///
/// Fixtures under `syntax/lexer` or `syntax/recovery`, and any fixture the
/// language expects to carry parser diagnostics, take the audit path (which
/// checks conservation without producing a corpus snapshot). Every other
/// fixture is formatted, reparsed, conservation-checked, and snapshotted via
/// `snapshot` (kept at the call site so insta snapshot names are preserved).
pub fn run_formatter_corpus<L: CorpusLanguage>(
    lang: &L,
    root: &Path,
    files: &[PathBuf],
    mut snapshot: impl FnMut(&str, &str),
) {
    let mut formatted_cases = 0usize;
    let mut conservation_failures = Vec::new();

    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let source = read_to_string(path);
        let input = lang.parse_facts(&source);
        assert!(
            input.has_tree,
            "{} formatter corpus fixture produced no represented tree: {}",
            lang.language_name(),
            path.display()
        );

        let dedicated_audit =
            relative.starts_with("syntax/lexer") || relative.starts_with("syntax/recovery");
        let expected_parser_diagnostics = lang.expects_parser_diagnostics(&relative);
        if !dedicated_audit {
            assert_eq!(
                !input.diagnostics.is_empty(),
                expected_parser_diagnostics,
                "{} formatter corpus route changed for {relative}: diagnostics={:#?}",
                lang.language_name(),
                input.diagnostics
            );
        }

        if dedicated_audit || expected_parser_diagnostics {
            if let Some(failure) = audit_diagnostic_source(lang, &source, &relative, &input) {
                conservation_failures.push(failure);
            }
            continue;
        }

        formatted_cases += 1;
        let label = path.display().to_string();
        let formatted = lang.format(&source, &label);
        let formatted_facts = lang.parse_facts(&formatted);
        assert!(
            formatted_facts.diagnostics.is_empty(),
            "formatted output did not parse cleanly for {}: {:#?}\n{}",
            path.display(),
            formatted_facts.diagnostics,
            formatted
        );
        assert!(
            formatted_facts.has_tree,
            "formatted output produced no syntax tree for {}",
            path.display()
        );
        let repeated = lang.format(&formatted, &label);
        let mut failure = String::new();
        append_stability_failures(
            &mut failure,
            &source,
            &formatted,
            &repeated,
            &input,
            &formatted_facts,
        );
        if !failure.is_empty() {
            conservation_failures.push(format!("{relative}:\n{failure}"));
        }

        let snapshot_body = SnapshotBuilder::new()
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(&[]))
            .finish();
        snapshot(&fixture_snapshot_name(root, path), &snapshot_body);
    }

    assert!(
        formatted_cases > 0,
        "expected at least one valid {} formatter corpus fixture",
        lang.language_name()
    );
    assert!(
        conservation_failures.is_empty(),
        "{} formatter conservation failures:\n{}",
        lang.language_name(),
        conservation_failures.join("\n")
    );
}

/// Drives the shared recovery snapshot loop over `files`.
///
/// Each recovery fixture is formatted, reparsed, stability-checked, and
/// snapshotted (input, formatted, and parser diagnostics).
pub fn run_recovery_corpus<L: CorpusLanguage>(
    lang: &L,
    recovery_root: &Path,
    files: &[PathBuf],
    mut snapshot: impl FnMut(&str, &str),
) {
    assert!(!files.is_empty(), "expected at least one recovery fixture");
    let mut conservation_failures = Vec::new();

    for path in files {
        let source = read_to_string(path);
        let input = lang.parse_facts(&source);
        assert!(
            input.has_tree,
            "recovery fixture did not produce a represented tree for {}",
            path.display()
        );
        let label = path.display().to_string();
        let formatted = lang.format(&source, &label);
        let formatted_facts = lang.parse_facts(&formatted);
        assert!(
            formatted_facts.has_tree,
            "formatted recovery output did not produce a represented tree for {}:\n{}",
            path.display(),
            formatted
        );
        let repeated = lang.format(&formatted, &label);
        let mut failure = String::new();
        append_stability_failures(
            &mut failure,
            &source,
            &formatted,
            &repeated,
            &input,
            &formatted_facts,
        );
        if !failure.is_empty() {
            conservation_failures.push(format!("{}:\n{failure}", path.display()));
        }

        let snapshot_body = SnapshotBuilder::new()
            .section("input", &source)
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(&input.diagnostics))
            .finish();
        snapshot(&fixture_snapshot_name(recovery_root, path), &snapshot_body);
    }

    assert!(
        conservation_failures.is_empty(),
        "formatter lost represented {} source:\n{}",
        lang.language_name(),
        conservation_failures.join("\n")
    );
}

/// Shared audit path: checks that reformatting a diagnostic-carrying fixture
/// preserves diagnostic classification, comment inventory, trivia markers, and
/// idempotence.
fn audit_diagnostic_source<L: CorpusLanguage>(
    lang: &L,
    source: &str,
    label: &str,
    before: &CorpusParseFacts,
) -> Option<String> {
    let formatted = lang.format(source, label);
    let after = lang.parse_facts(&formatted);
    if !after.has_tree {
        return Some(format!("{label}: formatted output has no represented tree"));
    }
    let repeated = lang.format(&formatted, label);

    let mut failures = String::new();
    if diagnostic_inventory(&before.diagnostics) != diagnostic_inventory(&after.diagnostics) {
        failures.push_str("parser diagnostic classification changed\n");
    }
    append_stability_failures(&mut failures, source, &formatted, &repeated, before, &after);
    (!failures.is_empty()).then(|| format!("{label}:\ninput:\n{source}\n{failures}"))
}

fn append_stability_failures(
    failures: &mut String,
    source: &str,
    formatted: &str,
    repeated: &str,
    input: &CorpusParseFacts,
    formatted_facts: &CorpusParseFacts,
) {
    if input.comment_inventory != formatted_facts.comment_inventory {
        write!(
            failures,
            "represented comment inventory changed\nexpected: {:#?}\nactual: {:#?}\n",
            input.comment_inventory, formatted_facts.comment_inventory
        )
        .expect("writing to a String cannot fail");
    }
    if input.structure != formatted_facts.structure {
        write!(
            failures,
            "formatting changed the parse tree beyond authorized normalizations\n{}\n",
            describe_structure_divergence(&input.structure, &formatted_facts.structure)
        )
        .expect("writing to a String cannot fail");
    }
    let expected_markers = trivia_markers(source);
    let actual_markers = trivia_markers(formatted);
    if actual_markers != expected_markers {
        write!(
            failures,
            "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
        )
        .expect("writing to a String cannot fail");
    }
    if repeated != formatted {
        write!(
            failures,
            "formatter output is not idempotent\nfirst:\n{formatted}\nsecond:\n{repeated}\n"
        )
        .expect("writing to a String cannot fail");
    }
}

/// Asserts that no rendered line of `formatted` exceeds `line_width` using the
/// same Unicode-aware width model as the formatter renderer.
pub fn assert_no_line_exceeds_width(formatted: &str, label: &str, line_width: u16) {
    let limit = usize::from(line_width);
    let offending = formatted
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line, line.width()))
        .find(|(_, _, width)| *width > limit);

    assert!(
        offending.is_none(),
        "formatted line exceeded width {line_width} in {label}:\n{formatted}\nfirst offending line: {offending:?}",
    );
}

/// Formats `sources` with a block comment inserted after every token boundary,
/// asserting the formatter never blocks and stays idempotent.
///
/// Line comments are a known gap: sweeping them finds further Kotlin failures
/// that are not yet fixed, so this stays on block comments until they are.
///
/// A trailing comment is only emitted if some rule takes responsibility for it.
/// `TrailingTrivia::RelocatedToEnclosingContext` moves that responsibility to
/// the enclosing rule, and nothing checks that the enclosing rule accepts it, so
/// a rule can silently drop the comments of a token it formats. The renderer
/// catches the loss, but only for source that actually places a comment there,
/// which no fixture had done for most token positions.
///
/// # Panics
///
/// Panics listing every position whose formatted output was blocked.
pub fn assert_comments_format_at_every_token_position<L: CorpusLanguage>(
    lang: &L,
    format: impl Fn(&str) -> Result<String, Diagnostic>,
    sources: &[&str],
) {
    let mut blocked = Vec::new();
    for source in sources {
        assert!(
            format(source).is_ok(),
            "{} baseline source did not format: {source}",
            lang.language_name()
        );

        for token_end in lang.token_end_offsets(source) {
            for insert in [" /*c*/"] {
                let mut probe = String::with_capacity(source.len() + insert.len());
                probe.push_str(&source[..token_end]);
                probe.push_str(insert);
                probe.push_str(&source[token_end..]);
                let once = match format(&probe) {
                    Ok(once) => once,
                    Err(diagnostic) => {
                        blocked.push(format!("  {probe:?}\n    refused: {}", diagnostic.message));
                        continue;
                    }
                };
                match format(&once) {
                    Err(diagnostic) => blocked.push(format!(
                        "  {probe:?}\n    refused on reformat: {}",
                        diagnostic.message
                    )),
                    Ok(twice) if twice != once => blocked.push(format!(
                        "  {probe:?}\n    not idempotent:\n--- once\n{once}--- twice\n{twice}"
                    )),
                    Ok(_) => {}
                }
            }
        }
    }

    assert!(
        blocked.is_empty(),
        "{} formatter mishandled a comment at {} token position(s):\n{}",
        lang.language_name(),
        blocked.len(),
        blocked.join("\n")
    );
}
