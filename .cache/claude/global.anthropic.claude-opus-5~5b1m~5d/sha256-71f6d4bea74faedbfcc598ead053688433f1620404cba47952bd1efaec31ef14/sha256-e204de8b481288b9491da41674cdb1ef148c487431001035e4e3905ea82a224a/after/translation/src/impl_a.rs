// Translation of c_src/src/a.c
//
// Copyright 2025 MIT Lincoln Laboratory  (see c_src for the full license text)

use std::cell::Cell;

thread_local! {
    /// `static int state_a;` (zero initialised, persists between calls)
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

/// `static inline int a_bias_call(int (*fp)(int), int x)`
fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp((x ^ 0x55).wrapping_add(7))
}

/// `static int target(int code)` local to a.c
fn target(code: i32) -> i32 {
    if code < 0 {
        return if (STATE_A.get() & 1) != 0 { 6 } else { 5 };
    }
    // state_a = state_a ^ (code<<1);
    let new_state = STATE_A.get() ^ (((code as u32) << 1) as i32);
    STATE_A.set(new_state);
    let k = ((code >> 2) ^ new_state) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        // `case 5:;` falls through to `case 6:`
        5 | 6 => 5,
        _ => 7,
    }
}

/// `static inline int wrap(int x){ return target(x-5); }`
fn wrap(x: i32) -> i32 {
    target(x.wrapping_sub(5))
}

/// `int call_a_once(int x)`
pub fn call_a_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target;
    let a = fp(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    // A_MAC_CALL(&target, b) == a_bias_call(&target, b)
    let d = a_bias_call(target, b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

/// `int process_a_stream(const int *xs, size_t n)`
///
/// `acc` is a `size_t` (64-bit unsigned) in the C source.  The two clamps at
/// the end compare it against `long long` constants; the usual arithmetic
/// conversions turn those into *unsigned* 64-bit values, so `-0x80000000LL`
/// becomes 0xFFFFFFFF80000000 and the second clamp always fires.  The function
/// therefore always returns INT_MIN.  This is faithfully reproduced here
/// (including all of `target`'s side effects on `state_a`).
pub fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: u64 = 0;
    for &v in xs.iter() {
        for j in 0..3i32 {
            let t = target(v.wrapping_add(j));
            if (t & 1) == 0 {
                // acc += t;  (int -> size_t conversion sign-extends)
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            acc ^= (t << j) as i64 as u64;
            if t == 5 {
                break;
            }
        }
    }
    // if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    if acc > 0x7fffffff_u64 {
        acc = 0x7fffffff_u64;
    }
    // if (acc < -0x80000000LL) acc = -0x80000000LL;
    const NEG_TWO31: u64 = (-0x80000000_i64) as u64;
    if acc < NEG_TWO31 {
        acc = NEG_TWO31;
    }
    // return (int)acc;
    acc as u32 as i32
}
