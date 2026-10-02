use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    println,
    time::Duration,
};

use clap::Parser;

use crossterm::{
    QueueableCommand,
    cursor::{self},
    event::{self, KeyCode, KeyEvent},
    terminal::{self, Clear, EnterAlternateScreen, LeaveAlternateScreen, size},
};

mod buffer;

use crate::buffer::Buffer;

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
struct Oxed {
    // Current working directory
    cwd: PathBuf,
    file: Option<PathBuf>,
    mode: Mode,
    state: State,
    terminal: Terminal,
    buffer: Buffer,
}

impl Oxed {
    fn init(cwd: PathBuf, file: Option<PathBuf>) -> io::Result<Self> {
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
            terminal: Terminal::new(POLLING_RATE_AS_MS),
            buffer,
        })
    }

    // TODO Rework this
    fn render_base_screen(buf: &Buffer) -> io::Result<()> {
        // Preparing alternate screen
        let mut handle = io::stdout();
        let screen_size = size()?;

        crossterm::terminal::enable_raw_mode()?;
        handle.queue(EnterAlternateScreen)?;
        // handle.queue(cursor::MoveTo(screen_size.0 / 2, screen_size.1 / 2))?;
        // handle.write_all(b"test alternative screen")?;
        // handle.queue(cursor::MoveTo(0, screen_size.1))?;

        View::render(buf, screen_size)?;

        // flushing only when its necessary
        io::stdout().flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Rework this
        Self::render_base_screen(&self.buffer)?;

        loop {
            let event = self.terminal.read_event();

            self.state = self.handle_event(&event);

            match self.state {
                State::Running => {
                    // Do some running things
                }

                State::Stop => {
                    Self::quit()?;
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

    fn quit() -> io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        io::stdout().queue(LeaveAlternateScreen)?;
        io::stdout().write_all(b"Leaving oxed...")?;
        io::stdout().flush()?;
        Ok(())
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

    let mut oxed = Oxed::init(cwd, args.file_path)?;

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

struct Terminal {
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
}

struct View;

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

        let size = terminal::size()?;
        let pos = cursor::position()?;

        // Move the cursor to bottom row
        if pos.1 != size.1 || pos.0 != 0 {
            handle.queue(cursor::MoveTo(0, size.1))?;
        }

        handle.queue(Clear(terminal::ClearType::CurrentLine))?;

        handle.write_all(msg.as_bytes())?;

        handle.flush()?;

        Ok(())
    }
}
