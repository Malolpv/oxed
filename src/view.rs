use crate::{buffer::Buffer, terminal::Terminal};
use crossterm::terminal::ClearType;
use crossterm::{QueueableCommand, cursor, terminal::Clear};
use std::io::{self, Write};

pub struct View;

impl View {
    pub fn render(buffer: &Buffer, size: (u16, u16)) -> io::Result<()> {
        let mut handle = io::stdout();

        handle.queue(cursor::MoveTo(0, 0))?;

        println!(
            "Nombre de lignes: {}\nHauteur du terminal: {}",
            buffer.len(),
            usize::from(size.1)
        );
        if let Some(lines) = buffer
            .content()
            .get(0..std::cmp::min(usize::from(size.1), buffer.len()))
        {
            for line in lines {
                handle.write_all(line.as_bytes())?;
                handle.queue(cursor::MoveToNextLine(1))?;
            }
        }

        handle.queue(cursor::MoveTo(0, 0))?;
        handle.flush()?;

        Ok(())
    }

    /// Render the message on the terminal last line
    pub fn render_error(msg: &str) -> io::Result<()> {
        let mut handle = io::stdout();

        //TODO Maybe View also need to get the writer type as generic ?
        let size = Terminal::<io::Stdout>::size()?;
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
}
