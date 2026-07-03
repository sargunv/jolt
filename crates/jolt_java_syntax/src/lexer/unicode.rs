use jolt_text::{TextRange, TextSize};

use super::{JavaLexDiagnosticCode, LexerDiagnostic, lexer_diagnostic};

// Java processes Unicode escapes before tokenization, everywhere in the source.
// For example, `\u000a` becomes an actual line terminator before string or
// comment scanning sees it; it is not the same as a string escape like `\n`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InputChar {
    pub(super) ch: char,
    pub(super) range: TextRange,
    from_escape: bool,
}

pub(super) fn translate_unicode_escapes(source: &str) -> (Vec<InputChar>, Vec<LexerDiagnostic>) {
    if !contains_unicode_escape_marker(source) {
        return (raw_input_chars(source), Vec::new());
    }

    let raw: Vec<(usize, char)> = source.char_indices().collect();
    let mut chars = Vec::with_capacity(raw.len());
    let mut diagnostics = Vec::new();
    let mut index = 0usize;

    while index < raw.len() {
        let (start, ch) = raw[index];

        if ch == '\\'
            && is_unicode_escape_eligible(&chars)
            && raw.get(index + 1).is_some_and(|(_, ch)| *ch == 'u')
        {
            if let Some(first_escape) = parse_unicode_escape(&raw, index, source) {
                if is_high_surrogate(first_escape.value)
                    && raw
                        .get(first_escape.end_index)
                        .is_some_and(|(_, ch)| *ch == '\\')
                    && raw
                        .get(first_escape.end_index + 1)
                        .is_some_and(|(_, ch)| *ch == 'u')
                    && let Some(second_escape) =
                        parse_unicode_escape(&raw, first_escape.end_index, source)
                    && is_low_surrogate(second_escape.value)
                {
                    let high = first_escape.value - 0xD800;
                    let low = second_escape.value - 0xDC00;
                    let scalar = 0x10000 + ((high << 10) | low);
                    chars.push(InputChar {
                        ch: char::from_u32(scalar).expect("valid surrogate pair scalar value"),
                        range: TextRange::new(
                            TextSize::new(start),
                            TextSize::new(second_escape.end_offset),
                        ),
                        from_escape: true,
                    });
                    index = second_escape.end_index;
                    continue;
                }

                chars.push(InputChar {
                    ch: char::from_u32(first_escape.value).unwrap_or(char::REPLACEMENT_CHARACTER),
                    range: TextRange::new(
                        TextSize::new(start),
                        TextSize::new(first_escape.end_offset),
                    ),
                    from_escape: true,
                });
                index = first_escape.end_index;
                continue;
            }

            let end = malformed_unicode_escape_end(&raw, index, source);
            diagnostics.push(lexer_diagnostic(
                JavaLexDiagnosticCode::MalformedUnicodeEscape,
                TextRange::new(TextSize::new(start), TextSize::new(end)),
            ));
        }

        let end = raw
            .get(index + 1)
            .map_or(source.len(), |(offset, _)| *offset);
        chars.push(InputChar {
            ch,
            range: TextRange::new(TextSize::new(start), TextSize::new(end)),
            from_escape: false,
        });
        index += 1;
    }

    (chars, diagnostics)
}

fn contains_unicode_escape_marker(source: &str) -> bool {
    source.as_bytes().windows(2).any(|pair| pair == b"\\u")
}

fn raw_input_chars(source: &str) -> Vec<InputChar> {
    let mut chars = Vec::with_capacity(source.len());
    for (start, ch) in source.char_indices() {
        let end = start + ch.len_utf8();
        chars.push(InputChar {
            ch,
            range: TextRange::new(TextSize::new(start), TextSize::new(end)),
            from_escape: false,
        });
    }
    chars
}

#[derive(Clone, Copy, Debug)]
struct UnicodeEscape {
    value: u32,
    end_index: usize,
    end_offset: usize,
}

fn parse_unicode_escape(
    raw: &[(usize, char)],
    start_index: usize,
    source: &str,
) -> Option<UnicodeEscape> {
    debug_assert_eq!(raw[start_index].1, '\\');
    debug_assert_eq!(raw[start_index + 1].1, 'u');

    let mut marker_end = start_index + 1;
    while raw.get(marker_end).is_some_and(|(_, ch)| *ch == 'u') {
        marker_end += 1;
    }

    if marker_end + 4 > raw.len()
        || !raw[marker_end..marker_end + 4]
            .iter()
            .all(|(_, ch)| ch.is_ascii_hexdigit())
    {
        return None;
    }

    let value = raw[marker_end..marker_end + 4]
        .iter()
        .fold(0u32, |value, (_, ch)| {
            (value << 4) + ch.to_digit(16).expect("hex digit checked")
        });
    let end_index = marker_end + 4;
    let end_offset = raw
        .get(end_index)
        .map_or(source.len(), |(offset, _)| *offset);

    Some(UnicodeEscape {
        value,
        end_index,
        end_offset,
    })
}

fn malformed_unicode_escape_end(raw: &[(usize, char)], start_index: usize, source: &str) -> usize {
    let mut marker_end = start_index + 1;
    while raw.get(marker_end).is_some_and(|(_, ch)| *ch == 'u') {
        marker_end += 1;
    }
    raw.get(marker_end)
        .map_or(source.len(), |(offset, _)| *offset)
}

fn is_high_surrogate(value: u32) -> bool {
    (0xD800..=0xDBFF).contains(&value)
}

fn is_low_surrogate(value: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&value)
}

fn is_unicode_escape_eligible(chars: &[InputChar]) -> bool {
    match chars.last() {
        None => true,
        Some(last) if last.from_escape => true,
        Some(_) => {
            chars
                .iter()
                .rev()
                .take_while(|input| input.ch == '\\')
                .count()
                % 2
                == 0
        }
    }
}
