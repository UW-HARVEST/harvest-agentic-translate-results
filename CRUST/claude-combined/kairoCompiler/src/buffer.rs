// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub rindex: usize,
}
// Function Declarations
/// Creates a new buffer.
pub fn buffer_create() -> Buffer {
    Buffer {
        data: Vec::with_capacity(BUFFER_REALLOC_AMOUNT),
        rindex: 0,
    }
}
/// Reads a character from the buffer.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.data.len() {
        return '\u{FFFF}'; // sentinel for EOF (-1)
    }
    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}
/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.data.len() {
        return '\u{FFFF}';
    }
    buffer.data[buffer.rindex] as char
}
/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.data.reserve(size);
}
/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    for b in fmt.bytes() {
        buffer.data.push(b);
    }
    buffer.data.push(0);
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    for b in fmt.bytes() {
        buffer.data.push(b);
    }
}
/// Writes a character into the buffer.
pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer.data.push(c as u8);
}
/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    let needed = buffer.data.len().saturating_add(size);
    if needed > buffer.data.capacity() {
        buffer.data.reserve(size + BUFFER_REALLOC_AMOUNT);
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
