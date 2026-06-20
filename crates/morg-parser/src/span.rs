/// Source location of a parsed node.
///
/// `start` and `end` are byte offsets into the original source string.
/// `line` and `col` are 1-based and point to the opening character of the
/// node, suitable for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Inclusive byte offset of the first character.
    pub start: usize,
    /// Exclusive byte offset past the last character.
    pub end: usize,
    /// 1-based line number of `start`.
    pub line: u32,
    /// 1-based column number of `start`.
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }

    pub fn empty(line: u32, col: u32) -> Self {
        Self {
            start: 0,
            end: 0,
            line,
            col,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            col: if self.line <= other.line {
                self.col
            } else {
                other.col
            },
        }
    }
}
