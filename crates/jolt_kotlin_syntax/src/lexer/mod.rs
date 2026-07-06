mod token;

use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticStage, Severity};
use jolt_syntax::{SyntaxTrivia, TriviaKind as SyntaxTriviaKind};
use jolt_text::{TextRange, TextSize};

use crate::KotlinSyntaxKind;

pub use token::{KotlinLexDiagnosticCode, LexedToken, LexerDiagnostic};

pub struct KotlinLexer<'source> {
    scanner: Scanner<'source>,
    emitted_eof: bool,
}

impl<'source> KotlinLexer<'source> {
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        Self {
            scanner: Scanner::new(source),
            emitted_eof: false,
        }
    }

    /// Returns the next token, appending its trivia to the supplied buffer.
    pub fn next_token_into(&mut self, trivia: &mut Vec<SyntaxTrivia>) -> LexedToken {
        if self.emitted_eof {
            return self.eof_token_into(trivia.len()..trivia.len());
        }

        let leading = self.scanner.leading_trivia_into(trivia);
        if self.scanner.at_end() {
            self.emitted_eof = true;
            return self.eof_token_into(leading);
        }

        let (kind, range) = self.scanner.token();
        let trailing = self.scanner.trailing_trivia_into(trivia);
        LexedToken {
            kind,
            range,
            leading,
            trailing,
        }
    }

    /// Drains the remaining source and returns all lexer diagnostics.
    #[must_use]
    pub fn finish(mut self) -> Vec<LexerDiagnostic> {
        self.scanner.drain();
        self.scanner.finish_diagnostics();
        self.scanner.diagnostics
    }

    fn eof_token_into(&self, leading: Range<usize>) -> LexedToken {
        let end = TextSize::new(self.scanner.source.len());
        let trivia_end = leading.end;
        LexedToken {
            kind: KotlinSyntaxKind::Eof,
            range: TextRange::empty(end),
            leading,
            trailing: trivia_end..trivia_end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    StringPrefix {
        required_dollars: usize,
        start: TextSize,
    },
    LineString {
        required_dollars: usize,
        start: TextSize,
    },
    RawString {
        required_dollars: usize,
        start: TextSize,
    },
    ShortTemplateEntry,
    LongTemplateEntry {
        previous: StringMode,
        lbrace_count: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringMode {
    Line {
        required_dollars: usize,
        start: TextSize,
    },
    Raw {
        required_dollars: usize,
        start: TextSize,
    },
}

impl StringMode {
    const fn mode(self) -> Mode {
        match self {
            Self::Line {
                required_dollars,
                start,
            } => Mode::LineString {
                required_dollars,
                start,
            },
            Self::Raw {
                required_dollars,
                start,
            } => Mode::RawString {
                required_dollars,
                start,
            },
        }
    }
}

struct Scanner<'source> {
    source: &'source str,
    pos: usize,
    previous_end: TextSize,
    diagnostics: Vec<LexerDiagnostic>,
    modes: Vec<Mode>,
}

fn lexer_diagnostic(code: KotlinLexDiagnosticCode, range: TextRange) -> Diagnostic {
    Diagnostic {
        code: code.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Lexer,
        message: code.message().to_owned(),
        range: Some(range),
    }
}

impl<'source> Scanner<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            pos: 0,
            previous_end: TextSize::new(0),
            diagnostics: Vec::new(),
            modes: Vec::new(),
        }
    }

    fn token(&mut self) -> (KotlinSyntaxKind, TextRange) {
        let start = self.current_range().expect("token called at EOF").start();
        let kind = match self.modes.last().copied() {
            Some(Mode::StringPrefix {
                required_dollars,
                start,
            }) => self.quote_after_prefix(required_dollars, start),
            Some(Mode::LineString {
                required_dollars,
                start,
            }) => self.line_string_token(required_dollars, start),
            Some(Mode::RawString {
                required_dollars,
                start,
            }) => self.raw_string_token(required_dollars, start),
            Some(Mode::ShortTemplateEntry) => self.short_template_entry_token(),
            Some(Mode::LongTemplateEntry { .. }) => self.long_template_or_default_token(),
            None => self.default_token(),
        };
        let end = self.previous_end();
        (kind, TextRange::new(start, end))
    }

    fn default_token(&mut self) -> KotlinSyntaxKind {
        match self.current_char().expect("token called at EOF") {
            '\'' => self.character_literal(),
            '$' if self.string_prefix_dollars().is_some() => self.interpolation_prefix(),
            '"' => self.open_quote_without_prefix(),
            '$' if self.peek_identifier_start_after(1) => self.field_identifier(),
            '`' => self.backtick_identifier(),
            '.' if self.peek_char(1).is_some_and(|ch| ch.is_ascii_digit()) => self.number_literal(),
            ch if ch.is_ascii_digit() => self.number_literal(),
            'a' if self.peek_char(1) == Some('s') && self.peek_char(2) == Some('?') => {
                self.three(KotlinSyntaxKind::AsSafe)
            }
            ch if is_kotlin_identifier_start(ch) => self.identifier_or_keyword(),
            _ => self.operator_or_punctuation(),
        }
    }

    fn long_template_or_default_token(&mut self) -> KotlinSyntaxKind {
        match self.current_char().expect("token called at EOF") {
            '{' => {
                self.increment_long_template_brace_count();
                self.one(KotlinSyntaxKind::LBrace)
            }
            '}' => {
                if self.close_long_template_if_at_outer_brace() {
                    self.one(KotlinSyntaxKind::LongTemplateEntryEnd)
                } else {
                    self.one(KotlinSyntaxKind::RBrace)
                }
            }
            _ => self.default_token(),
        }
    }

    fn drain(&mut self) {
        while !self.at_end() {
            if self.can_scan_trivia() {
                while self.trivia_piece().is_some() {}
            }
            if !self.at_end() {
                self.token();
            }
        }
    }

    fn finish_diagnostics(&mut self) {
        let end = TextSize::new(self.source.len());
        let modes = std::mem::take(&mut self.modes);
        for mode in modes {
            match mode {
                Mode::LineString { start, .. } | Mode::StringPrefix { start, .. } => {
                    self.diagnostics.push(lexer_diagnostic(
                        KotlinLexDiagnosticCode::UnterminatedStringLiteral,
                        TextRange::new(start, end),
                    ));
                }
                Mode::RawString { start, .. } => {
                    self.diagnostics.push(lexer_diagnostic(
                        KotlinLexDiagnosticCode::UnterminatedRawStringLiteral,
                        TextRange::new(start, end),
                    ));
                }
                Mode::LongTemplateEntry { previous, .. } => {
                    let start = match previous {
                        StringMode::Line { start, .. } | StringMode::Raw { start, .. } => start,
                    };
                    let code = match previous {
                        StringMode::Line { .. } => {
                            KotlinLexDiagnosticCode::UnterminatedStringLiteral
                        }
                        StringMode::Raw { .. } => {
                            KotlinLexDiagnosticCode::UnterminatedRawStringLiteral
                        }
                    };
                    self.diagnostics
                        .push(lexer_diagnostic(code, TextRange::new(start, end)));
                }
                Mode::ShortTemplateEntry => {}
            }
        }
    }

    fn leading_trivia_into(&mut self, trivia: &mut Vec<SyntaxTrivia>) -> Range<usize> {
        let start = trivia.len();
        if self.can_scan_trivia() {
            while let Some(piece) = self.trivia_piece() {
                trivia.push(piece);
            }
        }
        start..trivia.len()
    }

    fn trailing_trivia_into(&mut self, trivia: &mut Vec<SyntaxTrivia>) -> Range<usize> {
        let start = trivia.len();
        if !self.can_scan_trivia() {
            return start..start;
        }

        while self.current_char().is_some_and(is_horizontal_whitespace) {
            trivia.push(self.horizontal_whitespace());
        }

        match (self.current_char(), self.peek_char(1)) {
            (Some('/'), Some('/')) => {
                trivia.push(self.line_comment());
                while self.current_char().is_some_and(is_horizontal_whitespace) {
                    trivia.push(self.horizontal_whitespace());
                }
            }
            (Some('/'), Some('*')) => {
                let end = find_nested_block_comment_end(self.source, self.pos);
                if end.is_some_and(|end| !contains_line_terminator(&self.source[self.pos..end])) {
                    trivia.push(self.block_comment());
                    while self.current_char().is_some_and(is_horizontal_whitespace) {
                        trivia.push(self.horizontal_whitespace());
                    }
                }
            }
            _ => {}
        }

        start..trivia.len()
    }

    fn can_scan_trivia(&self) -> bool {
        matches!(
            self.modes.last(),
            None | Some(Mode::LongTemplateEntry { .. })
        )
    }

    fn trivia_piece(&mut self) -> Option<SyntaxTrivia> {
        match (self.current_char(), self.peek_char(1)) {
            (Some('#'), Some('!')) if self.pos == 0 => Some(self.shebang_comment()),
            (Some(ch), _) if is_horizontal_whitespace(ch) => Some(self.horizontal_whitespace()),
            (Some('\r'), Some('\n')) => Some(self.newline(2)),
            (Some('\r' | '\n'), _) => Some(self.newline(1)),
            (Some('/'), Some('/')) => Some(self.line_comment()),
            (Some('/'), Some('*')) => Some(self.block_comment()),
            _ => None,
        }
    }

    fn shebang_comment(&mut self) -> SyntaxTrivia {
        self.line_comment_like(SyntaxTriviaKind::LineComment)
    }

    fn horizontal_whitespace(&mut self) -> SyntaxTrivia {
        let start = self
            .current_range()
            .expect("whitespace starts before EOF")
            .start();
        while self.current_char().is_some_and(is_horizontal_whitespace) {
            self.bump();
        }
        SyntaxTrivia::new(
            SyntaxTriviaKind::Whitespace,
            TextRange::new(start, self.previous_end()).len(),
        )
    }

    fn newline(&mut self, len: usize) -> SyntaxTrivia {
        let start = self
            .current_range()
            .expect("newline starts before EOF")
            .start();
        for _ in 0..len {
            self.bump();
        }
        SyntaxTrivia::new(
            SyntaxTriviaKind::Newline,
            TextRange::new(start, self.previous_end()).len(),
        )
    }

    fn line_comment(&mut self) -> SyntaxTrivia {
        self.line_comment_like(SyntaxTriviaKind::LineComment)
    }

    fn line_comment_like(&mut self, kind: SyntaxTriviaKind) -> SyntaxTrivia {
        let start = self
            .current_range()
            .expect("comment starts before EOF")
            .start();
        self.bump();
        self.bump();
        while self
            .current_char()
            .is_some_and(|ch| ch != '\r' && ch != '\n')
        {
            self.bump();
        }
        SyntaxTrivia::new(kind, TextRange::new(start, self.previous_end()).len())
    }

    fn block_comment(&mut self) -> SyntaxTrivia {
        let start = self
            .current_range()
            .expect("comment starts before EOF")
            .start();
        let kind = if self.peek_char(2) == Some('*') && self.peek_char(3) != Some('/') {
            SyntaxTriviaKind::DocComment
        } else {
            SyntaxTriviaKind::BlockComment
        };

        self.bump();
        self.bump();
        let mut depth = 0usize;
        while !self.at_end() {
            match (self.current_char(), self.peek_char(1)) {
                (Some('/'), Some('*')) => {
                    depth += 1;
                    self.bump();
                    self.bump();
                }
                (Some('*'), Some('/')) if depth == 0 => {
                    self.bump();
                    self.bump();
                    return SyntaxTrivia::new(
                        kind,
                        TextRange::new(start, self.previous_end()).len(),
                    );
                }
                (Some('*'), Some('/')) => {
                    depth -= 1;
                    self.bump();
                    self.bump();
                }
                _ => self.bump(),
            }
        }

        self.diagnostics.push(lexer_diagnostic(
            KotlinLexDiagnosticCode::UnterminatedBlockComment,
            TextRange::new(start, self.previous_end_or_source_end()),
        ));
        SyntaxTrivia::new(kind, TextRange::new(start, self.previous_end()).len())
    }

    fn interpolation_prefix(&mut self) -> KotlinSyntaxKind {
        let dollars = self
            .string_prefix_dollars()
            .expect("interpolation prefix starts at current position");
        let start = self
            .current_range()
            .expect("interpolation prefix starts before EOF")
            .start();
        for _ in 0..dollars {
            self.bump();
        }
        self.modes.push(Mode::StringPrefix {
            required_dollars: dollars.max(1),
            start,
        });
        KotlinSyntaxKind::InterpolationPrefix
    }

    fn open_quote_without_prefix(&mut self) -> KotlinSyntaxKind {
        let start = self
            .current_range()
            .expect("quote starts before EOF")
            .start();
        if self.peek_char(1) == Some('"') && self.peek_char(2) == Some('"') {
            self.bump();
            self.bump();
            self.bump();
            self.modes.push(Mode::RawString {
                required_dollars: 1,
                start,
            });
        } else {
            self.bump();
            self.modes.push(Mode::LineString {
                required_dollars: 1,
                start,
            });
        }
        KotlinSyntaxKind::OpenQuote
    }

    fn quote_after_prefix(&mut self, required_dollars: usize, start: TextSize) -> KotlinSyntaxKind {
        if self.current_char() != Some('"') {
            self.modes.pop();
            return self.default_token();
        }

        self.modes.pop();
        if self.peek_char(1) == Some('"') && self.peek_char(2) == Some('"') {
            self.bump();
            self.bump();
            self.bump();
            self.modes.push(Mode::RawString {
                required_dollars,
                start,
            });
        } else {
            self.bump();
            self.modes.push(Mode::LineString {
                required_dollars,
                start,
            });
        }
        KotlinSyntaxKind::OpenQuote
    }

    fn line_string_token(
        &mut self,
        required_dollars: usize,
        string_start: TextSize,
    ) -> KotlinSyntaxKind {
        match self.current_char().expect("string token called at EOF") {
            '"' => {
                self.bump();
                self.modes.pop();
                KotlinSyntaxKind::ClosingQuote
            }
            '\r' | '\n' => {
                self.diagnostics.push(lexer_diagnostic(
                    KotlinLexDiagnosticCode::UnterminatedStringLiteral,
                    TextRange::new(
                        string_start,
                        self.current_range().expect("newline exists").start(),
                    ),
                ));
                self.modes.pop();
                KotlinSyntaxKind::DanglingNewline
            }
            '\\' => self.escape_sequence(),
            '$' => self.template_entry_or_dollars(
                required_dollars,
                StringMode::Line {
                    required_dollars,
                    start: string_start,
                },
            ),
            _ => self.line_string_part(),
        }
    }

    fn raw_string_token(
        &mut self,
        required_dollars: usize,
        string_start: TextSize,
    ) -> KotlinSyntaxKind {
        match self.current_char().expect("raw string token called at EOF") {
            '"' if self.peek_char(1) == Some('"') && self.peek_char(2) == Some('"') => {
                let quotes = self.count_current_char('"');
                if quotes == 3 {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.modes.pop();
                    KotlinSyntaxKind::ClosingQuote
                } else {
                    for _ in 0..quotes.saturating_sub(3) {
                        self.bump();
                    }
                    KotlinSyntaxKind::RegularStringPart
                }
            }
            '$' => self.template_entry_or_dollars(
                required_dollars,
                StringMode::Raw {
                    required_dollars,
                    start: string_start,
                },
            ),
            _ => self.raw_string_part(),
        }
    }

    fn template_entry_or_dollars(
        &mut self,
        required_dollars: usize,
        previous: StringMode,
    ) -> KotlinSyntaxKind {
        let dollars = self.count_current_char('$');
        let after_dollars = self.nth_char(dollars);
        match after_dollars {
            Some('{') if dollars == required_dollars => {
                for _ in 0..=dollars {
                    self.bump();
                }
                self.modes.pop();
                self.modes.push(Mode::LongTemplateEntry {
                    previous,
                    lbrace_count: 0,
                });
                KotlinSyntaxKind::LongTemplateEntryStart
            }
            Some('{') if dollars > required_dollars => {
                for _ in 0..dollars - required_dollars {
                    self.bump();
                }
                KotlinSyntaxKind::RegularStringPart
            }
            Some('{') => {
                for _ in 0..dollars {
                    self.bump();
                }
                KotlinSyntaxKind::RegularStringPart
            }
            Some(ch)
                if is_kotlin_identifier_start(ch)
                    || (ch == '`' && self.valid_escaped_identifier_at(dollars)) =>
            {
                match dollars.cmp(&required_dollars) {
                    std::cmp::Ordering::Equal => {
                        for _ in 0..dollars {
                            self.bump();
                        }
                        self.modes.push(Mode::ShortTemplateEntry);
                        KotlinSyntaxKind::ShortTemplateEntryStart
                    }
                    std::cmp::Ordering::Greater => {
                        for _ in 0..dollars - required_dollars {
                            self.bump();
                        }
                        KotlinSyntaxKind::RegularStringPart
                    }
                    std::cmp::Ordering::Less => {
                        for _ in 0..dollars {
                            self.bump();
                        }
                        KotlinSyntaxKind::RegularStringPart
                    }
                }
            }
            _ => {
                for _ in 0..dollars {
                    self.bump();
                }
                KotlinSyntaxKind::RegularStringPart
            }
        }
    }

    fn short_template_entry_token(&mut self) -> KotlinSyntaxKind {
        let kind = match self
            .current_char()
            .expect("short template token before EOF")
        {
            '`' => self.backtick_identifier(),
            ch if is_kotlin_identifier_start(ch) => self.short_template_identifier(),
            _ => self.operator_or_punctuation(),
        };
        self.modes.pop();
        kind
    }

    fn short_template_identifier(&mut self) -> KotlinSyntaxKind {
        let start = self.pos;
        self.bump();
        while self.current_char().is_some_and(is_kotlin_identifier_part) {
            self.bump();
        }

        if &self.source[start..self.pos] == "this" {
            KotlinSyntaxKind::ThisKw
        } else {
            KotlinSyntaxKind::Identifier
        }
    }

    fn escape_sequence(&mut self) -> KotlinSyntaxKind {
        let start = self
            .current_range()
            .expect("escape starts before EOF")
            .start();
        self.bump();
        if self.at_end() {
            self.diagnostics.push(lexer_diagnostic(
                KotlinLexDiagnosticCode::InvalidEscapeSequence,
                TextRange::empty(start),
            ));
            return KotlinSyntaxKind::EscapeSequence;
        }

        if self.current_char().is_some_and(is_line_terminator_start) {
            let range = self.current_range().expect("line terminator exists");
            self.diagnostics.push(lexer_diagnostic(
                KotlinLexDiagnosticCode::InvalidEscapeSequence,
                range,
            ));
            return KotlinSyntaxKind::EscapeSequence;
        }

        let valid_unicode_escape = self.current_char() == Some('u')
            && (1..=4).all(|n| self.peek_char(n).is_some_and(|ch| ch.is_ascii_hexdigit()));

        if self.current_char() == Some('u') && !valid_unicode_escape {
            let end = self.current_range().expect("escape tail exists").end();
            self.diagnostics.push(lexer_diagnostic(
                KotlinLexDiagnosticCode::InvalidEscapeSequence,
                TextRange::new(start, end),
            ));
        }

        if valid_unicode_escape {
            for _ in 0..5 {
                if !self.at_end() {
                    self.bump();
                }
            }
        } else {
            self.bump();
        }
        KotlinSyntaxKind::EscapeSequence
    }

    fn line_string_part(&mut self) -> KotlinSyntaxKind {
        while let Some(ch) = self.current_char() {
            if matches!(ch, '\\' | '"' | '$' | '\r' | '\n') {
                break;
            }
            self.bump();
        }
        KotlinSyntaxKind::RegularStringPart
    }

    fn raw_string_part(&mut self) -> KotlinSyntaxKind {
        while let Some(ch) = self.current_char() {
            if ch == '$'
                || (ch == '"' && self.peek_char(1) == Some('"') && self.peek_char(2) == Some('"'))
            {
                break;
            }
            self.bump();
        }
        KotlinSyntaxKind::RegularStringPart
    }

    fn backtick_identifier(&mut self) -> KotlinSyntaxKind {
        let start = self
            .current_range()
            .expect("identifier starts before EOF")
            .start();
        self.bump();
        if self
            .current_char()
            .is_none_or(|ch| matches!(ch, '`' | '\r' | '\n'))
        {
            self.diagnostics.push(lexer_diagnostic(
                KotlinLexDiagnosticCode::UnterminatedBacktickIdentifier,
                TextRange::new(start, self.previous_end_or_source_end()),
            ));
            return KotlinSyntaxKind::Unknown;
        }

        while let Some(ch) = self.current_char() {
            match ch {
                '`' => {
                    self.bump();
                    return KotlinSyntaxKind::Identifier;
                }
                '\r' | '\n' => break,
                _ => self.bump(),
            }
        }
        self.diagnostics.push(lexer_diagnostic(
            KotlinLexDiagnosticCode::UnterminatedBacktickIdentifier,
            TextRange::new(start, self.previous_end_or_source_end()),
        ));
        KotlinSyntaxKind::Unknown
    }

    fn character_literal(&mut self) -> KotlinSyntaxKind {
        let start = self
            .current_range()
            .expect("literal starts before EOF")
            .start();
        self.bump();
        while let Some(ch) = self.current_char() {
            match ch {
                '\'' => {
                    self.bump();
                    return KotlinSyntaxKind::CharacterLiteral;
                }
                '\r' | '\n' => break,
                '\\' => {
                    self.bump();
                    if !self.at_end() && !self.current_char().is_some_and(is_line_terminator_start)
                    {
                        self.bump();
                    }
                }
                _ => self.bump(),
            }
        }

        self.diagnostics.push(lexer_diagnostic(
            KotlinLexDiagnosticCode::UnterminatedCharacterLiteral,
            TextRange::new(start, self.previous_end_or_source_end()),
        ));
        KotlinSyntaxKind::CharacterLiteral
    }

    fn number_literal(&mut self) -> KotlinSyntaxKind {
        if self.current_char() == Some('.') {
            self.bump();
            self.consume_digits_or_underscores(10);
            self.consume_exponent();
            self.consume_float_suffix();
            return KotlinSyntaxKind::FloatLiteral;
        }

        if self.current_char() == Some('0') && matches!(self.peek_char(1), Some('x' | 'X')) {
            self.bump();
            self.bump();
            self.consume_digits_or_underscores(16);
            self.consume_integer_suffix();
            return KotlinSyntaxKind::IntegerLiteral;
        }

        if self.current_char() == Some('0') && matches!(self.peek_char(1), Some('b' | 'B')) {
            self.bump();
            self.bump();
            self.consume_digits_or_underscores(10);
            self.consume_integer_suffix();
            return KotlinSyntaxKind::IntegerLiteral;
        }

        self.consume_digits_or_underscores(10);

        if matches!(
            (self.current_char(), self.peek_char(1)),
            (Some('.'), Some('.' | '<'))
        ) {
            self.consume_integer_suffix();
            return KotlinSyntaxKind::IntegerLiteral;
        }

        let mut floating = false;
        if self.current_char() == Some('.')
            && self.peek_char(1).is_some_and(|ch| ch.is_ascii_digit())
        {
            floating = true;
            self.bump();
            self.consume_digits_or_underscores(10);
        }
        if matches!(self.current_char(), Some('e' | 'E')) {
            floating = true;
            self.consume_exponent();
        }
        if self
            .current_char()
            .is_some_and(|ch| matches!(ch, 'f' | 'F'))
        {
            floating = true;
            self.bump();
        } else if !floating {
            self.consume_integer_suffix();
        }

        if floating {
            KotlinSyntaxKind::FloatLiteral
        } else {
            KotlinSyntaxKind::IntegerLiteral
        }
    }

    fn field_identifier(&mut self) -> KotlinSyntaxKind {
        self.bump();
        if self.current_char() == Some('`') {
            self.backtick_identifier();
        } else {
            self.bump();
            while self.current_char().is_some_and(is_kotlin_identifier_part) {
                self.bump();
            }
        }
        KotlinSyntaxKind::FieldIdentifier
    }

    fn identifier_or_keyword(&mut self) -> KotlinSyntaxKind {
        let start = self.pos;
        self.bump();
        while self.current_char().is_some_and(is_kotlin_identifier_part) {
            self.bump();
        }

        match &self.source[start..self.pos] {
            "package" => KotlinSyntaxKind::PackageKw,
            "as" => KotlinSyntaxKind::AsKw,
            "typealias" => KotlinSyntaxKind::TypeAliasKw,
            "class" => KotlinSyntaxKind::ClassKw,
            "this" => KotlinSyntaxKind::ThisKw,
            "super" => KotlinSyntaxKind::SuperKw,
            "val" => KotlinSyntaxKind::ValKw,
            "var" => KotlinSyntaxKind::VarKw,
            "fun" => KotlinSyntaxKind::FunKw,
            "for" => KotlinSyntaxKind::ForKw,
            "null" => KotlinSyntaxKind::NullKw,
            "true" => KotlinSyntaxKind::TrueKw,
            "false" => KotlinSyntaxKind::FalseKw,
            "is" => KotlinSyntaxKind::IsKw,
            "in" => KotlinSyntaxKind::InKw,
            "throw" => KotlinSyntaxKind::ThrowKw,
            "return" => KotlinSyntaxKind::ReturnKw,
            "break" => KotlinSyntaxKind::BreakKw,
            "continue" => KotlinSyntaxKind::ContinueKw,
            "object" => KotlinSyntaxKind::ObjectKw,
            "if" => KotlinSyntaxKind::IfKw,
            "try" => KotlinSyntaxKind::TryKw,
            "else" => KotlinSyntaxKind::ElseKw,
            "while" => KotlinSyntaxKind::WhileKw,
            "do" => KotlinSyntaxKind::DoKw,
            "when" => KotlinSyntaxKind::WhenKw,
            "interface" => KotlinSyntaxKind::InterfaceKw,
            "typeof" => KotlinSyntaxKind::TypeOfKw,
            "all" => KotlinSyntaxKind::AllKw,
            "file" => KotlinSyntaxKind::FileKw,
            "field" => KotlinSyntaxKind::FieldKw,
            "property" => KotlinSyntaxKind::PropertyKw,
            "receiver" => KotlinSyntaxKind::ReceiverKw,
            "param" => KotlinSyntaxKind::ParamKw,
            "setparam" => KotlinSyntaxKind::SetParamKw,
            "delegate" => KotlinSyntaxKind::DelegateKw,
            "import" => KotlinSyntaxKind::ImportKw,
            "where" => KotlinSyntaxKind::WhereKw,
            "by" => KotlinSyntaxKind::ByKw,
            "get" => KotlinSyntaxKind::GetKw,
            "set" => KotlinSyntaxKind::SetKw,
            "constructor" => KotlinSyntaxKind::ConstructorKw,
            "init" => KotlinSyntaxKind::InitKw,
            "context" => KotlinSyntaxKind::ContextKw,
            "catch" => KotlinSyntaxKind::CatchKw,
            "dynamic" => KotlinSyntaxKind::DynamicKw,
            "finally" => KotlinSyntaxKind::FinallyKw,
            "abstract" => KotlinSyntaxKind::AbstractKw,
            "enum" => KotlinSyntaxKind::EnumKw,
            "contract" => KotlinSyntaxKind::ContractKw,
            "open" => KotlinSyntaxKind::OpenKw,
            "inner" => KotlinSyntaxKind::InnerKw,
            "override" => KotlinSyntaxKind::OverrideKw,
            "private" => KotlinSyntaxKind::PrivateKw,
            "public" => KotlinSyntaxKind::PublicKw,
            "internal" => KotlinSyntaxKind::InternalKw,
            "protected" => KotlinSyntaxKind::ProtectedKw,
            "out" => KotlinSyntaxKind::OutKw,
            "vararg" => KotlinSyntaxKind::VarargKw,
            "reified" => KotlinSyntaxKind::ReifiedKw,
            "companion" => KotlinSyntaxKind::CompanionKw,
            "sealed" => KotlinSyntaxKind::SealedKw,
            "final" => KotlinSyntaxKind::FinalKw,
            "lateinit" => KotlinSyntaxKind::LateinitKw,
            "data" => KotlinSyntaxKind::DataKw,
            "value" => KotlinSyntaxKind::ValueKw,
            "inline" => KotlinSyntaxKind::InlineKw,
            "noinline" => KotlinSyntaxKind::NoinlineKw,
            "tailrec" => KotlinSyntaxKind::TailrecKw,
            "external" => KotlinSyntaxKind::ExternalKw,
            "annotation" => KotlinSyntaxKind::AnnotationKw,
            "crossinline" => KotlinSyntaxKind::CrossinlineKw,
            "operator" => KotlinSyntaxKind::OperatorKw,
            "infix" => KotlinSyntaxKind::InfixKw,
            "const" => KotlinSyntaxKind::ConstKw,
            "suspend" => KotlinSyntaxKind::SuspendKw,
            "expect" => KotlinSyntaxKind::ExpectKw,
            "actual" => KotlinSyntaxKind::ActualKw,
            _ => KotlinSyntaxKind::Identifier,
        }
    }

    fn operator_or_punctuation(&mut self) -> KotlinSyntaxKind {
        let start = self
            .current_range()
            .expect("operator starts before EOF")
            .start();
        match self.current_char().expect("operator starts before EOF") {
            '.' if self.peek_char(1) == Some('.') && self.peek_char(2) == Some('.') => {
                self.three(KotlinSyntaxKind::Reserved)
            }
            '=' if self.peek_char(1) == Some('=') && self.peek_char(2) == Some('=') => {
                self.three(KotlinSyntaxKind::EqEqEq)
            }
            '!' if self.peek_char(1) == Some('=') && self.peek_char(2) == Some('=') => {
                self.three(KotlinSyntaxKind::BangEqEqEq)
            }
            '!' if self.peek_char(1) == Some('i')
                && self.peek_char(2) == Some('n')
                && !self.peek_char(3).is_some_and(is_kotlin_identifier_part) =>
            {
                self.three(KotlinSyntaxKind::NotIn)
            }
            '!' if self.peek_char(1) == Some('i')
                && self.peek_char(2) == Some('s')
                && !self.peek_char(3).is_some_and(is_kotlin_identifier_part) =>
            {
                self.three(KotlinSyntaxKind::NotIs)
            }
            '+' if self.peek_char(1) == Some('+') => self.two(KotlinSyntaxKind::PlusPlus),
            '-' if self.peek_char(1) == Some('-') => self.two(KotlinSyntaxKind::MinusMinus),
            '<' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::LtEq),
            '>' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::GtEq),
            '=' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::EqEq),
            '!' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::BangEq),
            '!' if self.peek_char(1) == Some('!') => self.two(KotlinSyntaxKind::BangBang),
            '&' if self.peek_char(1) == Some('&') => self.two(KotlinSyntaxKind::AndAnd),
            '|' if self.peek_char(1) == Some('|') => self.two(KotlinSyntaxKind::OrOr),
            '?' if self.peek_char(1) == Some('.') => self.two(KotlinSyntaxKind::SafeAccess),
            '?' if self.peek_char(1) == Some(':') => self.two(KotlinSyntaxKind::Elvis),
            '*' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::StarEq),
            '/' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::SlashEq),
            '%' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::PercentEq),
            '+' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::PlusEq),
            '-' if self.peek_char(1) == Some('=') => self.two(KotlinSyntaxKind::MinusEq),
            '-' if self.peek_char(1) == Some('>') => self.two(KotlinSyntaxKind::Arrow),
            '=' if self.peek_char(1) == Some('>') => self.two(KotlinSyntaxKind::DoubleArrow),
            '.' if self.peek_char(1) == Some('.') && self.peek_char(2) == Some('<') => {
                self.three(KotlinSyntaxKind::RangeUntil)
            }
            '.' if self.peek_char(1) == Some('.') => self.two(KotlinSyntaxKind::Range),
            ':' if self.peek_char(1) == Some(':') => self.two(KotlinSyntaxKind::ColonColon),
            ';' if self.peek_char(1) == Some(';') => self.two(KotlinSyntaxKind::DoubleSemicolon),
            '[' => self.one(KotlinSyntaxKind::LBracket),
            ']' => self.one(KotlinSyntaxKind::RBracket),
            '{' => self.one(KotlinSyntaxKind::LBrace),
            '}' => self.one(KotlinSyntaxKind::RBrace),
            '(' => self.one(KotlinSyntaxKind::LParen),
            ')' => self.one(KotlinSyntaxKind::RParen),
            '.' => self.one(KotlinSyntaxKind::Dot),
            '*' => self.one(KotlinSyntaxKind::Star),
            '+' => self.one(KotlinSyntaxKind::Plus),
            '-' => self.one(KotlinSyntaxKind::Minus),
            '!' => self.one(KotlinSyntaxKind::Bang),
            '/' => self.one(KotlinSyntaxKind::Slash),
            '%' => self.one(KotlinSyntaxKind::Percent),
            '<' => self.one(KotlinSyntaxKind::Lt),
            '>' => self.one(KotlinSyntaxKind::Gt),
            '?' => self.one(KotlinSyntaxKind::Question),
            ':' => self.one(KotlinSyntaxKind::Colon),
            ';' => self.one(KotlinSyntaxKind::Semicolon),
            '=' => self.one(KotlinSyntaxKind::Assign),
            '&' => self.one(KotlinSyntaxKind::Amp),
            ',' => self.one(KotlinSyntaxKind::Comma),
            '#' => self.one(KotlinSyntaxKind::Hash),
            '@' => self.one(KotlinSyntaxKind::At),
            _ => {
                self.bump();
                self.diagnostics.push(lexer_diagnostic(
                    KotlinLexDiagnosticCode::UnknownCharacter,
                    TextRange::new(start, self.previous_end()),
                ));
                KotlinSyntaxKind::Unknown
            }
        }
    }

    fn consume_digits_or_underscores(&mut self, radix: u32) {
        while self
            .current_char()
            .is_some_and(|ch| ch == '_' || ch.is_digit(radix))
        {
            self.bump();
        }
    }

    fn consume_exponent(&mut self) {
        if !matches!(self.current_char(), Some('e' | 'E')) {
            return;
        }
        self.bump();
        if matches!(self.current_char(), Some('+' | '-')) {
            self.bump();
        }
        self.consume_digits_or_underscores(10);
    }

    fn consume_float_suffix(&mut self) {
        if matches!(self.current_char(), Some('f' | 'F')) {
            self.bump();
        }
    }

    fn consume_integer_suffix(&mut self) {
        if matches!(self.current_char(), Some('u' | 'U')) {
            self.bump();
        }
        if matches!(self.current_char(), Some('l' | 'L')) {
            self.bump();
        }
    }

    fn string_prefix_dollars(&self) -> Option<usize> {
        let mut count = 0usize;
        while self.nth_char(count) == Some('$') {
            count += 1;
        }
        if count > 0 && self.nth_char(count) == Some('"') {
            Some(count)
        } else {
            None
        }
    }

    fn peek_identifier_start_after(&self, chars: usize) -> bool {
        match self.nth_char(chars) {
            Some(ch) if is_kotlin_identifier_start(ch) => true,
            Some('`') => self.valid_escaped_identifier_at(chars),
            _ => false,
        }
    }

    fn valid_escaped_identifier_at(&self, chars: usize) -> bool {
        if self.nth_char(chars) != Some('`') {
            return false;
        }

        let mut saw_content = false;
        let offset = self.byte_offset_after_chars(chars);
        for ch in self.source[offset..].chars().skip(1) {
            match ch {
                '`' => return saw_content,
                '\r' | '\n' => return false,
                _ => saw_content = true,
            }
        }
        false
    }

    fn byte_offset_after_chars(&self, chars: usize) -> usize {
        let mut offset = self.pos;
        for ch in self.source[self.pos..].chars().take(chars) {
            offset += ch.len_utf8();
        }
        offset
    }

    fn increment_long_template_brace_count(&mut self) {
        if let Some(Mode::LongTemplateEntry { lbrace_count, .. }) = self.modes.last_mut() {
            *lbrace_count += 1;
        }
    }

    fn close_long_template_if_at_outer_brace(&mut self) -> bool {
        let Some(Mode::LongTemplateEntry {
            previous,
            lbrace_count,
        }) = self.modes.last_mut()
        else {
            return false;
        };

        if *lbrace_count == 0 {
            let previous = *previous;
            self.modes.pop();
            self.modes.push(previous.mode());
            true
        } else {
            *lbrace_count -= 1;
            false
        }
    }

    fn one(&mut self, kind: KotlinSyntaxKind) -> KotlinSyntaxKind {
        self.bump();
        kind
    }

    fn two(&mut self, kind: KotlinSyntaxKind) -> KotlinSyntaxKind {
        self.bump();
        self.bump();
        kind
    }

    fn three(&mut self, kind: KotlinSyntaxKind) -> KotlinSyntaxKind {
        self.bump();
        self.bump();
        self.bump();
        kind
    }

    fn bump(&mut self) {
        let ch = self.current_char().expect("cannot bump EOF");
        self.pos += ch.len_utf8();
        self.previous_end = TextSize::new(self.pos);
    }

    fn current_char(&self) -> Option<char> {
        self.source.get(self.pos..)?.chars().next()
    }

    fn peek_char(&self, n: usize) -> Option<char> {
        self.nth_char(n)
    }

    fn nth_char(&self, n: usize) -> Option<char> {
        self.source.get(self.pos..)?.chars().nth(n)
    }

    fn count_current_char(&self, expected: char) -> usize {
        self.source
            .get(self.pos..)
            .unwrap_or_default()
            .chars()
            .take_while(|ch| *ch == expected)
            .count()
    }

    fn current_range(&self) -> Option<TextRange> {
        let start = TextSize::new(self.pos);
        let end = start + TextSize::new(self.current_char()?.len_utf8());
        Some(TextRange::new(start, end))
    }

    const fn previous_end(&self) -> TextSize {
        self.previous_end
    }

    fn previous_end_or_source_end(&self) -> TextSize {
        if self.pos == self.source.len() {
            TextSize::new(self.source.len())
        } else {
            self.previous_end
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }
}

fn is_horizontal_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000C}')
}

fn is_line_terminator_start(ch: char) -> bool {
    matches!(ch, '\r' | '\n')
}

fn contains_line_terminator(text: &str) -> bool {
    text.chars().any(is_line_terminator_start)
}

fn find_nested_block_comment_end(source: &str, start: usize) -> Option<usize> {
    let mut offset = start.checked_add(2)?;
    let mut depth = 0usize;
    while offset < source.len() {
        let rest = &source[offset..];
        if rest.starts_with("/*") {
            depth += 1;
            offset += 2;
        } else if rest.starts_with("*/") {
            offset += 2;
            if depth == 0 {
                return Some(offset);
            }
            depth -= 1;
        } else {
            offset += rest.chars().next()?.len_utf8();
        }
    }
    None
}

fn is_kotlin_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_kotlin_identifier_part(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}
