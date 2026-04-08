use CircularBuffer::circular_buffer::CircularBuffer;

fn main() {}

// --- Basic creation and accessors ---

#[test]
fn test_new_buffer_properties() {
    let cb = CircularBuffer::new(8);
    assert_eq!(cb.get_capacity(), 8);
    assert_eq!(cb.get_size(), 8);
    assert_eq!(cb.get_data_size(), 0);
}

// --- Sequential push/pop/read matching C test sequence ---

#[test]
fn test_push_pop_read_sequence() {
    let a = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut b = [0u8; 128];
    let mut offset = 0usize;

    let mut cb = CircularBuffer::new(8);

    // push 3 bytes "012"
    cb.push(&a[offset..offset + 3], 3);
    offset += 3;
    assert_eq!(cb.get_data_size(), 3);

    // push 7 bytes "3456789" -> fills buffer
    cb.push(&a[offset..offset + 7], 7);
    offset += 7;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 -> "234", ds=5
    b = [0u8; 128];
    let out = cb.pop(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"234");
    assert_eq!(cb.get_data_size(), 5);

    // read 2 -> "56", ds stays 5
    b = [0u8; 128];
    let out = cb.read(2, &mut b);
    assert_eq!(out, 2);
    assert_eq!(&b[..2], b"56");
    assert_eq!(cb.get_data_size(), 5);

    // push 10 bytes "abcdefghij"
    cb.push(&a[offset..offset + 10], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 -> "cde", ds=5
    b = [0u8; 128];
    let out = cb.pop(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"cde");
    assert_eq!(cb.get_data_size(), 5);

    // pop 30 -> only 5 available, "fghij", ds=0
    b = [0u8; 128];
    let out = cb.pop(30, &mut b);
    assert_eq!(out, 5);
    assert_eq!(&b[..5], b"fghij");
    assert_eq!(cb.get_data_size(), 0);

    // push 5 bytes "klmno"
    cb.push(&a[offset..offset + 5], 5);
    offset += 5;
    assert_eq!(cb.get_data_size(), 5);

    // pop 2 -> "kl", ds=3
    b = [0u8; 128];
    let out = cb.pop(2, &mut b);
    assert_eq!(out, 2);
    assert_eq!(&b[..2], b"kl");
    assert_eq!(cb.get_data_size(), 3);

    // push 10 bytes "pqrstuvwxy"
    cb.push(&a[offset..offset + 10], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 6 -> "rstuvw", ds=2
    b = [0u8; 128];
    let out = cb.pop(6, &mut b);
    assert_eq!(out, 6);
    assert_eq!(&b[..6], b"rstuvw");
    assert_eq!(cb.get_data_size(), 2);

    // push 4 bytes
    cb.push(&a[offset..offset + 4], 4);
    assert_eq!(cb.get_data_size(), 6);
}

// --- Reset ---

#[test]
fn test_reset() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"ABCD", 4);
    assert_eq!(cb.get_data_size(), 4);
    cb.reset();
    assert_eq!(cb.get_data_size(), 0);
}

// --- Pop/read from empty ---

#[test]
fn test_pop_from_empty() {
    let mut cb = CircularBuffer::new(8);
    let mut b = [0u8; 128];
    assert_eq!(cb.pop(5, &mut b), 0);
}

#[test]
fn test_read_from_empty() {
    let cb = CircularBuffer::new(8);
    let mut b = [0u8; 128];
    assert_eq!(cb.read(5, &mut b), 0);
}

// --- Push 0 bytes is no-op ---

#[test]
fn test_push_zero() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"XYZ", 0);
    assert_eq!(cb.get_data_size(), 0);
}

// --- Push exactly capacity ---

#[test]
fn test_push_exact_capacity() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"ABCDEFGH", 8);
    assert_eq!(cb.get_data_size(), 8);
    let mut b = [0u8; 128];
    let out = cb.pop(8, &mut b);
    assert_eq!(out, 8);
    assert_eq!(&b[..8], b"ABCDEFGH");
}

// --- Push more than capacity keeps last cap bytes ---

#[test]
fn test_push_over_capacity() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"0123456789AB", 12);
    assert_eq!(cb.get_data_size(), 8);
    let mut b = [0u8; 128];
    let out = cb.pop(8, &mut b);
    assert_eq!(out, 8);
    assert_eq!(&b[..8], b"456789AB");
}

// --- Read is non-destructive ---

#[test]
fn test_read_non_destructive() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"HELLO", 5);
    let mut b = [0u8; 128];

    let out = cb.read(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"HEL");
    assert_eq!(cb.get_data_size(), 5);

    b = [0u8; 128];
    let out = cb.read(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"HEL");
    assert_eq!(cb.get_data_size(), 5);
}

// --- Pop after reset returns 0 ---

#[test]
fn test_pop_after_reset() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"ABCD", 4);
    cb.reset();
    let mut b = [0u8; 128];
    assert_eq!(cb.pop(4, &mut b), 0);
    assert_eq!(cb.get_data_size(), 0);
}

// --- inter_read directly ---

#[test]
fn test_inter_read_with_reset_head() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"ABCDE", 5);
    let mut b = [0u8; 128];
    // inter_read with reset_head=true acts like pop
    let out = cb.inter_read(3, &mut b, true);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"ABC");
    assert_eq!(cb.get_data_size(), 2);
}

#[test]
fn test_inter_read_without_reset_head() {
    let mut cb = CircularBuffer::new(8);
    cb.push(b"ABCDE", 5);
    let mut b = [0u8; 128];
    // inter_read with reset_head=false acts like read
    let out = cb.inter_read(3, &mut b, false);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"ABC");
    assert_eq!(cb.get_data_size(), 5);
}

// --- free (just ensure it doesn't panic) ---

#[test]
fn test_free() {
    let cb = CircularBuffer::new(8);
    cb.free();
}
