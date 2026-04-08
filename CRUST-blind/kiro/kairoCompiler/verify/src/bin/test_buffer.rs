use kairoCompiler::buffer::*;

#[test]
fn test_buffer_empty_read_returns_0xff() {
    let mut buf = buffer_create();
    // C returns -1 as char, which is 0xFF unsigned
    assert_eq!(buffer_read(&mut buf), 0xFF as char);
}

#[test]
fn test_buffer_empty_peek_returns_0xff() {
    let buf = buffer_create();
    assert_eq!(buffer_peek(&buf), 0xFF as char);
}

#[test]
fn test_buffer_write_and_len() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'A');
    buffer_write(&mut buf, 'B');
    buffer_write(&mut buf, 'C');
    assert_eq!(buf.len, 3);
}

#[test]
fn test_buffer_peek_does_not_advance() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'A');
    assert_eq!(buffer_peek(&buf), 'A');
    assert_eq!(buffer_peek(&buf), 'A');
}

#[test]
fn test_buffer_read_advances() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'A');
    buffer_write(&mut buf, 'B');
    buffer_write(&mut buf, 'C');
    assert_eq!(buffer_read(&mut buf), 'A');
    assert_eq!(buffer_read(&mut buf), 'B');
    assert_eq!(buffer_peek(&buf), 'C');
    assert_eq!(buffer_read(&mut buf), 'C');
    assert_eq!(buffer_read(&mut buf), 0xFF as char);
}

#[test]
fn test_buffer_printf() {
    let mut buf = buffer_create();
    buffer_printf(&mut buf, "hello world");
    assert_eq!(buf.len, 11);
    assert_eq!(&buf.data[..11], b"hello world");
}

#[test]
fn test_buffer_printf_no_terminator() {
    let mut buf = buffer_create();
    buffer_printf_no_terminator(&mut buf, "test");
    // C: len += actual_len - 1, so "test" (4 chars) -> len = 3
    assert_eq!(buf.len, 3);
}

#[test]
fn test_buffer_ptr() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'X');
    buffer_write(&mut buf, 'Y');
    let ptr = buffer_ptr(&buf);
    assert_eq!(ptr, b"XY");
}

#[test]
fn test_buffer_extend() {
    let mut buf = buffer_create();
    let old_len = buf.data.len();
    buffer_extend(&mut buf, 100);
    assert_eq!(buf.data.len(), old_len + 100);
}

fn main() {}
