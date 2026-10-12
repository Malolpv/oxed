use std::{
    io::{self},
    path::PathBuf,
    println,
};

use clap::Parser;

mod buffer;
mod cell;
mod frame_buffer;
mod oxed;
mod terminal;
mod view;

use crate::oxed::Oxed;

#[derive(Parser, Debug)]
#[command(name = "oxed")]
struct Args {
    /// File path
    file_path: Option<PathBuf>,
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
