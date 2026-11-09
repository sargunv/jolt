//! The formatter oracle: reparse, structure fingerprint, comment multiset,
//! idempotence.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use jolt_fmt::FormatOptions;
use jolt_syntax::{
    Diagnostic, Language, SyntaxElement, SyntaxNode, SyntaxToken, TriviaKind, TriviaPiece,
};

use crate::doc_comments;

/// What a language provides to the oracle, the corpus tests and the sweep.
pub trait CorpusLanguage: Sync {
    type Syntax: Language;
    /// Human-readable language name for assertion messages. Lowercased, it
    /// names the sweep's report directory.
    fn name(&self) -> &'static str;
    /// The extension of the source files the sweep checks.
    fn extension(&self) -> &'static str;
    /// Parses `source` and hands the root and diagnostics to `visit`.
    fn with_parse<R>(
        &self,
        source: &str,
        visit: impl FnOnce(SyntaxNode<'_>, &[Diagnostic]) -> R,
    ) -> R;
    fn format(&self, source: &str, options: &FormatOptions) -> String;
    /// The tree edits the formatter may make.
    fn structure_policy(&self) -> &StructurePolicy<<Self::Syntax as Language>::Kind>;
    /// End offsets of the tokens where a probe comment may be inserted
    /// (positions inside literals are excluded by construction: only token
    /// ends are offered).
    fn token_end_offsets(&self, source: &str) -> Vec<usize> {
        self.with_parse(source, |root, _| {
            root.tokens()
                .filter(|token| !token.text().is_empty())
                .map(|token| token.text_range().end().get())
                .collect()
        })
    }
    /// The byte ranges of the non-empty tokens, which the sweep deletes one
    /// at a time.
    fn token_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        self.with_parse(source, |root, _| {
            root.tokens()
                .filter(|token| !token.text().is_empty())
                .map(|token| {
                    let range = token.text_range();
                    (range.start().get(), range.end().get())
                })
                .collect()
        })
    }
    /// Whether the fixture at `relative` is expected to carry diagnostics.
    fn expects_diagnostics(&self, relative: &str) -> bool;
    /// Whether `token` starts a declaration that javadoc documents, so that
    /// the doc comment right before it is documentation.
    fn starts_documented_declaration(&self, token: SyntaxToken<'_>) -> bool;
}

/// Owned facts about one parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFacts {
    pub diagnostics: Vec<Diagnostic>,
    pub comments: BTreeMap<String, usize>,
    pub structure: String,
    /// Tokens other than normalizable punctuation, by kind and text. The
    /// structure check covers clean inputs; this covers dirty ones, whose
    /// recovered trees may reparse differently.
    pub tokens: BTreeMap<String, usize>,
}

pub fn parse_facts<C: CorpusLanguage>(language: &C, source: &str) -> ParseFacts {
    let policy = language.structure_policy();
    language.with_parse(source, |root, diagnostics| ParseFacts {
        diagnostics: diagnostics.to_vec(),
        comments: comment_inventory(root, |token| {
            diagnostics.is_empty() && language.starts_documented_declaration(token)
        }),
        structure: structure_fingerprint::<C::Syntax>(root, policy),
        tokens: token_inventory::<C::Syntax>(root, policy),
    })
}

fn token_inventory<L: Language>(
    root: SyntaxNode<'_>,
    policy: &StructurePolicy<L::Kind>,
) -> BTreeMap<String, usize> {
    let mut tokens = BTreeMap::new();
    for token in root.tokens() {
        let kind = L::from_raw(token.kind());
        if token.text().is_empty() || policy.normalizable_punctuation.contains(&kind) {
            continue;
        }
        if policy.splittable_tokens.contains(&kind) {
            for piece in token.text().chars() {
                *tokens.entry(piece.to_string()).or_default() += 1;
            }
            continue;
        }
        // Line endings become `\n` (text blocks included) and lines lose
        // trailing whitespace (only an unterminated literal has any).
        let text = token.text().replace("\r\n", "\n").replace('\r', "\n");
        let text = text.trim_end_matches([' ', '\t', '\x0c']);
        *tokens.entry(text.to_owned()).or_default() += 1;
    }
    tokens
}

/// Diagnostic codes with their counts.
#[must_use]
fn diagnostic_codes(diagnostics: &[Diagnostic]) -> BTreeMap<&'static str, usize> {
    let mut codes = BTreeMap::new();
    for diagnostic in diagnostics {
        *codes.entry(diagnostic.code).or_default() += 1;
    }
    codes
}

/// Every comment by kind and canonical text. A `/**` comment has the form
/// [`doc_comments::javadoc`] gives it, and a run of `///` lines (see
/// [`MarkdownRuns`]) the form [`doc_comments::markdown`] gives the run.
/// `documented` tells whether javadoc reads the doc comment right before a
/// token: the token starts a declaration that javadoc documents, in a
/// source without syntax errors.
#[must_use]
fn comment_inventory(
    root: SyntaxNode<'_>,
    documented: impl Fn(SyntaxToken<'_>) -> bool,
) -> BTreeMap<String, usize> {
    let mut comments = BTreeMap::new();
    let mut runs = MarkdownRuns::default();
    for token in root.tokens() {
        for piece in token.leading() {
            inventory_piece(root.source(), piece, &mut runs, &mut comments);
        }
        if !token.text().is_empty() {
            runs.token(|| documented(token), &mut comments);
        }
        for piece in token.trailing() {
            inventory_piece(root.source(), piece, &mut runs, &mut comments);
        }
    }
    runs.token(|| false, &mut comments);
    comments
}

/// Adds one trivia piece to the comment inventory.
fn inventory_piece<'a>(
    source: &str,
    piece: TriviaPiece<'a>,
    runs: &mut MarkdownRuns<'a>,
    comments: &mut BTreeMap<String, usize>,
) {
    let (kind, text) = match piece.kind {
        TriviaKind::Newline => {
            runs.newline();
            return;
        }
        TriviaKind::LineComment if runs.member(piece.text, comments) => return,
        TriviaKind::LineComment => ("line", canonical_comment_text(piece.text)),
        TriviaKind::BlockComment | TriviaKind::UnterminatedBlockComment => {
            ("block", canonical_comment_text(piece.text))
        }
        TriviaKind::DocComment => {
            // Only the last doc comment before a declaration documents it.
            runs.doc_comment(comments);
            (
                "doc",
                doc_comments::javadoc(
                    piece.text,
                    line_indentation(source, piece.range.start().get()),
                )
                .unwrap_or_else(|| canonical_comment_text(piece.text)),
            )
        }
        TriviaKind::Whitespace => return,
        TriviaKind::ByteOrderMark | TriviaKind::TrailingSubstitute => {
            runs.token(|| false, comments);
            return;
        }
    };
    runs.end_run(comments);
    *comments.entry(format!("{kind}:{text}")).or_default() += 1;
}

/// The spaces and tabs that start the line containing `offset`.
fn line_indentation(source: &str, offset: usize) -> &str {
    let start = source[..offset]
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1);
    let line = &source[start..];
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

/// Groups `///` lines into runs from the trivia alone, as javadoc reads them
/// (JEP 467): lines that start with `///` but not `////`, each starting its
/// line, one after the other with no blank line between. A `///` comment
/// after code on its line is no doc comment to javadoc; it is a run of its
/// own here, so that a layout may put it on a line of its own, but not make
/// it part of a run.
///
/// A run is documentation when it is the last doc comment before a token
/// that starts a documented declaration; only then does its form read the
/// hard breaks of lines that end in two spaces (see
/// [`doc_comments::markdown`]).
#[derive(Default)]
struct MarkdownRuns<'a> {
    run: Vec<&'a str>,
    /// The last run that ended, before the token after it.
    waiting: Vec<&'a str>,
    /// Line breaks since the run's last line.
    newlines: usize,
    /// Only whitespace since the last line break or the file start.
    line_start: bool,
    /// Nothing but trivia seen yet.
    started: bool,
}

impl<'a> MarkdownRuns<'a> {
    fn newline(&mut self) {
        self.newlines += 1;
        self.line_start = true;
    }

    /// Adds a `///` comment to the runs; false for any other line comment.
    fn member(&mut self, text: &'a str, comments: &mut BTreeMap<String, usize>) -> bool {
        if !text.starts_with("///") || text.starts_with("////") {
            return false;
        }
        let at_line_start = self.line_start || !self.started;
        if !at_line_start || self.newlines != 1 {
            self.end_run(comments);
        }
        self.run.push(text);
        if !at_line_start {
            self.doc_comment(comments);
        }
        self.newlines = 0;
        self.line_start = false;
        self.started = true;
        true
    }

    /// A comment that is not a run member: the run ends, and waits for the
    /// token after it.
    fn end_run(&mut self, comments: &mut BTreeMap<String, usize>) {
        if !self.run.is_empty() {
            Self::add(&self.waiting, false, comments);
            self.waiting = std::mem::take(&mut self.run);
        }
        self.line_start = false;
        self.started = true;
    }

    /// A doc comment that is not a run member: the runs before it are not
    /// documentation.
    fn doc_comment(&mut self, comments: &mut BTreeMap<String, usize>) {
        self.end_run(comments);
        Self::add(&std::mem::take(&mut self.waiting), false, comments);
    }

    /// Code: the run ends, and the run before the token is documentation
    /// when `documented()`, which is asked only when a run waits.
    fn token(&mut self, documented: impl FnOnce() -> bool, comments: &mut BTreeMap<String, usize>) {
        self.end_run(comments);
        if !self.waiting.is_empty() {
            let documentation = documented();
            Self::add(&std::mem::take(&mut self.waiting), documentation, comments);
        }
    }

    fn add(run: &[&str], documentation: bool, comments: &mut BTreeMap<String, usize>) {
        if run.is_empty() {
            return;
        }
        let form = doc_comments::markdown(run, documentation);
        *comments.entry(format!("markdown:{form}")).or_default() += 1;
    }
}

/// A comment's text, ignoring delimiters, star decoration, blank lines, and
/// trailing whitespace. Indentation stays where re-indentation cannot change
/// it: after the `*` (and one space) of a star line, and relative to the
/// other continuation lines when no line starts with `*`.
#[must_use]
fn canonical_comment_text<'t>(text: &'t str) -> String {
    let doc = text.starts_with("/**");
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
        .or_else(|| text.strip_prefix("/*"))
        .unwrap_or(text);
    let star_text = |line: &'t str| {
        line.trim_start().strip_prefix('*').map_or_else(
            || line.trim(),
            |after| {
                let after = after.trim_end();
                after.strip_prefix(' ').unwrap_or(after)
            },
        )
    };
    let mut lines = body
        .split(['\r', '\n'])
        .filter(|line| !line.trim().is_empty());
    let mut canonical: Vec<&str> = Vec::new();
    if !body.contains(['\r', '\n']) {
        canonical.push(body.trim());
    } else if doc {
        canonical.extend(lines.map(star_text));
    } else {
        let opener = body.split(['\r', '\n']).next().unwrap_or("").trim();
        if !opener.is_empty() {
            lines.next();
        }
        let rest: Vec<&str> = lines.collect();
        canonical.push(opener);
        if opener.starts_with('*') || rest.iter().any(|line| line.trim_start().starts_with('*')) {
            canonical.extend(rest.iter().map(|line| star_text(line)));
        } else {
            let indentation = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
            let first = rest.first().copied().unwrap_or("");
            let shared = rest.iter().fold(indentation(first), |shared, line| {
                first
                    .bytes()
                    .zip(line.bytes())
                    .take(shared)
                    .take_while(|(left, right)| left == right)
                    .count()
            });
            canonical.extend(rest.iter().map(|line| &line.trim_end()[shared..]));
        }
    }
    canonical.retain(|line| !line.is_empty());
    canonical.join("\n")
}

/// The tree edits a language's formatter may make, which the structure
/// fingerprint is blind to.
pub struct StructurePolicy<K: 'static> {
    /// Punctuation the formatter may insert or drop.
    pub normalizable_punctuation: &'static [K],
    /// Nodes whose children are order-insensitive (recovery-free children
    /// only).
    pub unordered_nodes: &'static [K],
    /// Nodes whose keyword tokens are order-insensitive.
    pub unordered_keywords: &'static [K],
    /// Child kinds of an `unordered_keywords` node that may cross its other
    /// children but keep their order among themselves.
    pub unordered_keywords_ordered_children: &'static [K],
    /// Child kinds that may reorder among their own positions.
    pub reorderable_children: &'static [K],
    /// Single-child wrappers without delimiters of their own.
    pub elidable_wrappers: &'static [K],
    /// The wrapper of a promoted statement body, transparent only directly
    /// under `brace_promoting_parents`.
    pub promoted_body_wrapper: Option<K>,
    pub brace_promoting_parents: &'static [K],
    /// Nodes that may be dropped.
    pub elidable_nodes: &'static [K],
    /// Tokens the parser may split, counted by character in the token
    /// multiset (a split token's pieces relex as one where they touch).
    pub splittable_tokens: &'static [K],
}

/// Reports where two fingerprints first diverge.
#[must_use]
fn describe_divergence(expected: &str, actual: &str) -> String {
    const WINDOW: usize = 160;
    let prefix = expected
        .bytes()
        .zip(actual.bytes())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    let window = |text: &str| {
        let mut start = prefix.saturating_sub(WINDOW / 2);
        while !text.is_char_boundary(start.min(text.len())) {
            start -= 1;
        }
        let mut end = (prefix + WINDOW).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        let start = start.min(end);
        format!("…{}…", &text[start..end])
    };
    format!(
        "first divergence at byte {prefix}\nexpected: {}\nactual:   {}",
        window(expected),
        window(actual)
    )
}

fn kind<L: Language>(node: SyntaxNode<'_>) -> L::Kind {
    L::from_raw(node.kind())
}

/// Child elements of a node: its slots, or its raw children when malformed.
fn child_elements(node: SyntaxNode<'_>) -> Vec<SyntaxElement<'_>> {
    node.children().collect()
}

fn is_transparent<L: Language>(
    node: SyntaxNode<'_>,
    parent: Option<L::Kind>,
    policy: &StructurePolicy<L::Kind>,
) -> bool {
    let kind = kind::<L>(node);
    policy.elidable_wrappers.contains(&kind)
        || (policy.promoted_body_wrapper == Some(kind)
            && parent.is_some_and(|parent| policy.brace_promoting_parents.contains(&parent)))
}

fn canonicalizable<L: Language>(node: SyntaxNode<'_>, kinds: &[L::Kind]) -> bool {
    kinds.contains(&kind::<L>(node)) && node.is_recovery_free()
}

fn needs_buffer<L: Language>(
    node: SyntaxNode<'_>,
    parent: Option<L::Kind>,
    policy: &StructurePolicy<L::Kind>,
) -> bool {
    let kind = kind::<L>(node);
    is_transparent::<L>(node, parent, policy)
        || policy.unordered_nodes.contains(&kind)
        || policy.unordered_keywords.contains(&kind)
        || node.children().any(|child| {
            child
                .as_node()
                .is_some_and(|child| canonicalizable::<L>(child, policy.reorderable_children))
        })
}

enum Step<'a, K> {
    Enter {
        node: SyntaxNode<'a>,
        parent: Option<K>,
        separated: bool,
    },
    Token(SyntaxToken<'a>),
    Close,
}

/// A trivia-insensitive rendering of a tree's kinds, nesting and significant
/// tokens, up to the normalizations `policy` declares. Streams with an explicit
/// stack (deep left-nested spines are fine); only reordering or collapsing
/// nodes recurse, and those nest no deeper than the parser's depth limit.
#[must_use]
fn structure_fingerprint<L: Language>(
    root: SyntaxNode<'_>,
    policy: &StructurePolicy<L::Kind>,
) -> String {
    let mut out = String::new();
    write_structure::<L>(&mut out, root, None, false, policy);
    out
}

fn write_structure<L: Language>(
    out: &mut String,
    node: SyntaxNode<'_>,
    parent: Option<L::Kind>,
    separated: bool,
    policy: &StructurePolicy<L::Kind>,
) {
    let mut steps = vec![Step::Enter {
        node,
        parent,
        separated,
    }];
    while let Some(step) = steps.pop() {
        match step {
            Step::Close => out.push(')'),
            Step::Token(token) => {
                write!(out, " {}={}", L::kind_name(token.kind()), token.text())
                    .expect("write to String");
            }
            Step::Enter {
                node,
                parent,
                separated,
            } => {
                let mark = out.len();
                if separated {
                    out.push(' ');
                }
                if needs_buffer::<L>(node, parent, policy) {
                    let rendered = render_buffered::<L>(node, parent, policy);
                    if rendered.is_empty() {
                        out.truncate(mark);
                    } else {
                        out.push_str(&rendered);
                    }
                    continue;
                }
                let kind = kind::<L>(node);
                out.push('(');
                out.push_str(L::kind_name(node.kind()));
                steps.push(Step::Close);
                for child in child_elements(node).into_iter().rev() {
                    match child {
                        SyntaxElement::Node(child) => {
                            if policy.elidable_nodes.contains(&self::kind::<L>(child)) {
                                continue;
                            }
                            steps.push(Step::Enter {
                                node: child,
                                parent: Some(kind),
                                separated: true,
                            });
                        }
                        SyntaxElement::Token(token) => {
                            if policy
                                .normalizable_punctuation
                                .contains(&L::from_raw(token.kind()))
                            {
                                continue;
                            }
                            steps.push(Step::Token(token));
                        }
                    }
                }
            }
        }
    }
}

fn render_buffered<L: Language>(
    node: SyntaxNode<'_>,
    parent: Option<L::Kind>,
    policy: &StructurePolicy<L::Kind>,
) -> String {
    let kind = kind::<L>(node);
    let transparent = is_transparent::<L>(node, parent, policy);
    let unordered = policy.unordered_nodes.contains(&kind);
    let unordered_keywords = policy.unordered_keywords.contains(&kind);
    let mut keywords = Vec::new();
    let mut slots: Vec<String> = Vec::new();
    let mut ordered_child = Vec::new();
    let mut reorderable = Vec::new();
    for child in child_elements(node) {
        match child {
            SyntaxElement::Node(child) => {
                if policy.elidable_nodes.contains(&self::kind::<L>(child)) {
                    continue;
                }
                let mut rendered = String::new();
                write_structure::<L>(&mut rendered, child, Some(kind), false, policy);
                if rendered.is_empty() {
                    continue;
                }
                let canonical = if unordered {
                    child.is_recovery_free()
                } else {
                    canonicalizable::<L>(child, policy.reorderable_children)
                };
                if canonical {
                    reorderable.push(slots.len());
                }
                ordered_child.push(
                    unordered_keywords
                        && canonicalizable::<L>(child, policy.unordered_keywords_ordered_children),
                );
                slots.push(rendered);
            }
            SyntaxElement::Token(token) => {
                if policy
                    .normalizable_punctuation
                    .contains(&L::from_raw(token.kind()))
                {
                    continue;
                }
                let rendered = format!("{}={}", L::kind_name(token.kind()), token.text());
                if unordered_keywords {
                    keywords.push(rendered);
                } else {
                    if unordered {
                        reorderable.push(slots.len());
                    }
                    slots.push(rendered);
                }
            }
        }
    }
    if unordered_keywords {
        keywords.sort_unstable();
        let mut partitioned: Vec<(bool, String)> = ordered_child.into_iter().zip(slots).collect();
        partitioned.sort_by_key(|&(ordered, _)| !ordered);
        keywords.extend(partitioned.into_iter().map(|(_, rendered)| rendered));
        slots = keywords;
    } else if reorderable.len() > 1 {
        let mut moved: Vec<String> = reorderable
            .iter()
            .map(|&index| slots[index].clone())
            .collect();
        moved.sort_unstable();
        for (&index, rendered) in reorderable.iter().zip(moved) {
            slots[index] = rendered;
        }
    }
    if transparent && slots.len() <= 1 {
        return slots.into_iter().next().unwrap_or_default();
    }
    let mut out = format!("({}", L::kind_name(node.kind()));
    for slot in slots {
        out.push(' ');
        out.push_str(&slot);
    }
    out.push(')');
    out
}

/// The entries of `left` that `right` lacks, with the missing counts.
fn multiset_difference(
    left: &BTreeMap<String, usize>,
    right: &BTreeMap<String, usize>,
) -> BTreeMap<String, usize> {
    left.iter()
        .filter_map(|(key, &count)| {
            let missing = count.saturating_sub(right.get(key).copied().unwrap_or(0));
            (missing > 0).then(|| (key.clone(), missing))
        })
        .collect()
}

/// Checks the oracle for `source` at `options`, returning the failed checks
/// as `(check, detail)` pairs. Panics in the formatter propagate.
pub fn check_source<C: CorpusLanguage>(
    language: &C,
    source: &str,
    options: &FormatOptions,
) -> Vec<(&'static str, String)> {
    let mut failures = Vec::new();
    let input = parse_facts(language, source);
    let clean = input.diagnostics.is_empty();
    let formatted = language.format(source, options);
    let output = parse_facts(language, &formatted);
    if clean && !output.diagnostics.is_empty() {
        failures.push((
            "reparse-dirty",
            format!("diagnostics: {:?}\n{formatted}", output.diagnostics),
        ));
    }
    if !clean && diagnostic_codes(&input.diagnostics) != diagnostic_codes(&output.diagnostics) {
        failures.push((
            "diagnostics-changed",
            format!(
                "expected: {:?}\nactual: {:?}",
                diagnostic_codes(&input.diagnostics),
                diagnostic_codes(&output.diagnostics)
            ),
        ));
    }
    if !clean && input.tokens != output.tokens {
        failures.push((
            "tokens-changed",
            format!(
                "expected: {:?}\nactual: {:?}\n{formatted}",
                multiset_difference(&input.tokens, &output.tokens),
                multiset_difference(&output.tokens, &input.tokens)
            ),
        ));
    }
    if clean && input.structure != output.structure {
        failures.push((
            "structure-changed",
            describe_divergence(&input.structure, &output.structure),
        ));
    }
    if input.comments != output.comments {
        failures.push((
            "comments-changed",
            format!(
                "expected: {:?}\nactual: {:?}",
                input.comments, output.comments
            ),
        ));
    }
    let repeated = language.format(&formatted, options);
    if repeated != formatted {
        failures.push((
            "not-idempotent",
            format!("--- first\n{formatted}--- second\n{repeated}"),
        ));
    }
    failures
}
