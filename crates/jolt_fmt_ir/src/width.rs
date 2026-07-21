#![allow(
    clippy::inline_always,
    reason = "release profiles show literal width measurement remains a hot out-of-line leaf"
)]

use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct TextWidth(u32);

impl TextWidth {
    pub(crate) const ZERO: Self = Self(0);

    #[must_use]
    pub(crate) const fn new(width: u32) -> Self {
        Self(width)
    }

    #[must_use]
    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

impl From<u16> for TextWidth {
    fn from(value: u16) -> Self {
        Self(u32::from(value))
    }
}

pub(crate) fn add_width(lhs: TextWidth, rhs: TextWidth) -> TextWidth {
    TextWidth::new(lhs.get().saturating_add(rhs.get()))
}

pub(crate) fn display_width(text: &str) -> TextWidth {
    if text.bytes().all(|byte| matches!(byte, b'\t' | b' '..=b'~')) {
        return TextWidth::new(text.len().try_into().expect("display width fits u32"));
    }

    text.chars()
        .map(char_width)
        .fold(TextWidth::ZERO, add_width)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LiteralTextMetrics {
    pub(crate) final_width: TextWidth,
    pub(crate) first_width: TextWidth,
    pub(crate) line_count: usize,
}

#[inline(always)]
pub(crate) fn literal_text_metrics(text: &str) -> LiteralTextMetrics {
    if let Some(metrics) = ascii_literal_text_metrics(text.as_bytes()) {
        return metrics;
    }

    let mut line_count = 1;
    let mut width = TextWidth::ZERO;
    let mut first_width = None;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                first_width.get_or_insert(width);
                line_count += 1;
                width = TextWidth::ZERO;
            }
            '\n' => {
                first_width.get_or_insert(width);
                line_count += 1;
                width = TextWidth::ZERO;
            }
            _ => width = add_width(width, char_width(ch)),
        }
    }
    LiteralTextMetrics {
        first_width: first_width.unwrap_or(width),
        final_width: width,
        line_count,
    }
}

#[inline(always)]
fn ascii_literal_text_metrics(text: &[u8]) -> Option<LiteralTextMetrics> {
    let mut line_count = 1;
    let mut width = 0_u32;
    let mut first_width = None;
    let mut index = 0;
    while index < text.len() {
        match text[index] {
            byte if !byte.is_ascii() => return None,
            b'\r' => {
                if text.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                first_width.get_or_insert(width);
                line_count += 1;
                width = 0;
            }
            b'\n' => {
                first_width.get_or_insert(width);
                line_count += 1;
                width = 0;
            }
            b'\t' | b' '..=b'~' => width = width.saturating_add(1),
            _ => {}
        }
        index += 1;
    }
    Some(LiteralTextMetrics {
        final_width: TextWidth::new(width),
        first_width: TextWidth::new(first_width.unwrap_or(width)),
        line_count,
    })
}

fn char_width(ch: char) -> TextWidth {
    if ch == '\t' {
        TextWidth::new(1)
    } else {
        TextWidth::new(
            ch.width()
                .unwrap_or(0)
                .try_into()
                .expect("char width fits u32"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{LiteralTextMetrics, TextWidth, display_width, literal_text_metrics};

    #[test]
    fn ascii_fast_paths_preserve_control_and_line_widths() {
        assert_eq!(display_width("\0\t ~\u{7f}"), TextWidth::new(3));
        assert_eq!(
            literal_text_metrics("\0a\r\nb\u{7f}"),
            LiteralTextMetrics {
                final_width: TextWidth::new(1),
                first_width: TextWidth::new(1),
                line_count: 2,
            }
        );
    }
}
