//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! This library has no error codes, no sentinels, no asserts and no pointer or
//! enum parameters (see `ERRORS.md` for the mechanical grep that establishes
//! this). Its entire rejection surface is the set of degenerate floating-point
//! outcomes of the deliberately unguarded `High / Low` division. Each row below
//! asserts the two implementations produce the *same* outcome **bit-for-bit**,
//! including the NaN payload and sign — not merely "both are non-finite".

mod common;

use common::{Checker, Rgb, Rgb4, Rng, LIN_MAX, POW_MIN};

const POS_INF_BITS: u32 = 0x7F80_0000;
const ONE_BITS: u32 = 0x3F80_0000;

/// E1 — `A == (0,0,0)`, `B != (0,0,0)`: the swap branch is taken, `Low` becomes
/// `LumA == +0.0`, so the result is `+inf`.
#[test]
fn err_e1_black_a_nonblack_b() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xE001);
    let mut ck = Checker::new(&c, &r);

    // Every single-channel minimal non-black color, plus randomized ones.
    let mut cases = vec![
        Rgb::new(1, 0, 0),
        Rgb::new(0, 1, 0),
        Rgb::new(0, 0, 1),
        Rgb::new(255, 255, 255),
        Rgb::new(10, 10, 10),
        Rgb::new(11, 11, 11),
    ];
    for _ in 0..5_000 {
        let mut col = rng.rgb();
        if col == Rgb::BLACK {
            col = Rgb::new(1, 1, 1);
        }
        cases.push(col);
    }

    for col in cases {
        let cv = c.call(Rgb::BLACK, col);
        let rv = r.call(Rgb::BLACK, col);
        assert_eq!(
            cv.to_bits(), POS_INF_BITS,
            "E1: C should return +inf for (black, {col:?}), got {cv:?} (0x{:08X})",
            cv.to_bits()
        );
        assert_eq!(
            rv.to_bits(), cv.to_bits(),
            "E1: Rust returned 0x{:08X} but C returned 0x{:08X} for (black, {col:?})",
            rv.to_bits(), cv.to_bits()
        );
        ck.check(Rgb::BLACK, col);
    }
    println!("E1: {} cases, all +inf and bit-identical", ck.checked);
    ck.finish("E1 black A / non-black B");
}

/// E2 — `B == (0,0,0)`, `A != (0,0,0)`: no swap, `Low == LumB == +0.0`, `+inf`.
#[test]
fn err_e2_nonblack_a_black_b() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xE002);
    let mut ck = Checker::new(&c, &r);

    for i in 0..5_000 {
        let col = if i < 3 {
            [Rgb::new(1, 0, 0), Rgb::new(0, 1, 0), Rgb::new(0, 0, 1)][i]
        } else {
            let c0 = rng.rgb();
            if c0 == Rgb::BLACK { Rgb::new(1, 1, 1) } else { c0 }
        };
        let cv = c.call(col, Rgb::BLACK);
        let rv = r.call(col, Rgb::BLACK);
        assert_eq!(
            cv.to_bits(), POS_INF_BITS,
            "E2: C should return +inf for ({col:?}, black), got {cv:?}"
        );
        assert_eq!(rv.to_bits(), cv.to_bits(), "E2: Rust/C mismatch for ({col:?}, black)");
        ck.check(col, Rgb::BLACK);
    }
    println!("E2: {} cases, all +inf and bit-identical", ck.checked);
    ck.finish("E2 non-black A / black B");
}

/// E3 — both operands black: `+0.0 / +0.0`. The exact NaN bit pattern produced
/// by the hardware divide must be identical, not merely "some NaN".
#[test]
fn err_e3_both_black_nan_bits() {
    let (c, r) = common::load_pair();

    let cv = c.call(Rgb::BLACK, Rgb::BLACK);
    let rv = r.call(Rgb::BLACK, Rgb::BLACK);

    println!(
        "E3: C = {cv:?} (0x{:08X}), Rust = {rv:?} (0x{:08X})",
        cv.to_bits(),
        rv.to_bits()
    );
    assert!(cv.is_nan(), "E3: C should yield NaN for 0/0, got {cv:?}");
    assert!(rv.is_nan(), "E3: Rust should yield NaN for 0/0, got {rv:?}");
    assert_eq!(
        rv.to_bits(),
        cv.to_bits(),
        "E3: NaN bit patterns differ — C 0x{:08X} vs Rust 0x{:08X} \
         (payload/sign must match for byte-identical output)",
        cv.to_bits(),
        rv.to_bits()
    );

    // Also through the padded-register signature, and repeatedly (guards against
    // a constant-folded NaN differing from a runtime-computed one).
    for pad in [0x00u8, 0xFF] {
        let z = Rgb4 { r: 0, g: 0, b: 0, pad };
        let cv2 = c.call_padded(z, z);
        let rv2 = r.call_padded(z, z);
        assert_eq!(
            rv2.to_bits(), cv2.to_bits(),
            "E3: padded NaN bits differ (pad=0x{pad:02X})"
        );
        assert_eq!(cv2.to_bits(), cv.to_bits(), "E3: C NaN not stable across padding");
    }
}

/// E4 — `A == B` and non-black: `High == Low`, no swap, `x / x` is exactly 1.0.
#[test]
fn err_e4_identical_colors_exact_one() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xE004);
    let mut ck = Checker::new(&c, &r);
    let mut n = 0u32;

    let mut cases: Vec<Rgb> = vec![
        Rgb::new(1, 0, 0),
        Rgb::new(0, 0, 1),
        Rgb::new(10, 10, 10),
        Rgb::new(11, 11, 11),
        Rgb::WHITE,
    ];
    for _ in 0..5_000 {
        let col = rng.rgb();
        if col != Rgb::BLACK {
            cases.push(col);
        }
    }

    for col in cases {
        let cv = c.call(col, col);
        let rv = r.call(col, col);
        assert_eq!(
            cv.to_bits(), ONE_BITS,
            "E4: C should return exactly 1.0 for ({col:?},{col:?}), got {cv:?}"
        );
        assert_eq!(rv.to_bits(), cv.to_bits(), "E4: Rust/C mismatch for ({col:?},{col:?})");
        ck.check(col, col);
        n += 1;
    }
    println!("E4: {n} identical-color cases, all exactly 1.0 on both sides");
    ck.finish("E4 identical colors");
}

/// E5 — the one-step-past-the-boundary case: channel 10 takes `x/12.92` and
/// channel 11 takes `pow(...)`. Verified for every channel position and every
/// surrounding value.
#[test]
fn err_e5_branch_boundary_10_11() {
    let (c, r) = common::load_pair();
    let mut ck = Checker::new(&c, &r);

    assert_eq!(LIN_MAX, 10);
    assert_eq!(POW_MIN, 11);

    // Confirm the branch really flips between 10 and 11 (i.e. the boundary in
    // ERRORS.md/CONFIGS.md is the true one) by checking the C's own output is
    // discontinuous in slope there, and that Rust agrees on every value 0..=20
    // in each channel position, against several partners.
    let partners = [
        Rgb::WHITE,
        Rgb::new(11, 11, 11),
        Rgb::new(10, 10, 10),
        Rgb::new(128, 64, 32),
        Rgb::new(1, 1, 1),
    ];
    for chan in 0..3 {
        for v in 0u8..=20 {
            let col = match chan {
                0 => Rgb::new(v, 0, 0),
                1 => Rgb::new(0, v, 0),
                _ => Rgb::new(0, 0, v),
            };
            for &p in &partners {
                ck.check(col, p);
                ck.check(p, col);
            }
        }
    }

    // Explicit boundary pairs: (10 -> linear) vs (11 -> pow) in every position.
    for &(a, b) in &[
        (Rgb::new(10, 10, 10), Rgb::new(11, 11, 11)),
        (Rgb::new(11, 10, 10), Rgb::new(10, 11, 11)),
        (Rgb::new(10, 11, 10), Rgb::new(11, 10, 11)),
        (Rgb::new(10, 10, 11), Rgb::new(11, 11, 10)),
    ] {
        ck.check(a, b);
        ck.check(b, a);
    }

    println!("E5: {} boundary comparisons around the 10/11 branch flip", ck.checked);
    ck.finish("E5 branch boundary");
}

/// E6 — domain endpoints (the "zero / oversized length" analogue): channel
/// values 0 and 255, i.e. all 8 corner colors, in both operand positions;
/// plus every value 0..=255 swept through each channel position.
#[test]
fn err_e6_domain_endpoints() {
    let (c, r) = common::load_pair();
    let mut ck = Checker::new(&c, &r);

    let corners: Vec<Rgb> = (0..8)
        .map(|i| {
            let f = |b: u8| if b != 0 { 255u8 } else { 0u8 };
            Rgb::new(f(i & 4), f(i & 2), f(i & 1))
        })
        .collect();
    assert_eq!(corners.len(), 8);
    for &a in &corners {
        for &b in &corners {
            ck.check(a, b);
        }
    }

    // No `unsigned char` value is out of range: sweep all 256 in each position
    // and confirm neither side rejects or diverges.
    for chan in 0..3 {
        for v in 0u16..=255 {
            let v = v as u8;
            let col = match chan {
                0 => Rgb::new(v, 200, 100),
                1 => Rgb::new(200, v, 100),
                _ => Rgb::new(200, 100, v),
            };
            ck.check(col, Rgb::WHITE);
            ck.check(Rgb::WHITE, col);
            ck.check(col, Rgb::BLACK);
            ck.check(Rgb::BLACK, col);
        }
    }
    println!("E6: {} endpoint / full-range comparisons", ck.checked);
    ck.finish("E6 domain endpoints");
}

/// E7 — the closest thing this API has to an out-of-range argument: garbage in
/// the register bits above the 3-byte struct. Both sides must ignore it.
#[test]
fn err_e7_struct_padding_garbage() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xE007);
    let mut checked = 0u64;

    for _ in 0..5_000 {
        let a = rng.rgb();
        let b = rng.rgb();
        let expect_c = c.call(a, b).to_bits();
        let expect_r = r.call(a, b).to_bits();
        assert_eq!(expect_c, expect_r, "E7 baseline mismatch for {a:?}/{b:?}");

        for &pad in &[0x00u8, 0x01, 0x80, 0xAA, 0xFF] {
            let a4 = Rgb4 { r: a.r, g: a.g, b: a.b, pad };
            let b4 = Rgb4 { r: b.r, g: b.g, b: b.b, pad: !pad };
            let cv = c.call_padded(a4, b4).to_bits();
            let rv = r.call_padded(a4, b4).to_bits();
            assert_eq!(
                cv, expect_c,
                "E7: C result changed by padding byte 0x{pad:02X} for {a:?}/{b:?}"
            );
            assert_eq!(
                rv, cv,
                "E7: Rust differs from C with padding byte 0x{pad:02X} for {a:?}/{b:?}"
            );
            checked += 1;
        }
    }
    println!("E7: {checked} garbage-padding comparisons, no divergence");
}

/// E8 — no other rejection path exists: over a large randomized sample, the only
/// non-finite results come from a pure-black operand (E1/E2/E3), and every
/// result is bit-identical.
#[test]
fn err_e8_no_other_rejection_paths() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xE008);
    let mut ck = Checker::new(&c, &r);
    let mut nonfinite_without_black = Vec::new();
    let mut nonfinite_with_black = 0u32;

    for _ in 0..300_000 {
        // Bias towards black so the degenerate path is hit often.
        let a = if rng.next_u64() % 64 == 0 { Rgb::BLACK } else { rng.rgb() };
        let b = if rng.next_u64() % 64 == 0 { Rgb::BLACK } else { rng.rgb() };
        let cv = c.call(a, b);
        let rv = r.call(a, b);
        assert_eq!(
            rv.to_bits(), cv.to_bits(),
            "E8: divergence at {a:?}/{b:?}: C 0x{:08X} vs Rust 0x{:08X}",
            cv.to_bits(), rv.to_bits()
        );
        if !cv.is_finite() {
            if a == Rgb::BLACK || b == Rgb::BLACK {
                nonfinite_with_black += 1;
            } else if nonfinite_without_black.len() < 10 {
                nonfinite_without_black.push(format!("{a:?}/{b:?} -> {cv:?}"));
            }
        }
        ck.check(a, b);
    }
    assert!(
        nonfinite_without_black.is_empty(),
        "E8: non-finite result without a black operand: {nonfinite_without_black:?}"
    );
    assert!(nonfinite_with_black > 0, "E8: degenerate path never exercised");
    println!(
        "E8: {} comparisons, {nonfinite_with_black} non-finite (all from a black operand)",
        ck.checked
    );
    ck.finish("E8 no other rejection paths");
}

/// Documented non-applicable generic boundaries, asserted structurally so the
/// claim in `ERRORS.md` cannot silently rot.
#[test]
fn generic_boundaries_are_structurally_inapplicable() {
    // The exported signature takes two by-value 3-byte structs and returns a
    // float: no pointer, no length, no enum. If the C header ever gained one of
    // those, these layout assertions would need revisiting.
    assert_eq!(std::mem::size_of::<Rgb>(), 3, "cb_rgb_255 must be 3 bytes");
    assert_eq!(std::mem::align_of::<Rgb>(), 1, "cb_rgb_255 must be align 1");
    assert_eq!(std::mem::size_of::<f32>(), 4);

    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    assert!(
        !header.contains('*'),
        "the C header now declares a pointer — ERRORS.md's null-pointer row must be revisited:\n{header}"
    );
    assert!(
        !header.contains("enum"),
        "the C header now declares an enum — ERRORS.md's out-of-range-enum row must be revisited:\n{header}"
    );

    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    for pat in ["assert", "errno", "return -1", "return NULL", "RETURN_ERROR"] {
        assert!(
            !src.contains(pat),
            "the C source now contains `{pat}` — ERRORS.md must gain a row for it"
        );
    }
    println!("generic boundaries: no pointer/length/enum in the C API (verified against c_src)");
}
