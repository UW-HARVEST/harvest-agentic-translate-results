use CircularBuffer::circular_buffer::CircularBuffer;

#[test]
fn test_new_and_capacity() {
    let cb = CircularBuffer::new(8);
    assert_eq!(cb.get_capacity(), 8);
    assert_eq!(cb.get_size(), 8);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_new_zero_size() {
    let cb = CircularBuffer::new(0);
    assert_eq!(cb.get_capacity(), 0);
    assert_eq!(cb.get_size(), 0);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_push_basic() {
    let mut cb = CircularBuffer::new(8);
    let data = b"012";
    cb.push(data, 3);
    assert_eq!(cb.get_data_size(), 3);
    assert_eq!(cb.get_capacity(), 8);
    assert_eq!(cb.get_size(), 8);
}

#[test]
fn test_push_zero_length_is_noop() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"hello", 0);
    assert_eq!(cb.get_data_size(), 0);
    let mut out = [0u8; 8];
    assert_eq!(cb.read(8, &mut out), 0);
}

#[test]
fn test_push_then_pop() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    let mut out = [0u8; 16];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"012");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_does_not_modify_state() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"012", 3);
    let mut out = [0u8; 16];
    let n = cb.read(2, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"01");
    assert_eq!(cb.get_data_size(), 3);

    // Read again should return same data
    let mut out2 = [0u8; 16];
    let n2 = cb.read(2, &mut out2);
    assert_eq!(n2, 2);
    assert_eq!(&out2[..2], b"01");
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_pop_more_than_available() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"abc", 3);
    let mut out = [0u8; 16];
    let n = cb.pop(30, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"abc");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_pop_empty_returns_zero() {
    let mut cb = CircularBuffer::new(8);
    let mut out = [0u8; 16];
    let n = cb.pop(5, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_empty_returns_zero() {
    let cb = CircularBuffer::new(8);
    let mut out = [0u8; 16];
    let n = cb.read(5, &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_pop_zero_length() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"abc", 3);
    let mut out = [0u8; 16];
    let n = cb.pop(0, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_read_zero_length() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"abc", 3);
    let mut out = [0u8; 16];
    let n = cb.read(0, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_push_overflow_keeps_tail() {
    // When we push more than capacity in one call, the C code keeps the
    // tail bytes (the most recent ones) and discards the head.
    let mut cb = CircularBuffer::new(4);
    cb.push(b"0123456789", 10);
    assert_eq!(cb.get_data_size(), 4);
    let mut out = [0u8; 8];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"6789");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_push_exactly_size() {
    let mut cb = CircularBuffer::new(4);
    cb.push(b"abcd", 4);
    assert_eq!(cb.get_data_size(), 4);
    let mut out = [0u8; 8];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"abcd");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_reset_clears_state() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"hello", 5);
    assert_eq!(cb.get_data_size(), 5);
    cb.reset();
    assert_eq!(cb.get_data_size(), 0);
    assert_eq!(cb.get_capacity(), 8);
    let mut out = [0u8; 16];
    assert_eq!(cb.pop(5, &mut out), 0);
}

#[test]
fn test_push_after_reset() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"hello", 5);
    cb.reset();
    cb.push(b"abc", 3);
    assert_eq!(cb.get_data_size(), 3);
    let mut out = [0u8; 16];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"abc");
}

// Mirror of c_src/tests/test.c behavior — values verified by running the C test.
#[test]
fn test_full_c_test_behavior() {
    let mut cb = CircularBuffer::new(8);
    let a = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut offset: usize = 0;
    let mut b = [0u8; 128];

    // push 3 bytes "012"
    let len = 3;
    cb.push(&a[offset..], len);
    offset += len;
    assert_eq!(cb.get_data_size(), 3);

    // push 7 bytes "3456789"
    let len = 7;
    cb.push(&a[offset..], len);
    offset += len;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 bytes, should be "234"
    let len = 3;
    b.fill(0);
    let out_len = cb.pop(len, &mut b);
    assert_eq!(out_len, 3);
    assert_eq!(&b[..3], b"234");
    assert_eq!(cb.get_data_size(), 5);

    // read 2 bytes, should be "56"
    let len = 2;
    b.fill(0);
    let out_len = cb.read(len, &mut b);
    assert_eq!(out_len, 2);
    assert_eq!(&b[..2], b"56");
    assert_eq!(cb.get_data_size(), 5);

    // push 10 bytes "abcdefghij"; only "cdefghij" survives because of trim
    let len = 10;
    cb.push(&a[offset..], len);
    offset += len;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 bytes, should be "cde"
    let len = 3;
    b.fill(0);
    let out_len = cb.pop(len, &mut b);
    assert_eq!(out_len, 3);
    assert_eq!(&b[..3], b"cde");
    assert_eq!(cb.get_data_size(), 5);

    // pop 30 bytes (only 5 available), should be "fghij"
    let len = 30;
    b.fill(0);
    let out_len = cb.pop(len, &mut b);
    assert_eq!(out_len, 5);
    assert_eq!(&b[..5], b"fghij");
    assert_eq!(cb.get_data_size(), 0);

    // push 5 bytes "klmno"
    let len = 5;
    cb.push(&a[offset..], len);
    offset += len;
    assert_eq!(cb.get_data_size(), 5);

    // pop 2 bytes, should be "kl"
    let len = 2;
    b.fill(0);
    let out_len = cb.pop(len, &mut b);
    assert_eq!(out_len, 2);
    assert_eq!(&b[..2], b"kl");
    assert_eq!(cb.get_data_size(), 3);

    // push 10 bytes "pqrstuvwxy"; only "rstuvwxy" survives
    let len = 10;
    cb.push(&a[offset..], len);
    offset += len;
    assert_eq!(cb.get_data_size(), 8);

    // pop 6 bytes, should be "rstuvw"
    let len = 6;
    b.fill(0);
    let out_len = cb.pop(len, &mut b);
    assert_eq!(out_len, 6);
    assert_eq!(&b[..6], b"rstuvw");
    assert_eq!(cb.get_data_size(), 2);

    // push 4 bytes "zABC"
    let len = 4;
    cb.push(&a[offset..], len);
    let _ = offset + len;
    assert_eq!(cb.get_data_size(), 6);

    // After all that, contents should be "wxyzABC" wait, let's verify by popping all.
    // From C trace: head=3, tail=0, buffer=['C','v','w','x','y','z','A','B'], dataSize=6
    // Pop 6 bytes should yield "xyzABC"
    let mut out = [0u8; 16];
    let n = cb.pop(6, &mut out);
    assert_eq!(n, 6);
    assert_eq!(&out[..6], b"xyzABC");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_inter_read_no_reset() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"hello", 5);
    let mut out = [0u8; 16];
    let n = cb.inter_read(5, &mut out, false);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"hello");
    assert_eq!(cb.get_data_size(), 5);
}

#[test]
fn test_inter_read_with_reset() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"hello", 5);
    let mut out = [0u8; 16];
    let n = cb.inter_read(5, &mut out, true);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"hello");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_print_does_not_panic() {
    let mut cb = CircularBuffer::new(8);
    cb.print(false);
    cb.print(true);
    cb.push(b"hi", 2);
    cb.print(false);
    cb.print(true);
}

#[test]
fn test_free_consumes_buffer() {
    let cb = CircularBuffer::new(4);
    cb.free();
}

#[test]
fn test_wraparound_after_pop() {
    // Push, pop, then push enough to wrap around the internal buffer.
    let mut cb = CircularBuffer::new(8);
    cb.push(b"01234567", 8);
    assert_eq!(cb.get_data_size(), 8);
    let mut out = [0u8; 16];
    // pop 5 → head moves from 0 to 5, dataSize = 3
    assert_eq!(cb.pop(5, &mut out), 5);
    assert_eq!(&out[..5], b"01234");
    assert_eq!(cb.get_data_size(), 3);

    // push 4 more bytes "ABCD" → tail wraps to 3
    cb.push(b"ABCD", 4);
    assert_eq!(cb.get_data_size(), 7);

    // pop all 7 → should be "567ABCD"
    out.fill(0);
    assert_eq!(cb.pop(7, &mut out), 7);
    assert_eq!(&out[..7], b"567ABCD");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_across_wrap() {
    // Cause an internal wrap then read past the end-of-buffer boundary.
    let mut cb = CircularBuffer::new(8);
    cb.push(b"01234567", 8); // head=0, tail=7
    let mut out = [0u8; 16];
    assert_eq!(cb.pop(5, &mut out), 5); // head=5, tail=7, dataSize=3
    cb.push(b"ABCD", 4); // wraps; head=5, tail=3, dataSize=7

    // Non-destructive read across boundary
    out.fill(0);
    let n = cb.read(7, &mut out);
    assert_eq!(n, 7);
    assert_eq!(&out[..7], b"567ABCD");
    // unchanged
    assert_eq!(cb.get_data_size(), 7);

    // Subsequent pop yields the same bytes
    out.fill(0);
    let n = cb.pop(7, &mut out);
    assert_eq!(n, 7);
    assert_eq!(&out[..7], b"567ABCD");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_pop_more_than_available_when_wrapped() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"01234567", 8);
    let mut out = [0u8; 16];
    cb.pop(5, &mut out); // head=5, dataSize=3
    cb.push(b"ABCD", 4); // wrap, dataSize=7

    out.fill(0);
    let n = cb.pop(100, &mut out);
    assert_eq!(n, 7);
    assert_eq!(&out[..7], b"567ABCD");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_push_overflow_trims_to_capacity_keeping_tail_bytes() {
    // Single push larger than capacity keeps the last `size` bytes.
    let mut cb = CircularBuffer::new(5);
    cb.push(b"ABCDEFGHIJ", 10);
    assert_eq!(cb.get_data_size(), 5);
    let mut out = [0u8; 16];
    let n = cb.pop(5, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"FGHIJ");
}

fn main() {}
