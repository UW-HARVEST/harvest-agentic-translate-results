#![allow(unused_imports)]
use carrays::circ_array::CircBuf;

// CircBuf C semantics:
//   push    : add to FRONT (zeroed slot)
//   pop     : remove from FRONT
//   unshift : add to END (zeroed slot)
//   shift   : remove from END  (NB: returned ptr is one-past-end in C; we don't
//             assert on its bytes, only on the resulting buffer state)

#[test]
fn test_new_does_not_panic() {
    let _b = CircBuf::new(4, 1);
    let _b2 = CircBuf::new(4, 8);
    let _b3 = CircBuf::new(1, 16);
}

#[test]
fn test_push_returns_zeroed_slot_of_size_el() {
    let mut b = CircBuf::new(4, 4);
    let slot = b.push();
    assert_eq!(slot.len(), 4);
    assert_eq!(slot, &[0u8, 0, 0, 0]);
}

#[test]
fn test_unshift_returns_zeroed_slot_of_size_el() {
    let mut b = CircBuf::new(4, 4);
    let slot = b.unshift();
    assert_eq!(slot.len(), 4);
    assert_eq!(slot, &[0u8, 0, 0, 0]);
}

#[test]
fn test_push_then_pop_returns_pushed_value() {
    let mut b = CircBuf::new(4, 4);
    {
        let slot = b.push();
        slot.copy_from_slice(&[1u8, 2, 3, 4]);
    }
    let popped = b.pop();
    assert_eq!(popped.len(), 4);
    assert_eq!(popped, &[1u8, 2, 3, 4]);
}

#[test]
fn test_unshift_then_pop_returns_unshifted_value() {
    let mut b = CircBuf::new(4, 4);
    {
        let slot = b.unshift();
        slot.copy_from_slice(&[9u8, 8, 7, 6]);
    }
    // unshift puts item at end; with only 1 item, front == back. pop removes
    // from front, which is the single item.
    let popped = b.pop();
    assert_eq!(popped, &[9u8, 8, 7, 6]);
}

#[test]
fn test_push_order_lifo_via_pop() {
    // push adds to front; pop removes from front -> LIFO
    let mut b = CircBuf::new(1, 4);
    {
        b.push().copy_from_slice(&[10u8]);
    }
    {
        b.push().copy_from_slice(&[20u8]);
    }
    {
        b.push().copy_from_slice(&[30u8]);
    }
    // pop should return last pushed first
    assert_eq!(b.pop(), &[30u8]);
    assert_eq!(b.pop(), &[20u8]);
    assert_eq!(b.pop(), &[10u8]);
}

#[test]
fn test_unshift_order_fifo_via_pop() {
    // unshift adds to end; pop removes from front -> FIFO
    let mut b = CircBuf::new(1, 4);
    {
        b.unshift().copy_from_slice(&[10u8]);
    }
    {
        b.unshift().copy_from_slice(&[20u8]);
    }
    {
        b.unshift().copy_from_slice(&[30u8]);
    }
    // pop returns front, which is first unshifted
    assert_eq!(b.pop(), &[10u8]);
    assert_eq!(b.pop(), &[20u8]);
    assert_eq!(b.pop(), &[30u8]);
}

#[test]
fn test_mixed_push_unshift_pop_order() {
    // After push(A), unshift(B), unshift(C), push(D):
    //   front -> [D, A, B, C] <- back
    // pop should yield D, A, B, C
    let mut b = CircBuf::new(1, 4);
    {
        b.push().copy_from_slice(&[b'A']);
    }
    {
        b.unshift().copy_from_slice(&[b'B']);
    }
    {
        b.unshift().copy_from_slice(&[b'C']);
    }
    {
        b.push().copy_from_slice(&[b'D']);
    }
    assert_eq!(b.pop(), &[b'D']);
    assert_eq!(b.pop(), &[b'A']);
    assert_eq!(b.pop(), &[b'B']);
    assert_eq!(b.pop(), &[b'C']);
}

#[test]
fn test_unshift_grows_when_full() {
    // Initial size 2: unshift 5 items -> must grow.
    let mut b = CircBuf::new(1, 2);
    for i in 0u8..5 {
        b.unshift().copy_from_slice(&[i]);
    }
    // pop from front => 0,1,2,3,4
    for i in 0u8..5 {
        let v = b.pop();
        assert_eq!(v, &[i], "iter {}", i);
    }
}

#[test]
fn test_push_grows_when_full() {
    // Initial size 2: push 5 items -> must grow.
    let mut b = CircBuf::new(1, 2);
    for i in 0u8..5 {
        b.push().copy_from_slice(&[i]);
    }
    // pop from front => 4,3,2,1,0 (LIFO)
    for i in (0u8..5).rev() {
        let v = b.pop();
        assert_eq!(v, &[i], "iter {}", i);
    }
}

#[test]
fn test_capacity_grow() {
    // capacity should grow buffer such that subsequent unshift up to that
    // capacity does not require an additional resize. Behavior is
    // observable: after capacity, we should be able to unshift many items
    // and still recover them in order.
    let mut b = CircBuf::new(2, 1);
    b.capacity(10);
    for i in 0u8..10 {
        b.unshift().copy_from_slice(&[i, i.wrapping_add(100)]);
    }
    for i in 0u8..10 {
        let v = b.pop();
        assert_eq!(v, &[i, i.wrapping_add(100)]);
    }
}

#[test]
fn test_capacity_no_op_when_smaller() {
    let mut b = CircBuf::new(1, 8);
    // capacity smaller than current should be a no-op; existing items remain
    {
        b.unshift().copy_from_slice(&[42u8]);
    }
    b.capacity(4);
    let v = b.pop();
    assert_eq!(v, &[42u8]);
}

#[test]
fn test_resize_grows_to_power_of_two() {
    // resize requires the new size to be greater than current size and a
    // power of two. Verify it grows successfully and preserves data.
    let mut b = CircBuf::new(1, 2);
    {
        b.unshift().copy_from_slice(&[1u8]);
    }
    {
        b.unshift().copy_from_slice(&[2u8]);
    }
    b.resize(8);
    {
        b.unshift().copy_from_slice(&[3u8]);
    }
    {
        b.unshift().copy_from_slice(&[4u8]);
    }
    assert_eq!(b.pop(), &[1u8]);
    assert_eq!(b.pop(), &[2u8]);
    assert_eq!(b.pop(), &[3u8]);
    assert_eq!(b.pop(), &[4u8]);
}

#[test]
fn test_resize_with_wrapped_buffer_preserves_order() {
    // Force a wrap: alloc size=2, push then pop to advance start, then
    // unshift to wrap. Then resize and verify order.
    let mut b = CircBuf::new(1, 2);
    // After alloc: start=0, n=0
    {
        b.unshift().copy_from_slice(&[10u8]); // start=0, n=1; pos 0
    }
    {
        b.unshift().copy_from_slice(&[20u8]); // start=0, n=2; pos 1
    }
    // Now buffer is full. pop one to advance start.
    let p = b.pop();
    assert_eq!(p, &[10u8]); // start advances to 1
    // Now unshift another to wrap (the new item goes to position 0)
    {
        b.unshift().copy_from_slice(&[30u8]);
    }
    // State: items in order from front: [20, 30] (wrapped)
    // Now resize via direct call to grow capacity
    b.resize(4);
    // Order should be preserved.
    assert_eq!(b.pop(), &[20u8]);
    assert_eq!(b.pop(), &[30u8]);
}

#[test]
fn test_norm_no_wrap_no_op() {
    // If buffer is not wrapping, norm should not break it.
    let mut b = CircBuf::new(1, 8);
    for i in 0u8..4 {
        b.unshift().copy_from_slice(&[i]);
    }
    b.norm();
    for i in 0u8..4 {
        assert_eq!(b.pop(), &[i]);
    }
}

#[test]
fn test_norm_wrapped_buffer_preserves_order() {
    // Force wrap, norm, then pop and verify order.
    let mut b = CircBuf::new(1, 4);
    // Fill 4: start=0, items at pos 0,1,2,3
    for i in 0u8..4 {
        b.unshift().copy_from_slice(&[i + 1]);
    }
    // pop 3: start advances to 3
    assert_eq!(b.pop(), &[1]);
    assert_eq!(b.pop(), &[2]);
    assert_eq!(b.pop(), &[3]);
    // n=1, start=3. Now unshift 2: wraps to pos 0, then pos 1
    b.unshift().copy_from_slice(&[10]);
    b.unshift().copy_from_slice(&[20]);
    // Items order from front: [4, 10, 20] (wrapped)
    b.norm();
    assert_eq!(b.pop(), &[4]);
    assert_eq!(b.pop(), &[10]);
    assert_eq!(b.pop(), &[20]);
}

#[test]
fn test_multi_byte_element_size() {
    // Use 8-byte elements (simulating u64) and verify push/pop preserve all bytes.
    let mut b = CircBuf::new(8, 4);
    let data1: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let data2: [u8; 8] = [9, 10, 11, 12, 13, 14, 15, 16];
    let data3: [u8; 8] = [17, 18, 19, 20, 21, 22, 23, 24];
    {
        b.unshift().copy_from_slice(&data1);
    }
    {
        b.unshift().copy_from_slice(&data2);
    }
    {
        b.unshift().copy_from_slice(&data3);
    }
    assert_eq!(b.pop(), &data1);
    assert_eq!(b.pop(), &data2);
    assert_eq!(b.pop(), &data3);
}

#[test]
fn test_new_size_rounded_up_to_power_of_two() {
    // C's circa_alloc passes size through roundup64(size). If we ask for
    // size=3, it allocates capacity 4. Verify by unshifting 4 items without
    // a resize being required (we observe via final order being preserved).
    let mut b = CircBuf::new(1, 3);
    for i in 0u8..4 {
        b.unshift().copy_from_slice(&[i]);
    }
    // We should be able to pop them all in unshift order.
    for i in 0u8..4 {
        assert_eq!(b.pop(), &[i]);
    }
}

#[test]
fn test_dealloc_via_drop_does_not_panic() {
    // Drop calls dealloc; ensure it works on empty buffers, full buffers,
    // and after operations.
    {
        let _b = CircBuf::new(8, 4);
    }
    {
        let mut b = CircBuf::new(4, 2);
        b.unshift().copy_from_slice(&[1u8, 2, 3, 4]);
    }
    {
        let mut b = CircBuf::new(1, 2);
        for i in 0u8..10 {
            b.unshift().copy_from_slice(&[i]);
        }
    }
}

#[test]
fn test_long_sequence_roundtrip() {
    // Stress: repeatedly unshift many items and pop them, confirming FIFO.
    let mut b = CircBuf::new(4, 2);
    let n: u32 = 100;
    for i in 0..n {
        let bytes = i.to_ne_bytes();
        b.unshift().copy_from_slice(&bytes);
    }
    for i in 0..n {
        let bytes = i.to_ne_bytes();
        let v = b.pop();
        assert_eq!(v, &bytes, "i={}", i);
    }
}

fn main() {}
