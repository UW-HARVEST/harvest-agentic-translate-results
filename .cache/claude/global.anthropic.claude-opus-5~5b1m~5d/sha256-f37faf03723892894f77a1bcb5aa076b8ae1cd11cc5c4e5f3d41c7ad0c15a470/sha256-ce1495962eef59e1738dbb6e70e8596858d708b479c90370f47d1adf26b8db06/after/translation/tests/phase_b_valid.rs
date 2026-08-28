//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares their outputs byte-for-byte.

mod harness;

use harness::*;
use std::ffi::c_int;

// ===========================================================================
// Preconditions: struct layout + symbol availability through the .so exports
// ===========================================================================

fn layout_matches_c_abi() {
    assert_eq!(size_of::<DataBlock>(), 40, "sizeof(DataBlock)");
    assert_eq!(align_of::<DataBlock>(), 8, "_Alignof(DataBlock)");
    assert_eq!(std::mem::offset_of!(DataBlock, id), 0);
    assert_eq!(std::mem::offset_of!(DataBlock, value), 8);
    assert_eq!(std::mem::offset_of!(DataBlock, label), 16);
    assert_eq!(size_of::<RawBlock>(), DATA_BLOCK_SIZE);
}

fn both_libraries_expose_all_five_symbols() {
    // `Api::load` panics if any of the five dynamic symbols is missing, so
    // simply loading both proves the export surface is present in both.
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

// ===========================================================================
// CONFIGS rows 1-7 — safe_double_to_int
// ===========================================================================

#[track_caller]
fn diff_sdti(d: f64) {
    let (c, r) = both();
    let cv = c.safe_double_to_int(d);
    let rv = r.safe_double_to_int(d);
    assert_int_eq(
        &format!("safe_double_to_int({d:?} / bits {:#018x})", d.to_bits()),
        cv,
        rv,
    );
}

/// CONFIGS row 1 — in-range, exact integer values across the whole `int` range.
fn cfg01_sdti_exact_integers_randomized() {
    let mut rng = Rng::new(0x5EED_0001);
    for k in [0i32, 1, -1, 2, -2, 7, -7, i32::MAX, i32::MIN] {
        diff_sdti(k as f64);
    }
    for _ in 0..20_000 {
        diff_sdti(rng.next_i32() as f64);
    }
}

/// CONFIGS row 2 — in-range positive fractions (truncation toward zero).
fn cfg02_sdti_positive_fractions_randomized() {
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..20_000 {
        let base = (rng.next_u32() >> 1) as f64; // [0, 2^31)
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let d = base + frac;
        if d <= i32::MAX as f64 {
            diff_sdti(d);
        }
    }
    for f in [0.5, 0.9999999999, 1.5, 2.5, 1e-9, 42.75, 2147483646.5] {
        diff_sdti(f);
    }
}

/// CONFIGS row 3 — in-range negative fractions (truncation toward zero = up).
fn cfg03_sdti_negative_fractions_randomized() {
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..20_000 {
        let base = (rng.next_u32() >> 1) as f64;
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let d = -(base + frac);
        if d >= i32::MIN as f64 {
            diff_sdti(d);
        }
    }
    for f in [-0.5, -0.9999999999, -1.5, -2.5, -1e-9, -42.75, -2147483647.5] {
        diff_sdti(f);
    }
}

/// CONFIGS row 4 — zeros and subnormals.
fn cfg04_sdti_zeros_and_subnormals() {
    for d in [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
    ] {
        diff_sdti(d);
    }
}

/// CONFIGS row 5 — the exact clamp boundaries and their ULP neighbours.
fn cfg05_sdti_range_boundaries_and_ulps() {
    let hi = i32::MAX as f64; // 2147483647.0
    let lo = i32::MIN as f64; // -2147483648.0
    let candidates = [
        hi,
        lo,
        -hi,
        -lo,
        2147483646.0,
        2147483646.5,
        2147483647.5,
        2147483648.0,
        2147483649.0,
        -2147483647.0,
        -2147483647.5,
        -2147483648.5,
        -2147483649.0,
        f64::from_bits(hi.to_bits() + 1),
        f64::from_bits(hi.to_bits() - 1),
        f64::from_bits(lo.to_bits() + 1), // one ULP further from zero (more negative)
        f64::from_bits(lo.to_bits() - 1), // one ULP toward zero
        (i32::MAX as f64).next_up(),
        (i32::MAX as f64).next_down(),
        (i32::MIN as f64).next_up(),
        (i32::MIN as f64).next_down(),
    ];
    for d in candidates {
        diff_sdti(d);
    }
}

/// CONFIGS row 6 — out-of-range magnitudes, both signs.
fn cfg06_sdti_out_of_range_randomized() {
    let mut rng = Rng::new(0x5EED_0006);
    for _ in 0..20_000 {
        let exp = 9.0 + (rng.next_u32() % 292) as f64; // 1e9 .. 1e300
        let mant = 1.0 + ((rng.next_u32() >> 8) as f64 / (1u32 << 24) as f64) * 9.0;
        let mag = mant * 10f64.powf(exp);
        diff_sdti(mag);
        diff_sdti(-mag);
    }
    for d in [1e15, -1e15, f64::MAX, f64::MIN, f64::INFINITY, f64::NEG_INFINITY] {
        diff_sdti(d);
    }
}

/// CONFIGS row 7 — completely arbitrary bit patterns (joint NaN / inf /
/// subnormal / normal coverage; this is where a naive `as` cast would diverge).
fn cfg07_sdti_random_bit_patterns() {
    let mut rng = Rng::new(0x5EED_0007);
    for _ in 0..50_000 {
        diff_sdti(rng.next_f64_bits());
    }
}

// ===========================================================================
// CONFIGS rows 8-15 — process_with_fallthrough
// ===========================================================================

#[track_caller]
fn diff_pwf(code: c_int, base: c_int) {
    let (c, r) = both();
    let cv = c.process_with_fallthrough(code, base);
    let rv = r.process_with_fallthrough(code, base);
    assert_int_eq(&format!("process_with_fallthrough({code}, {base})"), cv, rv);
}

fn pwf_row(seed: u64, code: c_int) {
    let mut rng = Rng::new(seed);
    for base in [0i32, 1, -1, 100, -100, i32::MAX, i32::MIN, i32::MAX - 149, i32::MAX - 150] {
        diff_pwf(code, base);
    }
    for _ in 0..20_000 {
        diff_pwf(code, rng.next_i32());
    }
}

/// CONFIGS row 8 — `code == 5`: falls through 5→4→3→2→1 (+150 total).
fn cfg08_pwf_code5_fallthrough_chain() {
    pwf_row(0x5EED_0008, 5);
}

/// CONFIGS row 9 — `code == 4` (+130).
fn cfg09_pwf_code4() {
    pwf_row(0x5EED_0009, 4);
}

/// CONFIGS row 10 — `code == 3` (+90).
fn cfg10_pwf_code3() {
    pwf_row(0x5EED_000A, 3);
}

/// CONFIGS row 11 — `code == 2` (+30).
fn cfg11_pwf_code2() {
    pwf_row(0x5EED_000B, 2);
}

/// CONFIGS row 12 — `code == 1` (+10, then `break`).
fn cfg12_pwf_code1() {
    pwf_row(0x5EED_000C, 1);
}

/// CONFIGS row 13 — `code == 0`: result forced to 0, `base_value` discarded.
fn cfg13_pwf_code0_discards_base() {
    pwf_row(0x5EED_000D, 0);
}

/// CONFIGS row 14 — the `default:` arm over randomized `base_value`.
fn cfg14_pwf_default_arm() {
    for code in [-1i32, -2, -5, -6, 6, 7, 100, i32::MAX, i32::MIN] {
        pwf_row(0x5EED_000E ^ (code as u64), code);
    }
}

/// CONFIGS row 15 — fully random `(code, base_value)` pairs.
fn cfg15_pwf_fully_randomized_pairs() {
    let mut rng = Rng::new(0x5EED_000F);
    for _ in 0..50_000 {
        diff_pwf(rng.next_i32(), rng.next_i32());
    }
    // Bias a chunk of samples into the interesting 0..=6 code window.
    for _ in 0..50_000 {
        let code = (rng.next_u32() % 9) as i32 - 1; // -1 ..= 7
        diff_pwf(code, rng.next_i32());
    }
}

// ===========================================================================
// CONFIGS rows 16-20 — copy_data_block
// ===========================================================================

#[track_caller]
fn diff_copy(src: &RawBlock, prefill: u8, ctx: &str) {
    let (c, r) = both();
    let cv = c.copy_block(src, prefill);
    let rv = r.copy_block(src, prefill);
    assert_bytes_eq(&format!("copy_data_block {ctx}"), &cv.0, &rv.0);
    // The whole 40-byte object, padding included, must equal the source.
    assert_bytes_eq(
        &format!("copy_data_block {ctx} (C vs source)"),
        &src.0,
        &cv.0,
    );
}

/// CONFIGS row 16 — zeroed source into a poisoned destination.
fn cfg16_copy_zeroed_source() {
    for prefill in [0x00u8, 0xAA, 0xFF] {
        diff_copy(&RawBlock::zeroed(), prefill, "zeroed");
    }
}

/// CONFIGS row 17 — all-`0xFF` source.
fn cfg17_copy_all_ones_source() {
    for prefill in [0x00u8, 0x5A, 0xFF] {
        diff_copy(&RawBlock::filled(0xFF), prefill, "all-0xFF");
    }
}

/// CONFIGS row 18 — fully random 40-byte patterns: random padding bytes,
/// `label` with no NUL terminator, `value` holding NaN/inf bit patterns.
fn cfg18_copy_random_patterns_incl_padding() {
    let mut rng = Rng::new(0x5EED_0012);
    for _ in 0..5_000 {
        let mut src = RawBlock::zeroed();
        for i in 0..DATA_BLOCK_SIZE {
            src.0[i] = (rng.next_u32() & 0xff) as u8;
        }
        let prefill = (rng.next_u32() & 0xff) as u8;
        diff_copy(&src, prefill, "random-40-bytes");
        // Padding bytes (4..8 and 36..40) must survive the copy verbatim.
        let (c, r) = both();
        let cv = c.copy_block(&src, prefill);
        let rv = r.copy_block(&src, prefill);
        assert_bytes_eq("padding gap 4..8", &cv.0[4..8], &rv.0[4..8]);
        assert_bytes_eq("trailing pad 36..40", &cv.0[36..40], &rv.0[36..40]);
        assert_bytes_eq("padding gap 4..8 vs src", &src.0[4..8], &cv.0[4..8]);
        assert_bytes_eq("trailing pad 36..40 vs src", &src.0[36..40], &cv.0[36..40]);
    }
}

/// CONFIGS row 19 — `dest == src` self copy.
fn cfg19_copy_self_aliased() {
    let mut rng = Rng::new(0x5EED_0013);
    for _ in 0..1_000 {
        let mut src = RawBlock::zeroed();
        for i in 0..DATA_BLOCK_SIZE {
            src.0[i] = (rng.next_u32() & 0xff) as u8;
        }
        let (c, r) = both();
        let cv = c.copy_block_self(&src);
        let rv = r.copy_block_self(&src);
        assert_bytes_eq("copy_data_block self-aliased", &cv.0, &rv.0);
        assert_bytes_eq("copy_data_block self-aliased is a no-op", &src.0, &cv.0);
    }
}

/// CONFIGS row 20 — field-wise construction and read-back through the struct
/// layout, including extreme `value`s and non-ASCII labels.
fn cfg20_copy_field_wise_roundtrip() {
    let mut rng = Rng::new(0x5EED_0014);
    let values = [
        0.0f64,
        -0.0,
        1.0,
        -1.5,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        1e-308,
        1.234_567_890_123_456_7e18,
    ];
    let labels: [&[u8]; 5] = [
        b"",
        b"Source",
        b"0123456789012345678", // 19 bytes + implicit NUL
        b"\xff\xfe\x80\x01\x00mixed",
        b"NoNulTerminatorHere!", // exactly 20 bytes, unterminated
    ];
    for v in values {
        for lab in labels {
            let id = rng.next_i32();
            let src = RawBlock::from_fields(id, v, lab);
            diff_copy(&src, 0xCC, "field-wise");
            let (c, r) = both();
            let cv = c.copy_block(&src, 0xCC);
            let rv = r.copy_block(&src, 0xCC);
            assert_int_eq("copied id", cv.id(), rv.id());
            assert_eq!(cv.value_bits(), rv.value_bits(), "copied value bit pattern");
            assert_bytes_eq("copied label", cv.label(), rv.label());
            assert_int_eq("copied id vs source", id, cv.id());
            assert_eq!(v.to_bits(), cv.value_bits(), "value bits vs source");
        }
    }
}

// ===========================================================================
// CONFIGS rows 21-22 — handle_pointer_operations
// ===========================================================================

#[track_caller]
fn diff_hpo(v: c_int) {
    let (c, r) = both();
    let cv = c.handle_pointer_operations(v);
    let rv = r.handle_pointer_operations(v);
    assert_int_eq(&format!("handle_pointer_operations({v})"), cv, rv);
}

/// CONFIGS row 21 — full-range randomized input (covers the `value * 2` wrap).
fn cfg21_hpo_randomized_full_range() {
    let mut rng = Rng::new(0x5EED_0015);
    for _ in 0..50_000 {
        diff_hpo(rng.next_i32());
    }
}

/// CONFIGS row 22 — boundary values around the `*2` overflow threshold.
fn cfg22_hpo_boundaries() {
    for v in [
        0i32,
        1,
        -1,
        2,
        -2,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX / 2 - 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        i32::MIN / 2 + 1,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        -50,
        -51,
    ] {
        diff_hpo(v);
    }
}

// ===========================================================================
// CONFIGS rows 23-41 — overunder (composed pipeline, return value + stdout)
// ===========================================================================

/// Build an `a` with the requested non-negative residue mod 6.
fn a_with_residue(rng: &mut Rng, residue: i32) -> i32 {
    loop {
        let k = rng.next_i32_bounded(300_000_000);
        let cand = k - (k % 6) + residue;
        if cand % 6 == residue {
            return cand;
        }
    }
}

fn overunder_residue_row(seed: u64, residue: i32) {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(seed);
    for _ in 0..300 {
        let a = a_with_residue(&mut rng, residue);
        assert_eq!(a % 6, residue, "test setup: residue");
        let b = rng.next_i32_bounded(100_000);
        let c = rng.next_i32_bounded(100_000);
        let d = rng.next_i32_bounded(30_000);
        diff_overunder(&mut cap, a, b, c, d);
    }
}

/// CONFIGS row 23 — `a % 6 == 0` (`switch_result` forced to 0).
fn cfg23_overunder_residue0() {
    overunder_residue_row(0x5EED_0017, 0);
}

/// CONFIGS row 24 — `a % 6 == 1`.
fn cfg24_overunder_residue1() {
    overunder_residue_row(0x5EED_0018, 1);
}

/// CONFIGS row 25 — `a % 6 == 2`.
fn cfg25_overunder_residue2() {
    overunder_residue_row(0x5EED_0019, 2);
}

/// CONFIGS row 26 — `a % 6 == 3`.
fn cfg26_overunder_residue3() {
    overunder_residue_row(0x5EED_001A, 3);
}

/// CONFIGS row 27 — `a % 6 == 4`.
fn cfg27_overunder_residue4() {
    overunder_residue_row(0x5EED_001B, 4);
}

/// CONFIGS row 28 — `a % 6 == 5` (the full fall-through chain).
fn cfg28_overunder_residue5() {
    overunder_residue_row(0x5EED_001C, 5);
}

/// CONFIGS row 29 — negative residues `-1..-5` reach the `default:` arm,
/// because C's `%` truncates toward zero.
fn cfg29_overunder_negative_residues() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_001D);
    for residue in 1..=5i32 {
        for _ in 0..120 {
            let a = -a_with_residue(&mut rng, residue).abs();
            let a = if a % 6 == 0 { a - residue } else { a };
            let b = rng.next_i32_bounded(100_000);
            let c = rng.next_i32_bounded(100_000);
            let d = rng.next_i32_bounded(30_000);
            diff_overunder(&mut cap, a, b, c, d);
        }
    }
}

/// CONFIGS row 30 — `d*d + a*a` stays positive, so `sqrt` gets a real value.
/// `|a|, |d| <= 46340` guarantees each square fits, and the sum is bounded by
/// 2·46340² < INT_MAX, so no overflow at all.
fn cfg30_overunder_sqrt_no_overflow() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_001E);
    for _ in 0..1_500 {
        let a = rng.next_i32_bounded(32_767);
        let d = rng.next_i32_bounded(32_767);
        let b = rng.next_i32_bounded(1_000_000);
        let c = rng.next_i32_bounded(1_000_000);
        diff_overunder(&mut cap, a, b, c, d);
    }
}

/// CONFIGS row 31 — `d*d + a*a` overflows to a negative int, so
/// `sqrt(negative)` is NaN and `safe_double_to_int` returns 0.
fn cfg31_overunder_sqrt_overflow_to_nan() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_001F);
    let mut nan_paths = 0usize;
    for _ in 0..1_500 {
        let a = rng.next_i32();
        let d = rng.next_i32();
        let sum = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        if sum >= 0 {
            continue;
        }
        nan_paths += 1;
        let b = rng.next_i32_bounded(100_000);
        let c = rng.next_i32_bounded(100_000);
        diff_overunder(&mut cap, a, b, c, d);
    }
    assert!(
        nan_paths > 50,
        "expected to hit the sqrt-of-negative path many times, hit {nan_paths}"
    );
}

/// CONFIGS row 32 — `a == 0 && d == 0` ⇒ `sqrt(0)`.
fn cfg32_overunder_sqrt_zero() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0020);
    for _ in 0..200 {
        let b = rng.next_i32_bounded(1_000_000);
        let c = rng.next_i32_bounded(1_000_000);
        diff_overunder(&mut cap, 0, b, c, 0);
    }
    diff_overunder(&mut cap, 0, 0, 0, 0);
}

/// CONFIGS row 33 — `a * 1.5` clamps to `INT_MAX` / `INT_MIN`.
fn cfg33_overunder_a_scale_clamps() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0021);
    let threshold = (i32::MAX as f64 / 1.5) as i32; // ~1431655765
    for a in [
        threshold,
        threshold + 1,
        threshold - 1,
        i32::MAX,
        i32::MAX - 1,
        -threshold,
        -threshold - 1,
        i32::MIN,
        i32::MIN + 1,
    ] {
        for _ in 0..40 {
            let b = rng.next_i32_bounded(100_000);
            let c = rng.next_i32_bounded(100_000);
            let d = rng.next_i32_bounded(30_000);
            diff_overunder(&mut cap, a, b, c, d);
        }
    }
}

/// CONFIGS row 34 — `b * 2.7` clamps to `INT_MAX` / `INT_MIN`.
fn cfg34_overunder_b_scale_clamps() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0022);
    let threshold = (i32::MAX as f64 / 2.7) as i32; // ~795364314
    for b in [
        threshold,
        threshold + 1,
        threshold - 1,
        i32::MAX,
        i32::MAX - 1,
        -threshold,
        -threshold - 1,
        i32::MIN,
        i32::MIN + 1,
    ] {
        for _ in 0..40 {
            let a = rng.next_i32_bounded(30_000);
            let c = rng.next_i32_bounded(100_000);
            let d = rng.next_i32_bounded(30_000);
            diff_overunder(&mut cap, a, b, c, d);
        }
    }
}

/// CONFIGS row 35 — `c / 3.3` division rounding across the full `int` range
/// (also drives `handle_pointer_operations(c)`, whose `*2` wraps for large `c`).
fn cfg35_overunder_c_division_full_range() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0023);
    for _ in 0..600 {
        let c = rng.next_i32();
        let a = rng.next_i32_bounded(30_000);
        let b = rng.next_i32_bounded(100_000);
        let d = rng.next_i32_bounded(30_000);
        diff_overunder(&mut cap, a, b, c, d);
    }
    for c in [0i32, 1, -1, 3, -3, 4, -4, i32::MAX, i32::MIN, i32::MAX / 2 + 1] {
        diff_overunder(&mut cap, 6, 1, c, 1);
    }
}

/// CONFIGS row 36 — the `total` accumulation wraps two's-complement.
fn cfg36_overunder_total_wraps() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0024);
    for _ in 0..300 {
        // Four arguments a handful of steps below INT_MAX ...
        let off = |rng: &mut Rng| (rng.next_u32() % 7) as i32;
        let (oa, ob, oc, od) = (
            off(&mut rng),
            off(&mut rng),
            off(&mut rng),
            off(&mut rng),
        );
        diff_overunder(
            &mut cap,
            i32::MAX - oa,
            i32::MAX - ob,
            i32::MAX - oc,
            i32::MAX - od,
        );
        // ... and the mirrored tuple a handful of steps above INT_MIN.
        diff_overunder(
            &mut cap,
            i32::MIN + oa,
            i32::MIN + ob,
            i32::MIN + oc,
            i32::MIN + od,
        );
    }
}

/// CONFIGS row 37 — fully random arguments over the entire `int` range: the
/// joint cross-product of every `overunder` axis.
fn cfg37_overunder_fully_randomized() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0x5EED_0025);
    for _ in 0..3_000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        diff_overunder(&mut cap, a, b, c, d);
    }
}

/// CONFIGS row 38 — extremal argument tuples, incl. the 16-way sign
/// cross-product of `INT_MAX` / `INT_MIN`.
fn cfg38_overunder_extremal_tuples() {
    let mut cap = Capturer::new();
    diff_overunder(&mut cap, 0, 0, 0, 0);
    diff_overunder(&mut cap, 1, 1, 1, 1);
    diff_overunder(&mut cap, -1, -1, -1, -1);
    let ext = [i32::MAX, i32::MIN];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    diff_overunder(&mut cap, a, b, c, d);
                }
            }
        }
    }
    for &v in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        diff_overunder(&mut cap, v, 0, 0, 0);
        diff_overunder(&mut cap, 0, v, 0, 0);
        diff_overunder(&mut cap, 0, 0, v, 0);
        diff_overunder(&mut cap, 0, 0, 0, v);
    }
}

/// CONFIGS row 39 — the stdout channel is compared byte-for-byte by
/// `diff_overunder` in every row above. This test additionally asserts that the
/// captured text really contains all eight `printf` groups, so that a silently
/// empty capture could never make the other rows pass vacuously.
fn cfg39_overunder_stdout_shape_and_equality() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let mut rng = Rng::new(0x5EED_0027);
    for _ in 0..200 {
        let a = rng.next_i32_bounded(50_000);
        let b = rng.next_i32_bounded(50_000);
        let c = rng.next_i32_bounded(50_000);
        let d = rng.next_i32_bounded(20_000);

        let (c_out, c_ret) = cap.run(|| capi.overunder(a, b, c, d));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(a, b, c, d));
        let ctx = format!("overunder stdout ({a},{b},{c},{d})");
        assert_int_eq(&ctx, c_ret, r_ret);
        assert_bytes_eq(&ctx, &c_out, &r_out);

        let text = String::from_utf8(c_out).expect("printf output is ASCII");
        for needle in [
            "result_1 = ",
            "result_2 = ",
            "Converted values: ",
            "Switch fall-through result: ",
            "Copied block: id=",
            "Pointer operation result: ",
            "Overflow protected conversion: 2147483647",
            "Underflow protected conversion: -2147483648",
            "Array copied via memcpy: ",
        ] {
            assert!(
                text.contains(needle),
                "captured stdout missing {needle:?}; got:\n{text}"
            );
        }
        assert_eq!(text.lines().count(), 9, "expected 9 printed lines:\n{text}");
        assert!(text.starts_with(&format!("result_1 = {a}\nresult_2 = {b}\n")));
    }
}

/// CONFIGS row 40 / ERRORS row 30 — `strncpy(label, "Source", 19)` plus the
/// forced `label[19] = '\0'`: the printed label is exactly `Source`, and the
/// 40-byte block the C builds is reproduced bit-for-bit by the Rust.
fn cfg_overunder_label_is_source_padded() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let mut rng = Rng::new(0x5EED_0028);

    for _ in 0..100 {
        let a = rng.next_i32_bounded(10_000);
        let (c_out, _) = cap.run(|| capi.overunder(a, 3, 4, 5));
        let (r_out, _) = cap.run(|| rapi.overunder(a, 3, 4, 5));
        assert_bytes_eq("label line", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        let line = text
            .lines()
            .find(|l| l.starts_with("Copied block:"))
            .expect("Copied block line present");
        assert!(
            line.ends_with("label=Source"),
            "expected the label to print as exactly `Source`, got {line:?}"
        );
        assert!(line.contains(&format!("id={a}")), "line = {line:?}");
    }

    // Independently reconstruct the block the C builds and push it through
    // `copy_data_block` in both libraries: "Source" + 13 NULs + label[19] = 0.
    let mut label = [0u8; 20];
    label[..6].copy_from_slice(b"Source");
    let src = RawBlock::from_fields(1234, 1234.0 * 1.5, &label);
    diff_copy(&src, 0xEE, "Source label");
    assert_eq!(&src.label()[..6], b"Source");
    assert!(src.label()[6..].iter().all(|&b| b == 0), "zero padded");
}

/// CONFIGS row 41 — prove the *composed pipeline* is wired identically, not
/// just each wrapper: recompute `overunder`'s return value purely from the four
/// low-level `.so` exports and compare against the real `overunder`.
fn cfg41_overunder_matches_recomposition_from_low_level_exports() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let mut rng = Rng::new(0x5EED_0029);

    for i in 0..1_500 {
        let (a, b, c, d) = if i % 3 == 0 {
            (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32())
        } else {
            (
                rng.next_i32_bounded(60_000),
                rng.next_i32_bounded(60_000),
                rng.next_i32_bounded(60_000),
                rng.next_i32_bounded(20_000),
            )
        };

        // Recompose using ONLY the low-level exports, once per library.
        let recompose = |api: &Api| -> c_int {
            let temp1 = a as f64 * 1.5;
            let temp2 = b as f64 * 2.7;
            let temp3 = c as f64 / 3.3;
            let temp4 =
                (d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) as f64).sqrt();
            let conv1 = api.safe_double_to_int(temp1);
            let conv2 = api.safe_double_to_int(temp2);
            let conv3 = api.safe_double_to_int(temp3);
            let conv4 = api.safe_double_to_int(temp4);
            let switch_result = api.process_with_fallthrough(a.wrapping_rem(6), b);
            let ptr_result = api.handle_pointer_operations(c);

            // The DataBlock round-trip through copy_data_block, exactly as the
            // C does it, so `dest_block.id` comes back through the FFI.
            let mut label = [0u8; 20];
            label[..6].copy_from_slice(b"Source");
            let src = RawBlock::from_fields(a, temp1, &label);
            let dest = api.copy_block(&src, 0x00);

            let mut total = conv1
                .wrapping_add(conv2)
                .wrapping_add(conv3)
                .wrapping_add(conv4)
                .wrapping_add(switch_result)
                .wrapping_add(ptr_result);
            total = total.wrapping_add(dest.id());
            for v in [a, b, c, d, a.wrapping_add(b)] {
                total = total.wrapping_add(v);
            }
            total
        };

        let c_recomposed = recompose(capi);
        let r_recomposed = recompose(rapi);
        let ctx = format!("recomposition ({a},{b},{c},{d})");
        assert_int_eq(&ctx, c_recomposed, r_recomposed);

        // ... and the real one-shot wrapper must agree with the recomposition.
        let actual = diff_overunder(&mut cap, a, b, c, d);
        assert_int_eq(&format!("{ctx} vs real overunder"), actual, c_recomposed);
    }
}

// ===========================================================================
// Sequential entry point (`harness = false`, see Cargo.toml).
//
// Every case below corresponds to a numbered row of the Phase A artifacts and
// is listed here explicitly so a forgotten registration is visible in review.
// ===========================================================================
fn main() -> ! {
    let cases: &[harness::Case] = &[
        ("layout_matches_c_abi", layout_matches_c_abi as fn()),
        ("both_libraries_expose_all_five_symbols", both_libraries_expose_all_five_symbols as fn()),
        ("cfg01_sdti_exact_integers_randomized", cfg01_sdti_exact_integers_randomized as fn()),
        ("cfg02_sdti_positive_fractions_randomized", cfg02_sdti_positive_fractions_randomized as fn()),
        ("cfg03_sdti_negative_fractions_randomized", cfg03_sdti_negative_fractions_randomized as fn()),
        ("cfg04_sdti_zeros_and_subnormals", cfg04_sdti_zeros_and_subnormals as fn()),
        ("cfg05_sdti_range_boundaries_and_ulps", cfg05_sdti_range_boundaries_and_ulps as fn()),
        ("cfg06_sdti_out_of_range_randomized", cfg06_sdti_out_of_range_randomized as fn()),
        ("cfg07_sdti_random_bit_patterns", cfg07_sdti_random_bit_patterns as fn()),
        ("cfg08_pwf_code5_fallthrough_chain", cfg08_pwf_code5_fallthrough_chain as fn()),
        ("cfg09_pwf_code4", cfg09_pwf_code4 as fn()),
        ("cfg10_pwf_code3", cfg10_pwf_code3 as fn()),
        ("cfg11_pwf_code2", cfg11_pwf_code2 as fn()),
        ("cfg12_pwf_code1", cfg12_pwf_code1 as fn()),
        ("cfg13_pwf_code0_discards_base", cfg13_pwf_code0_discards_base as fn()),
        ("cfg14_pwf_default_arm", cfg14_pwf_default_arm as fn()),
        ("cfg15_pwf_fully_randomized_pairs", cfg15_pwf_fully_randomized_pairs as fn()),
        ("cfg16_copy_zeroed_source", cfg16_copy_zeroed_source as fn()),
        ("cfg17_copy_all_ones_source", cfg17_copy_all_ones_source as fn()),
        ("cfg18_copy_random_patterns_incl_padding", cfg18_copy_random_patterns_incl_padding as fn()),
        ("cfg19_copy_self_aliased", cfg19_copy_self_aliased as fn()),
        ("cfg20_copy_field_wise_roundtrip", cfg20_copy_field_wise_roundtrip as fn()),
        ("cfg21_hpo_randomized_full_range", cfg21_hpo_randomized_full_range as fn()),
        ("cfg22_hpo_boundaries", cfg22_hpo_boundaries as fn()),
        ("cfg23_overunder_residue0", cfg23_overunder_residue0 as fn()),
        ("cfg24_overunder_residue1", cfg24_overunder_residue1 as fn()),
        ("cfg25_overunder_residue2", cfg25_overunder_residue2 as fn()),
        ("cfg26_overunder_residue3", cfg26_overunder_residue3 as fn()),
        ("cfg27_overunder_residue4", cfg27_overunder_residue4 as fn()),
        ("cfg28_overunder_residue5", cfg28_overunder_residue5 as fn()),
        ("cfg29_overunder_negative_residues", cfg29_overunder_negative_residues as fn()),
        ("cfg30_overunder_sqrt_no_overflow", cfg30_overunder_sqrt_no_overflow as fn()),
        ("cfg31_overunder_sqrt_overflow_to_nan", cfg31_overunder_sqrt_overflow_to_nan as fn()),
        ("cfg32_overunder_sqrt_zero", cfg32_overunder_sqrt_zero as fn()),
        ("cfg33_overunder_a_scale_clamps", cfg33_overunder_a_scale_clamps as fn()),
        ("cfg34_overunder_b_scale_clamps", cfg34_overunder_b_scale_clamps as fn()),
        ("cfg35_overunder_c_division_full_range", cfg35_overunder_c_division_full_range as fn()),
        ("cfg36_overunder_total_wraps", cfg36_overunder_total_wraps as fn()),
        ("cfg37_overunder_fully_randomized", cfg37_overunder_fully_randomized as fn()),
        ("cfg38_overunder_extremal_tuples", cfg38_overunder_extremal_tuples as fn()),
        ("cfg39_overunder_stdout_shape_and_equality", cfg39_overunder_stdout_shape_and_equality as fn()),
        ("cfg_overunder_label_is_source_padded", cfg_overunder_label_is_source_padded as fn()),
        ("cfg41_overunder_matches_recomposition_from_low_level_exports", cfg41_overunder_matches_recomposition_from_low_level_exports as fn())
    ];
    harness::run_suite("phase_b_valid", cases)
}
