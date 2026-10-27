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
        KeyCode::{self},
        KeyEvent,
    },
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};

use crate::cell::Cell;
use crate::frame_buffer::FrameBuffer;
use crate::terminal::Terminal;
use crate::view::View;
use crate::{buffer::Buffer, oxed::Mode::Normal};

enum State {
    Running,
    Stop,
}

enum Mode {
    Normal,
    Insert,
    _Visual,
    _Command,
}

enum Action {
    None,
    Redraw,
    Resize,
    Quit,
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
}

impl<W: Write> Oxed<W> {
    pub fn init(cwd: PathBuf, file: Option<PathBuf>, mut writer: W) -> io::Result<Self> {
        let mut title: Option<&str> = None;
        // Load the buffer with given file if it exist
        let buffer = if let Some(file) = &file {
            println!("INFO opening file: {}", file.display());

            if let Some(name) = file.file_name() {
                title = Some(name.to_str().unwrap_or(""));
            }

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
        writer.queue(SetTitle(format!("Oxed : {}", title.unwrap_or("Untitled"))))?;
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
                    let action = self.handle_event(&event)?;

                    match action {
                        Action::Redraw | Action::Resize => {
                            let next_frame: FrameBuffer = self.generate_frame()?;
                            self.view
                                .handle_frame(next_frame, &mut self.writer, false)?;

                            let x_relative_pos = self.cursor_x.saturating_sub(self.col_offset);
                            let y_relative_pos = self.cursor_y.saturating_sub(self.row_offset);

                            // Puts cursor back in place after redraw
                            self.writer.queue(cursor::MoveTo(
                                u16::try_from(x_relative_pos).unwrap_or(u16::MIN),
                                u16::try_from(y_relative_pos).unwrap_or(u16::MIN),
                            ))?;
                            self.writer.flush()?;
                        }
                        Action::Quit => {
                            self.state = State::Stop;
                        }

                        Action::None => {
                            // Do Nothing
                        }
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

    fn handle_event(&mut self, event: &OxedEvent) -> io::Result<Action> {
        match event {
            OxedEvent::Key(key) => match self.mode {
                Mode::Normal => self.handle_normal_key(key),
                //TODO handle this one aswell
                Mode::Insert => self.handle_insert_key(key),
                _ => {
                    eprintln!("Unhandled mode");
                    Ok(Action::None)
                }
            },
            OxedEvent::Resize(_width, _height) => {
                //Signals redrawing
                Ok(Action::Resize)
            }
            OxedEvent::Tick => Ok(Action::None),
        }
    }

    fn handle_insert_key(&mut self, key: &KeyEvent) -> io::Result<Action> {
        if !key.is_press() {
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Char(c) => {
                // Retrieve current line
                if let Some(line) = self.buffer.content_mut().get_mut(self.cursor_y) {
                    let insert_idx = self.cursor_x.min(line.chars().count());

                    let byte_idx = std::cmp::min(insert_idx, line.len());

                    // let byte_idx = line
                    //     .char_indices()
                    //     .map(|(idx, _)| idx)
                    //     .nth(insert_idx)
                    //     .unwrap_or(line.len());

                    // Insert and update cursor
                    line.insert(byte_idx, c);
                    self.cursor_x = self.cursor_x.saturating_add(1);

                    // Handle automatic scroll
                    let (screen_width, _) = Terminal::size()?;
                    if self.cursor_x >= self.col_offset.saturating_add(usize::from(screen_width)) {
                        self.col_offset = self.col_offset.saturating_add(1);
                    }
                }
                Ok(Action::Redraw)
            }
            KeyCode::Backspace => {
                if self.cursor_x > 0 {
                    self.cursor_x = self.cursor_x.saturating_sub(1);
                    if let Some(line) = self.buffer.content_mut().get_mut(self.cursor_y) {
                        let remove_idx = self.cursor_x;
                        let byte_idx = line.char_indices().map(|(idx, _)| idx).nth(remove_idx);

                        if let Some(b_idx) = byte_idx {
                            line.remove(b_idx);
                        }
                    }

                    // Adjust coll offset based on previous actions
                    if self.cursor_x < self.col_offset {
                        self.col_offset = self.col_offset.saturating_sub(1);
                    }
                }
                Ok(Action::Redraw)
            }

            KeyCode::Esc => {
                self.mode = Normal;
                Ok(Action::None)
            }

            _ => {
                // unhandled key
                Ok(Action::None)
            }
        }
    }

    fn handle_normal_key(&mut self, key: &KeyEvent) -> io::Result<Action> {
        if !key.is_press() {
            return Ok(Action::None);
        }

        let (screen_width, screen_height) = Terminal::size()?;
        let screen_height = usize::from(screen_height);
        let screen_width = usize::from(screen_width);

        match key.code {
            KeyCode::Esc => Ok(Action::Quit),
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor_y = self.cursor_y.saturating_sub(1);

                // Do we need to scroll up ??
                if self.cursor_y < self.row_offset {
                    self.row_offset = self.row_offset.saturating_sub(1);
                    return Ok(Action::Redraw);
                }
                self.writer.execute(MoveUp(1))?;
                Ok(Action::None)
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
                        return Ok(Action::Redraw);
                    }

                    self.writer.execute(MoveDown(1))?;
                }
                Ok(Action::None)
            }

            // TODO implement horizontal scrolling
            KeyCode::Right | KeyCode::Char('l') => {
                if self.cursor_x.checked_add(1).is_some_and(|x| {
                    x < self
                        .buffer
                        .line(self.cursor_y)
                        .unwrap_or(&String::new())
                        .len()
                }) {
                    self.cursor_x = self.cursor_x.checked_add(1).unwrap_or_default();

                    if self.cursor_x >= self.col_offset.saturating_add(screen_width) {
                        self.col_offset = self.col_offset.checked_add(1).unwrap_or_default();
                        return Ok(Action::Redraw);
                    }
                    self.writer.execute(MoveRight(1))?;
                }

                Ok(Action::None)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.cursor_x = self.cursor_x.saturating_sub(1);

                if self.cursor_x < self.col_offset {
                    self.col_offset = self.col_offset.saturating_sub(1);
                    return Ok(Action::Redraw);
                }

                self.writer.execute(MoveLeft(1))?;

                Ok(Action::None)
            }
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
                Ok(Action::None)
            }
            _ => {
                let _res = View::render_error("Unhandled keybind");
                Ok(Action::None)
            }
        }
    }

    fn generate_frame(&self) -> io::Result<FrameBuffer> {
        let (width, height) = Terminal::size()?;
        let (w, h) = (usize::from(width), usize::from(height));

        let mut frame: Vec<Cell> = vec![Cell::new(' '); usize::from(height.saturating_mul(width))];

        let visible_lines = self.buffer.content().iter().skip(self.row_offset).take(h);

        for (y, line) in visible_lines.enumerate() {
            let visible_chars = line.chars().skip(self.col_offset).take(w);

            for (x, symbol) in visible_chars.enumerate() {
                // if x < w {
                let idx = y.saturating_mul(w).saturating_add(x);
                if let Some(cell) = frame.get_mut(idx) {
                    cell.symbol = symbol;
                }
                // }
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
