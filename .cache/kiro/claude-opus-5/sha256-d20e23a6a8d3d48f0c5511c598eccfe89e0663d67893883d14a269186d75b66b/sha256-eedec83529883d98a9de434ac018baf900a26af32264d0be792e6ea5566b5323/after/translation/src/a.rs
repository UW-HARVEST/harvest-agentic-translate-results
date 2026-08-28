//! Port of `c_src/src/a.c`.
//!
//! `target` here is `static` in the C file, so it shadows the global `target`
//! from `lib.c` for this translation unit only. `state_a` is file-scope mutable
//! state that persists for the whole process lifetime -- and therefore across
//! all three engine runs in `main`.

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

/// `a_bias_call`
fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp((x ^ 0x55).wrapping_add(7))
}

/// `static int target(int code)` from a.c
fn target(code: i32) -> i32 {
    if code < 0 {
        return if (STATE_A.get() & 1) != 0 { 6 } else { 5 };
    }
    STATE_A.set(STATE_A.get() ^ code.wrapping_shl(1));
    let k = ((code >> 2) ^ STATE_A.get()) & 7;
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

/// `wrap`
fn wrap(x: i32) -> i32 {
    target(x.wrapping_sub(5))
}

/// `call_a_once`
///
/// The four calls are separate statements in C, so their side effects on
/// `state_a` happen in the order a, b, c, d.
pub fn call_a_once(x: i32) -> i32 {
    let a = target(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    let d = a_bias_call(target, b); // A_MAC_CALL(&target, b)
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

/// `process_a_stream`
///
/// `acc` is a `size_t` (unsigned 64-bit) in the original. The two clamping
/// comparisons are therefore *unsigned*, which makes the second one always
/// true: after the first clamp `acc <= 0x7fffffff`, and `-0x80000000LL`
/// converts to the very large value `0xffffffff80000000`. So this function
/// always ends up returning `INT_MIN`. That is a bug in the C, and it is
/// reproduced here exactly.
pub fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(i64::from(t) as u64);
                continue;
            }
            acc ^= i64::from(t.wrapping_shl(j as u32)) as u64;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fff_ffff_u64 {
        acc = 0x7fff_ffff_u64;
    }
    if acc < (-0x8000_0000_i64) as u64 {
        acc = (-0x8000_0000_i64) as u64;
    }
    acc as u32 as i32
}
