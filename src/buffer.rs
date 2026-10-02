pub struct Buffer {
    content: Vec<String>,
}

impl Buffer {
    pub fn new(input: impl std::io::BufRead) -> Self {
        let mut content: Vec<String> = vec![];

        for line in input.lines() {
            if let Ok(line) = line {
                content.push(line);
            } else {
                eprintln!("Error while reading line from input");
            }
        }

        Self { content }
    }

    pub const fn empty() -> Self {
        Self { content: vec![] }
    }

    pub const fn len(&self) -> usize {
        self.content.len()
    }

    pub fn line(&self, index: usize) -> Option<&String> {
        self.content.get(index)
    }

    pub const fn content(&self) -> &Vec<String> {
        &self.content
    }
}

#[cfg(test)]
mod buffer_tests {
    use std::io::Cursor;

    use super::Buffer;

    #[test]
    fn test_buffer_empty() {
        let cursor = Cursor::new("");

        let buf = Buffer::new(cursor);

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_buffer_from_cursor() {
        let cursor = std::io::Cursor::new("line 1\nline 2\nline 3");

        let buf = Buffer::new(cursor);

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.line(0), Some(&String::from("line 1")));
        assert_eq!(buf.line(2), Some(&String::from("line 3")));
    }

    #[test]
    fn test_line_out_of_bounds() {
        let cursor = std::io::Cursor::new("");

        let buf = Buffer::new(cursor);

        assert_eq!(buf.line(99), None);
    }

    #[test]
    fn test_create_empty_buffer() {
        let buf = Buffer::empty();
        assert_eq!(buf.len(), 0);
    }
}
