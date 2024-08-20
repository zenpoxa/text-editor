pub struct Buffer {
    pub lines: Vec<String>,
}

impl Default for Buffer {

    fn default() -> Self {
        Buffer {
            lines: Vec::new()
        }
    }
}

impl Buffer {
    pub fn is_empty(&self) -> bool {
        return self.lines.is_empty();
    }
}
