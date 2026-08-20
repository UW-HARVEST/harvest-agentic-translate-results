// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every test drives BOTH shared objects (C reference + Rust translation) through
// `libloading` and compares the returned `int` byte-for-byte. Inputs are
// randomised from a fixed-seed PRNG so failures are reproducible.

mod common;

use common::{both, eq_i32, Buf, Rng};

/// How many randomised inputs each row uses by default.
const N: usize = 2000;

// ===========================================================================
// safe_double_to_int  (rows 1-5)
// ===========================================================================

fn check_sdti(row: &str, d: f64) {
    let (c, r) = both();
    let cv = unsafe { (c.safe_double_to_int)(d) };
    let rv = unsafe { (r.safe_double_to_int)(d) };
    eq_i32(row, (d, d.to_bits()), cv, rv);
}

#[test]
fn cfg_row01_sdti_in_range_positive() {
    let mut rng = Rng::fixed();
    for _ in 0..N {
        check_sdti("row01", rng.f64_in(0.0, 2_147_483_647.0));
    }
    // plus deliberately fractional small magnitudes
    for _ in 0..N {
        check_sdti("row01", rng.f64_in(0.0, 1000.0));
    }
    for d in [0.5, 0.9999999, 1.0, 1.5, 2.5, 42.75, 1e6 + 0.5, 2_147_483_646.75] {
        check_sdti("row01", d);
    }
}

#[test]
fn cfg_row02_sdti_in_range_negative() {
    let mut rng = Rng::fixed();
    for _ in 0..N {
        check_sdti("row02", rng.f64_in(-2_147_483_648.0, 0.0));
    }
    for _ in 0..N {
        check_sdti("row02", rng.f64_in(-1000.0, 0.0));
    }
    for d in [
        -0.5,
        -0.9999999,
        -1.0,
        -1.5,
        -2.5,
        -42.75,
        -(1e6 + 0.5),
        -2_147_483_646.75,
        -2_147_483_647.5,
    ] {
        check_sdti("row02", d);
    }
}

#[test]
fn cfg_row03_sdti_zero_and_subnormal() {
    for d in [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        -f64::MIN_POSITIVE / 2.0,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        f64::EPSILON,
        -f64::EPSILON,
    ] {
        check_sdti("row03", d);
    }
}

#[test]
fn cfg_row04_sdti_boundary_sweep() {
    // Every double within a few ULPs of the two clamp boundaries.
    for base in [2_147_483_647.0f64, -2_147_483_648.0f64, 2_147_483_648.0f64, -2_147_483_649.0f64]
    {
        let mut d = base;
        for _ in 0..8 {
            d = f64::from_bits(d.to_bits().wrapping_sub(1));
        }
        for _ in 0..24 {
            check_sdti("row04", d);
            d = f64::from_bits(d.to_bits().wrapping_add(1));
        }
    }
    for d in [
        2_147_483_646.5,
        -2_147_483_646.5,
        2_147_483_647.5,
        -2_147_483_647.5,
        2_147_483_646.0,
        -2_147_483_647.0,
        0.5,
        -0.5,
        1.0,
        -1.0,
    ] {
        check_sdti("row04", d);
    }
}

#[test]
fn cfg_row05_sdti_random_bitpatterns() {
    let mut rng = Rng::fixed();
    for _ in 0..(N * 10) {
        check_sdti("row05", rng.next_f64_bits());
    }
}

// ===========================================================================
// switch_fallthrough_calculator  (rows 6-11)
// ===========================================================================

fn check_switch(row: &str, value: i32, operation: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.switch_fallthrough_calculator)(value, operation) };
    let rv = unsafe { (r.switch_fallthrough_calculator)(value, operation) };
    eq_i32(row, (value, operation), cv, rv);
}

/// Shared body for rows 6-10: one fixed `operation`, randomised + boundary `value`s.
fn switch_op_row(row: &str, op: i32) {
    let mut rng = Rng::fixed();
    for v in [
        0i32,
        1,
        -1,
        7,
        -7,
        0o100,
        0o200,
        0o777,
        -0o777,
        0x0FFF_FFFF,
        0x1000_0000,
        0x2000_0000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 3,
        i32::MIN / 3,
        i32::MAX / 8,
        i32::MIN / 8,
        715_827_883, // *3 overflows by one
        -715_827_883,
        268_435_456, // *8 overflows
        -268_435_456,
    ] {
        check_switch(row, v, op);
    }
    for _ in 0..N {
        check_switch(row, rng.next_i32(), op);
    }
    for _ in 0..N {
        check_switch(row, rng.spicy_i32(), op);
    }
    for _ in 0..N {
        check_switch(row, rng.range_i32(-1000, 1000), op);
    }
}

#[test]
fn cfg_row06_switch_op0() {
    switch_op_row("row06", 0);
}

#[test]
fn cfg_row07_switch_op1() {
    switch_op_row("row07", 1);
}

#[test]
fn cfg_row08_switch_op2() {
    switch_op_row("row08", 2);
}

#[test]
fn cfg_row09_switch_op3() {
    switch_op_row("row09", 3);
}

#[test]
fn cfg_row10_switch_op4() {
    switch_op_row("row10", 4);
}

#[test]
fn cfg_row11_switch_random_op() {
    let mut rng = Rng::fixed();
    for _ in 0..(N * 5) {
        check_switch("row11", rng.next_i32(), rng.next_i32());
    }
    // biased towards the interesting neighbourhood of the switch labels
    for _ in 0..(N * 5) {
        check_switch("row11", rng.spicy_i32(), rng.range_i32(-8, 12));
    }
}

// ===========================================================================
// foreach_sum  (rows 12-15)
// ===========================================================================

fn check_foreach(row: &str, buf: &mut Buf, count: i32) {
    let (c, r) = both();
    let p = buf.ptr();
    let cv = unsafe { (c.foreach_sum)(p, count) };
    let rv = unsafe { (r.foreach_sum)(p, count) };
    eq_i32(row, (&buf.0, count), cv, rv);
}

#[test]
fn cfg_row12_foreach_single() {
    let mut rng = Rng::fixed();
    for v in [0i32, 1, -1, i32::MAX, i32::MIN, 0o777, -0o200] {
        let mut b = Buf::new(vec![v]);
        check_foreach("row12", &mut b, 1);
    }
    for _ in 0..N {
        let mut b = Buf::random(&mut rng, 1);
        check_foreach("row12", &mut b, 1);
    }
}

#[test]
fn cfg_row13_foreach_few() {
    let mut rng = Rng::fixed();
    for n in [2usize, 3] {
        for _ in 0..N {
            let mut b = Buf::random(&mut rng, n);
            check_foreach("row13", &mut b, n as i32);
        }
    }
    // also: sum only a prefix of a longer buffer
    for _ in 0..N {
        let mut b = Buf::random(&mut rng, 8);
        check_foreach("row13", &mut b, 3);
    }
}

#[test]
fn cfg_row14_foreach_many() {
    let mut rng = Rng::fixed();
    for n in [5usize, 16, 64, 257, 1024] {
        for _ in 0..50 {
            let mut b = Buf::random(&mut rng, n);
            check_foreach("row14", &mut b, n as i32);
        }
        // prefix counts inside the same buffer
        for k in [0usize, 1, n / 2, n - 1, n] {
            let mut b = Buf::random(&mut rng, n);
            check_foreach("row14", &mut b, k as i32);
        }
    }
}

#[test]
fn cfg_row15_foreach_overflow() {
    let mut rng = Rng::fixed();
    for n in [2usize, 3, 5, 17, 64] {
        let mut b = Buf::new(vec![i32::MAX; n]);
        check_foreach("row15", &mut b, n as i32);
        let mut b = Buf::new(vec![i32::MIN; n]);
        check_foreach("row15", &mut b, n as i32);
        let mut b = Buf::new(
            (0..n)
                .map(|i| if i % 2 == 0 { i32::MAX } else { i32::MIN })
                .collect(),
        );
        check_foreach("row15", &mut b, n as i32);
        for _ in 0..200 {
            let mut b = Buf::new((0..n).map(|_| rng.spicy_i32()).collect());
            check_foreach("row15", &mut b, n as i32);
        }
    }
}

// ===========================================================================
// process_array_reverse  (rows 16-19)
// ===========================================================================

fn check_reverse(row: &str, buf: &mut Buf, start_idx: usize, count: i32) {
    let (c, r) = both();
    let p = buf.ptr_at(start_idx);
    let cv = unsafe { (c.process_array_reverse)(p, count) };
    let rv = unsafe { (r.process_array_reverse)(p, count) };
    eq_i32(row, (&buf.0, start_idx, count), cv, rv);
}

#[test]
fn cfg_row16_reverse_single() {
    let mut rng = Rng::fixed();
    for v in [0i32, 1, -1, i32::MAX, i32::MIN] {
        let mut b = Buf::new(vec![v]);
        check_reverse("row16", &mut b, 0, 1);
    }
    for _ in 0..N {
        let mut b = Buf::random(&mut rng, 4);
        // read exactly one element, from various positions
        let k = (rng.below(4)) as usize;
        check_reverse("row16", &mut b, k, 1);
    }
}

#[test]
fn cfg_row17_reverse_full_buffer() {
    let mut rng = Rng::fixed();
    for n in [2usize, 3, 5, 16, 64, 1024] {
        for _ in 0..50 {
            let mut b = Buf::random(&mut rng, n);
            check_reverse("row17", &mut b, n - 1, n as i32);
        }
    }
}

#[test]
fn cfg_row18_reverse_partial_window() {
    let mut rng = Rng::fixed();
    for n in [4usize, 9, 33, 128] {
        for _ in 0..100 {
            let mut b = Buf::random(&mut rng, n);
            let k = rng.below(n as u64) as usize; // start index
            let count = rng.below(k as u64 + 1) as i32 + 1; // 1..=k+1, stays in bounds
            check_reverse("row18", &mut b, k, count);
        }
        // zero-length window from every start position
        for k in 0..n {
            let mut b = Buf::random(&mut rng, n);
            check_reverse("row18", &mut b, k, 0);
        }
    }
}

#[test]
fn cfg_row19_reverse_overflow() {
    let mut rng = Rng::fixed();
    for n in [2usize, 5, 17, 64] {
        let mut b = Buf::new(vec![i32::MIN; n]);
        check_reverse("row19", &mut b, n - 1, n as i32);
        let mut b = Buf::new(vec![i32::MAX; n]);
        check_reverse("row19", &mut b, n - 1, n as i32);
        for _ in 0..200 {
            let mut b = Buf::new((0..n).map(|_| rng.spicy_i32()).collect());
            check_reverse("row19", &mut b, n - 1, n as i32);
        }
    }
}

#[test]
fn cfg_row20_forward_vs_reverse_same_buffer() {
    // Axis-D interaction: forward and reverse traversal of the SAME buffer must
    // agree within each library (int addition is commutative under wrap-around)
    // and across the two libraries.
    let mut rng = Rng::fixed();
    let (c, r) = both();
    for n in [1usize, 2, 5, 16, 100] {
        for _ in 0..200 {
            let mut b = Buf::new((0..n).map(|_| rng.spicy_i32()).collect());
            let base = b.ptr();
            let last = b.ptr_at(n - 1);
            let cf = unsafe { (c.foreach_sum)(base, n as i32) };
            let rf = unsafe { (r.foreach_sum)(base, n as i32) };
            let cr = unsafe { (c.process_array_reverse)(last, n as i32) };
            let rr = unsafe { (r.process_array_reverse)(last, n as i32) };
            eq_i32("row20/forward", (&b.0, n), cf, rf);
            eq_i32("row20/reverse", (&b.0, n), cr, rr);
            eq_i32("row20/c-internal", (&b.0, n), cf, cr);
            eq_i32("row20/rust-internal", (&b.0, n), rf, rr);
        }
    }
}

// ===========================================================================
// allocate_and_compute  (rows 21-27)
// ===========================================================================

fn check_alloc(row: &str, size: i32, mult: f64) {
    let (c, r) = both();
    let cv = unsafe { (c.allocate_and_compute)(size, mult) };
    let rv = unsafe { (r.allocate_and_compute)(size, mult) };
    eq_i32(row, (size, mult, mult.to_bits()), cv, rv);
}

/// Every distinct `multiplier` class the C code can be handed.
const MULT_CLASSES: [f64; 14] = [
    0.0,
    -0.0,
    1.0,
    1.5,
    -1.5,
    2.3,
    -3.7,
    1e-300,
    1e300,
    -1e300,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    f64::EPSILON,
];

const MULT_NONFINITE: [f64; 5] = [
    f64::NAN,
    -f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    // signalling NaN bit pattern
    0.0,
];

fn snan() -> f64 {
    f64::from_bits(0x7ff0_0000_0000_0001)
}

#[test]
fn cfg_row21_alloc_size_zero_all_multipliers() {
    for m in MULT_CLASSES {
        check_alloc("row21", 0, m);
    }
    for m in MULT_NONFINITE {
        check_alloc("row21", 0, m);
    }
    check_alloc("row21", 0, snan());
}

#[test]
fn cfg_row22_alloc_size_one_all_multipliers() {
    for m in MULT_CLASSES {
        check_alloc("row22", 1, m);
    }
    for m in MULT_NONFINITE {
        check_alloc("row22", 1, m);
    }
    check_alloc("row22", 1, snan());
}

#[test]
fn cfg_row23_alloc_fallcalc_range() {
    // `fallcalc` calls allocate_and_compute(param4 % 10 + 1, 1.5)
    for size in 1..=10 {
        check_alloc("row23", size, 1.5);
    }
    // and the same range against every multiplier class
    for size in 1..=10 {
        for m in MULT_CLASSES {
            check_alloc("row23", size, m);
        }
        for m in MULT_NONFINITE {
            check_alloc("row23", size, m);
        }
        check_alloc("row23", size, snan());
    }
}

#[test]
fn cfg_row24_alloc_many_sizes_random_mult() {
    let mut rng = Rng::fixed();
    for size in [2i32, 3, 5, 16, 64, 1000, 65536] {
        for _ in 0..40 {
            check_alloc("row24", size, rng.f64_in(-1e3, 1e3));
        }
        for _ in 0..40 {
            check_alloc("row24", size, rng.f64_in(-1.0, 1.0));
        }
        for _ in 0..10 {
            check_alloc("row24", size, rng.f64_in(-1e12, 1e12));
        }
    }
}

#[test]
fn cfg_row25_alloc_sum_saturates() {
    // sum = SUM_i (8i) * (i * m) = 8m * SUM i^2 -> clamps for large |m| or size
    for (size, m) in [
        (2i32, 1e12),
        (2, -1e12),
        (10, 1e9),
        (10, -1e9),
        (1000, 1e6),
        (1000, -1e6),
        (65536, 1.0),
        (65536, -1.0),
        (65536, 1e300),
        (65536, -1e300),
        (2, f64::MAX),
        (2, f64::MIN),
        (100, f64::MAX),
        (100, f64::MIN),
    ] {
        check_alloc("row25", size, m);
    }
}

#[test]
fn cfg_row26_alloc_nonfinite_sum() {
    for size in [2i32, 3, 7, 64] {
        for m in [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            snan(),
            f64::from_bits(0xfff8_0000_0000_0001),
        ] {
            check_alloc("row26", size, m);
        }
    }
}

#[test]
fn cfg_row27_alloc_random_bitpattern_mult() {
    let mut rng = Rng::fixed();
    for size in [1i32, 2, 7] {
        for _ in 0..600 {
            check_alloc("row27", size, rng.next_f64_bits());
        }
    }
}

// ===========================================================================
// fallcalc  (rows 28-35)
// ===========================================================================

fn check_fallcalc(row: &str, p1: i32, p2: i32, p3: i32, p4: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
    let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
    eq_i32(row, (p1, p2, p3, p4), cv, rv);
    // Structural invariant from the C: `result &= 0777` is the last statement.
    assert!(
        (0..=0o777).contains(&cv),
        "[{row}] C fallcalc({p1},{p2},{p3},{p4}) = {cv} is outside 0..=0777"
    );
}

#[test]
fn cfg_row28_fallcalc_small_exhaustive_grid() {
    for p1 in -3..=3 {
        for p2 in -3..=3 {
            for p3 in -6..=6 {
                for p4 in -12..=12 {
                    check_fallcalc("row28", p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_row29_fallcalc_flag_boundary() {
    let mut rng = Rng::fixed();
    for p3 in 120..=136 {
        for _ in 0..40 {
            check_fallcalc("row29", rng.spicy_i32(), rng.spicy_i32(), p3, rng.spicy_i32());
        }
        for p4 in -12..=12 {
            check_fallcalc("row29", 1, 2, p3, p4);
        }
    }
    // exactly at, one below and one above OCTAL_FLAG
    for p3 in [127, 128, 129] {
        for p1 in -2..=2 {
            for p2 in -2..=2 {
                for p4 in -2..=12 {
                    check_fallcalc("row29", p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_row30_fallcalc_all_switch_submodes() {
    let mut rng = Rng::fixed();
    // param3 values realising every value of `param3 % 5`
    let p3s: [i32; 18] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -2, -3, -4, -5, -6, -9, i32::MIN,
    ];
    for p3 in p3s {
        for _ in 0..200 {
            check_fallcalc("row30", rng.spicy_i32(), rng.spicy_i32(), p3, rng.spicy_i32());
        }
    }
    // and large param3 that also trips the OCTAL_FLAG branch, one per residue
    for k in 0..5 {
        let p3 = 1000 + k; // 1000 % 5 == 0
        for _ in 0..200 {
            check_fallcalc("row30", rng.spicy_i32(), rng.spicy_i32(), p3, rng.spicy_i32());
        }
    }
}

#[test]
fn cfg_row31_fallcalc_all_alloc_submodes() {
    let mut rng = Rng::fixed();
    // param4 realising param4 % 10 + 1 == 1..10, 0, and -1..-8
    let mut p4s: Vec<i32> = (0..=9).collect(); // -> 1..10
    p4s.extend([-1i32, -11, -21]); // -> 0
    p4s.extend([-2i32, -3, -4, -5, -6, -7, -8, -9]); // -> -1..-8
    p4s.extend([i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 100, -100]);
    for p4 in p4s {
        for _ in 0..200 {
            check_fallcalc("row31", rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), p4);
        }
    }
}

#[test]
fn cfg_row32_fallcalc_overflow_params() {
    let mut rng = Rng::fixed();
    let extremes: [i32; 14] = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 24,
        -(1 << 24),
        1 << 25,
        -(1 << 25),
        i32::MAX / 0o100,
        i32::MIN / 0o100,
        i32::MAX / 0o100 + 1,
        i32::MIN / 0o100 - 1,
        0x0080_0000,
        -0x0080_0000,
    ];
    for p1 in extremes {
        for p2 in extremes {
            for p3 in [0, 1, 2, 3, 4, -1, 129, i32::MAX, i32::MIN] {
                check_fallcalc("row32", p1, p2, p3, 3);
            }
        }
    }
    for _ in 0..2000 {
        let p1 = extremes[rng.below(extremes.len() as u64) as usize];
        let p2 = extremes[rng.below(extremes.len() as u64) as usize];
        check_fallcalc("row32", p1, p2, rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn cfg_row33_fallcalc_float_saturation_path() {
    // floating_calc = p1*3.7 + p2*2.3 - p3*0.5 ; saturate on both ends
    let big = [
        i32::MAX,
        i32::MIN,
        1_000_000_000,
        -1_000_000_000,
        600_000_000,
        -600_000_000,
        580_000_000,
        -580_000_000,
    ];
    for p1 in big {
        for p2 in big {
            for p3 in big {
                check_fallcalc("row33", p1, p2, p3, 5);
            }
        }
    }
    // exactly around the INT_MAX / INT_MIN clamp of safe_double_to_int
    for p1 in 580_400_000..580_400_020 {
        check_fallcalc("row33", p1, 0, 0, 0);
        check_fallcalc("row33", -p1, 0, 0, 0);
    }
}

#[test]
fn cfg_row34_fallcalc_random_full_range() {
    let mut rng = Rng::fixed();
    for _ in 0..10_000 {
        check_fallcalc(
            "row34",
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn cfg_row35_fallcalc_boundary_pool() {
    let mut rng = Rng::fixed();
    for _ in 0..10_000 {
        check_fallcalc(
            "row35",
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
    }
    // small-magnitude sweep around every switch/flag/alloc boundary at once
    for p3 in [-5, -1, 0, 4, 5, 127, 128, 129, 130] {
        for p4 in [-11, -9, -1, 0, 9, 10, 19, 20] {
            for p1 in [-1, 0, 1, 0o100, -0o100] {
                for p2 in [-1, 0, 1, 0o200, -0o200] {
                    check_fallcalc("row35", p1, p2, p3, p4);
                }
            }
        }
    }
}

// ===========================================================================
// Composed pipeline + shared buffers  (rows 36-37)
// ===========================================================================

#[test]
fn cfg_row36_manual_pipeline_matches_fallcalc() {
    // Re-implement `fallcalc`'s body using ONLY the low-level exported entry
    // points of each library, then require that the manual composition equals
    // that library's own `fallcalc`, and that both libraries agree.
    let (c, r) = both();
    let mut rng = Rng::fixed();

    let run = |p1: i32, p2: i32, p3: i32, p4: i32| {
        let base_value = p1.wrapping_mul(0o100).wrapping_add(p2);
        let array_size: i32 = 5;
        let mut buf = Buf::new(
            (0..array_size)
                .map(|i| (i.wrapping_add(1)).wrapping_mul(0o10).wrapping_add(p1))
                .collect(),
        );
        let head = buf.ptr();
        let last = buf.ptr_at(array_size as usize - 1);
        let floating_calc = (p1 as f64) * 3.7 + (p2 as f64) * 2.3 - (p3 as f64) * 0.5;

        for imp in [c, r] {
            let foreach_result = unsafe { (imp.foreach_sum)(head, array_size) };
            let reverse_sum = unsafe { (imp.process_array_reverse)(last, array_size) };
            let switch_result =
                unsafe { (imp.switch_fallthrough_calculator)(p2, p3 % 5) };
            let converted = unsafe { (imp.safe_double_to_int)(floating_calc) };
            let alloc_result = unsafe { (imp.allocate_and_compute)(p4 % 10 + 1, 1.5) };

            let mut result = base_value
                .wrapping_add(foreach_result)
                .wrapping_add(reverse_sum)
                .wrapping_add(switch_result)
                .wrapping_add(converted)
                .wrapping_add(alloc_result);
            if p3 > 0o200 {
                result |= 0o200;
            }
            result &= 0o777;

            let oneshot = unsafe { (imp.fallcalc)(p1, p2, p3, p4) };
            eq_i32(
                &format!("row36/{}-manual-vs-oneshot", imp.name),
                (p1, p2, p3, p4),
                result,
                oneshot,
            );
        }
    };

    for p3 in [-7, -1, 0, 1, 2, 3, 4, 127, 128, 129, 1000] {
        for p4 in [-21, -9, -1, 0, 1, 5, 9, 10, 11] {
            run(3, 7, p3, p4);
            run(-3, -7, p3, p4);
            run(i32::MAX, i32::MIN, p3, p4);
        }
    }
    for _ in 0..3000 {
        run(
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
    }
}

#[test]
fn cfg_row37_shared_caller_owned_buffer() {
    // One caller-owned buffer, alternately read by the C `.so` and the Rust
    // `.so` (interleaved), proving there is no hidden state or ABI mismatch.
    let (c, r) = both();
    let mut rng = Rng::fixed();
    for n in [1usize, 2, 5, 33, 512] {
        let mut b = Buf::random(&mut rng, n);
        for _round in 0..20 {
            // mutate the buffer between rounds
            for v in b.0.iter_mut() {
                *v = rng.spicy_i32();
            }
            let head = b.ptr();
            let last = b.ptr_at(n - 1);
            let seq_c = [
                unsafe { (c.foreach_sum)(head, n as i32) },
                unsafe { (c.process_array_reverse)(last, n as i32) },
                unsafe { (c.foreach_sum)(head, (n / 2) as i32) },
                unsafe { (c.process_array_reverse)(b.ptr_at(n / 2), (n / 2 + 1) as i32) },
            ];
            let seq_r = [
                unsafe { (r.foreach_sum)(head, n as i32) },
                unsafe { (r.process_array_reverse)(last, n as i32) },
                unsafe { (r.foreach_sum)(head, (n / 2) as i32) },
                unsafe { (r.process_array_reverse)(b.ptr_at(n / 2), (n / 2 + 1) as i32) },
            ];
            for (i, (cv, rv)) in seq_c.iter().zip(seq_r.iter()).enumerate() {
                eq_i32(&format!("row37/step{i}"), (n, &b.0), *cv, *rv);
            }
        }
    }
}
