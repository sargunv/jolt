use super::*;
use crate::green::{GreenElement, GreenToken};
use jolt_text::TextSize;

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

struct TestTokenSource {
    tokens: Vec<&'static str>,
}

impl GreenTokenSource for TestTokenSource {
    fn token_count(&self) -> usize {
        self.tokens.len()
    }

    fn token_kind(&self, _index: usize) -> RawSyntaxKind {
        TOKEN
    }

    fn token_text_len(&self, index: usize) -> TextSize {
        TextSize::new(self.tokens[index].len())
    }

    fn leading_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
        std::iter::empty()
    }

    fn trailing_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
        std::iter::empty()
    }
}

fn token(kind: RawSyntaxKind, text: &'static str) -> GreenToken {
    GreenToken::with_trivia(kind, TextSize::new(text.len()), [], [])
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

    let tree = build_green_tree(
        &events,
        &TestTokenSource {
            tokens: vec!["a", "b"],
        },
    )
    .unwrap();
    let (root, _) = tree.into_parts();
    let wrapper = match &root.children()[0] {
        GreenElement::Node(node) => node,
        GreenElement::Token(_) => panic!("expected wrapper node"),
    };
    let leaf = match &wrapper.children()[0] {
        GreenElement::Node(node) => node,
        GreenElement::Token(_) => panic!("expected leaf node"),
    };

    assert_eq!(root.text_len(), TextSize::new("ab".len()));
    assert_eq!(wrapper.kind(), WRAPPER);
    assert_eq!(leaf.kind(), LEAF);
}

#[test]
fn token_source_supplies_borrowed_trivia_pieces() {
    struct TriviaTokenSource;

    impl GreenTokenSource for TriviaTokenSource {
        fn token_count(&self) -> usize {
            1
        }

        fn token_kind(&self, _index: usize) -> RawSyntaxKind {
            TOKEN
        }

        fn token_text_len(&self, _index: usize) -> TextSize {
            TextSize::new("token".len())
        }

        fn leading_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
            [GreenTriviaPiece::new(
                TriviaKind::Whitespace,
                TextSize::new("  ".len()),
            )]
            .into_iter()
        }

        fn trailing_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
            [GreenTriviaPiece::new(
                TriviaKind::LineComment,
                TextSize::new("// trailing".len()),
            )]
            .into_iter()
        }
    }

    let events = [Event::start_node(ROOT), Event::Token, Event::FinishNode];
    let tree = build_green_tree(&events, &TriviaTokenSource).unwrap();

    let (root, _) = tree.into_parts();
    assert_eq!(root.text_len(), TextSize::new("  token// trailing".len()));
}

#[test]
fn green_token_text_len_includes_trivia() {
    let token = GreenToken::with_trivia(
        TOKEN,
        TextSize::new("token".len()),
        [GreenTrivia::new(
            TriviaKind::Whitespace,
            TextSize::new("  ".len()),
        )],
        [GreenTrivia::new(
            TriviaKind::LineComment,
            TextSize::new("// trailing".len()),
        )],
    );

    assert_eq!(token.token_text_len(), 5usize.into());
    assert_eq!(token.text_len(), 18usize.into());
}

#[test]
fn last_token_ignores_empty_trailing_child_nodes() {
    let root = GreenNode::new(
        ROOT,
        [token(TOKEN, "a").into(), GreenNode::new(EMPTY, []).into()],
    );
    let root = SyntaxNode::<TestLanguage>::new_root("a", root);

    assert_eq!(root.last_token().unwrap().text(), "a");
}

#[test]
fn sibling_accessors_preserve_offsets() {
    let root = GreenNode::new(
        ROOT,
        [
            token(TOKEN, "a").into(),
            GreenNode::new(WRAPPER, [token(TOKEN, "bc").into()]).into(),
            token(TOKEN, "d").into(),
        ],
    );
    let root = SyntaxNode::<TestLanguage>::new_root("abcd", root);

    let wrapper = root.children().next().unwrap();

    assert_eq!(wrapper.offset(), 1usize.into());
    assert_eq!(
        wrapper.first_token().unwrap().token_text_range().start(),
        1usize.into()
    );
    assert_eq!(
        wrapper.first_token().unwrap().token_text_range().end(),
        3usize.into()
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
    let root = GreenNode::new(
        ROOT,
        [GreenNode::new(WRAPPER, [token(TOKEN, "a").into()]).into()],
    );
    let root = SyntaxNode::<TestLanguage>::new_root("a", root);

    assert_eq!(
        format!("{root:?}"),
        "RawSyntaxKind(0)\n  RawSyntaxKind(1)\n    RawSyntaxKind(3) \"a\" @ 0..1"
    );
}
