//! Phase B — valid-path differential tests for the LOW-LEVEL entry points.
//! Covers CONFIGS.md rows 1..=32. Both implementations are called only through
//! their exported symbols, loaded from the two `.so` files via `libloading`.

mod common;
use common::*;

// ===========================================================================
// classify_mode — CONFIGS rows 1..=7
// ===========================================================================

fn diff_classify(tag: &str, bytes: &[u8]) {
    let p = pair();
    let (c, rs) = (p.c.classify_mode(), p.rs.classify_mode());
    let s = cstr(bytes);
    // SAFETY: `s` is a NUL-terminated C string.
    let (a, b) = unsafe { (c(s.as_ptr()), rs(s.as_ptr())) };
    assert_eq!(
        a, b,
        "classify_mode divergence [{tag}] input={:?}: C=0x{a:X} Rust=0x{b:X}",
        show(bytes)
    );
}

const LITERALS: [&[u8]; 4] = [b"standard", b"enhanced", b"turbo", b"extreme"];

#[test]
fn row_01_04_classify_mode_exact_literals() {
    for (i, lit) in LITERALS.iter().enumerate() {
        diff_classify(&format!("exact literal #{i}"), lit);
    }
}

#[test]
fn row_05_classify_mode_random_nonmatching() {
    let mut rng = Rng::fixed();
    for _ in 0..4000 {
        let len = rng.below(33) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            // 1..=255, never 0 (interior NUL would truncate the C string)
            s.push((rng.below(255) + 1) as u8);
        }
        diff_classify("random non-matching", &s);
    }
    // Printable-only variants, more likely to collide with a literal's shape.
    for _ in 0..4000 {
        let len = rng.below(12) as usize;
        let s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
        diff_classify("random lowercase", &s);
    }
}

#[test]
fn row_06_classify_mode_single_byte_mutations() {
    let mut rng = Rng::fixed();
    for lit in LITERALS {
        for pos in 0..lit.len() {
            for _ in 0..40 {
                let mut s = lit.to_vec();
                s[pos] = (rng.below(255) + 1) as u8;
                diff_classify("single-byte mutation", &s);
            }
            // Deterministic case flip, since strcmp is case-sensitive.
            let mut s = lit.to_vec();
            s[pos] = s[pos].to_ascii_uppercase();
            diff_classify("case flip", &s);
        }
    }
}

#[test]
fn row_07_classify_mode_prefixes_and_extensions() {
    let mut rng = Rng::fixed();
    for lit in LITERALS {
        // every strict prefix, including the empty string (row 2/3)
        for n in 0..lit.len() {
            diff_classify("prefix", &lit[..n]);
        }
        // extensions by 1..=4 random bytes (row 4)
        for extra in 1..=4usize {
            for _ in 0..40 {
                let mut s = lit.to_vec();
                for _ in 0..extra {
                    s.push((rng.below(255) + 1) as u8);
                }
                diff_classify("extension", &s);
            }
            let mut s = lit.to_vec();
            s.extend(std::iter::repeat_n(b' ', extra));
            diff_classify("trailing spaces", &s);
        }
    }
    diff_classify("empty", b"");
}

// ===========================================================================
// apply_multiplier — CONFIGS rows 8..=14
// ===========================================================================

fn diff_multiplier(tag: &str, base: i32, level: i32) {
    let p = pair();
    let (c, rs) = (p.c.apply_multiplier(), p.rs.apply_multiplier());
    // SAFETY: plain scalar FFI call.
    let (a, b) = unsafe { (c(base, level), rs(base, level)) };
    assert_eq!(
        a, b,
        "apply_multiplier divergence [{tag}] base={base} level={level}: C={a} Rust={b}"
    );
}

#[test]
fn row_08_12_apply_multiplier_each_fallthrough_level() {
    let mut rng = Rng::fixed();
    for level in 0..=4 {
        // hand-picked plus randomized bases across the whole int range
        for base in [0, 1, -1, 0xA0, i32::MAX, i32::MIN] {
            diff_multiplier("fixed base", base, level);
        }
        for _ in 0..3000 {
            diff_multiplier("random base", rng.next_i32(), level);
        }
    }
}

#[test]
fn row_13_apply_multiplier_overflow_wrap() {
    // The largest accumulation is 0xFF+0xAB+0x7E+0x1C+0x05 = 0x24D, so bases
    // within 0x24D of INT_MAX overflow signed int on at least one level.
    for level in 0..=4 {
        for k in 0..=0x300i64 {
            let base = (i32::MAX as i64 - k) as i32;
            diff_multiplier("near INT_MAX", base, level);
        }
    }
}

#[test]
fn row_14_apply_multiplier_boundary_bases() {
    for level in 0..=4 {
        for k in 0..=0x300i64 {
            let base = (i32::MIN as i64 + k) as i32;
            diff_multiplier("near INT_MIN", base, level);
        }
    }
}

// ===========================================================================
// convert_time_factor — CONFIGS rows 15..=20
// convert_negative_overflow — CONFIGS rows 21..=25
// ===========================================================================

/// `which`: false = convert_time_factor, true = convert_negative_overflow.
fn diff_convert(tag: &str, which: bool, x: f64) {
    let p = pair();
    let (c, rs) = if which {
        (p.c.convert_negative_overflow(), p.rs.convert_negative_overflow())
    } else {
        (p.c.convert_time_factor(), p.rs.convert_time_factor())
    };
    let name = if which { "convert_negative_overflow" } else { "convert_time_factor" };
    // SAFETY: plain scalar FFI call.
    let (a, b) = unsafe { (c(x), rs(x)) };
    assert_eq!(
        a, b,
        "{name} divergence [{tag}] x={x:?} (bits=0x{:016X}): C={a} Rust={b}",
        x.to_bits()
    );
}

/// For `convert_time_factor` the product is `x*1e12`; for
/// `convert_negative_overflow` it is `x*-1e15`. `scale` is the multiplier so we
/// can aim the *product* at a target value.
fn scale_of(which: bool) -> f64 {
    if which { -1e15 } else { 1e12 }
}

#[test]
fn row_15_16_21_22_convert_in_range_and_tiny() {
    let mut rng = Rng::fixed();
    for which in [false, true] {
        let s = scale_of(which);
        // products uniformly inside [INT_MIN, INT_MAX]
        for _ in 0..4000 {
            let target = (rng.unit() * 2.0 - 1.0) * 2_147_483_647.0;
            diff_convert("product in range", which, target / s);
        }
        // tiny magnitudes: product truncates toward zero
        for _ in 0..2000 {
            diff_convert("tiny", which, rng.log_f64(-300.0, -13.0));
        }
        for x in [0.0f64, -0.0, f64::MIN_POSITIVE, -f64::MIN_POSITIVE, 5e-324, -5e-324] {
            diff_convert("zero/subnormal", which, x);
        }
    }
}

#[test]
fn row_17_23_convert_boundary_sweep() {
    for which in [false, true] {
        let s = scale_of(which);
        // Sweep the product across both int boundaries, one ULP-ish at a time.
        for edge in [2_147_483_647.0f64, -2_147_483_648.0] {
            for d in -600..=600i64 {
                let target = edge + d as f64 * 0.5;
                diff_convert("boundary sweep", which, target / s);
            }
        }
        // Exact boundary products, and one step past in f64 terms.
        for target in [
            2_147_483_646.0f64,
            2_147_483_647.0,
            2_147_483_647.5,
            2_147_483_648.0,
            2_147_483_649.0,
            -2_147_483_647.0,
            -2_147_483_648.0,
            -2_147_483_648.5,
            -2_147_483_649.0,
        ] {
            let x = target / s;
            diff_convert("exact boundary", which, x);
            diff_convert("boundary next_up", which, f64::from_bits(x.to_bits() + 1));
            diff_convert("boundary next_down", which, f64::from_bits(x.to_bits() - 1));
        }
    }
}

#[test]
fn row_19_24_convert_out_of_range() {
    let mut rng = Rng::fixed();
    for which in [false, true] {
        for _ in 0..4000 {
            // magnitudes far beyond what keeps the product in int range
            diff_convert("out of range", which, rng.log_f64(-2.0, 300.0));
        }
        for x in [
            f64::MAX,
            f64::MIN,
            1.0,
            -1.0,
            1e300,
            -1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            diff_convert("extreme", which, x);
        }
    }
}

#[test]
fn row_18_20_25_convert_random_bit_patterns() {
    // Random bit patterns give free coverage of NaNs (quiet & signalling),
    // infinities and subnormals in the exact proportions the encoding implies.
    let mut rng = Rng::fixed();
    for which in [false, true] {
        for _ in 0..20000 {
            diff_convert("random bits", which, rng.any_f64());
        }
        for x in [
            f64::NAN,
            -f64::NAN,
            f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
            f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
            f64::from_bits(0x7FF8_0000_0000_0000),
        ] {
            diff_convert("nan variants", which, x);
        }
    }
}

// ===========================================================================
// get_modified_time — CONFIGS rows 26..=30
// ===========================================================================

fn diff_modified_time(tag: &str, days: i32, hours: i32) {
    let p = pair();
    let (c, rs) = (p.c.get_modified_time(), p.rs.get_modified_time());
    // Call both back-to-back so `time(NULL) >> 29` (which advances only once
    // every ~17 years) is the same for both; retry once if it ever changes.
    // SAFETY: plain scalar FFI calls.
    let (a, b) = unsafe {
        let a = c(days, hours);
        let b = rs(days, hours);
        if a != b {
            // Re-sample in case a >>29 tick landed between the two calls.
            (c(days, hours), rs(days, hours))
        } else {
            (a, b)
        }
    };
    assert_eq!(
        a, b,
        "get_modified_time divergence [{tag}] days={days} hours={hours}: C={a} Rust={b}"
    );
}

#[test]
fn row_26_get_modified_time_small_offsets() {
    let mut rng = Rng::fixed();
    for _ in 0..4000 {
        diff_modified_time(
            "small",
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
        );
    }
    for d in [-1000, -1, 0, 1, 1000] {
        for h in [-1000, -24, -1, 0, 1, 23, 24, 1000] {
            diff_modified_time("small grid", d, h);
        }
    }
}

#[test]
fn row_27_get_modified_time_days_product_overflow() {
    let mut rng = Rng::fixed();
    // |days| > INT_MAX/86400 = 24855 overflows days*86400
    for _ in 0..4000 {
        let d = rng.range_i32(24856, i32::MAX);
        let d = if rng.next_u64() & 1 == 0 { d } else { d.wrapping_neg() };
        diff_modified_time("days overflow", d, rng.range_i32(-23, 23));
    }
    for d in [24855, 24856, -24855, -24856, i32::MAX, i32::MIN, i32::MAX - 1] {
        diff_modified_time("days boundary", d, 0);
    }
}

#[test]
fn row_28_get_modified_time_hours_product_overflow() {
    let mut rng = Rng::fixed();
    // |hours| > INT_MAX/3600 = 596523 overflows hours*3600
    for _ in 0..4000 {
        let h = rng.range_i32(596524, i32::MAX);
        let h = if rng.next_u64() & 1 == 0 { h } else { h.wrapping_neg() };
        diff_modified_time("hours overflow", rng.range_i32(-100, 100), h);
    }
    for h in [596523, 596524, -596523, -596524, i32::MAX, i32::MIN] {
        diff_modified_time("hours boundary", 0, h);
    }
}

#[test]
fn row_29_get_modified_time_sum_overflow() {
    // Each product fits in int, but their sum does not.
    let mut rng = Rng::fixed();
    for _ in 0..3000 {
        // days*86400 in [0, INT_MAX], hours*3600 in [0, INT_MAX], sum > INT_MAX
        let d = rng.range_i32(12000, 24855);
        let h = rng.range_i32(300000, 596523);
        diff_modified_time("sum overflow +", d, h);
        diff_modified_time("sum overflow -", -d, -h);
        diff_modified_time("mixed signs", d, -h);
        diff_modified_time("mixed signs", -d, h);
    }
    diff_modified_time("max products", 24855, 596523);
    diff_modified_time("min products", -24855, -596523);
}

#[test]
fn row_30_get_modified_time_fully_random_and_corners() {
    let mut rng = Rng::fixed();
    for _ in 0..6000 {
        diff_modified_time("fully random", rng.next_i32(), rng.next_i32());
    }
    let corners = [0i32, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for &d in &corners {
        for &h in &corners {
            diff_modified_time("corner grid", d, h);
        }
    }
}

// ===========================================================================
// hash_time_value — CONFIGS rows 31..=32
// ===========================================================================

fn diff_hash(tag: &str, t: i64) {
    let p = pair();
    let (c, rs) = (p.c.hash_time_value(), p.rs.hash_time_value());
    // SAFETY: plain scalar FFI call.
    let (a, b) = unsafe { (c(t), rs(t)) };
    assert_eq!(
        a, b,
        "hash_time_value divergence [{tag}] t={t} (0x{t:016X}): C=0x{a:X} Rust=0x{b:X}"
    );
    // The C masks with 0x7FFFFFFF, so the result can never be negative.
    assert!(a >= 0, "hash_time_value returned negative {a} for t={t}");
}

#[test]
fn row_31_hash_time_value_small_nonnegative() {
    for t in 0..=(1i64 << 12) {
        diff_hash("small dense", t);
    }
    let mut rng = Rng::fixed();
    for _ in 0..4000 {
        diff_hash("small random", (rng.next_u64() % (1 << 20)) as i64);
    }
}

#[test]
fn row_32_hash_time_value_negative_high_bytes_and_random() {
    let mut rng = Rng::fixed();

    // fully random i64 — every byte position gets high-bit-set values
    for _ in 0..20000 {
        diff_hash("fully random", rng.next_i64());
    }
    // negatives (sign-extended high bytes = 0xFF, exercises `<<24` into the
    // sign bit of int)
    for _ in 0..4000 {
        diff_hash("small negative", -((rng.next_u64() % (1 << 24)) as i64));
    }
    // every byte >= 0x80
    for _ in 0..2000 {
        let mut b = [0u8; 8];
        for x in b.iter_mut() {
            *x = 0x80 | (rng.below(0x80) as u8);
        }
        diff_hash("all high bytes", i64::from_ne_bytes(b));
    }
    // one byte hot at a time, at each of the 8 positions
    for pos in 0..8 {
        for v in [0x01u8, 0x7F, 0x80, 0xFF] {
            let mut b = [0u8; 8];
            b[pos] = v;
            diff_hash("single hot byte", i64::from_ne_bytes(b));
        }
    }
    // single-bit values and their negations
    for s in 0..64 {
        let v = 1i64 << s;
        diff_hash("single bit", v);
        diff_hash("single bit neg", v.wrapping_neg());
        diff_hash("single bit not", !v);
    }
    for t in [
        0i64,
        1,
        -1,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        1_757_000_000, // realistic epoch seconds
        3,             // what time(NULL)>>29 currently yields
        -1_757_000_000,
        0x0102_0304_0506_0708,
        0x8080_8080_8080_8080u64 as i64,
        0x7F7F_7F7F_7F7F_7F7F,
    ] {
        diff_hash("corner", t);
    }
}
