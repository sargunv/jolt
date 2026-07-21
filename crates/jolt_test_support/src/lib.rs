#![allow(clippy::missing_panics_doc)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Write as _};
use std::fs;
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
    represented_token_loss_report_from_inventories(
        token_text_inventory(before),
        token_text_inventory(after),
        removals,
    )
}

/// Inventory-based token-loss report used by the shared corpus harness.
#[must_use]
pub fn represented_token_loss_report_from_inventories(
    mut before: BTreeMap<String, usize>,
    mut after: BTreeMap<String, usize>,
    removals: &[RepresentedTokenRemoval],
) -> String {
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
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter blocked for {label}: {diagnostics:#?}")
        }
    }
}

/// Builds a conservation failure report for one formatted fixture, or `None`
/// when tokens, comments, trivia markers, and idempotence all hold.
#[must_use]
pub fn formatter_conservation_failure<'before, 'after, L>(
    path_label: impl Display,
    source: &str,
    formatted: &str,
    repeated: &str,
    input_tokens: impl IntoIterator<Item = SyntaxToken<'before, L>>,
    formatted_tokens: impl IntoIterator<Item = SyntaxToken<'after, L>>,
    allowed_removals: &[RepresentedTokenRemoval],
) -> Option<String>
where
    L: Language,
{
    let input_tokens = input_tokens.into_iter().collect::<Vec<_>>();
    let formatted_tokens = formatted_tokens.into_iter().collect::<Vec<_>>();
    let token_loss = represented_token_loss_report(
        input_tokens.iter().copied(),
        formatted_tokens.iter().copied(),
        allowed_removals,
    );
    let comment_loss = (represented_comment_inventory(input_tokens.iter().copied())
        != represented_comment_inventory(formatted_tokens.iter().copied()))
    .then_some("represented comment inventory changed\n");
    let expected_markers = trivia_markers(source);
    let actual_markers = trivia_markers(formatted);
    let marker_loss = (actual_markers != expected_markers).then(|| {
        format!(
            "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
        )
    });
    let mut failure = String::new();
    if !token_loss.is_empty() || comment_loss.is_some() || marker_loss.is_some() {
        let _ = write!(
            failure,
            "{path_label}:\n{token_loss}{}{}",
            comment_loss.unwrap_or_default(),
            marker_loss.unwrap_or_default()
        );
    }
    if repeated != formatted {
        if !failure.is_empty() {
            failure.push('\n');
        }
        let _ = write!(
            failure,
            "{path_label}:\nformatter output is not idempotent\nfirst:\n{formatted}\nsecond:\n{repeated}"
        );
    }
    (!failure.is_empty()).then_some(failure)
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

/// Owned parse facts for the shared formatter corpus harness.
///
/// Built inside each language's parse scope so the harness never holds borrowed
/// tokens across a second parse of formatted output.
#[derive(Clone, Debug)]
pub struct CorpusParseFacts {
    pub has_tree: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub token_inventory: BTreeMap<String, usize>,
    pub comment_inventory: BTreeMap<String, usize>,
}

/// Builds owned corpus facts from a language parse while its buffers are live.
#[must_use]
pub fn corpus_parse_facts<'source, L>(
    has_tree: bool,
    diagnostics: &[Diagnostic],
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
) -> CorpusParseFacts
where
    L: Language,
{
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    CorpusParseFacts {
        has_tree,
        diagnostics: diagnostics.to_vec(),
        token_inventory: token_text_inventory(tokens.iter().copied()),
        comment_inventory: represented_comment_inventory(tokens.iter().copied()),
    }
}

/// Language bindings for the shared formatter corpus / recovery harness.
///
/// The harness owns the fixture walk, audit-vs-format routing, conservation
/// checks, and snapshot orchestration. Implementors supply owned parse facts,
/// the format function, and language-specific classification policy.
pub trait CorpusLanguage {
    /// Human-readable language name used in harness assertion messages.
    fn language_name(&self) -> &'static str;

    /// Parses one fixture source into owned conservation facts.
    fn parse_facts(&self, source: &str) -> CorpusParseFacts;

    /// Formats `source`, panicking on halt/block like the corpus tests expect.
    fn format(&self, source: &str, label: &str) -> String;

    /// True when a fixture at `relative` is expected to carry parser
    /// diagnostics, routing it through the audit path instead of the format
    /// snapshot path.
    fn expects_parser_diagnostics(&self, relative: &str) -> bool;

    /// Bounded source-token removals permitted for a clean format fixture.
    fn allowed_clean_removals(&self, relative: &str) -> &'static [RepresentedTokenRemoval];
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
        let token_loss = represented_token_loss_report_from_inventories(
            input.token_inventory.clone(),
            formatted_facts.token_inventory,
            lang.allowed_clean_removals(&relative),
        );
        let expected_markers = trivia_markers(&source);
        let actual_markers = trivia_markers(&formatted);
        if !token_loss.is_empty() || actual_markers != expected_markers {
            conservation_failures.push(format!(
                "{relative}:\n{token_loss}{}",
                if actual_markers == expected_markers {
                    String::new()
                } else {
                    format!(
                        "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
                    )
                }
            ));
        }

        let repeated = lang.format(&formatted, &label);
        assert_eq!(
            repeated,
            formatted,
            "formatter output was not idempotent for {}",
            path.display()
        );

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
        "formatter lost represented {} source:\n{}",
        lang.language_name(),
        conservation_failures.join("\n")
    );
}

/// Drives the shared recovery snapshot loop over `files`.
///
/// Each recovery fixture is formatted, reparsed, conservation-checked, and
/// snapshotted (input, formatted, and parser diagnostics).
/// `allowed_removed_tokens` supplies per-fixture normalization removals.
pub fn run_recovery_corpus<L: CorpusLanguage>(
    lang: &L,
    recovery_root: &Path,
    files: &[PathBuf],
    allowed_removed_tokens: impl Fn(&Path) -> &'static [RepresentedTokenRemoval],
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
        if let Some(failure) = formatter_conservation_failure_from_facts(
            path.display(),
            &source,
            &formatted,
            &repeated,
            &input,
            &formatted_facts,
            allowed_removed_tokens(path),
        ) {
            conservation_failures.push(failure);
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
    let comments_changed = before.comment_inventory != after.comment_inventory;
    let expected_markers = trivia_markers(source);
    let actual_markers = trivia_markers(&formatted);
    let repeated = lang.format(&formatted, label);

    let mut failures = String::new();
    if diagnostic_inventory(&before.diagnostics) != diagnostic_inventory(&after.diagnostics) {
        failures.push_str("parser diagnostic classification changed\n");
    }
    if comments_changed {
        failures.push_str("represented comment inventory changed\n");
    }
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
    (!failures.is_empty()).then(|| format!("{label}:\ninput:\n{source}\n{failures}"))
}

/// Conservation failure report built from owned parse facts.
#[must_use]
pub fn formatter_conservation_failure_from_facts(
    path_label: impl Display,
    source: &str,
    formatted: &str,
    repeated: &str,
    input: &CorpusParseFacts,
    formatted_facts: &CorpusParseFacts,
    allowed_removals: &[RepresentedTokenRemoval],
) -> Option<String> {
    let token_loss = represented_token_loss_report_from_inventories(
        input.token_inventory.clone(),
        formatted_facts.token_inventory.clone(),
        allowed_removals,
    );
    let comment_loss = (input.comment_inventory != formatted_facts.comment_inventory)
        .then_some("represented comment inventory changed\n");
    let expected_markers = trivia_markers(source);
    let actual_markers = trivia_markers(formatted);
    let marker_loss = (actual_markers != expected_markers).then(|| {
        format!(
            "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
        )
    });
    let mut failure = String::new();
    if !token_loss.is_empty() || comment_loss.is_some() || marker_loss.is_some() {
        let _ = write!(
            failure,
            "{path_label}:\n{token_loss}{}{}",
            comment_loss.unwrap_or_default(),
            marker_loss.unwrap_or_default()
        );
    }
    if repeated != formatted {
        if !failure.is_empty() {
            failure.push('\n');
        }
        let _ = write!(
            failure,
            "{path_label}:\nformatter output is not idempotent\nfirst:\n{formatted}\nsecond:\n{repeated}"
        );
    }
    (!failure.is_empty()).then_some(failure)
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
