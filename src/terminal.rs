use crate::OxedEvent;
use crossterm::QueueableCommand;
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, terminal};
use std::io::{self, Write};
use std::time::Duration;

pub struct Terminal<W: Write> {
    polling_rate: u64,
    writer: W,
}

impl<W: Write> Terminal<W> {
    /// Instanciate a new terminal
    /// Polling rate must be exprimed in ms
    pub const fn new(polling_rate: u64, writer: W) -> Self {
        Self {
            polling_rate,
            writer,
        }
    }

    pub fn read_event(&self) -> OxedEvent {
        if matches!(
            event::poll(Duration::from_millis(self.polling_rate)),
            Ok(true)
        ) && let Ok(event) = crossterm::event::read()
        {
            return event.try_into().unwrap_or(OxedEvent::Tick);
        }

        OxedEvent::Tick
    }

    pub fn size() -> io::Result<(u16, u16)> {
        terminal::size()
    }

    pub fn open(&mut self) -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        self.writer.queue(EnterAlternateScreen)?;
        self.writer.queue(Clear(ClearType::All))?;

        Ok(())
    }

    pub fn close(&mut self) -> io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        self.writer.queue(LeaveAlternateScreen)?;
        self.writer.write_all(b"Leaving oxed...")?;
        self.writer.flush()?;
        Ok(())
    }
}
