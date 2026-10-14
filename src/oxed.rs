use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    println,
};

use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::{self, MoveDown, MoveLeft, MoveRight, MoveUp},
    event::{
        KeyCode::{self, Char},
        KeyEvent,
    },
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::frame_buffer::FrameBuffer;
use crate::terminal::Terminal;
use crate::view::View;

enum State {
    Running,
    Stop,
}

enum Mode {
    Normal,
    _Insert,
    _Visual,
    _Command,
}

/// Polling rate in ms
const POLLING_RATE_AS_MS: u64 = 100;

/// This represent the text editor
pub struct Oxed<W: Write> {
    // Current working directory
    _cwd: PathBuf,
    _file: Option<PathBuf>,
    mode: Mode,
    state: State,
    terminal: Terminal,
    buffer: Buffer,
    view: View,
    writer: W,

    cursor_x: usize,
    cursor_y: usize,
    row_offset: usize,
    col_offset: usize,
    needs_redraw: bool,
}

impl<W: Write> Oxed<W> {
    pub fn init(cwd: PathBuf, file: Option<PathBuf>, mut writer: W) -> io::Result<Self> {
        // Load the buffer with given file if it exist
        let buffer = if let Some(file) = &file {
            println!("INFO opening file: {}", file.display());
            let file = File::open(file)?;

            let reader = BufReader::new(file);
            Buffer::new(reader)
        } else {
            Buffer::empty()
        };

        println!("Oxed initialized with {} lines", buffer.len());

        // Create terminal (implicitly enables raw mode)
        let terminal = Terminal::new(POLLING_RATE_AS_MS)?;

        // Initiate terminal
        writer.queue(cursor::SavePosition)?;
        writer.queue(EnterAlternateScreen)?;
        writer.queue(Clear(ClearType::All))?;
        writer.flush()?;

        Ok(Self {
            _cwd: cwd,
            _file: file,
            mode: Mode::Normal,
            state: State::Running,
            terminal,
            buffer,
            view: View::new(None),
            writer,
            cursor_x: usize::default(),
            cursor_y: usize::default(),
            row_offset: usize::default(),
            col_offset: usize::default(),
            needs_redraw: false,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let initial_frame = self.generate_frame()?;
        self.view
            .handle_frame(initial_frame, &mut self.writer, false)?;
        // Reset cursor to top left corner
        self.writer.queue(cursor::MoveTo(0, 0))?;
        self.writer.flush()?;

        // Run oxed
        loop {
            match self.state {
                State::Running => {
                    let event = self.terminal.read_event();
                    self.handle_event(&event)?;

                    if self.needs_redraw {
                        let next_frame: FrameBuffer = self.generate_frame()?;
                        self.view
                            .handle_frame(next_frame, &mut self.writer, false)?;

                        // Puts cursor back in place after redraw
                        self.writer.queue(cursor::MoveTo(
                            u16::try_from(self.cursor_x).unwrap_or(u16::MAX),
                            u16::try_from(self.cursor_y).unwrap_or(u16::MAX),
                        ))?;
                        self.writer.flush()?;
                        self.needs_redraw = false;
                    }
                }

                State::Stop => {
                    // TODO here add the closing code
                    return Ok(());
                }
            }

            // Do some other things
        }
    }

    fn handle_event(&mut self, event: &OxedEvent) -> io::Result<()> {
        match event {
            OxedEvent::Key(key) => match self.mode {
                Mode::Normal => self.handle_normal_key(key)?,
                _ => {
                    eprintln!("Unhandled mode");
                }
            },
            OxedEvent::Resize(_width, _height) => {
                // manage redrawing
                let next_frame: FrameBuffer = self.generate_frame()?;
                self.view.handle_frame(next_frame, &mut self.writer, true)?;
                self.writer.execute(cursor::MoveTo(
                    u16::try_from(self.cursor_x).unwrap_or(u16::MAX),
                    u16::try_from(self.cursor_y).unwrap_or(u16::MAX),
                ))?;
            }
            OxedEvent::Tick => {
                // No need to recreate a frame since nothing happened
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: &KeyEvent) -> io::Result<()> {
        if !key.is_press() {
            return Ok(());
        }

        let (_, screen_height) = Terminal::size()?;
        let screen_height = usize::from(screen_height);

        match key.code {
            KeyCode::Esc => {
                if key.is_press() {
                    self.state = State::Stop;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor_y = self.cursor_y.saturating_sub(1);

                // Do we need to scroll up ??
                if self.cursor_y < self.row_offset {
                    self.row_offset = self.row_offset.saturating_sub(1);
                    self.needs_redraw = true;
                }
                self.writer.execute(MoveUp(1))?;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self
                    .cursor_y
                    .checked_add(1)
                    .is_some_and(|y| y < self.buffer.len())
                {
                    self.cursor_y = self.cursor_y.checked_add(1).unwrap_or(self.buffer.len());

                    if self.cursor_y >= self.row_offset.saturating_add(screen_height) {
                        self.row_offset =
                            self.row_offset.checked_add(1).unwrap_or(self.buffer.len());
                        self.needs_redraw = true;
                    }

                    self.writer.execute(MoveDown(1))?;
                }
            }

            // TODO implement horizontal scrolling
            KeyCode::Right | KeyCode::Char('l') => {
                if key.is_press() {
                    _ = self.writer.execute(MoveRight(1));
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if key.is_press() {
                    _ = self.writer.execute(MoveLeft(1));
                }
            }
            _ => {
                if key.is_press() {
                    let _res = View::render_error("Unhandled keybind");
                }
            }
        }
        Ok(())
    }

    fn generate_frame(&self) -> io::Result<FrameBuffer> {
        let (width, height) = Terminal::size()?;
        let (w, h) = (usize::from(width), usize::from(height));

        let mut frame: Vec<Cell> = vec![Cell::new(' '); usize::from(height.saturating_mul(width))];

        let visible_lines = self.buffer.content().iter().skip(self.row_offset).take(h);

        for (y, line) in visible_lines.enumerate() {
            for (x, symbol) in line.chars().enumerate() {
                if x < w {
                    let idx = y.saturating_mul(w).saturating_add(x);
                    if let Some(cell) = frame.get_mut(idx) {
                        cell.symbol = symbol;
                    }
                }
            }
        }

        Ok(FrameBuffer::new((width, height), frame))
    }
}

impl<W: Write> Drop for Oxed<W> {
    fn drop(&mut self) {
        _ = self.writer.queue(LeaveAlternateScreen);

        // Restore cursor position before leaving oxed
        _ = self.writer.queue(cursor::RestorePosition);

        _ = self.writer.flush();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OxedEvent {
    Resize(u16, u16),
    Key(KeyEvent),
    Tick,
}

impl TryFrom<crossterm::event::Event> for OxedEvent {
    type Error = String;

    fn try_from(value: crossterm::event::Event) -> Result<Self, Self::Error> {
        match value {
            crossterm::event::Event::Key(key) => Ok(Self::Key(key)),
            crossterm::event::Event::Resize(w, h) => Ok(Self::Resize(w, h)),
            _ => Err(String::from("ERROR : Unsupported crossterm event")),
        }
    }
}
