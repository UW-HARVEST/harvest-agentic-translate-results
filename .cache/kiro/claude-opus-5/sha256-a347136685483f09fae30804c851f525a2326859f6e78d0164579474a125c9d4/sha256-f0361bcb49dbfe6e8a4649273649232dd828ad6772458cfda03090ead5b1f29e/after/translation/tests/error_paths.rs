//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The C library has no error channel at all (no error codes, no `assert`, no
//! null checks, no pointer parameters — see `ERRORS.md` for the mechanical
//! derivation), so "the same error/rejection" here means *the same exact
//! IEEE-754 exceptional result bit pattern*: the same infinity with the same
//! sign, or the same NaN with the same payload and sign bit. Asserting merely
//! "both produced some NaN" would be exactly the weak check this phase exists
//! to avoid, so every assertion below compares raw `u32` bits.

mod harness;

use harness::*;

/// Asserts C and Rust agree bit-for-bit, and additionally that the result has
/// the *class* the C is expected to produce, so a row cannot silently pass
/// because neither implementation reached the intended path.
#[track_caller]
fn expect_agree(row: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    match diff(row, p1, p2, p3, p) {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    }
}

#[track_caller]
fn assert_nan(row: &str, v: Vec2) {
    assert!(
        v.x.is_nan() && v.y.is_nan(),
        "{row}: expected both components NaN, got {v:?}"
    );
}

#[track_caller]
fn assert_not_finite(row: &str, v: Vec2) {
    assert!(
        !v.x.is_finite() || !v.y.is_finite(),
        "{row}: expected a non-finite component, got {v:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — coincident p1 == p2  =>  v1 == 0  =>  denom == +0.0, invDenom = +inf
// ---------------------------------------------------------------------------
#[test]
fn row01_coincident_p1_p2() {
    let mut rng = Rng::for_row("err01");
    for _ in 0..2000 {
        let a = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let b = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let p = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let out = expect_agree("err01", a, a, b, p);
        assert_not_finite("err01", out);
    }
    // Pinned case: exact expected bits are whatever the C produces, but the
    // shape is asserted explicitly.
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(4.0, 6.0);
    let p = Vec2::new(2.0, 3.0);
    let c = call_c(a, a, b, p);
    let r = call_rust(a, a, b, p);
    assert_eq!(c.to_bits(), r.to_bits(), "err01 pinned: {c:?} vs {r:?}");
    // dot11 == dot01 == 0 => u numerator is +-0.0, invDenom == +inf => u is NaN
    assert!(c.x.is_nan(), "err01: u should be 0*inf = NaN, got {c:?}");
}

// ---------------------------------------------------------------------------
// Row 2 — coincident p1 == p3  =>  v0 == 0
// ---------------------------------------------------------------------------
#[test]
fn row02_coincident_p1_p3() {
    let mut rng = Rng::for_row("err02");
    for _ in 0..2000 {
        let a = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let b = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let p = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let out = expect_agree("err02", a, b, a, p);
        assert_not_finite("err02", out);
    }
    let a = Vec2::new(-3.0, 0.5);
    let b = Vec2::new(7.0, -2.25);
    let p = Vec2::new(1.0, 1.0);
    let c = call_c(a, b, a, p);
    assert_eq!(c.to_bits(), call_rust(a, b, a, p).to_bits());
    // dot00 == dot01 == 0 => v numerator is +-0.0, invDenom == +inf => v is NaN
    assert!(c.y.is_nan(), "err02: v should be 0*inf = NaN, got {c:?}");
}

// ---------------------------------------------------------------------------
// Row 3 — all three vertices coincident: both numerators +0.0, denom +0.0
// ---------------------------------------------------------------------------
#[test]
fn row03_all_coincident() {
    let mut rng = Rng::for_row("err03");
    for _ in 0..2000 {
        let a = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let p = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let out = expect_agree("err03", a, a, a, p);
        assert_nan("err03", out);
    }
    // Pinned: p1 == p2 == p3 == origin, p arbitrary.
    let z = Vec2::new(0.0, 0.0);
    let c = call_c(z, z, z, Vec2::new(1.0, 1.0));
    let r = call_rust(z, z, z, Vec2::new(1.0, 1.0));
    assert_eq!(c.to_bits(), r.to_bits(), "err03 pinned");
    assert_nan("err03", c);
}

// ---------------------------------------------------------------------------
// Row 4 — exactly collinear, distinct vertices: Cauchy-Schwarz equality
// ---------------------------------------------------------------------------
#[test]
fn row04_collinear_distinct() {
    // p1 = (0,0), p2 = (k,k), p3 = (m,m): exactly collinear, denom == +0.0
    let mut cases = Vec::new();
    for k in 1..12i32 {
        for m in 1..12i32 {
            if k == m {
                continue;
            }
            cases.push([
                Vec2::new(0.0, 0.0),
                Vec2::new(k as f32, k as f32),
                Vec2::new(m as f32, m as f32),
                Vec2::new(3.0, 5.0),
            ]);
        }
    }
    for &[a, b, c_, p] in &cases {
        let out = expect_agree("err04", a, b, c_, p);
        assert_not_finite("err04", out);
    }
    // The denominator really is exactly zero for these.
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(1.0, 1.0);
    let c_ = Vec2::new(2.0, 2.0);
    let out = call_c(a, b, c_, Vec2::new(3.0, 5.0));
    assert!(
        !out.x.is_finite() || !out.y.is_finite(),
        "err04: collinear triangle should divide by zero, got {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 5 — denominator driven NEGATIVE by cancellation  =>  negative invDenom,
// and -inf when |denom| < 1/FLT_MAX. (denom == -0.0 is unreachable; see
// ERRORS.md row 5 for the proof and the sweep that confirms it.)
// ---------------------------------------------------------------------------
#[test]
fn row05_negative_denominator() {
    let mut rng = Rng::for_row("err05");
    let mut saw_negative_denom = 0usize;
    let mut saw_neg_inf_invdenom = 0usize;
    let mut saw_negative_zero_denom = 0usize;

    for _ in 0..200_000 {
        // v0 and v1 near-parallel at a random scale: the exact denominator is a
        // tiny non-negative (the squared cross product), so the float
        // computation cancels catastrophically and can land on either sign.
        let e = rng.below(61) as i32 - 30;
        let sc = (2.0f64).powi(e) as f32;
        let v0 = Vec2::new(rng.unit() * sc, rng.unit() * sc);
        let k = rng.unit() * 2.0;
        let pert = sc * (2.0f32).powi(-(rng.below(24) as i32));
        let v1 = Vec2::new(v0.x * k + rng.unit() * pert, v0.y * k + rng.unit() * pert);

        let p1 = Vec2::new(0.0, 0.0);
        let p3 = v0;
        let p2 = v1;
        let p = Vec2::new(rng.unit() * sc, rng.unit() * sc);

        // Reproduce the C's denominator to classify the case (the differential
        // assertion below is what actually validates the translation).
        let dot00 = v0.y * v0.y + v0.x * v0.x;
        let dot11 = v1.y * v1.y + v1.x * v1.x;
        let dot01 = v1.y * v0.y + v0.x * v1.x;
        let denom = dot00 * dot11 - dot01 * dot01;
        if !denom.is_nan() {
            if denom == 0.0 && denom.is_sign_negative() {
                saw_negative_zero_denom += 1;
            } else if denom < 0.0 {
                saw_negative_denom += 1;
                if 1.0f32 / denom == f32::NEG_INFINITY {
                    saw_neg_inf_invdenom += 1;
                }
            }
        }

        expect_agree("err05", p1, p2, p3, p);
    }

    assert!(
        saw_negative_denom > 0,
        "err05: the sweep never produced a negative denominator, so the path was \
         not exercised"
    );
    assert!(
        saw_neg_inf_invdenom > 0,
        "err05: the sweep never produced invDenom == -inf ({saw_negative_denom} \
         negative denominators seen), so the -inf path was not exercised"
    );
    assert_eq!(
        saw_negative_zero_denom, 0,
        "err05: a -0.0 denominator was observed, contradicting the ERRORS.md \
         row 5 argument that it is unreachable"
    );

    // Pinned case from the reachability sweep: denom == 0x80200000 (a negative
    // subnormal) => invDenom == -inf.
    let p1 = Vec2::new(0.0, 0.0);
    let p3 = Vec2::bits(0xb167_2278, 0xb230_de6e);
    let p2 = Vec2::bits(0x3165_309c, 0x322f_617c);
    let p = Vec2::bits(0x3000_0000, 0xb000_0000);
    let c = call_c(p1, p2, p3, p);
    let r = call_rust(p1, p2, p3, p);
    assert_eq!(c.to_bits(), r.to_bits(), "err05 pinned: C {c:?} vs Rust {r:?}");
}

// ---------------------------------------------------------------------------
// Row 6 — denominator underflows from non-zero products
// ---------------------------------------------------------------------------
#[test]
fn row06_denominator_underflow() {
    let mut rng = Rng::for_row("err06");
    let mut saw_inf = false;
    for _ in 0..4000 {
        let p1 = Vec2::new(rng.scaled(1.0e-23), rng.scaled(1.0e-23));
        let p2 = Vec2::new(rng.scaled(1.0e-23), rng.scaled(1.0e-23));
        let p3 = Vec2::new(rng.scaled(1.0e-23), rng.scaled(1.0e-23));
        let p = Vec2::new(rng.scaled(1.0e-23), rng.scaled(1.0e-23));
        let out = expect_agree("err06", p1, p2, p3, p);
        if !out.x.is_finite() || !out.y.is_finite() {
            saw_inf = true;
        }
    }
    assert!(
        saw_inf,
        "err06: tiny coordinates never underflowed the denominator to zero"
    );
}

// ---------------------------------------------------------------------------
// Row 7 — denominator overflows to +inf  =>  invDenom = +0.0
// ---------------------------------------------------------------------------
#[test]
fn row07_denominator_overflow_pos() {
    let mut rng = Rng::for_row("err07");
    let mut saw_nan = false;
    for _ in 0..4000 {
        let p1 = Vec2::new(rng.scaled(1.0e30), rng.scaled(1.0e30));
        let p2 = Vec2::new(rng.scaled(1.0e30), rng.scaled(1.0e30));
        let p3 = Vec2::new(rng.scaled(1.0e30), rng.scaled(1.0e30));
        let p = Vec2::new(rng.scaled(1.0e30), rng.scaled(1.0e30));
        let out = expect_agree("err07", p1, p2, p3, p);
        if out.x.is_nan() || out.y.is_nan() {
            saw_nan = true;
        }
    }
    assert!(
        saw_nan,
        "err07: huge coordinates never overflowed into the inf/NaN tail"
    );

    // Pinned: legs 1e30 and 1e20 => dot00*dot11 = 1e100 => +inf => invDenom +0.
    let p1 = Vec2::new(0.0, 0.0);
    let p3 = Vec2::new(1.0e30, 0.0);
    let p2 = Vec2::new(0.0, 1.0e20);
    let p = Vec2::new(1.0e10, 1.0e10);
    let c = call_c(p1, p2, p3, p);
    assert_eq!(c.to_bits(), call_rust(p1, p2, p3, p).to_bits(), "err07 pinned");
}

// ---------------------------------------------------------------------------
// Row 8 — denominator is -inf  =>  invDenom = -0.0
// ---------------------------------------------------------------------------
#[test]
fn row08_denominator_overflow_neg() {
    // dot01*dot01 overflows while dot00*dot11 stays finite => denom = -inf.
    let p1 = Vec2::new(0.0, 0.0);
    let cases: Vec<[Vec2; 4]> = (1..40)
        .map(|i| {
            let s = 1.0e19f32 * i as f32;
            [
                p1,
                Vec2::new(s, s),
                Vec2::new(s, -s * 0.5),
                Vec2::new(1.0, 2.0),
            ]
        })
        .collect();
    for &[a, b, c_, p] in &cases {
        expect_agree("err08", a, b, c_, p);
    }
    // Also a broad randomized sweep in the overflow regime, with negative signs.
    let mut rng = Rng::for_row("err08");
    for _ in 0..4000 {
        let s = 1.0e19f32;
        let p2 = Vec2::new(rng.scaled(s), rng.scaled(s));
        let p3 = Vec2::new(rng.scaled(s), rng.scaled(s));
        let p = Vec2::new(rng.scaled(s), rng.scaled(s));
        expect_agree("err08", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — a single +inf component
// ---------------------------------------------------------------------------
#[test]
fn row09_component_pos_inf() {
    let mut rng = Rng::for_row("err09");
    for slot in 0..8usize {
        for _ in 0..500 {
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            set_slot(&mut vs, slot, f32::INFINITY);
            let out = expect_agree("err09", vs[0], vs[1], vs[2], vs[3]);
            assert_not_finite("err09", out);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — a single -inf component
// ---------------------------------------------------------------------------
#[test]
fn row10_component_neg_inf() {
    let mut rng = Rng::for_row("err10");
    for slot in 0..8usize {
        for _ in 0..500 {
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            set_slot(&mut vs, slot, f32::NEG_INFINITY);
            let out = expect_agree("err10", vs[0], vs[1], vs[2], vs[3]);
            assert_not_finite("err10", out);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — p == p1 exactly: numerators are signed zeros
// ---------------------------------------------------------------------------
#[test]
fn row11_p_equals_p1() {
    let mut rng = Rng::for_row("err11");
    for _ in 0..4000 {
        let p1 = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let p2 = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let p3 = Vec2::new(rng.scaled(8.0), rng.scaled(8.0));
        let out = expect_agree("err11", p1, p2, p3, p1);
        // Both barycentrics must be a zero (of some sign) or NaN if degenerate.
        assert!(
            (out.x == 0.0 || out.x.is_nan()) && (out.y == 0.0 || out.y.is_nan()),
            "err11: expected zeros/NaN, got {out:?}"
        );
    }
    // Signed-zero variants of p1 itself.
    for &(x, y) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        let p1 = Vec2::new(x, y);
        let p2 = Vec2::new(1.0, 0.0);
        let p3 = Vec2::new(0.0, 1.0);
        expect_agree("err11-signed-zero", p1, p2, p3, p1);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — quiet NaN input
// ---------------------------------------------------------------------------
#[test]
fn row12_quiet_nan() {
    let mut rng = Rng::for_row("err12");
    for slot in 0..8usize {
        for &n in &[QNAN, NAN_A, NAN_B, NAN_D] {
            for _ in 0..250 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, n);
                let out = expect_agree("err12", vs[0], vs[1], vs[2], vs[3]);
                assert_nan("err12", out);
            }
        }
    }
    // A quiet NaN's payload must survive into the result, not be replaced by a
    // freshly minted default NaN.
    let vs = [
        Vec2::new(f32::from_bits(0x7fc0_1357), 1.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(0.0, 3.0),
        Vec2::new(0.5, 0.5),
    ];
    let c = call_c(vs[0], vs[1], vs[2], vs[3]);
    assert_eq!(c.to_bits(), call_rust(vs[0], vs[1], vs[2], vs[3]).to_bits());
    assert_eq!(
        c.x.to_bits() & 0x007f_ffff,
        0x0040_1357,
        "err12: payload 0x1357 should be preserved, got {:#010x}",
        c.x.to_bits()
    );
}

// ---------------------------------------------------------------------------
// Row 13 — signalling NaN input must be QUIETED identically (no trap)
// ---------------------------------------------------------------------------
#[test]
fn row13_signalling_nan() {
    let mut rng = Rng::for_row("err13");
    for slot in 0..8usize {
        for &n in &[SNAN, SNAN_NEG, f32::from_bits(0x7f80_1234), f32::from_bits(0x7fbf_ffff)] {
            for _ in 0..250 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, n);
                let out = expect_agree("err13", vs[0], vs[1], vs[2], vs[3]);
                assert_nan("err13", out);
                // The result must be QUIET in both (bit 22 set): an sNaN must
                // never leak through unquieted.
                assert!(
                    out.x.to_bits() & 0x0040_0000 != 0 && out.y.to_bits() & 0x0040_0000 != 0,
                    "err13: sNaN {:#010x} leaked unquieted: {out:?}",
                    n.to_bits()
                );
            }
        }
    }
    // Pinned: sNaN 0x7f800001 must become qNaN 0x7fc00001, payload retained.
    let vs = [
        Vec2::new(SNAN, 1.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(0.0, 3.0),
        Vec2::new(0.5, 0.5),
    ];
    let c = call_c(vs[0], vs[1], vs[2], vs[3]);
    assert_eq!(c.to_bits(), call_rust(vs[0], vs[1], vs[2], vs[3]).to_bits());
    assert_eq!(
        c.x.to_bits(),
        0x7fc0_0001,
        "err13: expected quieted 0x7fc00001, got {:#010x}",
        c.x.to_bits()
    );
}

// ---------------------------------------------------------------------------
// Row 14 — two DIFFERENT NaN payloads meeting in one commutative operation
// ---------------------------------------------------------------------------
#[test]
fn row14_two_distinct_nan_payloads() {
    // Exhaustive over ordered slot pairs x an assortment of payload pairs.
    const PAYLOADS: &[(u32, u32)] = &[
        (0x7fc0_00aa, 0x7fc0_00bb),
        (0x7fc0_00bb, 0x7fc0_00aa),
        (0x7fc0_0001, 0xffc0_0001),
        (0xffc0_00cc, 0x7fd0_00dd),
        (0x7f80_0001, 0x7fc0_ffff),
        (0x7fff_ffff, 0xff80_0002),
    ];
    let mut rng = Rng::for_row("err14");
    let mut divergences = 0usize;
    let mut total = 0usize;
    let mut reports = Vec::new();
    for s1 in 0..8usize {
        for s2 in 0..8usize {
            if s1 == s2 {
                continue;
            }
            for &(a, b) in PAYLOADS {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, s1, f32::from_bits(a));
                set_slot(&mut vs, s2, f32::from_bits(b));
                total += 1;
                if let Err(e) = diff("err14", vs[0], vs[1], vs[2], vs[3]) {
                    divergences += 1;
                    if reports.len() < 3 {
                        reports.push(e);
                    }
                }
            }
        }
    }
    assert_eq!(
        divergences,
        0,
        "err14: {divergences}/{total} distinct-NaN-payload cases diverged\n{}",
        reports.join("\n---\n")
    );

    // Three or more distinct payloads at once.
    for _ in 0..5000 {
        let mut vs = [
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
        ];
        for slot in 0..8 {
            if rng.bool() {
                let payload = (rng.next_u32() & 0x007f_ffff).max(1);
                let sign = if rng.bool() { 0x8000_0000u32 } else { 0 };
                set_slot(&mut vs, slot, f32::from_bits(sign | 0x7f80_0000 | payload));
            }
        }
        expect_agree("err14-multi", vs[0], vs[1], vs[2], vs[3]);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — negative NaN: the sign bit must propagate identically
// ---------------------------------------------------------------------------
#[test]
fn row15_negative_nan() {
    let mut rng = Rng::for_row("err15");
    let mut saw_negative_result = false;
    for slot in 0..8usize {
        for &n in &[QNAN_NEG, NAN_C, SNAN_NEG, f32::from_bits(0xffff_ffff)] {
            for _ in 0..250 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, n);
                let out = expect_agree("err15", vs[0], vs[1], vs[2], vs[3]);
                assert_nan("err15", out);
                if out.x.to_bits() & 0x8000_0000 != 0 {
                    saw_negative_result = true;
                }
            }
        }
    }
    assert!(
        saw_negative_result,
        "err15: a negative NaN input never produced a negative NaN result, so \
         the sign-propagation path was not actually exercised"
    );
}

// ---------------------------------------------------------------------------
// Row 16 — subnormal inputs (an FTZ/DAZ mismatch detector)
// ---------------------------------------------------------------------------
#[test]
fn row16_subnormal() {
    // Exhaustive over a set of subnormal magnitudes in every slot.
    let subs: Vec<f32> = (0..24)
        .map(|k| f32::from_bits(1u32 << k))
        .filter(|f| f.to_bits() < 0x0080_0000)
        .collect();
    let mut rng = Rng::for_row("err16");
    for slot in 0..8usize {
        for &s in &subs {
            for &sv in &[s, -s] {
                for _ in 0..40 {
                    let mut vs = [
                        rng.vec_with(|r| r.scaled(1.0)),
                        rng.vec_with(|r| r.scaled(1.0)),
                        rng.vec_with(|r| r.scaled(1.0)),
                        rng.vec_with(|r| r.scaled(1.0)),
                    ];
                    set_slot(&mut vs, slot, sv);
                    expect_agree("err16", vs[0], vs[1], vs[2], vs[3]);
                }
            }
        }
    }
    // All-subnormal input: every intermediate is subnormal or zero. If either
    // build had FTZ/DAZ enabled, these would disagree.
    for _ in 0..4000 {
        let pick = |r: &mut Rng| {
            let bits = (r.next_u32() % 0x0080_0000).max(1);
            let sign = if r.bool() { 0x8000_0000u32 } else { 0 };
            f32::from_bits(sign | bits)
        };
        let vs = [
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
        ];
        expect_agree("err16-all-subnormal", vs[0], vs[1], vs[2], vs[3]);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — negative zero inputs
// ---------------------------------------------------------------------------
#[test]
fn row17_negative_zero() {
    // Exhaustive: every slot independently 0.0 or -0.0 (256 combinations),
    // and every slot independently 0.0 / -0.0 / 1.0 / -1.0 (65536 combinations).
    let pool = [0.0f32, -0.0, 1.0, -1.0];
    let mut cases = Vec::with_capacity(65536);
    for m in 0u32..65536 {
        let mut vs = [Vec2::new(0.0, 0.0); 4];
        for slot in 0..8 {
            let idx = ((m >> (2 * slot)) & 0b11) as usize;
            set_slot(&mut vs, slot, pool[idx]);
        }
        cases.push(vs);
    }
    let mut divergences = 0usize;
    let mut reports = Vec::new();
    for &[a, b, c_, p] in &cases {
        if let Err(e) = diff("err17", a, b, c_, p) {
            divergences += 1;
            if reports.len() < 3 {
                reports.push(e);
            }
        }
    }
    assert_eq!(
        divergences,
        0,
        "err17: {divergences}/{} signed-zero cases diverged\n{}",
        cases.len(),
        reports.join("\n---\n")
    );
}

// ---------------------------------------------------------------------------
// Row 18 — all components at the maximum finite float
// ---------------------------------------------------------------------------
#[test]
fn row18_all_max_finite() {
    let m = f32::MAX;
    let cases: Vec<[Vec2; 4]> = {
        let mut v = Vec::new();
        for mask in 0u32..256 {
            let mut vs = [Vec2::new(m, m); 4];
            for slot in 0..8 {
                let s = if mask & (1 << slot) != 0 { -m } else { m };
                set_slot(&mut vs, slot, s);
            }
            v.push(vs);
        }
        v
    };
    for &[a, b, c_, p] in &cases {
        expect_agree("err18", a, b, c_, p);
    }
    // The canonical all-MAX case must be a NaN in both (inf - inf).
    let out = call_c(
        Vec2::new(m, m),
        Vec2::new(m, m),
        Vec2::new(m, m),
        Vec2::new(m, m),
    );
    assert_nan("err18", out);
}

// ---------------------------------------------------------------------------
// Row 19 — struct layout parity, and garbage in the XMM upper eightbyte
// ---------------------------------------------------------------------------
#[test]
fn row19_struct_layout_and_upper_bits() {
    assert_eq!(std::mem::size_of::<Vec2>(), 8, "sizeof(lm_vec2)");
    assert_eq!(std::mem::align_of::<Vec2>(), 4, "_Alignof(lm_vec2)");

    // `lm_vec2` has no pointer members and the API has no pointer parameters,
    // so "pass a null pointer" is not an expressible input. Assert that
    // mechanically against the actual header rather than trusting a comment.
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/include/lib.h"
    ))
    .expect("read c_src/include/lib.h");
    assert!(
        !header.contains('*'),
        "lib.h now contains a pointer type; ERRORS.md must gain null-pointer rows:\n{header}"
    );
    // Likewise there are no enum parameters, so there is no out-of-range enum
    // value to smuggle across the FFI boundary. Every one of the 2^32 bit
    // patterns of a `float` argument IS a valid input, and rows 12-18 plus
    // CONFIGS.md row 25 cover them by class.
    assert!(
        !header.contains("enum"),
        "lib.h now declares an enum; ERRORS.md must gain out-of-range-enum rows"
    );

    #[cfg(target_arch = "x86_64")]
    {
        let im = impls();
        let mut rng = Rng::for_row("err19");
        // Garbage patterns include NaNs and infinities, which is what would
        // corrupt a result if either side illegally read the upper lane.
        let garbages: [[u32; 2]; 6] = [
            [0x0000_0000, 0x0000_0000],
            [0xffff_ffff, 0xffff_ffff],
            [0x7fc0_00aa, 0xffc0_00bb],
            [0x7f80_0000, 0xff80_0000],
            [0x0000_0001, 0x007f_ffff],
            [0xdead_beef, 0xcafe_babe],
        ];
        for _ in 0..400 {
            // Degenerate / NaN-laden inputs: the error paths, not happy paths.
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            match rng.below(3) {
                0 => vs[1] = vs[0], // degenerate
                1 => set_slot(&mut vs, rng.below(8) as usize, QNAN),
                _ => set_slot(&mut vs, rng.below(8) as usize, f32::INFINITY),
            }
            let reference = call_c(vs[0], vs[1], vs[2], vs[3]);
            for g in garbages {
                let dc = call_dirty(im.c_addr, vs, g);
                let dr = call_dirty(im.rust_addr, vs, g);
                assert_eq!(
                    dc.to_bits(),
                    dr.to_bits(),
                    "err19: dirty-XMM divergence, garbage={g:08x?} vs={vs:?}: C {dc:?} Rust {dr:?}"
                );
                assert_eq!(
                    dc.to_bits(),
                    reference.to_bits(),
                    "err19: upper-lane garbage changed the C result, garbage={g:08x?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — reentrancy on the error paths (no shared mutable state)
// ---------------------------------------------------------------------------
#[test]
fn row20_thread_safety() {
    let mut rng = Rng::for_row("err20");
    // Inputs that all land on exceptional paths.
    let inputs: Vec<[Vec2; 4]> = (0..400)
        .map(|_| {
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            match rng.below(5) {
                0 => vs[1] = vs[0],
                1 => vs[2] = vs[0],
                2 => {
                    vs[1] = vs[0];
                    vs[2] = vs[0];
                }
                3 => set_slot(&mut vs, rng.below(8) as usize, NAN_A),
                _ => set_slot(&mut vs, rng.below(8) as usize, f32::NEG_INFINITY),
            }
            vs
        })
        .collect();

    let expected: Vec<(u32, u32)> = inputs
        .iter()
        .map(|&[a, b, c_, p]| {
            let out = call_c(a, b, c_, p);
            assert_eq!(
                call_rust(a, b, c_, p).to_bits(),
                out.to_bits(),
                "err20: single-threaded divergence on {a:?} {b:?} {c_:?} {p:?}"
            );
            out.to_bits()
        })
        .collect();

    let inputs = std::sync::Arc::new(inputs);
    let expected = std::sync::Arc::new(expected);
    let mut handles = Vec::new();
    for _ in 0..8 {
        let inputs = inputs.clone();
        let expected = expected.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..25 {
                for (i, &[a, b, c_, p]) in inputs.iter().enumerate() {
                    assert_eq!(call_c(a, b, c_, p).to_bits(), expected[i], "C not reentrant");
                    assert_eq!(
                        call_rust(a, b, c_, p).to_bits(),
                        expected[i],
                        "Rust not reentrant / != C"
                    );
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread");
    }
}
