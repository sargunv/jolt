use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct TextWidth(u32);

impl TextWidth {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(width: u32) -> Self {
        Self(width)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u16> for TextWidth {
    fn from(value: u16) -> Self {
        Self(u32::from(value))
    }
}

impl From<u32> for TextWidth {
    fn from(value: u32) -> Self {
        Self(value)
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

pub(crate) fn literal_line_widths(text: &str) -> Box<[TextWidth]> {
    let mut widths = Vec::new();
    let mut width = TextWidth::ZERO;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                widths.push(width);
                width = TextWidth::ZERO;
            }
            '\n' => {
                widths.push(width);
                width = TextWidth::ZERO;
            }
            _ => width = add_width(width, char_width(ch)),
        }
    }
    widths.push(width);
    widths.into_boxed_slice()
}

pub(crate) fn literal_line_count(text: &str) -> usize {
    let mut count = 1;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                count += 1;
            }
            '\n' => count += 1,
            _ => {}
        }
    }
    count
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

pub(crate) fn has_line_terminator(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

pub(crate) fn push_repeated(output: &mut String, ch: char, count: u32) {
    for _ in 0..count {
        output.push(ch);
    }
}
