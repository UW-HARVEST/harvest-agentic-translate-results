//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1..32). Every test drives BOTH `.so`
//! exports over many randomized inputs from the fixed seed `common::SEED` and
//! asserts the stdout bytes match exactly.

mod common;

use common::*;

/// `bits` moved `steps` representable values away (toward +inf for positive
/// `steps`), operating on the raw pattern the way `nextafter` does for finite
/// same-sign values.
fn ulp_step(bits: u64, steps: i64) -> u64 {
    if steps >= 0 {
        bits.wrapping_add(steps as u64)
    } else {
        bits.wrapping_sub((-steps) as u64)
    }
}

fn nonzero_mantissa(rng: &mut Rng) -> u64 {
    loop {
        let m = rng.next_u64() & MANT_MASK;
        if m != 0 {
            return m;
        }
    }
}

// --- row 1 ------------------------------------------------------------------

#[test]
fn cfg_01_signed_zeros() {
    check_row(
        "CONFIGS row 1 (expfield=0, mantissa=0, both signs)",
        &[bits_of(false, 0, 0), bits_of(true, 0, 0)],
    );
}

// --- rows 2, 3 --------------------------------------------------------------

#[test]
fn cfg_02_positive_subnormals() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut v: Vec<u64> = (0..4096).map(|_| bits_of(false, 0, nonzero_mantissa(&mut rng))).collect();
    v.push(bits_of(false, 0, 1));
    v.push(bits_of(false, 0, MANT_MASK));
    check_row("CONFIGS row 2 (positive subnormals)", &v);
}

#[test]
fn cfg_03_negative_subnormals() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut v: Vec<u64> = (0..4096).map(|_| bits_of(true, 0, nonzero_mantissa(&mut rng))).collect();
    v.push(bits_of(true, 0, 1));
    v.push(bits_of(true, 0, MANT_MASK));
    check_row("CONFIGS row 3 (negative subnormals)", &v);
}

// --- row 4 ------------------------------------------------------------------

#[test]
fn cfg_04_single_bit_subnormals_exhaustive() {
    let mut v = Vec::new();
    for i in 0..52u32 {
        for sign in [false, true] {
            v.push(bits_of(sign, 0, 1u64 << i));
        }
    }
    assert_eq!(v.len(), 104);
    check_row("CONFIGS row 4 (single-bit subnormals, exhaustive)", &v);
}

// --- row 5 ------------------------------------------------------------------

#[test]
fn cfg_05_subnormal_trailing_zero_nibbles() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut v = Vec::new();
    for k in 1..=12u32 {
        let shift = 4 * k;
        for _ in 0..96 {
            let high = (rng.next_u64() & (MANT_MASK >> shift)) | 1;
            let m = high << shift;
            if m == 0 {
                continue;
            }
            for sign in [false, true] {
                v.push(bits_of(sign, 0, m));
            }
        }
    }
    check_row("CONFIGS row 5 (subnormal %a fraction trimming)", &v);
}

// --- row 6 ------------------------------------------------------------------

#[test]
fn cfg_06_min_normal_expfield_one() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut v = Vec::new();
    for sign in [false, true] {
        v.push(bits_of(sign, 1, 0));
        v.push(bits_of(sign, 1, 1));
        v.push(bits_of(sign, 1, MANT_MASK));
        for _ in 0..1024 {
            v.push(bits_of(sign, 1, rng.next_u64() & MANT_MASK));
        }
    }
    check_row("CONFIGS row 6 (expfield=1, min normal)", &v);
}

// --- row 7 ------------------------------------------------------------------

#[test]
fn cfg_07_broad_normal_sweep() {
    let mut rng = Rng::new(SEED ^ 7);
    let v: Vec<u64> = (0..200_000)
        .map(|_| {
            let e = rng.range(1, 0x7fe);
            let m = rng.next_u64() & MANT_MASK;
            let s = rng.next_u64() & 1 == 1;
            bits_of(s, e, m)
        })
        .collect();
    check_row("CONFIGS row 7 (broad normal sweep, 200k)", &v);
}

// --- rows 8, 9 --------------------------------------------------------------

#[test]
fn cfg_08_powers_of_two_exhaustive_exponent() {
    let mut v = Vec::new();
    for e in 1..=0x7feu64 {
        for sign in [false, true] {
            v.push(bits_of(sign, e, 0));
        }
    }
    assert_eq!(v.len(), 4092);
    check_row("CONFIGS row 8 (mantissa=0, every normal exponent)", &v);
}

#[test]
fn cfg_09_all_ones_mantissa_exhaustive_exponent() {
    let mut v = Vec::new();
    for e in 1..=0x7feu64 {
        for sign in [false, true] {
            v.push(bits_of(sign, e, MANT_MASK));
        }
    }
    assert_eq!(v.len(), 4092);
    check_row("CONFIGS row 9 (mantissa=all ones, every normal exponent)", &v);
}

// --- row 10 -----------------------------------------------------------------

#[test]
fn cfg_10_low_nibble_set_no_trimming() {
    let mut rng = Rng::new(SEED ^ 10);
    let mut v = Vec::new();
    for _ in 0..4096 {
        let m = (rng.next_u64() & MANT_MASK & !0xF) | rng.range(1, 15);
        let e = rng.range(1, 0x7fe);
        let s = rng.next_u64() & 1 == 1;
        v.push(bits_of(s, e, m));
    }
    // The extreme of "interior zeroes must be preserved": 0x1.0000000000001p+0
    v.push(bits_of(false, 0x3ff, 1));
    v.push(bits_of(true, 0x3ff, 1));
    check_row("CONFIGS row 10 (low nibble set, no trimming)", &v);
}

// --- row 11 -----------------------------------------------------------------

#[test]
fn cfg_11_exact_trailing_zero_nibble_counts() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut v = Vec::new();
    for k in 1..=12u32 {
        let shift = 4 * k;
        for _ in 0..128 {
            // low nibble of `high` nonzero => exactly k trailing zero nibbles
            let high = (rng.next_u64() & (MANT_MASK >> shift) & !0xF) | rng.range(1, 15);
            let m = (high << shift) & MANT_MASK;
            let e = rng.range(1, 0x7fe);
            for sign in [false, true] {
                v.push(bits_of(sign, e, m));
            }
        }
    }
    check_row("CONFIGS row 11 (exactly k trailing zero nibbles, k=1..12)", &v);
}

// --- rows 12, 13 ------------------------------------------------------------

#[test]
fn cfg_12_exponent_p_plus_zero() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut v = Vec::new();
    for sign in [false, true] {
        v.push(bits_of(sign, 0x3ff, 0));
        v.push(bits_of(sign, 0x3ff, MANT_MASK));
        for _ in 0..2048 {
            v.push(bits_of(sign, 0x3ff, rng.next_u64() & MANT_MASK));
        }
    }
    check_row("CONFIGS row 12 (expfield=0x3ff, p+0, values in [1,2))", &v);
}

#[test]
fn cfg_13_exponent_p_minus_one() {
    let mut rng = Rng::new(SEED ^ 13);
    let mut v = Vec::new();
    for sign in [false, true] {
        v.push(bits_of(sign, 0x3fe, 0));
        v.push(bits_of(sign, 0x3fe, MANT_MASK));
        for _ in 0..2048 {
            v.push(bits_of(sign, 0x3fe, rng.next_u64() & MANT_MASK));
        }
    }
    check_row("CONFIGS row 13 (expfield=0x3fe, p-1, values in [0.5,1))", &v);
}

// --- row 14 -----------------------------------------------------------------

#[test]
fn cfg_14_max_normal_exponent() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut v = Vec::new();
    for sign in [false, true] {
        v.push(bits_of(sign, 0x7fe, 0));
        v.push(bits_of(sign, 0x7fe, MANT_MASK));
        v.push(bits_of(sign, 0x7fe, 1));
        for _ in 0..1024 {
            v.push(bits_of(sign, 0x7fe, rng.next_u64() & MANT_MASK));
        }
    }
    check_row("CONFIGS row 14 (expfield=0x7fe, p+1023, max normal)", &v);
}

// --- rows 15, 16 ------------------------------------------------------------

#[test]
fn cfg_15_infinities() {
    check_row(
        "CONFIGS row 15 (+/-inf)",
        &[bits_of(false, 0x7ff, 0), bits_of(true, 0x7ff, 0)],
    );
}

#[test]
fn cfg_16_nan_payloads() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut v = Vec::new();
    for sign in [false, true] {
        v.push(bits_of(sign, 0x7ff, 0x8_0000_0000_0000)); // canonical quiet
        v.push(bits_of(sign, 0x7ff, 0x4_0000_0000_0000)); // signalling
        v.push(bits_of(sign, 0x7ff, 1)); // smallest payload
        v.push(bits_of(sign, 0x7ff, MANT_MASK)); // largest payload
        v.push(bits_of(sign, 0x7ff, 0xF_FFFF_FFFF_FFFE));
        for _ in 0..1024 {
            v.push(bits_of(sign, 0x7ff, nonzero_mantissa(&mut rng)));
        }
    }
    check_row("CONFIGS row 16 (NaN, all payload classes, both signs)", &v);
}

// --- rows 17..23: %.4f magnitude classes ------------------------------------

fn magnitude_row(name: &str, seed: u64, lo: f64, hi: f64, n: usize, extras: &[f64]) {
    let mut rng = Rng::new(SEED ^ seed);
    let mut v = random_in_range(&mut rng, lo, hi, n);
    for &x in extras {
        v.push(x.to_bits());
        v.push((-x).to_bits());
    }
    check_row(name, &v);
}

#[test]
fn cfg_17_below_rounding_boundary() {
    magnitude_row(
        "CONFIGS row 17 (|f| in (0, 5e-5) -> 0.0000)",
        17,
        0.0,
        5e-5,
        4096,
        &[f64::from_bits(1), 4.9999999999999996e-5, 5e-5, 1e-300, 2.2250738585072014e-308],
    );
}

#[test]
fn cfg_18_sub_one() {
    magnitude_row("CONFIGS row 18 (|f| in [5e-5, 1))", 18, 5e-5, 1.0, 8192, &[0.5, 0.25, 0.99999, 0.0001]);
}

#[test]
fn cfg_19_one_to_1e4() {
    magnitude_row("CONFIGS row 19 (|f| in [1, 1e4))", 19, 1.0, 1e4, 8192, &[1.0, 3.14159265358979, 9999.99995]);
}

#[test]
fn cfg_20_1e4_to_1e15() {
    magnitude_row("CONFIGS row 20 (|f| in [1e4, 1e15))", 20, 1e4, 1e15, 8192, &[1e14, 123456789.12345]);
}

#[test]
fn cfg_21_integral_crossover() {
    magnitude_row(
        "CONFIGS row 21 (|f| in [1e15, 1e17), fraction always .0000)",
        21,
        1e15,
        1e17,
        8192,
        &[1e15, 9007199254740992.0, 1e16, 1e17],
    );
}

#[test]
fn cfg_22_1e100_to_1e200() {
    magnitude_row("CONFIGS row 22 (|f| in [1e100, 1e200))", 22, 1e100, 1e200, 4096, &[1e100, 1e200]);
}

#[test]
fn cfg_23_near_dbl_max() {
    magnitude_row(
        "CONFIGS row 23 (|f| in [1e300, DBL_MAX], 309 integer digits)",
        23,
        1e300,
        f64::MAX,
        4096,
        &[1e300, f64::MAX, f64::from_bits(0x7fef_ffff_ffff_fffe)],
    );
}

// --- row 24 -----------------------------------------------------------------

#[test]
fn cfg_24_exact_ties_half_to_even() {
    // A binary fraction k/32 with odd k has an exact decimal expansion whose
    // fifth (and last) fraction digit is 5 -- the true half-way case for %.4f.
    // The parity of the fourth digit decides the direction under
    // round-half-to-even, so both parities must appear.
    let mut v = Vec::new();
    let mut odd_even_seen = (false, false);
    let mut k: u64 = 1;
    while k <= 20001 {
        let x = k as f64 / 32.0;
        // fourth fraction digit
        let frac = (x.fract() * 100_000.0).round() as u64 % 100_000;
        let d4 = (frac / 10) % 10;
        if d4 % 2 == 0 {
            odd_even_seen.0 = true;
        } else {
            odd_even_seen.1 = true;
        }
        v.push(x.to_bits());
        v.push((-x).to_bits());
        k += 2;
    }
    assert!(odd_even_seen.0 && odd_even_seen.1, "tie family must cover both parities");
    // The two canonical illustrations from ERRORS.md rows 21/22.
    for x in [0.03125f64, 0.09375] {
        v.push(x.to_bits());
        v.push((-x).to_bits());
    }
    check_row("CONFIGS row 24 (exactly-representable %.4f ties, both parities)", &v);
}

// --- row 25 -----------------------------------------------------------------

#[test]
fn cfg_25_near_ties_ulp_neighbours() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut v = Vec::new();
    for _ in 0..2000 {
        let k = rng.below(2_000_000) as f64;
        let target = k / 10_000.0 + 0.000_05;
        let b = target.to_bits();
        for step in [-2i64, -1, 0, 1, 2] {
            let nb = ulp_step(b, step);
            v.push(nb);
            v.push(nb | SIGN);
        }
    }
    check_row("CONFIGS row 25 (+/-ulp neighbours of %.4f tie boundaries)", &v);
}

// --- row 26 -----------------------------------------------------------------

#[test]
fn cfg_26_four_digit_fractions() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut v = Vec::new();
    for _ in 0..4000 {
        let k = rng.below(100_000_000) as f64;
        let x = k / 10_000.0;
        let b = x.to_bits();
        for step in [-1i64, 0, 1] {
            v.push(ulp_step(b, step));
            v.push(ulp_step(b, step) | SIGN);
        }
    }
    check_row("CONFIGS row 26 (nearest doubles to k/10000)", &v);
}

// --- row 27 -----------------------------------------------------------------

#[test]
fn cfg_27_integral_doubles() {
    let mut rng = Rng::new(SEED ^ 27);
    let mut v = Vec::new();
    for _ in 0..8192 {
        let i = rng.below(1u64 << 53) as i64;
        let x = if rng.next_u64() & 1 == 1 { -(i as f64) } else { i as f64 };
        v.push(x.to_bits());
    }
    for x in [0.0f64, 1.0, 2.0, 9007199254740992.0, -9007199254740992.0] {
        v.push(x.to_bits());
    }
    check_row("CONFIGS row 27 (integral doubles, fraction .0000)", &v);
}

// --- row 28 -----------------------------------------------------------------

#[test]
fn cfg_28_llx_width_axis() {
    let mut rng = Rng::new(SEED ^ 28);
    let mut v = vec![0u64];
    for w in 1..=16u32 {
        let hi_bit = 4 * w;
        let lo = if w == 1 { 0u64 } else { 1u64 << (hi_bit - 4) };
        let hi = if w == 16 { u64::MAX } else { (1u64 << hi_bit) - 1 };
        v.push(lo);
        v.push(hi);
        if w > 1 {
            v.push(lo - 1); // one below the width boundary
        }
        for _ in 0..256 {
            v.push(rng.range(lo, hi));
        }
    }
    check_row("CONFIGS row 28 (%llx result width 1..16 hex digits)", &v);
}

// --- row 29 -----------------------------------------------------------------

#[test]
fn cfg_29_sign_bit_paired() {
    let mut rng = Rng::new(SEED ^ 29);
    let mut v = Vec::new();
    for _ in 0..4096 {
        let low = rng.next_u64() & !SIGN;
        v.push(low);
        v.push(low | SIGN);
    }
    check_row("CONFIGS row 29 (identical low 63 bits, sign bit clear vs set)", &v);
}

// --- row 30 -----------------------------------------------------------------

#[test]
fn cfg_30_whole_domain_fuzz() {
    let mut rng = Rng::new(SEED ^ 30);
    let v: Vec<u64> = (0..500_000).map(|_| rng.next_u64()).collect();
    check_row("CONFIGS row 30 (uniform random u64 as double, 500k)", &v);
}

// --- row 31 -----------------------------------------------------------------

#[test]
fn cfg_31_exhaustive_exponent_sweep() {
    let mut rng = Rng::new(SEED ^ 31);
    let mut v = Vec::new();
    for e in 0..=0x7ffu64 {
        for sign in [false, true] {
            for m in [0u64, 1, MANT_MASK, rng.next_u64() & MANT_MASK] {
                v.push(bits_of(sign, e, m));
            }
        }
    }
    assert_eq!(v.len(), 2048 * 8);
    check_row("CONFIGS row 31 (every exponent field 0..0x7ff x sign x mantissa)", &v);
}

// --- row 32 -----------------------------------------------------------------

#[test]
fn cfg_32_stream_state_single_buffered_run() {
    // One capture, one buffered `FILE`, a long mixed sequence: verifies the
    // composed output stream rather than isolated calls.
    let mut rng = Rng::new(SEED ^ 32);
    let mut seq = Vec::new();
    for i in 0..2048u64 {
        seq.push(match i % 8 {
            0 => bits_of(false, 0, 0),
            1 => bits_of(true, 0x7ff, 0x8_0000_0000_0000),
            2 => bits_of(false, 0x7ff, 0),
            3 => bits_of(false, 0, nonzero_mantissa(&mut rng)),
            4 => f64::MAX.to_bits(),
            5 => 0.03125f64.to_bits(),
            6 => rng.next_u64(),
            _ => bits_of(rng.next_u64() & 1 == 1, rng.range(1, 0x7fe), rng.next_u64() & MANT_MASK),
        });
    }
    let divs = compare_batch(&seq);
    assert!(
        divs.is_empty(),
        "CONFIGS row 32 (single buffered run of {} calls): {} divergences, first:\n  {}",
        seq.len(),
        divs.len(),
        divs[0]
    );
    eprintln!("CONFIGS row 32: OK ({} calls in one buffered run)", seq.len());
}
