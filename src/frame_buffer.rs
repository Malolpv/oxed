use crate::cell::Cell;

pub struct FrameBuffer {
    /// (width, height)
    pub size: (u16, u16),
    // lenght is W x H from the terminal size
    pub content: Vec<Cell>,
}

impl FrameBuffer {
    pub const fn new(size: (u16, u16), content: Vec<Cell>) -> Self {
        Self { size, content }
    }

    pub fn get_at(&self, x: u16, y: u16) -> Option<&Cell> {
        let idx = y.saturating_mul(self.size.0).saturating_add(x);
        self.content.get(usize::from(idx))
    }

    pub fn diff(&self, other: &Self) -> Diff {
        if self.size != other.size || self.size.0 == 0 || self.size.1 == 0 {
            // They aren't the same size lets redraw
            return Diff::Full;
        }

        let (width, height) = self.size;
        let total_cells = width.saturating_mul(height);

        let mut spans: Vec<Span> = Vec::new();
        let mut i: u16 = 0;

        while i < total_cells {
            let cell_a = self.content.get(usize::from(i));
            let cell_b = other.content.get(usize::from(i));

            if let (Some(a), Some(b)) = (cell_a, cell_b)
                && a != b
            {
                let start_i = i;
                let mut sequence = Vec::new();

                while i < total_cells {
                    if i > start_i && (i.is_multiple_of(width)) {
                        // we reached end of line the span must stop here
                        break;
                    }
                    match (
                        self.content.get(usize::from(i)),
                        other.content.get(usize::from(i)),
                    ) {
                        (Some(a_cell), Some(b_cell)) if a_cell != b_cell => {
                            sequence.push(*b_cell);
                            i = i.saturating_add(1);
                        }
                        _ => break,
                    }
                }

                // Compute origin coordinates
                let x = start_i.checked_rem(width).unwrap_or_default();
                let y = start_i.checked_div(width).unwrap_or_default();

                spans.push(Span { x, y, sequence });
            } else {
                i = i.saturating_add(1);
            }
        }

        if spans.is_empty() {
            Diff::NoChange
        } else {
            Diff::Partial(spans)
        }
    }
}

pub enum Diff {
    Full,
    Partial(Vec<Span>),
    NoChange,
}

pub struct Span {
    pub x: u16,
    pub y: u16,
    pub sequence: Vec<Cell>,
}
