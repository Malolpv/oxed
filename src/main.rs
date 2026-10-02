use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    println,
};

use clap::Parser;

use crossterm::event::{KeyCode, KeyEvent};

mod buffer;
mod terminal;
mod view;

use crate::buffer::Buffer;
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
    Insert,
    Visual,
    Command,
}

/// Polling rate in ms
const POLLING_RATE_AS_MS: u64 = 100;

/// This represent the text editor
struct Oxed<W: Write> {
    // Current working directory
    cwd: PathBuf,
    file: Option<PathBuf>,
    mode: Mode,
    state: State,
    terminal: Terminal<W>,
    buffer: Buffer,
}

impl<W: Write> Oxed<W> {
    fn init(cwd: PathBuf, file: Option<PathBuf>, writer: W) -> io::Result<Self> {
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

        Ok(Self {
            cwd,
            file,
            mode: Mode::Normal,
            state: State::Running,
            terminal: Terminal::new(POLLING_RATE_AS_MS, writer),
            buffer,
        })
    }

    // TODO Rework this
    fn render_base_screen(&mut self) -> io::Result<()> {
        // Preparing alternate screen
        let screen_size = Terminal::<W>::size()?;

        self.terminal.open(false)?;

        View::render(&self.buffer, screen_size)?;

        // flushing only when its necessary
        self.terminal.flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.render_base_screen()?;

        loop {
            match self.state {
                State::Running => {
                    let event = self.terminal.read_event();
                    self.handle_event(&event);
                }

                State::Stop => {
                    self.terminal.close()?;
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

    fn handle_event(&mut self, event: &OxedEvent) {
        match event {
            OxedEvent::Key(key) => match self.mode {
                Mode::Normal => self.handle_normal_key(key),
                _ => eprintln!("Unhandled mode"),
            },
            OxedEvent::Resize(width, height) => eprintln!("Unhandled resize"),
            OxedEvent::Tick => {}
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
