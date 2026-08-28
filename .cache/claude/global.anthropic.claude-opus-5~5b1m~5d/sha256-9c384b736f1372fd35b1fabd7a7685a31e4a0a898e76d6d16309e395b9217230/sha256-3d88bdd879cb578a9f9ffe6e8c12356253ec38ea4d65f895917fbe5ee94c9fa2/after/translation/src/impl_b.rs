// Translation of c_src/src/b.c
//
// Copyright 2025 MIT Lincoln Laboratory  (see c_src for the full license text)

use std::cell::Cell;

thread_local! {
    /// `static int flipflop;` (zero initialised, persists between calls)
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

/// `static inline int b_twist_call(int (*fp)(int), int x)`
fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

/// `static int target(int code)` local to b.c
fn target(code: i32) -> i32 {
    let ff = FLIPFLOP.get() ^ 1;
    FLIPFLOP.set(ff);
    if code < 0 {
        return if ff != 0 { 2 } else { 6 };
    }
    let z = (code ^ if ff != 0 { 0x7f } else { 0x1f }) % 8;
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

/// `static inline int w2(int x){ return target(x+9); }`
fn w2(x: i32) -> i32 {
    target(x.wrapping_add(9))
}

/// `int call_b_once(int x)`
pub fn call_b_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target;
    let a = target(x);
    let b = w2(a);
    // B_MAC_CALL(&target, a) == b_twist_call(&target, a)
    let c = b_twist_call(target, a);
    let d = fp(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

/// `int process_b_stream(const int *xs, size_t n)`
pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs.iter() {
        let mut iter: i32 = 0;
        // while (++iter <= 4)
        loop {
            iter = iter.wrapping_add(1);
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
