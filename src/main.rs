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
    // Visual,
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

        self.terminal.open()?;

        View::render(&self.buffer, screen_size)?;

        // flushing only when its necessary
        io::stdout().flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Rework this
        self.render_base_screen()?;

        loop {
            let event = self.terminal.read_event();

            self.state = self.handle_event(&event);

            match self.state {
                State::Running => {
                    // Do some running things
                }

                State::Stop => {
                    self.terminal.close()?;
                    return Ok(());
                }
            }

            // Do some other things
        }
    }

    fn handle_event(&self, event: &OxedEvent) -> State {
        match event {
            OxedEvent::KeyPressed(key) => {
                match self.mode {
                    Mode::Normal => {
                        // Here we should handle any keypress as a keyshortcut
                        // println!("Handle keyr shortcuts");
                        match key.code {
                            // KeyCode::Up | KeyCode::Char('z') => TerminalEvent::MoveUp(1),
                            // KeyCode::Down | KeyCode::Char('s') => TerminalEvent::MoveDown(1),
                            // KeyCode::Left | KeyCode::Char('q') => TerminalEvent::MoveLeft(1),
                            // KeyCode::Right | KeyCode::Char('d') => TerminalEvent::MoveRight(1),
                            // KeyCode::Char('c')
                            //     if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            // {
                            //     TerminalEvent::Quit
                            // }
                            KeyCode::Esc => {
                                // TODO add the stdout handle to oxed struct to simplify testing
                                return State::Stop;
                            }
                            _ => {
                                let _res = View::render_error("Unhandled keybind");
                            }
                        }
                    }
                    Mode::Insert => {
                        //Here we should handle any keypress as a char
                        println!("Handle key press as user input");
                    }
                }
                // println!("Handle key press event");
            }
            OxedEvent::Resize(width, height) => {
                // println!("INFO Resize event [{width},{height}]");
            }
            OxedEvent::Tick => {
                // Later we can handle routines in there
                // println!("INFO: Tick received");
            }
        }

        State::Running
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

    let mut oxed = Oxed::<io::Stdout>::init(cwd, args.file_path, io::stdout())?;

    oxed.run()?;

    Ok(())
}

#[derive(Debug, PartialEq)]
enum OxedEvent {
    Resize(u16, u16),
    KeyPressed(KeyEvent),
    Tick,
}

impl TryFrom<crossterm::event::Event> for OxedEvent {
    type Error = String;

    fn try_from(value: crossterm::event::Event) -> Result<Self, Self::Error> {
        match value {
            crossterm::event::Event::Key(key) => Ok(Self::KeyPressed(key)),
            crossterm::event::Event::Resize(w, h) => Ok(Self::Resize(w, h)),
            _ => Err(String::from("ERROR : Unsupported crossterm event")),
        }
    }
}
