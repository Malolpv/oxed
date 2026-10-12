#[derive(Clone, Copy)]
pub struct Cell {
    pub symbol: char,
    // foreground: Color,
    // background: Color,
    // attributes: Vec<Attribute>,
}

impl Cell {
    pub const fn new(symbol: char) -> Self {
        Self { symbol }
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }

    fn ne(&self, other: &Self) -> bool {
        self.symbol != other.symbol
    }
}
