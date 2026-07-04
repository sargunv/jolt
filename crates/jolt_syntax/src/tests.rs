use super::*;
use jolt_text::{TextRange, TextSize};

const ROOT: RawSyntaxKind = RawSyntaxKind::new(0);
const WRAPPER: RawSyntaxKind = RawSyntaxKind::new(1);
const LEAF: RawSyntaxKind = RawSyntaxKind::new(2);
const TOKEN: RawSyntaxKind = RawSyntaxKind::new(3);
const EMPTY: RawSyntaxKind = RawSyntaxKind::new(4);

enum TestLanguage {}

impl Language for TestLanguage {
    type Kind = RawSyntaxKind;

    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
        raw
    }

    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
        kind
    }
}

fn token(start: usize, end: usize) -> SyntaxTokenData {
    SyntaxTokenData::new(
        TOKEN,
        TextRange::new(TextSize::new(start), TextSize::new(end)),
        0..0,
        0..0,
        TextSize::new(end - start),
    )
}

fn parse<'source>(
    source: &'source str,
    events: Vec<Event>,
    tokens: Vec<SyntaxTokenData>,
) -> SyntaxNode<'source, TestLanguage> {
    let (tree, diagnostics) = build_syntax_tree(events, tokens, Vec::new()).unwrap();
    assert!(diagnostics.is_empty());

    SyntaxNode::<TestLanguage>::new_root(source, Box::leak(Box::new(tree)))
}

#[test]
fn completed_marker_can_precede_and_wrap_a_completed_node() {
    let mut events = Vec::new();

    let root = Marker::new(&mut events);
    let leaf = Marker::new(&mut events);
    events.push(Event::Token);
    let leaf = leaf.complete(&mut events, LEAF);
    let wrapper = leaf.precede(&mut events);
    events.push(Event::Token);
    wrapper.complete(&mut events, WRAPPER);
    root.complete(&mut events, ROOT);

    let root = parse("ab", events, vec![token(0, 1), token(1, 2)]);
    let wrapper = root.children().next().expect("wrapper node");
    let leaf = wrapper.children().next().expect("leaf node");

    assert_eq!(
        root.text_range(),
        TextRange::new(0usize.into(), 2usize.into())
    );
    assert_eq!(wrapper.kind(), WRAPPER);
    assert_eq!(leaf.kind(), LEAF);
}

#[test]
fn token_trivia_contributes_to_offsets() {
    let trivia = vec![
        SyntaxTrivia::new(TriviaKind::Whitespace, TextSize::new("  ".len())),
        SyntaxTrivia::new(TriviaKind::LineComment, TextSize::new("// trailing".len())),
    ];
    let token = SyntaxTokenData::new(
        TOKEN,
        TextRange::new(2usize.into(), 7usize.into()),
        0..1,
        1..2,
        TextSize::new("  token// trailing".len()),
    );
    let events = vec![Event::start_node(ROOT), Event::Token, Event::FinishNode];
    let (tree, diagnostics) = build_syntax_tree(events, vec![token], trivia).unwrap();
    assert!(diagnostics.is_empty());

    let root = SyntaxNode::<TestLanguage>::new_root("  token// trailing", &tree);
    let token = root.first_token().unwrap();

    assert_eq!(
        root.text_range(),
        TextRange::new(0usize.into(), 18usize.into())
    );
    assert_eq!(
        token.token_text_range(),
        TextRange::new(2usize.into(), 7usize.into())
    );
    assert_eq!(
        token.text_range(),
        TextRange::new(0usize.into(), 18usize.into())
    );
}

#[test]
fn last_token_ignores_empty_trailing_child_nodes() {
    let events = vec![
        Event::start_node(ROOT),
        Event::Token,
        Event::start_node(EMPTY),
        Event::FinishNode,
        Event::FinishNode,
    ];
    let root = parse("a", events, vec![token(0, 1)]);

    assert_eq!(root.last_token().unwrap().text(), "a");
}

#[test]
fn sibling_accessors_preserve_offsets() {
    let events = vec![
        Event::start_node(ROOT),
        Event::Token,
        Event::start_node(WRAPPER),
        Event::Token,
        Event::FinishNode,
        Event::Token,
        Event::FinishNode,
    ];
    let root = parse("abcd", events, vec![token(0, 1), token(1, 3), token(3, 4)]);
    let wrapper = root.children().next().unwrap();

    assert_eq!(wrapper.offset(), 1usize.into());
    assert_eq!(
        wrapper.first_token().unwrap().token_text_range(),
        TextRange::new(1usize.into(), 3usize.into())
    );

    match wrapper.prev_sibling_or_token().unwrap() {
        SyntaxElement::Token(token) => assert_eq!(token.text(), "a"),
        SyntaxElement::Node(_) => panic!("expected previous token"),
    }

    match wrapper.next_sibling_or_token().unwrap() {
        SyntaxElement::Token(token) => {
            assert_eq!(token.text(), "d");
            assert_eq!(token.offset(), 3usize.into());
        }
        SyntaxElement::Node(_) => panic!("expected next token"),
    }
}

#[test]
fn syntax_node_debug_prints_tree_shape_without_parent_recursion() {
    let events = vec![
        Event::start_node(ROOT),
        Event::start_node(WRAPPER),
        Event::Token,
        Event::FinishNode,
        Event::FinishNode,
    ];
    let root = parse("a", events, vec![token(0, 1)]);

    assert_eq!(
        format!("{root:?}"),
        "RawSyntaxKind(0)\n  RawSyntaxKind(1)\n    RawSyntaxKind(3) \"a\" @ 0..1"
    );
}
