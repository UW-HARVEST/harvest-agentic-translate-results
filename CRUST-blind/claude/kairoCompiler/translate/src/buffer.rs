// Constants
pub const BUFFER_REALLOC_AMOUNT: usize = 2000;
// Structs
#[derive(Debug, Default)]
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
/// Reads a character from the buffer.
pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        // -1 in C, here represent as 0xFF -> as char that's not perfect
        // But matching C behaviour we return char that comes from -1 truncated to char
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
    let c = buffer.data[buffer.rindex];
    c as char
}
/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    let new_size = buffer.msize + size;
    buffer.data.resize(new_size, 0u8);
    buffer.msize = new_size;
}
/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    // We don't really support varargs format. Just append fmt characters and a null terminator.
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
    buffer_need(buffer, std::mem::size_of::<u8>());
    if buffer.len >= buffer.data.len() {
        buffer.data.resize(buffer.len + 1, 0u8);
    }
    buffer.data[buffer.len] = c as u8;
    buffer.len += 1;
}
/// Ensures the buffer has capacity for at least `size` more characters.
pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    if buffer.msize <= buffer.len + size {
        let extra = size + BUFFER_REALLOC_AMOUNT;
        buffer_extend(buffer, extra);
    }
}
/// Obtains an internal reference to the buffer's data.
pub fn buffer_ptr(buffer: &Buffer) -> &[u8] {
    &buffer.data[..buffer.len]
}
/// Frees the buffer's resources (in Rust, this typically happens automatically when dropped).
pub fn buffer_free(_buffer: Buffer) {
    // Drop happens automatically.
}
