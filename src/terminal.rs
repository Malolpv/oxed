use crate::oxed::OxedEvent;
// use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
// use crossterm::{QueueableCommand, cursor};
use crossterm::{event, terminal};
use std::io::{self};
use std::time::Duration;

pub struct Terminal {
    polling_rate: u64,
}

impl Terminal {
    /// Instanciate a new terminal
    /// Polling rate must be exprimed in ms
    pub fn new(polling_rate: u64) -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self { polling_rate })
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

    /// Returns the terminal size `(columns, rows)`.
    ///
    /// The top left cell is represented `(1, 1)`.
    pub fn size() -> io::Result<(u16, u16)> {
        terminal::size()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // TODO log this in case of error maybe
        _ = crossterm::terminal::disable_raw_mode();
    }
}
