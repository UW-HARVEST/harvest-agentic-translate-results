use kairoCompiler::buffer::*;

#[test]
fn test_buffer_peek_empty() {
    let buf = buffer_create();
    // C returns -1 which is 0xFF as unsigned char
    assert_eq!(buffer_peek(&buf) as u8, 0xFF);
}

#[test]
fn test_buffer_read_empty() {
    let mut buf = buffer_create();
    assert_eq!(buffer_read(&mut buf) as u8, 0xFF);
}

#[test]
fn test_buffer_write_and_peek() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'A');
    buffer_write(&mut buf, 'B');
    buffer_write(&mut buf, 'C');
    assert_eq!(buffer_peek(&buf), 'A');
}

#[test]
fn test_buffer_read_sequence() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'A');
    buffer_write(&mut buf, 'B');
    buffer_write(&mut buf, 'C');
    assert_eq!(buffer_read(&mut buf), 'A');
    assert_eq!(buffer_read(&mut buf), 'B');
    assert_eq!(buffer_peek(&buf), 'C');
    assert_eq!(buffer_read(&mut buf), 'C');
    assert_eq!(buffer_read(&mut buf) as u8, 0xFF);
}

#[test]
fn test_buffer_printf() {
    let mut buf = buffer_create();
    buffer_printf(&mut buf, "hello");
    let ptr = buffer_ptr(&buf);
    // buffer_printf appends the string bytes + null terminator
    assert_eq!(ptr[0], b'h');
    assert_eq!(ptr[1], b'e');
    assert_eq!(ptr[2], b'l');
    assert_eq!(ptr[3], b'l');
    assert_eq!(ptr[4], b'o');
    assert_eq!(ptr[5], 0); // null terminator
}

#[test]
fn test_buffer_printf_no_terminator() {
    let mut buf = buffer_create();
    buffer_printf_no_terminator(&mut buf, "test");
    let ptr = buffer_ptr(&buf);
    assert_eq!(ptr[0], b't');
    assert_eq!(ptr[1], b'e');
    assert_eq!(ptr[2], b's');
    assert_eq!(ptr[3], b't');
}

#[test]
fn test_buffer_ptr() {
    let mut buf = buffer_create();
    buffer_write(&mut buf, 'X');
    let ptr = buffer_ptr(&buf);
    assert_eq!(ptr[0], b'X');
}

#[test]
fn test_buffer_extend() {
    let mut buf = buffer_create();
    buffer_extend(&mut buf, 5000);
    // Should not panic, just increases capacity
    buffer_write(&mut buf, 'Z');
    assert_eq!(buffer_peek(&buf), 'Z');
}

#[test]
fn test_buffer_free() {
    let buf = buffer_create();
    buffer_free(buf);
    // Should not panic
}

fn main() {}
