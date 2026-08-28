//! Port of `c_src/src/b.c`.
//!
//! As in a.c, `target` is file-static and shadows the global one. `flipflop`
//! is toggled on *every* call, including the negative-code early return, and
//! persists for the process lifetime.

use std::cell::Cell;

thread_local! {
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

/// `b_twist_call`
fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

/// `static int target(int code)` from b.c
fn target(code: i32) -> i32 {
    FLIPFLOP.set(FLIPFLOP.get() ^ 1);
    if code < 0 {
        return if FLIPFLOP.get() != 0 { 2 } else { 6 };
    }
    let mask = if FLIPFLOP.get() != 0 { 0x7f } else { 0x1f };
    let z = (code ^ mask) % 8;
    if z == 0 || z == 7 {
        return 4;
    }
    if z == 1 || z == 2 {
        return 3;
    }
    if z == 3 {
        return 1;
    }
    if z == 4 {
        return 0;
    }
    if z == 5 {
        return 5;
    }
    7
}

/// `w2`
fn w2(x: i32) -> i32 {
    target(x.wrapping_add(9))
}

/// `call_b_once`
pub fn call_b_once(x: i32) -> i32 {
    let a = target(x);
    let b = w2(a);
    let c = b_twist_call(target, a); // B_MAC_CALL(&target, a)
    let d = target(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

/// `process_b_stream`
pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter: i32 = 0;
        // while (++iter <= 4)
        loop {
            iter += 1;
            if iter > 4 {
                break;
            }
            let t = target(v.wrapping_sub(iter));
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}
