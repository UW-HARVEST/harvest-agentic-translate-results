// Translated from c_src/src/a.c

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = Cell::new(0);
}

fn a_bias_call<F: Fn(i32) -> i32>(fp: F, x: i32) -> i32 {
    // C: fp((x ^ 0x55) + 7)
    fp((x ^ 0x55).wrapping_add(7))
}

fn target_a(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|c| c.get());
        return if (s & 1) != 0 { 6 } else { 5 };
    }
    STATE_A.with(|c| {
        let new_state = c.get() ^ (code.wrapping_shl(1));
        c.set(new_state);
    });
    let state = STATE_A.with(|c| c.get());
    let k = ((code >> 2) ^ state) & 7;
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
    // C uses a function pointer fp = &target; then a = fp(x)
    let a = target_a(x);
    let b = wrap_a(a);
    let c = target_a(b ^ 3);
    // A_MAC_CALL(F,X) = a_bias_call(F, X) — note this is NOT MAC_CALL from engine.c
    let d = a_bias_call(target_a, b);
    a ^ (b.wrapping_shl(1)) ^ (c.wrapping_shl(2)) ^ (d.wrapping_shl(3))
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // C uses size_t acc = 0; (unsigned!) and accumulates with += and ^=
    // Then clamps to int range. The C code:
    //   if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //   if (acc < -0x80000000LL) acc = -0x80000000LL;
    // Since acc is size_t (unsigned), only the first check applies.
    // We model size_t as u64 (matches the typical 64-bit Linux target the C is built for).
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                // size_t acc += int t (with t signed): in C, t is sign-extended to ptrdiff_t/long?
                // Actually: t is int. Adding int to size_t: int promoted to size_t.
                // If t is negative, it becomes a huge unsigned number — but here target_a returns
                // values in {0,1,2,3,4,5,7} so always non-negative.
                acc = acc.wrapping_add(t as u64);
                continue;
            }
            // acc ^= (t << j); — t is int, shifted, then implicitly converted to size_t
            let shifted = t.wrapping_shl(j as u32);
            // sign-extend to i64 then to u64 for XOR-equivalence with C size_t
            acc ^= shifted as i64 as u64;
            if t == 5 {
                break;
            }
        }
    }
    // C: if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //    if (acc < -0x80000000LL) acc = -0x80000000LL;
    // First comparison: acc (size_t) > 0x7fffffff (long long, but converted to size_t — if size_t is 64-bit unsigned, it's just 0x7fffffff).
    if acc > 0x7fffffff_u64 {
        acc = 0x7fffffff_u64;
    }
    // Second comparison: acc (size_t, unsigned) < -0x80000000LL.
    // The right side -0x80000000LL is a negative long long; when comparing with size_t,
    // it gets converted to size_t (unsigned), which makes it a huge value.
    // So this check on a 64-bit machine becomes acc < (size_t)(-0x80000000LL) = 0xFFFFFFFF80000000.
    // After the previous clamp, acc is at most 0x7fffffff, which is less than 0xFFFFFFFF80000000,
    // so this assignment WILL fire. Let's reproduce that bug.
    let neg_limit: u64 = (-0x80000000_i64) as u64; // = 0xFFFFFFFF80000000
    if acc < neg_limit {
        acc = neg_limit;
    }
    // return (int)acc — truncate to i32
    acc as i32
}
