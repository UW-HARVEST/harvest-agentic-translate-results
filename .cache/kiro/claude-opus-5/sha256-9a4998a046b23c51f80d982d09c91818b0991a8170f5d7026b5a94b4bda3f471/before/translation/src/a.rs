// Translation of c_src/src/a.c
//
// `target` here is file-static in the C original, distinct from lib.c's global
// `target`. It carries mutable state (`state_a`) that persists across calls.

use std::cell::Cell;

thread_local! {
    /// C: `static int state_a;` (zero-initialized, persists for process lifetime)
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

/// C: `static inline int a_bias_call(int (*fp)(int), int x) { return fp((x ^ 0x55) + 7); }`
/// The only call site passes `&target`, so the indirection is devirtualized here.
fn a_bias_call(x: i32) -> i32 {
    target((x ^ 0x55).wrapping_add(7))
}

/// C: `static int target(int code)` in a.c
fn target(code: i32) -> i32 {
    STATE_A.with(|state_a| {
        if code < 0 {
            return if (state_a.get() & 1) != 0 { 6 } else { 5 };
        }
        state_a.set(state_a.get() ^ code.wrapping_shl(1));
        let k = ((code >> 2) ^ state_a.get()) & 7;
        match k {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 1,
            4 => 3,
            // C: `case 5:;` falls through into `case 6:`
            5 | 6 => 5,
            _ => 7,
        }
    })
}

/// C: `static inline int wrap(int x){ return target(x-5); }`
fn wrap(x: i32) -> i32 {
    target(x.wrapping_sub(5))
}

pub fn call_a_once(x: i32) -> i32 {
    // Each `target` call mutates `state_a`, so the sequencing below is
    // load-bearing: a, then b, then c, then d.
    let a = target(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    // C: `A_MAC_CALL(&target, b)` -> `a_bias_call((&target), (b))`
    let d = a_bias_call(b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // C: `size_t acc = 0;` -- an *unsigned* 64-bit accumulator.
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target(v.wrapping_add(j));
            if (t & 1) == 0 {
                // C: `acc += t;`  (int -> size_t conversion)
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            // C: `acc ^= (t << j);`
            acc ^= t.wrapping_shl(j as u32) as i64 as u64;
            if t == 5 {
                break;
            }
        }
    }
    // Faithful reproduction of the original's clamping bug.
    //
    // `acc` is `size_t` (unsigned 64-bit). In `acc > 0x7fffffffLL` and
    // `acc < -0x80000000LL`, the usual arithmetic conversions promote *both*
    // operands to `unsigned long long`, so `-0x80000000LL` becomes
    // 0xFFFFFFFF80000000. Any acc that survived the first clamp is <= 0x7fffffff
    // and therefore always compares less than that, so acc is unconditionally
    // overwritten with 0xFFFFFFFF80000000 and the (int) cast truncates it to
    // INT_MIN. This function always returns -2147483648.
    if acc > 0x7fffffffu64 {
        acc = 0x7fffffffu64;
    }
    if acc < (-0x80000000i64) as u64 {
        acc = (-0x80000000i64) as u64;
    }
    // C: `(int)acc` -- gcc truncates to the low 32 bits.
    acc as u32 as i32
}
