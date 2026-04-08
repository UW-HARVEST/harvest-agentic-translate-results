use CircularBuffer::circular_buffer::CircularBuffer;

#[test]
fn test_new_and_capacity() {
    let cb = CircularBuffer::new(8);
    assert_eq!(cb.get_capacity(), 8);
    assert_eq!(cb.get_size(), 8);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_push_basic() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_push_fills_buffer() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    cb.push(b"3456789", 7);
    assert_eq!(cb.get_data_size(), 8);
}

#[test]
fn test_pop_after_overflow() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    cb.push(b"3456789", 7);
    let mut out = [0u8; 128];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"234");
    assert_eq!(cb.get_data_size(), 5);
}

#[test]
fn test_read_does_not_consume() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    cb.push(b"3456789", 7);
    let mut out = [0u8; 128];
    cb.pop(3, &mut out);
    let mut out2 = [0u8; 128];
    let n = cb.read(2, &mut out2);
    assert_eq!(n, 2);
    assert_eq!(&out2[..2], b"56");
    assert_eq!(cb.get_data_size(), 5);
}

#[test]
fn test_full_sequence_from_c_test() {
    let a = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut cb = CircularBuffer::new(8);
    let mut out;
    let mut offset = 0usize;

    // push 3
    cb.push(&a[offset..offset + 3], 3);
    offset += 3;
    assert_eq!(cb.get_data_size(), 3);

    // push 7
    cb.push(&a[offset..offset + 7], 7);
    offset += 7;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3
    out = [0u8; 128];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"234");
    assert_eq!(cb.get_data_size(), 5);

    // read 2
    out = [0u8; 128];
    let n = cb.read(2, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"56");
    assert_eq!(cb.get_data_size(), 5);

    // push 10
    cb.push(&a[offset..offset + 10], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3
    out = [0u8; 128];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"cde");
    assert_eq!(cb.get_data_size(), 5);

    // pop 30 (only 5 available)
    out = [0u8; 128];
    let n = cb.pop(30, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"fghij");
    assert_eq!(cb.get_data_size(), 0);

    // push 5
    cb.push(&a[offset..offset + 5], 5);
    offset += 5;
    assert_eq!(cb.get_data_size(), 5);

    // pop 2
    out = [0u8; 128];
    let n = cb.pop(2, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"kl");
    assert_eq!(cb.get_data_size(), 3);

    // push 10
    cb.push(&a[offset..offset + 10], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 6
    out = [0u8; 128];
    let n = cb.pop(6, &mut out);
    assert_eq!(n, 6);
    assert_eq!(&out[..6], b"rstuvw");
    assert_eq!(cb.get_data_size(), 2);

    // push 4
    cb.push(&a[offset..offset + 4], 4);
    assert_eq!(cb.get_data_size(), 6);
}

#[test]
fn test_empty_pop_returns_zero() {
    let mut cb = CircularBuffer::new(4);
    let mut out = [0u8; 128];
    let n = cb.pop(5, &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_push_zero_length() {
    let mut cb = CircularBuffer::new(4);
    cb.push(b"hello", 0);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_push_exceeds_capacity() {
    let mut cb = CircularBuffer::new(4);
    cb.push(b"0123456789", 10);
    assert_eq!(cb.get_data_size(), 4);
    let mut out = [0u8; 128];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"6789");
}

#[test]
fn test_reset() {
    let mut cb = CircularBuffer::new(16);
    cb.push(b"01234", 5);
    assert_eq!(cb.get_data_size(), 5);
    cb.reset();
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_zero_length() {
    let mut cb = CircularBuffer::new(16);
    cb.push(b"01234", 5);
    let mut out = [0u8; 128];
    let n = cb.read(0, &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_pop_zero_length() {
    let mut cb = CircularBuffer::new(16);
    cb.push(b"01234", 5);
    let mut out = [0u8; 128];
    let n = cb.pop(0, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 5);
}

#[test]
fn test_pop_more_than_available() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"abc", 3);
    let mut out = [0u8; 128];
    let n = cb.pop(10, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"abc");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_more_than_available() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"abc", 3);
    let mut out = [0u8; 128];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"abc");
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_push_exact_capacity() {
    let mut cb = CircularBuffer::new(4);
    cb.push(b"abcd", 4);
    assert_eq!(cb.get_data_size(), 4);
    let mut out = [0u8; 128];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"abcd");
}

#[test]
fn test_multiple_wraps() {
    let mut cb = CircularBuffer::new(4);
    let mut out;

    cb.push(b"ab", 2);
    assert_eq!(cb.get_data_size(), 2);

    cb.push(b"cdef", 4);
    assert_eq!(cb.get_data_size(), 4);

    out = [0u8; 128];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"cdef");
}

#[test]
fn test_read_after_empty() {
    let cb = CircularBuffer::new(4);
    let mut out = [0u8; 128];
    let n = cb.read(1, &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_push_pop_push_pop_wrap() {
    let mut cb = CircularBuffer::new(4);
    let mut out = [0u8; 128];

    cb.push(b"ab", 2);
    let n = cb.pop(2, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"ab");
    assert_eq!(cb.get_data_size(), 0);

    cb.push(b"cdef", 4);
    assert_eq!(cb.get_data_size(), 4);
    out = [0u8; 128];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"cdef");
}

#[test]
fn test_capacity_one() {
    let mut cb = CircularBuffer::new(1);
    assert_eq!(cb.get_capacity(), 1);
    cb.push(b"x", 1);
    assert_eq!(cb.get_data_size(), 1);
    let mut out = [0u8; 4];
    let n = cb.pop(1, &mut out);
    assert_eq!(n, 1);
    assert_eq!(out[0], b'x');
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_overwrite_wraps_head() {
    let mut cb = CircularBuffer::new(4);
    cb.push(b"1234", 4);
    assert_eq!(cb.get_data_size(), 4);
    cb.push(b"56", 2);
    assert_eq!(cb.get_data_size(), 4);
    let mut out = [0u8; 128];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"3456");
}

#[test]
fn test_free_consumes() {
    let cb = CircularBuffer::new(8);
    cb.free();
    // Just verifying it compiles and doesn't panic
}

fn main() {}
