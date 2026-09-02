// Translation of c_src/src/b.c
//
// `target` here is file-static in the C original, distinct from both lib.c's
// global `target` and a.c's static one. It toggles `flipflop` on *every* call,
// including the `code < 0` early-return path.

use std::cell::Cell;

thread_local! {
    /// C: `static int flipflop;` (zero-initialized, persists for process lifetime)
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

/// C: `static inline int b_twist_call(int (*fp)(int), int x) { return fp(((x + 9) ^ 0x2222) - 17); }`
fn b_twist_call(x: i32) -> i32 {
    target(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

/// C: `static int target(int code)` in b.c
fn target(code: i32) -> i32 {
    FLIPFLOP.with(|flipflop| {
        flipflop.set(flipflop.get() ^ 1);
        if code < 0 {
            return if flipflop.get() != 0 { 2 } else { 6 };
        }
        // code >= 0 and the xor masks keep the sign bit clear, so `%` yields 0..=7
        let z = (code ^ (if flipflop.get() != 0 { 0x7f } else { 0x1f })) % 8;
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
    })
}

/// C: `static inline int w2(int x){ return target(x+9); }`
fn w2(x: i32) -> i32 {
    target(x.wrapping_add(9))
}

pub fn call_b_once(x: i32) -> i32 {
    let a = target(x);
    let b = w2(a);
    // C: `B_MAC_CALL(&target, a)` -> `b_twist_call((&target), (a))`
    let c = b_twist_call(a);
    let d = target(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        // C: `int iter=0; while(++iter<=4) { ... }`
        let mut iter: i32 = 0;
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
