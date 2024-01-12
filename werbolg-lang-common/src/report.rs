use crate::filemap::Line;

use super::filemap::LinesMap;
use super::fileunit::FileUnit;
use super::span::Span;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp;
use core::fmt::Write;

const BOXING: [char; 11] = ['╭', '╮', '╯', '╰', '─', '│', '├', '┤', '┬', '┴', '┼'];
const TL: usize = 0;
#[allow(unused)]
const TR: usize = 1;
const BR: usize = 2;
const BL: usize = 3;
const H: usize = 4;
const V: usize = 5;
#[allow(unused)]
const HR: usize = 6;
#[allow(unused)]
const HL: usize = 7;
const TD: usize = 8;
#[allow(unused)]
const BU: usize = 9;
#[allow(unused)]
const CROSS: usize = 10;

pub struct Report {
    code: Option<String>,
    kind: ReportKind,
    header: String,
    context: Option<Span>,
    context_before: Option<usize>,
    context_after: Option<usize>,
    notes: Vec<String>,
    highlight: Option<(Span, String)>,
}

pub enum ReportKind {
    Error,
    Warning,
    Info,
}

impl Report {
    pub fn new(kind: ReportKind, header: String) -> Self {
        Self {
            header,
            kind,
            code: None,
            context: None,
            context_before: None,
            context_after: None,
            notes: Vec::new(),
            highlight: None,
        }
    }

    pub fn code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn context(mut self, context: Span) -> Self {
        self.context = Some(context);
        self
    }

    pub fn highlight(mut self, highlight: Span, message: String) -> Self {
        self.highlight = Some((highlight, message));
        self
    }

    pub fn lines_before(mut self, context_before: usize) -> Self {
        self.context_before = Some(context_before);
        self
    }

    pub fn lines_after(mut self, context_after: usize) -> Self {
        self.context_after = Some(context_after);
        self
    }

    pub fn note(mut self, s: String) -> Self {
        self.notes.push(s);
        self
    }

    fn context_lines<'a>(
        &self,
        file_map: &LinesMap,
    ) -> Result<impl Iterator<Item = Line>, Option<String>> {
        let Some(highlight) = &self.highlight else {
            return Err(None);
        };
        let Some((highlight_start, highlight_end)) = file_map.resolve_span(&highlight.0) else {
            return Err(Some(format!("cannot resolve highlight span")));
        };

        let (_context, context_start_line, context_end_line) = if let Some(context) = &self.context
        {
            let Some((context_start, context_end)) = file_map.resolve_span(&context) else {
                //writeln!(writer, "internal error resolving span {:?}", context)?;
                return Err(None);
            };
            (context, context_start.line(), context_end.line())
        } else {
            let context_start = if let Some(before) = self.context_before {
                highlight_start.line() - before
            } else {
                highlight_start.line()
            };

            let context_end = if let Some(after) = self.context_after {
                let r = highlight_end.line() + after;
                cmp::min(r, file_map.last_line())
            } else {
                highlight_end.line()
            };

            (&highlight.0, context_start, context_end)
        };

        Ok(file_map.lines_iterator(context_start_line, context_end_line))
    }

    pub fn write<W: Write>(
        self,
        file_unit: &FileUnit,
        file_map: &LinesMap,
        writer: &mut W,
    ) -> Result<(), core::fmt::Error> {
        // write the first line
        let code_format = if let Some(code) = &self.code {
            format!("[{}] ", code)
        } else {
            String::new()
        };
        let hd = match self.kind {
            ReportKind::Error => "Error",
            ReportKind::Warning => "Warning",
            ReportKind::Info => "Info",
        };
        writeln!(writer, "{}{}: {}", code_format, hd, self.header)?;

        let context_lines = match self.context_lines(file_map) {
            Ok(o) => o,
            Err(None) => return Ok(()),
            Err(Some(_)) => return Ok(()),
        };

        let Some(highlight) = self.highlight else {
            unreachable!();
        };
        let (start_highlight, end_highlight) = file_map.resolve_span(&highlight.0).unwrap();
        let multiline = !(start_highlight.line() == end_highlight.line());

        writeln!(
            writer,
            "{} {}{}[{}]",
            line_format(None),
            BOXING[TL],
            BOXING[H],
            file_unit.filename
        )?;

        for line in context_lines {
            let line_text = file_map.get_line_trim(file_unit, line);
            writeln!(
                writer,
                "{} {} {}",
                line_format(Some(line)),
                BOXING[V],
                line_text
            )?;
            if !multiline {
                if start_highlight.line() == line {
                    let col_start = start_highlight.col();
                    let col_end = end_highlight.col();
                    let under = col_end - col_start;

                    let s = string_repeat(col_start as usize, ' ');
                    writeln!(
                        writer,
                        "{} {} {}{}{}",
                        line_format(None),
                        BOXING[V],
                        &s,
                        underline(under as usize),
                        string_repeat(col_end as usize, ' '),
                    )?;
                    writeln!(
                        writer,
                        "{} {} {}{}{} {}",
                        line_format(None),
                        BOXING[V],
                        &s,
                        underline2(under as usize),
                        string_repeat(2, BOXING[H]),
                        highlight.1,
                    )?;
                }
            }
        }

        if !self.notes.is_empty() {
            writeln!(writer, "{} {}", line_format(None), BOXING[V])?;
            writeln!(writer, "{} {} {}", line_format(None), BOXING[V], "Notes")?;
            for note in self.notes.iter() {
                writeln!(writer, "{} {}", line_format(None), BOXING[V])?;
                writeln!(writer, "{} {}   {}", line_format(None), BOXING[V], note)?;
            }
        }

        writeln!(
            writer,
            "{}{}{}",
            string_repeat(LINE_SZ, BOXING[H]),
            BOXING[H],
            BOXING[BR],
        )?;

        Ok(())
    }
}

const LINE_SZ: usize = 4;

fn line_format(r: Option<Line>) -> String {
    pad_left(LINE_SZ, r.map(|x| x.0 + 1))
}

fn pad_left(sz: usize, r: Option<u32>) -> String {
    let mut out = String::new();

    let x = match r {
        None => String::new(),
        Some(r) => format!("{}", r),
    };
    let pad_chars = if x.len() < sz { sz - x.len() } else { 0 };

    for _ in 0..pad_chars {
        out.push(' ');
    }
    out.push_str(&x);
    out
}

fn string_repeat(sz: usize, c: char) -> String {
    let mut out = String::new();
    for _ in 0..sz {
        out.push(c);
    }
    out
}

fn underline(sz: usize) -> String {
    assert!(sz > 0);

    let mut out = String::new();
    let middle = sz / 2;
    for i in 0..sz {
        if i == middle {
            out.push(BOXING[TD]);
        } else {
            out.push(BOXING[H]);
        }
    }
    out
}

fn underline2(sz: usize) -> String {
    assert!(sz > 0);

    let mut out = String::new();
    let middle = sz / 2;
    for i in 0..=middle {
        if i == middle {
            out.push(BOXING[BL]);
        } else {
            out.push(' ');
        }
    }
    out
}
