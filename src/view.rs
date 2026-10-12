use crate::frame_buffer::{Diff, FrameBuffer, Span};
use crate::terminal::Terminal;
use crossterm::cursor::MoveTo;
use crossterm::terminal::ClearType;
use crossterm::{QueueableCommand, cursor, terminal::Clear};
use std::io::{self, Write};

pub struct View {
    frame: Option<FrameBuffer>,
}

impl View {
    pub const fn new(frame: Option<FrameBuffer>) -> Self {
        Self { frame }
    }

    /// Render the message on the terminal last line
    /// TODO Remove this use after temporary debuging
    pub fn render_error(msg: &str) -> io::Result<()> {
        let mut handle = io::stdout();

        let size = Terminal::size()?;
        let pos = cursor::position()?;

        // Move the cursor to bottom row
        if pos.1 != size.1 || pos.0 != 0 {
            handle.queue(cursor::MoveTo(0, size.1))?;
        }

        handle.queue(Clear(ClearType::CurrentLine))?;

        handle.write_all(msg.as_bytes())?;

        handle.flush()?;

        Ok(())
    }

    pub fn handle_frame<W: Write>(&mut self, next: FrameBuffer, writer: &mut W) -> io::Result<()> {
        let diff = self
            .frame
            .as_ref()
            .map_or(Diff::Full, |frame| frame.diff(&next));

        match diff {
            Diff::Partial(spans) => {
                Self::render_partial_frame(spans, writer)?;
            }
            Diff::NoChange => {
                // TODO Implement no change, meybe we just want to do nothing ?
            }
            Diff::Full => {
                Self::render_full_frame(&next, writer)?;
            }
        }

        self.frame = Some(next);

        Ok(())
    }

    fn render_partial_frame<W: Write>(spans: Vec<Span>, writer: &mut W) -> io::Result<()> {
        for span in spans {
            Self::write_span(span, writer)?;
        }
        Ok(())
    }

    fn render_full_frame<W: Write>(frame: &FrameBuffer, writer: &mut W) -> io::Result<()> {
        // Reset cursor and screen
        writer.queue(Clear(ClearType::All))?;
        writer.queue(MoveTo(0, 0))?;

        let (width, _) = frame.size;
        let w = usize::from(width);

        for (row_cells, y) in frame.content.chunks(w).zip(0u16..) {
            writer.queue(MoveTo(0, y))?;

            let mut line = String::with_capacity(row_cells.len());
            for cell in row_cells {
                line.push(cell.symbol);
            }

            writer.write_all(line.as_bytes())?;
        }

        // TODO check if this is the right place for flushing
        writer.flush()?;

        Ok(())
    }

    fn write_span<W: Write>(span: Span, writer: &mut W) -> io::Result<()> {
        writer.queue(MoveTo(span.x, span.y))?;

        let mut s: String = String::with_capacity(span.sequence.len());
        for cell in span.sequence {
            s.push(cell.symbol);
        }

        writer.write_all(s.as_bytes())?;

        Ok(())
    }
}
