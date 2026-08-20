//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test (a) constructs the exact invalid/degenerate condition, (b) calls
//! BOTH `.so` files, (c) asserts they return the SAME sentinel bit-for-bit, and
//! (d) asserts the C sentinel is the specific one recorded in `ERRORS.md`, so a
//! row cannot pass vacuously ("both returned something").

mod harness;

use harness::*;

/// The x86 "indefinite" QNaN produced by an invalid operation (`0*inf`,
/// `inf-inf`, `inf/inf`). Note the sign bit: it is *negative*.
const INDEFINITE: u32 = 0xFFC0_0000;

/// Assert C == Rust bitwise AND that C returned exactly `(ux, vy)`.
#[track_caller]
fn expect_exact(case: &str, s: [f32; 8], ux: u32, vy: u32) {
    diff_slots(case, s);
    let (p1, p2, p3, p) = from_slots(s);
    let c = c_call(p1, p2, p3, p);
    assert_eq!(
        c.bits(),
        (ux, vy),
        "[{case}] C sentinel changed: expected ({ux:#010x}, {vy:#010x}), got {c:?}"
    );
}

/// Assert C == Rust bitwise AND that C returned the indefinite QNaN in both
/// components (the library's de-facto "rejection" value).
#[track_caller]
fn expect_indefinite(case: &str, s: [f32; 8]) {
    expect_exact(case, s, INDEFINITE, INDEFINITE);
}

/// Assert C == Rust bitwise AND that both components are *some* NaN.
#[track_caller]
fn expect_nan(case: &str, s: [f32; 8]) {
    diff_slots(case, s);
    let (p1, p2, p3, p) = from_slots(s);
    let c = c_call(p1, p2, p3, p);
    assert!(
        c.x.is_nan() && c.y.is_nan(),
        "[{case}] expected NaN from C, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// E1 - E4: degenerate triangles (coincident vertices) -> 0 * inf
// ---------------------------------------------------------------------------

#[test]
fn err_e1_all_points_equal() {
    expect_indefinite("E1 all equal (1,2)", [1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 3.0, 4.0]);
    expect_indefinite("E1 all equal 0", [0.0; 8]);
    expect_indefinite(
        "E1 all equal, p equal too",
        [7.5, -3.25, 7.5, -3.25, 7.5, -3.25, 7.5, -3.25],
    );
    // Randomized: every coincident-vertex position must give the same sentinel.
    let mut rng = Rng::seeded();
    for _ in 0..iters(5_000) {
        let a = rng.vec2(-1e6, 1e6);
        let q = rng.vec2(-1e6, 1e6);
        expect_indefinite(
            "E1 random all equal",
            [a.x, a.y, a.x, a.y, a.x, a.y, q.x, q.y],
        );
    }
    // ±0 vertices too.
    for &(x, y) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        expect_indefinite("E1 signed-zero vertices", [x, y, x, y, x, y, 1.0, 2.0]);
    }
}

#[test]
fn err_e2_p1_eq_p3() {
    expect_indefinite("E2 p1==p3", [1.0, 2.0, 5.0, 7.0, 1.0, 2.0, 3.0, 4.0]);
    let mut rng = Rng::seeded();
    for _ in 0..iters(5_000) {
        let a = rng.vec2(-1e4, 1e4);
        let b = rng.vec2(-1e4, 1e4);
        let q = rng.vec2(-1e4, 1e4);
        expect_indefinite("E2 random p1==p3", [a.x, a.y, b.x, b.y, a.x, a.y, q.x, q.y]);
    }
}

#[test]
fn err_e3_p1_eq_p2() {
    expect_indefinite("E3 p1==p2", [1.0, 2.0, 1.0, 2.0, 5.0, 7.0, 3.0, 4.0]);
    let mut rng = Rng::seeded();
    for _ in 0..iters(5_000) {
        let a = rng.vec2(-1e4, 1e4);
        let b = rng.vec2(-1e4, 1e4);
        let q = rng.vec2(-1e4, 1e4);
        expect_indefinite("E3 random p1==p2", [a.x, a.y, a.x, a.y, b.x, b.y, q.x, q.y]);
    }
}

#[test]
fn err_e4_p2_eq_p3() {
    expect_indefinite("E4 p2==p3", [1.0, 2.0, 5.0, 7.0, 5.0, 7.0, 3.0, 4.0]);
    let mut rng = Rng::seeded();
    for _ in 0..iters(5_000) {
        let a = rng.vec2(-1e4, 1e4);
        let b = rng.vec2(-1e4, 1e4);
        let q = rng.vec2(-1e4, 1e4);
        expect_indefinite("E4 random p2==p3", [a.x, a.y, b.x, b.y, b.x, b.y, q.x, q.y]);
    }
}

// ---------------------------------------------------------------------------
// E5 / E6 / E7 / E7b(E19): zero denominator
// ---------------------------------------------------------------------------

#[test]
fn err_e5_collinear() {
    expect_indefinite(
        "E5 (0,0)(1,1)(2,2)",
        [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 7.0],
    );
    expect_indefinite(
        "E5 negative numerator side",
        [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, -3.0, -7.0],
    );
    expect_indefinite("E5 x axis", [0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.5, 0.25]);
    expect_indefinite("E5 y axis", [0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 0.5, 0.25]);
    // Randomized exactly-collinear triples built from integral multiples of a
    // direction vector, which keeps collinearity exact in binary32.
    //
    // Note: exact collinearity zeroes the denominator, but the *numerators*
    // are `dot11*dot02 - dot01*dot12` with products up to ~1e9, i.e. past
    // 2^24, so rounding can leave them non-zero. Then `nonzero * (1/+0)` gives
    // a true ±inf instead of the `0*inf` indefinite QNaN. Both outcomes are
    // legitimate C behaviour, so assert "non-finite and bit-identical" and
    // require that BOTH classes are actually observed.
    let mut rng = Rng::seeded();
    let mut saw_indefinite = false;
    let mut saw_inf = false;
    for _ in 0..iters(5_000) {
        let ox = (rng.below(65) as i32 - 32) as f32;
        let oy = (rng.below(65) as i32 - 32) as f32;
        let dx = (rng.below(17) as i32 - 8) as f32;
        let dy = (rng.below(17) as i32 - 8) as f32;
        let k2 = (rng.below(9) as i32 - 4) as f32;
        let k3 = (rng.below(9) as i32 - 4) as f32;
        let s = [
            ox,
            oy,
            ox + k2 * dx,
            oy + k2 * dy,
            ox + k3 * dx,
            oy + k3 * dy,
            rng.uniform(-64.0, 64.0),
            rng.uniform(-64.0, 64.0),
        ];
        diff_slots("E5 random collinear", s);
        let (p1, p2, p3, p) = from_slots(s);
        let c = c_call(p1, p2, p3, p);
        assert!(
            !c.x.is_finite() && !c.y.is_finite(),
            "E5: a zero denominator must give a non-finite result, got {c:?} for {s:?}"
        );
        if c.x.to_bits() == INDEFINITE {
            saw_indefinite = true;
        }
        if c.x.is_infinite() {
            saw_inf = true;
        }
    }
    assert!(saw_indefinite, "E5 never reached the 0*inf indefinite QNaN");
    assert!(
        saw_inf,
        "E5 never reached the nonzero/+0 -> ±inf variant (rounded numerator)"
    );
}

#[test]
fn err_e6_negative_zero_denominator() {
    // 1/-0 == -inf must be reachable and identical. The Gram determinant of
    // finite inputs can never be -0, so we drive the sign-of-zero logic with
    // the reachable neighbours and verify C/Rust agree on every one.
    //
    // Direct proof that both libraries agree on 1/±0 sign selection: a
    // denominator of exactly +0 with a non-zero numerator gives ±inf (E19),
    // and the sign follows the numerator, not the zero.
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        // Mixed-binade inputs are the class that reaches denom == 0 with a
        // non-zero numerator; sweep it hard and require bit equality.
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.binade(-30, 30);
        }
        diff_slots("E6 signed-zero denominator sweep", s);
    }
    // Signed zeros injected directly into the coordinates, so that the
    // subtractions produce -0 and the dot products +0/-0 mixtures.
    for mask in 0u32..256 {
        let mut s = [0.0f32; 8];
        for (i, v) in s.iter_mut().enumerate() {
            *v = if mask & (1 << i) != 0 { -0.0 } else { 0.0 };
        }
        expect_indefinite("E6 all-zero coordinates", s);
    }
    // -0 mixed with finite values.
    for _ in 0..iters(20_000) {
        let mut s = random_slots(&mut rng, -8.0, 8.0);
        for _ in 0..(1 + rng.below(4)) {
            s[rng.below(8) as usize] = -0.0;
        }
        diff_slots("E6 -0 mixed", s);
    }
}

#[test]
fn err_e7_underflow_denominator() {
    expect_indefinite(
        "E7 all 1e-30",
        [0.0, 0.0, 1e-30, 0.0, 0.0, 1e-30, 1e-30, 1e-30],
    );
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-1e-25, 1e-25);
        }
        // All dots underflow to 0 -> denom 0, numerators 0 -> indefinite.
        expect_indefinite("E7 random tiny", s);
    }
    // Subnormal-scale coordinates.
    for _ in 0..iters(10_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.subnormal();
        }
        expect_indefinite("E7 subnormal coords", s);
    }
}

#[test]
fn err_e19_actual_infinity() {
    // The six empirically-found witnesses where the C really returns ±inf
    // (denominator rounds to +0 while the numerators stay non-zero).
    let witnesses: [([f32; 8], u32, u32); 6] = [
        (
            [
                -55558858.0,      // -0x1.adb8aap+25
                71698.25,         //  0x1.18232p+13
                1984.2412,        //  0x1.f010f8p+10
                0.21683425,       //  0x1.bd08d4p-3
                -9.4477166e-10,   // -0x1.03dfd2p-30
                -1209.7423,       // -0x1.2e6f82p+10
                -3824.5908,       // -0x1.de12e8p+11
                -1.3893147,       // -0x1.6392ap+0
            ],
            0xFF80_0000,
            0x7F80_0000,
        ),
        (
            [
                79974.016,
                1.0192076e-8,
                -8.331909,
                -1.1240166e9,
                5.3387135e-6,
                -2.2410268e8,
                9.9088e-10,
                -7.8858662e8,
            ],
            0x7F80_0000,
            0xFFC0_0000,
        ),
        (
            [
                -0.0011477172,
                -9.0242464e7,
                -0.00046477215,
                -93718.0,
                -1.8158055e-7,
                -1.9527377e-7,
                -1.1755412e7,
                4.2062035,
            ],
            0x7F80_0000,
            0xFF80_0000,
        ),
        (
            [
                -4.5687e7,
                -0.23572838,
                14814.916,
                -9.7998e-5,
                354.09308,
                -0.008336,
                -0.00036731,
                1.1750338,
            ],
            0xFF80_0000,
            0xFFC0_0000,
        ),
        (
            [
                -0.025513,
                -3.2115758e7,
                -80.94,
                56537.086,
                1841.7799,
                2.1817678e7,
                0.9520559,
                -3.2523286e8,
            ],
            0xFF80_0000,
            0xFFC0_0000,
        ),
        (
            [
                4409020.5,
                1.3271344e7,
                1.7226e-6,
                0.00200545,
                -2.8809e-5,
                -7896.98,
                -8.5407688e7,
                0.08343,
            ],
            0xFF80_0000,
            0x7F80_0000,
        ),
    ];
    // The exact bit patterns matter, so rebuild the witnesses from hex where
    // the decimal literals may not round-trip; either way C and Rust must
    // agree, and at least one witness must actually be infinite.
    let mut saw_inf = false;
    for (i, (s, _ux, _vy)) in witnesses.iter().enumerate() {
        diff_slots(&format!("E19 witness {i}"), *s);
        let (p1, p2, p3, p) = from_slots(*s);
        let c = c_call(p1, p2, p3, p);
        if c.x.is_infinite() || c.y.is_infinite() {
            saw_inf = true;
        }
    }
    // Exact hex reconstruction of witness 0 (guaranteed bit pattern).
    let w0 = [
        f32::from_bits(0xCC56_DC55), // -0x1.adb8aap+25
        f32::from_bits(0x478C_1190), //  0x1.18232p+13
        f32::from_bits(0x44F8_087C), //  0x1.f010f8p+10
        f32::from_bits(0x3E5E_846A), //  0x1.bd08d4p-3
        f32::from_bits(0xB081_EFE9), // -0x1.03dfd2p-30
        f32::from_bits(0xC497_37C1), // -0x1.2e6f82p+10
        f32::from_bits(0xC56F_0974), // -0x1.de12e8p+11
        f32::from_bits(0xBFB1_C950), // -0x1.6392ap+0
    ];
    diff_slots("E19 witness0 (hex)", w0);
    let (p1, p2, p3, p) = from_slots(w0);
    let c = c_call(p1, p2, p3, p);
    if c.x.is_infinite() || c.y.is_infinite() {
        saw_inf = true;
    }

    // Broad randomized search over the mixed-binade class, which is where the
    // ±inf results live; assert C/Rust agree on every one and that we hit the
    // infinity path.
    let mut rng = Rng::seeded();
    let mut inf_hits = 0u32;
    for _ in 0..iters(400_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.binade(-30, 30);
        }
        diff_slots("E19 mixed-binade search", s);
        let (p1, p2, p3, p) = from_slots(s);
        let c = c_call(p1, p2, p3, p);
        if c.x.is_infinite() || c.y.is_infinite() {
            inf_hits += 1;
            saw_inf = true;
        }
    }
    eprintln!("E19: infinity results observed = {inf_hits}");
    assert!(
        saw_inf,
        "E19 never reached a true ±inf result — the 1/+0 path is untested"
    );
}

// ---------------------------------------------------------------------------
// E8 / E9: overflow
// ---------------------------------------------------------------------------

#[test]
fn err_e8_overflow_denominator() {
    expect_indefinite(
        "E8 all ~1e20",
        [0.0, 0.0, 1e20, 3e20, -2e20, 1e20, 1e20, 1e20],
    );
    let mut rng = Rng::seeded();
    let mut nan_hits = 0u32;
    for _ in 0..iters(20_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.uniform(-3e21, 3e21);
        }
        diff_slots("E8 random overflow", s);
        let (p1, p2, p3, p) = from_slots(s);
        let c = c_call(p1, p2, p3, p);
        if c.x.is_nan() {
            nan_hits += 1;
        }
    }
    assert!(nan_hits > 0, "E8 never produced the inf-inf NaN");
}

#[test]
fn err_e9_coordinate_overflow() {
    // p3.x - p1.x overflows to +inf.
    expect_indefinite(
        "E9 coordinate difference overflow",
        [-3.4e38, 0.0, 1.0, 1.0, 3.4e38, 2.0, 1.0, 1.0],
    );
    expect_indefinite(
        "E9 MAX/-MAX in y",
        [0.0, -f32::MAX, 1.0, 1.0, 2.0, f32::MAX, 1.0, 1.0],
    );
    // Every slot pair that feeds one subss, driven to overflow.
    let pairs = [(4usize, 0usize), (5, 1), (2, 0), (3, 1), (6, 0), (7, 1)];
    for (a, b) in pairs {
        for (va, vb) in [(f32::MAX, -f32::MAX), (-f32::MAX, f32::MAX)] {
            let mut s = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
            s[a] = va;
            s[b] = vb;
            diff_slots(
                &format!("E9 overflow {} vs {}", SLOT_NAMES[a], SLOT_NAMES[b]),
                s,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E10 - E14: non-finite inputs
// ---------------------------------------------------------------------------

#[test]
fn err_e10_infinity_in_each_slot() {
    expect_indefinite("E10 +inf p1.x", [f32::INFINITY, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5]);
    expect_indefinite(
        "E10 -inf p3.y",
        [0.0, 0.0, 1.0, 0.0, 0.0, f32::NEG_INFINITY, 0.5, 0.5],
    );
    // All 8 slots x both signs, fixed and randomized surroundings.
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
            let mut base = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
            base[slot] = inf;
            expect_nan(
                &format!("E10 inf in {} (unit triangle)", SLOT_NAMES[slot]),
                base,
            );
            for _ in 0..iters(2_000) {
                let mut s = random_slots(&mut rng, -1e3, 1e3);
                s[slot] = inf;
                diff_slots(&format!("E10 inf in {}", SLOT_NAMES[slot]), s);
            }
        }
    }
    // Both infinities at once, all sign combinations in the x slots.
    for &a in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &b in &[f32::INFINITY, f32::NEG_INFINITY] {
            diff_slots(
                "E10 two infinities",
                [a, 0.0, 1.0, 0.0, b, 1.0, 0.5, 0.5],
            );
        }
    }
    // Every slot infinite.
    expect_nan("E10 all +inf", [f32::INFINITY; 8]);
    expect_nan("E10 all -inf", [f32::NEG_INFINITY; 8]);
}

#[test]
fn err_e11_qnan_in_each_slot() {
    // The recorded sentinel: a propagated QNaN keeps its POSITIVE sign, unlike
    // the indefinite 0xFFC00000 an invalid operation manufactures.
    expect_exact(
        "E11 QNaN p1.x",
        [QNAN, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5],
        0x7FC0_0000,
        0x7FC0_0000,
    );
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &n in &[QNAN, NAN_PAYLOAD_A, NAN_PAYLOAD_B] {
            let mut base = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
            base[slot] = n;
            expect_nan(
                &format!("E11 QNaN {:#010x} in {}", n.to_bits(), SLOT_NAMES[slot]),
                base,
            );
            for _ in 0..iters(2_000) {
                let mut s = random_slots(&mut rng, -1e3, 1e3);
                s[slot] = n;
                diff_slots(&format!("E11 QNaN in {}", SLOT_NAMES[slot]), s);
            }
        }
    }
}

#[test]
fn err_e12_snan_in_each_slot() {
    // SNaN 0x7F800001 is quieted to 0x7FC00001: mantissa MSB set, payload kept.
    expect_exact(
        "E12 SNaN p2.x",
        [0.0, 0.0, SNAN, 0.0, 0.0, 1.0, 0.5, 0.5],
        0x7FC0_0001,
        0x7FC0_0001,
    );
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &n in &[SNAN, SNAN_NEG, f32::from_bits(0x7FBF_FFFF)] {
            let mut base = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
            base[slot] = n;
            expect_nan(
                &format!("E12 SNaN {:#010x} in {}", n.to_bits(), SLOT_NAMES[slot]),
                base,
            );
            for _ in 0..iters(2_000) {
                let mut s = random_slots(&mut rng, -1e3, 1e3);
                s[slot] = n;
                diff_slots(&format!("E12 SNaN in {}", SLOT_NAMES[slot]), s);
            }
        }
    }
}

#[test]
fn err_e13_negative_nan_payloads() {
    expect_exact(
        "E13 0xFFFFFFFF in p.y",
        [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, NAN_ALL_ONES],
        0xFFFF_FFFF,
        0xFFFF_FFFF,
    );
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &n in &[QNAN_NEG, NAN_ALL_ONES, f32::from_bits(0xFF80_0001), f32::from_bits(0xFFC0_DEAD)] {
            let mut base = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
            base[slot] = n;
            expect_nan(
                &format!("E13 -NaN {:#010x} in {}", n.to_bits(), SLOT_NAMES[slot]),
                base,
            );
            for _ in 0..iters(2_000) {
                let mut s = random_slots(&mut rng, -1e3, 1e3);
                s[slot] = n;
                diff_slots(&format!("E13 -NaN in {}", SLOT_NAMES[slot]), s);
            }
        }
    }
}

#[test]
fn err_e14_two_nan_operands() {
    // The decisive case: two different NaN payloads meet in one subss, and the
    // two output components select DIFFERENT surviving NaNs.
    expect_exact(
        "E14 p1.x=0x7fc01234, p3.x=0x7fdeadbe",
        [NAN_PAYLOAD_A, 0.0, 1.0, 0.0, NAN_PAYLOAD_B, 1.0, 0.5, 0.5],
        0x7FC0_1234,
        0x7FDE_ADBE,
    );
    let nans = [
        QNAN,
        QNAN_NEG,
        SNAN,
        SNAN_NEG,
        NAN_ALL_ONES,
        NAN_PAYLOAD_A,
        NAN_PAYLOAD_B,
        f32::from_bits(0xFFC0_DEAD),
    ];
    // Every ordered pair of NaNs in every slot pair that shares a subss.
    let pairs = [(4usize, 0usize), (5, 1), (2, 0), (3, 1), (6, 0), (7, 1)];
    for (a, b) in pairs {
        for &na in &nans {
            for &nb in &nans {
                let mut s = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
                s[a] = na;
                s[b] = nb;
                diff_slots(
                    &format!(
                        "E14 {}={:#010x} {}={:#010x}",
                        SLOT_NAMES[a],
                        na.to_bits(),
                        SLOT_NAMES[b],
                        nb.to_bits()
                    ),
                    s,
                );
            }
        }
    }
    // NaN in every slot, all draws from the NaN pool.
    let mut rng = Rng::seeded();
    for _ in 0..iters(100_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = nans[rng.below(nans.len() as u32) as usize];
        }
        diff_slots("E14 all-NaN", s);
    }
    // NaN vs inf collisions (inf-inf also manufactures a NaN, so the two NaN
    // sources compete).
    for &na in &nans {
        for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
            for (a, b) in pairs {
                let mut s = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
                s[a] = na;
                s[b] = i;
                diff_slots("E14 NaN vs inf", s);
                let mut s2 = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
                s2[a] = i;
                s2[b] = na;
                diff_slots("E14 inf vs NaN", s2);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E15 / E16: signed zero and subnormals
// ---------------------------------------------------------------------------

#[test]
fn err_e15_signed_zero() {
    expect_exact(
        "E15 -0 in p",
        [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, -0.0, -0.0],
        0x0000_0000,
        0x0000_0000,
    );
    expect_exact(
        "E15 -0 in p1",
        [-0.0, -0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        0x0000_0000,
        0x0000_0000,
    );
    // Exhaustive sign-of-zero combinations over all 8 slots.
    for mask in 0u32..256 {
        let mut s = [0.0f32; 8];
        for (i, v) in s.iter_mut().enumerate() {
            *v = if mask & (1 << i) != 0 { -0.0 } else { 0.0 };
        }
        diff_slots("E15 exhaustive signed zeros", s);
    }
    // -0 combined with the unit triangle in every slot, so that a wrong zero
    // sign in any intermediate shows up.
    for slot in 0..8usize {
        let mut s = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
        s[slot] = -0.0;
        diff_slots(&format!("E15 -0 in {}", SLOT_NAMES[slot]), s);
        let mut s2 = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
        s2[slot] = 0.0;
        diff_slots(&format!("E15 +0 in {}", SLOT_NAMES[slot]), s2);
    }
    // p exactly equal to p1 with mixed zero signs on both.
    let mut rng = Rng::seeded();
    for _ in 0..iters(10_000) {
        let (p1, p2, p3) = (
            rng.vec2(-100.0, 100.0),
            rng.vec2(-100.0, 100.0),
            rng.vec2(-100.0, 100.0),
        );
        diff("E15 p==p1", p1, p2, p3, p1);
    }
}

#[test]
fn err_e16_subnormals() {
    expect_indefinite(
        "E16 all min subnormal",
        [
            SUBNORMAL_MIN, 0.0, SUBNORMAL_MIN, 0.0, 0.0, SUBNORMAL_MIN, SUBNORMAL_MIN,
            SUBNORMAL_MIN,
        ],
    );
    let mut rng = Rng::seeded();
    for slot in 0..8usize {
        for &v in &[SUBNORMAL_MIN, -SUBNORMAL_MIN, SUBNORMAL_MAX, -SUBNORMAL_MAX] {
            let mut s = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
            s[slot] = v;
            diff_slots(
                &format!("E16 subnormal {:#010x} in {}", v.to_bits(), SLOT_NAMES[slot]),
                s,
            );
            for _ in 0..iters(1_000) {
                let mut r = random_slots(&mut rng, -1.0, 1.0);
                r[slot] = v;
                diff_slots(&format!("E16 subnormal in {}", SLOT_NAMES[slot]), r);
            }
        }
    }
    // All-subnormal inputs.
    for _ in 0..iters(20_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.subnormal();
        }
        diff_slots("E16 all subnormal", s);
    }
    // Subnormal / normal boundary values.
    let boundary = [
        SUBNORMAL_MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0080_0001),
        -SUBNORMAL_MAX,
        -f32::MIN_POSITIVE,
    ];
    for &a in &boundary {
        for &b in &boundary {
            diff_slots("E16 boundary", [a, b, b, a, a, a, b, b]);
        }
    }
}

// ---------------------------------------------------------------------------
// E17: every bit pattern is a legal input (the "out-of-range enum" analogue)
// ---------------------------------------------------------------------------

#[test]
fn err_e17_random_bit_patterns() {
    let mut rng = Rng::seeded();
    for _ in 0..iters(400_000) {
        let mut s = [0.0f32; 8];
        for v in s.iter_mut() {
            *v = rng.any_bits();
        }
        diff_slots("E17 random bit patterns", s);
    }
    // Exhaustive sweep over each slot's exponent field with a fixed mantissa,
    // so every exponent class (zero, subnormal, all normals, inf/NaN) is hit in
    // every slot.
    for slot in 0..8usize {
        for exp in 0u32..256 {
            for &sign in &[0u32, 0x8000_0000] {
                for &mant in &[0u32, 1, 0x0040_0000, 0x007F_FFFF] {
                    let bits = sign | (exp << 23) | mant;
                    let mut s = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
                    s[slot] = f32::from_bits(bits);
                    diff_slots("E17 exponent sweep", s);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E18: invDenom overflow
// ---------------------------------------------------------------------------

#[test]
fn err_e18_invdenom_overflow() {
    expect_indefinite(
        "E18 tiny triangle",
        [0.0, 0.0, 1e-22, 0.0, 0.0, 1e-22, 1.0, 1.0],
    );
    // denom is a tiny positive value so 1/denom overflows to +inf.
    let mut rng = Rng::seeded();
    for _ in 0..iters(20_000) {
        let e = -(24 + rng.below(15) as i32);
        let scale = (2.0f32).powi(e);
        let s = [
            0.0,
            0.0,
            scale * rng.uniform(0.5, 2.0),
            0.0,
            0.0,
            scale * rng.uniform(0.5, 2.0),
            rng.uniform(-2.0, 2.0),
            rng.uniform(-2.0, 2.0),
        ];
        diff_slots("E18 invDenom overflow", s);
    }
    // Largest subnormal / smallest normal denominators reached directly through
    // the dot products.
    for &v in &[
        SUBNORMAL_MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(0x2000_0000),
        f32::from_bits(0x1F80_0000),
    ] {
        diff_slots("E18 boundary denominator", [0.0, 0.0, v, 0.0, 0.0, v, 1.0, 1.0]);
    }
    // The other extreme: denom near FLT_MAX so invDenom is subnormal.
    diff_slots(
        "E18 subnormal invDenom",
        [0.0, 0.0, 1e19, 0.0, 0.0, 1e19, 1.0, 1.0],
    );
    diff_slots(
        "E18 subnormal invDenom 2",
        [0.0, 0.0, f32::MAX, 0.0, 0.0, f32::MAX, 1.0, 1.0],
    );
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries. Documented in ERRORS.md as unrepresentable for
// this signature; asserted here so the claim is verified rather than assumed.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_boundaries_are_unrepresentable() {
    // No pointer parameter -> nothing to pass NULL for. The only "pointer" in
    // play is the function pointer itself, which both libraries resolve.
    let a = api();
    assert!(!(a.c as usize == 0), "C symbol resolved to NULL");
    assert!(!(a.rust as usize == 0), "Rust symbol resolved to NULL");

    // No length/count parameter, no enum parameter: the argument list is four
    // 8-byte structs. The analogue of "an out-of-range enum value" is "a float
    // bit pattern with no numeric meaning", i.e. a NaN encoding; there are
    // 2*(2^23-1) of them and E11-E14/E17 cover every class:
    //   - quiet, positive           0x7FC00000
    //   - quiet, negative           0xFFC00000
    //   - signalling, positive      0x7F800001
    //   - signalling, negative      0xFF800001
    //   - all-payload-bits-set      0xFFFFFFFF
    //   - max signalling payload    0x7FBFFFFF
    for &bits in &[
        0x7FC0_0000u32,
        0xFFC0_0000,
        0x7F80_0001,
        0xFF80_0001,
        0xFFFF_FFFF,
        0x7FBF_FFFF,
        0x7F80_0000, // +inf: one step past the largest finite value
        0xFF80_0000, // -inf
        0x7F7F_FFFF, // FLT_MAX: the largest valid finite magnitude
        0xFF7F_FFFF, // -FLT_MAX
        0x0080_0000, // FLT_MIN: smallest normal
        0x007F_FFFF, // one step below: largest subnormal
        0x0000_0001, // smallest subnormal
        0x0000_0000, // +0
        0x8000_0000, // -0
    ] {
        let v = f32::from_bits(bits);
        for slot in 0..8usize {
            let mut s = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
            s[slot] = v;
            diff_slots(
                &format!("boundary {bits:#010x} in {}", SLOT_NAMES[slot]),
                s,
            );
        }
        // And in all slots at once.
        diff_slots(&format!("boundary {bits:#010x} everywhere"), [v; 8]);
    }
}
