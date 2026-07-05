use jolt_diagnostics::DiagnosticCodeId;
use jolt_syntax::{SyntaxTrivia, TriviaKind};
use jolt_text::{TextRange, TextSize};

use super::{KotlinLexDiagnosticCode, KotlinLexer, KotlinSyntaxKind, LexerDiagnostic};

struct Lexed {
    source: String,
    tokens: Vec<Token>,
    diagnostics: Vec<LexerDiagnostic>,
}

#[derive(Debug, Eq, PartialEq)]
struct Token {
    kind: KotlinSyntaxKind,
    range: TextRange,
    leading: Vec<Trivia>,
    trailing: Vec<Trivia>,
}

#[derive(Debug, Eq, PartialEq)]
struct Trivia {
    kind: TriviaKind,
    range: TextRange,
}

fn lex(source: &str) -> Lexed {
    let mut lexer = KotlinLexer::new(source);
    let mut token_data = Vec::new();
    let mut trivia = Vec::new();
    loop {
        let token = lexer.next_token_into(&mut trivia);
        let at_eof = token.kind == KotlinSyntaxKind::Eof;
        token_data.push(token);
        if at_eof {
            break;
        }
    }
    Lexed {
        source: source.to_owned(),
        tokens: tokens_with_trivia(token_data, &trivia),
        diagnostics: lexer.finish(),
    }
}

fn tokens_with_trivia(tokens: Vec<super::LexedToken>, trivia: &[SyntaxTrivia]) -> Vec<Token> {
    tokens
        .into_iter()
        .map(|token| {
            let leading = trivia_with_ranges(
                &trivia[token.leading.clone()],
                token.range.start() - trivia_text_len(&trivia[token.leading]),
            );
            let trailing = trivia_with_ranges(&trivia[token.trailing], token.range.end());
            Token {
                kind: token.kind,
                range: token.range,
                leading,
                trailing,
            }
        })
        .collect()
}

fn trivia_with_ranges(trivia: &[SyntaxTrivia], mut offset: TextSize) -> Vec<Trivia> {
    trivia
        .iter()
        .map(|trivia| {
            let range = TextRange::new(offset, offset + trivia.text_len());
            offset += trivia.text_len();
            Trivia {
                kind: trivia.kind(),
                range,
            }
        })
        .collect()
}

fn trivia_text_len(trivia: &[SyntaxTrivia]) -> TextSize {
    trivia
        .iter()
        .fold(TextSize::new(0), |len, trivia| len + trivia.text_len())
}

fn real_tokens(source: &str) -> Vec<KotlinSyntaxKind> {
    lex(source)
        .tokens
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != KotlinSyntaxKind::Eof)
        .collect()
}

fn token_texts(source: &str) -> Vec<(KotlinSyntaxKind, String)> {
    let lexed = lex(source);
    lexed
        .tokens
        .into_iter()
        .filter(|token| token.kind != KotlinSyntaxKind::Eof)
        .map(|token| {
            let text = lexed.source[token.range.start().get()..token.range.end().get()].to_owned();
            (token.kind, text)
        })
        .collect()
}

fn diagnostic_codes(source: &str) -> Vec<DiagnosticCodeId> {
    lex(source)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn diagnostic_codes_after_consuming_tokens(
    source: &str,
    tokens_to_consume: usize,
) -> Vec<DiagnosticCodeId> {
    let mut lexer = KotlinLexer::new(source);
    let mut trivia = Vec::new();
    for _ in 0..tokens_to_consume {
        lexer.next_token_into(&mut trivia);
    }
    lexer
        .finish()
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn token_by_text_start(source: &str, start: usize) -> Token {
    lex(source)
        .tokens
        .into_iter()
        .find(|token| token.range.start().get() == start)
        .expect("token at requested start")
}

fn reconstructed(source: &str) -> String {
    let lexed = lex(source);
    let mut out = String::new();
    for token in lexed.tokens {
        for trivia in token.leading {
            out.push_str(&lexed.source[trivia.range.start().get()..trivia.range.end().get()]);
        }
        out.push_str(&lexed.source[token.range.start().get()..token.range.end().get()]);
        for trivia in token.trailing {
            out.push_str(&lexed.source[trivia.range.start().get()..trivia.range.end().get()]);
        }
    }
    out
}

#[test]
fn recognizes_hard_soft_and_modifier_keywords() {
    assert_eq!(
        real_tokens("package context data value field all class"),
        vec![
            KotlinSyntaxKind::PackageKw,
            KotlinSyntaxKind::ContextKw,
            KotlinSyntaxKind::DataKw,
            KotlinSyntaxKind::ValueKw,
            KotlinSyntaxKind::FieldKw,
            KotlinSyntaxKind::AllKw,
            KotlinSyntaxKind::ClassKw,
        ]
    );
}

#[test]
fn recognizes_backtick_and_unicode_identifiers() {
    assert_eq!(
        real_tokens("π `_ weird identifier` _x x2"),
        vec![
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn rejects_empty_backtick_identifiers() {
    assert_eq!(
        real_tokens("``"),
        vec![KotlinSyntaxKind::Unknown, KotlinSyntaxKind::Unknown]
    );
    assert_eq!(
        real_tokens("$``"),
        vec![
            KotlinSyntaxKind::Unknown,
            KotlinSyntaxKind::Unknown,
            KotlinSyntaxKind::Unknown,
        ]
    );
    assert_eq!(
        real_tokens("$`unterminated"),
        vec![KotlinSyntaxKind::Unknown, KotlinSyntaxKind::Unknown]
    );
    assert_eq!(
        diagnostic_codes("``")
            .into_iter()
            .filter(|code| *code == KotlinLexDiagnosticCode::UnterminatedBacktickIdentifier.id())
            .count(),
        2
    );
}

#[test]
fn recognizes_field_identifier_outside_strings() {
    assert_eq!(
        token_texts("$field $`backtick field`"),
        vec![
            (KotlinSyntaxKind::FieldIdentifier, "$field".to_owned()),
            (
                KotlinSyntaxKind::FieldIdentifier,
                "$`backtick field`".to_owned()
            ),
        ]
    );
}

#[test]
fn invalid_backtick_after_dollar_does_not_start_template_entry() {
    assert_eq!(
        token_texts("\"$`` $`unterminated\""),
        vec![
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "$".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "`` ".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "$".to_owned()),
            (
                KotlinSyntaxKind::RegularStringPart,
                "`unterminated".to_owned(),
            ),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
        ]
    );
}

#[test]
fn preserves_shebang_as_leading_comment_trivia_only_at_start() {
    let lexed = lex("#!/usr/bin/env kotlin\nval x = 1\n#! not shebang");
    let val = lexed
        .tokens
        .iter()
        .find(|token| token.kind == KotlinSyntaxKind::ValKw)
        .expect("val token after shebang");
    assert_eq!(
        val.leading
            .iter()
            .map(|trivia| trivia.kind)
            .collect::<Vec<_>>(),
        vec![TriviaKind::LineComment, TriviaKind::Newline]
    );
    assert_eq!(
        real_tokens("val x\n#! later"),
        vec![
            KotlinSyntaxKind::ValKw,
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Hash,
            KotlinSyntaxKind::Bang,
            KotlinSyntaxKind::Identifier,
        ]
    );
}

#[test]
fn supports_nested_block_and_doc_comments() {
    let source = "val a = 1 /* outer /* inner */ done */ /** doc */ val b = 2";
    let lexed = lex(source);
    assert!(lexed.diagnostics.is_empty());
    assert_eq!(
        real_tokens(source),
        vec![
            KotlinSyntaxKind::ValKw,
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Assign,
            KotlinSyntaxKind::IntegerLiteral,
            KotlinSyntaxKind::ValKw,
            KotlinSyntaxKind::Identifier,
            KotlinSyntaxKind::Assign,
            KotlinSyntaxKind::IntegerLiteral,
        ]
    );
    assert_eq!(reconstructed(source), source);
}

#[test]
fn diagnoses_unterminated_nested_block_comment() {
    assert_eq!(
        diagnostic_codes("val a = 1 /* outer /* inner */"),
        vec![KotlinLexDiagnosticCode::UnterminatedBlockComment.id()]
    );
}

#[test]
fn recognizes_numeric_literals_and_range_boundaries() {
    assert_eq!(
        token_texts("1..2 1..<2 1.5 .5 1e3 1f 1.foo 1.e2 0xCAFEu 0b1010UL"),
        vec![
            (KotlinSyntaxKind::IntegerLiteral, "1".to_owned()),
            (KotlinSyntaxKind::Range, "..".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "2".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "1".to_owned()),
            (KotlinSyntaxKind::RangeUntil, "..<".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "2".to_owned()),
            (KotlinSyntaxKind::FloatLiteral, "1.5".to_owned()),
            (KotlinSyntaxKind::FloatLiteral, ".5".to_owned()),
            (KotlinSyntaxKind::FloatLiteral, "1e3".to_owned()),
            (KotlinSyntaxKind::FloatLiteral, "1f".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "1".to_owned()),
            (KotlinSyntaxKind::Dot, ".".to_owned()),
            (KotlinSyntaxKind::Identifier, "foo".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "1".to_owned()),
            (KotlinSyntaxKind::Dot, ".".to_owned()),
            (KotlinSyntaxKind::Identifier, "e2".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "0xCAFEu".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "0b1010UL".to_owned()),
        ]
    );
}

#[test]
fn recognizes_operator_longest_matches_and_not_keyword_boundaries() {
    assert_eq!(
        token_texts("... === !== !in !is !inside as? ?. ?: ..< ;; => ->"),
        vec![
            (KotlinSyntaxKind::Reserved, "...".to_owned()),
            (KotlinSyntaxKind::EqEqEq, "===".to_owned()),
            (KotlinSyntaxKind::BangEqEqEq, "!==".to_owned()),
            (KotlinSyntaxKind::NotIn, "!in".to_owned()),
            (KotlinSyntaxKind::NotIs, "!is".to_owned()),
            (KotlinSyntaxKind::Bang, "!".to_owned()),
            (KotlinSyntaxKind::Identifier, "inside".to_owned()),
            (KotlinSyntaxKind::AsSafe, "as?".to_owned()),
            (KotlinSyntaxKind::SafeAccess, "?.".to_owned()),
            (KotlinSyntaxKind::Elvis, "?:".to_owned()),
            (KotlinSyntaxKind::RangeUntil, "..<".to_owned()),
            (KotlinSyntaxKind::DoubleSemicolon, ";;".to_owned()),
            (KotlinSyntaxKind::DoubleArrow, "=>".to_owned()),
            (KotlinSyntaxKind::Arrow, "->".to_owned()),
        ]
    );
}

#[test]
fn lexes_regular_string_template_modes() {
    assert_eq!(
        token_texts(r#""hi $name ${call("{") + 1} done""#),
        vec![
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "hi ".to_owned()),
            (KotlinSyntaxKind::ShortTemplateEntryStart, "$".to_owned()),
            (KotlinSyntaxKind::Identifier, "name".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, " ".to_owned()),
            (KotlinSyntaxKind::LongTemplateEntryStart, "${".to_owned()),
            (KotlinSyntaxKind::Identifier, "call".to_owned()),
            (KotlinSyntaxKind::LParen, "(".to_owned()),
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "{".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RParen, ")".to_owned()),
            (KotlinSyntaxKind::Plus, "+".to_owned()),
            (KotlinSyntaxKind::IntegerLiteral, "1".to_owned()),
            (KotlinSyntaxKind::LongTemplateEntryEnd, "}".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, " done".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
        ]
    );
}

#[test]
fn lexes_raw_strings_and_keeps_escapes_as_content() {
    assert_eq!(
        token_texts("\"\"\"a\\nb $name\"\"\""),
        vec![
            (KotlinSyntaxKind::OpenQuote, "\"\"\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "a\\nb ".to_owned()),
            (KotlinSyntaxKind::ShortTemplateEntryStart, "$".to_owned()),
            (KotlinSyntaxKind::Identifier, "name".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"\"\"".to_owned()),
        ]
    );
}

#[test]
fn honors_multi_dollar_interpolation_prefix() {
    assert_eq!(
        token_texts(r#"$$"cost $value $$value $${value} $$$value""#),
        vec![
            (KotlinSyntaxKind::InterpolationPrefix, "$$".to_owned()),
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "cost ".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "$".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "value ".to_owned()),
            (KotlinSyntaxKind::ShortTemplateEntryStart, "$$".to_owned()),
            (KotlinSyntaxKind::Identifier, "value".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, " ".to_owned()),
            (KotlinSyntaxKind::LongTemplateEntryStart, "$${".to_owned()),
            (KotlinSyntaxKind::ValueKw, "value".to_owned()),
            (KotlinSyntaxKind::LongTemplateEntryEnd, "}".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, " ".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "$".to_owned()),
            (KotlinSyntaxKind::ShortTemplateEntryStart, "$$".to_owned()),
            (KotlinSyntaxKind::Identifier, "value".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
        ]
    );
}

#[test]
fn emits_escape_sequence_tokens_in_regular_strings() {
    assert_eq!(
        token_texts(r#""a\n\u0041""#),
        vec![
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "a".to_owned()),
            (KotlinSyntaxKind::EscapeSequence, "\\n".to_owned()),
            (KotlinSyntaxKind::EscapeSequence, "\\u0041".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
        ]
    );
}

#[test]
fn malformed_unicode_escape_consumes_only_escape_head() {
    assert_eq!(
        token_texts(r#""\u12x4""#),
        vec![
            (KotlinSyntaxKind::OpenQuote, "\"".to_owned()),
            (KotlinSyntaxKind::EscapeSequence, "\\u".to_owned()),
            (KotlinSyntaxKind::RegularStringPart, "12x4".to_owned()),
            (KotlinSyntaxKind::ClosingQuote, "\"".to_owned()),
        ]
    );
    assert_eq!(
        diagnostic_codes(r#""\u12x4""#),
        vec![KotlinLexDiagnosticCode::InvalidEscapeSequence.id()]
    );
}

#[test]
fn diagnoses_dangling_newline_in_regular_string() {
    assert_eq!(
        real_tokens("\"a\nb"),
        vec![
            KotlinSyntaxKind::OpenQuote,
            KotlinSyntaxKind::RegularStringPart,
            KotlinSyntaxKind::DanglingNewline,
            KotlinSyntaxKind::Identifier,
        ]
    );
    assert_eq!(
        diagnostic_codes("\"a\nb"),
        vec![KotlinLexDiagnosticCode::UnterminatedStringLiteral.id()]
    );

    let b = token_by_text_start("\"a\nb", 3);
    assert_eq!(b.kind, KotlinSyntaxKind::Identifier);
    assert_eq!(
        b.leading
            .iter()
            .map(|trivia| trivia.kind)
            .collect::<Vec<_>>(),
        vec![TriviaKind::Newline]
    );
    let dangling = token_by_text_start("\"a\nb", 2);
    assert_eq!(dangling.kind, KotlinSyntaxKind::DanglingNewline);
    assert_eq!(dangling.range.len(), TextSize::new(0));
}

#[test]
fn diagnoses_unterminated_raw_string_at_eof() {
    assert_eq!(
        diagnostic_codes("\"\"\"raw"),
        vec![KotlinLexDiagnosticCode::UnterminatedRawStringLiteral.id()]
    );
}

#[test]
fn finish_preserves_string_mode_when_draining_remaining_source() {
    let source = "\"a\nb";
    assert_eq!(
        diagnostic_codes_after_consuming_tokens(source, 2),
        diagnostic_codes(source)
    );
}

#[test]
fn diagnoses_unterminated_raw_string_inside_long_template_at_eof() {
    assert_eq!(
        diagnostic_codes("\"\"\"${value"),
        vec![KotlinLexDiagnosticCode::UnterminatedRawStringLiteral.id()]
    );
}
