// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default)]
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
    buffer_need(_buffer, bytes.len() + 1);
    for &b in bytes {
        _buffer.data[_buffer.len] = b;
        _buffer.len += 1;
    }
    // null terminator
    _buffer.data[_buffer.len] = 0;
    // len includes the content but not the null terminator in the C version
    // Actually in C, buffer_printf uses vsnprintf which writes the null but actual_len doesn't include it
    // So len points past the content, and the null is at data[len]
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(_buffer: &mut Buffer, _fmt: &str) {
    let bytes = _fmt.as_bytes();
    buffer_need(_buffer, bytes.len());
    for &b in bytes {
        _buffer.data[_buffer.len] = b;
        _buffer.len += 1;
    }
    // In C: actual_len = vsnprintf(...) which includes null, then len += actual_len - 1
    // So it writes content without counting the null terminator
    // We just write the bytes without null
}
/// Writes a character into the buffer.
pub fn buffer_write(_buffer: &mut Buffer, _c: char) {
    buffer_need(_buffer, 1);
    _buffer.data[_buffer.len] = _c as u8;
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
    drop(_buffer);
}
