// Translation of a.c
// Implementation A. Uses its own static `state_a` and a private `target`.

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    // C: fp((x ^ 0x55) + 7)
    fp((x ^ 0x55).wrapping_add(7))
}

fn target(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|c| c.get());
        return if (s & 1) != 0 { 6 } else { 5 };
    }
    let mut s = STATE_A.with(|c| c.get());
    s ^= code.wrapping_shl(1);
    STATE_A.with(|c| c.set(s));
    let k = ((code >> 2) ^ s) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        5 | 6 => 5,
        _ => 7,
    }
}

fn wrap(x: i32) -> i32 {
    target(x.wrapping_sub(5))
}

pub fn call_a_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target;
    let a = fp(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    // A_MAC_CALL(F, X) = a_bias_call(F, X)
    let d = a_bias_call(target, b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // C uses size_t acc (unsigned), then clamps to [-2^31, 2^31-1] before cast.
    // size_t is 64-bit on x86_64 Linux. We mirror that with u64.
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3 {
            let t = target(v.wrapping_add(j));
            if (t & 1) == 0 {
                // acc += t   (size_t += int, with sign-extension)
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            // acc ^= (t << j)   (size_t ^= int, with sign-extension)
            let v2 = (t.wrapping_shl(j as u32)) as i64 as u64;
            acc ^= v2;
            if t == 5 {
                break;
            }
        }
    }
    // Clamp using signed comparisons mirroring the C code.
    // C: if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //    if (acc < -0x80000000LL) acc = -0x80000000LL;
    // 'acc' is size_t (unsigned 64-bit). Comparing size_t > 0x7fffffffLL compares as i64
    // since 0x7fffffffLL is signed long long (positive), and integer promotion converts both
    // to unsigned long. The literal 0x7fffffffLL becomes (unsigned long)0x7fffffff.
    // Comparing acc < -0x80000000LL: -0x80000000LL is (long long)-2147483648, when
    // converted to unsigned long becomes a huge positive number; thus the second check
    // can succeed if acc < that huge value (i.e. always true unless acc is even larger).
    // But after the first check, acc <= 0x7fffffff, so acc < (unsigned)(-0x80000000LL)
    // is true (since 0x7fffffff < ~0xFFFFFFFF80000000). Then acc would be set to
    // (size_t)(-0x80000000LL) = 0xFFFFFFFF80000000, then cast to int = -0x80000000.
    // We need to reproduce this behavior exactly. Let's mirror C semantics.
    let signed_max: u64 = 0x7fffffff_u64; // 0x7fffffffLL as unsigned long
    let neg_min_unsigned: u64 = (-0x80000000_i64) as u64; // 0xFFFFFFFF80000000
    if acc > signed_max {
        acc = signed_max;
    }
    if acc < neg_min_unsigned {
        acc = neg_min_unsigned;
    }
    acc as i32
}
