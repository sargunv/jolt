//! The character classes of `java.lang.Character` that javac's doc comment
//! reader uses. javac reads UTF-16 `char`s, so a character outside the Basic
//! Multilingual Plane (a surrogate pair) is in none of them.

use unicode_general_category::{GeneralCategory as G, get_general_category};

/// `Character.isWhitespace`: a space, line or paragraph separator other than
/// a no-break space (U+00A0, U+2007, U+202F), or a control character from
/// `\t` to `\r` or from U+001C to U+001F.
pub(crate) fn is_whitespace(ch: char) -> bool {
    match ch {
        '\t'..='\r' | '\u{1c}'..='\u{1f}' | ' ' => true,
        '\u{a0}' | '\u{2007}' | '\u{202f}' => false,
        _ if ch.is_ascii() => false,
        _ => matches!(
            get_general_category(ch),
            G::SpaceSeparator | G::LineSeparator | G::ParagraphSeparator
        ),
    }
}

fn is_letter(category: G) -> bool {
    matches!(
        category,
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
    )
}

/// `Character.isIdentifierIgnorable`.
fn is_ignorable(ch: char, category: G) -> bool {
    matches!(ch, '\0'..='\u{8}' | '\u{e}'..='\u{1b}' | '\u{7f}'..='\u{9f}') || category == G::Format
}

/// `Character.isUnicodeIdentifierStart` (UAX #31 `ID_Start`).
pub(crate) fn is_unicode_identifier_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphabetic();
    }
    if u32::from(ch) > 0xffff {
        return false;
    }
    let category = get_general_category(ch);
    is_letter(category)
        || category == G::LetterNumber
        // Other_ID_Start.
        || matches!(ch, '\u{1885}' | '\u{1886}' | '\u{2118}' | '\u{212e}' | '\u{309b}' | '\u{309c}')
}

/// `Character.isUnicodeIdentifierPart` (UAX #31 `ID_Continue`, and the
/// ignorable characters).
pub(crate) fn is_unicode_identifier_part(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphanumeric() || ch == '_' || is_ignorable(ch, G::Control);
    }
    if u32::from(ch) > 0xffff {
        return false;
    }
    let category = get_general_category(ch);
    is_unicode_identifier_start(ch)
        || matches!(
            category,
            G::NonspacingMark | G::SpacingMark | G::DecimalNumber | G::ConnectorPunctuation
        )
        || is_ignorable(ch, category)
        // Other_ID_Continue.
        || matches!(ch, '\u{b7}' | '\u{387}' | '\u{1369}'..='\u{1371}' | '\u{19da}' | '\u{200c}' | '\u{200d}' | '\u{30fb}' | '\u{ff65}')
}

/// `Character.isJavaIdentifierStart`.
pub(crate) fn is_java_identifier_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphabetic() || ch == '$' || ch == '_';
    }
    if u32::from(ch) > 0xffff {
        return false;
    }
    let category = get_general_category(ch);
    is_letter(category)
        || matches!(
            category,
            G::LetterNumber | G::CurrencySymbol | G::ConnectorPunctuation
        )
}

/// `Character.isJavaIdentifierPart`.
pub(crate) fn is_java_identifier_part(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphanumeric()
            || ch == '$'
            || ch == '_'
            || is_ignorable(ch, G::Control);
    }
    if u32::from(ch) > 0xffff {
        return false;
    }
    let category = get_general_category(ch);
    is_java_identifier_start(ch)
        || matches!(
            category,
            G::DecimalNumber | G::SpacingMark | G::NonspacingMark
        )
        || is_ignorable(ch, category)
}
