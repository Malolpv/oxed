use std::{
    io::{self, Write},
    path::PathBuf,
    println,
};

use clap::Parser;

use crossterm::{
    QueueableCommand, cursor,
    event::{Event, KeyCode, KeyEvent, read},
    execute,
    terminal::{self, Clear, EnterAlternateScreen, LeaveAlternateScreen, size},
};

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
    // Insert,
    // Visual,
}

/// This represent the text editor
struct Oxed {
    // Current working directory
    cwd: PathBuf,
    file: Option<PathBuf>,
    mode: Mode,
}

impl Oxed {
    const fn init(cwd: PathBuf, file: Option<PathBuf>) -> Self {
        Self {
            cwd,
            file,
            mode: Mode::Normal,
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

    let _oxed = Oxed::init(cwd, args.file_path);

    // Preparing alternate screen
    let mut handle = io::stdout();
    let screen_size = size()?;

    handle.queue(EnterAlternateScreen)?;
    handle.queue(cursor::MoveTo(screen_size.0 / 2, screen_size.1 / 2))?;
    handle.write_all(b"test alternative screen")?;
    handle.queue(cursor::MoveTo(0, screen_size.1))?;

    // flushing only when its necessary
    io::stdout().flush()?;

    let mut state: State = State::Running;

    while matches!(state, State::Running) {
        match read()? {
            Event::Key(event) => {
                state = handle_key_event(event)?;
            }
            _ => {
                println!("Received isnt a key press");
            }
        }
    }

    handle.write_all(b"Leaving oxed...")?;

    Ok(())
}

fn handle_key_event(e: KeyEvent) -> io::Result<State> {
    let mut handle = io::stdout();

    let buf = format!("{e:?}");

    // Perform actions on press only
    if e.is_press() {
        match e.code {
            KeyCode::Esc => {
                // Escape key has been pressed so we can notify the program to leave the alternate screen
                execute!(handle, LeaveAlternateScreen)?;
                return Ok(State::Stop);
            }

            code @ (KeyCode::Left | KeyCode::Down | KeyCode::Up | KeyCode::Right) => {
                if let Some(direction) = Direction::from_keycode(code) {
                    move_cursor(&direction)?;
                }
            }

            _ => {
                handle.queue(Clear(terminal::ClearType::All))?;
                handle.queue(cursor::RestorePosition)?;
                handle.write_all(&buf.into_bytes())?;
            }
        }
    }

    handle.flush()?;

    Ok(State::Running)
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const fn from_keycode(key_code: KeyCode) -> Option<Self> {
        match key_code {
            KeyCode::Up => Some(Self::Up),
            KeyCode::Down => Some(Self::Down),
            KeyCode::Left => Some(Self::Left),
            KeyCode::Right => Some(Self::Right),
            _ => None,
        }
    }
}

fn move_cursor(direction: &Direction) -> io::Result<()> {
    let mut handle = io::stdout();

    let t_size = size()?;
    let position = cursor::position()?;

    match direction {
        Direction::Up => {
            if position.1 > 0 {
                handle.queue(cursor::MoveUp(1))?;
            }
        }
        Direction::Down => {
            if position.1 < t_size.1 {
                handle.queue(cursor::MoveDown(1))?;
            }
        }
        Direction::Left => {
            if position.0 > 0 {
                handle.queue(cursor::MoveLeft(1))?;
            }
        }
        Direction::Right => {
            if position.0 < t_size.0 {
                handle.queue(cursor::MoveRight(1))?;
            }
        }
    }

    Ok(())
}
