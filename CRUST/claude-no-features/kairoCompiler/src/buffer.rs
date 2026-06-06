// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;

// Structs
#[derive(Debug, Default, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub rindex: i32,
    pub len: i32,
    pub msize: i32,
}

// Function Declarations
/// Creates a new buffer.
pub fn buffer_create() -> Buffer {
    Buffer {
        data: vec![0u8; BUFFER_REALLOC_AMOUNT],
        rindex: 0,
        len: 0,
        msize: BUFFER_REALLOC_AMOUNT as i32,
    }
}

/// Reads a character from the buffer.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\u{FFFF}'; // Sentinel for EOF (-1 as char)
    }
    let c = buffer.data[buffer.rindex as usize] as char;
    buffer.rindex += 1;
    c
}

/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\u{FFFF}';
    }
    buffer.data[buffer.rindex as usize] as char
}

/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    let new_size = (buffer.msize as usize) + size;
    buffer.data.resize(new_size, 0);
    buffer.msize = new_size as i32;
}

/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    let bytes = fmt.as_bytes();
    for &b in bytes {
        buffer_write(buffer, b as char);
    }
    // Append null terminator
    buffer_write(buffer, '\0');
}

/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    let bytes = fmt.as_bytes();
    for &b in bytes {
        buffer_write(buffer, b as char);
    }
}

/// Writes a character into the buffer.
pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer_need(buffer, 1);
    buffer.data[buffer.len as usize] = c as u8;
    buffer.len += 1;
}

/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    if (buffer.msize as usize) <= (buffer.len as usize + size) {
        let extra = size + BUFFER_REALLOC_AMOUNT;
        buffer_extend(buffer, extra);
    }
}

/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(buffer: &Buffer) -> &[u8] {
    &buffer.data
}

/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    // dropped automatically
}
