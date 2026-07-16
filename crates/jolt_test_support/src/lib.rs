#![allow(clippy::missing_panics_doc)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Write as _};
use std::fs;
use std::path::{Path, PathBuf};

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId};
use jolt_fmt_ir::{RenderControl, RenderSink};
use jolt_syntax::{
    CommentKind, Language, SyntaxDiagnosticOwner, SyntaxNode, SyntaxSlot, SyntaxToken,
};
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;

mod schema_audit;

pub use schema_audit::{PhysicalNodeAudit, SchemaAudit};

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
    assert!(
        root.is_dir(),
        "required Java fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_java_files_into(root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one Java fixture under {}",
        root.display()
    );
    files
}

#[must_use]
pub fn collect_kotlin_files(root: &Path) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required Kotlin fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_kotlin_files_into(root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one Kotlin fixture under {}",
        root.display()
    );
    files
}

fn collect_java_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files_into(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}

fn collect_kotlin_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_kotlin_files_into(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "kt" || extension == "kts")
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub enum DeferredReason {
    ParserDiagnostics,
    NoSyntaxTree,
    SyntaxReconstructionMismatch,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObservedDeferredPath {
    suite: String,
    path: String,
    reasons: Vec<DeferredReason>,
}

impl ObservedDeferredPath {
    #[must_use]
    pub fn new(
        suite: &str,
        suite_root: &Path,
        path: &Path,
        reasons: impl IntoIterator<Item = DeferredReason>,
    ) -> Self {
        let relative = path
            .strip_prefix(suite_root)
            .unwrap_or_else(|error| {
                panic!(
                    "{} is outside {}: {error}",
                    path.display(),
                    suite_root.display()
                )
            })
            .to_string_lossy()
            .replace('\\', "/");
        let mut reasons = reasons.into_iter().collect::<Vec<_>>();
        reasons.sort_unstable();
        reasons.dedup();
        assert!(!reasons.is_empty(), "deferred path must have a reason");
        Self {
            suite: suite.to_owned(),
            path: relative,
            reasons,
        }
    }
}

#[derive(Deserialize)]
struct DeferredManifest {
    version: u8,
    entries: Vec<DeferredEntry>,
}

#[derive(Deserialize)]
struct DeferredEntry {
    suite: String,
    path: String,
    reasons: Vec<DeferredReason>,
    owner_phase: u8,
}

/// Verifies that the exact imported migration queue matches the paths which
/// cannot yet enter the hard formatter gate.
pub fn assert_deferred_import_manifest(
    workspace_root: &Path,
    imports_root: &Path,
    suite_names: &[&str],
    reason_filter: &[DeferredReason],
    observed: &[ObservedDeferredPath],
) {
    const HARD_GATE_SUITES: &[&str] = &[
        "google-java-format/input",
        "palantir-java-format/input",
        "prettier-java/input",
        "ktfmt/source",
        "maplibre-compose/source",
    ];
    let manifest_path = workspace_root.join("tools/import/deferred-formatter-paths.json");
    let manifest: DeferredManifest = serde_json::from_str(&read_to_string(&manifest_path))
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", manifest_path.display()));
    assert_eq!(
        manifest.version, 1,
        "unsupported deferred formatter manifest version"
    );

    let mut expected = Vec::with_capacity(manifest.entries.len());
    let mut unique_paths = std::collections::BTreeSet::new();
    for entry in manifest.entries {
        assert!(
            HARD_GATE_SUITES.contains(&entry.suite.as_str()),
            "deferred path belongs to a suite outside the formatter hard gate: {}/{}",
            entry.suite,
            entry.path
        );
        assert!(
            unique_paths.insert((entry.suite.clone(), entry.path.clone())),
            "duplicate deferred formatter path: {}/{}",
            entry.suite,
            entry.path
        );
        assert!(
            (8..=20).contains(&entry.owner_phase),
            "invalid owner phase {} for {}/{}",
            entry.owner_phase,
            entry.suite,
            entry.path
        );
        let path = imports_root.join(&entry.suite).join(&entry.path);
        assert!(path.is_file(), "missing deferred path {}", path.display());
        let mut reasons = entry.reasons;
        let reason_count = reasons.len();
        reasons.sort_unstable();
        reasons.dedup();
        assert!(
            !reasons.is_empty(),
            "deferred path has no reason: {}/{}",
            entry.suite,
            entry.path
        );
        assert_eq!(
            reasons.len(),
            reason_count,
            "duplicate deferred reason for {}/{}",
            entry.suite,
            entry.path
        );
        if suite_names.contains(&entry.suite.as_str())
            && (reason_filter.is_empty()
                || reasons.iter().any(|reason| reason_filter.contains(reason)))
        {
            expected.push(ObservedDeferredPath {
                suite: entry.suite,
                path: entry.path,
                reasons,
            });
        }
    }
    expected.sort();
    let mut actual = observed.to_vec();
    assert!(
        actual
            .iter()
            .all(|entry| suite_names.contains(&entry.suite.as_str())),
        "observed a deferred path outside the selected suites"
    );
    actual.sort();
    assert_eq!(
        actual, expected,
        "deferred formatter paths changed; fix a path in its owning phase or update the reviewed migration queue"
    );
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorpusSummary {
    suite: String,
    files: usize,
    reconstructed_changed: usize,
    diagnostics: BTreeMap<String, usize>,
}

impl CorpusSummary {
    #[must_use]
    pub fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            reconstructed_changed: 0,
            diagnostics: BTreeMap::new(),
        }
    }

    pub fn note_reconstruction_changed(&mut self) {
        self.reconstructed_changed += 1;
    }

    pub fn record_diagnostics(&mut self, diagnostics: &[Diagnostic]) {
        for diagnostic in diagnostics {
            let key = format!("{:?}:{}", diagnostic.stage, diagnostic.code.as_str());
            *self.diagnostics.entry(key).or_default() += 1;
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
        writeln!(
            &mut output,
            "reconstructed changed: {}",
            self.reconstructed_changed
        )
        .expect("write summary");
        output.push_str("\ndiagnostics:\n");
        if self.diagnostics.is_empty() {
            output.push_str("  <none>: 0\n");
        } else {
            for (kind, count) in &self.diagnostics {
                writeln!(&mut output, "  {kind}: {count}").expect("write summary");
            }
        }
        output
    }
}

/// Summary of an imported fixture corpus run through the formatter, tracking
/// how many files formatted cleanly versus were blocked by syntax or
/// formatter diagnostics. Shared across the Java and Kotlin formatter crates
/// since the imported-corpus test flow is identical modulo parse/format entry
/// points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedFormatterSummary {
    suite: String,
    files: usize,
    formatted: usize,
    syntax_blocked: usize,
    reconstructed_changed: usize,
    diagnostics: BTreeMap<String, usize>,
}

impl ImportedFormatterSummary {
    #[must_use]
    pub fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            formatted: 0,
            syntax_blocked: 0,
            reconstructed_changed: 0,
            diagnostics: BTreeMap::new(),
        }
    }

    pub fn note_formatted(&mut self) {
        self.formatted += 1;
    }

    pub fn note_syntax_blocked(&mut self) {
        self.syntax_blocked += 1;
    }

    pub fn note_reconstruction_changed(&mut self) {
        self.reconstructed_changed += 1;
    }

    pub fn record_diagnostics(&mut self, diagnostics: &[Diagnostic]) {
        for diagnostic in diagnostics {
            let key = format!("{:?}:{}", diagnostic.stage, diagnostic.code.as_str());
            *self.diagnostics.entry(key).or_default() += 1;
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
        writeln!(&mut output, "formatted: {}", self.formatted).expect("write summary");
        writeln!(&mut output, "syntax blocked: {}", self.syntax_blocked).expect("write summary");
        writeln!(
            &mut output,
            "reconstructed changed: {}",
            self.reconstructed_changed
        )
        .expect("write summary");
        output.push_str("\ndiagnostics:\n");
        if self.diagnostics.is_empty() {
            output.push_str("  <none>: 0\n");
        } else {
            for (kind, count) in &self.diagnostics {
                writeln!(&mut output, "  {kind}: {count}").expect("write summary");
            }
        }
        output
    }
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
    let multiline = body.contains('\n');
    body.lines()
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
        .collect::<Vec<_>>()
        .join(" ")
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
}

/// Describes a bounded source-token removal performed by a formatter
/// normalization rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepresentedTokenRemoval {
    pub source: &'static str,
    pub count: usize,
}

/// Reports represented source tokens that disappeared while formatting.
///
/// Token order and output-only tokens are ignored because formatters may sort
/// constructs and synthesize documented readability tokens. Source-token
/// removals and spelling changes must be listed explicitly by the caller.
#[must_use]
pub fn represented_token_loss_report<'before, 'after, L>(
    before: impl IntoIterator<Item = SyntaxToken<'before, L>>,
    after: impl IntoIterator<Item = SyntaxToken<'after, L>>,
    removals: &[RepresentedTokenRemoval],
) -> String
where
    L: Language,
{
    let mut before = token_text_inventory(before);
    let mut after = token_text_inventory(after);

    let exact_tokens = before.keys().cloned().collect::<Vec<_>>();
    for token in exact_tokens {
        cancel_inventory_counts(&mut before, &mut after, &token, &token, usize::MAX);
    }

    for removal in removals {
        subtract_inventory_count(&mut before, removal.source, removal.count);
    }

    let mut report = String::new();
    for (token, count) in before {
        if count > 0 {
            writeln!(&mut report, "missing {count} x {token:?}").expect("write token-loss report");
        }
    }
    report
}

fn token_text_inventory<'source, L>(
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
) -> BTreeMap<String, usize>
where
    L: Language,
{
    let mut inventory = BTreeMap::new();
    for token in tokens {
        if token.kind() != L::eof_kind() {
            *inventory.entry(token.text().to_owned()).or_default() += 1;
        }
    }
    inventory
}

fn cancel_inventory_counts(
    before: &mut BTreeMap<String, usize>,
    after: &mut BTreeMap<String, usize>,
    source: &str,
    output: &str,
    limit: usize,
) {
    let matched = before
        .get(source)
        .copied()
        .unwrap_or_default()
        .min(after.get(output).copied().unwrap_or_default())
        .min(limit);
    if matched == 0 {
        return;
    }

    if let Some(count) = before.get_mut(source) {
        *count -= matched;
    }
    if let Some(count) = after.get_mut(output) {
        *count -= matched;
    }
}

fn subtract_inventory_count(inventory: &mut BTreeMap<String, usize>, token: &str, count: usize) {
    if let Some(remaining) = inventory.get_mut(token) {
        *remaining = remaining.saturating_sub(count);
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
