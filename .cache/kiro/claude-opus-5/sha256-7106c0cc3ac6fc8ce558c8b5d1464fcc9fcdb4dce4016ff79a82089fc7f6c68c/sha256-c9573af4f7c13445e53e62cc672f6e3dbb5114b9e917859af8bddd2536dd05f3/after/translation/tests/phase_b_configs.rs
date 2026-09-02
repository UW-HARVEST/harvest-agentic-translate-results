//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both the C `.so` and the Rust `.so` are
//! loaded with `libloading`; every call crosses the FFI boundary.
//!
//! Every row uses many randomized inputs with a fixed seed.

mod common;

use common::*;

/// Iteration count for the randomized (property-style) rows.
const N: usize = 4000;

// ===========================================================================
// C1 -- safe_double_to_int: special double classes
// ===========================================================================
#[test]
fn cfg_c1_safe_double_to_int_special() {
    let p = pair();
    let specials: &[f64] = &[
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xFFF8_0000_0000_0001), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,  // smallest subnormal
        -5e-324,
        f64::MAX,
        f64::MIN,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999999999,
        -0.9999999999999999,
        1e-300,
        1e300,
        -1e300,
    ];
    for &d in specials {
        cmp_sdti(p, d);
    }
}

// ===========================================================================
// C2 -- safe_double_to_int: exact INT_MAX / INT_MIN boundaries
// ===========================================================================
#[test]
fn cfg_c2_safe_double_to_int_boundaries() {
    let p = pair();
    let mut vals: Vec<f64> = Vec::new();
    for base in [
        2147483647.0f64,  // (double)INT_MAX exactly
        2147483646.0,
        2147483648.0,
        2147483647.5,
        2147483646.5,
        -2147483648.0,    // (double)INT_MIN exactly
        -2147483647.0,
        -2147483649.0,
        -2147483648.5,
        -2147483647.5,
        0.0,
        1.0,
        -1.0,
    ] {
        vals.push(base);
        // Both neighbours in the double lattice.
        vals.push(f64::from_bits(base.to_bits().wrapping_add(1)));
        vals.push(f64::from_bits(base.to_bits().wrapping_sub(1)));
        vals.push(base.next_up());
        vals.push(base.next_down());
    }
    for d in vals {
        cmp_sdti(p, d);
    }
}

// ===========================================================================
// C3 -- safe_double_to_int: random in-range positive
// ===========================================================================
#[test]
fn cfg_c3_safe_double_to_int_positive() {
    let p = pair();
    let mut r = Rng::new(0xC3_0001);
    for _ in 0..N {
        cmp_sdti(p, r.range_f64(0.0, 2147483648.0));
    }
    // Dense sweep right at the top of the range where truncation and the
    // `>= INT_MAX` guard interact.
    for _ in 0..N {
        cmp_sdti(p, r.range_f64(2147483640.0, 2147483655.0));
    }
}

// ===========================================================================
// C4 -- safe_double_to_int: random in-range negative
// ===========================================================================
#[test]
fn cfg_c4_safe_double_to_int_negative() {
    let p = pair();
    let mut r = Rng::new(0xC4_0001);
    for _ in 0..N {
        cmp_sdti(p, r.range_f64(-2147483649.0, 0.0));
    }
    for _ in 0..N {
        cmp_sdti(p, r.range_f64(-2147483655.0, -2147483640.0));
    }
    // Negative fractions: C casts toward zero, not toward -inf.
    for _ in 0..N {
        let whole = r.range_i32(-100000, 0) as f64;
        cmp_sdti(p, whole - r.unit_f64());
    }
}

// ===========================================================================
// C5 -- safe_double_to_int: random raw bit patterns
// ===========================================================================
#[test]
fn cfg_c5_safe_double_to_int_raw_bits() {
    let p = pair();
    let mut r = Rng::new(0xC5_0001);
    for _ in 0..(N * 4) {
        cmp_sdti(p, r.raw_f64());
    }
    // Sweep every exponent with a random mantissa, both signs.
    for exp in 0u64..2048 {
        for _ in 0..4 {
            let mant = r.next_u64() & 0x000F_FFFF_FFFF_FFFF;
            for sign in [0u64, 1u64 << 63] {
                cmp_sdti(p, f64::from_bits(sign | (exp << 52) | mant));
            }
        }
    }
}

// ===========================================================================
// C6/C7/C8/C9/C10 -- process_array_reverse
// ===========================================================================
#[test]
fn cfg_c6_reverse_single() {
    let p = pair();
    let mut r = Rng::new(0xC6_0001);
    for _ in 0..N {
        let buf = vec![r.interesting_i32()];
        cmp_reverse(p, &buf, 0, 1);
    }
}

#[test]
fn cfg_c7_reverse_two() {
    let p = pair();
    let mut r = Rng::new(0xC7_0001);
    for _ in 0..N {
        let buf = vec![r.interesting_i32(), r.interesting_i32()];
        cmp_reverse(p, &buf, 1, 2);
    }
}

#[test]
fn cfg_c8_reverse_many() {
    let p = pair();
    let mut r = Rng::new(0xC8_0001);
    for _ in 0..N {
        let n = r.range_i32(3, 64) as usize;
        let buf: Vec<i32> = (0..n).map(|_| r.interesting_i32()).collect();
        cmp_reverse(p, &buf, n - 1, n as i32);
    }
}

#[test]
fn cfg_c9_reverse_partial() {
    let p = pair();
    let mut r = Rng::new(0xC9_0001);
    for _ in 0..N {
        let n = r.range_i32(1, 64) as usize;
        let buf: Vec<i32> = (0..n).map(|_| r.interesting_i32()).collect();
        let end_idx = r.range_i32(0, n as i32 - 1) as usize;
        let count = r.range_i32(0, end_idx as i32 + 1);
        cmp_reverse(p, &buf, end_idx, count);
    }
}

#[test]
fn cfg_c10_reverse_overflow() {
    let p = pair();
    let mut r = Rng::new(0xCA_0001);
    for _ in 0..N {
        let n = r.range_i32(2, 32) as usize;
        // All elements huge so the running sum wraps repeatedly.
        let buf: Vec<i32> = (0..n)
            .map(|_| {
                if r.next_u64() & 1 == 0 {
                    i32::MAX - r.range_i32(0, 8)
                } else {
                    i32::MIN + r.range_i32(0, 8)
                }
            })
            .collect();
        cmp_reverse(p, &buf, n - 1, n as i32);
    }
}

// ===========================================================================
// C11/C12/C13/C14/C15 -- foreach_sum
// ===========================================================================
#[test]
fn cfg_c11_foreach_single() {
    let p = pair();
    let mut r = Rng::new(0xCB_0001);
    for _ in 0..N {
        let buf = vec![r.interesting_i32()];
        cmp_foreach(p, &buf, 1);
    }
}

#[test]
fn cfg_c12_foreach_two() {
    let p = pair();
    let mut r = Rng::new(0xCC_0001);
    for _ in 0..N {
        let buf = vec![r.interesting_i32(), r.interesting_i32()];
        cmp_foreach(p, &buf, 2);
    }
}

#[test]
fn cfg_c13_foreach_many() {
    let p = pair();
    let mut r = Rng::new(0xCD_0001);
    for _ in 0..N {
        let n = r.range_i32(3, 64) as usize;
        let buf: Vec<i32> = (0..n).map(|_| r.interesting_i32()).collect();
        cmp_foreach(p, &buf, n as i32);
        // Also a prefix walk: count < len.
        let count = r.range_i32(0, n as i32);
        cmp_foreach(p, &buf, count);
    }
}

#[test]
fn cfg_c14_foreach_overflow() {
    let p = pair();
    let mut r = Rng::new(0xCE_0001);
    for _ in 0..N {
        let n = r.range_i32(2, 32) as usize;
        let buf: Vec<i32> = (0..n).map(|_| i32::MIN + r.range_i32(0, 16)).collect();
        cmp_foreach(p, &buf, n as i32);
    }
}

/// C15 -- forward and backward traversal of the same buffer must agree, in both
/// implementations. This pins down that `FOREACH` visits each element exactly
/// once (a double-visit or skipped element would break the equality).
#[test]
fn cfg_c15_foreach_vs_reverse_equal() {
    let p = pair();
    let mut r = Rng::new(0xCF_0001);
    for _ in 0..N {
        let n = r.range_i32(1, 48) as usize;
        let buf: Vec<i32> = (0..n).map(|_| r.interesting_i32()).collect();

        let mut a = buf.clone();
        let mut b = buf.clone();
        let fwd_c = p.c.foreach_sum(a.as_mut_ptr(), n as i32);
        let fwd_rs = p.rs.foreach_sum(b.as_mut_ptr(), n as i32);
        let rev_c = unsafe { p.c.process_array_reverse(a.as_mut_ptr().add(n - 1), n as i32) };
        let rev_rs = unsafe { p.rs.process_array_reverse(b.as_mut_ptr().add(n - 1), n as i32) };

        cmp("foreach_sum (C15)", (n, "fwd"), fwd_c, fwd_rs);
        cmp("process_array_reverse (C15)", (n, "rev"), rev_c, rev_rs);
        // Wrapping addition is commutative and associative, so both directions
        // must produce the identical 32-bit sum.
        assert_eq!(fwd_c, rev_c, "C: fwd != rev for {buf:?}");
        assert_eq!(fwd_rs, rev_rs, "Rust: fwd != rev for {buf:?}");
    }
}

// ===========================================================================
// C16..C23 -- switch_fallthrough_calculator
// ===========================================================================
fn switch_row(seed: u64, operation: i32) {
    let p = pair();
    let mut r = Rng::new(seed);
    for _ in 0..N {
        cmp_switch(p, r.interesting_i32(), operation);
    }
    for v in [
        0,
        1,
        -1,
        7,
        8,
        63,
        64,
        127,
        128,
        129,
        255,
        256,
        511,
        512,
        -511,
        -512,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 8,
        i32::MAX / 8 + 1,
        i32::MIN / 8,
        i32::MIN / 8 - 1,
        i32::MAX / 3,
        i32::MAX / 3 + 1,
        i32::MIN / 3,
        i32::MIN / 3 - 1,
        i32::MAX - 128,
        i32::MAX - 127,
        i32::MAX - 64,
        i32::MAX - 63,
    ] {
        cmp_switch(p, v, operation);
    }
}

/// Overflow-focused sweep for one switch arm: values clustered around the
/// points where `*8`, `*3`, `+128` and `+64` wrap.
fn switch_overflow_row(seed: u64, operation: i32) {
    let p = pair();
    let mut r = Rng::new(seed);
    let pivots = [
        i32::MAX,
        i32::MIN,
        i32::MAX / 8,
        i32::MIN / 8,
        i32::MAX / 3,
        i32::MIN / 3,
        i32::MAX - 128,
        i32::MIN + 128,
        i32::MAX - 64,
        i32::MIN + 64,
        1 << 28,
        -(1 << 28),
        1 << 30,
        -(1 << 30),
    ];
    for &pivot in &pivots {
        for delta in -8..=8i32 {
            cmp_switch(p, pivot.wrapping_add(delta), operation);
        }
    }
    for _ in 0..N {
        let pivot = pivots[(r.next_u64() as usize) % pivots.len()];
        cmp_switch(p, pivot.wrapping_add(r.range_i32(-1000, 1000)), operation);
    }
}

/// C16 -- arm 0: `*= 8`, `+= 128`, `&= 511` (the deepest fall-through chain).
#[test]
fn cfg_c16_switch_op0() {
    switch_row(0xD0_0000, 0);
}

/// C17 -- arm 0 with operands that make `*8` and `+128` overflow before the mask.
#[test]
fn cfg_c17_switch_op0_overflow() {
    switch_overflow_row(0xD0_1000, 0);
}

#[test]
fn cfg_c18_switch_op1() {
    switch_row(0xD1_0000, 1);
}

#[test]
fn cfg_c19_switch_op2() {
    switch_row(0xD2_0000, 2);
}

/// C20 -- arm 3: `*= 3`, `+= 64`, **no** mask, so the full 32-bit result is
/// observable across the FFI boundary.
#[test]
fn cfg_c20_switch_op3() {
    switch_row(0xD3_0000, 3);
}

/// C21 -- arm 3 with operands near `INT_MAX/3` / `INT_MIN/3` so `*3` overflows.
#[test]
fn cfg_c21_switch_op3_overflow() {
    switch_overflow_row(0xD3_1000, 3);
}

#[test]
fn cfg_c22_switch_op4() {
    switch_row(0xD4_0000, 4);
}

#[test]
fn cfg_c23_switch_cross_product() {
    let p = pair();
    let mut r = Rng::new(0xD5_0000);
    // Every arm crossed with every interesting value class.
    for operation in -8..=8 {
        for _ in 0..(N / 2) {
            cmp_switch(p, r.interesting_i32(), operation);
        }
    }
    // Fully random pairs.
    for _ in 0..(N * 4) {
        cmp_switch(p, r.interesting_i32(), r.range_i32(-10, 10));
    }
    for _ in 0..(N * 2) {
        cmp_switch(p, r.next_i32(), r.next_i32());
    }
}

// ===========================================================================
// C24..C34 -- allocate_and_compute
// ===========================================================================
#[test]
fn cfg_c24_alloc_size1() {
    let p = pair();
    let mut r = Rng::new(0xE0_0000);
    cmp_alloc(p, 1, 1.5);
    for _ in 0..N {
        // size == 1 => the single term is 0*0*mult == 0 (or NaN for inf mult).
        cmp_alloc(p, 1, r.range_f64(-1e6, 1e6));
    }
}

#[test]
fn cfg_c25_alloc_size2() {
    let p = pair();
    let mut r = Rng::new(0xE1_0000);
    cmp_alloc(p, 2, 1.5);
    for _ in 0..N {
        cmp_alloc(p, 2, r.range_f64(-1e9, 1e9));
    }
}

#[test]
fn cfg_c26_alloc_small_mult_1_5() {
    let p = pair();
    // The exact range `fallcalc` can produce.
    for size in 1..=10 {
        cmp_alloc(p, size, 1.5);
    }
}

#[test]
fn cfg_c27_alloc_zero_mult() {
    let p = pair();
    for size in 0..=32 {
        cmp_alloc(p, size, 0.0);
        cmp_alloc(p, size, -0.0);
    }
}

#[test]
fn cfg_c28_alloc_negative_mult() {
    let p = pair();
    let mut r = Rng::new(0xE2_0000);
    for _ in 0..N {
        let size = r.range_i32(1, 32);
        cmp_alloc(p, size, -r.range_f64(0.0, 1e7));
    }
}

#[test]
fn cfg_c29_alloc_random_mult() {
    let p = pair();
    let mut r = Rng::new(0xE3_0000);
    for _ in 0..N {
        let size = r.range_i32(1, 40);
        cmp_alloc(p, size, r.range_f64(-1000.0, 1000.0));
    }
}

#[test]
fn cfg_c30_alloc_tiny_mult() {
    let p = pair();
    let mut r = Rng::new(0xE4_0000);
    for m in [1e-300, -1e-300, 5e-324, -5e-324, f64::MIN_POSITIVE, 1e-30] {
        for size in [1, 2, 5, 10, 33] {
            cmp_alloc(p, size, m);
        }
    }
    for _ in 0..N {
        cmp_alloc(p, r.range_i32(1, 20), r.range_f64(-1e-290, 1e-290));
    }
}

#[test]
fn cfg_c31_alloc_large_mult() {
    let p = pair();
    let mut r = Rng::new(0xE5_0000);
    for m in [1e9, -1e9, 1e18, -1e18, 1e300, -1e300, 1e-1] {
        for size in [1, 2, 3, 5, 10, 40] {
            cmp_alloc(p, size, m);
        }
    }
    for _ in 0..N {
        cmp_alloc(p, r.range_i32(1, 16), r.range_f64(1e8, 1e12));
    }
}

#[test]
fn cfg_c32_alloc_larger_sizes() {
    let p = pair();
    let mut r = Rng::new(0xE6_0000);
    for size in [64, 100, 255, 256, 1000, 4096, 65536] {
        cmp_alloc(p, size, 1.5);
        cmp_alloc(p, size, -0.25);
        cmp_alloc(p, size, r.range_f64(-100.0, 100.0));
    }
}

#[test]
fn cfg_c33_alloc_cross_product() {
    let p = pair();
    let mut r = Rng::new(0xE7_0000);
    for _ in 0..(N * 2) {
        let size = match r.next_u64() % 4 {
            0 => r.range_i32(0, 3),
            1 => r.range_i32(1, 12),
            2 => r.range_i32(1, 200),
            _ => r.range_i32(0, 2000),
        };
        let mult = match r.next_u64() % 5 {
            0 => 1.5,
            1 => r.range_f64(-1.0, 1.0),
            2 => r.range_f64(-1e12, 1e12),
            3 => r.raw_f64(),
            _ => r.range_f64(-1e-200, 1e-200),
        };
        cmp_alloc(p, size, mult);
    }
}

/// C34 -- interleave allocations between the two `.so`s. Both must be using the
/// same libc heap (`nm -D` shows `U malloc`/`U free` on both); if the Rust side
/// used Rust's global allocator, `size == 0` and failure semantics could drift.
#[test]
fn cfg_c34_alloc_interleaved_heap() {
    let p = pair();
    let mut r = Rng::new(0xE8_0000);
    for _ in 0..(N / 2) {
        let size = r.range_i32(0, 64);
        let m = r.range_f64(-10.0, 10.0);
        let c1 = p.c.allocate_and_compute(size, m);
        let r1 = p.rs.allocate_and_compute(size, m);
        let c2 = p.c.allocate_and_compute(size, m);
        let r2 = p.rs.allocate_and_compute(size, m);
        cmp("allocate_and_compute (C34 interleaved)", (size, m), c1, r1);
        cmp("allocate_and_compute (C34 interleaved)", (size, m), c2, r2);
        assert_eq!(c1, c2, "C is not deterministic");
        assert_eq!(r1, r2, "Rust is not deterministic");
    }
}

// ===========================================================================
// C35..C48 -- fallcalc (composed top-level entry point)
// ===========================================================================

/// Random `fallcalc` sweep with `param3` constrained so that `param3 % 5 == r`
/// and `param3 <= 128` (A11 false).
fn fallcalc_residue_row(seed: u64, residue: i32) {
    let p = pair();
    let mut r = Rng::new(seed);
    let mut done = 0usize;
    while done < N {
        let p3 = r.range_i32(0, 128);
        if p3 % 5 != residue {
            continue;
        }
        cmp_fallcalc(p, r.interesting_i32(), r.interesting_i32(), p3, r.interesting_i32());
        done += 1;
    }
}

#[test]
fn cfg_c35_fallcalc_res0_noflag() {
    fallcalc_residue_row(0xF0_0000, 0);
}
#[test]
fn cfg_c36_fallcalc_res1_noflag() {
    fallcalc_residue_row(0xF1_0000, 1);
}
#[test]
fn cfg_c37_fallcalc_res2_noflag() {
    fallcalc_residue_row(0xF2_0000, 2);
}
#[test]
fn cfg_c38_fallcalc_res3_noflag() {
    fallcalc_residue_row(0xF3_0000, 3);
}
#[test]
fn cfg_c39_fallcalc_res4_noflag() {
    fallcalc_residue_row(0xF4_0000, 4);
}

/// C40 -- `param3 > 0200` so `result |= 0200` fires, crossed with all residues.
#[test]
fn cfg_c40_fallcalc_flag_set_all_residues() {
    let p = pair();
    let mut r = Rng::new(0xF5_0000);
    let mut per_residue = [0usize; 5];
    while per_residue.iter().any(|&c| c < N / 5) {
        let p3 = r.range_i32(129, i32::MAX);
        let res = (p3 % 5) as usize;
        if per_residue[res] >= N / 5 {
            continue;
        }
        per_residue[res] += 1;
        cmp_fallcalc(p, r.interesting_i32(), r.interesting_i32(), p3, r.interesting_i32());
    }
    // Exactly at the boundary of the `>` comparison.
    for p3 in [127, 128, 129, 130, 131, 132, 133] {
        for _ in 0..200 {
            cmp_fallcalc(p, r.interesting_i32(), r.interesting_i32(), p3, r.interesting_i32());
        }
    }
}

/// C41 -- negative residues drive `switch_fallthrough_calculator` into
/// `default` (C `%` truncates toward zero, so `-7 % 5 == -2`).
#[test]
fn cfg_c41_fallcalc_negative_residues() {
    let p = pair();
    let mut r = Rng::new(0xF6_0000);
    let mut per_residue = [0usize; 5]; // index = -residue
    while per_residue[1..].iter().any(|&c| c < N / 4) {
        let p3 = r.range_i32(i32::MIN, -1);
        let res = (-(p3 % 5)) as usize;
        if res == 0 || per_residue[res] >= N / 4 {
            continue;
        }
        per_residue[res] += 1;
        cmp_fallcalc(p, r.interesting_i32(), r.interesting_i32(), p3, r.interesting_i32());
    }
    for p3 in [-1, -2, -3, -4, -5, -6, -7, -8, -9, -10, i32::MIN, i32::MIN + 1] {
        for _ in 0..200 {
            cmp_fallcalc(p, r.interesting_i32(), r.interesting_i32(), p3, r.interesting_i32());
        }
    }
}

/// C42 -- each allocation size `1..=10` reachable from `param4 % 10 + 1`.
#[test]
fn cfg_c42_fallcalc_each_alloc_size() {
    let p = pair();
    let mut r = Rng::new(0xF7_0000);
    for last_digit in 0..=9i32 {
        for _ in 0..(N / 2) {
            let p4 = r.range_i32(0, (i32::MAX - 9) / 10) * 10 + last_digit;
            assert_eq!(p4 % 10, last_digit);
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                p4,
            );
        }
    }
}

/// C43 -- `param4 % 10 + 1 <= 0`, so the nested `allocate_and_compute` takes
/// the malloc-failure (`-1`) or `size == 0` (`0`) path.
#[test]
fn cfg_c43_fallcalc_nonpositive_alloc_size() {
    let p = pair();
    let mut r = Rng::new(0xF8_0000);
    for last_digit in -9..=-1i32 {
        for _ in 0..(N / 4) {
            let p4 = r.range_i32((i32::MIN + 9) / 10, 0) * 10 + last_digit;
            assert_eq!(p4 % 10, last_digit, "p4 = {p4}");
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                p4,
            );
        }
    }
    for p4 in [-1, -9, -10, -11, -19, -21, i32::MIN, i32::MIN + 1, i32::MIN + 8] {
        for _ in 0..200 {
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                p4,
            );
        }
    }
}

/// C44 -- integer overflow in `param1 * 0100 + param2` and in the array fill.
#[test]
fn cfg_c44_fallcalc_int_overflow() {
    let p = pair();
    let mut r = Rng::new(0xF9_0000);
    let extremes = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 8,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 8,
        i32::MAX / 64,
        i32::MAX / 64 + 1,
        i32::MIN / 64,
        i32::MIN / 64 - 1,
        1 << 25,
        -(1 << 25),
    ];
    for &p1 in &extremes {
        for &p2 in &extremes {
            for _ in 0..8 {
                cmp_fallcalc(p, p1, p2, r.interesting_i32(), r.interesting_i32());
            }
        }
    }
    for _ in 0..N {
        cmp_fallcalc(
            p,
            extremes[(r.next_u64() as usize) % extremes.len()],
            r.next_i32(),
            r.next_i32(),
            r.next_i32(),
        );
    }
}

/// C45 -- `floating_calc` placed just inside / just outside the saturation
/// boundaries of `safe_double_to_int`.
#[test]
fn cfg_c45_fallcalc_float_saturation() {
    let p = pair();
    let mut r = Rng::new(0xFA_0000);

    // param1 * 3.7 alone can reach +-7.9e9, well past INT_MAX.
    let near_max = (2147483647.0f64 / 3.7) as i32; // ~580_400_985
    let near_min = (-2147483648.0f64 / 3.7) as i32;
    for delta in -4..=4i32 {
        for p2 in [0, 1, -1, 1000, -1000, i32::MAX, i32::MIN] {
            for p3 in [0, 1, -1, 129, -129] {
                cmp_fallcalc(p, near_max.wrapping_add(delta), p2, p3, 3);
                cmp_fallcalc(p, near_min.wrapping_add(delta), p2, p3, 3);
            }
        }
    }
    // Randomized around the saturation cliff.
    for _ in 0..N {
        let p1 = near_max.wrapping_add(r.range_i32(-1000, 1000));
        cmp_fallcalc(p, p1, r.next_i32(), r.interesting_i32(), r.interesting_i32());
        let p1 = near_min.wrapping_add(r.range_i32(-1000, 1000));
        cmp_fallcalc(p, p1, r.next_i32(), r.interesting_i32(), r.interesting_i32());
    }
    // param3 * 0.5 dominating.
    for _ in 0..N {
        cmp_fallcalc(p, 0, 0, r.next_i32(), r.interesting_i32());
    }
}

/// C46 -- exhaustive small cube: 13^4 = 28_561 calls covering every
/// A11 x A12 x A13 interaction at small magnitudes.
#[test]
fn cfg_c46_fallcalc_exhaustive_small_cube() {
    let p = pair();
    for p1 in -6..=6 {
        for p2 in -6..=6 {
            for p3 in -6..=6 {
                for p4 in -6..=6 {
                    cmp_fallcalc(p, p1, p2, p3, p4);
                }
            }
        }
    }
}

/// C47 -- full-random 32-bit sweep of all four parameters.
#[test]
fn cfg_c47_fallcalc_random_sweep() {
    let p = pair();
    let mut r = Rng::new(0xFB_0000);
    for _ in 0..25_000 {
        cmp_fallcalc(p, r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
    for _ in 0..25_000 {
        cmp_fallcalc(
            p,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// C48 -- every 4^4 combination drawn from a boundary pool.
#[test]
fn cfg_c48_fallcalc_boundary_pool() {
    let p = pair();
    const POOL: [i32; 12] = [
        i32::MIN,
        i32::MIN + 1,
        -128,
        -10,
        -1,
        0,
        1,
        5,
        10,
        128,
        129,
        i32::MAX,
    ];
    for &a in &POOL {
        for &b in &POOL {
            for &c in &POOL {
                for &d in &POOL {
                    cmp_fallcalc(p, a, b, c, d);
                }
            }
        }
    }
}

/// C49 -- recompose `fallcalc` from the low-level exports of the *same* `.so`
/// and check it reproduces that `.so`'s own `fallcalc`. This catches wiring
/// bugs in the composed pipeline that per-function tests cannot see.
#[test]
fn cfg_c49_fallcalc_recomposed_from_low_level() {
    fn recompose(im: &Impl, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        let base_value = p1.wrapping_mul(0o100).wrapping_add(p2);

        let array_size: i32 = 5;
        let mut data: Vec<i32> = (0..array_size)
            .map(|i| i.wrapping_add(1).wrapping_mul(0o10).wrapping_add(p1))
            .collect();

        let foreach_result = im.foreach_sum(data.as_mut_ptr(), array_size);
        let reverse_sum = unsafe {
            im.process_array_reverse(data.as_mut_ptr().add(array_size as usize - 1), array_size)
        };
        let switch_result = im.switch_fallthrough_calculator(p2, p3.wrapping_rem(5));

        let floating_calc = (p1 as f64) * 3.7 + (p2 as f64) * 2.3 - (p3 as f64) * 0.5;
        let converted = im.safe_double_to_int(floating_calc);

        let alloc_result = im.allocate_and_compute(p4.wrapping_rem(10).wrapping_add(1), 1.5);

        let mut result = base_value
            .wrapping_add(foreach_result)
            .wrapping_add(reverse_sum)
            .wrapping_add(switch_result)
            .wrapping_add(converted)
            .wrapping_add(alloc_result);

        if p3 > 0o200 {
            result |= 0o200;
        }
        result & 0o777
    }

    let p = pair();
    let mut r = Rng::new(0xFC_0000);
    for _ in 0..10_000 {
        let (a, b, c, d) = (
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let want_c = p.c.fallcalc(a, b, c, d);
        let want_rs = p.rs.fallcalc(a, b, c, d);
        cmp("fallcalc (C49)", (a, b, c, d), want_c, want_rs);

        assert_eq!(
            recompose(&p.c, a, b, c, d),
            want_c,
            "C recomposition mismatch for {:?}",
            (a, b, c, d)
        );
        assert_eq!(
            recompose(&p.rs, a, b, c, d),
            want_rs,
            "Rust recomposition mismatch for {:?}",
            (a, b, c, d)
        );
    }
}
