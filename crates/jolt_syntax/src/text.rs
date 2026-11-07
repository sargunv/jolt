//! Text sizes, ranges, and line/column positions.

use std::fmt;
use std::ops::{Add, Sub};

/// A UTF-8 byte offset or length. Sources are smaller than 4 GiB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSize(u32);

impl TextSize {
    /// Creates a text size from a byte count.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not fit in `u32`.
    #[must_use]
    pub fn new(value: usize) -> Self {
        Self(u32::try_from(value).expect("source offsets fit in u32"))
    }

    /// Returns the byte count.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0 as usize
    }

    /// Returns the byte count as stored.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for TextSize {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Add for TextSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for TextSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl fmt::Display for TextSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A half-open byte range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    /// Creates a range.
    ///
    /// # Panics
    ///
    /// Panics when `start > end`.
    #[must_use]
    pub fn new(start: TextSize, end: TextSize) -> Self {
        assert!(start <= end, "text range start must not exceed its end");
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

    #[must_use]
    pub const fn start(self) -> TextSize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> TextSize {
        self.end
    }

    #[must_use]
    pub fn len(self) -> TextSize {
        self.end - self.start
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A zero-based line and byte column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// Converts byte offsets to line and column positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<TextSize>,
}

impl LineIndex {
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![TextSize::default()];
        line_starts.extend(
            source
                .bytes()
                .enumerate()
                .filter(|&(_, byte)| byte == b'\n')
                .map(|(offset, _)| TextSize::new(offset + 1)),
        );
        Self { line_starts }
    }

    /// # Panics
    ///
    /// Panics when the line count exceeds `u32`.
    #[must_use]
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        LineCol {
            line: u32::try_from(line).expect("line count fits in u32"),
            col: (offset - self.line_starts[line]).raw(),
        }
    }
}
