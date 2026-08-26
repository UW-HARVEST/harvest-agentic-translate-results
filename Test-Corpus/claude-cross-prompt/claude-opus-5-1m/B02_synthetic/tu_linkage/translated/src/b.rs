// Translated from c_src/src/b.c
// Provides public symbols: call_b_once, process_b_stream
// (target here is file-local in C; we mirror that as a private function.)

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Mirrors C's `static int flipflop;` — zero-initialized, preserved across calls.
static FLIPFLOP: AtomicI32 = AtomicI32::new(0);

fn b_target(code: c_int) -> c_int {
    // flipflop ^= 1;
    let new_ff = FLIPFLOP.load(Ordering::Relaxed) ^ 1;
    FLIPFLOP.store(new_ff, Ordering::Relaxed);
    if code < 0 {
        return if new_ff != 0 { 2 } else { 6 };
    }
    let mask = if new_ff != 0 { 0x7f } else { 0x1f };
    // C `%` operator: truncated toward zero. Rust's `%` matches for c_int.
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

fn b_twist_call(fp: fn(c_int) -> c_int, x: c_int) -> c_int {
    // ((x + 9) ^ 0x2222) - 17
    fp((x.wrapping_add(9) ^ 0x2222).wrapping_sub(17))
}

fn w2(x: c_int) -> c_int {
    b_target(x.wrapping_add(9))
}

#[unsafe(no_mangle)]
pub extern "C" fn call_b_once(x: c_int) -> c_int {
    let fp: fn(c_int) -> c_int = b_target;
    let a = b_target(x);
    let b = w2(a);
    // B_MAC_CALL(&target, a) -> b_twist_call(&target, a)
    let c = b_twist_call(b_target, a);
    let d = fp(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_b_stream(xs: *const c_int, n: usize) -> c_int {
    let mut acc: c_int = 1;
    unsafe {
        for i in 0..n {
            let v = *xs.add(i);
            let mut iter: c_int = 0;
            // while(++iter <= 4)
            loop {
                iter = iter.wrapping_add(1);
                if iter > 4 {
                    break;
                }
                let t = b_target(v.wrapping_sub(iter));
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
    }
    acc
}
