// Phase B — valid-path differential tests for the LOW-LEVEL exports.
//
// Rows C1..C26 of CONFIGS.md. Every test drives both `.so`s through `dlsym`
// and compares the returned `int` byte-for-byte. All randomness is a fixed-seed
// SplitMix64 so failures reproduce exactly.

mod common;

use common::{diff_eq, foreach_both, reverse_both, Bits, Rng};

const N: usize = 4000; // randomized iterations per row

// ===========================================================================
// safe_double_to_int  (axis A)
// ===========================================================================

#[test]
fn c1_safe_double_to_int_in_range_positive() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC1_0000_0001);
    for _ in 0..N {
        // Spread across the whole positive representable int range, with fractions.
        let whole = rng.i32_in(0, i32::MAX - 1) as f64;
        let frac = (rng.next_u64() % 1_000_000) as f64 / 1_000_000.0;
        for d in [whole, whole + frac, whole * 0.5 + frac, frac] {
            let cv = unsafe { (c.safe_double_to_int)(d) };
            let rv = unsafe { (r.safe_double_to_int)(d) };
            diff_eq("C1", Bits(d), cv, rv);
        }
    }
}

#[test]
fn c2_safe_double_to_int_in_range_negative() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC2_0000_0002);
    for _ in 0..N {
        let whole = rng.i32_in(i32::MIN + 1, 0) as f64;
        let frac = (rng.next_u64() % 1_000_000) as f64 / 1_000_000.0;
        for d in [whole, whole + frac, whole - frac, whole * 0.5 - frac, -frac] {
            let cv = unsafe { (c.safe_double_to_int)(d) };
            let rv = unsafe { (r.safe_double_to_int)(d) };
            diff_eq("C2", Bits(d), cv, rv);
        }
    }
}

#[test]
fn c3_safe_double_to_int_zeros_and_subnormals() {
    let (c, r) = common::both();
    let mut fixed = vec![
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),               // smallest positive subnormal
        f64::from_bits(1) * -1.0,        // smallest negative subnormal
        f64::from_bits(0x000F_FFFF_FFFF_FFFF), // largest subnormal
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.0,
        -1.0,
        1e-300,
        -1e-300,
    ];
    let mut rng = Rng::new(0xC3_0000_0003);
    for _ in 0..N {
        // Random subnormals: exponent field all zero, random mantissa.
        let bits = rng.next_u64() & 0x800F_FFFF_FFFF_FFFF;
        fixed.push(f64::from_bits(bits));
    }
    for d in fixed {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("C3", Bits(d), cv, rv);
    }
}

#[test]
fn c4_safe_double_to_int_exact_int_boundaries() {
    let (c, r) = common::both();
    let mut cases: Vec<f64> = Vec::new();
    // Walk every double one ULP either side of the two clamp thresholds, plus
    // the neighbourhood of the extreme representable ints.
    for base in [
        i32::MAX as f64,
        i32::MIN as f64,
        (i32::MAX as f64) - 1.0,
        (i32::MIN as f64) + 1.0,
        2147483646.0,
        -2147483647.0,
        2147483648.0,
        -2147483649.0,
    ] {
        cases.push(base);
        cases.push(nextafter(base, f64::INFINITY));
        cases.push(nextafter(base, f64::NEG_INFINITY));
        for delta in [0.5, 0.25, 0.75, 1.0, 2.0, 1e-9] {
            cases.push(base + delta);
            cases.push(base - delta);
        }
    }
    for d in cases {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("C4", Bits(d), cv, rv);
    }
}

/// Minimal `nextafter` for finite inputs (avoids needing libm bindings).
fn nextafter(x: f64, toward: f64) -> f64 {
    if x == toward || x.is_nan() {
        return x;
    }
    let bits = x.to_bits();
    let up = (toward > x) == (x.is_sign_positive() || x == 0.0);
    if x == 0.0 {
        return if toward > 0.0 {
            f64::from_bits(1)
        } else {
            -f64::from_bits(1)
        };
    }
    f64::from_bits(if up { bits + 1 } else { bits - 1 })
}

#[test]
fn c5_safe_double_to_int_random_bit_patterns() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC5_0000_0005);
    // Uniform u64 -> f64 covers NaNs (both signs, all payloads), infinities,
    // subnormals and enormous magnitudes: the entire f64 domain.
    for _ in 0..(N * 8) {
        let d = rng.f64_bits();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("C5", Bits(d), cv, rv);
    }
    // ...plus moderate values, which is where truncation direction matters.
    for _ in 0..(N * 4) {
        let d = rng.f64_moderate();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("C5", Bits(d), cv, rv);
    }
}

// ===========================================================================
// process_array_reverse  (axes B, C)
// ===========================================================================

#[test]
fn c6_process_array_reverse_degenerate_counts() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC6_0000_0006);
    for _ in 0..N {
        let len = rng.usize_in(1, 8);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        for count in [0, 1] {
            let (cv, rv) = reverse_both(&c, &r, &mut buf, count);
            diff_eq("C6", format!("len={len} count={count} buf={buf:?}"), cv, rv);
        }
    }
}

#[test]
fn c7_process_array_reverse_many_small_values() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC7_0000_0007);
    for _ in 0..N {
        let len = rng.usize_in(2, 64);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_in(-10_000, 10_000)).collect();
        let count = rng.i32_in(1, len as i32); // stay in bounds walking backwards
        let (cv, rv) = reverse_both(&c, &r, &mut buf, count);
        diff_eq("C7", format!("len={len} count={count} buf={buf:?}"), cv, rv);
    }
}

#[test]
fn c8_process_array_reverse_full_range_values_wrapping() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC8_0000_0008);
    for _ in 0..N {
        let len = rng.usize_in(2, 64);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let count = rng.i32_in(1, len as i32);
        let (cv, rv) = reverse_both(&c, &r, &mut buf, count);
        diff_eq("C8", format!("len={len} count={count} buf={buf:?}"), cv, rv);
    }
}

#[test]
fn c9_process_array_reverse_forced_overflow() {
    let (c, r) = common::both();
    for fill in [i32::MAX, i32::MIN, -1, 1, i32::MAX - 1, i32::MIN + 1] {
        for len in 1..=40usize {
            let mut buf = vec![fill; len];
            let (cv, rv) = reverse_both(&c, &r, &mut buf, len as i32);
            diff_eq("C9", format!("fill={fill} len={len}"), cv, rv);
        }
    }
}

// ===========================================================================
// foreach_sum  (FOREACH macro; axes B, C)
// ===========================================================================

#[test]
fn c10_foreach_sum_degenerate_counts() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC10_0000_000A);
    for _ in 0..N {
        let len = rng.usize_in(1, 8);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        for count in [0, 1] {
            let (cv, rv) = foreach_both(&c, &r, &mut buf, count);
            diff_eq("C10", format!("len={len} count={count} buf={buf:?}"), cv, rv);
        }
    }
}

#[test]
fn c11_foreach_sum_many_full_range_wrapping() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC11_0000_000B);
    for _ in 0..N {
        let len = rng.usize_in(2, 64);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let count = rng.i32_in(1, len as i32);
        let (cv, rv) = foreach_both(&c, &r, &mut buf, count);
        diff_eq("C11", format!("len={len} count={count} buf={buf:?}"), cv, rv);
    }
    // Forced-overflow fills, exhaustive over small lengths.
    for fill in [i32::MAX, i32::MIN, -1, 1] {
        for len in 1..=40usize {
            let mut buf = vec![fill; len];
            let (cv, rv) = foreach_both(&c, &r, &mut buf, len as i32);
            diff_eq("C11", format!("fill={fill} len={len}"), cv, rv);
        }
    }
}

#[test]
fn c12_foreach_and_reverse_agree_on_same_buffer() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xC12_0000_000C);
    for _ in 0..N {
        let len = rng.usize_in(1, 48);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let n = len as i32;

        let (cf, rf) = foreach_both(&c, &r, &mut buf, n);
        diff_eq("C12/foreach", format!("len={len} buf={buf:?}"), cf, rf);

        let (cr, rr) = reverse_both(&c, &r, &mut buf, n);
        diff_eq("C12/reverse", format!("len={len} buf={buf:?}"), cr, rr);

        // Summing the whole buffer forwards and backwards wraps identically,
        // so C's own two traversals must agree — and so must Rust's.
        diff_eq(
            "C12/cross-traversal",
            format!("len={len} buf={buf:?}"),
            cf,
            cr,
        );
        diff_eq(
            "C12/cross-traversal-rust",
            format!("len={len} buf={buf:?}"),
            rf,
            rr,
        );
    }
}

// ===========================================================================
// switch_fallthrough_calculator  (axes D, E)
// ===========================================================================

fn switch_row(row: &str, op: i32, seed: u64) {
    let (c, r) = common::both();
    let mut rng = Rng::new(seed);
    let mut values: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        8,
        63,
        64,
        127,
        128,
        255,
        256,
        511,
        512,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 8,
        i32::MIN / 8,
        i32::MAX / 3,
        i32::MIN / 3,
        0x4000_0000,
        -0x4000_0000,
    ];
    for _ in 0..N {
        values.push(rng.i32_any());
    }
    for _ in 0..N {
        values.push(rng.i32_in(-2048, 2048));
    }
    for v in values {
        let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
        let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
        diff_eq(row, format!("value={v} operation={op}"), cv, rv);
    }
}

#[test]
fn c13_switch_op0_triple_fallthrough() {
    switch_row("C13", 0, 0x13_0000_0013);
}

#[test]
fn c14_switch_op1_double_fallthrough() {
    switch_row("C14", 1, 0x14_0000_0014);
}

#[test]
fn c15_switch_op2_mask_only() {
    switch_row("C15", 2, 0x15_0000_0015);
}

#[test]
fn c16_switch_op3_fallthrough_no_mask() {
    switch_row("C16", 3, 0x16_0000_0016);
}

#[test]
fn c17_switch_op4_add_no_mask() {
    switch_row("C17", 4, 0x17_0000_0017);
}

#[test]
fn c18_switch_default_arm() {
    for (i, op) in [-1i32, -2, -5, 5, 6, 100, i32::MAX, i32::MIN]
        .into_iter()
        .enumerate()
    {
        switch_row("C18", op, 0x18_0000_0000 + i as u64);
    }
}

#[test]
fn c19_switch_exhaustive_operation_window() {
    let (c, r) = common::both();
    let extremes = [
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        511,
        512,
        -512,
        0x2000_0000,
        -0x2000_0000,
        1431655765,
        -1431655765,
    ];
    // Exhaustive over every `operation` from well below the first case to well
    // above the last, so no off-by-one in the match arms can hide.
    for op in -8i32..=12 {
        for v in extremes {
            let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
            diff_eq("C19", format!("value={v} operation={op}"), cv, rv);
        }
    }
}

// ===========================================================================
// allocate_and_compute  (axes F, G)
// ===========================================================================

#[test]
fn c20_allocate_size_zero_all_multipliers() {
    let (c, r) = common::both();
    let mut mults = vec![
        0.0f64,
        -0.0,
        1.5,
        -1.5,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN_POSITIVE,
    ];
    let mut rng = Rng::new(0x20_0000_0020);
    for _ in 0..64 {
        mults.push(rng.f64_bits());
    }
    for m in mults {
        let cv = unsafe { (c.allocate_and_compute)(0, m) };
        let rv = unsafe { (r.allocate_and_compute)(0, m) };
        diff_eq("C20", format!("size=0 mult={}", Bits(m)), cv, rv);
    }
}

#[test]
fn c21_allocate_size_one_multiplier_1_5() {
    let (c, r) = common::both();
    for size in [1i32] {
        let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
        let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
        diff_eq("C21", format!("size={size} mult=1.5"), cv, rv);
    }
}

#[test]
fn c22_allocate_small_many_multiplier_1_5() {
    let (c, r) = common::both();
    for size in 0..=64i32 {
        let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
        let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
        diff_eq("C22", format!("size={size} mult=1.5"), cv, rv);
    }
}

#[test]
fn c23_allocate_small_many_random_finite_multiplier() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x23_0000_0023);
    for _ in 0..N {
        let size = rng.i32_in(0, 256);
        let m = rng.f64_finite();
        let cv = unsafe { (c.allocate_and_compute)(size, m) };
        let rv = unsafe { (r.allocate_and_compute)(size, m) };
        diff_eq("C23", format!("size={size} mult={}", Bits(m)), cv, rv);
    }
    for _ in 0..N {
        let size = rng.i32_in(0, 256);
        let m = rng.f64_moderate();
        let cv = unsafe { (c.allocate_and_compute)(size, m) };
        let rv = unsafe { (r.allocate_and_compute)(size, m) };
        diff_eq("C23", format!("size={size} mult={}", Bits(m)), cv, rv);
    }
}

#[test]
fn c24_allocate_zero_and_negative_zero_multiplier() {
    let (c, r) = common::both();
    for size in 0..=48i32 {
        for m in [0.0f64, -0.0] {
            let cv = unsafe { (c.allocate_and_compute)(size, m) };
            let rv = unsafe { (r.allocate_and_compute)(size, m) };
            diff_eq("C24", format!("size={size} mult={}", Bits(m)), cv, rv);
        }
    }
}

#[test]
fn c25_allocate_large_many_overflowing_multiplier() {
    let (c, r) = common::both();
    // Sizes chosen to keep the allocation modest (4096 * 16 B = 64 KiB) while
    // still pushing the double accumulator to +/-Inf so the clamp fires.
    for size in [1i32, 2, 3, 17, 255, 256, 1024, 4096] {
        for m in [
            f64::MAX,
            -f64::MAX,
            1e308,
            -1e308,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            1e150,
            -1e150,
            1e10,
            -1e10,
        ] {
            let cv = unsafe { (c.allocate_and_compute)(size, m) };
            let rv = unsafe { (r.allocate_and_compute)(size, m) };
            diff_eq("C25", format!("size={size} mult={}", Bits(m)), cv, rv);
        }
    }
}

#[test]
fn c26_allocate_exact_range_reachable_from_fallcalc() {
    let (c, r) = common::both();
    // `fallcalc` calls allocate_and_compute(param4 % 10 + 1, 1.5), so the
    // reachable sizes are -8..=10. Cover every one of them exhaustively.
    for size in -8i32..=10 {
        let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
        let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
        diff_eq("C26", format!("size={size} mult=1.5"), cv, rv);
    }
}
