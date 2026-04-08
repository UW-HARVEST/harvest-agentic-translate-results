use carrays::circ_array::CircBuf;

fn read_i32(slice: &[u8]) -> i32 {
    i32::from_ne_bytes(slice[..4].try_into().unwrap())
}

fn write_i32(slice: &mut [u8], val: i32) {
    slice[..4].copy_from_slice(&val.to_ne_bytes());
}

#[test]
fn test_new_and_dealloc() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 4);
    // size should be roundup64(4) = 4
    assert_eq!(cb.n, 0);
    assert_eq!(cb.start, 0);
    cb.dealloc();
}

#[test]
fn test_push_pop() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 4);

    // push adds to start
    write_i32(cb.push(), 10);
    assert_eq!(cb.n, 1);
    assert_eq!(cb.start, 3);

    write_i32(cb.push(), 20);
    assert_eq!(cb.n, 2);
    assert_eq!(cb.start, 2);

    write_i32(cb.push(), 30);
    assert_eq!(cb.n, 3);
    assert_eq!(cb.start, 1);

    // pop removes from start (LIFO for push/pop)
    assert_eq!(read_i32(cb.pop()), 30);
    assert_eq!(cb.n, 2);
    assert_eq!(cb.start, 2);

    assert_eq!(read_i32(cb.pop()), 20);
    assert_eq!(read_i32(cb.pop()), 10);
    assert_eq!(cb.n, 0);

    cb.dealloc();
}

#[test]
fn test_unshift_shift() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 4);

    // unshift adds to end
    write_i32(cb.unshift(), 5);
    write_i32(cb.unshift(), 6);
    write_i32(cb.unshift(), 7);
    write_i32(cb.unshift(), 8);
    assert_eq!(cb.n, 4);

    // shift removes from end — C behavior: gets element at index n then decrements
    // After 4 unshifts of [5,6,7,8], elements at indices 0..3 are 5,6,7,8
    // shift gets index n=4 (wraps), then n becomes 3
    // From C ground truth: shift sequence is 5, 8, 7, 6
    let v0 = read_i32(cb.shift());
    let v1 = read_i32(cb.shift());
    let v2 = read_i32(cb.shift());
    let v3 = read_i32(cb.shift());
    assert_eq!([v0, v1, v2, v3], [5, 8, 7, 6]);

    cb.dealloc();
}

#[test]
fn test_push_triggers_resize() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 2);
    // size = roundup64(2) = 2

    for i in 1..=4 {
        write_i32(cb.push(), i);
    }
    assert_eq!(cb.n, 4);
    // Should have resized to 4
    assert_eq!(cb.size, 4);

    // Pop all — from C ground truth: 4,3,2,1
    let mut vals = Vec::new();
    for _ in 0..4 {
        vals.push(read_i32(cb.pop()));
    }
    assert_eq!(vals, vec![4, 3, 2, 1]);

    cb.dealloc();
}

#[test]
fn test_push_pop_interleaved() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 4);

    write_i32(cb.push(), 10);
    write_i32(cb.push(), 20);
    write_i32(cb.push(), 30);

    // pop 30 (start)
    assert_eq!(read_i32(cb.pop()), 30);

    // unshift 40 (end)
    write_i32(cb.unshift(), 40);
    assert_eq!(cb.n, 3);

    // shift removes from end — C ground truth: shift()=30
    // (C gets index n which is the old wrapped position)
    let shifted = read_i32(cb.shift());
    assert_eq!(shifted, 30);
    assert_eq!(cb.n, 2);

    // Remaining elements should be 20, 10
    let v0 = read_i32(cb.pop());
    let v1 = read_i32(cb.pop());
    assert_eq!(v0, 20);
    assert_eq!(v1, 10);

    cb.dealloc();
}

#[test]
fn test_capacity() {
    let mut cb = CircBuf::new(std::mem::size_of::<i32>(), 4);
    assert_eq!(cb.size, 4);

    cb.capacity(10);
    assert!(cb.size >= 10);
    // roundup64(10) = 16
    assert_eq!(cb.size, 16);

    // No resize if already big enough
    cb.capacity(5);
    assert_eq!(cb.size, 16);

    cb.dealloc();
}

fn main() {}
