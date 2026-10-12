use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    println,
};

use clap::Parser;

use crossterm::{
    QueueableCommand, cursor,
    event::{KeyCode, KeyEvent},
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

mod buffer;
mod cell;
mod frame_buffer;
mod terminal;
mod view;

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::frame_buffer::FrameBuffer;
use crate::terminal::Terminal;
use crate::view::View;

#[derive(Parser, Debug)]
#[command(name = "oxed")]
struct Args {
    /// File path
    file_path: Option<PathBuf>,
}

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
struct Oxed<W: Write> {
    // Current working directory
    _cwd: PathBuf,
    _file: Option<PathBuf>,
    mode: Mode,
    state: State,
    terminal: Terminal,
    buffer: Buffer,
    view: View,
    writer: W,
}

impl<W: Write> Oxed<W> {
    fn init(cwd: PathBuf, file: Option<PathBuf>, mut writer: W) -> io::Result<Self> {
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
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let initial_frame = self.generate_frame()?;
        self.view.handle_frame(initial_frame, &mut self.writer)?;
        self.writer.flush()?;

        loop {
            match self.state {
                State::Running => {
                    let event = self.terminal.read_event();
                    self.handle_event(&event)?;
                }

                State::Stop => {
                    // TODO here add the closing code
                    return Ok(());
                }
            }

            // Do some other things
        }
    }

    fn handle_normal_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if key.is_press() {
                    self.state = State::Stop;
                }
            }
            _ => {
                let _res = View::render_error("Unhandled keybind");
            }
        }
    }

    fn handle_event(&mut self, event: &OxedEvent) -> io::Result<()> {
        match event {
            OxedEvent::Key(key) => match self.mode {
                Mode::Normal => self.handle_normal_key(key),
                _ => eprintln!("Unhandled mode"),
            },
            OxedEvent::Resize(_width, _height) => {
                // manage redrawing
                let next_frame: FrameBuffer = self.generate_frame()?;
                self.view.handle_frame(next_frame, &mut self.writer)?;
                self.writer.flush()?;
            }
            OxedEvent::Tick => {
                // No need to recreate a frame since nothing happened
            }
        }
        Ok(())
    }

    fn generate_frame(&self) -> io::Result<FrameBuffer> {
        let (width, height) = Terminal::size()?;

        let mut frame: Vec<Cell> = vec![Cell::new(' '); usize::from(height.saturating_mul(width))];

        if let Some(buf) = self
            .buffer
            .content()
            .get(0..std::cmp::min(usize::from(height), self.buffer.len()))
        {
            for (y, line) in buf.iter().enumerate() {
                for (x, symbol) in line.chars().enumerate() {
                    if x < usize::from(width) {
                        let idx = y.saturating_mul(usize::from(width)).saturating_add(x);
                        if let Some(cell) = frame.get_mut(idx) {
                            cell.symbol = symbol;
                        }
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

#[derive(Debug, PartialEq)]
enum OxedEvent {
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

fn main() -> io::Result<()> {
    println!("Initializing oxed...");

    let args = Args::parse();
    if let Some(ref path) = args.file_path {
        println!("Opening : {}", path.display());
    }

    let cwd = std::env::current_dir()?;
    println!("Current working directory: {}", cwd.display());

    // We are wrapping the stdout handle inside a bufwritter to minimize syscalls.
    // this way syscalls happens only when flushing the bufwriter
    // Cannot restore cursor position correctly for some reason ???
    // let mut oxed =
    //     Oxed::<BufWriter<io::Stdout>>::init(cwd, args.file_path, BufWriter::new(io::stdout()))?;

    // Successfully retore position so stay with this for now
    let mut oxed = Oxed::<io::Stdout>::init(cwd, args.file_path, io::stdout())?;

    oxed.run()?;

    Ok(())
}
