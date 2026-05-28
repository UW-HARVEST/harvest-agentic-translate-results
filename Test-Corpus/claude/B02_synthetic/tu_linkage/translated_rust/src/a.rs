// Translation of c_src/src/a.c
// NOTE: a.c has its OWN static `target` function (different from lib.c's target).
// Static state `state_a` is module-local.

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    // C: fp((x ^ 0x55) + 7)
    fp((x ^ 0x55).wrapping_add(7))
}

fn a_target(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|s| s.get());
        return if (s & 1) != 0 { 6 } else { 5 };
    }
    let new_state = STATE_A.with(|s| {
        let v = s.get() ^ (code.wrapping_shl(1));
        s.set(v);
        v
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

fn wrap(x: i32) -> i32 {
    a_target(x.wrapping_sub(5))
}

#[allow(non_snake_case)]
pub fn call_a_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = a_target;
    let a = fp(x);
    let b = wrap(a);
    let c = a_target(b ^ 3);
    // A_MAC_CALL(F,X) = a_bias_call(F, X)  -- note: differs from MAC_CALL in engine.c
    let d = a_bias_call(a_target, b);
    a ^ (b.wrapping_shl(1)) ^ (c.wrapping_shl(2)) ^ (d.wrapping_shl(3))
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // C: size_t acc=0;  -- this is unsigned!
    // Then in the loop it does acc += t, acc ^= (t<<j)
    // C: if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //     if (acc < -0x80000000LL) acc = -0x80000000LL;
    // Note: size_t is unsigned, so the < comparison with negative is always false.
    // Therefore, only the upper clamp can happen.
    // Then return (int)acc — which truncates / cast.
    // We must reproduce this exactly.
    let mut acc: u64 = 0; // simulate size_t (assume 64-bit platform)
    for &v in xs {
        for j in 0..3i32 {
            let t = a_target(v.wrapping_add(j));
            if (t & 1) == 0 {
                // acc += t  -- t is int, but added to size_t (u64).
                // In C, int is promoted to size_t (unsigned). For negative t, this
                // becomes a large unsigned value.
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            // acc ^= (t<<j)
            let shifted = (t as i64 as u64).wrapping_shl(j as u32);
            // Wait — in C, t<<j is done in int (since t is int and j is int).
            // Then result is xor'd with size_t — int promoted to size_t.
            let t_shifted: i32 = ((t as u32).wrapping_shl(j as u32)) as i32;
            acc ^= t_shifted as i64 as u64;
            let _ = shifted;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fffffff_u64 {
        acc = 0x7fffffff_u64;
    }
    // The < -0x80000000LL comparison: acc is unsigned, -0x80000000LL is long long.
    // In C, the comparison: acc (size_t = unsigned) < -0x80000000LL (signed long long).
    // Usual arithmetic conversion: signed long long converted to size_t (unsigned long).
    // -0x80000000LL as unsigned long = very large value (0xFFFFFFFF80000000 on 64-bit).
    // So acc < that is possible. But after the prior clamp acc <= 0x7fffffff which is
    // less than 0xFFFFFFFF80000000, so the comparison acc < 0xFFFFFFFF80000000 is true!
    // That would set acc = 0xFFFFFFFF80000000 (which is what -0x80000000LL is when
    // converted to size_t).
    // Wait: -0x80000000LL is -2147483648 as long long.
    // When compared with size_t (unsigned long), the long long is converted to unsigned long.
    // -2147483648 as unsigned long (64-bit) = 0xFFFFFFFF80000000 = 18446744071562067968.
    // So if acc <= 0x7fffffff, then acc < 0xFFFFFFFF80000000 is TRUE.
    // Then acc = -0x80000000LL converted to size_t = 0xFFFFFFFF80000000.
    // Then return (int)acc — truncates to lower 32 bits = 0x80000000 = -2147483648.
    //
    // So in practice: process_a_stream ALWAYS returns -2147483648 (INT_MIN)!
    // Wait — let me recheck the upper clamp. After "if acc > 0x7fffffffLL: acc = 0x7fffffffLL"
    // Now acc is at most 0x7fffffff. Then "if acc < -0x80000000LL" → since size_t comparison,
    // -0x80000000LL is converted to a huge unsigned, so acc < huge is true, so acc gets set
    // to that huge value (when assigned to size_t variable). Then return (int)acc truncates
    // to int.
    //
    // Wait: "acc = -0x80000000LL" — the assignment converts the long long to size_t.
    // The value of -0x80000000LL in long long is -2147483648.
    // Converted to size_t (unsigned 64-bit): 18446744071562067968 (0xFFFFFFFF80000000).
    // Then return (int)acc — cast to int. For 0xFFFFFFFF80000000, lower 32 bits = 0x80000000.
    // As int, that's -2147483648 (INT_MIN).
    //
    // So yes, this function always returns INT_MIN due to a buggy clamp.
    // We must reproduce this bug.
    let neg_min_as_size_t: u64 = (-0x80000000_i64) as u64;
    if acc < neg_min_as_size_t {
        acc = neg_min_as_size_t;
    }
    acc as i32
}
