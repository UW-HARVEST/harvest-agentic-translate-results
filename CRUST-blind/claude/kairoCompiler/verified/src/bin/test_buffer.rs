use kairoCompiler::buffer::{
    buffer_create, buffer_read, buffer_peek, buffer_extend, buffer_printf,
    buffer_printf_no_terminator, buffer_write, buffer_need, buffer_ptr, buffer_free,
    BUFFER_REALLOC_AMOUNT,
};

#[test]
fn test_buffer_create_initial_state() {
    let buf = buffer_create();
    assert_eq!(buf.len, 0);
    assert_eq!(buf.msize, 2000);
    assert_eq!(buf.rindex, 0);
    assert_eq!(BUFFER_REALLOC_AMOUNT, 2000);
}

#[test]
fn test_buffer_write_and_ptr() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'h');
    buffer_write(&mut buf, 'i');
    assert_eq!(buf.len, 2);
    let data = buffer_ptr(&buf);
    assert_eq!(data.len(), 2);
    assert_eq!(data[0], b'h');
    assert_eq!(data[1], b'i');
}

#[test]
fn test_buffer_read_consumes() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'h');
    buffer_write(&mut buf, 'i');
    let c1 = buffer_read(&mut buf);
    let c2 = buffer_read(&mut buf);
    assert_eq!(c1, 'h');
    assert_eq!(c2, 'i');
    assert_eq!(buf.rindex, 2);
}

#[test]
fn test_buffer_read_eof() {
    let mut buf = buffer_create();
    let c = buffer_read(&mut buf);
    // Rust translation returns '\u{FFFF}' to represent EOF (-1 in C)
    assert_eq!(c as u32, 0xFFFF);
}

#[test]
fn test_buffer_peek_eof() {
    let buf = buffer_create();
    let c = buffer_peek(&buf);
    assert_eq!(c as u32, 0xFFFF);
}

#[test]
fn test_buffer_peek_does_not_consume() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'a');
    let p1 = buffer_peek(&buf);
    let p2 = buffer_peek(&buf);
    assert_eq!(p1, 'a');
    assert_eq!(p2, 'a');
    assert_eq!(buf.rindex, 0);
}

#[test]
fn test_buffer_peek_after_read() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'a');
    let _ = buffer_read(&mut buf);
    let p = buffer_peek(&buf);
    assert_eq!(p as u32, 0xFFFF);
}

#[test]
fn test_buffer_extend() {
    let mut buf = buffer_create();
    let original_msize = buf.msize;
    buffer_extend(&mut buf, 100);
    assert_eq!(buf.msize, original_msize + 100);
}

#[test]
fn test_buffer_printf_with_terminator() {
    let mut buf = buffer_create();
    buffer_printf(&mut buf, "abc");
    // Adds null terminator at end
    assert_eq!(buf.len, 4);
    let data = buffer_ptr(&buf);
    assert_eq!(data[0], b'a');
    assert_eq!(data[1], b'b');
    assert_eq!(data[2], b'c');
    assert_eq!(data[3], 0);
}

#[test]
fn test_buffer_printf_no_terminator() {
    let mut buf = buffer_create();
    buffer_printf_no_terminator(&mut buf, "abc");
    assert_eq!(buf.len, 3);
    let data = buffer_ptr(&buf);
    assert_eq!(data[0], b'a');
    assert_eq!(data[1], b'b');
    assert_eq!(data[2], b'c');
}

#[test]
fn test_buffer_need() {
    let mut buf = buffer_create();
    let original = buf.msize;
    // Make sure no extension when capacity is fine
    buffer_need(&mut buf, 100);
    assert_eq!(buf.msize, original);
}

#[test]
fn test_buffer_free() {
    let buf = buffer_create();
    // Should not panic
    buffer_free(buf);
}

fn main() {}
