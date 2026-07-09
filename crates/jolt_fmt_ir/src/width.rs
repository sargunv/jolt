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
    text.chars()
        .map(char_width)
        .fold(TextWidth::ZERO, add_width)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LiteralTextMetrics {
    pub(crate) final_width: TextWidth,
    pub(crate) line_count: usize,
}

pub(crate) fn literal_text_metrics(text: &str) -> LiteralTextMetrics {
    let mut line_count = 1;
    let mut width = TextWidth::ZERO;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                line_count += 1;
                width = TextWidth::ZERO;
            }
            '\n' => {
                line_count += 1;
                width = TextWidth::ZERO;
            }
            _ => width = add_width(width, char_width(ch)),
        }
    }
    LiteralTextMetrics {
        final_width: width,
        line_count,
    }
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
