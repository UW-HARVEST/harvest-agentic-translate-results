// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub rindex: usize,
    pub len: usize,
}
// Function Declarations
/// Creates a new buffer.
pub fn buffer_create() -> Buffer {
    Buffer {
        data: vec![0u8; BUFFER_REALLOC_AMOUNT],
        rindex: 0,
        len: 0,
    }
}
/// Reads a character from the buffer.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return (-1i8) as u8 as char;
    }
    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}
/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return (-1i8) as u8 as char;
    }
    buffer.data[buffer.rindex] as char
}
/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.data.resize(buffer.data.len() + size, 0);
}
/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(_buffer: &mut Buffer, _fmt: &str /* varargs not directly supported in safe Rust */) {
    let bytes = _fmt.as_bytes();
    let index = _buffer.len;
    if index + bytes.len() + 1 > _buffer.data.len() {
        buffer_extend(_buffer, bytes.len() + 1 + BUFFER_REALLOC_AMOUNT);
    }
    _buffer.data[index..index + bytes.len()].copy_from_slice(bytes);
    _buffer.len += bytes.len();
    // Add null terminator like the C version's vsnprintf
    if _buffer.len < _buffer.data.len() {
        _buffer.data[_buffer.len] = 0;
    }
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(_buffer: &mut Buffer, _fmt: &str) {
    let bytes = _fmt.as_bytes();
    let index = _buffer.len;
    if index + bytes.len() + 1 > _buffer.data.len() {
        buffer_extend(_buffer, bytes.len() + 1 + BUFFER_REALLOC_AMOUNT);
    }
    _buffer.data[index..index + bytes.len()].copy_from_slice(bytes);
    // C version does len += actual_len - 1 (strips null terminator from count)
    if !bytes.is_empty() {
        _buffer.len += bytes.len() - 1;
    }
}
/// Writes a character into the buffer.
pub fn buffer_write(_buffer: &mut Buffer, _c: char) {
    buffer_need(_buffer, 1);
    if _buffer.len < _buffer.data.len() {
        _buffer.data[_buffer.len] = _c as u8;
    }
    _buffer.len += 1;
}
/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(_buffer: &mut Buffer, _size: usize) {
    if _buffer.data.len() <= _buffer.len + _size {
        let extra = _size + BUFFER_REALLOC_AMOUNT;
        buffer_extend(_buffer, extra);
    }
}
/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(_buffer: &Buffer) -> &[u8] {
    &_buffer.data[.._buffer.len]
}
/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    // drop happens automatically
}
