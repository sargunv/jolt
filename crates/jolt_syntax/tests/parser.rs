//! Guard behavior that a correct grammar never triggers, checked with a toy
//! language whose "grammar" is deliberately broken.

use jolt_syntax::{
    Diagnostic, Event, Language, LexedToken, Lexer, NodeId, NodeSink, ParseOutput, ParsedChildren,
    Parser, RawSyntaxKind, SyntaxFactory, SyntaxNode, TextRange, TextSize, Trivia, build_tree,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Toy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Eof,
    Word,
    Root,
    Bogus,
}

impl Language for Toy {
    type Kind = Kind;
    const EOF: Kind = Kind::Eof;
    const BOGUS: Kind = Kind::Bogus;
    const BRACKETS: &'static [(Kind, Kind)] = &[];
    const EXPECTED_CODE: &'static str = "toy.expected";
    const UNEXPECTED_CODE: &'static str = "toy.unexpected";
    const TOO_DEEP_CODE: &'static str = "toy.too_deep";

    fn to_raw(kind: Kind) -> RawSyntaxKind {
        RawSyntaxKind(kind as u16)
    }

    fn from_raw(raw: RawSyntaxKind) -> Kind {
        [Kind::Eof, Kind::Word, Kind::Root, Kind::Bogus][usize::from(raw.0)]
    }

    fn is_list(_: RawSyntaxKind) -> bool {
        false
    }

    fn kind_name(raw: RawSyntaxKind) -> &'static str {
        ["Eof", "Word", "Root", "Bogus"][usize::from(raw.0)]
    }
}

/// One `Word` token per byte.
struct ToyLexer {
    len: usize,
    pos: usize,
}

impl Lexer<'_, Toy> for ToyLexer {
    fn next_token(&mut self, trivia: &mut Vec<Trivia>) -> LexedToken<Kind> {
        let empty = trivia.len()..trivia.len();
        let start = TextSize::new(self.pos);
        let kind = if self.pos < self.len {
            self.pos += 1;
            Kind::Word
        } else {
            Kind::Eof
        };
        LexedToken {
            kind,
            range: TextRange::new(start, TextSize::new(self.pos)),
            leading: empty.clone(),
            trailing: empty,
        }
    }

    fn finish(self) -> Vec<Diagnostic> {
        Vec::new()
    }
}

struct ToyFactory;

impl SyntaxFactory for ToyFactory {
    type Language = Toy;

    fn make(&self, kind: RawSyntaxKind, _: ParsedChildren<'_>, sink: &mut NodeSink<'_>) -> NodeId {
        sink.raw(kind, kind == Toy::to_raw(Kind::Bogus))
    }
}

fn parser(source: &str) -> Parser<'_, Toy> {
    Parser::new(
        source,
        ToyLexer {
            len: source.len(),
            pos: 0,
        },
    )
}

#[test]
fn loop_guard_advances_a_loop_that_never_consumes() {
    let source = "abc";
    let mut p = parser(source);
    let root = p.start();
    let mut guard = p.loop_guard();
    let mut iterations = 0;
    // A broken grammar loop: its body consumes nothing.
    while guard.check(&mut p) {
        iterations += 1;
    }
    p.bump();
    p.complete(root, Kind::Root);
    let output = p.finish();
    assert_eq!(
        iterations, 3,
        "one iteration per token; the last wrap reaches EOF"
    );
    assert_eq!(output.diagnostics.len(), 3);
    let (tree, _) = build_tree(source, output, &ToyFactory);
    let root = SyntaxNode::root(&tree, source);
    assert_eq!(root.children().count(), 4, "three bogus nodes and EOF");
    assert!(!root.is_recovery_free());
}

#[test]
fn nesting_stops_at_the_depth_limit() {
    fn descend(p: &mut Parser<'_, Toy>, depth: &mut u32) {
        *depth += 1;
        if Parser::nested(p, |p| descend(p, depth)).is_none() {
            p.recover_too_deep(jolt_syntax::TokenSet::EMPTY);
        }
    }
    let source = "ab";
    let mut p = parser(source);
    let root = p.start();
    let mut depth = 0;
    descend(&mut p, &mut depth);
    assert_eq!(depth, jolt_syntax::MAX_DEPTH + 1);
    p.bump();
    p.complete(root, Kind::Root);
    let output = p.finish();
    assert_eq!(output.diagnostics[0].code, Toy::TOO_DEEP_CODE);
    let (tree, _) = build_tree(source, output, &ToyFactory);
    let root = SyntaxNode::root(&tree, source);
    let bogus = root
        .children()
        .next()
        .and_then(jolt_syntax::SyntaxElement::as_node);
    assert!(bogus.is_some_and(|node| node.is_malformed() && node.tokens().count() == 2));
}

#[test]
fn deep_forward_parent_chains_build_iteratively() {
    const DEPTH: u32 = 100_000;
    let mut events: Vec<Event> = (0..DEPTH)
        .map(|index| Event::Start {
            kind: Toy::to_raw(Kind::Root),
            forward_parent: u32::from(index + 1 < DEPTH),
        })
        .collect();
    events.push(Event::Token);
    events.extend((0..DEPTH).map(|_| Event::Finish));
    let output = ParseOutput {
        events,
        tokens: vec![jolt_syntax::TokenData::new(
            Toy::to_raw(Kind::Eof),
            TextSize::default(),
            TextRange::default(),
            0..0,
            0..0,
        )],
        trivia: Vec::new(),
        diagnostics: Vec::new(),
    };
    let (tree, _) = build_tree("", output, &ToyFactory);
    let root = SyntaxNode::root(&tree, "");
    assert_eq!(root.descendants().count(), DEPTH as usize + 1);
}

#[test]
fn separated_lists_fill_missing_items_and_separators() {
    struct ListFactory;
    impl SyntaxFactory for ListFactory {
        type Language = Toy;
        fn make(
            &self,
            kind: RawSyntaxKind,
            input: ParsedChildren<'_>,
            sink: &mut NodeSink<'_>,
        ) -> NodeId {
            if kind == Toy::to_raw(Kind::Root) {
                // Items are words at even byte offsets; separators at odd ones.
                let text = |c: usize| input.token_text(c).unwrap_or_default();
                sink.separated_list(
                    kind,
                    input.len(),
                    1,
                    jolt_syntax::Trailing::Forbidden,
                    |c| text(c) == "i",
                    |c| text(c) == ",",
                )
            } else {
                sink.raw(kind, false)
            }
        }
    }
    // `i , , i ,` → item sep (missing item) sep item sep (missing item).
    let source = "i,,i,";
    let mut p = parser(source);
    let list = p.start();
    for _ in 0..source.len() {
        p.bump();
    }
    p.complete(list, Kind::Root);
    let wrapper = p.start();
    // The wrapper must own EOF; reuse the list kind's sibling.
    p.bump();
    p.complete(wrapper, Kind::Bogus);
    let output = p.finish();
    let mut events = output.events;
    // Make both nodes children of one root.
    events.insert(
        0,
        Event::Start {
            kind: Toy::to_raw(Kind::Word),
            forward_parent: 0,
        },
    );
    events.push(Event::Finish);
    let output = ParseOutput { events, ..output };
    let (tree, _) = build_tree(source, output, &ListFactory);
    let root = SyntaxNode::root(&tree, source);
    let list = root
        .children()
        .next()
        .and_then(jolt_syntax::SyntaxElement::as_node)
        .expect("list");
    let shape: Vec<bool> = list.slots().map(|slot| slot.is_some()).collect();
    assert_eq!(shape, [true, true, false, true, true, true, false]);
    assert!(!list.is_recovery_free());
    assert!(!list.is_malformed());
}
