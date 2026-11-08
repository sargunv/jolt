//! Unicode escape translation. Java processes Unicode escapes before
//! tokenization, everywhere in the source. For example, `\u000a` becomes an
//! actual line terminator before string or comment scanning sees it; it is
//! not the same as a string escape like `\n`.

use std::{borrow::Cow, ops::Range};

use jolt_syntax::{Diagnostic, TextRange, TextSize};

use super::JavaLexDiagnosticCode;

/// The source with its Unicode escapes translated, and the map back to raw
/// source offsets.
pub(crate) struct NormalizedJavaSource<'source> {
    source: Cow<'source, str>,
    diagnostics: Vec<Diagnostic>,
    replacements: Vec<UnicodeReplacement>,
}

impl NormalizedJavaSource<'_> {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn has_replacements(&self) -> bool {
        !self.replacements.is_empty()
    }

    pub(crate) fn raw_range(&self, range: TextRange) -> TextRange {
        TextRange::new(
            TextSize::new(self.raw_offset(range.start().get())),
            TextSize::new(self.raw_offset(range.end().get())),
        )
    }

    pub(crate) fn remap_diagnostics(&self, diagnostics: &mut [Diagnostic]) {
        for diagnostic in diagnostics {
            diagnostic.range = self.raw_range(diagnostic.range);
        }
    }

    pub(crate) fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    fn raw_offset(&self, normalized_offset: usize) -> usize {
        let completed = self
            .replacements
            .partition_point(|replacement| replacement.normalized.end <= normalized_offset);
        let added_raw_bytes = completed
            .checked_sub(1)
            .map_or(0, |index| self.replacements[index].added_raw_bytes_after);
        if let Some(replacement) = self.replacements.get(completed)
            && replacement.normalized.start < normalized_offset
        {
            // Lexer, trivia, and parser diagnostic boundaries are UTF-8 scalar
            // boundaries, so they cannot split a normalized replacement.
            debug_assert!(false, "source boundary split a normalized Unicode escape");
            return replacement.raw.start;
        }
        normalized_offset + added_raw_bytes
    }
}

#[derive(Clone, Debug)]
struct UnicodeReplacement {
    normalized: Range<usize>,
    raw: Range<usize>,
    added_raw_bytes_after: usize,
}

pub(crate) fn normalize_unicode_escapes(source: &str) -> NormalizedJavaSource<'_> {
    // Unicode escapes always begin with `\u`. Most source files do not
    // contain it at all, so avoid decoding every UTF-8 scalar just to discover
    // that normalization has no work to do.
    if !source.contains("\\u") {
        return NormalizedJavaSource {
            source: Cow::Borrowed(source),
            diagnostics: Vec::new(),
            replacements: Vec::new(),
        };
    }

    let mut normalized = None;
    let mut diagnostics = Vec::new();
    let mut replacements = Vec::new();
    let mut eligibility = UnicodeEscapeEligibility::default();
    let mut offset = 0usize;

    while let Some((ch, end)) = char_at(source, offset) {
        let start = offset;

        if has_unicode_escape_marker_at(source, start) && eligibility.is_eligible() {
            if let Some(first_escape) = parse_unicode_escape(source, start) {
                let (ch, raw_end) = if is_high_surrogate(first_escape.value)
                    && has_unicode_escape_marker_at(source, first_escape.end_offset)
                    && let Some(second_escape) =
                        parse_unicode_escape(source, first_escape.end_offset)
                    && is_low_surrogate(second_escape.value)
                {
                    let high = first_escape.value - 0xD800;
                    let low = second_escape.value - 0xDC00;
                    let scalar = 0x10000 + ((high << 10) | low);
                    (
                        char::from_u32(scalar).expect("valid surrogate pair scalar value"),
                        second_escape.end_offset,
                    )
                } else {
                    (
                        char::from_u32(first_escape.value).unwrap_or(char::REPLACEMENT_CHARACTER),
                        first_escape.end_offset,
                    )
                };
                let normalized = normalized_source(source, &mut normalized, start);
                let normalized_start = normalized.len();
                push_char(normalized, &mut eligibility, ch, true);
                record_replacement(
                    &mut replacements,
                    normalized_start..normalized.len(),
                    start..raw_end,
                );
                offset = raw_end;
                continue;
            }

            let end = malformed_unicode_escape_end(source, start);
            let normalized_start = normalized.as_ref().map_or(start, String::len);
            diagnostics.push(
                JavaLexDiagnosticCode::MalformedUnicodeEscape.at(TextRange::new(
                    TextSize::new(normalized_start),
                    TextSize::new(normalized_start + end - start),
                )),
            );
        }

        if let Some(normalized) = normalized.as_mut() {
            push_char(normalized, &mut eligibility, ch, false);
        } else {
            eligibility.advance(ch, false);
        }
        offset = end;
    }

    NormalizedJavaSource {
        source: normalized.map_or(Cow::Borrowed(source), Cow::Owned),
        diagnostics,
        replacements,
    }
}

fn record_replacement(
    replacements: &mut Vec<UnicodeReplacement>,
    normalized: Range<usize>,
    raw: Range<usize>,
) {
    let added_raw_bytes_after = replacements
        .last()
        .map_or(0, |replacement| replacement.added_raw_bytes_after)
        + raw.len()
        - normalized.len();
    replacements.push(UnicodeReplacement {
        normalized,
        raw,
        added_raw_bytes_after,
    });
}

fn normalized_source<'a>(
    source: &str,
    normalized: &'a mut Option<String>,
    changed_at: usize,
) -> &'a mut String {
    normalized.get_or_insert_with(|| {
        let mut text = String::with_capacity(source.len());
        text.push_str(&source[..changed_at]);
        text
    })
}

#[derive(Default)]
struct UnicodeEscapeEligibility {
    previous_from_escape: bool,
    trailing_backslashes: usize,
}

impl UnicodeEscapeEligibility {
    fn is_eligible(&self) -> bool {
        self.previous_from_escape || self.trailing_backslashes.is_multiple_of(2)
    }

    fn advance(&mut self, ch: char, from_escape: bool) {
        self.previous_from_escape = from_escape;
        if ch == '\\' {
            self.trailing_backslashes += 1;
        } else {
            self.trailing_backslashes = 0;
        }
    }
}

fn push_char(
    normalized: &mut String,
    eligibility: &mut UnicodeEscapeEligibility,
    ch: char,
    from_escape: bool,
) {
    eligibility.advance(ch, from_escape);
    normalized.push(ch);
}

#[derive(Clone, Copy, Debug)]
struct UnicodeEscape {
    value: u32,
    end_offset: usize,
}

fn char_at(source: &str, offset: usize) -> Option<(char, usize)> {
    let ch = source.get(offset..)?.chars().next()?;
    Some((ch, offset + ch.len_utf8()))
}

fn has_unicode_escape_marker_at(source: &str, offset: usize) -> bool {
    let bytes = source.as_bytes();
    bytes.get(offset).is_some_and(|byte| *byte == b'\\')
        && bytes.get(offset + 1).is_some_and(|byte| *byte == b'u')
}

fn parse_unicode_escape(source: &str, start_offset: usize) -> Option<UnicodeEscape> {
    debug_assert!(has_unicode_escape_marker_at(source, start_offset));

    let bytes = source.as_bytes();
    let marker_end = unicode_marker_end(bytes, start_offset);

    if marker_end + 4 > bytes.len()
        || !bytes[marker_end..marker_end + 4]
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return None;
    }

    let value = bytes[marker_end..marker_end + 4]
        .iter()
        .fold(0u32, |value, byte| (value << 4) + hex_digit(*byte));

    Some(UnicodeEscape {
        value,
        end_offset: marker_end + 4,
    })
}

fn hex_digit(byte: u8) -> u32 {
    match byte {
        b'0'..=b'9' => u32::from(byte - b'0'),
        b'a'..=b'f' => u32::from(byte - b'a' + 10),
        b'A'..=b'F' => u32::from(byte - b'A' + 10),
        _ => unreachable!("hex digit checked"),
    }
}

fn malformed_unicode_escape_end(source: &str, start_offset: usize) -> usize {
    unicode_marker_end(source.as_bytes(), start_offset)
}

fn unicode_marker_end(bytes: &[u8], start_offset: usize) -> usize {
    let mut marker_end = start_offset + 1;
    while bytes.get(marker_end).is_some_and(|byte| *byte == b'u') {
        marker_end += 1;
    }
    marker_end
}

fn is_high_surrogate(value: u32) -> bool {
    (0xD800..=0xDBFF).contains(&value)
}

fn is_low_surrogate(value: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&value)
}
