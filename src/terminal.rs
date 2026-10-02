use crate::OxedEvent;
use crossterm::{event, terminal};
use std::io;
use std::time::Duration;

pub struct Terminal {
    polling_rate: u64,
}

impl Terminal {
    /// Instanciate a new terminal
    /// Polling rate must be exprimed in ms
    pub const fn new(polling_rate: u64) -> Self {
        Self { polling_rate }
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
}
