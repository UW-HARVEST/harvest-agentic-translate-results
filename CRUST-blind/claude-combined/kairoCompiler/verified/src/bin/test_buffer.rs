use kairoCompiler::buffer::{
    buffer_create, buffer_extend, buffer_peek, buffer_ptr, buffer_read, buffer_write,
    BUFFER_REALLOC_AMOUNT,
};

#[test]
fn test_buffer_create_initial_state() {
    let b = buffer_create();
    assert_eq!(b.len, 0);
    assert_eq!(b.msize, BUFFER_REALLOC_AMOUNT as i32);
    assert_eq!(b.rindex, 0);
    assert_eq!(b.data.len(), BUFFER_REALLOC_AMOUNT);
}

#[test]
fn test_buffer_write_and_read() {
    let mut b = buffer_create();
    buffer_write(&mut b, 'H');
    buffer_write(&mut b, 'i');
    assert_eq!(b.len, 2);
    assert_eq!(b.data[0], b'H');
    assert_eq!(b.data[1], b'i');

    let r1 = buffer_read(&mut b);
    let r2 = buffer_read(&mut b);
    let r3 = buffer_read(&mut b);
    assert_eq!(r1, 'H');
    assert_eq!(r2, 'i');
    // EOF sentinel
    assert_eq!(r3 as u8 as i8, -1i8);
}

#[test]
fn test_buffer_peek_empty() {
    let b = buffer_create();
    let p = buffer_peek(&b);
    assert_eq!(p as u8 as i8, -1i8);
}

#[test]
fn test_buffer_peek_one() {
    let mut b = buffer_create();
    buffer_write(&mut b, 'x');
    let p = buffer_peek(&b);
    assert_eq!(p, 'x');
    // peek does not consume
    let p2 = buffer_peek(&b);
    assert_eq!(p2, 'x');
}

#[test]
fn test_buffer_extend() {
    let mut b = buffer_create();
    let initial_msize = b.msize;
    buffer_extend(&mut b, 500);
    assert_eq!(b.msize, initial_msize + 500);
}

#[test]
fn test_buffer_ptr_returns_data() {
    let mut b = buffer_create();
    buffer_write(&mut b, 'A');
    buffer_write(&mut b, 'B');
    let p = buffer_ptr(&b);
    assert_eq!(p[0], b'A');
    assert_eq!(p[1], b'B');
}

fn main() {}
