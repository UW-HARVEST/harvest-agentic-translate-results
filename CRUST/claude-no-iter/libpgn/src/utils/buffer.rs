pub const PGN_BUFFER_INITIAL_SIZE: usize = 16;
pub const PGN_BUFFER_GROW_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnBuffer {
    buf: String,
}

impl PgnBuffer {
    /// Initializes an empty buffer with a predefined capacity (`pgn_buffer_init`)
    pub fn new() -> Self {
        PgnBuffer {
            buf: String::with_capacity(PGN_BUFFER_INITIAL_SIZE),
        }
    }

    /// Expands buffer capacity (`pgn_buffer_grow`)
    pub fn grow(&mut self) {
        self.buf.reserve(PGN_BUFFER_GROW_SIZE);
    }

    /// Clears the buffer content while keeping allocated space (`pgn_buffer_reset`)
    pub fn reset(&mut self) {
        self.buf.clear();
    }

    /// Appends a single character to the buffer (`pgn_buffer_append`)
    pub fn append(&mut self, ch: char) {
        self.buf.push(ch);
    }

    /// Appends a null terminator (C-style) (`pgn_buffer_append_null_terminator`)
    pub fn append_null_terminator(&mut self) {
        // In Rust, strings are not null-terminated. This is a no-op for safe Rust.
    }

    /// Concatenates a string to the buffer (`pgn_buffer_concat`)
    pub fn concat(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Detaches the buffer's content and returns it, while clearing the structure (`pgn_buffer_detach`)
    pub fn detach(self) -> String {
        self.buf
    }
}

impl Default for PgnBuffer {
    fn default() -> Self {
        PgnBuffer::new()
    }
}

impl PgnBuffer {
    /// Borrow the underlying string contents.
    pub fn as_str(&self) -> &str {
        &self.buf
    }

    /// Length of the buffer's content.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Equality check against a string slice (`pgn_buffer_equal`).
    pub fn equal(&self, s: &str) -> bool {
        self.buf == s
    }
}
