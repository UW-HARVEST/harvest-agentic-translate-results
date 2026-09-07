// Phase B — valid-path differential tests, one test per CONFIGS.md row.
//
// Every test drives BOTH shared objects through their exported `my_pow` symbol
// and compares the returned bit pattern, the caller-visible `errno`, and the
// exact stderr bytes. Inputs are randomized with a fixed seed.

mod common;

use common::*;

const N: usize = 2000;

// ---------------------------------------------------------------------------
// C1 — positive normal base > 1, positive even integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c01_pos_base_gt1_even_positive_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1.0, 100.0);
            let e = ((r.below(20) + 1) * 2) as f64; // 2,4,...,40
            (base, e)
        })
        .collect();
    assert_all_match("C1 base>1 even +int exp", cases);
}

// ---------------------------------------------------------------------------
// C2 — positive normal base > 1, positive odd integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c02_pos_base_gt1_odd_positive_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1.0, 100.0);
            let e = (r.below(20) * 2 + 1) as f64; // 1,3,...,39
            (base, e)
        })
        .collect();
    assert_all_match("C2 base>1 odd +int exp", cases);
}

// ---------------------------------------------------------------------------
// C3 — positive normal base > 1, positive non-integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c03_pos_base_gt1_nonintegral_positive_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1.0, 100.0);
            let mut e = r.range(0.0, 40.0);
            if e.fract() == 0.0 {
                e += 0.37;
            }
            (base, e)
        })
        .collect();
    assert_all_match("C3 base>1 non-int +exp", cases);
}

// ---------------------------------------------------------------------------
// C4 — positive normal base > 1, negative integral exponent (even and odd)
// ---------------------------------------------------------------------------
#[test]
fn c04_pos_base_gt1_negative_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1.0, 100.0);
            let e = if r.bool() {
                -(((r.below(20) + 1) * 2) as f64)
            } else {
                -((r.below(20) * 2 + 1) as f64)
            };
            (base, e)
        })
        .collect();
    assert_all_match("C4 base>1 -int exp", cases);
}

// ---------------------------------------------------------------------------
// C5 — positive normal base > 1, negative non-integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c05_pos_base_gt1_negative_nonintegral_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1.0, 100.0);
            let mut e = -r.range(0.0, 40.0);
            if e.fract() == 0.0 {
                e -= 0.61;
            }
            (base, e)
        })
        .collect();
    assert_all_match("C5 base>1 -non-int exp", cases);
}

// ---------------------------------------------------------------------------
// C6 — positive normal base < 1, positive integral / non-integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c06_pos_base_lt1_positive_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(f64::MIN_POSITIVE, 1.0).max(1e-320);
            let e = if r.bool() {
                r.below(60) as f64
            } else {
                r.range(0.0, 60.0)
            };
            (base, e)
        })
        .collect();
    assert_all_match("C6 base<1 +exp", cases);
}

// ---------------------------------------------------------------------------
// C7 — positive normal base < 1, negative exponent (result grows)
// ---------------------------------------------------------------------------
#[test]
fn c07_pos_base_lt1_negative_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = r.range(1e-30, 1.0);
            let e = if r.bool() {
                -(r.below(60) as f64)
            } else {
                -r.range(0.0, 60.0)
            };
            (base, e)
        })
        .collect();
    assert_all_match("C7 base<1 -exp", cases);
}

// ---------------------------------------------------------------------------
// C8 — negative base, even integral exponent (positive result, no EDOM)
// ---------------------------------------------------------------------------
#[test]
fn c08_neg_base_even_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = -r.range(0.001, 100.0);
            let e = ((r.below(20) + 1) * 2) as f64;
            (base, e)
        })
        .collect();
    assert_all_match("C8 neg base even int exp", cases);
}

// ---------------------------------------------------------------------------
// C9 — negative base, odd integral exponent (negative result, no EDOM)
// ---------------------------------------------------------------------------
#[test]
fn c09_neg_base_odd_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = -r.range(0.001, 100.0);
            let e = (r.below(20) * 2 + 1) as f64;
            (base, e)
        })
        .collect();
    assert_all_match("C9 neg base odd int exp", cases);
}

// ---------------------------------------------------------------------------
// C10 — negative base, negative integral exponent (even / odd)
// ---------------------------------------------------------------------------
#[test]
fn c10_neg_base_negative_int_exp() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..N)
        .map(|_| {
            let base = -r.range(0.001, 100.0);
            let e = if r.bool() { r.even_int() } else { r.odd_int() };
            (base, -e.abs())
        })
        .collect();
    assert_all_match("C10 neg base -int exp", cases);
}

// ---------------------------------------------------------------------------
// C11 — base == 1.0 with arbitrary exponent (incl. NaN / ±inf / huge)
// ---------------------------------------------------------------------------
#[test]
fn c11_base_one_arbitrary_exp() {
    let mut r = Rng::new();
    let specials = special_doubles();
    let mut cases: Vec<(f64, f64)> = specials.iter().map(|&e| (1.0f64, e)).collect();
    for _ in 0..N {
        cases.push((1.0, r.any_f64()));
    }
    assert_all_match("C11 base==1", cases);
}

// ---------------------------------------------------------------------------
// C12 — base == -1.0 with integral / ±inf / NaN exponent
// ---------------------------------------------------------------------------
#[test]
fn c12_base_minus_one_arbitrary_exp() {
    let mut r = Rng::new();
    let specials = special_doubles();
    let mut cases: Vec<(f64, f64)> = specials.iter().map(|&e| (-1.0f64, e)).collect();
    for _ in 0..N {
        let e = if r.bool() {
            (r.below(200) as i64 - 100) as f64
        } else {
            r.any_f64()
        };
        cases.push((-1.0, e));
    }
    assert_all_match("C12 base==-1", cases);
}

// ---------------------------------------------------------------------------
// C13 — exponent == ±0.0 with arbitrary base
// ---------------------------------------------------------------------------
#[test]
fn c13_exp_zero_arbitrary_base() {
    let mut r = Rng::new();
    let specials = special_doubles();
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for &b in &specials {
        cases.push((b, 0.0));
        cases.push((b, -0.0));
    }
    for _ in 0..N {
        let b = r.any_f64();
        cases.push((b, if r.bool() { 0.0 } else { -0.0 }));
    }
    assert_all_match("C13 exp==±0", cases);
}

// ---------------------------------------------------------------------------
// C14 — exponent == ±1.0 with arbitrary base
// ---------------------------------------------------------------------------
#[test]
fn c14_exp_plus_minus_one_arbitrary_base() {
    let mut r = Rng::new();
    let specials = special_doubles();
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for &b in &specials {
        cases.push((b, 1.0));
        cases.push((b, -1.0));
    }
    for _ in 0..N {
        let b = r.any_f64();
        cases.push((b, if r.bool() { 1.0 } else { -1.0 }));
    }
    assert_all_match("C14 exp==±1", cases);
}

// ---------------------------------------------------------------------------
// C15 — exponent == ±0.5 (square root) with positive base
// ---------------------------------------------------------------------------
#[test]
fn c15_exp_half_positive_base() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for _ in 0..N {
        let b = r.range(0.0, 1e10);
        cases.push((b, 0.5));
        cases.push((b, -0.5));
    }
    for &b in &special_doubles() {
        if b.is_sign_positive() {
            cases.push((b, 0.5));
            cases.push((b, -0.5));
        }
    }
    assert_all_match("C15 exp==±0.5", cases);
}

// ---------------------------------------------------------------------------
// C16 — base == +0.0, positive exponent (int, non-int, +inf)
// ---------------------------------------------------------------------------
#[test]
fn c16_base_plus_zero_positive_exp() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (0.0, 1.0),
        (0.0, 2.0),
        (0.0, 3.0),
        (0.0, 0.5),
        (0.0, 1.5),
        (0.0, f64::INFINITY),
        (0.0, f64::MAX),
        (0.0, f64::MIN_POSITIVE),
    ];
    for _ in 0..N {
        cases.push((0.0, r.range(0.0, 1000.0)));
    }
    assert_all_match("C16 base==+0, +exp", cases);
}

// ---------------------------------------------------------------------------
// C17 — base == -0.0, positive exponent (odd int -> -0.0, else +0.0)
// ---------------------------------------------------------------------------
#[test]
fn c17_base_minus_zero_positive_exp() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-0.0, 1.0),
        (-0.0, 2.0),
        (-0.0, 3.0),
        (-0.0, 4.0),
        (-0.0, 0.5),
        (-0.0, 1.5),
        (-0.0, f64::INFINITY),
        (-0.0, f64::MAX),
        (-0.0, 9007199254740992.0),
        (-0.0, 9007199254740993.0),
    ];
    for _ in 0..N {
        let e = if r.bool() {
            (r.below(100) + 1) as f64
        } else {
            r.range(0.0, 1000.0)
        };
        cases.push((-0.0, e));
    }
    assert_all_match("C17 base==-0, +exp", cases);
}

// ---------------------------------------------------------------------------
// C18 — base == +inf
// ---------------------------------------------------------------------------
#[test]
fn c18_base_plus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> =
        special_doubles().iter().map(|&e| (f64::INFINITY, e)).collect();
    for _ in 0..N {
        cases.push((f64::INFINITY, r.any_f64()));
        cases.push((f64::INFINITY, r.range(-500.0, 500.0)));
    }
    assert_all_match("C18 base==+inf", cases);
}

// ---------------------------------------------------------------------------
// C19 — base == -inf
// ---------------------------------------------------------------------------
#[test]
fn c19_base_minus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = special_doubles()
        .iter()
        .map(|&e| (f64::NEG_INFINITY, e))
        .collect();
    for _ in 0..N {
        cases.push((f64::NEG_INFINITY, r.any_f64()));
        cases.push((f64::NEG_INFINITY, r.even_int()));
        cases.push((f64::NEG_INFINITY, r.odd_int()));
        cases.push((f64::NEG_INFINITY, r.range(-500.0, 500.0)));
    }
    assert_all_match("C19 base==-inf", cases);
}

// ---------------------------------------------------------------------------
// C20 — exponent == +inf, all base classes
// ---------------------------------------------------------------------------
#[test]
fn c20_exp_plus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> =
        special_doubles().iter().map(|&b| (b, f64::INFINITY)).collect();
    for _ in 0..N {
        cases.push((r.any_f64(), f64::INFINITY));
        cases.push((r.range(-2.0, 2.0), f64::INFINITY));
    }
    assert_all_match("C20 exp==+inf", cases);
}

// ---------------------------------------------------------------------------
// C21 — exponent == -inf, all base classes
// ---------------------------------------------------------------------------
#[test]
fn c21_exp_minus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = special_doubles()
        .iter()
        .map(|&b| (b, f64::NEG_INFINITY))
        .collect();
    for _ in 0..N {
        cases.push((r.any_f64(), f64::NEG_INFINITY));
        cases.push((r.range(-2.0, 2.0), f64::NEG_INFINITY));
    }
    assert_all_match("C21 exp==-inf", cases);
}

// ---------------------------------------------------------------------------
// C22 — NaN operands, incl. distinct quiet payloads and signalling patterns
// ---------------------------------------------------------------------------
#[test]
fn c22_nan_operands_and_payloads() {
    let mut r = Rng::new();
    let nans: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0001),
        f64::from_bits(0x7ff8_dead_beef_cafe),
        f64::from_bits(0xfff8_dead_beef_cafe),
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling
        f64::from_bits(0xfff0_0000_0000_0001), // signalling
        f64::from_bits(0x7ff4_0000_0000_0000), // signalling
        f64::from_bits(0x7ff_f_ffff_ffff_ffffu64 & 0x7fff_ffff_ffff_ffff),
    ];
    let others = special_doubles();
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for &n in &nans {
        for &o in &others {
            cases.push((n, o));
            cases.push((o, n));
        }
        for &m in &nans {
            cases.push((n, m));
        }
    }
    for _ in 0..N {
        // random NaN payloads
        let nb = 0x7ff8_0000_0000_0000u64 | (r.next_u64() & 0x8007_ffff_ffff_ffff);
        cases.push((f64::from_bits(nb), r.any_f64()));
        cases.push((r.any_f64(), f64::from_bits(nb)));
    }
    assert_all_match("C22 NaN operands/payloads", cases);
}

// ---------------------------------------------------------------------------
// C23 — subnormal base (positive and negative), assorted exponents
// ---------------------------------------------------------------------------
#[test]
fn c23_subnormal_base() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for &sb in &[
        1u64,
        2,
        0x000f_ffff_ffff_ffff,
        0x0008_0000_0000_0000,
        0x8000_0000_0000_0001,
        0x800f_ffff_ffff_ffff,
    ] {
        let b = f64::from_bits(sb);
        for &e in &special_doubles() {
            cases.push((b, e));
        }
    }
    for _ in 0..N {
        // random subnormal magnitude, random sign
        let mant = (r.next_u64() & 0x000f_ffff_ffff_ffff).max(1);
        let sign = if r.bool() { 0x8000_0000_0000_0000u64 } else { 0 };
        let b = f64::from_bits(sign | mant);
        let e = match r.below(4) {
            0 => r.even_int(),
            1 => r.odd_int(),
            2 => r.range(-10.0, 10.0),
            _ => r.any_f64(),
        };
        cases.push((b, e));
    }
    assert_all_match("C23 subnormal base", cases);
}

// ---------------------------------------------------------------------------
// C24 — exponent producing a subnormal, nonzero result (no ERANGE)
// ---------------------------------------------------------------------------
#[test]
fn c24_subnormal_result_band() {
    let mut cases: Vec<(f64, f64)> = Vec::new();
    // 2^-1023 .. 2^-1074 are exactly the subnormal powers of two.
    for k in 1022..=1080 {
        cases.push((2.0, -(k as f64)));
        cases.push((-2.0, -(k as f64)));
        cases.push((0.5, k as f64));
        cases.push((-0.5, k as f64));
    }
    // non-integral exponents landing in the subnormal band
    let mut r = Rng::new();
    for _ in 0..N {
        let e = -r.range(1020.0, 1085.0);
        cases.push((2.0, e));
        cases.push((10.0, -r.range(300.0, 330.0)));
    }
    assert_all_match("C24 subnormal result band", cases);
}

// ---------------------------------------------------------------------------
// C25 — boundary at the smallest normal
// ---------------------------------------------------------------------------
#[test]
fn c25_smallest_normal_boundary() {
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for k in 1018..=1026 {
        cases.push((2.0, -(k as f64)));
    }
    for &b in &[
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(0x0010_0000_0000_0001),
        f64::from_bits(0x000f_ffff_ffff_ffff),
    ] {
        for &e in &[1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.0, -3.0, 0.0, -0.0] {
            cases.push((b, e));
        }
    }
    let mut r = Rng::new();
    for _ in 0..N {
        cases.push((f64::MIN_POSITIVE, r.range(-2.0, 2.0)));
        cases.push((2.0, -r.range(1015.0, 1030.0)));
    }
    assert_all_match("C25 smallest-normal boundary", cases);
}

// ---------------------------------------------------------------------------
// C26 — boundary at the largest finite (result finite, no ERANGE)
// ---------------------------------------------------------------------------
#[test]
fn c26_largest_finite_boundary() {
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for k in 1020..=1030 {
        cases.push((2.0, k as f64));
        cases.push((-2.0, k as f64));
    }
    for &b in &[
        f64::MAX,
        f64::MIN,
        f64::from_bits(0x7fef_ffff_ffff_fffe),
        1e308,
        -1e308,
    ] {
        for &e in &[1.0, -1.0, 0.5, -0.5, 0.0, -0.0, 2.0, -2.0, 0.9999999, 1.0000001] {
            cases.push((b, e));
        }
    }
    let mut r = Rng::new();
    for _ in 0..N {
        cases.push((2.0, r.range(1015.0, 1030.0)));
        cases.push((f64::MAX, r.range(-2.0, 2.0)));
        cases.push((r.range(1e300, f64::MAX), r.range(0.9, 1.1)));
    }
    assert_all_match("C26 largest-finite boundary", cases);
}

// ---------------------------------------------------------------------------
// C27 — huge integral exponents beyond exact-integer precision
// ---------------------------------------------------------------------------
#[test]
fn c27_huge_integral_exponents() {
    let huge = [
        1e17f64,
        -1e17,
        9007199254740992.0,
        -9007199254740992.0,
        9007199254740993.0,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        1e22,
        -1e22,
    ];
    let bases = [
        1.0f64, -1.0, 2.0, -2.0, 0.5, -0.5, 0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY,
        f64::NAN, 1.0000001, 0.9999999, f64::MAX, f64::MIN_POSITIVE,
    ];
    let mut cases: Vec<(f64, f64)> = Vec::new();
    for &b in &bases {
        for &e in &huge {
            cases.push((b, e));
        }
    }
    assert_all_match("C27 huge integral exponents", cases);
}

// ---------------------------------------------------------------------------
// C28 — fully unstructured random bit patterns for both operands
// ---------------------------------------------------------------------------
#[test]
fn c28_uniform_random_bit_patterns() {
    let mut r = Rng::new();
    let cases: Vec<(f64, f64)> = (0..200_000).map(|_| (r.any_f64(), r.any_f64())).collect();
    assert_all_match("C28 uniform random bits x200000", cases);
}

// ---------------------------------------------------------------------------
// C29 — exhaustive cross-product of the curated special-value pool
// ---------------------------------------------------------------------------
#[test]
fn c29_special_value_cross_product() {
    let s = special_doubles();
    let mut cases: Vec<(f64, f64)> = Vec::with_capacity(s.len() * s.len());
    for &b in &s {
        for &e in &s {
            cases.push((b, e));
        }
    }
    assert_all_match("C29 special x special cross-product", cases);
}

// ---------------------------------------------------------------------------
// C30 — stale errno preset by the caller before a valid call
// ---------------------------------------------------------------------------
#[test]
fn c30_stale_errno_is_reset() {
    let mut r = Rng::new();
    let presets = [0, 33 /*EDOM*/, 34 /*ERANGE*/, 22 /*EINVAL*/, -1, 9999];
    let mut cases: Vec<(f64, f64, i32)> = Vec::new();
    for &pe in &presets {
        // valid inputs
        for &(b, e) in &[
            (2.0f64, 10.0f64),
            (-2.0, 3.0),
            (-2.0, 4.0),
            (1.0, f64::NAN),
            (f64::NAN, 0.0),
            (0.0, 2.0),
            (-1.0, 3.0),
            (2.0, -1040.0),
        ] {
            cases.push((b, e, pe));
        }
        // and error-triggering inputs, to confirm the error path is identical
        for &(b, e) in &[(-2.0f64, 0.5f64), (1e300, 3.0), (1e-300, 3.0), (0.0, -3.0)] {
            cases.push((b, e, pe));
        }
        for _ in 0..200 {
            cases.push((r.any_f64(), r.any_f64(), pe));
        }
    }
    assert_all_match_with_errno("C30 stale errno", cases);
}

// ---------------------------------------------------------------------------
// C31 — interleaved error / valid calls: no state must leak between calls
// ---------------------------------------------------------------------------
#[test]
fn c31_interleaved_error_and_valid_calls() {
    let p = Pair::load();
    let mut cap = StderrCapture::new();
    let seq: Vec<(f64, f64)> = vec![
        (-2.0, 0.5),   // EDOM
        (2.0, 10.0),   // valid
        (1e300, 3.0),  // ERANGE overflow
        (-2.0, 3.0),   // valid
        (1e-300, 3.0), // ERANGE underflow
        (1.0, f64::NAN),
        (0.0, -3.0), // ERANGE pole
        (2.0, -1040.0),
        (-2.0, 0.5), // EDOM again
        (-1.0, 3.0), // valid, genuine -1.0 result
        (2.0, 0.5),
        (-0.0, -3.0), // ERANGE pole
        (4.0, 0.5),
    ];
    let mut diffs = Vec::new();
    // Run the sequence several times to catch any accumulating state.
    for _round in 0..50 {
        for &(b, e) in &seq {
            if let Some(d) = diff_once(&mut cap, &p, b, e, 0) {
                if diffs.len() < 25 {
                    diffs.push(d);
                }
            }
        }
    }
    drop(cap);
    if !diffs.is_empty() {
        let mut msg = String::from("[C31 interleaved] divergences:\n");
        for d in &diffs {
            msg.push_str(&format!("  - {}\n", d));
        }
        panic!("{}", msg);
    }
    println!("[C31 interleaved] 50 rounds x {} calls matched", seq.len());
}

// ---------------------------------------------------------------------------
// C32 — happy-path inputs whose genuine result is exactly -1.0
// ---------------------------------------------------------------------------
#[test]
fn c32_genuine_minus_one_result_no_stderr() {
    let p = Pair::load();
    let mut cap = StderrCapture::new();
    let cases: Vec<(f64, f64)> = vec![
        (-1.0, 1.0),
        (-1.0, 3.0),
        (-1.0, 5.0),
        (-1.0, 7.0),
        (-1.0, -1.0),
        (-1.0, -3.0),
        (-1.0, 9007199254740993.0),
    ];
    let mut diffs = Vec::new();
    let mut obs = Vec::new();
    for &(b, e) in &cases {
        if let Some(d) = diff_once(&mut cap, &p, b, e, 0) {
            diffs.push(d);
        } else {
            obs.push((b, e, observe(&mut cap, p.c, b, e, 0)));
        }
    }
    drop(cap);
    if !diffs.is_empty() {
        let mut msg = String::from("[C32 -1.0 sentinel aliasing] divergences:\n");
        for d in &diffs {
            msg.push_str(&format!("  - {}\n", d));
        }
        panic!("{}", msg);
    }
    // Sanity: these must be genuine results, i.e. produced with NO diagnostic.
    for (b, e, o) in &obs {
        assert!(
            o.stderr.is_empty(),
            "expected no diagnostic for my_pow({}, {}), got {:?}",
            b,
            e,
            String::from_utf8_lossy(&o.stderr)
        );
    }
    println!("[C32 -1.0 sentinel aliasing] {} cases matched", cases.len());
}
