// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default)]
pub struct Buffer {
    data: Vec<u8>,
    rindex: usize,
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
        return (-1i8) as u8 as char;
    }
    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}
/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.data.len() {
        return (-1i8) as u8 as char;
    }
    buffer.data[buffer.rindex] as char
}
/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.data.reserve(size);
}
/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(_buffer: &mut Buffer, _fmt: &str /* varargs not directly supported in safe Rust */) {
    _buffer.data.extend_from_slice(_fmt.as_bytes());
    _buffer.data.push(0);
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(_buffer: &mut Buffer, _fmt: &str) {
    _buffer.data.extend_from_slice(_fmt.as_bytes());
}
/// Writes a character into the buffer.
pub fn buffer_write(_buffer: &mut Buffer, _c: char) {
    _buffer.data.push(_c as u8);
}
/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(_buffer: &mut Buffer, _size: usize) {
    if _buffer.data.capacity() <= _buffer.data.len() + _size {
        buffer_extend(_buffer, _size + BUFFER_REALLOC_AMOUNT);
    }
}
/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(_buffer: &Buffer) -> &[u8] {
    &_buffer.data
}
/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    // drop
}
