// Java SE 26 lexical specification:
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-3.html
//
// Java lexer focused-test bar. Focused tests should cover:
//
// - every lexical category in JLS Chapter 3, using small representative
//   snippets rather than every possible spelling;
// - every lexer ambiguity or longest-match boundary, including Unicode escape
//   translation, line terminators, comments, contextual keyword spellings,
//   numeric literal boundaries, text blocks, and adjacent operators;
// - token/trivia reconstruction behavior when source ownership or attachment
//   policy matters to the formatter;
// - diagnostics and recovery shapes for malformed input the lexer accepts
//   losslessly;
// - regression tests grounded in actual bugs we have written.
//
// Focused tests should not try to enumerate the combinatorial product of the
// lexical grammar. Each test should make one source-shape claim obvious.

use jolt_diagnostics::{DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_text::{TextRange, TextSize};

use super::{
    JavaLexDiagnosticCode, JavaLexer, JavaSyntaxKind, JavaTokenSource, LexerDiagnostic, Token,
    TriviaKind,
};

struct Lexed {
    tokens: Vec<Token>,
    diagnostics: Vec<LexerDiagnostic>,
}

fn lex(source: &str) -> Lexed {
    let mut lexer = JavaLexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let at_eof = token.kind == JavaSyntaxKind::Eof;
        tokens.push(token);
        if at_eof {
            break;
        }
    }
    let diagnostics = lexer.finish();
    Lexed {
        tokens,
        diagnostics,
    }
}

fn real_tokens(source: &str) -> Vec<JavaSyntaxKind> {
    lex(source)
        .tokens
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != JavaSyntaxKind::Eof)
        .collect()
}

fn diagnostic_codes(source: &str) -> Vec<DiagnosticCodeId> {
    lex(source)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn reconstructed(source: &str) -> String {
    let lexed = lex(source);
    let mut out = String::new();
    for token in lexed.tokens {
        for trivia in token.leading {
            out.push_str(&source[trivia.range.start().get()..trivia.range.end().get()]);
        }
        out.push_str(&source[token.range.start().get()..token.range.end().get()]);
        for trivia in token.trailing {
            out.push_str(&source[trivia.range.start().get()..trivia.range.end().get()]);
        }
    }
    out
}

#[test]
fn token_source_supports_lookahead_without_bumping() {
    let mut source = JavaTokenSource::new("class A {}");

    assert_eq!(source.current().kind, JavaSyntaxKind::ClassKw);
    assert_eq!(source.nth(1).kind, JavaSyntaxKind::Identifier);
    assert_eq!(source.nth(2).kind, JavaSyntaxKind::LBrace);
    assert_eq!(source.current().kind, JavaSyntaxKind::ClassKw);

    source.bump();
    assert_eq!(source.current().kind, JavaSyntaxKind::Identifier);
    source.bump();
    assert_eq!(source.current().kind, JavaSyntaxKind::LBrace);
}

#[test]
fn token_source_can_rewind_to_checkpoint() {
    let mut source = JavaTokenSource::new("class A {}");
    let checkpoint = source.checkpoint();

    assert_eq!(source.nth(2).kind, JavaSyntaxKind::LBrace);
    source.bump();
    source.bump();
    assert_eq!(source.current().kind, JavaSyntaxKind::LBrace);

    source.rewind(checkpoint);
    assert_eq!(source.current().kind, JavaSyntaxKind::ClassKw);
    assert_eq!(source.nth(1).kind, JavaSyntaxKind::Identifier);
}

#[test]
fn lexer_rewind_restores_diagnostics() {
    let mut lexer = JavaLexer::new("/* unterminated");
    let checkpoint = lexer.checkpoint();

    assert_eq!(lexer.next_token().kind, JavaSyntaxKind::Eof);
    lexer.rewind(checkpoint);
    assert_eq!(lexer.next_token().kind, JavaSyntaxKind::Eof);
    assert_eq!(lexer.finish().len(), 1);
}

#[test]
fn uses_longest_match_for_decimal_floats_before_dots() {
    // Spec: JLS 3.2 Lexical Translations.
    assert_eq!(
        real_tokens("1... 1..2"),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::Dot,
            JavaSyntaxKind::Dot,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn uses_longest_match_for_adjacent_minus_operators() {
    // Spec: JLS 3.2 Lexical Translations.
    assert_eq!(
        real_tokens("a--b a- -b a -> b"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::MinusMinus,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Arrow,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn uses_longest_match_for_keyword_and_literal_prefixes() {
    // Spec: JLS 3.2 Lexical Translations.
    assert_eq!(
        real_tokens("publicstatic truex falsex nullx"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn uses_longest_match_for_adjacent_separators() {
    // Spec: JLS 3.2 Lexical Translations.
    assert_eq!(
        real_tokens(".... :: :"),
        vec![
            JavaSyntaxKind::Ellipsis,
            JavaSyntaxKind::Dot,
            JavaSyntaxKind::DoubleColon,
            JavaSyntaxKind::Colon,
        ]
    );
}

#[test]
fn rejects_non_ascii_separator_and_operator_lookalikes() {
    // Spec: JLS 3.1 Unicode and JLS 3.11/3.12 punctuation tables.
    assert_eq!(
        real_tokens("a\u{FF1B}b a\u{FF0B}b"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn preserves_raw_ranges_for_unicode_escape_tokens() {
    // Spec: JLS 3.3 Unicode Escapes.
    // Contract: token ranges remain raw-source ranges for lossless formatting.
    let source = "cl\\u0061ss A {}";
    let lexed = lex(source);
    assert_eq!(lexed.tokens[0].kind, JavaSyntaxKind::ClassKw);
    assert_eq!(
        &source[lexed.tokens[0].range.start().get()..lexed.tokens[0].range.end().get()],
        "cl\\u0061ss"
    );
    assert_eq!(reconstructed(source), source);
}

#[test]
fn accepts_unicode_escapes_with_multiple_u_markers() {
    // Spec: JLS 3.3 Unicode Escapes permits one or more `u` marker characters.
    assert_eq!(real_tokens("\\uu0069\\uu0066"), vec![JavaSyntaxKind::IfKw]);
}

#[test]
fn raw_backslash_without_lowercase_u_is_not_a_unicode_escape() {
    // Spec: JLS 3.3 Unicode Escapes.
    let malformed_unicode_errors = diagnostic_codes("\\x \\U0061")
        .into_iter()
        .filter(|kind| *kind == JavaLexDiagnosticCode::MalformedUnicodeEscape.id())
        .collect::<Vec<_>>();
    assert!(malformed_unicode_errors.is_empty());
}

#[test]
fn raw_backslash_without_lowercase_u_remains_input() {
    // Spec: JLS 3.3 Unicode Escapes.
    let source = "a\\x b\\U0061";
    assert_eq!(reconstructed(source), source);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn respects_unicode_escape_backslash_eligibility() {
    // Spec: JLS 3.3 Unicode Escapes defines eligibility by the preceding
    // contiguous raw backslash count and by whether the previous backslash came
    // from a Unicode escape.
    assert_eq!(
        real_tokens("\\\\u0061 \\u005c\\u0061 \\u005cu0061"),
        vec![
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn diagnoses_malformed_eligible_unicode_escapes() {
    // Spec: JLS 3.3 Unicode Escapes requires four hexadecimal digits after the
    // eligible backslash and `u` marker sequence.
    assert_eq!(
        diagnostic_codes("\\u12G4 \\u \\u123 \\uu12")
            .into_iter()
            .filter(|kind| *kind == JavaLexDiagnosticCode::MalformedUnicodeEscape.id())
            .collect::<Vec<_>>(),
        vec![
            JavaLexDiagnosticCode::MalformedUnicodeEscape.id(),
            JavaLexDiagnosticCode::MalformedUnicodeEscape.id(),
            JavaLexDiagnosticCode::MalformedUnicodeEscape.id(),
            JavaLexDiagnosticCode::MalformedUnicodeEscape.id(),
        ]
    );
}

#[test]
fn unicode_line_escape_is_a_newline_before_string_scanning() {
    // Spec: JLS 3.3 Unicode Escapes.
    // Unicode escapes are translated before tokenization, so `\u000a` is an
    // actual line terminator here, not six source characters inside a string.
    let diagnostics = lex("\"a\\u000ab\"").diagnostics;
    assert_eq!(
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec![
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
        ]
    );
}

#[test]
fn unicode_line_escape_is_a_newline_before_character_scanning() {
    // Spec: JLS 3.3 Unicode Escapes.
    // Unicode escapes are translated before tokenization, so line-terminator
    // escapes cannot appear inside a character literal.
    assert_eq!(
        diagnostic_codes("'\\u000a' '\\u000d'"),
        vec![
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
}

#[test]
fn unicode_line_escape_terminates_line_comments_before_tokenization() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        real_tokens("int a; // comment \\u000a int b;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn unicode_escapes_become_character_literal_delimiters_before_scanning() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        real_tokens("'\\u0027'"),
        vec![
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
        ]
    );
}

#[test]
fn unicode_escaped_character_delimiter_does_not_escape_character_literal() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        diagnostic_codes("'\\u0027'"),
        vec![
            JavaLexDiagnosticCode::InvalidCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
}

#[test]
fn unicode_escapes_become_string_literal_delimiters_before_scanning() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        real_tokens("\"\\u0022\""),
        vec![JavaSyntaxKind::TextBlockLiteral]
    );
}

#[test]
fn unicode_escaped_string_delimiter_obeys_text_block_rules() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        diagnostic_codes("\"\\u0022\""),
        vec![
            JavaLexDiagnosticCode::MissingTextBlockLineTerminator.id(),
            JavaLexDiagnosticCode::UnterminatedTextBlock.id(),
        ]
    );
}

#[test]
fn recognizes_cr_and_crlf_line_comment_terminators() {
    // Spec: JLS 3.4 Line Terminators.
    assert_eq!(
        real_tokens("int a; // cr\rint b; // crlf\r\nint c;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn unicode_newline_like_characters_are_not_line_terminators() {
    // Spec: JLS 3.4 Line Terminators.
    assert_eq!(
        real_tokens("int x; // comment\u{2028} int y;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
    let lexed = lex("\"a\u{2028}b\"");
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens("\"a\u{2028}b\""),
        vec![JavaSyntaxKind::StringLiteral]
    );
}

#[test]
fn keeps_crlf_as_one_line_terminator_trivia() {
    // Spec: JLS 3.4 Line Terminators.
    let source = "a\r\nb";
    let lexed = lex(source);
    let b = lexed
        .tokens
        .iter()
        .find(|token| token.kind == JavaSyntaxKind::Identifier && token.range.start().get() == 3)
        .expect("identifier after CRLF");
    assert_eq!(b.leading.len(), 1);
    assert_eq!(b.leading[0].kind, TriviaKind::Newline);
    assert_eq!(
        &source[b.leading[0].range.start().get()..b.leading[0].range.end().get()],
        "\r\n"
    );
}

#[test]
fn keeps_unicode_escape_crlf_as_one_line_terminator_trivia() {
    // Spec: JLS 3.3 Unicode Escapes and JLS 3.4 Line Terminators.
    let source = "a\\u000d\\u000ab";
    let lexed = lex(source);
    let b = lexed
        .tokens
        .iter()
        .find(|token| token.kind == JavaSyntaxKind::Identifier && token.range.start().get() == 13)
        .expect("identifier after escaped CRLF");
    assert_eq!(b.leading.len(), 1);
    assert_eq!(b.leading[0].kind, TriviaKind::Newline);
    assert_eq!(
        &source[b.leading[0].range.start().get()..b.leading[0].range.end().get()],
        "\\u000d\\u000a"
    );
}

#[test]
fn recognizes_unicode_escape_surrogate_pair_identifier() {
    // Spec: JLS 3.3 Unicode Escapes.
    assert_eq!(
        real_tokens("\\uD835\\uDC82"),
        vec![JavaSyntaxKind::Identifier]
    );
}

#[test]
fn ignores_trailing_sub_character() {
    // Spec: JLS 3.5 Input Elements ignores final SUB/control-Z.
    // Contract: ignored input still appears as trivia for lossless formatting.
    let source = "class A {}\u{001A}";
    assert_eq!(
        reconstructed(source),
        source,
        "ignored final SUB must still be present in trivia for lossless formatting"
    );
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::ClassKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::LBrace,
            JavaSyntaxKind::RBrace,
        ]
    );
}

#[test]
fn ignores_trailing_sub_after_unicode_escape_translation() {
    // Spec: JLS 3.5 Input Elements ignores final SUB/control-Z after Unicode
    // escape processing.
    let source = "class A {}\\u001a";
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::ClassKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::LBrace,
            JavaSyntaxKind::RBrace,
        ]
    );
    assert_eq!(reconstructed(source), source);
}

#[test]
fn does_not_ignore_non_trailing_sub_character() {
    // Spec: JLS 3.5 Input Elements only ignores final SUB/control-Z, while
    // JLS 3.8 Identifiers still allows identifier-ignorable characters in identifiers.
    let source = "a\u{001A}b";
    let lexed = lex(source);
    assert_eq!(reconstructed(source), source);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::Identifier]);
    assert_eq!(
        &source[lexed.tokens[0].range.start().get()..lexed.tokens[0].range.end().get()],
        source
    );
}

#[test]
fn does_not_ignore_non_trailing_sub_outside_identifier() {
    // Spec: JLS 3.5 Input Elements.
    assert_eq!(
        real_tokens("a; \u{001A} b;"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn attaches_trailing_line_comment_before_newline() {
    // Local contract, not JLS coverage: same-line comments are trailing trivia
    // for the previous token.
    let source = "int x; // trailing\nclass A {}";
    let lexed = lex(source);
    let semicolon = lexed
        .tokens
        .iter()
        .find(|token| token.kind == JavaSyntaxKind::Semicolon)
        .expect("semicolon token");
    assert_eq!(
        semicolon
            .trailing
            .iter()
            .map(|trivia| trivia.kind)
            .collect::<Vec<_>>(),
        vec![TriviaKind::Whitespace, TriviaKind::LineComment]
    );
    assert_eq!(reconstructed(source), source);
}

#[test]
fn separates_tokens_with_tab_and_form_feed_whitespace() {
    // Spec: JLS 3.6 White Space.
    assert_eq!(
        real_tokens("static\tvoid\u{000C}form"),
        vec![
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::VoidKw,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn separates_tokens_with_all_jls_whitespace() {
    // Spec: JLS 3.6 White Space.
    assert_eq!(
        real_tokens("static void\nclass\rinterface\r\nenum\tassert\u{000C}while"),
        vec![
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::VoidKw,
            JavaSyntaxKind::ClassKw,
            JavaSyntaxKind::InterfaceKw,
            JavaSyntaxKind::EnumKw,
            JavaSyntaxKind::AssertKw,
            JavaSyntaxKind::WhileKw,
        ]
    );
}

#[test]
fn rejects_non_jls_whitespace_as_token_separator() {
    // Spec: JLS 3.6 White Space.
    assert_eq!(
        real_tokens("static\u{000B}void static\u{00A0}void"),
        vec![
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::VoidKw,
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::VoidKw,
        ]
    );
}

#[test]
fn comments_separate_otherwise_adjacent_tokens() {
    // Spec: JLS 3.5 Input Elements.
    assert_eq!(
        real_tokens("static/**/void a-/*x*/=b"),
        vec![
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::VoidKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Assign,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn block_comments_do_not_nest() {
    // Spec: JLS 3.7 Comments.
    let source = "int a; /* outer /* inner */ int b;";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn block_comments_close_after_repeated_stars() {
    // Spec: JLS 3.7 Comments.
    let source = "int a; /****/ int b;";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn block_comments_may_contain_line_terminators() {
    // Spec: JLS 3.7 Comments.
    let source = "int a; /* one\n two\r\n three\r */ int b;";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn line_comments_ignore_block_comment_markers() {
    // Spec: JLS 3.7 Comments; `/*` and `*/` have no special meaning in a line comment.
    assert_eq!(
        real_tokens("int a; // /* not a block */\nint b;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn line_comments_can_end_at_eof() {
    // Spec: JLS 3.7 Comments.
    assert_eq!(
        real_tokens("int a; // eof comment"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn block_comments_ignore_line_comment_markers() {
    // Spec: JLS 3.7 Comments; `//` has no special meaning in a block comment.
    assert_eq!(
        real_tokens("int a; /* // not a line comment */ int b;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn documentation_comments_ignore_line_comment_markers() {
    // Spec: JLS 3.7 Comments; documentation comments are traditional comments
    // that begin with `/**`.
    assert_eq!(
        real_tokens("int a; /** // not a line comment */ int b;"),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn comment_markers_inside_string_literals_are_literal_content() {
    // Spec: JLS 3.7 Comments.
    let source = "\"/* not a comment */\" \"// not a comment\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![JavaSyntaxKind::StringLiteral, JavaSyntaxKind::StringLiteral]
    );
}

#[test]
fn comment_markers_inside_character_literals_are_literal_content() {
    // Spec: JLS 3.7 Comments.
    let source = "'/'";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::CharacterLiteral]);
}

#[test]
fn comment_markers_inside_text_blocks_are_literal_content() {
    // Spec: JLS 3.7 Comments.
    let source = "\"\"\"\n/* text */\n// text\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn non_ascii_characters_are_allowed_inside_comments() {
    // Spec: JLS 3.7 Comments.
    let source = "/* \u{1D482} */ int x;";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Semicolon,
        ]
    );
}

#[test]
fn reports_unterminated_block_comments() {
    // Spec: JLS 3.7 Comments.
    assert_eq!(
        diagnostic_codes("/* c"),
        vec![JavaLexDiagnosticCode::UnterminatedBlockComment.id()]
    );
}

#[test]
fn reports_unterminated_character_literals() {
    // Spec: JLS 3.10.4 Character Literals.
    assert_eq!(
        diagnostic_codes("'a\n"),
        vec![JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id()]
    );
}

#[test]
fn reports_character_literals_unterminated_at_eof() {
    // Spec: JLS 3.10.4 Character Literals.
    assert_eq!(
        diagnostic_codes("'a"),
        vec![JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id()]
    );
    assert_eq!(
        diagnostic_codes("'\\n"),
        vec![JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id()]
    );
}

#[test]
fn reports_unterminated_string_literals() {
    // Spec: JLS 3.10.5 String Literals.
    assert_eq!(
        diagnostic_codes("\"b\n"),
        vec![JavaLexDiagnosticCode::UnterminatedStringLiteral.id()]
    );
}

#[test]
fn reports_string_literals_unterminated_at_eof() {
    // Spec: JLS 3.10.5 String Literals.
    assert_eq!(
        diagnostic_codes("\"abc"),
        vec![JavaLexDiagnosticCode::UnterminatedStringLiteral.id()]
    );
}

#[test]
fn recognizes_java_currency_symbol_identifier_start() {
    // Spec: JLS 3.8 Identifiers defines Java letters via Character.isJavaIdentifierStart.
    assert_eq!(
        real_tokens("\u{00A2}value"),
        vec![JavaSyntaxKind::Identifier]
    );
}

#[test]
fn recognizes_dollar_and_digits_in_identifiers() {
    // Spec: JLS 3.8 Identifiers defines Java letters and Java letter-or-digit characters.
    assert_eq!(
        real_tokens("$value value1 _value"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn recognizes_identifier_part_that_is_not_identifier_start() {
    // Spec: JLS 3.8 Identifiers.
    assert_eq!(real_tokens("a\u{0301}"), vec![JavaSyntaxKind::Identifier]);
}

#[test]
fn rejects_identifier_part_as_identifier_start() {
    // Spec: JLS 3.8 Identifiers.
    assert_eq!(
        real_tokens("\u{0301}a"),
        vec![JavaSyntaxKind::Unknown, JavaSyntaxKind::Identifier]
    );
}

#[test]
fn recognizes_raw_supplementary_identifier_start() {
    // Spec: JLS 3.8 Identifiers.
    assert_eq!(real_tokens("𝒂"), vec![JavaSyntaxKind::Identifier]);
}

#[test]
fn recognizes_all_reserved_keywords() {
    // Spec: JLS 3.9 Keywords.
    assert_eq!(
        real_tokens(
            "abstract assert boolean break byte case catch char class const continue default do \
             double else enum extends final finally float for goto if implements import instanceof \
             int interface long native new package private protected public return short static \
             strictfp super switch synchronized this throw throws transient try void volatile while _",
        ),
        vec![
            JavaSyntaxKind::AbstractKw,
            JavaSyntaxKind::AssertKw,
            JavaSyntaxKind::BooleanKw,
            JavaSyntaxKind::BreakKw,
            JavaSyntaxKind::ByteKw,
            JavaSyntaxKind::CaseKw,
            JavaSyntaxKind::CatchKw,
            JavaSyntaxKind::CharKw,
            JavaSyntaxKind::ClassKw,
            JavaSyntaxKind::ConstKw,
            JavaSyntaxKind::ContinueKw,
            JavaSyntaxKind::DefaultKw,
            JavaSyntaxKind::DoKw,
            JavaSyntaxKind::DoubleKw,
            JavaSyntaxKind::ElseKw,
            JavaSyntaxKind::EnumKw,
            JavaSyntaxKind::ExtendsKw,
            JavaSyntaxKind::FinalKw,
            JavaSyntaxKind::FinallyKw,
            JavaSyntaxKind::FloatKw,
            JavaSyntaxKind::ForKw,
            JavaSyntaxKind::GotoKw,
            JavaSyntaxKind::IfKw,
            JavaSyntaxKind::ImplementsKw,
            JavaSyntaxKind::ImportKw,
            JavaSyntaxKind::InstanceofKw,
            JavaSyntaxKind::IntKw,
            JavaSyntaxKind::InterfaceKw,
            JavaSyntaxKind::LongKw,
            JavaSyntaxKind::NativeKw,
            JavaSyntaxKind::NewKw,
            JavaSyntaxKind::PackageKw,
            JavaSyntaxKind::PrivateKw,
            JavaSyntaxKind::ProtectedKw,
            JavaSyntaxKind::PublicKw,
            JavaSyntaxKind::ReturnKw,
            JavaSyntaxKind::ShortKw,
            JavaSyntaxKind::StaticKw,
            JavaSyntaxKind::StrictfpKw,
            JavaSyntaxKind::SuperKw,
            JavaSyntaxKind::SwitchKw,
            JavaSyntaxKind::SynchronizedKw,
            JavaSyntaxKind::ThisKw,
            JavaSyntaxKind::ThrowKw,
            JavaSyntaxKind::ThrowsKw,
            JavaSyntaxKind::TransientKw,
            JavaSyntaxKind::TryKw,
            JavaSyntaxKind::VoidKw,
            JavaSyntaxKind::VolatileKw,
            JavaSyntaxKind::WhileKw,
            JavaSyntaxKind::UnderscoreKw,
        ]
    );
}

#[test]
fn recognizes_boolean_literals() {
    // Spec: JLS 3.10.3 Boolean Literals.
    assert_eq!(
        real_tokens("true false"),
        vec![
            JavaSyntaxKind::BooleanLiteral,
            JavaSyntaxKind::BooleanLiteral
        ]
    );
}

#[test]
fn recognizes_null_literal() {
    // Spec: JLS 3.10.8 The Null Literal.
    assert_eq!(real_tokens("null"), vec![JavaSyntaxKind::NullLiteral]);
}

#[test]
fn contextual_keyword_spellings_remain_identifiers_without_parser_context() {
    // Spec: JLS 3.9 Keywords; contextual keywords are recognized only by parser context
    // and only when they are not adjacent to Java letters or digits.
    assert_eq!(
        real_tokens(
            "exports opens requires uses yield module permits sealed var non-sealed provides to \
             when open record transitive with varfilename yieldx non-sealedclass",
        ),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn unicode_lookalikes_do_not_match_keywords_or_literals() {
    // Spec: JLS 3.8 Identifiers, JLS 3.9 Keywords, JLS 3.10.3 Boolean Literals,
    // and JLS 3.10.8 The Null Literal.
    assert_eq!(
        real_tokens(
            "\u{FF49}\u{FF46} \u{FF54}\u{FF52}\u{FF55}\u{FF45} \
             \u{FF4E}\u{FF55}\u{FF4C}\u{FF4C}",
        ),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn recognizes_integer_literal_radices_and_suffixes() {
    // Spec: JLS 3.10.1 Integer Literals.
    let source = "0 00 0_7 0__7 0XCAFE 0xdada_cafe 0b1010 0B1010 123L 123l 0xFFL 0777L 0b1L";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
        ]
    );
}

#[test]
fn diagnoses_malformed_integer_literals() {
    // Spec: JLS 3.10.1 Integer Literals.
    assert_eq!(
        diagnostic_codes("0x_1 1_ 0x 0X 0b 0B 0x1_ 0b1_ 01_ 1_L"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn permits_multiple_underscores_between_integer_digits() {
    // Spec: JLS 3.10.1 Integer Literals.
    let lexed = lex("1__2 0x1__2 0b1__0");
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens("1__2 0x1__2 0b1__0"),
        vec![
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
        ]
    );
}

#[test]
fn recognizes_lowercase_integer_suffix_for_non_decimal_radices() {
    // Spec: JLS 3.10.1 Integer Literals.
    let source = "0xFFl 0777l 0b1l";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
        ]
    );
}

#[test]
fn diagnoses_invalid_hexadecimal_digits() {
    // Spec: JLS 3.10.1 Integer Literals.
    assert_eq!(
        diagnostic_codes("0xG 0XCAFE_Z"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn diagnoses_underscores_next_to_floating_point_parts() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    assert_eq!(
        diagnostic_codes("1e+ 1_f 1_e2 1.0_f 1_.0 1._0 0x1_.p0 0x1._p0 1e_2 1e+_2"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn permits_multiple_underscores_between_floating_point_digits() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "1e2__3 1.2__3 1__2.0 1__2e3 0x1.f__ep1 0x1p1__2 0x1__2p0 0x1__2.8p0";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn diagnoses_invalid_octal_digits() {
    // Spec: JLS 3.10.1 Integer Literals.
    assert_eq!(
        diagnostic_codes("08 09 078 0_8"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn diagnoses_invalid_binary_digits() {
    // Spec: JLS 3.10.1 Integer Literals.
    assert_eq!(
        diagnostic_codes("0b2 0b10_2 0B102"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn rejects_non_ascii_numeric_lookalikes() {
    // Spec: JLS 3.10.1 Integer Literals and JLS 3.10.2 Floating-Point Literals.
    assert_eq!(
        real_tokens("\u{0661} 0x\u{FF21} 1\u{FF26}"),
        vec![
            JavaSyntaxKind::Unknown,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::Identifier,
        ]
    );
    assert_eq!(
        diagnostic_codes("0x\u{FF21}"),
        vec![JavaLexDiagnosticCode::InvalidNumericLiteral.id()]
    );
}

#[test]
fn accepts_integer_literals_without_semantic_range_checks() {
    // Spec: JLS 3.10.1 Integer Literals.
    // Literal range is a semantic/compile-time rule. The formatter lexer keeps
    // oversized but syntactically valid literals parseable.
    let source = "2147483647 2147483648 2147483649 0xffff_ffff 0x1_0000_0000 \
                  9223372036854775807L 9223372036854775808L 9223372036854775809L \
                  0xffff_ffff_ffff_ffffL 0x1_0000_0000_0000_0000L";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
        ]
    );
}

#[test]
fn accepts_binary_and_octal_integer_literals_without_semantic_width_checks() {
    // Spec: JLS 3.10.1 Integer Literals.
    let source = "0b11111111111111111111111111111111 037777777777 \
                  0b111111111111111111111111111111111 040000000000 \
                  0b1111111111111111111111111111111111111111111111111111111111111111L \
                  01777777777777777777777L \
                  0b11111111111111111111111111111111111111111111111111111111111111111L \
                  02000000000000000000000L";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
            JavaSyntaxKind::IntegerLiteral,
        ]
    );
}

#[test]
fn diagnoses_hex_floats_without_binary_exponent() {
    // Spec: JLS 3.10.2 Floating-Point Literals requires a binary exponent for hex floats.
    assert_eq!(
        diagnostic_codes("0x1. 0x.1 0x1p 0x1p+ 0x1p_1 0xp1 0x.p1 0X.p1"),
        vec![
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
            JavaLexDiagnosticCode::InvalidNumericLiteral.id(),
        ]
    );
}

#[test]
fn recognizes_decimal_floating_point_literal_forms() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "1. .2 3e+4 5e-6 7f 8D 9.0d 1.e2 .2E-3 1.2e3F";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn recognizes_hexadecimal_floating_point_literal_forms() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "0x1p0 0x1.p-1 0x.1P+2 0X1.8p1F 0x1p0f 0x1p0d 0x1p0D";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn accepts_floating_point_literals_without_semantic_range_checks() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "3.4028235e38f 1.4e-45f 1.7976931348623157e308 4.9e-324 \
                  3.5e38f 1e-46f 1e309 1e-325";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn accepts_hexadecimal_floating_point_literals_without_semantic_range_checks() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "0x1.fffffeP+127f 0x1.0P-149f 0x1.f_ffff_ffff_ffffP+1023 \
                  0x1.0P-1074 0x1.0p128f 0x1.0p-150f 0x1.0p1024 0x1.0p-1075";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn accepts_alternate_hexadecimal_minimum_floating_point_literals() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "0x0.000002P-126f 0x0.0_0000_0000_0001P-1022";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn permits_zero_floating_point_literals_with_extreme_exponents() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "0e-999999 0x0p-999999 0e999999 0x0p999999 0f 0d";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn permits_nonzero_floating_point_literals_that_round_to_subnormal() {
    // Spec: JLS 3.10.2 Floating-Point Literals.
    let source = "1e-45f 2.5e-324";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::FloatingPointLiteral,
            JavaSyntaxKind::FloatingPointLiteral,
        ]
    );
}

#[test]
fn recognizes_single_character_literals() {
    // Spec: JLS 3.10.4 Character Literals.
    let source = "'a' '%' '\u{2122}'";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
        ]
    );
}

#[test]
fn recognizes_raw_double_quote_character_literal() {
    // Spec: JLS 3.10.4 Character Literals.
    let source = "'\"'";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::CharacterLiteral]);
}

#[test]
fn recognizes_single_utf16_code_unit_character_literals() {
    // Spec: JLS 3.10.4 Character Literals.
    let source = "'\\uFFFF' '\\uD800'";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
        ]
    );
}

#[test]
fn rejects_character_literals_with_multiple_utf16_code_units() {
    // Spec: JLS 3.10.4 Character Literals are limited to one UTF-16 code unit.
    assert_eq!(
        diagnostic_codes("'𝒂'"),
        vec![JavaLexDiagnosticCode::InvalidCharacterLiteral.id()]
    );
    assert_eq!(
        diagnostic_codes("'\\uD835\\uDC82'"),
        vec![JavaLexDiagnosticCode::InvalidCharacterLiteral.id()]
    );
}

#[test]
fn rejects_empty_and_multi_character_literals() {
    // Spec: JLS 3.10.4 Character Literals require exactly one character or escape sequence.
    assert_eq!(
        diagnostic_codes("'' 'ab'"),
        vec![
            JavaLexDiagnosticCode::InvalidCharacterLiteral.id(),
            JavaLexDiagnosticCode::InvalidCharacterLiteral.id(),
        ]
    );
}

#[test]
fn recognizes_escape_sequences_in_character_and_string_literals() {
    // Spec: JLS 3.10.7 Escape Sequences.
    // Context: character and string literals use the same escape inventory.
    let source = r#"'\b' '\s' '\t' '\n' '\f' '\r' '\"' '\'' '\\' '\0' '\77' '\377' "\b\s\t\n\f\r\"\'\\\0\77\377""#;
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::CharacterLiteral,
            JavaSyntaxKind::StringLiteral,
        ]
    );
}

#[test]
fn recognizes_empty_string_literals() {
    // Spec: JLS 3.10.5 String Literals.
    let source = "\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::StringLiteral]);
}

#[test]
fn recognizes_non_empty_string_literals() {
    // Spec: JLS 3.10.5 String Literals.
    let source = "\"abc\" \"hello world\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![JavaSyntaxKind::StringLiteral, JavaSyntaxKind::StringLiteral]
    );
}

#[test]
fn recognizes_raw_single_quote_string_literal_content() {
    // Spec: JLS 3.10.5 String Literals.
    let source = "\"can't\" \"'\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens(source),
        vec![JavaSyntaxKind::StringLiteral, JavaSyntaxKind::StringLiteral]
    );
}

#[test]
fn recognizes_non_ascii_string_literal_content() {
    // Spec: JLS 3.10.5 String Literals.
    let source = "\"\u{2122}\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::StringLiteral]);
}

#[test]
fn recognizes_supplementary_string_literal_content() {
    // Spec: JLS 3.10.5 String Literals.
    let source = "\"\\uD835\\uDC82\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::StringLiteral]);
}

#[test]
fn rejects_line_continuation_escape_in_character_literals() {
    // Spec: JLS 3.10.4 Character Literals disallows line terminators before the
    // closing quote, even after a backslash. The InvalidEscapeSequence diagnostic
    // is this lexer's recovery wording for the backslash before that line terminator.
    assert_eq!(
        diagnostic_codes("'\\\n'"),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
}

#[test]
fn rejects_cr_and_crlf_line_continuation_escape_in_character_literals() {
    // Spec: JLS 3.10.4 Character Literals.
    assert_eq!(
        diagnostic_codes("'\\\r'"),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
    assert_eq!(
        diagnostic_codes("'\\\r\n'"),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
}

#[test]
fn rejects_raw_cr_and_crlf_in_character_literals() {
    // Spec: JLS 3.10.4 Character Literals.
    assert_eq!(
        diagnostic_codes("'\r' '\r\n'"),
        vec![
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedCharacterLiteral.id(),
        ]
    );
}

#[test]
fn rejects_line_continuation_escape_in_string_literals() {
    // Spec: JLS 3.10.5 String Literals disallows line terminators before the
    // closing quote, even after a backslash. The InvalidEscapeSequence diagnostic
    // is this lexer's recovery wording for the backslash before that line terminator.
    assert_eq!(
        diagnostic_codes("\"hello\\\nworld\""),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
        ]
    );
}

#[test]
fn rejects_cr_and_crlf_line_continuation_escape_in_string_literals() {
    // Spec: JLS 3.10.5 String Literals.
    assert_eq!(
        diagnostic_codes("\"hello\\\rworld\""),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
        ]
    );
    assert_eq!(
        diagnostic_codes("\"hello\\\r\nworld\""),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
        ]
    );
}

#[test]
fn rejects_raw_cr_and_crlf_in_string_literals() {
    // Spec: JLS 3.10.5 String Literals.
    assert_eq!(
        diagnostic_codes("\"a\rb\" \"a\r\nb\""),
        vec![
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
            JavaLexDiagnosticCode::UnterminatedStringLiteral.id(),
        ]
    );
}

#[test]
fn reports_invalid_escape_sequences_in_string_and_character_literals() {
    // Spec: JLS 3.10.7 Escape Sequences.
    assert_eq!(
        diagnostic_codes(r#""\q" '\q' "\8" "\9" '\8' '\9'"#),
        vec![
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
            JavaLexDiagnosticCode::InvalidEscapeSequence.id(),
        ]
    );
}

#[test]
fn permits_text_block_opening_whitespace_before_line_terminator() {
    // Spec: JLS 3.10.6 Text Blocks permits whitespace before the opening line terminator.
    let lexed = lex("\"\"\" \t\u{000C}\r\nhello\r\n\"\"\"");
    assert_eq!(
        lexed
            .tokens
            .iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != JavaSyntaxKind::Eof)
            .collect::<Vec<_>>(),
        vec![JavaSyntaxKind::TextBlockLiteral]
    );
    assert_eq!(lexed.diagnostics, vec![]);
}

#[test]
fn permits_text_block_opening_cr_line_terminator() {
    // Spec: JLS 3.10.6 Text Blocks.
    let source = "\"\"\"\rhello\r\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn recognizes_empty_text_blocks() {
    // Spec: JLS 3.10.6 Text Blocks.
    let source = "\"\"\"\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn rejects_text_block_without_opening_line_terminator() {
    // Spec: JLS 3.10.6 Text Blocks requires optional whitespace after the opening
    // delimiter to be followed by a line terminator.
    assert_eq!(
        diagnostic_codes("\"\"\"hello\"\"\""),
        vec![JavaLexDiagnosticCode::MissingTextBlockLineTerminator.id()]
    );
}

#[test]
fn rejects_text_block_opening_whitespace_without_line_terminator() {
    // Spec: JLS 3.10.6 Text Blocks.
    assert_eq!(
        diagnostic_codes("\"\"\" \t\u{000C}hello\"\"\""),
        vec![JavaLexDiagnosticCode::MissingTextBlockLineTerminator.id()]
    );
}

#[test]
fn rejects_text_block_opening_non_jls_whitespace() {
    // Spec: JLS 3.10.6 Text Blocks.
    assert_eq!(
        diagnostic_codes("\"\"\"\u{00A0}\ntext\n\"\"\""),
        vec![JavaLexDiagnosticCode::MissingTextBlockLineTerminator.id()]
    );
}

#[test]
fn unescaped_triple_quote_closes_text_blocks() {
    // Spec: JLS 3.10.6 Text Blocks excludes an unescaped `"""` from text block content.
    assert_eq!(
        real_tokens("\"\"\"\ntext \"\"\" tail"),
        vec![JavaSyntaxKind::TextBlockLiteral, JavaSyntaxKind::Identifier]
    );
}

#[test]
fn fourth_quote_after_text_block_closing_delimiter_is_outside_token() {
    // Spec: JLS 3.10.6 Text Blocks.
    let source = "\"\"\"\ntext\n\"\"\"\"";
    assert_eq!(
        real_tokens(source),
        vec![
            JavaSyntaxKind::TextBlockLiteral,
            JavaSyntaxKind::StringLiteral,
        ]
    );
    assert_eq!(
        diagnostic_codes(source),
        vec![JavaLexDiagnosticCode::UnterminatedStringLiteral.id()]
    );
}

#[test]
fn permits_unescaped_double_quotes_inside_text_blocks() {
    // Spec: JLS 3.10.6 Text Blocks.
    let source = "\"\"\"\n\"Bob\"\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn recognizes_non_ascii_text_block_content() {
    // Spec: JLS 3.10.6 Text Blocks.
    let source = "\"\"\"\n\u{1D482}\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn reports_unterminated_text_blocks() {
    // Spec: JLS 3.10.6 Text Blocks require a closing delimiter.
    assert_eq!(
        diagnostic_codes("\"\"\"\nabc"),
        vec![JavaLexDiagnosticCode::UnterminatedTextBlock.id()]
    );
}

#[test]
fn escaped_quote_prevents_text_block_closing_delimiter() {
    // Spec: JLS 3.10.6 Text Blocks permits escaping one quote so three quote
    // characters in content do not mimic the closing delimiter.
    let source = "\"\"\"\n\\\"\"\"\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn permits_line_continuation_escape_in_text_blocks() {
    // Spec: JLS 3.10.7 Escape Sequences.
    let source = "\"\"\"\na\\\nb\\\rc\\\r\nd\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn recognizes_escape_sequences_in_text_blocks() {
    // Spec: JLS 3.10.7 Escape Sequences.
    let source = "\"\"\"\n\\b\\s\\t\\n\\f\\r\\\"\\'\\\\\\0\\77\\377\n\"\"\"";
    let lexed = lex(source);
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(real_tokens(source), vec![JavaSyntaxKind::TextBlockLiteral]);
}

#[test]
fn reports_invalid_text_block_escapes() {
    // Spec: JLS 3.10.7 Escape Sequences.
    assert_eq!(
        diagnostic_codes("\"\"\"\n\\q\n\"\"\""),
        vec![JavaLexDiagnosticCode::InvalidEscapeSequence.id()]
    );
}

#[test]
fn two_digit_octal_escape_followed_by_digit_is_extra_character_in_char_literals() {
    // Spec: JLS 3.10.7 Escape Sequences; `\477` is `\47` followed by `7`
    // because three-digit octal escapes may start only with 0..3.
    let lexed = lex("'\\477'");
    assert_eq!(
        real_tokens("'\\477'"),
        vec![JavaSyntaxKind::CharacterLiteral]
    );
    assert_eq!(
        lexed.diagnostics,
        vec![LexerDiagnostic {
            code: JavaLexDiagnosticCode::InvalidCharacterLiteral.id(),
            severity: Severity::Error,
            stage: DiagnosticStage::Lexer,
            message: JavaLexDiagnosticCode::InvalidCharacterLiteral
                .message()
                .to_owned(),
            range: Some(TextRange::new(TextSize::new(0), TextSize::new(6))),
        }]
    );
}

#[test]
fn two_digit_octal_escape_followed_by_digit_is_valid_in_string_literals() {
    // Spec: JLS 3.10.7 Escape Sequences; `\477` is `\47` followed by `7`.
    let lexed = lex("\"\\477\"");
    assert_eq!(lexed.diagnostics, vec![]);
    assert_eq!(
        real_tokens("\"\\477\""),
        vec![JavaSyntaxKind::StringLiteral]
    );
}

#[test]
fn two_digit_octal_escape_before_non_octal_digit_falls_back() {
    // Spec: JLS 3.10.7 Escape Sequences; `\378` is `\37` followed by `8`.
    let string = lex("\"\\378\"");
    assert_eq!(string.diagnostics, vec![]);
    assert_eq!(
        real_tokens("\"\\378\""),
        vec![JavaSyntaxKind::StringLiteral]
    );
    assert_eq!(
        diagnostic_codes("'\\378'"),
        vec![JavaLexDiagnosticCode::InvalidCharacterLiteral.id()]
    );
}

#[test]
fn recognizes_all_separators() {
    // Spec: JLS 3.11 Separators.
    assert_eq!(
        real_tokens("( ) { } [ ] ; , . ... @ ::"),
        vec![
            JavaSyntaxKind::LParen,
            JavaSyntaxKind::RParen,
            JavaSyntaxKind::LBrace,
            JavaSyntaxKind::RBrace,
            JavaSyntaxKind::LBracket,
            JavaSyntaxKind::RBracket,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::Comma,
            JavaSyntaxKind::Dot,
            JavaSyntaxKind::Ellipsis,
            JavaSyntaxKind::At,
            JavaSyntaxKind::DoubleColon,
        ]
    );
}

#[test]
fn recognizes_all_operators() {
    // Spec: JLS 3.12 Operators.
    assert_eq!(
        real_tokens(
            "= > < ! ~ ? : -> == >= <= != && || ++ -- + - * / & | ^ % << >> >>> \
             += -= *= /= &= |= ^= %= <<= >>= >>>=",
        ),
        vec![
            JavaSyntaxKind::Assign,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Lt,
            JavaSyntaxKind::Bang,
            JavaSyntaxKind::Tilde,
            JavaSyntaxKind::Question,
            JavaSyntaxKind::Colon,
            JavaSyntaxKind::Arrow,
            JavaSyntaxKind::EqEq,
            JavaSyntaxKind::GtEq,
            JavaSyntaxKind::LtEq,
            JavaSyntaxKind::BangEq,
            JavaSyntaxKind::AndAnd,
            JavaSyntaxKind::OrOr,
            JavaSyntaxKind::PlusPlus,
            JavaSyntaxKind::MinusMinus,
            JavaSyntaxKind::Plus,
            JavaSyntaxKind::Minus,
            JavaSyntaxKind::Star,
            JavaSyntaxKind::Slash,
            JavaSyntaxKind::Amp,
            JavaSyntaxKind::Bar,
            JavaSyntaxKind::Caret,
            JavaSyntaxKind::Percent,
            JavaSyntaxKind::LShift,
            JavaSyntaxKind::RShift,
            JavaSyntaxKind::UnsignedRShift,
            JavaSyntaxKind::PlusEq,
            JavaSyntaxKind::MinusEq,
            JavaSyntaxKind::StarEq,
            JavaSyntaxKind::SlashEq,
            JavaSyntaxKind::AmpEq,
            JavaSyntaxKind::BarEq,
            JavaSyntaxKind::CaretEq,
            JavaSyntaxKind::PercentEq,
            JavaSyntaxKind::LShiftEq,
            JavaSyntaxKind::RShiftEq,
            JavaSyntaxKind::UnsignedRShiftEq,
        ]
    );
}

#[test]
fn recognizes_longest_match_greater_than_operators() {
    // Spec: JLS 3.12 Operators.
    // Parser note: type contexts may later split/reinterpret adjacent `>` tokens per JLS 3.5.
    assert_eq!(
        real_tokens("a >>>= b >>> c >>= d >> e >= f > g"),
        vec![
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::UnsignedRShiftEq,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::UnsignedRShift,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::RShiftEq,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::RShift,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::GtEq,
            JavaSyntaxKind::Identifier,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Identifier,
        ]
    );
}
