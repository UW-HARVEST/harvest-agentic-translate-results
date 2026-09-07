//! Phase C — error/rejection-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md`.  Each
//! constructs the exact invalid input/condition, calls BOTH `.so`s, and asserts
//! the SAME sentinel value comes back — not merely "both failed".

mod harness;

use harness::*;

use std::os::raw::{c_uint, c_void};
use std::ptr;

/// Enum values with no valid `C2_TYPE` variant.  C enums accept any `int`, so
/// these are real inputs the C handles (its `switch` falls to `default:`).
const BAD_ENUMS: &[c_uint] = &[
    2,
    3,
    7,
    255,
    256,
    0x7FFF_FFFF,            // INT_MAX
    0x8000_0000,            // INT_MIN reinterpreted
    0xFFFF_FFFF,            // -1 reinterpreted  (row B1)
    0xFFFF_FFFE,
    0x0000_0100,
];

fn a_circle() -> C2Circle {
    C2Circle {
        p: C2v { x: 1.5, y: -2.5 },
        r: 3.0,
    }
}
fn an_aabb() -> C2Aabb {
    C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    }
}

// ============================================================ E1, E2, E3, B1
#[test]
fn e1_e2_e3_b1_f2_out_of_range_enums_return_zero() {
    let p = load();
    let c = a_circle();
    let b = an_aabb();
    let pc = &c as *const C2Circle as *const c_void;
    let pb = &b as *const C2Aabb as *const c_void;

    // E1: typeA == CIRCLE, typeB invalid -> inner `default:`.
    for &bad in BAD_ENUMS {
        let (rc, rr) = unsafe { ((p.c.f2)(pc, C2_TYPE_CIRCLE, pb, bad), (p.r.f2)(pc, C2_TYPE_CIRCLE, pb, bad)) };
        assert_eq!(rc, rr, "E1 f2(CIRCLE, {bad:#x}) diverged: C={rc} Rust={rr}");
        assert_eq!(rc, 0, "E1 sentinel: C should return 0 for typeB={bad:#x}");
    }

    // E2: typeA == AABB, typeB invalid -> inner `default:`.
    for &bad in BAD_ENUMS {
        let (rc, rr) = unsafe { ((p.c.f2)(pb, C2_TYPE_AABB, pc, bad), (p.r.f2)(pb, C2_TYPE_AABB, pc, bad)) };
        assert_eq!(rc, rr, "E2 f2(AABB, {bad:#x}) diverged: C={rc} Rust={rr}");
        assert_eq!(rc, 0, "E2 sentinel: C should return 0 for typeB={bad:#x}");
    }

    // E3/B1: typeA invalid -> outer `default:`, for every typeB incl. valid.
    for &bad in BAD_ENUMS {
        for &tb in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, 2u32, 0xFFFF_FFFFu32] {
            let (rc, rr) = unsafe { ((p.c.f2)(pc, bad, pb, tb), (p.r.f2)(pc, bad, pb, tb)) };
            assert_eq!(rc, rr, "E3 f2({bad:#x}, {tb:#x}) diverged: C={rc} Rust={rr}");
            assert_eq!(rc, 0, "E3 sentinel: C should return 0 for typeA={bad:#x}");
        }
    }

    // The C never dereferences either pointer on a `default:` path, so NULL is
    // a legitimate input there.  Verify both libraries agree (and neither
    // faults) with null pointers on every rejecting combination.
    let nul: *const c_void = ptr::null();
    for &bad in BAD_ENUMS {
        let (rc, rr) = unsafe { ((p.c.f2)(nul, bad, nul, bad), (p.r.f2)(nul, bad, nul, bad)) };
        assert_eq!(rc, rr, "E3 f2(NULL, {bad:#x}, NULL, {bad:#x}) diverged");
        assert_eq!(rc, 0);
        let (rc, rr) = unsafe {
            (
                (p.c.f2)(nul, C2_TYPE_CIRCLE, nul, bad),
                (p.r.f2)(nul, C2_TYPE_CIRCLE, nul, bad),
            )
        };
        assert_eq!(rc, rr, "E1 f2(NULL, CIRCLE, NULL, {bad:#x}) diverged");
        assert_eq!(rc, 0);
        let (rc, rr) = unsafe {
            (
                (p.c.f2)(nul, C2_TYPE_AABB, nul, bad),
                (p.r.f2)(nul, C2_TYPE_AABB, nul, bad),
            )
        };
        assert_eq!(rc, rr, "E2 f2(NULL, AABB, NULL, {bad:#x}) diverged");
        assert_eq!(rc, 0);
    }

    // Exhaustive over the low enum range so nothing between 0 and 4096 is
    // treated specially by either side.
    for t in 0u32..4096 {
        for &other in &[C2_TYPE_CIRCLE, C2_TYPE_AABB] {
            let (rc, rr) = unsafe { ((p.c.f2)(pc, other, pb, t), (p.r.f2)(pc, other, pb, t)) };
            assert_eq!(rc, rr, "f2({other}, {t}) diverged");
            let (rc, rr) = unsafe { ((p.c.f2)(pb, t, pc, other), (p.r.f2)(pb, t, pc, other)) };
            assert_eq!(rc, rr, "f2({t}, {other}) diverged");
        }
    }
}

// ==================================================================== E4..E9
#[test]
fn e4_f3_divide_by_zero_guard_returns_zero() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 104);
    let mut v1s: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..20_000 {
        v1s.push(rng.next_i32());
    }
    for v1 in v1s {
        let (c, r) = unsafe { ((p.c.f3)(v1, 0), (p.r.f3)(v1, 0)) };
        assert_eq!(c, r, "E4 f3({v1}, 0) diverged: C={c} Rust={r}");
        assert_eq!(c, 0, "E4 sentinel: f3({v1}, 0) should be 0, C gave {c}");
    }
}

#[test]
fn e5_e9_f3_int_min_overflow_arms() {
    let p = load();
    let chk = |v1: i32, v2: i32, row: &str| {
        let (c, r) = unsafe { ((p.c.f3)(v1, v2), (p.r.f3)(v1, v2)) };
        assert_eq!(c, r, "{row} f3({v1}, {v2}) diverged: C={c} Rust={r}");
        c
    };

    // E5: the single signed-division overflow pair.
    let got = chk(i32::MIN, -1, "E5");
    assert_eq!(got, i32::MIN, "E5 sentinel: C returns INT_MIN, got {got}");

    // E6: v1 == v2 == INT_MIN -> the final `else` (q = 1, r = 0).
    let got = chk(i32::MIN, i32::MIN, "E6");
    assert_eq!(got, 1, "E6 sentinel: C returns 1, got {got}");

    // E7: v1 == INT_MIN, v2 > 0 (incl. v2 == 1 where -(v1+v2) itself overflows).
    let got = chk(i32::MIN, 1, "E7");
    assert_eq!(got, i32::MIN, "E7 sentinel: f3(INT_MIN, 1) == INT_MIN, got {got}");
    for v2 in 1i32..=512 {
        chk(i32::MIN, v2, "E7");
    }
    for &v2 in &[i32::MAX, i32::MAX - 1, 1 << 30, 1 << 20] {
        chk(i32::MIN, v2, "E7");
    }

    // E8: v1 >= 0, v2 == INT_MIN -> q = 0, r = v1.
    for &v1 in &[0i32, 1, 2, 1000, i32::MAX, i32::MAX - 1, 1 << 30] {
        let got = chk(v1, i32::MIN, "E8");
        assert_eq!(got, 0, "E8 sentinel: f3({v1}, INT_MIN) == 0, got {got}");
    }

    // E9: v1 < 0 (not INT_MIN), v2 == INT_MIN -> q = 1, r = v1 - q*v2 (wraps).
    for &v1 in &[-1i32, -2, -1000, i32::MIN + 1, -(1 << 30)] {
        let got = chk(v1, i32::MIN, "E9");
        assert!(
            got == 1 || got == 2,
            "E9 sentinel: f3({v1}, INT_MIN) should be 1 or 2, got {got}"
        );
    }

    // Every INT_MIN-adjacent combination, both argument positions.
    let edge = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &edge {
        for &b in &edge {
            chk(a, b, "E5-E9 edge");
        }
    }
}

// ==================================================================== E10/E11
#[test]
fn e10_e11_f11_degenerate_and_out_of_range_hue() {
    let p = load();
    let chk = |src: [f32; 3], row: &str| -> [f32; 3] {
        let c = p.c.call_triple(Triple::F11, src);
        let r = p.r.call_triple(Triple::F11, src);
        assert_eq!(
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
            [r[0].to_bits(), r[1].to_bits(), r[2].to_bits()],
            "{row} f11({src:?}) diverged: C={c:?} Rust={r:?}"
        );
        c
    };

    // E10: s == 0 (and -0.0) -> all three outputs = l, regardless of h.
    for &s in &[0.0f32, -0.0f32] {
        for &h in &[
            0.0f32,
            -1.0,
            30.0,
            90.0,
            150.0,
            210.0,
            270.0,
            330.0,
            360.0,
            1e30,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ] {
            for &l in &[0.0f32, -0.0, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN, 1e-45] {
                let out = chk([h, s, l], "E10");
                assert_eq!(
                    [out[0].to_bits(), out[1].to_bits(), out[2].to_bits()],
                    [l.to_bits(); 3],
                    "E10 sentinel: s==0 must copy l to all three outputs"
                );
            }
        }
    }

    // E11: h NaN / outside every sector -> the final `else` (all outputs = m).
    for &h in &[
        -1e-45f32,
        -1.0,
        -360.0,
        360.0,
        360.000_03,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FFF_FFFF),
    ] {
        for &s in &[0.5f32, 1.0, -1.0, 2.0, f32::INFINITY, f32::NAN, 1e-45] {
            for &l in &[0.0f32, 0.25, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                chk([h, s, l], "E11");
            }
        }
    }
    // The 120..180 range is unreachable-as-written (`h < 120 && h < 180`), so
    // it must also land in the final `else`.  Verified against the C, not the
    // intent.
    for i in 0..600 {
        let h = 120.0f32 + i as f32 * 0.1;
        chk([h, 0.5, 0.5], "E11/unreachable-arm");
    }
}

// ==================================================================== E12/E13
#[test]
fn e12_e13_f12_degenerate_and_switch_default() {
    let p = load();
    let chk = |src: [f32; 3], row: &str| -> [f32; 3] {
        let c = p.c.call_triple(Triple::F12, src);
        let r = p.r.call_triple(Triple::F12, src);
        assert_eq!(
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
            [r[0].to_bits(), r[1].to_bits(), r[2].to_bits()],
            "{row} f12({src:?}) diverged: C={c:?} Rust={r:?}"
        );
        c
    };

    // E12: s == 0 -> all three outputs = v.
    for &s in &[0.0f32, -0.0f32] {
        for &h in &[0.0f32, 30.0, 359.0, -5.0, 1e30, f32::INFINITY, f32::NAN] {
            for &v in &[0.0f32, -0.0, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN, 1e-45] {
                let out = chk([h, s, v], "E12");
                assert_eq!(
                    [out[0].to_bits(), out[1].to_bits(), out[2].to_bits()],
                    [v.to_bits(); 3],
                    "E12 sentinel: s==0 must copy v to all three outputs"
                );
            }
        }
    }

    // E13: (int)floorf(h/60) outside 0..=4 -> the `default:` arm.
    // Includes NaN, where the x86 `cvttss2si` yields INT_MIN.
    for &h in &[
        300.0f32,
        301.0,
        359.9,
        360.0,
        1e10,
        1e30,
        f32::MAX,
        -0.000_001,
        -1.0,
        -60.0,
        -1e30,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FFF_FFFF),
        // exactly at / past the cvttss2si range boundary
        1.288_490_2e11,
        -1.288_490_2e11,
    ] {
        for &s in &[0.5f32, 1.0, -1.0, 2.0, f32::INFINITY, f32::NAN, 1e-45] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                chk([h, s, v], "E13");
            }
        }
    }
}

// ============================================================== E14, E15, E16
#[test]
fn e14_e16_f13_degenerate_paths() {
    let p = load();
    let chk = |src: [f32; 3], row: &str| -> [f32; 3] {
        let c = p.c.call_triple(Triple::F13, src);
        let r = p.r.call_triple(Triple::F13, src);
        assert_eq!(
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
            [r[0].to_bits(), r[1].to_bits(), r[2].to_bits()],
            "{row} f13({src:?}) diverged: C={c:?} Rust={r:?}"
        );
        c
    };

    // E14: delta == 0 (all three channels equal) -> writes 0, 0, max.
    // For the infinities `delta` is `inf - inf == NaN`, so the C does NOT take
    // the early return there; only the finite cases carry the 0/0/max sentinel.
    for &x in &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1e-45,
        1.175_494_4e-38,
        3.402_823_5e38,
    ] {
        let out = chk([x, x, x], "E14");
        assert_eq!(out[0].to_bits(), 0u32, "E14 sentinel: h must be +0.0");
        assert_eq!(out[1].to_bits(), 0u32, "E14 sentinel: s must be +0.0");
        assert_eq!(out[2].to_bits(), x.to_bits(), "E14 sentinel: v must be max");
    }
    for &x in &[f32::INFINITY, f32::NEG_INFINITY] {
        chk([x, x, x], "E14/infinite-delta");
    }
    // Signed-zero mixtures: bitwise-different but numerically equal.
    for &a in &[0.0f32, -0.0] {
        for &b in &[0.0f32, -0.0] {
            for &c in &[0.0f32, -0.0] {
                chk([a, b, c], "E14/signed-zero");
            }
        }
    }

    // E15: max == 0 with delta != 0 (requires a negative channel).
    for &n in &[-1.0f32, -1e-45, -3.402_823_5e38, f32::NEG_INFINITY] {
        for perm in 0..3 {
            let mut s = [0.0f32; 3];
            s[perm] = n;
            let out = chk(s, "E15");
            assert_eq!(out[0].to_bits(), 0u32, "E15 sentinel: h must be +0.0");
            assert_eq!(out[1].to_bits(), 0u32, "E15 sentinel: s must be +0.0");
        }
        // two negatives, one zero
        for zero_at in 0..3 {
            let mut s = [n; 3];
            s[zero_at] = 0.0;
            chk(s, "E15/two-negative");
            let mut s = [n; 3];
            s[zero_at] = -0.0;
            chk(s, "E15/two-negative-nz");
        }
    }

    // E16: NaN channels -> every comparison is false, delta is NaN, and the
    // C falls through to `h = 4 + (r-g)/delta`.
    let nans: Vec<f32> = NAN_BITS.iter().map(|&b| f32::from_bits(b)).collect();
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                chk([a, b, c], "E16/all-nan");
            }
        }
    }
    for &n in &nans {
        for slot in 0..3 {
            for &other in &[0.0f32, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
                let mut s = [other; 3];
                s[slot] = n;
                chk(s, "E16/one-nan");
            }
        }
    }
}

// ======================================================================= E17
#[test]
fn e17_f4_degenerate_zero_state_is_a_fixed_point() {
    let p = load();
    let mut sc = CnRnd { state: [0, 0] };
    let mut sr = CnRnd { state: [0, 0] };
    for i in 0..64 {
        let (c, r) = unsafe { ((p.c.f4)(&mut sc), (p.r.f4)(&mut sr)) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "E17 f4 diverged at step {i}: C={c:?} Rust={r:?}"
        );
        assert_eq!(
            c.to_bits(),
            0u64,
            "E17 sentinel: the zero state must yield exactly 0.0, C gave {c:?}"
        );
        assert_eq!(sc, sr, "E17 state diverged at step {i}");
        assert_eq!(sc.state, [0, 0], "E17 sentinel: state must stay {{0,0}}");
    }
}

// ==================================================================== E18/E19
#[test]
fn e18_e19_f7_overflow_and_zero_channels() {
    let p = load();
    let chk = |bs: u32, ch: u32, bd: u32, row: &str| -> u32 {
        let (c, r) = unsafe { ((p.c.f7)(bs, ch, bd), (p.r.f7)(bs, ch, bd)) };
        assert_eq!(
            c, r,
            "{row} f7({bs}, {ch}, {bd}) diverged: C={c:#010x} Rust={r:#010x}"
        );
        c
    };

    // E18: wrapping uint32 overflow.
    chk(0xFFFF_FFFF, 3, 0xFFFF_FFFF, "E18");
    chk(0xFFFF_FFFF, 2, 0xFFFF_FFFF, "E18");
    chk(0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, "E18");
    chk(0x8000_0000, 2, 32, "E18");
    chk(0x0FFF_FFFF, 8, 24, "E18");
    let mut rng = Rng::new(SEED ^ 118);
    for _ in 0..100_000 {
        // Bias toward huge values so the products wrap.
        chk(
            rng.next_u32() | 0x8000_0000,
            rng.pick(&[0u32, 1, 2, 3, 0xFFFF_FFFF]),
            rng.next_u32() | 0x8000_0000,
            "E18",
        );
    }

    // E19: channels == 0 -> every product vanishes -> 18.
    for &bd in &[0u32, 1, 16, 32, 33, 0xFFFF_FFFF] {
        for &bs in &[0u32, 1, 4096, 0xFFFF_FFFF] {
            let got = chk(bs, 0, bd, "E19");
            assert_eq!(
                got, 18,
                "E19 sentinel: f7({bs}, 0, {bd}) must be 18, C gave {got}"
            );
        }
    }
}

// ==================================================================== E20/E21
#[test]
fn e20_e21_f9_degenerate_and_special_inputs() {
    let p = load();
    let chk = |p1: LmVec2, p2: LmVec2, p3: LmVec2, q: LmVec2, row: &str| -> LmVec2 {
        let (c, r) = unsafe { ((p.c.f9)(p1, p2, p3, q), (p.r.f9)(p1, p2, p3, q)) };
        assert_eq!(
            (c.x.to_bits(), c.y.to_bits()),
            (r.x.to_bits(), r.y.to_bits()),
            "{row} f9({p1:?},{p2:?},{p3:?},{q:?}) diverged: C={c:?} Rust={r:?}"
        );
        c
    };
    let v = |x: f32, y: f32| LmVec2 { x, y };

    // E20: denominator exactly zero -> invDenom = +/-inf -> NaN result.
    let a = v(1.0, 2.0);
    for &q in &[v(0.0, 0.0), v(1.0, 2.0), v(-5.0, 7.0)] {
        let out = chk(a, a, a, q, "E20");
        assert!(
            out.x.is_nan() && out.y.is_nan(),
            "E20 sentinel: coincident vertices must give NaN, C gave {out:?}"
        );
    }
    // Zero-length v0 / v1 only.
    chk(a, a, v(3.0, 4.0), v(0.5, 0.5), "E20/v1==0");
    chk(a, v(3.0, 4.0), a, v(0.5, 0.5), "E20/v0==0");
    // Collinear.
    chk(v(0.0, 0.0), v(1.0, 1.0), v(2.0, 2.0), v(0.5, 0.5), "E20/collinear");
    chk(v(0.0, 0.0), v(1.0, 0.0), v(2.0, 0.0), v(0.5, 0.5), "E20/collinear-x");

    // E21: NaN / infinite components in every slot.
    let specials: Vec<f32> = {
        let mut s = vec![f32::INFINITY, f32::NEG_INFINITY];
        s.extend(NAN_BITS.iter().map(|&b| f32::from_bits(b)));
        s
    };
    for &s in &specials {
        for slot in 0..8 {
            let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
            c[slot] = s;
            chk(
                v(c[0], c[1]),
                v(c[2], c[3]),
                v(c[4], c[5]),
                v(c[6], c[7]),
                "E21",
            );
        }
    }
    for &s1 in &specials {
        for &s2 in &specials {
            for slot in 0..8 {
                let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
                c[slot] = s1;
                c[(slot + 4) % 8] = s2;
                chk(
                    v(c[0], c[1]),
                    v(c[2], c[3]),
                    v(c[4], c[5]),
                    v(c[6], c[7]),
                    "E21/pair",
                );
            }
        }
    }
}

// ======================================================================= E22
#[test]
fn e22_f5_discards_the_high_half() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 122);
    let mut inputs: Vec<u32> = vec![0xFFFF_0000, 0x8000_0000, 0xDEAD_0000];
    for b in 16..32 {
        inputs.push(1u32 << b);
    }
    for _ in 0..50_000 {
        inputs.push(rng.next_u32() & 0xFFFF_0000);
    }
    for a in inputs {
        let (c, r) = unsafe { ((p.c.f5)(a), (p.r.f5)(a)) };
        assert_eq!(c, r, "E22 f5({a:#010x}) diverged: C={c:#010x} Rust={r:#010x}");
        assert!(
            c <= 0xFFFF,
            "E22 sentinel: the high half must be discarded, C gave {c:#010x}"
        );
    }
    // High-half noise must not change the result for the same low half.
    for lo in 0u32..=0xFFFF {
        let base = unsafe { (p.c.f5)(lo) };
        for &hi in &[0x0001_0000u32, 0xFFFF_0000, 0x8000_0000] {
            let (c, r) = unsafe { ((p.c.f5)(lo | hi), (p.r.f5)(lo | hi)) };
            assert_eq!(c, r, "E22 f5({:#010x}) diverged", lo | hi);
            assert_eq!(c, base, "E22 sentinel: high half affected the result");
        }
    }
}

// =================================================================== E23 / B5
#[test]
fn e23_b5_f10_full_uint16_domain_and_exponent_extremes() {
    let p = load();
    // B5: exhaustive over the complete domain -- proves the unchecked
    // `m__mantissa[(h & 0x3ff) + m__offset[h >> 10]]` index is always in range.
    for h in 0u16..=u16::MAX {
        let (c, r) = unsafe { ((p.c.f10)(h), (p.r.f10)(h)) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "B5 f10({h:#06x}) diverged: C={:#010x} Rust={:#010x}",
            c.to_bits(),
            r.to_bits()
        );
    }
    // E23: the two index extremes and the special exponent rows n == 31 / 63,
    // which map to the 0x47800000 / 0xc7800000 inf-encoding entries.
    for &h in &[0u16, 0xFFFF, 0x03FF, 0x0400, 0x7C00, 0xFC00, 0x7FFF, 0xFBFF] {
        let (c, r) = unsafe { ((p.c.f10)(h), (p.r.f10)(h)) };
        assert_eq!(c.to_bits(), r.to_bits(), "E23 f10({h:#06x}) diverged");
    }
    for n in 0u16..64 {
        for &m in &[0u16, 1, 0x1FF, 0x3FE, 0x3FF] {
            let h = (n << 10) | m;
            let (c, r) = unsafe { ((p.c.f10)(h), (p.r.f10)(h)) };
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "E23 f10({h:#06x}) (n={n}) diverged"
            );
        }
    }
}

// ======================================================================== B2
#[test]
fn b2_f3_unit_divisors_against_every_extreme() {
    let p = load();
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -(1 << 30),
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        1000,
        1 << 30,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v1 in &extremes {
        for &v2 in &[1i32, -1] {
            let (c, r) = unsafe { ((p.c.f3)(v1, v2), (p.r.f3)(v1, v2)) };
            assert_eq!(c, r, "B2 f3({v1}, {v2}) diverged: C={c} Rust={r}");
        }
    }
}

// ======================================================================== B3
#[test]
fn b3_f7_every_branch_boundary() {
    let p = load();
    let vals = [0u32, 1, 2, 3, 31, 32, 33, 0xFFFF_FFFF];
    for &bs in &vals {
        for &ch in &vals {
            for &bd in &vals {
                let (c, r) = unsafe { ((p.c.f7)(bs, ch, bd), (p.r.f7)(bs, ch, bd)) };
                assert_eq!(
                    c, r,
                    "B3 f7({bs}, {ch}, {bd}) diverged: C={c:#010x} Rust={r:#010x}"
                );
            }
        }
    }
}

// ======================================================================== B4
// Null `dest`/`src` for f11/f12/f13 is UB in the C (it dereferences both
// unconditionally, with no null check -- see ERRORS.md row B4).  There is no
// rejection to compare, so this is deliberately not exercised.

// ================================================== generic boundary coverage
#[test]
fn generic_boundaries_zero_and_oversized_and_one_past_range() {
    let p = load();

    // f10: `h` is a uint16_t, so there is no "one past the range" -- the whole
    // domain is valid (covered exhaustively above).  But the *promoted* int
    // that the C shifts must not sign-extend: pass 0xFFFF and check the top
    // exponent row is used.
    let (c, r) = unsafe { ((p.c.f10)(0xFFFF), (p.r.f10)(0xFFFF)) };
    assert_eq!(c.to_bits(), r.to_bits(), "f10(0xFFFF) diverged");

    // f2: pointers to *exactly*-sized objects at both alignments, so neither
    // library reads past the end of a 12-byte c2Circle when told it is one.
    let circle = a_circle();
    let aabb = an_aabb();
    unsafe {
        let a = (p.c.f2)(
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        let b = (p.r.f2)(
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        assert_eq!(a, b);
        let a = (p.c.f2)(
            &aabb as *const _ as *const c_void,
            C2_TYPE_AABB,
            &aabb as *const _ as *const c_void,
            C2_TYPE_AABB,
        );
        let b = (p.r.f2)(
            &aabb as *const _ as *const c_void,
            C2_TYPE_AABB,
            &aabb as *const _ as *const c_void,
            C2_TYPE_AABB,
        );
        assert_eq!(a, b);
    }

    // f4: the struct is mutated through the pointer; make sure an aliased call
    // sequence (same pointer twice) matches.
    let mut sc = CnRnd { state: [7, 11] };
    let mut sr = CnRnd { state: [7, 11] };
    unsafe {
        let a1 = (p.c.f4)(&mut sc);
        let a2 = (p.c.f4)(&mut sc);
        let b1 = (p.r.f4)(&mut sr);
        let b2 = (p.r.f4)(&mut sr);
        assert_eq!(a1.to_bits(), b1.to_bits(), "f4 aliased call 1 diverged");
        assert_eq!(a2.to_bits(), b2.to_bits(), "f4 aliased call 2 diverged");
        assert_eq!(sc, sr);
    }

    // f11/f12/f13: `dest` aliasing `src` (the C reads all three inputs into
    // locals first, so an in-place call is well-defined).
    for which in [Triple::F11, Triple::F12, Triple::F13] {
        for src in [
            [30.0f32, 0.5, 0.5],
            [0.0, 0.0, 0.0],
            [f32::NAN, f32::NAN, f32::NAN],
            [400.0, 2.0, -1.0],
        ] {
            let mut bc = src;
            let mut br = src;
            unsafe {
                let f_c = match which {
                    Triple::F11 => p.c.f11,
                    Triple::F12 => p.c.f12,
                    Triple::F13 => p.c.f13,
                };
                let f_r = match which {
                    Triple::F11 => p.r.f11,
                    Triple::F12 => p.r.f12,
                    Triple::F13 => p.r.f13,
                };
                f_c(bc.as_mut_ptr(), bc.as_ptr());
                f_r(br.as_mut_ptr(), br.as_ptr());
            }
            assert_eq!(
                [bc[0].to_bits(), bc[1].to_bits(), bc[2].to_bits()],
                [br[0].to_bits(), br[1].to_bits(), br[2].to_bits()],
                "{} in-place (dest == src) diverged for {src:?}",
                which.name()
            );
        }
    }
}
