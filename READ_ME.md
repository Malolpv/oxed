# Oxed

Oxed is a lightweight, modular, and performant Vim-like terminal text editor written in Rust.

> **Status:** Work in progress. This is a personal side project under active development.

## Features

Oxed features a custom rendering architecture optimized for performance and minimal system overhead:

* **File Loading & Rendering:** Efficiently parses and displays text files within the terminal viewport.
* **Optimized Rendering Pipeline:** Employs a custom `FrameBuffer` with a differential update engine (`diff_v2`) to minimize terminal I/O syscalls and eliminate screen flicker.
* **Dynamic Resizing:** Fully handles terminal window resize events with automatic fallback to full-screen redraws.
* **Viewport Management:** Maps text lines dynamically based on screen width dimensions.

## Tech Stack

* **Language:** Rust
* **Terminal Management:** `crossterm` (raw mode, alternative screen management, event polling)
* **Architecture:** Double-buffering pattern with span-based buffer diffing

## Installation & Usage

Ensure you have Rust and Cargo installed on your system. Clone the repository and run the project using Cargo:

```bash
# Clone the repository
git clone [https://github.com/your-username/oxed.git](https://github.com/your-username/oxed.git)
cd oxed

# Run the editor with an optional file path
cargo run -- path/to/file.txt

# Or run it with an empty buffer
cargo run