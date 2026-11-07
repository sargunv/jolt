//! The formatter oracle: reparse, structure fingerprint, comment multiset,
//! idempotence.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use jolt_fmt::FormatOptions;
use jolt_syntax::{Diagnostic, Language, SyntaxElement, SyntaxNode, TriviaKind};

/// What a language provides to the oracle, the corpus tests and the sweep.
pub trait CorpusLanguage: Sync {
    type Syntax: Language;
    /// Human-readable language name for assertion messages.
    fn name(&self) -> &'static str;
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
    /// Whether the fixture at `relative` is expected to carry diagnostics.
    fn expects_diagnostics(&self, relative: &str) -> bool;
}

/// Owned facts about one parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFacts {
    pub diagnostics: Vec<Diagnostic>,
    pub comments: BTreeMap<String, usize>,
    pub structure: String,
}

pub fn parse_facts<C: CorpusLanguage>(language: &C, source: &str) -> ParseFacts {
    language.with_parse(source, |root, diagnostics| ParseFacts {
        diagnostics: diagnostics.to_vec(),
        comments: comment_inventory(root),
        structure: structure_fingerprint::<C::Syntax>(root, language.structure_policy()),
    })
}

/// Diagnostic codes with their counts.
#[must_use]
pub fn diagnostic_codes(diagnostics: &[Diagnostic]) -> BTreeMap<&'static str, usize> {
    let mut codes = BTreeMap::new();
    for diagnostic in diagnostics {
        *codes.entry(diagnostic.code).or_default() += 1;
    }
    codes
}

/// Every comment by kind and canonical text.
#[must_use]
pub fn comment_inventory(root: SyntaxNode<'_>) -> BTreeMap<String, usize> {
    let mut comments = BTreeMap::new();
    for token in root.tokens() {
        for piece in token.leading().chain(token.trailing()) {
            let kind = match piece.kind {
                TriviaKind::LineComment => "line",
                TriviaKind::BlockComment | TriviaKind::UnterminatedBlockComment => "block",
                TriviaKind::DocComment => "doc",
                _ => continue,
            };
            *comments
                .entry(format!("{kind}:{}", canonical_comment_text(piece.text)))
                .or_default() += 1;
        }
    }
    comments
}

/// A comment's words, ignoring delimiters, star decoration, and whitespace.
#[must_use]
pub fn canonical_comment_text(text: &str) -> String {
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
    let multiline = body.contains(['\r', '\n']);
    let mut canonical = String::new();
    for line in body.split(['\r', '\n']) {
        let line = line.trim();
        let line = if multiline {
            line.strip_prefix('*').unwrap_or(line)
        } else {
            line
        };
        for word in line.split_whitespace() {
            if !canonical.is_empty() {
                canonical.push(' ');
            }
            canonical.push_str(word);
        }
    }
    canonical
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
}

/// Reports where two fingerprints first diverge.
#[must_use]
pub fn describe_divergence(expected: &str, actual: &str) -> String {
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
    Token(jolt_syntax::SyntaxToken<'a>),
    Close,
}

/// A trivia-insensitive rendering of a tree's kinds, nesting and significant
/// tokens, up to the normalizations `policy` declares. Streams with an explicit
/// stack (deep left-nested spines are fine); only reordering or collapsing
/// nodes recurse, and those nest no deeper than the parser's depth limit.
#[must_use]
pub fn structure_fingerprint<L: Language>(
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
