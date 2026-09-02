use std::{
    io::{self, Write},
    println,
};

use crossterm::{
    QueueableCommand, cursor,
    event::{Event, KeyCode, KeyEvent, read},
    execute,
    terminal::{self, Clear, EnterAlternateScreen, LeaveAlternateScreen, size},
};

enum State {
    Running,
    Stop,
}

fn main() -> io::Result<()> {
    println!("Initializing oxed...");

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

    if e.code == KeyCode::Esc {
        // Escape key has been pressed so we can notify the program to leave the alternate screen
        execute!(handle, LeaveAlternateScreen)?;
        return Ok(State::Stop);
    }

    handle.queue(Clear(terminal::ClearType::All))?;
    handle.queue(cursor::RestorePosition)?;
    handle.write_all(&buf.into_bytes())?;

    handle.flush()?;

    Ok(State::Running)
}
