// Translated from c_src/src/a.c
// Provides public symbols: call_a_once, process_a_stream
// (target here is file-local in C; we mirror that as a private function.)

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Mirrors C's `static int state_a;` — zero-initialized, preserved across calls.
static STATE_A: AtomicI32 = AtomicI32::new(0);

// File-local target() in a.c (NOT the public lib.c target).
fn a_target(code: c_int) -> c_int {
    if code < 0 {
        let s = STATE_A.load(Ordering::Relaxed);
        return if (s & 1) != 0 { 6 } else { 5 };
    }
    let prev = STATE_A.load(Ordering::Relaxed);
    // state_a = state_a ^ (code<<1);
    let new_state = prev ^ code.wrapping_shl(1);
    STATE_A.store(new_state, Ordering::Relaxed);
    let k = ((code >> 2) ^ new_state) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        // case 5 falls through to 6 in C
        5 | 6 => 5,
        _ => 7,
    }
}

// Mirrors: static inline int a_bias_call(int (*fp)(int), int x){ return fp((x ^ 0x55) + 7); }
fn a_bias_call(fp: fn(c_int) -> c_int, x: c_int) -> c_int {
    fp((x ^ 0x55).wrapping_add(7))
}

// Mirrors: static inline int wrap(int x){ return target(x-5); }
fn wrap_a(x: c_int) -> c_int {
    a_target(x.wrapping_sub(5))
}

#[unsafe(no_mangle)]
pub extern "C" fn call_a_once(x: c_int) -> c_int {
    // int (*fp)(int) = &target;  — points to file-local target.
    let fp: fn(c_int) -> c_int = a_target;
    let a = fp(x);
    let b = wrap_a(a);
    let c = a_target(b ^ 3);
    // A_MAC_CALL(&target, b) -> a_bias_call(&target, b)
    let d = a_bias_call(a_target, b);
    // a ^ (b << 1) ^ (c << 2) ^ (d << 3)
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_a_stream(xs: *const c_int, n: usize) -> c_int {
    // int process_a_stream(const int *xs, size_t n){
    //     size_t acc=0;
    //     for(size_t i=0;i<n;i++){
    //         int v=xs[i];
    //         for(int j=0;j<3;j++){
    //             int t=target(v+j);
    //             if ((t&1)==0) { acc += t; continue; }
    //             acc ^= (t<<j);
    //             if (t==5) break;
    //         }
    //     }
    //     if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //     if (acc < -0x80000000LL) acc = -0x80000000LL;
    //     return (int)acc;
    // }
    //
    // Note: acc is `size_t` (unsigned). When we add a negative `int t`, it's
    // first sign-extended to (signed long), then converted to size_t. Since
    // size_t is unsigned, `acc < -0x80000000LL` is always false.
    // We mirror the exact bit-level behavior using u64 (size_t is typically 64-bit).
    let mut acc: u64 = 0;
    unsafe {
        for i in 0..n {
            let v = *xs.add(i);
            for j in 0..3 {
                let t = a_target(v.wrapping_add(j));
                if (t & 1) == 0 {
                    // acc += t; — t is sign-extended via long to size_t.
                    acc = acc.wrapping_add(t as i64 as u64);
                    continue;
                }
                // acc ^= (t << j);
                let shifted = t.wrapping_shl(j as u32);
                acc ^= shifted as i64 as u64;
                if t == 5 {
                    break;
                }
            }
        }
    }
    // if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    // 0x7fffffffLL is a (long long) signed positive constant, but compared with
    // size_t it gets converted to size_t (unsigned), so this is unsigned
    // comparison.
    if acc > 0x7fffffff_u64 {
        acc = 0x7fffffff_u64;
    }
    // if (acc < -0x80000000LL) — negative LL converts to size_t, becomes a huge
    // unsigned number. acc < (huge) is sometimes true; in C: `-0x80000000LL` =
    // 0xFFFFFFFF80000000 as u64 — acc < that is true if acc < 0xFFFFFFFF80000000.
    // Then acc would be set to that huge value. The cast (int) truncates low 32
    // bits → 0x80000000 → INT_MIN.
    let neg_lit: u64 = (-0x80000000_i64) as u64; // = 0xFFFFFFFF80000000
    if acc < neg_lit {
        acc = neg_lit;
    }
    acc as i32
}
