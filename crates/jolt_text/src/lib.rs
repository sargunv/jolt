//! Source text primitives shared by Jolt parser and formatter crates.

use std::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

/// A UTF-8 byte offset or length in source text.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextSize(usize);

impl TextSize {
    /// Creates a text size from a raw byte count.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw byte count.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for TextSize {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<TextSize> for usize {
    fn from(value: TextSize) -> Self {
        value.0
    }
}

impl Add for TextSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("text size addition overflowed"),
        )
    }
}

impl AddAssign for TextSize {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for TextSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("text size subtraction underflowed"),
        )
    }
}

impl SubAssign for TextSize {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl fmt::Display for TextSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A half-open UTF-8 byte range in source text.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    /// Creates a half-open text range.
    ///
    /// # Panics
    ///
    /// Panics when `start` is greater than `end`.
    #[must_use]
    pub fn new(start: TextSize, end: TextSize) -> Self {
        assert!(start <= end, "text range start must be before end");
        Self { start, end }
    }

    /// Creates an empty range at `offset`.
    #[must_use]
    pub const fn empty(offset: TextSize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Returns the first byte offset in the range.
    #[must_use]
    pub const fn start(self) -> TextSize {
        self.start
    }

    /// Returns the first byte offset after the range.
    #[must_use]
    pub const fn end(self) -> TextSize {
        self.end
    }

    /// Returns the byte length of the range.
    #[must_use]
    pub fn len(self) -> TextSize {
        self.end - self.start
    }

    /// Returns true when the range contains no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.get() == self.end.get()
    }

    /// Returns true when `offset` is inside the half-open range.
    #[must_use]
    pub const fn contains(self, offset: TextSize) -> bool {
        self.start.get() <= offset.get() && offset.get() < self.end.get()
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A zero-based source position.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LineCol {
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based UTF-8 byte column within the line.
    pub column: TextSize,
}

/// A compact index for converting source byte offsets to line/column positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
    line_starts: Vec<TextSize>,
}

impl LineIndex {
    /// Builds a line index for `source`.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![TextSize::new(0)];

        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(TextSize::new(offset + 1));
            }
        }

        Self { line_starts }
    }

    /// Returns the number of lines represented by the index.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a source byte offset to a zero-based line and column.
    #[must_use]
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next_line) => next_line.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        let column = offset - line_start;

        LineCol { line, column }
    }
}
