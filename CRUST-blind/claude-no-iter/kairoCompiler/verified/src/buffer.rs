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
        // Sentinel for EOF — using '\u{FF}' to mirror C's signed char -1.
        return '\u{FF}';
    }
    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}
/// Peeks at a character from the buffer without consuming it.
pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\u{FF}';
    }
    buffer.data[buffer.rindex] as char
}
/// Extends the buffer by a given size.
pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.msize += size;
    buffer.data.resize(buffer.msize, 0);
}
/// Prints into the buffer using a format string, appending a terminator.
pub fn buffer_printf(buffer: &mut Buffer, fmt: &str /* varargs not directly supported in safe Rust */) {
    buffer_extend(buffer, 2048);
    let bytes = fmt.as_bytes();
    let dst_start = buffer.len;
    let copy_len = bytes.len();
    if dst_start + copy_len + 1 > buffer.data.len() {
        buffer.data.resize(dst_start + copy_len + 1, 0);
    }
    buffer.data[dst_start..dst_start + copy_len].copy_from_slice(bytes);
    buffer.data[dst_start + copy_len] = 0;
    buffer.len += copy_len + 1;
}
/// Prints into the buffer without appending a terminator.
pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    buffer_extend(buffer, 2048);
    let bytes = fmt.as_bytes();
    let dst_start = buffer.len;
    let copy_len = bytes.len();
    if dst_start + copy_len > buffer.data.len() {
        buffer.data.resize(dst_start + copy_len, 0);
    }
    buffer.data[dst_start..dst_start + copy_len].copy_from_slice(bytes);
    buffer.len += copy_len;
}
/// Writes a character into the buffer.
pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer_need(buffer, 1);
    if buffer.len >= buffer.data.len() {
        buffer.data.resize(buffer.len + 1, 0);
    }
    buffer.data[buffer.len] = c as u32 as u8;
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
    // Drop runs automatically when the buffer goes out of scope.
}
