// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default, Clone)]
pub struct Buffer {
    pub(crate) data: Vec<u8>,
    pub(crate) rindex: usize,
    pub(crate) len: usize,
    pub(crate) msize: usize,
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
/// Reads a character from the buffer.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        // -1 cast to char in C is 0xFF; but here we'll mimic EOF as char 0xFF
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
pub fn buffer_printf(_buffer: &mut Buffer, _fmt: &str /* varargs not directly supported in safe Rust */) {
    // Without varargs we treat _fmt as the literal string and append a NUL terminator.
    let bytes = _fmt.as_bytes();
    buffer_need(_buffer, bytes.len() + 1);
    for &b in bytes {
        _buffer.data[_buffer.len] = b;
        _buffer.len += 1;
    }
    // null terminator
    _buffer.data[_buffer.len] = 0;
    _buffer.len += 1;
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(_buffer: &mut Buffer, _fmt: &str) {
    let bytes = _fmt.as_bytes();
    buffer_need(_buffer, bytes.len());
    for &b in bytes {
        _buffer.data[_buffer.len] = b;
        _buffer.len += 1;
    }
}
/// Writes a character into the buffer.
pub fn buffer_write(_buffer: &mut Buffer, _c: char) {
    buffer_need(_buffer, 1);
    _buffer.data[_buffer.len] = _c as u8;
    _buffer.len += 1;
}
/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(_buffer: &mut Buffer, _size: usize) {
    if _buffer.msize <= _buffer.len + _size {
        let extra = _size + BUFFER_REALLOC_AMOUNT;
        buffer_extend(_buffer, extra);
    }
}
/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(_buffer: &Buffer) -> &[u8] {
    &_buffer.data[..]
}
/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    drop(_buffer);
}
