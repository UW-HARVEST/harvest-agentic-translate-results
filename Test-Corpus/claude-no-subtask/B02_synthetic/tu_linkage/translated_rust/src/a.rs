// Translated from c_src/src/a.c
// Has a file-scope mutable static `state_a`. We mirror the same behavior using
// a thread_local (the C program is single-threaded).

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

fn a_bias_call<F: Fn(i32) -> i32>(fp: F, x: i32) -> i32 {
    fp((x ^ 0x55).wrapping_add(7))
}

fn target_a(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|c| c.get());
        return if (s & 1) != 0 { 6 } else { 5 };
    }
    let new_state = STATE_A.with(|c| {
        let s = c.get() ^ (code.wrapping_shl(1));
        c.set(s);
        s
    });
    let k = ((code >> 2) ^ new_state) & 7;
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

fn wrap_a(x: i32) -> i32 {
    target_a(x.wrapping_sub(5))
}

pub fn call_a_once(x: i32) -> i32 {
    let a = target_a(x);
    let b = wrap_a(a);
    let c = target_a(b ^ 3);
    // A_MAC_CALL(F, X) = a_bias_call((F), (X))
    let d = a_bias_call(target_a, b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // C uses size_t acc (unsigned 64-bit on Linux x86_64). The clamps below
    // compare an unsigned value to signed long long literals, which due to
    // C's usual arithmetic conversion rules become unsigned comparisons —
    // resulting in the original code almost always saturating to -0x80000000.
    // We faithfully reproduce that with u64 here.
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                // C: acc += t;  // int -> size_t (zero/sign-extend to u64)
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            // C: acc ^= (t << j);  // (t<<j) is int, then converted to size_t
            let shifted = (t.wrapping_shl(j as u32)) as i64 as u64;
            acc ^= shifted;
            if t == 5 {
                break;
            }
        }
    }
    // C bounds: signed literals converted to unsigned via usual conversions.
    let upper: u64 = 0x7fffffff_i64 as u64; // 0x000000007FFFFFFF
    let lower: u64 = (-0x80000000_i64) as u64; // 0xFFFFFFFF80000000
    if acc > upper {
        acc = upper;
    }
    if acc < lower {
        acc = lower;
    }
    // C: return (int)acc;
    acc as i32
}
