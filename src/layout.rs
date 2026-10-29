use crate::{
    buffer::Buffer,
    cell::Cell,
    oxed::{Mode, Position, Status},
};

struct Layout<'a> {
    // header: Option<TopBar>,
    viewport: Viewport,
    status_bar: StatusBar<'a>,
    // command_line: CommandLine,
}

struct StatusBar<'a> {
    mode: &'a Mode,
    file_status: String,
    cursor_position: &'a Position,
}

impl<'a> From<&'a Status> for StatusBar<'a> {
    fn from(value: &'a Status) -> Self {
        Self {
            mode: &value.mode,
            file_status: String::from("Unchanged"),
            cursor_position: &value.cursor_position,
        }
    }
}

struct Viewport {
    size: (u16, u16),
    cells: Vec<Cell>,
}

impl Viewport {
    fn generate_cells(buffer: &Buffer, height: usize, width: usize, status: &Status) -> Vec<Cell> {
        let mut visible_buffer: Vec<Cell> = vec![Cell::new(' '); height.saturating_mul(width)];

        let visible_lines = buffer.content().iter().skip(status.offset.row).take(height);

        for (y, line) in visible_lines.enumerate() {
            let visible_chars = line.chars().skip(status.offset.col).take(width);

            for (x, symbol) in visible_chars.enumerate() {
                let idx = y.saturating_mul(width).saturating_add(x);
                if let Some(cell) = visible_buffer.get_mut(idx) {
                    cell.symbol = symbol;
                }
            }
        }

        visible_buffer
    }
}

impl<'a> Layout<'a> {
    const MIN_WIDTH: u16 = 20;
    const MIN_HEIGHT: u16 = 20;
    const STATUS_BAR_HEIGHT: u16 = 1;

    fn new(visible_buffer: &'a Buffer, status: &'a Status, size: (u16, u16)) -> Self {
        let (width, height) = size;

        // TODO Return an empty layout with an error message
        if width < Self::MIN_WIDTH || height < Self::MIN_HEIGHT {}

        let viewport_height = height.saturating_sub(Self::STATUS_BAR_HEIGHT);
        let viewport_width = width;

        let cells = Viewport::generate_cells(
            visible_buffer,
            usize::from(viewport_height),
            usize::from(viewport_width),
            status,
        );

        Self {
            viewport: Viewport {
                size: (viewport_width, viewport_height),
                cells,
            },
            status_bar: StatusBar::from(status),
        }
    }
}
