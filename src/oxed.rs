use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
    println,
};

use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::{self},
    event::{
        KeyCode::{self},
        KeyEvent, KeyModifiers,
    },
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};

use crate::cell::Cell;
use crate::frame_buffer::FrameBuffer;
use crate::terminal::Terminal;
use crate::view::View;
use crate::{buffer::Buffer, oxed::Mode::Normal};

enum Direction {
    Up,
    Down,
    Right,
    Left,
}

enum State {
    Running,
    Stop,
}

pub enum Mode {
    Normal,
    Insert,
    _Visual,
    _Command,
}

enum Action {
    None,
    Redraw,
    UpdateCursor,
    Resize,
    Quit,
}

#[derive(Default)]
pub struct Offset {
    pub row: usize,
    pub col: usize,
}

#[derive(Default)]
pub struct Position {
    x: usize,
    y: usize,
}

pub struct Status {
    /// Editor Mode
    pub mode: Mode,
    /// Current cursor position
    pub cursor_position: Position,
    // Might be needed to compute current line/col position
    pub offset: Offset,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            cursor_position: Position::default(),
            offset: Offset::default(),
        }
    }
}

/// Polling rate in ms
const POLLING_RATE_AS_MS: u64 = 100;

/// This represent the text editor
pub struct Oxed<W: Write> {
    // Current working directory
    cwd: PathBuf,
    file: Option<PathBuf>,
    state: State,
    terminal: Terminal,
    buffer: Buffer,
    view: View,
    writer: W,

    /// Oxed status
    status: Status,
    // row_offset: usize,
    // col_offset: usize,
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
            cwd,
            file,
            state: State::Running,
            terminal,
            buffer,
            view: View::new(None),
            writer,
            status: Status::default(),
            // row_offset: usize::default(),
            // col_offset: usize::default(),
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

                            let x_relative_pos = self
                                .status
                                .cursor_position
                                .x
                                .saturating_sub(self.status.offset.col);
                            let y_relative_pos = self
                                .status
                                .cursor_position
                                .y
                                .saturating_sub(self.status.offset.row);

                            // Puts cursor back in place after redraw
                            self.writer.queue(cursor::MoveTo(
                                u16::try_from(x_relative_pos).unwrap_or(u16::MIN),
                                u16::try_from(y_relative_pos).unwrap_or(u16::MIN),
                            ))?;
                            self.writer.flush()?;
                        }
                        Action::UpdateCursor => {
                            let x_relative_pos = self
                                .status
                                .cursor_position
                                .x
                                .saturating_sub(self.status.offset.col);
                            let y_relative_pos = self
                                .status
                                .cursor_position
                                .y
                                .saturating_sub(self.status.offset.row);
                            self.writer.execute(cursor::MoveTo(
                                u16::try_from(x_relative_pos).unwrap_or(u16::MIN),
                                u16::try_from(y_relative_pos).unwrap_or(u16::MIN),
                            ))?;
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
            OxedEvent::Key(key) => match self.status.mode {
                Mode::Normal => self.handle_normal_key(key),
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

        let (screen_width, screen_height) = Terminal::size()?;
        let screen_height = usize::from(screen_height);
        let screen_width = usize::from(screen_width);

        match key.code {
            KeyCode::Char(c) => {
                // Retrieve current line
                if let Some(line) = self
                    .buffer
                    .content_mut()
                    .get_mut(self.status.cursor_position.y)
                {
                    let insert_idx = self.status.cursor_position.x.min(line.chars().count());

                    let byte_idx = std::cmp::min(insert_idx, line.len());

                    // Insert and update cursor
                    line.insert(byte_idx, c);
                    self.status.cursor_position.x = self.status.cursor_position.x.saturating_add(1);

                    // Handle automatic scroll
                    let (screen_width, _) = Terminal::size()?;
                    if self.status.cursor_position.x
                        >= self
                            .status
                            .offset
                            .col
                            .saturating_add(usize::from(screen_width))
                    {
                        self.status.offset.col = self.status.offset.col.saturating_add(1);
                    }
                }
                Ok(Action::Redraw)
            }
            KeyCode::Backspace => {
                if self.status.cursor_position.x > 0 {
                    self.status.cursor_position.x = self.status.cursor_position.x.saturating_sub(1);
                    if let Some(line) = self
                        .buffer
                        .content_mut()
                        .get_mut(self.status.cursor_position.y)
                    {
                        let remove_idx = self.status.cursor_position.x;
                        let byte_idx = line.char_indices().map(|(idx, _)| idx).nth(remove_idx);

                        if let Some(b_idx) = byte_idx {
                            line.remove(b_idx);
                        }
                    }

                    // Adjust col offset based on previous actions
                    if self.status.cursor_position.x < self.status.offset.col {
                        self.status.offset.col = self.status.offset.col.saturating_sub(1);
                    }
                }
                Ok(Action::Redraw)
            }
            KeyCode::Up => {
                Ok(self.compute_next_position(&Direction::Up, screen_width, screen_height))
            }
            KeyCode::Down => {
                Ok(self.compute_next_position(&Direction::Down, screen_width, screen_height))
            }
            KeyCode::Right => {
                Ok(self.compute_next_position(&Direction::Right, screen_width, screen_height))
            }
            KeyCode::Left => {
                Ok(self.compute_next_position(&Direction::Left, screen_width, screen_height))
            }

            KeyCode::Esc => {
                self.status.mode = Normal;
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
                Ok(self.compute_next_position(&Direction::Up, screen_width, screen_height))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                Ok(self.compute_next_position(&Direction::Down, screen_width, screen_height))
            }
            KeyCode::Right | KeyCode::Char('l') => {
                Ok(self.compute_next_position(&Direction::Right, screen_width, screen_height))
            }
            KeyCode::Left | KeyCode::Char('h') => {
                Ok(self.compute_next_position(&Direction::Left, screen_width, screen_height))
            }
            KeyCode::Char('i') => {
                self.status.mode = Mode::Insert;
                Ok(Action::None)
            }
            // TODO change this to properly handle save
            KeyCode::Char('s') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.save()?;
                }
                Ok(Action::None)
            }
            _ => {
                let _res = View::render_error("Unhandled keybind");
                Ok(Action::None)
            }
        }
    }

    fn compute_next_position(
        &mut self,
        direction: &Direction,
        screen_width: usize,
        screen_height: usize,
    ) -> Action {
        match direction {
            Direction::Up => self.compute_move_up(),
            Direction::Down => self.compute_move_down(screen_height),
            Direction::Right => self.compute_move_right(screen_width),
            Direction::Left => self.compute_move_left(),
        }
    }

    fn compute_move_up(&mut self) -> Action {
        self.status.cursor_position.y = self.status.cursor_position.y.saturating_sub(1);

        let mut action =
            if let Some(line) = self.buffer.content().get(self.status.cursor_position.y) {
                self.status.cursor_position.x =
                    std::cmp::min(line.chars().count(), self.status.cursor_position.x);
                if self.status.cursor_position.x < self.status.offset.col {
                    self.status.offset.col = self.status.cursor_position.x;
                }
                Action::Redraw
            } else {
                Action::UpdateCursor
            };

        action = if self.status.cursor_position.y < self.status.offset.row {
            self.status.offset.row = self.status.offset.row.saturating_sub(1);
            Action::Redraw
        } else {
            action
        };

        action
    }

    fn compute_move_down(&mut self, screen_height: usize) -> Action {
        if self
            .status
            .cursor_position
            .y
            .checked_add(1)
            .is_some_and(|y| y < self.buffer.len())
        {
            self.status.cursor_position.y = self
                .status
                .cursor_position
                .y
                .checked_add(1)
                .unwrap_or(self.buffer.len());

            let mut action =
                if let Some(line) = self.buffer.content().get(self.status.cursor_position.y) {
                    self.status.cursor_position.x =
                        std::cmp::min(line.chars().count(), self.status.cursor_position.x);
                    if self.status.cursor_position.x < self.status.offset.col {
                        self.status.offset.col = self.status.cursor_position.x;
                    }
                    Action::Redraw
                } else {
                    Action::UpdateCursor
                };

            action = if self.status.cursor_position.y
                >= self.status.offset.row.saturating_add(screen_height)
            {
                self.status.offset.row = self
                    .status
                    .offset
                    .row
                    .checked_add(1)
                    .unwrap_or(self.buffer.len());
                Action::Redraw
            } else {
                action
            };

            return action;
        }
        Action::None
    }

    fn compute_move_right(&mut self, screen_width: usize) -> Action {
        if self
            .status
            .cursor_position
            .x
            .checked_add(1)
            .is_some_and(|x| {
                x <= self
                    .buffer
                    .line(self.status.cursor_position.y)
                    .unwrap_or(&String::new())
                    .chars()
                    .count()
            })
        {
            self.status.cursor_position.x = self
                .status
                .cursor_position
                .x
                .checked_add(1)
                .unwrap_or_default();

            if self.status.cursor_position.x >= self.status.offset.col.saturating_add(screen_width)
            {
                self.status.offset.col = self.status.offset.col.checked_add(1).unwrap_or_default();
                return Action::Redraw;
            }
        }
        Action::UpdateCursor
    }

    const fn compute_move_left(&mut self) -> Action {
        self.status.cursor_position.x = self.status.cursor_position.x.saturating_sub(1);

        if self.status.cursor_position.x < self.status.offset.col {
            self.status.offset.col = self.status.offset.col.saturating_sub(1);
            return Action::Redraw;
        }
        Action::UpdateCursor
    }

    fn generate_frame(&self) -> io::Result<FrameBuffer> {
        let (width, height) = Terminal::size()?;
        let (w, h) = (usize::from(width), usize::from(height));

        let mut frame: Vec<Cell> = vec![Cell::new(' '); usize::from(height.saturating_mul(width))];

        let visible_lines = self
            .buffer
            .content()
            .iter()
            .skip(self.status.offset.row)
            .take(h);

        for (y, line) in visible_lines.enumerate() {
            let visible_chars = line.chars().skip(self.status.offset.col).take(w);

            for (x, symbol) in visible_chars.enumerate() {
                let idx = y.saturating_mul(w).saturating_add(x);
                if let Some(cell) = frame.get_mut(idx) {
                    cell.symbol = symbol;
                }
            }
        }

        Ok(FrameBuffer::new((width, height), frame))
    }

    // TODO Handle new file
    // when file isnt specified at oxed init, we should ask user to provide a file name
    fn save(&self) -> io::Result<()> {
        // TODO if filename is None ask user to give one
        let path = self.file.as_ref().map_or_else(
            || {
                let mut path = self.cwd.clone();
                path.push("new_file.txt");
                path
            },
            Clone::clone,
        );

        let file = OpenOptions::new()
            // Create if file does not exist
            .create(true)
            // Clear file content
            .truncate(true)
            //Open in edition
            .write(true)
            .open(path)?;

        // Bufferise to write in 1 syscall
        let mut writer = BufWriter::new(file);

        for line in self.buffer.content() {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;

        Ok(())
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
