// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;

// Structs
#[derive(Debug, Default, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub rindex: usize,
    pub len: usize,
    pub msize: usize,
}

// Function Declarations
/// Creates a new buffer.
pub fn buffer_create() -> Buffer {
    Buffer {
        data: vec![0u8; BUFFER_REALLOC_AMOUNT],
        rindex: 0,
        len: 0,
        msize: BUFFER_REALLOC_AMOUNT,
    }
}

/// Reads a character from the buffer. Returns '\u{FFFF}' (or treat as -1) on EOF.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        // Mimic returning -1 (cast to char)
        return '\u{FFFF}';
    }
    let c = buffer.data[buffer.rindex];
    buffer.rindex += 1;
    c as char
}

/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\u{FFFF}';
    }
    buffer.data[buffer.rindex] as char
}

/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.msize += size;
    buffer.data.resize(buffer.msize, 0);
}

/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    // We don't have varargs in safe Rust; we just append the bytes of `fmt`
    // and add a null terminator.
    let bytes = fmt.as_bytes();
    buffer_need(buffer, bytes.len() + 1);
    for &b in bytes {
        buffer.data[buffer.len] = b;
        buffer.len += 1;
    }
    // Null terminator
    buffer.data[buffer.len] = 0;
    buffer.len += 1;
}

/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    let bytes = fmt.as_bytes();
    buffer_need(buffer, bytes.len());
    for &b in bytes {
        buffer.data[buffer.len] = b;
        buffer.len += 1;
    }
}

/// Writes a character into the buffer.
pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer_need(buffer, 1);
    buffer.data[buffer.len] = c as u8;
    buffer.len += 1;
}

/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    if buffer.msize <= buffer.len + size {
        let extend_by = size + BUFFER_REALLOC_AMOUNT;
        buffer_extend(buffer, extend_by);
    }
}

/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(buffer: &Buffer) -> &[u8] {
    &buffer.data
}

/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    // dropped
}
