use super::*;

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

    fn token_text(&self, index: usize) -> &str {
        self.tokens[index]
    }

    fn leading_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
        std::iter::empty()
    }

    fn trailing_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
        std::iter::empty()
    }
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
    let root = tree.root();
    let wrapper = root.children()[0].as_node().unwrap();
    let leaf = wrapper.children()[0].as_node().unwrap();

    assert_eq!(green_text(root), "ab");
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

        fn token_text(&self, _index: usize) -> &str {
            "token"
        }

        fn leading_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
            [GreenTriviaPiece::new(TriviaKind::Whitespace, "  ")].into_iter()
        }

        fn trailing_trivia(&self, _index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
            [GreenTriviaPiece::new(
                TriviaKind::LineComment,
                "// trailing",
            )]
            .into_iter()
        }
    }

    let events = [Event::start_node(ROOT), Event::Token, Event::FinishNode];
    let tree = build_green_tree(&events, &TriviaTokenSource).unwrap();

    assert_eq!(green_text(tree.root()), "  token// trailing");
}

#[test]
fn green_token_text_len_includes_trivia() {
    let token = GreenToken::with_trivia(
        TOKEN,
        "token",
        [GreenTrivia::new(TriviaKind::Whitespace, "  ")],
        [GreenTrivia::new(TriviaKind::LineComment, "// trailing")],
    );

    assert_eq!(token.token_text_len(), 5usize.into());
    assert_eq!(token.text_len(), 18usize.into());
}

#[test]
fn last_token_ignores_empty_trailing_child_nodes() {
    let root = GreenNode::new(
        ROOT,
        [
            GreenToken::new(TOKEN, "a").into(),
            GreenNode::new(EMPTY, []).into(),
        ],
    );
    let root = SyntaxNode::<TestLanguage>::new_root(root);

    assert_eq!(root.last_token().unwrap().text(), "a");
}
