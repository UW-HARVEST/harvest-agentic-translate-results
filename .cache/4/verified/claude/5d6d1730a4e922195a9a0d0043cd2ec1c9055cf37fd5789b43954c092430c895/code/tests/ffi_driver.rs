// Phase B — differential tests for the lowest-level entry point, `driver`.
//
// CONFIGS.md rows 1-15 (and ERRORS.md row 23).  Both shared objects are loaded
// with `libloading` and only their exported `driver` symbol is called, so the
// `#[no_mangle]` wrapper is part of what is being tested.

mod common;

use common::{diff_driver, diff_driver_bits, Rng, SEED};

const EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const MANT_MASK: u64 = 0x000f_ffff_ffff_ffff;
const SIGN: u64 = 0x8000_0000_0000_0000;

fn bits(sign: bool, exp: u64, mant: u64) -> u64 {
    (if sign { SIGN } else { 0 }) | ((exp & 0x7ff) << 52) | (mant & MANT_MASK)
}

/// CONFIGS row 1 — ±0.0
fn row_01_signed_zeros() {
    diff_driver_bits("row01 zeros", &[0x0000_0000_0000_0000, SIGN]);
}

/// CONFIGS row 2 — ±inf
fn row_02_infinities() {
    diff_driver_bits("row02 inf", &[EXP_MASK, SIGN | EXP_MASK]);
}

/// CONFIGS row 3 — the default quiet NaN, both signs
fn row_03_default_nan() {
    diff_driver_bits(
        "row03 nan",
        &[0x7ff8_0000_0000_0000, 0xfff8_0000_0000_0000],
    );
}

/// CONFIGS row 4 / ERRORS row 23 — every NaN payload, quiet and signalling
fn row_04_nan_payloads() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut v = vec![
        0x7ff0_0000_0000_0001, // smallest signalling NaN
        0xfff0_0000_0000_0001,
        0x7ff7_ffff_ffff_ffff, // largest signalling NaN
        0xfff7_ffff_ffff_ffff,
        0x7ff8_0000_0000_0001,
        0x7fff_ffff_ffff_ffff, // largest quiet NaN
        0xffff_ffff_ffff_ffff,
        0x7ffa_aaaa_aaaa_aaaa,
        0xfff5_5555_5555_5555,
    ];
    for _ in 0..2000 {
        let mant = (rng.next_u64() & MANT_MASK).max(1);
        v.push(bits(rng.flip(), 0x7ff, mant));
    }
    diff_driver_bits("row04 nan payloads", &v);
}

/// CONFIGS row 5 — subnormals
fn row_05_subnormals() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut v = vec![
        0x0000_0000_0000_0001,
        SIGN | 0x0000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        SIGN | 0x000f_ffff_ffff_ffff,
        0x0008_0000_0000_0000,
        0x0000_0000_0000_0010,
        0x0000_1000_0000_0000,
    ];
    for _ in 0..2000 {
        let mant = (rng.next_u64() & MANT_MASK).max(1);
        v.push(bits(rng.flip(), 0, mant));
    }
    // also a sweep over "one bit set" subnormals
    for i in 0..52 {
        v.push(bits(false, 0, 1u64 << i));
        v.push(bits(true, 0, 1u64 << i));
    }
    diff_driver_bits("row05 subnormals", &v);
}

/// CONFIGS row 6 — arbitrary bit patterns across the whole 64-bit space
fn row_06_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut v = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        v.push(rng.next_u64());
    }
    diff_driver_bits("row06 random bits", &v);
}

/// CONFIGS row 7 — mantissa == 0 (exact powers of two): `%a` must emit no `.`
fn row_07_powers_of_two() {
    let mut v = Vec::new();
    for exp in 1..=2046u64 {
        v.push(bits(false, exp, 0));
        v.push(bits(true, exp, 0));
    }
    diff_driver_bits("row07 powers of two", &v);
}

/// CONFIGS row 8 — trailing-zero-nibble trimming in `%a`, every length
fn row_08_trailing_zero_nibbles() {
    let mut rng = Rng::new(SEED ^ 8);
    let mut v = Vec::new();
    for k in 1..=12u32 {
        for _ in 0..40 {
            let keep = 13 - k;
            let mut mant = rng.next_u64() & ((1u64 << (4 * keep)) - 1);
            mant <<= 4 * k;
            let exp = rng.below(2046) + 1;
            v.push(bits(rng.flip(), exp, mant));
            // and the same nibbles as a subnormal
            v.push(bits(rng.flip(), 0, mant));
        }
    }
    // a nibble sweep: exactly one nibble set, at each of the 13 positions
    for pos in 0..13u32 {
        for d in 1..16u64 {
            v.push(bits(false, 1023, d << (4 * pos)));
        }
    }
    diff_driver_bits("row08 trailing zero nibbles", &v);
}

/// CONFIGS row 9 — all 13 mantissa nibbles significant
fn row_09_full_mantissa() {
    let mut v = Vec::new();
    for exp in [1u64, 2, 1000, 1022, 1023, 1024, 2045, 2046] {
        v.push(bits(false, exp, MANT_MASK));
        v.push(bits(true, exp, MANT_MASK));
        v.push(bits(false, exp, 0x1234_5678_9abc_d & MANT_MASK));
        v.push(bits(true, exp, 0xfedc_ba98_7654_3 & MANT_MASK));
    }
    v.push(bits(false, 0, MANT_MASK));
    v.push(bits(true, 0, MANT_MASK));
    diff_driver_bits("row09 full mantissa", &v);
}

/// CONFIGS row 10 — exponent-field boundaries
fn row_10_exponent_boundaries() {
    let mut v = Vec::new();
    for exp in [0u64, 1, 2, 1021, 1022, 1023, 1024, 1025, 2044, 2045, 2046, 2047] {
        for mant in [0u64, 1, 0x8_0000_0000_000, MANT_MASK, 0x5_5555_5555_555] {
            v.push(bits(false, exp, mant));
            v.push(bits(true, exp, mant));
        }
    }
    diff_driver_bits("row10 exponent boundaries", &v);
}

/// CONFIGS row 11 — `%.4f` exact round-half-to-even ties.
///
/// Every odd multiple of 1/32 has exactly five decimal digits, the last of which
/// is a `5`, i.e. it is an exact tie for `%.4f`.
fn row_11_rounding_ties() {
    let mut v = Vec::new();
    for int_part in 0..40i64 {
        for k in (1..32i64).step_by(2) {
            let x = int_part as f64 + k as f64 / 32.0;
            v.push(x);
            v.push(-x);
        }
    }
    // ties one digit further out (1/64 steps) and nearer (1/16 steps)
    for k in (1..64i64).step_by(2) {
        v.push(k as f64 / 64.0);
        v.push(-(k as f64) / 64.0);
    }
    for k in (1..16i64).step_by(2) {
        v.push(k as f64 / 16.0);
        v.push(-(k as f64) / 16.0);
    }
    // large integer parts, where the tie interacts with a long digit string
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..500 {
        let int_part = (rng.next_u64() % (1u64 << 40)) as f64;
        let k = (rng.below(16) * 2 + 1) as f64;
        let x = int_part + k / 32.0;
        v.push(x);
        v.push(-x);
    }
    // The largest doubles that can be exact `%.4f` ties are the odd multiples of
    // 1/32 (an exact tie needs value == j/32 with j odd), i.e. up to 2^48.  Cover
    // the top end of that range explicitly, where the printed digit string is
    // longest and the tie is hardest to get right.
    for shift in [40u32, 44, 45, 46, 47, 48] {
        let base = (1u64 << shift) as f64;
        for k in (1..32i64).step_by(2) {
            for delta in 0..4u64 {
                let x = base + delta as f64 + k as f64 / 32.0;
                v.push(x);
                v.push(-x);
            }
        }
    }
    for _ in 0..500 {
        let int_part = (rng.next_u64() % (1u64 << 47)) as f64;
        let k = (rng.below(16) * 2 + 1) as f64;
        let x = int_part + k / 32.0;
        v.push(x);
        v.push(-x);
    }
    diff_driver("row11 rounding ties", &v);
}

/// CONFIGS row 12 — either side of the `%.4f` rounding threshold
fn row_12_near_threshold() {
    let mut v = Vec::new();
    for k in 0..24u64 {
        let center = (k as f64) * 0.0001 + 0.00005;
        let b = center.to_bits();
        for d in 0..4u64 {
            v.push(f64::from_bits(b.wrapping_sub(d)));
            v.push(f64::from_bits(b.wrapping_add(d)));
            v.push(-f64::from_bits(b.wrapping_sub(d)));
            v.push(-f64::from_bits(b.wrapping_add(d)));
        }
    }
    // exact 0.00005-ish boundaries expressed as dyadic ties
    for x in [
        1.0 / 32768.0,
        3.0 / 32768.0,
        1.0 / 16384.0,
        1.0 / 65536.0,
        4.9999e-5,
        5.0001e-5,
        9.99999e-5,
    ] {
        v.push(x);
        v.push(-x);
    }
    // Rounding that has to CARRY out of the fraction into the integer part, and
    // carry through a run of nines — `1 - 2^-n` is `0.9999…` in decimal.
    for n in 5..=40u32 {
        let x = 1.0 - 2f64.powi(-(n as i32));
        v.push(x);
        v.push(-x);
        for base in [0.0f64, 9.0, 99.0, 999.0, 1e6, 1e15, 1e16] {
            v.push(base + x);
            v.push(-(base + x));
        }
    }
    for x in [
        0.99995f64, 0.99999, 9.99995, 9.99999, 99.99995, 0.9999999999,
        999999.99999, 1e15 - 1e-5, 0.00004999999, 0.00005000001,
    ] {
        v.push(x);
        v.push(-x);
        // and the two neighbouring representable values
        v.push(f64::from_bits(x.to_bits() + 1));
        v.push(f64::from_bits(x.to_bits() - 1));
    }
    diff_driver("row12 near threshold", &v);
}

/// CONFIGS row 13 — huge magnitudes, 300+ digit `%.4f` expansions
fn row_13_huge_magnitudes() {
    let mut rng = Rng::new(SEED ^ 13);
    let mut v = vec![
        f64::MAX,
        -f64::MAX,
        f64::from_bits(0x7fef_ffff_ffff_fffe),
        1e300,
        -1e300,
        1e308,
        1.7976931348623157e308,
        9.9e307,
    ];
    for _ in 0..1500 {
        let exp = rng.below(2046 - 1970) + 1970; // ~1e295 .. DBL_MAX
        v.push(f64::from_bits(bits(rng.flip(), exp, rng.next_u64() & MANT_MASK)));
    }
    // also the whole "integral" range where %.4f prints many digits
    for e in (60..=1023u64).step_by(7) {
        v.push(f64::from_bits(bits(false, e + 1023 - 1023, MANT_MASK)));
    }
    diff_driver("row13 huge magnitudes", &v);
}

/// CONFIGS row 14 — magnitudes that `%.4f` prints as ±0.0000
fn row_14_tiny_magnitudes() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut v = vec![
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-320,
        1e-100,
        -1e-100,
        4.9e-5,
    ];
    for _ in 0..1500 {
        let exp = rng.below(60);
        v.push(f64::from_bits(bits(rng.flip(), exp, rng.next_u64() & MANT_MASK)));
    }
    diff_driver("row14 tiny magnitudes", &v);
}

/// CONFIGS row 15 — exactly representable integers and powers of ten
fn row_15_integers_and_powers_of_ten() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut v = Vec::new();
    for i in 0..64u32 {
        let x = (1u64 << i) as f64;
        v.push(x);
        v.push(-x);
    }
    for e in 0..=22i32 {
        let x = 10f64.powi(e);
        v.push(x);
        v.push(-x);
    }
    for e in -22..=0i32 {
        let x = 10f64.powi(e);
        v.push(x);
        v.push(-x);
    }
    for _ in 0..2000 {
        let x = (rng.next_u64() % (1u64 << 53)) as f64;
        v.push(x);
        v.push(-x);
    }
    diff_driver("row15 integers", &v);
}

/// ERRORS row 23 — `driver` never rejects: sweep every exponent/sign pair with
/// several mantissas, i.e. all 2^12 "enum-like" values of the exponent field
/// including the ones with no valid numeric meaning.
fn row_23_driver_accepts_every_bit_pattern() {
    let mut v = Vec::with_capacity(2048 * 6);
    for exp in 0..=2047u64 {
        for mant in [0u64, 1, MANT_MASK] {
            v.push(bits(false, exp, mant));
            v.push(bits(true, exp, mant));
        }
    }
    diff_driver_bits("row23 every exponent field", &v);
}


fn main() {
    common::run_suite(
        "ffi_driver",
        &[
            ("row_01_signed_zeros", row_01_signed_zeros),
            ("row_02_infinities", row_02_infinities),
            ("row_03_default_nan", row_03_default_nan),
            ("row_04_nan_payloads", row_04_nan_payloads),
            ("row_05_subnormals", row_05_subnormals),
            ("row_06_random_bit_patterns", row_06_random_bit_patterns),
            ("row_07_powers_of_two", row_07_powers_of_two),
            ("row_08_trailing_zero_nibbles", row_08_trailing_zero_nibbles),
            ("row_09_full_mantissa", row_09_full_mantissa),
            ("row_10_exponent_boundaries", row_10_exponent_boundaries),
            ("row_11_rounding_ties", row_11_rounding_ties),
            ("row_12_near_threshold", row_12_near_threshold),
            ("row_13_huge_magnitudes", row_13_huge_magnitudes),
            ("row_14_tiny_magnitudes", row_14_tiny_magnitudes),
            ("row_15_integers_and_powers_of_ten", row_15_integers_and_powers_of_ten),
            ("row_23_driver_accepts_every_bit_pattern", row_23_driver_accepts_every_bit_pattern),
        ],
    );
}
