use core::{
    fmt::{Debug, Display},
    ops::{Add, Sub},
};

use super::fileunit::FileUnit;
use alloc::vec::Vec;

/// Store a fast resolver from raw bytes offset to (line,col)
///
/// This store the offset in bytes of the end of every lines (including the trailing newline)
pub struct LinesMap {
    max_ofs: usize,
    lines: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Line(pub u32);

impl Sub<usize> for Line {
    type Output = Line;

    fn sub(self, rhs: usize) -> Self::Output {
        if self.0 as usize > rhs {
            Line(self.0 - rhs as u32)
        } else {
            Line(0)
        }
    }
}

impl Add<usize> for Line {
    type Output = Line;

    fn add(self, rhs: usize) -> Self::Output {
        Line(self.0 + rhs as u32)
    }
}

/// Iterator from start line to end line, including the last line
pub struct LineIteratorInclusive(Line, Line);

impl LineIteratorInclusive {
    pub fn new(span: core::ops::RangeInclusive<Line>) -> Self {
        Self(*span.start(), *span.end())
    }
}

impl Iterator for LineIteratorInclusive {
    type Item = Line;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 > self.1 {
            None
        } else {
            let l = self.0;
            self.0 = Line(self.0.0 + 1);
            Some(l)
        }
    }
}

pub type Column = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    line: Line,
    col: Column,
}

impl LineCol {
    pub fn new(line: Line, col: Column) -> Self {
        Self { line, col }
    }

    pub fn line(&self) -> Line {
        self.line
    }

    pub fn col(&self) -> Column {
        self.col
    }

    pub fn line_col(&self) -> (Line, Column) {
        (self.line, self.col)
    }
}

impl Debug for Line {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0 + 1)
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0 + 1)
    }
}

impl Debug for LineCol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
impl Display for LineCol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

impl LinesMap {
    pub fn last_line(&self) -> Line {
        Line(self.lines.len() as u32)
    }

    pub fn new(content: &str) -> Self {
        let mut line_indexes = Vec::new();
        let mut pos = 0;
        let content = content.as_bytes();
        let max_ofs = content.len();

        for line_content in content.split_inclusive(|c| *c == b'\n') {
            pos += line_content.len();
            line_indexes.push(pos)
        }
        if content[max_ofs - 1] == b'\n' {
            line_indexes.push(pos)
        }

        assert_eq!(pos, max_ofs);

        Self {
            max_ofs,
            lines: line_indexes,
        }
    }

    pub fn resolve(&self, offset: usize) -> Option<LineCol> {
        if offset >= self.max_ofs {
            return None;
        }
        if offset == 0 {
            return Some(LineCol {
                line: Line(0),
                col: 0,
            });
        }
        match self.lines.binary_search(&offset) {
            Ok(found_index) => Some(LineCol {
                line: Line((found_index + 1) as u32),
                col: 0,
            }),
            Err(not_found_above) => {
                if not_found_above == 0 {
                    Some(LineCol {
                        line: Line(0),
                        col: offset as u32,
                    })
                } else {
                    let prev_line_end = self.lines[not_found_above - 1];
                    let col = offset - prev_line_end;
                    Some(LineCol {
                        line: Line(not_found_above as u32),
                        col: col as u32,
                    })
                }
            }
        }
    }

    pub fn line_start(&self, line: Line) -> usize {
        if (line.0 as usize) >= self.lines.len() {
            panic!("line {} is out of bound : {} lines", line, self.lines.len())
        }
        if line.0 == 0 {
            0
        } else {
            self.lines[(line.0 - 1) as usize]
        }
    }

    /// Return the offset of the end of the specified line
    pub fn line_end(&self, line: Line) -> usize {
        if line.0 > self.last_line().0 {
            panic!("line {} is out of bound : {} lines", line, self.lines.len())
        }
        self.lines[line.0 as usize]
    }

    pub fn resolve_span(&self, span: &core::ops::Range<usize>) -> Option<(LineCol, LineCol)> {
        let Some(start) = self.resolve(span.start) else {
            return None;
        };
        let Some(end) = self.resolve(span.end) else {
            return None;
        };
        Some((start, end))
    }

    pub fn get_line<'a>(&'a self, file_unit: &'a FileUnit, line: Line) -> &'a str {
        if line > self.last_line() {
            panic!("line {} is out of bound : {} lines", line, self.lines.len())
        }
        let start = if line.0 == 0 {
            0
        } else {
            self.lines[(line.0 - 1) as usize]
        };
        let end = self.lines[line.0 as usize];
        let line_text = &file_unit.content.as_bytes()[start..end];
        core::str::from_utf8(line_text).expect("valid utf8 get-line")
    }

    pub fn get_line_trim<'a>(&'a self, file_unit: &'a FileUnit, line: Line) -> &'a str {
        let x = self.get_line(file_unit, line);
        x.trim_end()
    }

    pub fn lines_iterator(&self, start: Line, end: Line) -> impl Iterator<Item = Line> {
        let end = core::cmp::min(end, self.last_line());
        let start = core::cmp::min(start, end);
        LineIteratorInclusive::new(start..=end)
    }
}
