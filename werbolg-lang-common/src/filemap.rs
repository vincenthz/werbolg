use core::fmt::{Debug, Display};

use alloc::vec::Vec;

/// Store a fast resolver from raw bytes offset to (line,col) where line starts at 1
///
/// Also line 1 starts at 0 and is not stored, so effectively this starts with the index
/// of line 2.
///
/// self.lines[0] effectively contains the bytes offset of the beginning of line 2
pub struct LinesMap {
    max_ofs: usize,
    lines: Vec<usize>,
}

pub type Line = u32;
pub type Column = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    line: Line,
    col: Column,
}

impl LineCol {
    pub fn new(line: Line, col: Column) -> Self {
        assert!(line > 0);
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
        self.lines.len() as u32 + 1
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

        assert_eq!(pos, max_ofs);

        Self {
            max_ofs,
            lines: line_indexes,
        }
    }

    pub fn resolve(&self, offset: usize) -> Option<LineCol> {
        if offset > self.max_ofs {
            return None;
        }
        match self.lines.binary_search(&offset) {
            Ok(found) => Some(LineCol {
                line: found as u32 + 2,
                col: 0,
            }),
            Err(not_found_above) => {
                if not_found_above == 0 {
                    Some(LineCol {
                        line: 1,
                        col: offset as u32,
                    })
                } else {
                    let prev_line_start = self.lines[not_found_above - 1];
                    let col = offset - prev_line_start;
                    Some(LineCol {
                        line: (not_found_above + 1) as u32,
                        col: col as u32,
                    })
                }
            }
        }
    }

    pub fn line_start(&self, line: Line) -> usize {
        if line == 0 || ((line - 1) as usize) >= self.lines.len() {
            panic!("line {} is out of bound : {}", line, self.lines.len())
        }
        if line == 1 {
            0
        } else {
            self.lines[(line - 2) as usize]
        }
    }

    pub fn line_end(&self, line: Line) -> usize {
        assert!(line > 0);
        if self.last_line() < line - 1 {
            self.lines[(self.last_line() - 2) as usize]
        } else {
            self.lines[(line - 2) as usize]
        }
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
}
