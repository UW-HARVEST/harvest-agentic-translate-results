pub const BUFFER_REALLOC_AMOUNT: usize = 2000;

#[derive(Debug, Clone)]
pub struct Buffer {
    data: Vec<u8>,
    rindex: usize,
    len: usize,
    msize: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            data: vec![0; BUFFER_REALLOC_AMOUNT],
            rindex: 0,
            len: 0,
            msize: BUFFER_REALLOC_AMOUNT,
        }
    }
}

fn eof_char() -> char {
    '\0'
}

fn char_to_byte(c: char) -> u8 {
    if c.is_ascii() {
        c as u8
    } else {
        b'?'
    }
}

pub fn buffer_create() -> Buffer {
    Buffer::default()
}

pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return eof_char();
    }

    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}

pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return eof_char();
    }

    buffer.data[buffer.rindex] as char
}

pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.msize += size;
    buffer.data.resize(buffer.msize, 0);
}

pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    if buffer.msize <= buffer.len + size {
        buffer_extend(buffer, size + BUFFER_REALLOC_AMOUNT);
    }
}

pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    buffer_need(buffer, fmt.len() + 1);
    let start = buffer.len;
    let end = start + fmt.len();
    buffer.data[start..end].copy_from_slice(fmt.as_bytes());
    buffer.data[end] = 0;
    buffer.len += fmt.len();
}

pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    buffer_need(buffer, fmt.len() + 1);
    let start = buffer.len;
    let end = start + fmt.len();
    buffer.data[start..end].copy_from_slice(fmt.as_bytes());
    buffer.data[end] = 0;
    buffer.len += fmt.len().saturating_sub(1);
}

pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer_need(buffer, 1);
    buffer.data[buffer.len] = char_to_byte(c);
    buffer.len += 1;
}

pub fn buffer_ptr(buffer: &Buffer) -> &[u8] {
    &buffer.data[..=buffer.len.min(buffer.data.len().saturating_sub(1))]
}

pub fn buffer_free(_buffer: Buffer) {}
