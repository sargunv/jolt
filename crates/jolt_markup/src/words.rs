//! The words a wrapped line must not start with, and the lines whose line
//! breaks stay: shared by both flavours.

/// Whether `word` is a plain-text list marker: `-`, `*`, `+`, `•`, or a
/// number and `.` or `)`.
pub(crate) fn is_list_marker(word: &str) -> bool {
    match word {
        "-" | "*" | "+" | "•" => true,
        _ => word
            .strip_suffix(['.', ')'])
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())),
    }
}

/// Whether `word` would start new structure at the start of a line, so a
/// wrap must not put it there: a block tag (`@`) or a list marker, and in
/// Markdown also a word that could start a block that interrupts a
/// paragraph or ends it: an ATX heading (`#` to `######`), a block quote
/// (`>`), a fence, an HTML block (`<` and a letter, `/`, `!` or `?`), a
/// table row (`|`), or a thematic break or setext underline (a word made
/// only of `*`, `-`, `_`, `=` or `+`).
pub(crate) fn starts_structure(word: &str, markdown: bool) -> bool {
    if word.starts_with('@') || is_list_marker(word) {
        return true;
    }
    if !markdown {
        return false;
    }
    let bytes = word.as_bytes();
    let Some(&first) = bytes.first() else {
        return false;
    };
    match first {
        b'>' | b'|' => true,
        b'#' => bytes.len() <= 6 && bytes.iter().all(|&byte| byte == b'#'),
        fence @ (b'`' | b'~') => bytes.len() >= 3 && bytes[..3].iter().all(|&byte| byte == fence),
        b'<' => bytes
            .get(1)
            .is_some_and(|&next| next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?')),
        b'0'..=b'9' => {
            let digits = bytes
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            matches!(bytes.get(digits), Some(b'.' | b')'))
        }
        _ => bytes.iter().all(|byte| b"*-_=+".contains(byte)),
    }
}

/// Whether a Markdown line that starts with `word` ends javadoc's first
/// sentence there (`DocTreeMaker`'s `endPara`: a line that starts with
/// `-`, `+`, `*`, `#`, `=`, `_` or `>`).
pub(crate) fn ends_first_paragraph(word: &str) -> bool {
    word.starts_with(['-', '+', '*', '#', '=', '_', '>'])
}

/// Whether a line's text is a ruler: three or more punctuation characters
/// (`-------`, `|-----|`), with nothing else but spaces.
pub(crate) fn is_ruler(text: &str) -> bool {
    let mut count = 0;
    for byte in text.bytes() {
        match byte {
            b' ' | b'\t' => {}
            b'-' | b'=' | b'_' | b'*' | b'+' | b'~' | b'#' | b'|' | b'.' | b':' | b'/' | b'\\' => {
                count += 1;
            }
            _ => return false,
        }
    }
    count >= 3
}
