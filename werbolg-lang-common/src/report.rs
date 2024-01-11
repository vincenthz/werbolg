use super::filemap::LinesMap;
use super::fileunit::FileUnit;
use super::span::Span;

use alloc::format;
use alloc::string::String;
use core::fmt::Write;
use core::ops::Range;

const BOXING: [char; 11] = ['╭', '╮', '╯', '╰', '─', '│', '├', '┤', '┬', '┴', '┼'];
const TL: usize = 0;
#[allow(unused)]
const TR: usize = 1;
#[allow(unused)]
const BR: usize = 2;
#[allow(unused)]
const BL: usize = 3;
const H: usize = 4;
const V: usize = 5;
#[allow(unused)]
const HR: usize = 6;
#[allow(unused)]
const HL: usize = 7;
#[allow(unused)]
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
    highlight: Option<Span>,
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

    pub fn highlight(mut self, highlight: Span) -> Self {
        self.highlight = Some(highlight);
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

    pub fn write<W: Write>(
        self,
        file_unit: &FileUnit,
        file_map: &LinesMap,
        writer: &mut W,
    ) -> Result<(), core::fmt::Error> {
        // write the first line
        let code_format = if let Some(code) = self.code {
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

        let Some(highlight) = self.highlight else {
            return Ok(());
        };
        let Some((highlight_start, highlight_end)) = file_map.resolve_span(&highlight) else {
            writeln!(writer, "internal error resolving span {:?}", highlight)?;
            return Ok(());
        };

        let (_context, context_start_line, context_end_line) = if let Some(context) = self.context {
            let Some((context_start, context_end)) = file_map.resolve_span(&context) else {
                writeln!(writer, "internal error resolving span {:?}", context)?;
                return Ok(());
            };
            (context, context_start.line(), context_end.line())
        } else {
            let context_start = if let Some(before) = self.context_before {
                if highlight_start.line() - 1 < before as u32 {
                    1
                } else {
                    highlight_start.line() - before as u32
                }
            } else {
                highlight_start.line()
            };

            let context_end = if let Some(after) = self.context_after {
                if highlight_end.line() + (after as u32) > file_map.last_line() {
                    file_map.last_line()
                } else {
                    highlight_end.line() + (after as u32)
                }
            } else {
                highlight_end.line()
            };

            (highlight, context_start, context_end)
        };

        let start_slice = file_map.line_start(context_start_line);
        let end_slice = file_map.line_end(context_end_line);

        let context_text = file_unit.slice(Range {
            start: start_slice,
            end: end_slice,
        });
        /*
        std::println!(
            "context line start {} -> {} line end {} -> {}\n\"{}\"",
            context_start_line,
            start_slice,
            context_end_line,
            end_slice,
            context_text,
        );
        */

        writeln!(
            writer,
            "     {}{}[{}]",
            BOXING[TL], BOXING[H], file_unit.filename
        )?;
        for (line_i_rel, line) in context_text.lines().enumerate() {
            let line_nb = context_start_line + line_i_rel as u32;
            writeln!(writer, "{:4} {} {}", line_nb, BOXING[V], line)?;
        }

        writeln!(
            writer,
            "{}{}{}{}{}{}",
            BOXING[H], BOXING[H], BOXING[H], BOXING[H], BOXING[H], BOXING[BR],
        )?;

        Ok(())
    }
}
