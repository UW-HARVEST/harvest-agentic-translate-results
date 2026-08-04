pub const BUFFER_REALLOC_AMOUNT: usize = 2000;

#[derive(Debug, Default, Clone)]
pub struct Buffer {
    data: Vec<u8>,
    rindex: usize,
    len: usize,
    msize: usize,
}

pub fn buffer_create() -> Buffer {
    Buffer {
        data: vec![0; BUFFER_REALLOC_AMOUNT],
        rindex: 0,
        len: 0,
        msize: BUFFER_REALLOC_AMOUNT,
    }
}

pub fn buffer_read(buffer: &mut Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\0';
    }

    let c = buffer.data[buffer.rindex] as char;
    buffer.rindex += 1;
    c
}

pub fn buffer_peek(buffer: &Buffer) -> char {
    if buffer.rindex >= buffer.len {
        return '\0';
    }

    buffer.data[buffer.rindex] as char
}

pub fn buffer_extend(buffer: &mut Buffer, size: usize) {
    buffer.data.resize(buffer.msize + size, 0);
    buffer.msize += size;
}

pub fn buffer_printf(buffer: &mut Buffer, fmt: &str) {
    let bytes = fmt.as_bytes();
    buffer_extend(buffer, 2048);
    let start = buffer.len;
    let end = start + bytes.len() + 1;
    if end > buffer.data.len() {
        buffer.data.resize(end, 0);
        buffer.msize = buffer.data.len();
    }
    buffer.data[start..start + bytes.len()].copy_from_slice(bytes);
    buffer.data[start + bytes.len()] = 0;
    buffer.len += bytes.len();
}

pub fn buffer_printf_no_terminator(buffer: &mut Buffer, fmt: &str) {
    let bytes = fmt.as_bytes();
    buffer_extend(buffer, 2048);
    let start = buffer.len;
    let end = start + bytes.len() + 1;
    if end > buffer.data.len() {
        buffer.data.resize(end, 0);
        buffer.msize = buffer.data.len();
    }
    buffer.data[start..start + bytes.len()].copy_from_slice(bytes);
    buffer.data[start + bytes.len()] = 0;
    buffer.len += bytes.len().saturating_sub(1);
}

pub fn buffer_write(buffer: &mut Buffer, c: char) {
    buffer_need(buffer, 1);
    if buffer.len >= buffer.data.len() {
        buffer.data.resize(buffer.len + 1, 0);
        buffer.msize = buffer.data.len();
    }
    buffer.data[buffer.len] = c as u8;
    buffer.len += 1;
}

pub fn buffer_need(buffer: &mut Buffer, size: usize) {
    if buffer.msize <= buffer.len + size {
        buffer_extend(buffer, size + BUFFER_REALLOC_AMOUNT);
    }
}

pub fn buffer_ptr(buffer: &Buffer) -> &[u8] {
    &buffer.data[..buffer.len.min(buffer.data.len())]
}

pub fn buffer_free(_buffer: Buffer) {}
