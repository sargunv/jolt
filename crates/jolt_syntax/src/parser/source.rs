#![allow(
    clippy::inline_always,
    reason = "release profiles show these parser cursor fast paths regress without forced inlining"
)]

use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_text::{TextRange, TextSize};

use crate::{
    CompletedMarker, Event, Language, LanguageLexer, LexedToken, Marker, NodeAnchor,
    SyntaxTokenData, SyntaxTrivia,
};

/// Parser-time identity of the syntax location responsible for a diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnresolvedDiagnosticOwner {
    pub(crate) node: NodeAnchor,
    pub(crate) slot: Option<u16>,
    pub(crate) directly_malformed: bool,
}

impl UnresolvedDiagnosticOwner {
    /// Owns a diagnostic with an entire represented node.
    #[must_use]
    pub const fn node(node: NodeAnchor) -> Self {
        Self {
            node,
            slot: None,
            directly_malformed: false,
        }
    }

    const fn recovery_node(node: NodeAnchor) -> Self {
        Self {
            node,
            slot: None,
            directly_malformed: true,
        }
    }

    /// Owns a diagnostic with one generated physical slot on a represented node.
    #[must_use]
    pub const fn missing_slot(node: NodeAnchor, slot: u16) -> Self {
        Self {
            node,
            slot: Some(slot),
            directly_malformed: false,
        }
    }
}

/// Handle used to assign structural ownership after emitting a diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticMarker(usize);

/// A parser diagnostic whose source range has been captured but whose
/// structural consequence has not yet been selected.
///
/// Pending diagnostics must be consumed by a recovery operation or explicitly
/// reported as non-structural.
#[must_use = "attach this diagnostic to recovery or report it as non-structural"]
pub struct PendingDiagnostic {
    index: usize,
}

struct ParserDiagnostic {
    diagnostic: Diagnostic,
    ownership: DiagnosticOwnership,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DiagnosticOwnership {
    Pending,
    Ownerless,
    Structural(UnresolvedDiagnosticOwner),
}

pub struct ParseEvents {
    pub events: Vec<Event>,
    pub tokens: Vec<SyntaxTokenData>,
    pub trivia: Vec<SyntaxTrivia>,
    pub diagnostics: Vec<Diagnostic>,
    pub diagnostic_owners: Vec<Option<UnresolvedDiagnosticOwner>>,
}

pub struct Parser<'source, L: Language> {
    pub source: &'source str,
    pub buffer: TokenBuffer<'source, L>,
    cursor: TokenCursor,
    events: Vec<Event>,
    diagnostics: Vec<ParserDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorCheckpoint {
    pos: usize,
}

#[derive(Clone, Copy)]
pub struct TokenCursor {
    pos: usize,
}

impl<'source, L: Language> Parser<'source, L> {
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            buffer: TokenBuffer::new(source),
            cursor: TokenCursor::new(),
            events: Vec::with_capacity(L::initial_event_capacity(source.len())),
            diagnostics: Vec::new(),
        }
    }

    /// Finishes parser event and diagnostic production.
    ///
    /// # Panics
    ///
    /// Panics if a pending diagnostic was not attached to recovery or
    /// explicitly reported as non-structural.
    pub fn finish(self) -> ParseEvents {
        let events = self.events;
        let committed_len = self.cursor.position();
        let (tokens, trivia, mut diagnostics) = self.buffer.finish(committed_len);
        let mut diagnostic_owners = vec![None; diagnostics.len()];
        diagnostics.reserve(self.diagnostics.len());
        diagnostic_owners.reserve(self.diagnostics.len());
        for parser_diagnostic in self.diagnostics {
            diagnostics.push(parser_diagnostic.diagnostic);
            diagnostic_owners.push(match parser_diagnostic.ownership {
                DiagnosticOwnership::Pending => {
                    panic!("pending parser diagnostic must be consumed before finish")
                }
                DiagnosticOwnership::Ownerless => None,
                DiagnosticOwnership::Structural(owner) => Some(owner),
            });
        }
        ParseEvents {
            events,
            tokens,
            trivia,
            diagnostics,
            diagnostic_owners,
        }
    }

    pub const fn position(&self) -> usize {
        self.cursor.position()
    }

    pub fn expect(&mut self, kind: L::Kind, message: &str) {
        if !self.eat(kind) {
            self.expected_here(message);
        }
    }

    pub fn expect_owned(&mut self, kind: L::Kind, message: &str, owner: NodeAnchor, slot: u16) {
        if !self.eat(kind) {
            self.expected_owned_slot(message, owner, slot);
        }
    }

    pub fn expected_owned_node(&mut self, message: &str, owner: NodeAnchor) {
        let diagnostic = self.pending_expected(message);
        self.record_recovery(
            UnresolvedDiagnosticOwner::node(owner),
            diagnostic,
            std::iter::empty(),
        );
    }

    pub fn expected_owned_slot(&mut self, message: &str, owner: NodeAnchor, slot: u16) {
        let diagnostic = self.pending_expected(message);
        self.record_recovery(
            UnresolvedDiagnosticOwner::missing_slot(owner, slot),
            diagnostic,
            std::iter::empty(),
        );
    }

    pub fn unexpected_owned_node(&mut self, message: &str, owner: NodeAnchor) {
        let diagnostic = self.pending_unexpected(message);
        self.record_recovery(
            UnresolvedDiagnosticOwner::node(owner),
            diagnostic,
            std::iter::empty(),
        );
    }

    pub fn eat(&mut self, kind: L::Kind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn at(&mut self, kind: L::Kind) -> bool {
        self.current_kind() == kind
    }

    pub fn at_eof(&mut self) -> bool {
        self.current_kind() == L::eof_kind()
    }

    #[inline(always)]
    pub fn current_kind(&mut self) -> L::Kind {
        self.cursor.kind(&mut self.buffer)
    }

    #[inline(always)]
    pub fn nth_kind(&mut self, n: usize) -> L::Kind {
        self.cursor.nth_kind(&mut self.buffer, n)
    }

    #[inline(always)]
    pub fn kind_at(&mut self, index: usize) -> L::Kind {
        self.buffer.kind_at(index)
    }

    pub fn current_text(&mut self) -> Option<&'source str> {
        self.cursor.text(self.source, &mut self.buffer)
    }

    pub fn text_at(&mut self, index: usize) -> Option<&'source str> {
        self.buffer.text_at(self.source, index)
    }

    pub fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        self.buffer.tokens_are_adjacent(index, count)
    }

    pub fn bump(&mut self) {
        self.cursor.bump(&mut self.buffer);
        self.events.push(Event::Token);
    }

    pub fn fork_cursor(&self) -> TokenCursor {
        self.cursor.fork()
    }

    pub fn expected_here(&mut self, message: &str) -> DiagnosticMarker {
        let diagnostic = self.pending_expected(message);
        self.report_non_structural(diagnostic)
    }

    pub fn unexpected_here(&mut self, message: &str) -> DiagnosticMarker {
        let diagnostic = self.pending_unexpected(message);
        self.report_non_structural(diagnostic)
    }

    /// Adds a parser error at the current token, or at the last token if the cursor is past EOF.
    ///
    /// # Panics
    ///
    /// Panics if the parser token stream does not contain EOF.
    pub fn error_here(&mut self, code: DiagnosticCodeId, message: &str) -> DiagnosticMarker {
        let diagnostic = self.pending_error(code, message);
        self.report_non_structural(diagnostic)
    }

    /// Captures an "expected syntax" diagnostic before recovery consumes input.
    pub fn pending_expected(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_expected_at(0, message)
    }

    /// Captures an "expected syntax" diagnostic at a bounded token lookahead
    /// without consuming parser input.
    pub fn pending_expected_at(&mut self, offset: usize, message: &str) -> PendingDiagnostic {
        self.pending_error_at(offset, L::expected_diagnostic_code(), message)
    }

    /// Captures an "unexpected syntax" diagnostic before recovery consumes
    /// input.
    pub fn pending_unexpected(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_unexpected_at(0, message)
    }

    /// Captures an "unexpected syntax" diagnostic at a bounded token lookahead
    /// without consuming parser input.
    pub fn pending_unexpected_at(&mut self, offset: usize, message: &str) -> PendingDiagnostic {
        self.pending_error_at(offset, L::unexpected_diagnostic_code(), message)
    }

    /// Captures a language-specific parser diagnostic before recovery consumes
    /// input.
    ///
    /// # Panics
    ///
    /// Panics if the parser token stream does not contain EOF.
    pub fn pending_error(&mut self, code: DiagnosticCodeId, message: &str) -> PendingDiagnostic {
        self.pending_error_at(0, code, message)
    }

    /// Captures a language-specific parser diagnostic at a bounded token
    /// lookahead without consuming parser input.
    ///
    /// # Panics
    ///
    /// Panics if `offset` overflows the parser position or the parser token
    /// stream does not contain EOF.
    pub fn pending_error_at(
        &mut self,
        offset: usize,
        code: DiagnosticCodeId,
        message: &str,
    ) -> PendingDiagnostic {
        let index = self
            .position()
            .checked_add(offset)
            .expect("diagnostic lookahead position must not overflow");
        let range = self
            .buffer
            .range_at(index)
            .or_else(|| self.buffer.last_token_range())
            .expect("parser token stream must include EOF");
        let index = self.diagnostics.len();
        self.diagnostics.push(ParserDiagnostic {
            diagnostic: Diagnostic {
                code,
                severity: Severity::Error,
                stage: DiagnosticStage::Parser,
                message: message.to_owned(),
                range: Some(range),
            },
            ownership: DiagnosticOwnership::Pending,
        });
        PendingDiagnostic { index }
    }

    /// Reports a parser diagnostic that has no structural recovery consequence.
    pub fn report_non_structural(&mut self, diagnostic: PendingDiagnostic) -> DiagnosticMarker {
        let index = diagnostic.index;
        self.finalize_pending(diagnostic, DiagnosticOwnership::Ownerless);
        DiagnosticMarker(index)
    }

    /// Completes a malformed node with the diagnostics that structurally caused
    /// that recovery.
    ///
    /// `diagnostics` must contain at least one item. Arrays permit multiple
    /// diagnostics to share this node without allocating an intermediate
    /// collection.
    ///
    /// # Panics
    ///
    /// Panics if `diagnostics` is empty, or under the same conditions as
    /// [`Parser::complete`].
    pub fn complete_recovery(
        &mut self,
        marker: Marker,
        kind: L::Kind,
        diagnostics: impl IntoIterator<Item = PendingDiagnostic>,
    ) -> CompletedMarker {
        let mut diagnostics = diagnostics.into_iter();
        let first = diagnostics
            .next()
            .expect("structural recovery must record at least one diagnostic cause");
        let owner = UnresolvedDiagnosticOwner::recovery_node(marker.anchor());
        let completed = self.complete(marker, kind);
        self.record_recovery(owner, first, diagnostics);
        completed
    }

    /// Records a required empty slot and all diagnostics that structurally
    /// caused it.
    ///
    /// `diagnostics` must contain at least one item. Arrays permit multiple
    /// diagnostics to share this slot without allocating an intermediate
    /// collection.
    ///
    /// # Panics
    ///
    /// Panics if `diagnostics` is empty.
    pub fn missing_required_slot(
        &mut self,
        owner: NodeAnchor,
        slot: u16,
        diagnostics: impl IntoIterator<Item = PendingDiagnostic>,
    ) {
        let mut diagnostics = diagnostics.into_iter();
        let first = diagnostics
            .next()
            .expect("structural recovery must record at least one diagnostic cause");
        self.record_recovery(
            UnresolvedDiagnosticOwner::missing_slot(owner, slot),
            first,
            diagnostics,
        );
    }

    fn record_recovery(
        &mut self,
        owner: UnresolvedDiagnosticOwner,
        first: PendingDiagnostic,
        diagnostics: impl IntoIterator<Item = PendingDiagnostic>,
    ) {
        self.finalize_pending(first, DiagnosticOwnership::Structural(owner));
        for diagnostic in diagnostics {
            self.finalize_pending(diagnostic, DiagnosticOwnership::Structural(owner));
        }
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "consuming the non-Copy handle enforces one diagnostic finalization"
    )]
    fn finalize_pending(&mut self, diagnostic: PendingDiagnostic, ownership: DiagnosticOwnership) {
        let PendingDiagnostic { index } = diagnostic;
        let record = self
            .diagnostics
            .get_mut(index)
            .expect("pending diagnostic must belong to this parser");
        assert_eq!(
            record.ownership,
            DiagnosticOwnership::Pending,
            "pending diagnostic consumed twice"
        );
        record.ownership = ownership;
    }

    /// Assigns exact structural ownership to a parser diagnostic.
    ///
    /// # Panics
    ///
    /// Panics if the marker came from another parser or ownership was already
    /// assigned to the diagnostic.
    pub fn own_diagnostic(
        &mut self,
        diagnostic: DiagnosticMarker,
        owner: UnresolvedDiagnosticOwner,
    ) {
        let diagnostic = self
            .diagnostics
            .get_mut(diagnostic.0)
            .expect("diagnostic marker must belong to this parser");
        assert_eq!(
            diagnostic.ownership,
            DiagnosticOwnership::Ownerless,
            "diagnostic owner assigned twice"
        );
        diagnostic.ownership = DiagnosticOwnership::Structural(owner);
    }

    pub fn start(&mut self) -> Marker {
        Marker::new(&mut self.events)
    }

    pub fn complete(&mut self, marker: Marker, kind: L::Kind) -> CompletedMarker {
        marker.complete(&mut self.events, L::kind_to_raw(kind))
    }

    pub fn precede(&mut self, marker: CompletedMarker) -> Marker {
        marker.precede(&mut self.events)
    }

    pub fn abandon(&mut self, marker: Marker) {
        marker.abandon(&mut self.events);
    }
}

impl TokenCursor {
    const fn new() -> Self {
        Self { pos: 0 }
    }

    #[must_use]
    pub const fn position(self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn kind<L: Language>(self, buffer: &mut TokenBuffer<'_, L>) -> L::Kind {
        buffer.kind_at(self.pos)
    }

    #[inline(always)]
    pub fn nth_kind<L: Language>(self, buffer: &mut TokenBuffer<'_, L>, n: usize) -> L::Kind {
        buffer.kind_at(self.pos + n)
    }

    pub fn text<'source, L: Language>(
        self,
        source: &'source str,
        buffer: &mut TokenBuffer<'source, L>,
    ) -> Option<&'source str> {
        let range = self.range(buffer)?;
        Some(source_text(source, range))
    }

    pub fn range<L: Language>(self, buffer: &mut TokenBuffer<'_, L>) -> Option<TextRange> {
        buffer.range_at(self.pos)
    }

    pub fn bump<L: Language>(&mut self, buffer: &mut TokenBuffer<'_, L>) {
        buffer.ensure(self.pos);
        self.pos += 1;
    }

    #[must_use]
    pub const fn checkpoint(self) -> CursorCheckpoint {
        CursorCheckpoint { pos: self.pos }
    }

    pub fn rewind(&mut self, checkpoint: CursorCheckpoint) {
        self.pos = checkpoint.pos;
    }

    #[must_use]
    pub const fn fork(self) -> Self {
        self
    }
}

fn source_text(source: &str, range: TextRange) -> &str {
    let start = range.start().get();
    let end = range.end().get();
    &source[start..end]
}

pub struct TokenBuffer<'source, L: Language> {
    lexer: L::Lexer<'source>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}

impl<'source, L: Language> TokenBuffer<'source, L> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: L::Lexer::new(source),
            tokens: Vec::with_capacity(L::initial_token_capacity(source.len())),
            trivia: Vec::with_capacity(L::initial_trivia_capacity(source.len())),
        }
    }

    #[inline(always)]
    fn kind_at(&mut self, index: usize) -> L::Kind {
        self.ensure(index);
        self.tokens
            .get(index)
            .map_or(L::eof_kind(), |token| L::kind_from_raw(token.raw_kind()))
    }

    fn range_at(&mut self, index: usize) -> Option<TextRange> {
        self.ensure(index);
        self.tokens
            .get(index)
            .map(SyntaxTokenData::token_text_range)
    }

    pub fn text_at(&mut self, source: &'source str, index: usize) -> Option<&'source str> {
        let range = self.range_at(index)?;
        Some(source_text(source, range))
    }

    fn last_token_range(&mut self) -> Option<TextRange> {
        self.ensure(0);
        self.tokens.last().map(SyntaxTokenData::token_text_range)
    }

    pub fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        if count <= 1 {
            return true;
        }

        self.ensure(index + count - 1);
        let Some(tokens) = self.tokens.get(index..index + count) else {
            return false;
        };

        tokens.windows(2).all(|window| {
            let [left, right] = window else {
                return true;
            };
            left.token_text_range().end() == right.token_text_range().start()
                && left.trailing().is_empty()
                && right.leading().is_empty()
        })
    }

    pub fn trivia_has_newline(&self, range: Range<usize>) -> bool {
        self.trivia[range.start..range.end]
            .iter()
            .any(|trivia| trivia.kind() == crate::TriviaKind::Newline)
    }

    pub fn newline_before(&mut self, index: usize) -> bool {
        self.ensure(index);
        self.tokens
            .get(index)
            .is_some_and(|token| self.trivia_has_newline(token.leading()))
    }

    pub fn newline_between(&mut self, left: usize, right: usize) -> bool {
        self.ensure(right);
        let left_has_newline = self
            .tokens
            .get(left)
            .is_some_and(|token| self.trivia_has_newline(token.trailing()));
        let right_has_newline = self
            .tokens
            .get(right)
            .is_some_and(|token| self.trivia_has_newline(token.leading()));
        left_has_newline || right_has_newline
    }

    #[inline(always)]
    fn ensure(&mut self, index: usize) {
        if index < self.tokens.len() {
            return;
        }
        while self.tokens.len() <= index
            && self
                .tokens
                .last()
                .is_none_or(|token| token.raw_kind() != L::kind_to_raw(L::eof_kind()))
        {
            let token = self.lexer.next_token_into(&mut self.trivia);
            self.push_token(token);
        }
    }

    fn push_token(&mut self, token: LexedToken<L>) {
        if let Some(kinds) = L::split_token(&token) {
            self.push_split_token(&token, kinds);
        } else {
            self.push_buffered_token(token.kind, token.range, token.leading, token.trailing);
        }
    }

    fn push_split_token(&mut self, token: &LexedToken<L>, kinds: &[L::Kind]) {
        let start = token.range.start();
        let last_index = kinds.len().saturating_sub(1);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let token_start = start + TextSize::new(index);
            self.push_buffered_token(
                kind,
                TextRange::new(token_start, token_start + TextSize::new(1)),
                if index == 0 {
                    token.leading.start..token.leading.end
                } else {
                    self.empty_trivia()
                },
                if index == last_index {
                    token.trailing.start..token.trailing.end
                } else {
                    self.empty_trivia()
                },
            );
        }
    }

    fn push_buffered_token(
        &mut self,
        kind: L::Kind,
        range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
    ) {
        let leading_len = self.trivia_text_len(&leading);
        let text_len = leading_len + range.len() + self.trivia_text_len(&trailing);
        let full_start = range.start() - leading_len;
        self.tokens.push(SyntaxTokenData::new(
            L::kind_to_raw(kind),
            TextRange::new(full_start, full_start + text_len),
            range,
            leading,
            trailing,
        ));
    }

    fn trivia_text_len(&self, range: &Range<usize>) -> TextSize {
        self.trivia[range.start..range.end]
            .iter()
            .fold(TextSize::new(0), |len, trivia| len + trivia.text_len())
    }

    fn empty_trivia(&self) -> Range<usize> {
        self.trivia.len()..self.trivia.len()
    }

    fn finish(
        mut self,
        committed_len: usize,
    ) -> (Vec<SyntaxTokenData>, Vec<SyntaxTrivia>, Vec<Diagnostic>) {
        self.tokens.truncate(committed_len.min(self.tokens.len()));
        let trivia_len = self
            .tokens
            .iter()
            .flat_map(|token| [token.leading().end, token.trailing().end])
            .max()
            .unwrap_or(0);
        self.trivia.truncate(trivia_len);
        let diagnostics = self.lexer.finish();
        (self.tokens, self.trivia, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use jolt_diagnostics::{DiagnosticCodeId, DiagnosticStage};
    use jolt_text::{TextRange, TextSize};

    use crate::{
        BuildSyntaxTreeError, FactoryNode, FactorySlot, Language, LanguageLexer, LexedToken,
        ParsedChildren, RawSyntaxKind, SyntaxFactory, SyntaxNode, SyntaxSlot, SyntaxTreeSink,
        SyntaxTrivia, build_parser_syntax_tree,
    };

    use super::Parser;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestKind {
        Root = 1,
        Bogus = 2,
        Delimited = 3,
        Missing = 4,
        Word = 5,
        Eof = 6,
        RecoveredValid = 7,
    }

    struct TestLanguage;

    impl Language for TestLanguage {
        type Kind = TestKind;
        type Lexer<'source> = TestLexer<'source>;
        type NormalizationAuthority = ();

        fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
            match raw.get() {
                1 => TestKind::Root,
                2 => TestKind::Bogus,
                3 => TestKind::Delimited,
                4 => TestKind::Missing,
                5 => TestKind::Word,
                6 => TestKind::Eof,
                7 => TestKind::RecoveredValid,
                _ => panic!("unknown test kind"),
            }
        }

        fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
            RawSyntaxKind::new(kind as u16)
        }

        fn eof_kind() -> Self::Kind {
            TestKind::Eof
        }

        fn expected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test/expected")
        }

        fn unexpected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test/unexpected")
        }

        fn split_token(_token: &LexedToken<Self>) -> Option<&'static [Self::Kind]> {
            None
        }
    }

    struct TestLexer<'source> {
        source: &'source str,
        emitted_word: bool,
    }

    impl<'source> LanguageLexer<'source> for TestLexer<'source> {
        type Language = TestLanguage;

        fn new(source: &'source str) -> Self {
            Self {
                source,
                emitted_word: false,
            }
        }

        fn next_token_into(
            &mut self,
            trivia: &mut Vec<SyntaxTrivia>,
        ) -> LexedToken<Self::Language> {
            let empty = trivia.len()..trivia.len();
            if !self.source.is_empty() && !self.emitted_word {
                self.emitted_word = true;
                return LexedToken {
                    kind: TestKind::Word,
                    range: TextRange::new(TextSize::new(0), TextSize::new(self.source.len())),
                    leading: empty.clone(),
                    trailing: empty,
                };
            }
            let end = TextSize::new(self.source.len());
            LexedToken {
                kind: TestKind::Eof,
                range: TextRange::empty(end),
                leading: empty.clone(),
                trailing: empty,
            }
        }

        fn finish(self) -> Vec<jolt_diagnostics::Diagnostic> {
            Vec::new()
        }
    }

    struct TestFactory;

    impl SyntaxFactory for TestFactory {
        fn make_syntax(
            &self,
            kind: RawSyntaxKind,
            _children: ParsedChildren<'_>,
            sink: &mut SyntaxTreeSink<'_>,
        ) -> Result<FactoryNode, BuildSyntaxTreeError> {
            Ok(match kind.get() {
                2 | 3 => sink.raw_malformed(kind),
                4 => sink.fixed(kind, [FactorySlot::Missing]),
                _ => sink.raw(kind),
            })
        }
    }

    fn build(
        source: &str,
        parse: super::ParseEvents,
    ) -> (crate::SyntaxTree, Vec<Option<crate::SyntaxDiagnosticOwner>>) {
        build_parser_syntax_tree(
            source,
            parse.events,
            parse.tokens,
            parse.trivia,
            &parse.diagnostic_owners,
            &TestFactory,
        )
        .expect("atomic recovery owners must resolve")
    }

    #[test]
    fn malformed_completion_captures_causes_before_consuming_recovery_input() {
        for recovered_kind in [TestKind::Bogus, TestKind::Delimited] {
            let mut parser = Parser::<TestLanguage>::new("x");
            let root = parser.start();
            let recovered = parser.start();
            let expected = parser.pending_expected("expected construct");
            let specific =
                parser.pending_error(DiagnosticCodeId::new("test/specific"), "invalid construct");
            parser.bump();
            parser.complete_recovery(recovered, recovered_kind, [expected, specific]);
            parser.complete(root, TestKind::Root);

            let parse = parser.finish();
            assert_eq!(parse.diagnostics.len(), 2);
            assert!(parse.diagnostics.iter().all(|diagnostic| {
                diagnostic.stage == DiagnosticStage::Parser
                    && diagnostic.range == Some(TextRange::new(TextSize::new(0), TextSize::new(1)))
            }));
            let (tree, owners) = build("x", parse);
            let root = SyntaxNode::<TestLanguage>::new_root("x", &tree);
            let recovered = root.children().next().expect("recovered child");
            assert!(recovered.is_directly_malformed());
            assert_eq!(owners[0].expect("first owner").node(), recovered.id());
            assert_eq!(owners[1].expect("second owner").node(), recovered.id());
        }
    }

    #[test]
    fn required_empty_slot_and_ownerless_diagnostic_remain_distinct() {
        let mut parser = Parser::<TestLanguage>::new("");
        let root = parser.start();
        let structural = parser.pending_expected("expected required field");
        parser.missing_required_slot(root.anchor(), 0, [structural]);
        let advisory = parser.pending_error(DiagnosticCodeId::new("test/advisory"), "advisory");
        parser.report_non_structural(advisory);
        parser.complete(root, TestKind::Missing);

        let parse = parser.finish();
        let (tree, owners) = build("", parse);
        let root = SyntaxNode::<TestLanguage>::new_root("", &tree);
        assert!(matches!(root.slot_at(0), Some(SyntaxSlot::Empty)));
        let structural = owners[0].expect("missing slot owner");
        assert_eq!(structural.node(), root.id());
        assert_eq!(structural.slot(), Some(0));
        assert_eq!(owners[1], None);
    }

    #[test]
    fn atomic_recovery_marks_a_valid_shape_and_its_ancestor_recovered() {
        let mut parser = Parser::<TestLanguage>::new("");
        let root = parser.start();
        let recovered = parser.start();
        let diagnostic = parser.pending_expected("invalid valid-shaped construct");
        parser.complete_recovery(recovered, TestKind::RecoveredValid, [diagnostic]);
        parser.complete(root, TestKind::Root);

        let parse = parser.finish();
        assert_eq!(
            parse.events,
            [
                crate::Event::Start {
                    kind: RawSyntaxKind::new(TestKind::Root as u16),
                    forward_parent: 0,
                },
                crate::Event::Start {
                    kind: RawSyntaxKind::new(TestKind::RecoveredValid as u16),
                    forward_parent: 0,
                },
                crate::Event::Finish,
                crate::Event::Finish,
            ]
        );
        let (tree, owners) = build("", parse);
        let root = SyntaxNode::<TestLanguage>::new_root("", &tree);
        let recovered = root.children().next().expect("recovered child");
        assert!(recovered.is_directly_malformed());
        assert!(!root.is_recovery_free());
        assert_eq!(owners[0].expect("node owner").node(), recovered.id());
    }

    #[test]
    fn unresolved_owner_storage_does_not_grow_for_atomic_recovery() {
        assert_eq!(
            std::mem::size_of::<super::UnresolvedDiagnosticOwner>(),
            std::mem::size_of::<(crate::NodeAnchor, Option<u16>)>()
        );
        assert_eq!(
            std::mem::size_of::<super::ParserDiagnostic>(),
            std::mem::size_of::<(
                jolt_diagnostics::Diagnostic,
                Option<super::UnresolvedDiagnosticOwner>,
            )>()
        );
        assert_eq!(
            std::mem::size_of::<super::PendingDiagnostic>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn pending_capture_preserves_nested_diagnostic_order() {
        let mut parser = Parser::<TestLanguage>::new("x");
        let outer = parser.start();
        let outer_cause = parser.pending_expected("outer");
        let inner = parser.start();
        let inner_cause = parser.pending_unexpected("inner");
        parser.complete_recovery(inner, TestKind::RecoveredValid, [inner_cause]);
        parser.complete_recovery(outer, TestKind::RecoveredValid, [outer_cause]);

        let parse = parser.finish();
        assert_eq!(
            parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            ["outer", "inner"]
        );
        assert!(parse.diagnostics.iter().all(|diagnostic| {
            diagnostic.range == Some(TextRange::new(TextSize::new(0), TextSize::new(1)))
        }));
    }

    #[test]
    fn lookahead_capture_preserves_cursor_events_and_atomic_ownership() {
        let mut parser = Parser::<TestLanguage>::new("x");
        let recovered = parser.start();
        let diagnostic = parser.pending_expected_at(1, "lookahead");
        assert_eq!(parser.position(), 0);
        parser.complete_recovery(recovered, TestKind::RecoveredValid, [diagnostic]);

        let parse = parser.finish();
        assert_eq!(
            parse.events,
            [
                crate::Event::Start {
                    kind: RawSyntaxKind::new(TestKind::RecoveredValid as u16),
                    forward_parent: 0,
                },
                crate::Event::Finish,
            ]
        );
        assert_eq!(
            parse.diagnostics[0].range,
            Some(TextRange::empty(TextSize::new(1)))
        );
        let (tree, owners) = build("x", parse);
        let root = SyntaxNode::<TestLanguage>::new_root("x", &tree);
        assert!(root.is_directly_malformed());
        assert_eq!(owners[0].expect("atomic owner").node(), root.id());
    }

    #[test]
    fn pending_diagnostic_cannot_be_consumed_twice() {
        let mut parser = Parser::<TestLanguage>::new("");
        let diagnostic = parser.pending_expected("once");
        let duplicate = super::PendingDiagnostic {
            index: diagnostic.index,
        };
        parser.report_non_structural(diagnostic);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parser.report_non_structural(duplicate);
        }));

        assert!(panic.is_err());
        assert_eq!(parser.finish().diagnostics.len(), 1);
    }

    #[test]
    fn finish_rejects_an_unconsumed_pending_diagnostic() {
        let mut parser = Parser::<TestLanguage>::new("");
        let _pending = parser.pending_expected("unfinished");

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parser.finish()));
        assert!(panic.is_err());
    }

    #[test]
    fn malformed_completion_rejects_no_cause_before_mutating_events() {
        let mut parser = Parser::<TestLanguage>::new("");
        let marker = parser.start();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parser.complete_recovery(marker, TestKind::Bogus, []);
        }));

        assert!(panic.is_err());
        assert!(matches!(
            parser.finish().events.as_slice(),
            [crate::Event::Tombstone]
        ));
    }
}
