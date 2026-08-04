use carrays::circ_array::CircBuf;

#[test]
fn test_new_basic() {
    // size=2 element=4 bytes
    let buf = CircBuf::new(4, 2);
    drop(buf);
}

#[test]
fn test_new_zero_init_push() {
    // After push, the slot should be zero'd
    let mut buf = CircBuf::new(4, 4);
    let s = buf.push();
    assert_eq!(s, &[0u8, 0, 0, 0]);
}

#[test]
fn test_new_zero_init_unshift() {
    // After unshift, the slot should be zero'd
    let mut buf = CircBuf::new(4, 4);
    let s = buf.unshift();
    assert_eq!(s, &[0u8, 0, 0, 0]);
}

#[test]
fn test_push_pop() {
    // Use size=8 to avoid any resize during the test
    let mut buf = CircBuf::new(1, 8);
    {
        let s = buf.push();
        s[0] = 10;
    }
    {
        let s = buf.push();
        s[0] = 20;
    }
    {
        let s = buf.push();
        s[0] = 30;
    }
    // pop returns from front (LIFO with push) - removes most recently pushed
    {
        let s = buf.pop();
        assert_eq!(s[0], 30);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 20);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 10);
    }
}

#[test]
fn test_unshift_pop() {
    // unshift adds to back, pop removes from front
    let mut buf = CircBuf::new(1, 8);
    {
        let s = buf.unshift();
        s[0] = 1;
    }
    {
        let s = buf.unshift();
        s[0] = 2;
    }
    {
        let s = buf.unshift();
        s[0] = 3;
    }
    // FIFO order: pop from front returns 1, 2, 3
    {
        let s = buf.pop();
        assert_eq!(s[0], 1);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 2);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 3);
    }
}

#[test]
fn test_capacity_no_op() {
    // capacity should not change anything if requested size is smaller
    let mut buf = CircBuf::new(1, 8);
    {
        let s = buf.unshift();
        s[0] = 7;
    }
    buf.capacity(4); // no-op since 4 < 8
    {
        let s = buf.pop();
        assert_eq!(s[0], 7);
    }
}

#[test]
fn test_capacity_grows() {
    // Calling capacity should grow without losing data when no wrap
    let mut buf = CircBuf::new(1, 4);
    {
        let s = buf.unshift();
        s[0] = 100;
    }
    {
        let s = buf.unshift();
        s[0] = 101;
    }
    buf.capacity(16);
    // Verify data still accessible via pop
    {
        let s = buf.pop();
        assert_eq!(s[0], 100);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 101);
    }
}

#[test]
fn test_dealloc_safe() {
    // Calling dealloc explicitly should be safe (and Drop should be a no-op)
    let mut buf = CircBuf::new(1, 4);
    {
        let s = buf.unshift();
        s[0] = 42;
    }
    buf.dealloc();
    // Now dropping is a no-op
}

#[test]
fn test_shift_decrements_n() {
    // shift in C is buggy - returns slot AFTER last item.
    // We verify behavior: shift can be called n times then we can refill.
    let mut buf = CircBuf::new(1, 8);
    for i in 0..3u8 {
        let s = buf.unshift();
        s[0] = i + 1;
    }
    // Three shifts should empty the buffer (matching C: shift just decrements n)
    let _ = buf.shift();
    let _ = buf.shift();
    let _ = buf.shift();
    // Now empty - we should be able to refill
    {
        let s = buf.unshift();
        s[0] = 99;
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 99);
    }
}

#[test]
fn test_norm_no_wrap() {
    // norm is no-op when not wrapped
    let mut buf = CircBuf::new(1, 8);
    {
        let s = buf.unshift();
        s[0] = 5;
    }
    {
        let s = buf.unshift();
        s[0] = 6;
    }
    buf.norm(); // no-op
    {
        let s = buf.pop();
        assert_eq!(s[0], 5);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 6);
    }
}

#[test]
fn test_norm_with_wrap() {
    // Cause a wrap-around then norm.
    let mut buf = CircBuf::new(1, 4);
    // Fill with 4 unshift -> 1,2,3,4 at positions 0,1,2,3
    for i in 1u8..=4 {
        let s = buf.unshift();
        s[0] = i;
    }
    // pop 2 items -> removes 1, 2 (FIFO from front), start advances to 2
    let _ = buf.pop();
    let _ = buf.pop();
    // unshift 2 more (5, 6) - they wrap to positions 0, 1
    {
        let s = buf.unshift();
        s[0] = 5;
    }
    {
        let s = buf.unshift();
        s[0] = 6;
    }
    // Now wrapped: items in logical order 3,4,5,6 at physical 2,3,0,1
    buf.norm();
    // After norm, we can still iterate through items in logical order via pop
    {
        let s = buf.pop();
        assert_eq!(s[0], 3);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 4);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 5);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 6);
    }
}

#[test]
fn test_pop_advances_start() {
    let mut buf = CircBuf::new(1, 8);
    {
        let s = buf.unshift();
        s[0] = 99;
    }
    {
        let s = buf.unshift();
        s[0] = 100;
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 99);
    }
    {
        let s = buf.unshift();
        s[0] = 101;
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 100);
    }
    {
        let s = buf.pop();
        assert_eq!(s[0], 101);
    }
}

#[test]
fn test_push_advances_start_back() {
    // push at the start when buffer is empty wraps start to size-1
    let mut buf = CircBuf::new(1, 4);
    {
        let s = buf.push();
        s[0] = 11;
    }
    // Now start should be 3 (size-1)
    // pop returns the item at start (which is 3)
    {
        let s = buf.pop();
        assert_eq!(s[0], 11);
    }
}

#[test]
fn test_multibyte_elements() {
    // Test with 4-byte elements
    let mut buf = CircBuf::new(4, 4);
    {
        let s = buf.unshift();
        s.copy_from_slice(&100u32.to_ne_bytes());
    }
    {
        let s = buf.unshift();
        s.copy_from_slice(&200u32.to_ne_bytes());
    }
    {
        let s = buf.pop();
        let mut arr = [0u8; 4];
        arr.copy_from_slice(s);
        assert_eq!(u32::from_ne_bytes(arr), 100);
    }
    {
        let s = buf.pop();
        let mut arr = [0u8; 4];
        arr.copy_from_slice(s);
        assert_eq!(u32::from_ne_bytes(arr), 200);
    }
}

fn main() {}
