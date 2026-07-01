#[cfg(test)]
mod tests;
mod token;
mod unicode;

use std::collections::VecDeque;

use crate::JavaSyntaxKind;
use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};
use jolt_text::{TextRange, TextSize};
use unicode_general_category::{GeneralCategory, get_general_category};

pub use token::{JavaLexDiagnosticCode, LexerDiagnostic, Token, Trivia, TriviaKind};
use unicode::{InputChar, translate_unicode_escapes};

/// A Java lexer that produces tokens on demand.
pub struct JavaLexer<'source> {
    scanner: Scanner<'source>,
    emitted_eof: bool,
}

impl<'source> JavaLexer<'source> {
    /// Creates a lexer for Java source text.
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        Self {
            scanner: Scanner::new(source),
            emitted_eof: false,
        }
    }

    /// Returns the next token, including trivia attached to that token.
    pub fn next_token(&mut self) -> Token {
        if self.emitted_eof {
            return self.eof_token(Vec::new());
        }

        let leading = self.scanner.leading_trivia();
        if self.scanner.at_end() {
            self.emitted_eof = true;
            return self.eof_token(leading);
        }

        let (kind, range) = self.scanner.token();
        let trailing = self.scanner.trailing_trivia();
        Token {
            kind,
            range,
            leading,
            trailing,
        }
    }

    /// Creates a checkpoint that can be restored with [`Self::rewind`].
    #[must_use]
    pub fn checkpoint(&self) -> JavaLexerCheckpoint {
        JavaLexerCheckpoint {
            pos: self.scanner.pos,
            diagnostics_len: self.scanner.diagnostics.len(),
            emitted_eof: self.emitted_eof,
        }
    }

    /// Restores the lexer to a previous checkpoint.
    pub fn rewind(&mut self, checkpoint: JavaLexerCheckpoint) {
        self.scanner.pos = checkpoint.pos;
        self.scanner
            .diagnostics
            .truncate(checkpoint.diagnostics_len);
        self.emitted_eof = checkpoint.emitted_eof;
    }

    /// Drains the remaining source and returns all lexer diagnostics.
    #[must_use]
    pub fn finish(mut self) -> Vec<LexerDiagnostic> {
        while !self.emitted_eof {
            self.next_token();
        }
        self.scanner.diagnostics
    }

    fn eof_token(&self, leading: Vec<Trivia>) -> Token {
        Token {
            kind: JavaSyntaxKind::Eof,
            range: TextRange::empty(TextSize::new(self.scanner.source.len())),
            leading,
            trailing: Vec::new(),
        }
    }
}

/// A checkpoint for [`JavaLexer`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaLexerCheckpoint {
    pos: usize,
    diagnostics_len: usize,
    emitted_eof: bool,
}

/// Parser-facing token source with lookahead and rewind support.
pub struct JavaTokenSource<'source> {
    lexer: JavaLexer<'source>,
    current: Token,
    lookahead: VecDeque<Token>,
}

impl<'source> JavaTokenSource<'source> {
    /// Creates a token source and positions it at the first token.
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        let mut lexer = JavaLexer::new(source);
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            lookahead: VecDeque::new(),
        }
    }

    /// Returns the current token.
    #[must_use]
    pub fn current(&self) -> &Token {
        &self.current
    }

    /// Advances to the next token.
    pub fn bump(&mut self) {
        self.current = self
            .lookahead
            .pop_front()
            .unwrap_or_else(|| self.lexer.next_token());
    }

    /// Returns the nth token from the current token, where zero is current.
    pub fn nth(&mut self, n: usize) -> &Token {
        if n == 0 {
            return &self.current;
        }

        while self.lookahead.len() < n {
            self.lookahead.push_back(self.lexer.next_token());
        }

        &self.lookahead[n - 1]
    }

    /// Creates a checkpoint that can be restored with [`Self::rewind`].
    #[must_use]
    pub fn checkpoint(&self) -> JavaTokenSourceCheckpoint {
        JavaTokenSourceCheckpoint {
            lexer: self.lexer.checkpoint(),
            current: self.current.clone(),
            lookahead: self.lookahead.clone(),
        }
    }

    /// Restores the token source to a previous checkpoint.
    pub fn rewind(&mut self, checkpoint: JavaTokenSourceCheckpoint) {
        self.lexer.rewind(checkpoint.lexer);
        self.current = checkpoint.current;
        self.lookahead = checkpoint.lookahead;
    }

    /// Drains the remaining source and returns all lexer diagnostics.
    #[must_use]
    pub fn finish(mut self) -> Vec<LexerDiagnostic> {
        while self.current.kind != JavaSyntaxKind::Eof {
            self.bump();
        }
        self.lexer.finish()
    }
}

/// A checkpoint for [`JavaTokenSource`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaTokenSourceCheckpoint {
    lexer: JavaLexerCheckpoint,
    current: Token,
    lookahead: VecDeque<Token>,
}

struct Scanner<'source> {
    source: &'source str,
    chars: Vec<InputChar>,
    pos: usize,
    diagnostics: Vec<LexerDiagnostic>,
}

fn lexer_diagnostic(code: JavaLexDiagnosticCode, range: TextRange) -> Diagnostic {
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
        let (chars, diagnostics) = translate_unicode_escapes(source);
        Self {
            source,
            chars,
            pos: 0,
            diagnostics,
        }
    }

    fn token(&mut self) -> (JavaSyntaxKind, TextRange) {
        let start = self.current().expect("token called at EOF").range.start();
        let kind = match self.current_char().expect("token called at EOF") {
            '\'' => self.character_literal(),
            '"' if self.peek_char(1) == Some('"') && self.peek_char(2) == Some('"') => {
                self.text_block_literal()
            }
            '"' => self.string_literal(),
            '.' if self.peek_char(1).is_some_and(|ch| ch.is_ascii_digit()) => self.number_literal(),
            ch if ch.is_ascii_digit() => self.number_literal(),
            ch if is_java_identifier_start(ch) => self.identifier_or_keyword(),
            _ => self.operator_or_punctuation(),
        };
        let end = self.previous_end();
        (kind, TextRange::new(start, end))
    }

    fn leading_trivia(&mut self) -> Vec<Trivia> {
        let mut trivia = Vec::new();
        while let Some(piece) = self.trivia_piece() {
            trivia.push(piece);
        }
        trivia
    }

    fn trailing_trivia(&mut self) -> Vec<Trivia> {
        let mut trivia = Vec::new();

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
                let end = find_block_comment_end(&self.chars, self.pos);
                if end.is_some_and(|end| !contains_line_terminator(&self.chars[self.pos..end])) {
                    trivia.push(self.block_comment());
                    while self.current_char().is_some_and(is_horizontal_whitespace) {
                        trivia.push(self.horizontal_whitespace());
                    }
                }
            }
            _ => {}
        }

        trivia
    }

    fn trivia_piece(&mut self) -> Option<Trivia> {
        match (self.current_char(), self.peek_char(1)) {
            (Some('\u{001A}'), _) if self.is_ignored_final_sub() => Some(self.ignored_final_sub()),
            (Some(ch), _) if is_horizontal_whitespace(ch) => Some(self.horizontal_whitespace()),
            (Some('\r'), Some('\n')) => Some(self.newline(2)),
            (Some('\r' | '\n'), _) => Some(self.newline(1)),
            (Some('/'), Some('/')) => Some(self.line_comment()),
            (Some('/'), Some('*')) => Some(self.block_comment()),
            _ => None,
        }
    }

    fn ignored_final_sub(&mut self) -> Trivia {
        let current = self.current().expect("SUB starts before EOF");
        self.bump();
        // JLS 3.5 ignores a final ASCII SUB/control-Z after Unicode escape
        // processing. Keep its raw range as trivia so formatting remains lossless.
        Trivia {
            kind: TriviaKind::Ignored,
            range: current.range,
        }
    }

    fn horizontal_whitespace(&mut self) -> Trivia {
        let start = self
            .current()
            .expect("whitespace starts before EOF")
            .range
            .start();
        while self.current_char().is_some_and(is_horizontal_whitespace) {
            self.bump();
        }
        Trivia {
            kind: TriviaKind::Whitespace,
            range: TextRange::new(start, self.previous_end()),
        }
    }

    fn newline(&mut self, len: usize) -> Trivia {
        let start = self
            .current()
            .expect("newline starts before EOF")
            .range
            .start();
        for _ in 0..len {
            self.bump();
        }
        Trivia {
            kind: TriviaKind::Newline,
            range: TextRange::new(start, self.previous_end()),
        }
    }

    fn line_comment(&mut self) -> Trivia {
        let start = self
            .current()
            .expect("comment starts before EOF")
            .range
            .start();
        self.bump();
        self.bump();
        while self
            .current_char()
            .is_some_and(|ch| ch != '\r' && ch != '\n')
        {
            self.bump();
        }
        Trivia {
            kind: TriviaKind::LineComment,
            range: TextRange::new(start, self.previous_end()),
        }
    }

    fn block_comment(&mut self) -> Trivia {
        let start_pos = self.pos;
        let start = self
            .current()
            .expect("comment starts before EOF")
            .range
            .start();
        let kind = if self.peek_char(2) == Some('*') && self.peek_char(3) != Some('/') {
            TriviaKind::JavadocComment
        } else {
            TriviaKind::BlockComment
        };

        self.bump();
        self.bump();
        while !self.at_end() {
            if self.current_char() == Some('*') && self.peek_char(1) == Some('/') {
                self.bump();
                self.bump();
                return Trivia {
                    kind,
                    range: TextRange::new(start, self.previous_end()),
                };
            }
            self.bump();
        }

        self.diagnostics.push(lexer_diagnostic(
            JavaLexDiagnosticCode::UnterminatedBlockComment,
            TextRange::new(start, self.raw_end_for_pos(start_pos)),
        ));
        Trivia {
            kind,
            range: TextRange::new(start, self.previous_end_or_source_end()),
        }
    }

    fn character_literal(&mut self) -> JavaSyntaxKind {
        let start = self
            .current()
            .expect("literal starts before EOF")
            .range
            .start();
        self.bump();
        let mut content_chars = 0usize;
        let mut terminated = false;

        while let Some(ch) = self.current_char() {
            match ch {
                '\'' => {
                    self.bump();
                    terminated = true;
                    break;
                }
                '\r' | '\n' => break,
                '\\' => {
                    let escape_start = self
                        .current()
                        .expect("escape starts before EOF")
                        .range
                        .start();
                    self.bump();
                    if self.current_char().is_some_and(is_line_terminator_start) {
                        // Line-continuation escapes are valid only in text blocks;
                        // ordinary char literals still terminate at the line break.
                        let range = self.current().expect("line terminator exists").range;
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            range,
                        ));
                        break;
                    }
                    if self.at_end() {
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            TextRange::empty(escape_start),
                        ));
                        break;
                    }
                    if !is_valid_escape_tail(self.current_char().expect("escape tail exists")) {
                        let range = self.current().expect("escape tail exists").range;
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            range,
                        ));
                    }
                    self.bump_escape_tail();
                    content_chars += 1;
                }
                _ => {
                    content_chars += ch.len_utf16();
                    self.bump();
                }
            }
        }

        if !terminated {
            self.diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::UnterminatedCharacterLiteral,
                TextRange::new(start, self.previous_end_or_source_end()),
            ));
        } else if content_chars != 1 {
            self.diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::InvalidCharacterLiteral,
                TextRange::new(start, self.previous_end()),
            ));
        }

        JavaSyntaxKind::CharacterLiteral
    }

    fn string_literal(&mut self) -> JavaSyntaxKind {
        let start = self
            .current()
            .expect("literal starts before EOF")
            .range
            .start();
        self.bump();
        let mut terminated = false;

        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.bump();
                    terminated = true;
                    break;
                }
                '\r' | '\n' => break,
                '\\' => {
                    let escape_start = self
                        .current()
                        .expect("escape starts before EOF")
                        .range
                        .start();
                    self.bump();
                    if self.current_char().is_some_and(is_line_terminator_start) {
                        // Line-continuation escapes are valid only in text blocks;
                        // ordinary string literals still terminate at the line break.
                        let range = self.current().expect("line terminator exists").range;
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            range,
                        ));
                        break;
                    }
                    if self.at_end() {
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            TextRange::empty(escape_start),
                        ));
                        break;
                    }
                    if !is_valid_escape_tail(self.current_char().expect("escape tail exists")) {
                        let range = self.current().expect("escape tail exists").range;
                        self.diagnostics.push(lexer_diagnostic(
                            JavaLexDiagnosticCode::InvalidEscapeSequence,
                            range,
                        ));
                    }
                    self.bump_escape_tail();
                }
                _ => {
                    self.bump();
                }
            }
        }

        if !terminated {
            self.diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::UnterminatedStringLiteral,
                TextRange::new(start, self.previous_end_or_source_end()),
            ));
        }

        JavaSyntaxKind::StringLiteral
    }

    fn text_block_literal(&mut self) -> JavaSyntaxKind {
        let start = self
            .current()
            .expect("literal starts before EOF")
            .range
            .start();
        self.bump();
        self.bump();
        self.bump();

        // JLS 3.10.6 allows spaces, tabs, and form feeds between the opening
        // `"""` and the required line terminator.
        while self.current_char().is_some_and(is_horizontal_whitespace) {
            self.bump();
        }
        if !self.current_char().is_some_and(is_line_terminator_start) {
            let range = TextRange::new(start, self.previous_end());
            self.diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::MissingTextBlockLineTerminator,
                range,
            ));
        }

        let mut terminated = false;
        while !self.at_end() {
            if self.current_char() == Some('"')
                && self.peek_char(1) == Some('"')
                && self.peek_char(2) == Some('"')
            {
                self.bump();
                self.bump();
                self.bump();
                terminated = true;
                break;
            }

            if self.current_char() == Some('\\') {
                let escape_start = self
                    .current()
                    .expect("escape starts before EOF")
                    .range
                    .start();
                self.bump();
                if self.current_char().is_some_and(is_line_terminator_start) {
                    // JLS 3.10.7 line-continuation escapes are specific to text blocks.
                    self.bump_line_terminator();
                } else if !self.at_end() && self.current_char().is_some_and(is_valid_escape_tail) {
                    self.bump_escape_tail();
                } else if !self.at_end() {
                    let range = self.current().expect("escape tail exists").range;
                    self.diagnostics.push(lexer_diagnostic(
                        JavaLexDiagnosticCode::InvalidEscapeSequence,
                        range,
                    ));
                    self.bump();
                } else {
                    self.diagnostics.push(lexer_diagnostic(
                        JavaLexDiagnosticCode::InvalidEscapeSequence,
                        TextRange::empty(escape_start),
                    ));
                }
            } else {
                self.bump();
            }
        }

        if !terminated {
            self.diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::UnterminatedTextBlock,
                TextRange::new(start, self.previous_end_or_source_end()),
            ));
        }

        JavaSyntaxKind::TextBlockLiteral
    }

    fn number_literal(&mut self) -> JavaSyntaxKind {
        let start_pos = self.pos;
        let start = self
            .current()
            .expect("literal starts before EOF")
            .range
            .start();
        let kind = if self.current_char() == Some('.') {
            self.bump();
            self.consume_digits_for_radix(10);
            self.consume_decimal_exponent();
            self.consume_float_suffix();
            JavaSyntaxKind::FloatingPointLiteral
        } else if self.current_char() == Some('0') && matches!(self.peek_char(1), Some('x' | 'X')) {
            self.bump();
            self.bump();
            let before_digits = self.pos;
            self.consume_digits_for_radix(16);
            let has_whole_digits = self.chars[before_digits..self.pos]
                .iter()
                .any(|input| input.ch.is_ascii_hexdigit());
            let mut floating = false;
            let mut has_binary_exponent = false;
            let mut has_fraction_digits = false;
            if self.current_char() == Some('.') {
                floating = true;
                self.bump();
                let before_fraction = self.pos;
                self.consume_digits_for_radix(16);
                has_fraction_digits = self.chars[before_fraction..self.pos]
                    .iter()
                    .any(|input| input.ch.is_ascii_hexdigit());
            }
            if matches!(self.current_char(), Some('p' | 'P')) {
                floating = true;
                has_binary_exponent = true;
                self.consume_binary_exponent();
            }
            if floating {
                self.consume_float_suffix();
                if before_digits == self.pos {
                    self.invalid_numeric_literal(start);
                }
                if !has_binary_exponent {
                    self.invalid_numeric_literal(start);
                }
                if !has_whole_digits && !has_fraction_digits {
                    self.invalid_numeric_literal(start);
                }
                JavaSyntaxKind::FloatingPointLiteral
            } else {
                self.consume_integer_suffix();
                if before_digits == self.pos {
                    self.invalid_numeric_literal(start);
                }
                JavaSyntaxKind::IntegerLiteral
            }
        } else if self.current_char() == Some('0') && matches!(self.peek_char(1), Some('b' | 'B')) {
            self.bump();
            self.bump();
            let before_digits = self.pos;
            self.consume_digits_for_radix(2);
            let invalid_digit = self.current_char().is_some_and(|ch| ch.is_ascii_digit());
            if invalid_digit {
                self.consume_digits_for_radix(10);
            }
            self.consume_integer_suffix();
            if before_digits == self.pos || invalid_digit {
                self.invalid_numeric_literal(start);
            }
            JavaSyntaxKind::IntegerLiteral
        } else {
            self.consume_digits_for_radix(10);
            let mut floating = false;
            if self.current_char() == Some('.') {
                floating = true;
                self.bump();
                self.consume_digits_for_radix(10);
            }
            if matches!(self.current_char(), Some('e' | 'E')) {
                floating = true;
                self.consume_decimal_exponent();
            }
            if matches!(self.current_char(), Some('f' | 'F' | 'd' | 'D')) {
                floating = true;
                self.bump();
            } else if !floating {
                self.consume_integer_suffix();
            }

            if floating {
                JavaSyntaxKind::FloatingPointLiteral
            } else {
                JavaSyntaxKind::IntegerLiteral
            }
        };

        self.validate_numeric_literal(start_pos, start, kind);
        self.validate_octal_literal(start_pos, start, kind);
        kind
    }

    fn identifier_or_keyword(&mut self) -> JavaSyntaxKind {
        let start = self.pos;
        self.bump();
        while self.current_char().is_some_and(is_java_identifier_part) {
            self.bump();
        }

        let text = self.logical_text(start, self.pos);
        // JLS 3.9 contextual keywords are recognized only in parser context, so
        // the lexer keeps `record`, `var`, `yield`, `non-sealed`, etc. as identifiers.
        match text.as_str() {
            "true" | "false" => JavaSyntaxKind::BooleanLiteral,
            "null" => JavaSyntaxKind::NullLiteral,
            "abstract" => JavaSyntaxKind::AbstractKw,
            "assert" => JavaSyntaxKind::AssertKw,
            "boolean" => JavaSyntaxKind::BooleanKw,
            "break" => JavaSyntaxKind::BreakKw,
            "byte" => JavaSyntaxKind::ByteKw,
            "case" => JavaSyntaxKind::CaseKw,
            "catch" => JavaSyntaxKind::CatchKw,
            "char" => JavaSyntaxKind::CharKw,
            "class" => JavaSyntaxKind::ClassKw,
            "const" => JavaSyntaxKind::ConstKw,
            "continue" => JavaSyntaxKind::ContinueKw,
            "default" => JavaSyntaxKind::DefaultKw,
            "do" => JavaSyntaxKind::DoKw,
            "double" => JavaSyntaxKind::DoubleKw,
            "else" => JavaSyntaxKind::ElseKw,
            "enum" => JavaSyntaxKind::EnumKw,
            "extends" => JavaSyntaxKind::ExtendsKw,
            "final" => JavaSyntaxKind::FinalKw,
            "finally" => JavaSyntaxKind::FinallyKw,
            "float" => JavaSyntaxKind::FloatKw,
            "for" => JavaSyntaxKind::ForKw,
            "goto" => JavaSyntaxKind::GotoKw,
            "if" => JavaSyntaxKind::IfKw,
            "implements" => JavaSyntaxKind::ImplementsKw,
            "import" => JavaSyntaxKind::ImportKw,
            "instanceof" => JavaSyntaxKind::InstanceofKw,
            "int" => JavaSyntaxKind::IntKw,
            "interface" => JavaSyntaxKind::InterfaceKw,
            "long" => JavaSyntaxKind::LongKw,
            "native" => JavaSyntaxKind::NativeKw,
            "new" => JavaSyntaxKind::NewKw,
            "package" => JavaSyntaxKind::PackageKw,
            "private" => JavaSyntaxKind::PrivateKw,
            "protected" => JavaSyntaxKind::ProtectedKw,
            "public" => JavaSyntaxKind::PublicKw,
            "return" => JavaSyntaxKind::ReturnKw,
            "short" => JavaSyntaxKind::ShortKw,
            "static" => JavaSyntaxKind::StaticKw,
            "strictfp" => JavaSyntaxKind::StrictfpKw,
            "super" => JavaSyntaxKind::SuperKw,
            "switch" => JavaSyntaxKind::SwitchKw,
            "synchronized" => JavaSyntaxKind::SynchronizedKw,
            "this" => JavaSyntaxKind::ThisKw,
            "throw" => JavaSyntaxKind::ThrowKw,
            "throws" => JavaSyntaxKind::ThrowsKw,
            "transient" => JavaSyntaxKind::TransientKw,
            "try" => JavaSyntaxKind::TryKw,
            "void" => JavaSyntaxKind::VoidKw,
            "volatile" => JavaSyntaxKind::VolatileKw,
            "while" => JavaSyntaxKind::WhileKw,
            "_" => JavaSyntaxKind::UnderscoreKw,
            _ => JavaSyntaxKind::Identifier,
        }
    }

    fn operator_or_punctuation(&mut self) -> JavaSyntaxKind {
        let start = self
            .current()
            .expect("operator starts before EOF")
            .range
            .start();
        match self.current_char().expect("operator starts before EOF") {
            '(' => self.one(JavaSyntaxKind::LParen),
            ')' => self.one(JavaSyntaxKind::RParen),
            '{' => self.one(JavaSyntaxKind::LBrace),
            '}' => self.one(JavaSyntaxKind::RBrace),
            '[' => self.one(JavaSyntaxKind::LBracket),
            ']' => self.one(JavaSyntaxKind::RBracket),
            ';' => self.one(JavaSyntaxKind::Semicolon),
            ',' => self.one(JavaSyntaxKind::Comma),
            '@' => self.one(JavaSyntaxKind::At),
            '~' => self.one(JavaSyntaxKind::Tilde),
            '?' => self.one(JavaSyntaxKind::Question),
            ':' if self.peek_char(1) == Some(':') => self.two(JavaSyntaxKind::DoubleColon),
            ':' => self.one(JavaSyntaxKind::Colon),
            '.' if self.peek_char(1) == Some('.') && self.peek_char(2) == Some('.') => {
                self.three(JavaSyntaxKind::Ellipsis)
            }
            '.' => self.one(JavaSyntaxKind::Dot),
            '=' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::EqEq),
            '=' => self.one(JavaSyntaxKind::Assign),
            '!' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::BangEq),
            '!' => self.one(JavaSyntaxKind::Bang),
            '<' if self.peek_char(1) == Some('<') && self.peek_char(2) == Some('=') => {
                self.three(JavaSyntaxKind::LShiftEq)
            }
            '<' if self.peek_char(1) == Some('<') => self.two(JavaSyntaxKind::LShift),
            '<' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::LtEq),
            '<' => self.one(JavaSyntaxKind::Lt),
            // JLS 3.5 lets the parser reinterpret adjacent `>` tokens in type
            // contexts, but the lexical grammar still uses longest-match tokens here.
            '>' if self.peek_char(1) == Some('>')
                && self.peek_char(2) == Some('>')
                && self.peek_char(3) == Some('=') =>
            {
                self.bump();
                self.bump();
                self.bump();
                self.bump();
                JavaSyntaxKind::UnsignedRShiftEq
            }
            '>' if self.peek_char(1) == Some('>') && self.peek_char(2) == Some('>') => {
                self.three(JavaSyntaxKind::UnsignedRShift)
            }
            '>' if self.peek_char(1) == Some('>') && self.peek_char(2) == Some('=') => {
                self.three(JavaSyntaxKind::RShiftEq)
            }
            '>' if self.peek_char(1) == Some('>') => self.two(JavaSyntaxKind::RShift),
            '>' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::GtEq),
            '>' => self.one(JavaSyntaxKind::Gt),
            '&' if self.peek_char(1) == Some('&') => self.two(JavaSyntaxKind::AndAnd),
            '&' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::AmpEq),
            '&' => self.one(JavaSyntaxKind::Amp),
            '|' if self.peek_char(1) == Some('|') => self.two(JavaSyntaxKind::OrOr),
            '|' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::BarEq),
            '|' => self.one(JavaSyntaxKind::Bar),
            '+' if self.peek_char(1) == Some('+') => self.two(JavaSyntaxKind::PlusPlus),
            '+' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::PlusEq),
            '+' => self.one(JavaSyntaxKind::Plus),
            '-' if self.peek_char(1) == Some('>') => self.two(JavaSyntaxKind::Arrow),
            '-' if self.peek_char(1) == Some('-') => self.two(JavaSyntaxKind::MinusMinus),
            '-' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::MinusEq),
            '-' => self.one(JavaSyntaxKind::Minus),
            '*' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::StarEq),
            '*' => self.one(JavaSyntaxKind::Star),
            '/' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::SlashEq),
            '/' => self.one(JavaSyntaxKind::Slash),
            '^' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::CaretEq),
            '^' => self.one(JavaSyntaxKind::Caret),
            '%' if self.peek_char(1) == Some('=') => self.two(JavaSyntaxKind::PercentEq),
            '%' => self.one(JavaSyntaxKind::Percent),
            _ => {
                self.bump();
                self.diagnostics.push(lexer_diagnostic(
                    JavaLexDiagnosticCode::UnknownCharacter,
                    TextRange::new(start, self.previous_end()),
                ));
                JavaSyntaxKind::Unknown
            }
        }
    }

    fn consume_digits_for_radix(&mut self, radix: u32) {
        while self
            .current_char()
            .is_some_and(|ch| ch == '_' || ch.is_digit(radix))
        {
            self.bump();
        }
    }

    fn consume_decimal_exponent(&mut self) {
        if !matches!(self.current_char(), Some('e' | 'E')) {
            return;
        }
        self.bump();
        if matches!(self.current_char(), Some('+' | '-')) {
            self.bump();
        }
        self.consume_digits_for_radix(10);
    }

    fn consume_binary_exponent(&mut self) {
        if !matches!(self.current_char(), Some('p' | 'P')) {
            return;
        }
        self.bump();
        if matches!(self.current_char(), Some('+' | '-')) {
            self.bump();
        }
        self.consume_digits_for_radix(10);
    }

    fn consume_float_suffix(&mut self) {
        if matches!(self.current_char(), Some('f' | 'F' | 'd' | 'D')) {
            self.bump();
        }
    }

    fn consume_integer_suffix(&mut self) {
        if matches!(self.current_char(), Some('l' | 'L')) {
            self.bump();
        }
    }

    fn validate_numeric_literal(
        &mut self,
        start_pos: usize,
        start: TextSize,
        kind: JavaSyntaxKind,
    ) {
        let text = self.logical_text(start_pos, self.pos);
        if !underscores_are_between_digits(&text) || exponent_is_missing_digits(&text, kind) {
            self.invalid_numeric_literal(start);
        }
    }

    fn validate_octal_literal(&mut self, start_pos: usize, start: TextSize, kind: JavaSyntaxKind) {
        if kind != JavaSyntaxKind::IntegerLiteral
            || self
                .chars
                .get(start_pos)
                .is_none_or(|input| input.ch != '0')
            || matches!(
                self.chars.get(start_pos + 1).map(|input| input.ch),
                Some('x' | 'X' | 'b' | 'B')
            )
        {
            return;
        }

        let text = self.logical_text(start_pos, self.pos);
        let digits = text.trim_end_matches(['l', 'L']);
        if digits.chars().any(|ch| matches!(ch, '8' | '9')) {
            self.invalid_numeric_literal(start);
        }
    }

    fn invalid_numeric_literal(&mut self, start: TextSize) {
        self.diagnostics.push(lexer_diagnostic(
            JavaLexDiagnosticCode::InvalidNumericLiteral,
            TextRange::new(start, self.previous_end_or_source_end()),
        ));
    }

    fn bump_escape_tail(&mut self) {
        match self.current_char() {
            Some('0'..='7') => {
                // JLS 3.10.7 permits three-digit octal escapes only when the
                // first digit is 0..3; otherwise the escape is one or two digits.
                let first = self.current_char().expect("octal escape starts before EOF");
                self.bump();
                if matches!(self.current_char(), Some('0'..='7')) {
                    self.bump();
                }
                if matches!(first, '0'..='3') && matches!(self.current_char(), Some('0'..='7')) {
                    self.bump();
                }
            }
            Some(_) => {
                self.bump();
            }
            None => {}
        }
    }

    fn bump_line_terminator(&mut self) {
        debug_assert!(self.current_char().is_some_and(is_line_terminator_start));
        if self.current_char() == Some('\r') && self.peek_char(1) == Some('\n') {
            self.bump();
            self.bump();
        } else {
            self.bump();
        }
    }

    fn one(&mut self, kind: JavaSyntaxKind) -> JavaSyntaxKind {
        self.bump();
        kind
    }

    fn two(&mut self, kind: JavaSyntaxKind) -> JavaSyntaxKind {
        self.bump();
        self.bump();
        kind
    }

    fn three(&mut self, kind: JavaSyntaxKind) -> JavaSyntaxKind {
        self.bump();
        self.bump();
        self.bump();
        kind
    }

    fn current(&self) -> Option<InputChar> {
        self.chars.get(self.pos).copied()
    }

    fn current_char(&self) -> Option<char> {
        self.current().map(|input| input.ch)
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).map(|input| input.ch)
    }

    fn bump(&mut self) {
        debug_assert!(!self.at_end());
        self.pos += 1;
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn previous_end(&self) -> TextSize {
        self.chars
            .get(self.pos.saturating_sub(1))
            .map_or_else(|| TextSize::new(0), |input| input.range.end())
    }

    fn previous_end_or_source_end(&self) -> TextSize {
        if self.pos == 0 {
            TextSize::new(self.source.len())
        } else {
            self.previous_end()
        }
    }

    fn raw_end_for_pos(&self, pos: usize) -> TextSize {
        self.chars.get(pos).map_or_else(
            || TextSize::new(self.source.len()),
            |input| input.range.end(),
        )
    }

    fn is_ignored_final_sub(&self) -> bool {
        self.pos + 1 == self.chars.len() && self.current_char() == Some('\u{001A}')
    }

    fn logical_text(&self, start: usize, end: usize) -> String {
        self.chars[start..end]
            .iter()
            .map(|input| input.ch)
            .collect()
    }
}

fn find_block_comment_end(chars: &[InputChar], start: usize) -> Option<usize> {
    let mut pos = start + 2;
    while pos + 1 < chars.len() {
        if chars[pos].ch == '*' && chars[pos + 1].ch == '/' {
            return Some(pos + 2);
        }
        pos += 1;
    }
    None
}

fn contains_line_terminator(chars: &[InputChar]) -> bool {
    chars.iter().any(|input| matches!(input.ch, '\r' | '\n'))
}

fn is_horizontal_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000C}')
}

fn is_line_terminator_start(ch: char) -> bool {
    matches!(ch, '\r' | '\n')
}

fn is_java_identifier_start(ch: char) -> bool {
    matches!(
        get_general_category(ch),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::LetterNumber
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ConnectorPunctuation
    )
}

fn is_java_identifier_part(ch: char) -> bool {
    is_java_identifier_start(ch)
        || matches!(
            get_general_category(ch),
            GeneralCategory::DecimalNumber
                | GeneralCategory::NonspacingMark
                | GeneralCategory::SpacingMark
                | GeneralCategory::EnclosingMark
        )
        || is_identifier_ignorable(ch)
}

fn is_identifier_ignorable(ch: char) -> bool {
    matches!(ch, '\u{0000}'..='\u{0008}' | '\u{000E}'..='\u{001B}' | '\u{007F}'..='\u{009F}')
        || get_general_category(ch) == GeneralCategory::Format
}

fn is_valid_escape_tail(ch: char) -> bool {
    matches!(
        ch,
        'b' | 's' | 't' | 'n' | 'f' | 'r' | '"' | '\'' | '\\' | '0'..='7'
    )
}

fn underscores_are_between_digits(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0usize;
    while pos < chars.len() {
        if chars[pos] != '_' {
            pos += 1;
            continue;
        }

        let start = pos;
        while pos < chars.len() && chars[pos] == '_' {
            pos += 1;
        }

        if start == 0
            || pos == chars.len()
            || !is_numeric_digit_for_literal(text, chars[start - 1])
            || !is_numeric_digit_for_literal(text, chars[pos])
        {
            return false;
        }
    }

    true
}

fn is_numeric_digit_for_literal(text: &str, ch: char) -> bool {
    if text.starts_with("0x") || text.starts_with("0X") {
        ch.is_ascii_hexdigit()
    } else {
        ch.is_ascii_digit()
    }
}

fn exponent_is_missing_digits(text: &str, kind: JavaSyntaxKind) -> bool {
    if kind != JavaSyntaxKind::FloatingPointLiteral {
        return false;
    }

    let markers = if text.starts_with("0x") || text.starts_with("0X") {
        ['p', 'P']
    } else {
        ['e', 'E']
    };

    for (index, ch) in text.char_indices() {
        if !markers.contains(&ch) {
            continue;
        }
        let after_marker = &text[index + ch.len_utf8()..];
        let digits = after_marker
            .strip_prefix(['+', '-'])
            .unwrap_or(after_marker)
            .trim_end_matches(['f', 'F', 'd', 'D']);
        if !digits.chars().any(|ch| ch.is_ascii_digit()) {
            return true;
        }
    }

    false
}
