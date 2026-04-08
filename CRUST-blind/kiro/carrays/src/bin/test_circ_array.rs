use carrays::circ_array::CircBuf;

fn write_u32(slot: &mut [u8], val: u32) {
    slot[..4].copy_from_slice(&val.to_ne_bytes());
}

fn read_u32(slot: &[u8]) -> u32 {
    u32::from_ne_bytes(slot[..4].try_into().unwrap())
}

// --- new / dealloc ---

#[test]
fn test_new_and_dealloc() {
    let mut cb = CircBuf::new(4, 8);
    cb.dealloc();
}

#[test]
fn test_new_rounds_up_size() {
    let mut cb = CircBuf::new(4, 5);
    // 5 rounds up to 8
    cb.dealloc();
}

// --- push / pop (add to start, remove from start) ---

#[test]
fn test_push_pop_single() {
    let mut cb = CircBuf::new(4, 4);
    write_u32(cb.push(), 42);
    assert_eq!(read_u32(cb.pop()), 42);
    cb.dealloc();
}

#[test]
fn test_push_pop_lifo() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.push(), 1);
    write_u32(cb.push(), 2);
    write_u32(cb.push(), 3);
    // pop removes from start (LIFO): 3, 2, 1
    assert_eq!(read_u32(cb.pop()), 3);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 1);
    cb.dealloc();
}

// --- unshift / shift ---
// Note: C circa_shift returns circa_get(l, l->n) BEFORE decrementing n,
// which is one past the last element. So shift() does NOT return the
// last-pushed data. We test shift as a "remove from end" operation
// and verify via pop that the correct elements remain.

#[test]
fn test_unshift_then_shift_removes_from_end() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.unshift(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.unshift(), 3);
    // shift removes from end (but returns slot past end per C behavior)
    cb.shift();
    cb.shift();
    // one element remains: 1
    assert_eq!(read_u32(cb.pop()), 1);
    cb.dealloc();
}

#[test]
fn test_unshift_pop_queue() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.unshift(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.unshift(), 3);
    // pop removes from start: 1, 2, 3
    assert_eq!(read_u32(cb.pop()), 1);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 3);
    cb.dealloc();
}

// --- push + shift (add to start, remove from end) ---

#[test]
fn test_push_shift_removes_from_end() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.push(), 1);
    write_u32(cb.push(), 2);
    write_u32(cb.push(), 3);
    // logical order from start: 3, 2, 1
    // shift removes from end; verify remaining via pop
    cb.shift(); // removes last (1)
    cb.shift(); // removes last (2)
    assert_eq!(read_u32(cb.pop()), 3);
    cb.dealloc();
}

// --- push zeroes the slot ---

#[test]
fn test_push_zeroes_slot() {
    let mut cb = CircBuf::new(4, 4);
    let slot = cb.push();
    assert_eq!(slot, &[0, 0, 0, 0]);
    write_u32(slot, 0xFFFFFFFF);
    cb.pop();
    let slot = cb.push();
    assert_eq!(slot, &[0, 0, 0, 0]);
    cb.dealloc();
}

// --- unshift zeroes the slot ---

#[test]
fn test_unshift_zeroes_slot() {
    let mut cb = CircBuf::new(4, 4);
    let slot = cb.unshift();
    assert_eq!(slot, &[0, 0, 0, 0]);
    write_u32(slot, 0xFFFFFFFF);
    cb.shift();
    let slot = cb.unshift();
    assert_eq!(slot, &[0, 0, 0, 0]);
    cb.dealloc();
}

// --- auto-resize on push when full ---

#[test]
fn test_push_auto_resize() {
    let mut cb = CircBuf::new(4, 2);
    write_u32(cb.push(), 1);
    write_u32(cb.push(), 2);
    write_u32(cb.push(), 3);
    write_u32(cb.push(), 4);
    // pop (LIFO from start): 4, 3, 2, 1
    assert_eq!(read_u32(cb.pop()), 4);
    assert_eq!(read_u32(cb.pop()), 3);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 1);
    cb.dealloc();
}

// --- auto-resize on unshift when full ---

#[test]
fn test_unshift_auto_resize() {
    let mut cb = CircBuf::new(4, 2);
    write_u32(cb.unshift(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.unshift(), 3);
    write_u32(cb.unshift(), 4);
    // pop from start: 1, 2, 3, 4
    assert_eq!(read_u32(cb.pop()), 1);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 3);
    assert_eq!(read_u32(cb.pop()), 4);
    cb.dealloc();
}

// --- capacity ---

#[test]
fn test_capacity_grow() {
    let mut cb = CircBuf::new(4, 2);
    cb.capacity(16);
    for i in 0..16u32 {
        write_u32(cb.push(), i);
    }
    for i in (0..16u32).rev() {
        assert_eq!(read_u32(cb.pop()), i);
    }
    cb.dealloc();
}

#[test]
fn test_capacity_no_shrink() {
    let mut cb = CircBuf::new(4, 16);
    cb.capacity(4);
    write_u32(cb.push(), 1);
    assert_eq!(read_u32(cb.pop()), 1);
    cb.dealloc();
}

// --- norm ---

#[test]
fn test_norm_no_wrap() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.unshift(), 10);
    write_u32(cb.unshift(), 20);
    cb.norm();
    assert_eq!(read_u32(cb.pop()), 10);
    assert_eq!(read_u32(cb.pop()), 20);
    cb.dealloc();
}

#[test]
fn test_norm_with_wrap() {
    let mut cb = CircBuf::new(4, 4);
    write_u32(cb.unshift(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.unshift(), 3);
    // pop from start to make room, then push to wrap
    cb.pop();
    write_u32(cb.push(), 10);
    write_u32(cb.push(), 20);
    cb.norm();
    // pop from start: 20, 10, 2, 3
    assert_eq!(read_u32(cb.pop()), 20);
    assert_eq!(read_u32(cb.pop()), 10);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 3);
    cb.dealloc();
}

// --- mixed operations ---

#[test]
fn test_mixed_push_unshift_pop() {
    let mut cb = CircBuf::new(4, 8);
    // push(1), unshift(2), push(3), unshift(4)
    // Logical order from start: 3, 1, 2, 4
    write_u32(cb.push(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.push(), 3);
    write_u32(cb.unshift(), 4);
    // pop all from start: 3, 1, 2, 4
    assert_eq!(read_u32(cb.pop()), 3);
    assert_eq!(read_u32(cb.pop()), 1);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 4);
    cb.dealloc();
}

#[test]
fn test_mixed_with_shift() {
    let mut cb = CircBuf::new(4, 8);
    write_u32(cb.push(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.push(), 3);
    write_u32(cb.unshift(), 4);
    // Logical: 3, 1, 2, 4
    // pop from start: 3
    assert_eq!(read_u32(cb.pop()), 3);
    // shift from end (removes 4)
    cb.shift();
    // remaining: 1, 2
    assert_eq!(read_u32(cb.pop()), 1);
    assert_eq!(read_u32(cb.pop()), 2);
    cb.dealloc();
}

// --- resize preserves data ---

#[test]
fn test_resize_preserves_data() {
    let mut cb = CircBuf::new(4, 4);
    write_u32(cb.unshift(), 1);
    write_u32(cb.unshift(), 2);
    write_u32(cb.unshift(), 3);
    write_u32(cb.unshift(), 4);
    // triggers resize
    write_u32(cb.unshift(), 5);
    assert_eq!(read_u32(cb.pop()), 1);
    assert_eq!(read_u32(cb.pop()), 2);
    assert_eq!(read_u32(cb.pop()), 3);
    assert_eq!(read_u32(cb.pop()), 4);
    assert_eq!(read_u32(cb.pop()), 5);
    cb.dealloc();
}

// --- large element size ---

#[test]
fn test_large_element_size() {
    let mut cb = CircBuf::new(8, 4);
    let slot = cb.push();
    slot[..8].copy_from_slice(&100u64.to_ne_bytes());
    let slot = cb.unshift();
    slot[..8].copy_from_slice(&200u64.to_ne_bytes());
    // pop from start gets the push'd value
    let val = u64::from_ne_bytes(cb.pop()[..8].try_into().unwrap());
    assert_eq!(val, 100);
    // pop the unshift'd value
    let val = u64::from_ne_bytes(cb.pop()[..8].try_into().unwrap());
    assert_eq!(val, 200);
    cb.dealloc();
}

fn main() {}
