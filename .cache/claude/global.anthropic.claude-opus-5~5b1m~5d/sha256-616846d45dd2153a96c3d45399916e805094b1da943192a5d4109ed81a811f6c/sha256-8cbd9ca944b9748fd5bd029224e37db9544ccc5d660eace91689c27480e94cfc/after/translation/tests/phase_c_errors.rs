//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition from the table, calls
//! BOTH `.so`s, and asserts they return the SAME value (not merely "both
//! failed"). Sentinel expectations from the C source are asserted too, so a
//! test that stopped exercising its branch would fail loudly.

mod common;
use common::*;
use std::ffi::c_void;

const QNAN: f32 = f32::from_bits(0x7fc0_0000);
const SNAN: f32 = f32::from_bits(0x7f80_0001);

fn put<T: Copy>(buf: &mut [u8], off: usize, v: T) -> *const c_void {
    unsafe {
        let p = buf.as_mut_ptr().add(off);
        std::ptr::copy_nonoverlapping((&raw const v).cast::<u8>(), p, size_of::<T>());
        p.cast()
    }
}

// ===========================================================================
// Row 1 — c2Collided `default:` (out-of-range enum value across FFI)
// ===========================================================================

/// Every `int` with no valid `C2_TYPE` variant. C dispatches with
/// `cmp 2 / ja default`, i.e. an UNSIGNED compare, so all negatives also land
/// in `default` and must return 0.
const BAD_TYPES: &[i32] = &[
    3,
    4,
    5,
    100,
    255,
    256,
    0xffff,
    0x1_0000,
    i32::MAX,
    i32::MAX - 1,
    -1,
    -2,
    -3,
    -100,
    -256,
    i32::MIN,
    i32::MIN + 1,
    0x7fff_fffe,
    -0x7fff_ffff,
];

#[test]
fn err01_collided_out_of_range_enum() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(101);
    let mut case = Case::new("E01 c2Collided default: out-of-range enum");
    let mut ba = [0u8; 64];
    let mut bb = [0u8; 64];
    for &t in BAD_TYPES {
        for _ in 0..200 {
            let a = C2Circle {
                p: g.v_spicy(),
                r: g.spicy(),
            };
            let b = C2Capsule {
                a: g.v_spicy(),
                b: g.v_spicy(),
                r: g.spicy(),
            };
            let pa = put(&mut ba, 0, a);
            let pb = put(&mut bb, 0, b);
            let cv = unsafe { (c.c2Collided)(pa, pb, t) };
            let rv = unsafe { (r.c2Collided)(pa, pb, t) };
            case.eq(cv, rv, || format!("typeB={t}"));
            // The C source's `default:` returns 0 — assert the sentinel itself.
            case.eq(cv, 0, || format!("C default: must return 0, typeB={t}"));
        }
    }
    // And the one step past each end of the valid range, plus the boundary
    // values that ARE valid (0,1,2) for contrast.
    for t in -4i32..=6 {
        let pa = put(&mut ba, 0, C2Circle::default());
        let pb = put(&mut bb, 0, C2Capsule::default());
        let cv = unsafe { (c.c2Collided)(pa, pb, t) };
        let rv = unsafe { (r.c2Collided)(pa, pb, t) };
        case.eq(cv, rv, || format!("typeB={t} (boundary sweep)"));
        if !(0..=2).contains(&t) {
            case.eq(cv, 0, || format!("typeB={t} must be rejected"));
        }
    }
    case.finish();
}

// ===========================================================================
// Row 2 — null pointers with an out-of-range enum (never dereferenced)
// ===========================================================================

#[test]
fn err02_collided_null_pointers_bad_type() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E02 c2Collided NULL + bad typeB");
    let nul: *const c_void = std::ptr::null();
    let mut buf = [0u8; 64];
    let good = put(&mut buf, 0, C2Circle::default());
    for &t in BAD_TYPES {
        for (pa, pb, what) in [
            (nul, nul, "A=NULL B=NULL"),
            (nul, good, "A=NULL"),
            (good, nul, "B=NULL"),
        ] {
            let cv = unsafe { (c.c2Collided)(pa, pb, t) };
            let rv = unsafe { (r.c2Collided)(pa, pb, t) };
            case.eq(cv, rv, || format!("{what} typeB={t}"));
            case.eq(cv, 0, || format!("{what} typeB={t} must return 0"));
        }
    }
    case.finish();
}

// ===========================================================================
// Row 3 — correctly sized but UNALIGNED B (the testable half of the row)
// ===========================================================================

#[test]
fn err03_collided_unaligned_shapes() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(103);
    let mut case = Case::new("E03 c2Collided unaligned A and B");
    for _ in 0..6_000 {
        let mut ba = [0u8; 64];
        let mut bb = [0u8; 64];
        for i in 0..64 {
            ba[i] = g.next_u32() as u8;
            bb[i] = g.next_u32() as u8;
        }
        for off_a in [1usize, 2, 3, 5, 6, 7, 9, 11, 13] {
            for off_b in [1usize, 2, 3, 5, 6, 7, 9, 11, 13] {
                let pa: *const c_void = unsafe { ba.as_ptr().add(off_a).cast() };
                let pb: *const c_void = unsafe { bb.as_ptr().add(off_b).cast() };
                for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                    let cv = unsafe { (c.c2Collided)(pa, pb, t) };
                    let rv = unsafe { (r.c2Collided)(pa, pb, t) };
                    case.eq(cv, rv, || {
                        format!("typeB={t} off_a={off_a} off_b={off_b}")
                    });
                }
            }
        }
        if case.n > 200_000 {
            break;
        }
    }
    case.finish();
}

// ===========================================================================
// Row 4 — A and B aliasing the same object
// ===========================================================================

#[test]
fn err04_collided_aliasing() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(104);
    let mut case = Case::new("E04 c2Collided A aliases B");
    let mut buf = [0u8; 64];
    for _ in 0..5_000 {
        let p0 = g.v_finite(30.0);
        for rr in [0.0f32, -0.0, 1.0, -1.0, g.unit() * 20.0, QNAN, f32::INFINITY] {
            let ptr = put(
                &mut buf,
                0,
                C2Capsule {
                    a: p0,
                    b: p0,
                    r: rr,
                },
            );
            for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                let cv = unsafe { (c.c2Collided)(ptr, ptr, t) };
                let rv = unsafe { (r.c2Collided)(ptr, ptr, t) };
                case.eq(cv, rv, || format!("aliased typeB={t} r={rr}"));
            }
        }

        // Sentinel check, with an actual `c2Circle` in the buffer this time.
        // (Above, the buffer holds a `c2Capsule`, so the reinterpreted
        // `c2Circle.r` is the capsule's `b.x`, not its `r` — that is exactly the
        // blind `*(c2Circle *)A` cast the C code performs, and it is preserved.)
        let mut cbuf = [0u8; 64];
        for rr in [0.0f32, -0.0, 1.0, -1.0, 12.5, QNAN, f32::INFINITY] {
            let cp = put(&mut cbuf, 0, C2Circle { p: p0, r: rr });
            let cv = unsafe { (c.c2Collided)(cp, cp, C2_TYPE_CIRCLE) };
            let rv = unsafe { (r.c2Collided)(cp, cp, C2_TYPE_CIRCLE) };
            case.eq(cv, rv, || format!("aliased circle, r={rr}"));
            // Distance is exactly 0 and r2 == (2r)^2, so the strict `<` rejects
            // r == ±0 and NaN, and accepts any other finite/infinite radius.
            let expect = if rr == 0.0 || rr.is_nan() { 0 } else { 1 };
            case.eq(cv, expect, || {
                format!("aliased circle r={rr}: expected {expect}, C gave {cv}")
            });
        }
    }
    case.finish();
}

// ===========================================================================
// Row 5 — unguarded `da / c2Dot(n,n)` division
// ===========================================================================

#[test]
fn err05_capsule_division_by_zero() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E05 c2CircletoCapsule da / c2Dot(n,n)");
    let mut g = Rng::seeded(105);

    // (a) Exactly degenerate capsule: n == (0,0) so c2Dot(n,n) == 0. `da` and
    //     `db` are then ±0, neither is `< 0`, so branch C runs and the division
    //     is skipped. Verified against both libraries.
    for _ in 0..3_000 {
        let pt = g.v_finite(40.0);
        let cap = C2Capsule {
            a: pt,
            b: pt,
            r: g.unit() * 15.0,
        };
        let a = C2Circle {
            p: g.v_finite(60.0),
            r: g.unit() * 15.0,
        };
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("degenerate {a:?} {cap:?}"));
    }

    // (b) Underflowing capsule where c2Dot(n,n) rounds to zero on the
    //     branch-B path, so the division really is `x / 0` -> ±inf or NaN, then
    //     `c2Mulvs` yields NaN and `NaN < r*r` is false -> 0.
    let mut hit_div0 = 0usize;
    let scales = [1e-23f32, 3e-24, 1e-24, 5e-23, 1e-30, f32::from_bits(1)];
    for _ in 0..40_000 {
        let s = scales[g.below(scales.len() as u32) as usize];
        let a0 = C2v {
            x: g.sym(1.0) * s,
            y: g.sym(1.0) * s,
        };
        let cap = C2Capsule {
            a: a0,
            b: C2v {
                x: a0.x + g.sym(1.0) * s,
                y: a0.y + g.sym(1.0) * s,
            },
            r: g.unit() * s,
        };
        let a = C2Circle {
            p: C2v {
                x: g.sym(1.0) * s,
                y: g.sym(1.0) * s,
            },
            r: g.unit() * s,
        };
        let n = unsafe { (c.c2Sub)(cap.b, cap.a) };
        let ap = unsafe { (c.c2Sub)(a.p, cap.a) };
        let da = unsafe { (c.c2Dot)(ap, n) };
        let db = unsafe { (c.c2Dot)((c.c2Sub)(a.p, cap.b), n) };
        let on_b = !(da < 0.0) && db < 0.0;
        if on_b && unsafe { (c.c2Dot)(n, n) } == 0.0 {
            hit_div0 += 1;
        }
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("underflow s={s:e} {a:?} {cap:?}"));
    }
    assert!(
        hit_div0 > 0,
        "E05 never reached the unguarded division with a zero divisor"
    );
    eprintln!("  E05: division by exactly-zero c2Dot(n,n) reached {hit_div0} times");

    // (c) A hand-built case that lands on branch B with c2Dot(n,n) == +inf, so
    //     the quotient is 0 and `c2Mulvs(n, 0)` multiplies inf-scale values by 0.
    let cap = C2Capsule {
        a: C2v { x: 0.0, y: 0.0 },
        b: C2v { x: 1e30, y: 0.0 },
        r: 1.0,
    };
    let a = C2Circle {
        p: C2v { x: 1.0, y: 0.5 },
        r: 1.0,
    };
    let nn = unsafe {
        let n = (c.c2Sub)(cap.b, cap.a);
        (c.c2Dot)(n, n)
    };
    assert!(nn.is_infinite(), "expected c2Dot(n,n) == inf, got {nn}");
    let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
    let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
    case.eq(cv, rv, || "inf divisor".to_string());
    case.finish();
}

// ===========================================================================
// Rows 6, 7, 8, 9 — c2CircletoCapsule NaN / overflow / negative radii
// ===========================================================================

#[test]
fn err06_capsule_nan_centre() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(106);
    let mut case = Case::new("E06 capsule NaN A.p -> 0");
    for _ in 0..8_000 {
        for p in [
            C2v { x: QNAN, y: g.sym(50.0) },
            C2v { x: g.sym(50.0), y: QNAN },
            C2v { x: QNAN, y: QNAN },
            C2v { x: SNAN, y: g.sym(50.0) },
            C2v { x: g.sym(50.0), y: SNAN },
        ] {
            let a0 = g.v_finite(40.0);
            let cap = C2Capsule {
                a: a0,
                b: C2v {
                    x: a0.x + g.sym(50.0),
                    y: a0.y + g.sym(50.0),
                },
                r: g.unit() * 20.0,
            };
            let a = C2Circle { p, r: g.unit() * 20.0 };
            let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
            let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
            case.eq(cv, rv, || format!("{a:?} {cap:?}"));
            case.eq(cv, 0, || format!("NaN centre must reject: {a:?} {cap:?}"));
        }
    }
    case.finish();
}

#[test]
fn err07_capsule_nan_radius() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(107);
    let mut case = Case::new("E07 capsule NaN radius sum -> 0");
    for _ in 0..8_000 {
        for (ar, br) in [
            (QNAN, g.unit() * 20.0),
            (g.unit() * 20.0, QNAN),
            (QNAN, QNAN),
            (SNAN, 1.0),
            (f32::INFINITY, f32::NEG_INFINITY), // inf + -inf = NaN
            (f32::NEG_INFINITY, f32::INFINITY),
        ] {
            let a0 = g.v_finite(40.0);
            let cap = C2Capsule {
                a: a0,
                b: C2v {
                    x: a0.x + g.sym(50.0),
                    y: a0.y + g.sym(50.0),
                },
                r: br,
            };
            let a = C2Circle {
                p: g.v_finite(40.0),
                r: ar,
            };
            let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
            let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
            case.eq(cv, rv, || format!("ar={ar} br={br} {a:?} {cap:?}"));
            case.eq(cv, 0, || format!("NaN r*r must reject: ar={ar} br={br}"));
        }
    }
    case.finish();
}

#[test]
fn err08_capsule_radius_overflow() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(108);
    let mut case = Case::new("E08 capsule radius sum -> +inf");
    for _ in 0..8_000 {
        let a0 = g.v_finite(40.0);
        let cap = C2Capsule {
            a: a0,
            b: C2v {
                x: a0.x + g.sym(50.0),
                y: a0.y + g.sym(50.0),
            },
            r: f32::MAX,
        };
        // Finite centre -> finite d2 -> `d2 < inf` is 1.
        let a = C2Circle {
            p: g.v_finite(40.0),
            r: f32::MAX,
        };
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("{a:?} {cap:?}"));
        case.eq(cv, 1, || format!("finite d2 < inf must accept: {a:?}"));

        // Infinite centre -> d2 = inf or NaN -> `inf < inf` is 0.
        let a2 = C2Circle {
            p: C2v {
                x: f32::INFINITY,
                y: g.sym(40.0),
            },
            r: f32::MAX,
        };
        let cv2 = unsafe { (c.c2CircletoCapsule)(a2, cap) };
        let rv2 = unsafe { (r.c2CircletoCapsule)(a2, cap) };
        case.eq(cv2, rv2, || format!("{a2:?} {cap:?}"));
        case.eq(cv2, 0, || format!("inf < inf must reject: {a2:?}"));
    }
    case.finish();
}

#[test]
fn err09_capsule_negative_radii() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(109);
    let mut case = Case::new("E09 capsule negative radii");
    for _ in 0..10_000 {
        let a0 = g.v_finite(40.0);
        let cap_pos = C2Capsule {
            a: a0,
            b: C2v {
                x: a0.x + g.sym(50.0),
                y: a0.y + g.sym(50.0),
            },
            r: g.unit() * 20.0,
        };
        let ar = g.unit() * 20.0;
        let a_pos = C2Circle {
            p: g.v_finite(60.0),
            r: ar,
        };
        // Flip both signs: `r = A.r + B.r` negates, and `r*r` is unchanged, so
        // the C result must be identical to the all-positive case.
        let cap_neg = C2Capsule { r: -cap_pos.r, ..cap_pos };
        let a_neg = C2Circle { r: -ar, ..a_pos };
        let c_pos = unsafe { (c.c2CircletoCapsule)(a_pos, cap_pos) };
        let c_neg = unsafe { (c.c2CircletoCapsule)(a_neg, cap_neg) };
        let r_neg = unsafe { (r.c2CircletoCapsule)(a_neg, cap_neg) };
        case.eq(c_neg, r_neg, || format!("{a_neg:?} {cap_neg:?}"));
        case.eq(c_pos, c_neg, || {
            format!("sign flip must not change the C result: {a_pos:?}")
        });
        // Mixed signs too (no cancellation assumption).
        let a_mix = C2Circle { r: -ar, ..a_pos };
        let cm = unsafe { (c.c2CircletoCapsule)(a_mix, cap_pos) };
        let rm = unsafe { (r.c2CircletoCapsule)(a_mix, cap_pos) };
        case.eq(cm, rm, || format!("mixed signs {a_mix:?} {cap_pos:?}"));
    }
    case.finish();
}

// ===========================================================================
// Rows 10, 11, 12 — c2CircletoCircle
// ===========================================================================

#[test]
fn err10_circle_radius_overflow_nan_negative() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(110);
    let mut case = Case::new("E10 circle radius overflow/NaN/negative");
    for _ in 0..10_000 {
        for (ar, br) in [
            (f32::MAX, f32::MAX),               // -> +inf
            (3.0e38, 3.0e38),                   // -> +inf
            (QNAN, 1.0),
            (1.0, QNAN),
            (SNAN, SNAN),
            (f32::INFINITY, f32::NEG_INFINITY), // -> NaN
            (-20.0, -30.0),                     // negative, squares positive
            (-20.0, 30.0),
        ] {
            let a = C2Circle {
                p: g.v_finite(30.0),
                r: ar,
            };
            let b = C2Circle {
                p: g.v_finite(30.0),
                r: br,
            };
            let cv = unsafe { (c.c2CircletoCircle)(a, b) };
            let rv = unsafe { (r.c2CircletoCircle)(a, b) };
            case.eq(cv, rv, || format!("ar={ar} br={br} {a:?} {b:?}"));
            if ar.is_nan() || br.is_nan() || (ar.is_infinite() && br.is_infinite()) {
                case.eq(cv, 0, || format!("NaN r2 must reject: ar={ar} br={br}"));
            }
        }
    }
    case.finish();
}

#[test]
fn err11_circle_centre_overflow() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E11 circle centre difference -> ±inf");
    let mut g = Rng::seeded(111);
    for _ in 0..4_000 {
        for (bigx, bigy) in [
            (f32::MAX, f32::MAX),
            (-f32::MAX, f32::MAX),
            (3.0e38, -3.0e38),
            (f32::INFINITY, 0.0),
            (f32::NEG_INFINITY, f32::INFINITY),
        ] {
            for br in [1.0f32, f32::MAX, f32::INFINITY, QNAN] {
                let a = C2Circle {
                    p: C2v { x: -bigx, y: -bigy },
                    r: g.unit(),
                };
                let b = C2Circle {
                    p: C2v { x: bigx, y: bigy },
                    r: br,
                };
                let cv = unsafe { (c.c2CircletoCircle)(a, b) };
                let rv = unsafe { (r.c2CircletoCircle)(a, b) };
                case.eq(cv, rv, || format!("{a:?} {b:?}"));
                case.eq(cv, 0, || {
                    format!("inf/NaN d2 must reject: {a:?} {b:?}")
                });
            }
        }
    }
    case.finish();
}

#[test]
fn err12_circle_nan_centre() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(112);
    let mut case = Case::new("E12 circle NaN centre -> 0");
    for _ in 0..10_000 {
        for p in [
            C2v { x: QNAN, y: 0.0 },
            C2v { x: 0.0, y: QNAN },
            C2v { x: SNAN, y: SNAN },
            C2v { x: QNAN, y: QNAN },
        ] {
            let a = C2Circle { p, r: g.unit() * 50.0 };
            let b = C2Circle {
                p: g.v_finite(30.0),
                r: g.unit() * 50.0,
            };
            let cv = unsafe { (c.c2CircletoCircle)(a, b) };
            let rv = unsafe { (r.c2CircletoCircle)(a, b) };
            case.eq(cv, rv, || format!("{a:?} {b:?}"));
            case.eq(cv, 0, || format!("NaN d2 must reject: {a:?}"));
            // Same NaN, but on B's centre.
            let a2 = C2Circle {
                p: g.v_finite(30.0),
                r: g.unit() * 50.0,
            };
            let b2 = C2Circle { p, r: g.unit() * 50.0 };
            let cv2 = unsafe { (c.c2CircletoCircle)(a2, b2) };
            let rv2 = unsafe { (r.c2CircletoCircle)(a2, b2) };
            case.eq(cv2, rv2, || format!("{a2:?} {b2:?}"));
            case.eq(cv2, 0, || format!("NaN d2 must reject: {b2:?}"));
        }
    }
    case.finish();
}

// ===========================================================================
// Rows 13, 14, 15, 16 — c2CircletoAABB
// ===========================================================================

#[test]
fn err13_aabb_inverted_box() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(113);
    let mut case = Case::new("E13 AABB min > max (inverted)");
    for _ in 0..15_000 {
        let lo = g.v_finite(40.0);
        let hi = C2v {
            x: lo.x - g.unit() * 50.0 - 0.5, // strictly less than min
            y: lo.y - g.unit() * 50.0 - 0.5,
        };
        let bx = C2Aabb { min: lo, max: hi };
        let a = C2Circle {
            p: g.v_finite(60.0),
            r: g.unit() * 30.0,
        };
        let cv = unsafe { (c.c2CircletoAABB)(a, bx) };
        let rv = unsafe { (r.c2CircletoAABB)(a, bx) };
        case.eq(cv, rv, || format!("{a:?} {bx:?}"));
        // With lo > hi the clamp collapses to `lo`, so the result must equal
        // the distance to the `min` corner. Cross-check via the primitives.
        let expect = unsafe {
            let ab = (c.c2Sub)(a.p, lo);
            ((c.c2Dot)(ab, ab) < a.r * a.r) as i32
        };
        case.eq(cv, expect, || format!("clamp must collapse to lo: {a:?} {bx:?}"));
    }
    case.finish();
}

#[test]
fn err14_aabb_nan_bounds() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(114);
    let mut case = Case::new("E14 AABB NaN bounds");
    for _ in 0..8_000 {
        let f = g.v_finite(40.0);
        for bx in [
            C2Aabb {
                min: C2v { x: QNAN, y: f.y },
                max: f,
            },
            C2Aabb {
                min: f,
                max: C2v { x: QNAN, y: f.y },
            },
            C2Aabb {
                min: C2v { x: QNAN, y: QNAN },
                max: C2v { x: QNAN, y: QNAN },
            },
            C2Aabb {
                min: C2v { x: SNAN, y: f.y },
                max: C2v { x: f.x, y: SNAN },
            },
        ] {
            let a = C2Circle {
                p: g.v_finite(40.0),
                r: g.unit() * 30.0,
            };
            let cv = unsafe { (c.c2CircletoAABB)(a, bx) };
            let rv = unsafe { (r.c2CircletoAABB)(a, bx) };
            case.eq(cv, rv, || format!("{a:?} {bx:?}"));
        }
    }
    case.finish();
}

#[test]
fn err15_aabb_radius_specials() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(115);
    let mut case = Case::new("E15 AABB radius NaN/overflow/negative");
    for _ in 0..10_000 {
        let lo = g.v_finite(40.0);
        let bx = C2Aabb {
            min: lo,
            max: C2v {
                x: lo.x + g.unit() * 40.0,
                y: lo.y + g.unit() * 40.0,
            },
        };
        for rr in [
            0.0f32,
            -0.0,
            QNAN,
            SNAN,
            f32::MAX,
            3.0e38,
            f32::INFINITY,
            -30.0,
            -f32::MAX,
        ] {
            let a = C2Circle {
                p: g.v_finite(60.0),
                r: rr,
            };
            let cv = unsafe { (c.c2CircletoAABB)(a, bx) };
            let rv = unsafe { (r.c2CircletoAABB)(a, bx) };
            case.eq(cv, rv, || format!("r={rr} {a:?} {bx:?}"));
            if rr.is_nan() {
                case.eq(cv, 0, || format!("NaN r2 must reject: r={rr}"));
            }
            if rr == 0.0 {
                case.eq(cv, 0, || format!("zero r2 must reject: r={rr}"));
            }
            // A negative radius must behave exactly like its magnitude.
            if rr.is_finite() && rr != 0.0 {
                let flipped = C2Circle { r: -rr, ..a };
                let cf = unsafe { (c.c2CircletoAABB)(flipped, bx) };
                case.eq(cv, cf, || format!("|r| symmetry broken: r={rr}"));
            }
        }
    }
    case.finish();
}

#[test]
fn err16_aabb_nan_centre() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(116);
    let mut case = Case::new("E16 AABB NaN centre -> 0");
    for _ in 0..10_000 {
        let lo = g.v_finite(40.0);
        let bx = C2Aabb {
            min: lo,
            max: C2v {
                x: lo.x + g.unit() * 40.0,
                y: lo.y + g.unit() * 40.0,
            },
        };
        for p in [
            C2v { x: QNAN, y: lo.y },
            C2v { x: lo.x, y: QNAN },
            C2v { x: QNAN, y: QNAN },
            C2v { x: SNAN, y: SNAN },
        ] {
            let a = C2Circle {
                p,
                r: g.unit() * 50.0 + 1.0,
            };
            let cv = unsafe { (c.c2CircletoAABB)(a, bx) };
            let rv = unsafe { (r.c2CircletoAABB)(a, bx) };
            case.eq(cv, rv, || format!("{a:?} {bx:?}"));
            case.eq(cv, 0, || format!("NaN centre must reject: {a:?} {bx:?}"));
        }
    }
    case.finish();
}

// ===========================================================================
// Rows 17, 18 — c2Dot
// ===========================================================================

#[test]
fn err17_dot_inf_minus_inf() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E17 c2Dot inf + -inf -> NaN");
    let mut g = Rng::seeded(117);
    for _ in 0..5_000 {
        let m = [1e20f32, 1e30, f32::MAX, 1e25][g.below(4) as usize];
        // x lane -> +inf, y lane -> -inf (and the mirror).
        for sgn in [1.0f32, -1.0] {
            let a = C2v { x: m, y: m };
            let b = C2v {
                x: m * sgn,
                y: m * -sgn,
            };
            let cv = unsafe { (c.c2Dot)(a, b) };
            let rv = unsafe { (r.c2Dot)(a, b) };
            case.eq(fb(cv), fb(rv), || format!("{a:?} {b:?}"));
            assert!(cv.is_nan(), "expected NaN from inf + -inf, got {cv}");
        }
        // Explicit ±inf operands too.
        for (ax, ay, bx, by) in [
            (f32::INFINITY, f32::INFINITY, 1.0f32, -1.0f32),
            (f32::INFINITY, f32::NEG_INFINITY, 1.0, 1.0),
            (f32::INFINITY, 0.0, 0.0, 1.0), // inf * 0 -> NaN
            (0.0, f32::INFINITY, 1.0, 0.0),
        ] {
            let a = C2v { x: ax, y: ay };
            let b = C2v { x: bx, y: by };
            let cv = unsafe { (c.c2Dot)(a, b) };
            let rv = unsafe { (r.c2Dot)(a, b) };
            case.eq(fb(cv), fb(rv), || format!("{a:?} {b:?}"));
        }
    }
    case.finish();
}

#[test]
fn err18_dot_nan_operand_order() {
    // The SSE destination-operand rule: with two NaNs the result's payload is
    // the destination's. GCC's -O0 codegen makes `a.x` the destination in the
    // x lane, `b.y` in the y lane, and the y product the destination of the
    // addss. Exhaustive over the NaN edge list, both lanes.
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E18 c2Dot NaN payload / operand order");
    let all: Vec<f32> = NANS
        .iter()
        .copied()
        .chain([0.0f32, -0.0, f32::NEG_INFINITY, 2.0, -3.5])
        .collect();
    for &ax in &all {
        for &ay in &all {
            for &bx in &all {
                for &by in &all {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let cv = unsafe { (c.c2Dot)(a, b) };
                    let rv = unsafe { (r.c2Dot)(a, b) };
                    case.eq(fb(cv), fb(rv), || {
                        format!(
                            "c2Dot(({:#010x},{:#010x}),({:#010x},{:#010x}))",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        )
                    });
                }
            }
        }
    }
    case.finish();
}

// ===========================================================================
// Row 19 — c2Mulvs
// ===========================================================================

#[test]
fn err19_mulvs_nan_and_zero_times_inf() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E19 c2Mulvs NaN order / 0*inf");
    let all: Vec<f32> = NANS
        .iter()
        .copied()
        .chain([0.0f32, -0.0, f32::NEG_INFINITY, f32::MAX, f32::MIN_POSITIVE, 1.5])
        .collect();
    for &b in &all {
        for &ax in &all {
            for &ay in &all {
                let a = C2v { x: ax, y: ay };
                let cv = unsafe { (c.c2Mulvs)(a, b) };
                let rv = unsafe { (r.c2Mulvs)(a, b) };
                case.eq(vb(cv), vb(rv), || {
                    format!(
                        "c2Mulvs(({:#010x},{:#010x}),{:#010x})",
                        fb(ax),
                        fb(ay),
                        fb(b)
                    )
                });
            }
        }
    }
    case.finish();
}

// ===========================================================================
// Rows 20, 21 — c2Maxv / c2Minv ternary (NOT IEEE min/max)
// ===========================================================================

#[test]
fn err20_minmax_nan_returns_b() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E20 c2Maxv/c2Minv NaN -> b operand");
    let all: Vec<f32> = NANS
        .iter()
        .copied()
        .chain([0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY])
        .collect();
    for &ax in &all {
        for &bx in &all {
            for &ay in &all {
                for &by in &all {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    for (cf, rf, what) in [
                        (c.c2Maxv, r.c2Maxv, "c2Maxv"),
                        (c.c2Minv, r.c2Minv, "c2Minv"),
                    ] {
                        let cv = unsafe { cf(a, b) };
                        let rv = unsafe { rf(a, b) };
                        case.eq(vb(cv), vb(rv), || format!("{what} {a:?} {b:?}"));
                        // When a lane of `a` is NaN the ternary is false, so the
                        // C result must be the corresponding lane of `b`.
                        if ax.is_nan() {
                            case.eq(fb(cv.x), fb(bx), || {
                                format!("{what}: NaN a.x must yield b.x")
                            });
                        }
                        if ay.is_nan() {
                            case.eq(fb(cv.y), fb(by), || {
                                format!("{what}: NaN a.y must yield b.y")
                            });
                        }
                    }
                }
            }
        }
    }
    case.finish();
}

#[test]
fn err21_minmax_signed_zero() {
    // `+0.0 > -0.0` and `+0.0 < -0.0` are both false, so the ternary always
    // yields `b` and the SIGN of the returned zero comes from `b`.
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E21 c2Maxv/c2Minv signed zero");
    for &ax in &[0.0f32, -0.0] {
        for &ay in &[0.0f32, -0.0] {
            for &bx in &[0.0f32, -0.0] {
                for &by in &[0.0f32, -0.0] {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    for (cf, rf, what) in [
                        (c.c2Maxv, r.c2Maxv, "c2Maxv"),
                        (c.c2Minv, r.c2Minv, "c2Minv"),
                    ] {
                        let cv = unsafe { cf(a, b) };
                        let rv = unsafe { rf(a, b) };
                        case.eq(vb(cv), vb(rv), || {
                            format!(
                                "{what}(({:#010x},{:#010x}),({:#010x},{:#010x}))",
                                fb(ax),
                                fb(ay),
                                fb(bx),
                                fb(by)
                            )
                        });
                        // Both comparisons are false for equal zeros -> `b` wins.
                        case.eq(vb(cv), vb(b), || {
                            format!("{what}: equal zeros must yield b")
                        });
                    }
                }
            }
        }
    }
    case.finish();
}

// ===========================================================================
// Row 22 — c2Clampv inverted range / NaN
// ===========================================================================

#[test]
fn err22_clampv_inverted_and_nan() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E22 c2Clampv inverted range / NaN");
    let vals: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        5.0,
        -5.0,
        QNAN,
        SNAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &a in &vals {
        for &lo in &vals {
            for &hi in &vals {
                let av = C2v { x: a, y: -a };
                let lv = C2v { x: lo, y: hi };
                let hv = C2v { x: hi, y: lo };
                let cv = unsafe { (c.c2Clampv)(av, lv, hv) };
                let rv = unsafe { (r.c2Clampv)(av, lv, hv) };
                case.eq(vb(cv), vb(rv), || format!("{av:?} {lv:?} {hv:?}"));
                // Composition identity: c2Clampv == c2Maxv(lo, c2Minv(a, hi)).
                let expect = unsafe { (c.c2Maxv)(lv, (c.c2Minv)(av, hv)) };
                case.eq(vb(cv), vb(expect), || {
                    format!("clamp != Maxv(lo, Minv(a, hi)): {av:?} {lv:?} {hv:?}")
                });
            }
        }
    }
    case.finish();
}

// ===========================================================================
// Row 23 — c2Sub
// ===========================================================================

#[test]
fn err23_sub_inf_and_signed_zero() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E23 c2Sub inf-inf / signed zero / overflow");
    let vals: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        QNAN,
        SNAN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let cv = unsafe { (c.c2Sub)(a, b) };
                    let rv = unsafe { (r.c2Sub)(a, b) };
                    case.eq(vb(cv), vb(rv), || {
                        format!(
                            "c2Sub(({:#010x},{:#010x}),({:#010x},{:#010x}))",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        )
                    });
                }
            }
        }
    }
    // x - x == +0.0 ; -0.0 - +0.0 == -0.0 (sign of zero must match).
    let z = unsafe { (c.c2Sub)(C2v { x: 3.0, y: 3.0 }, C2v { x: 3.0, y: 3.0 }) };
    assert_eq!(fb(z.x), fb(0.0f32), "x - x must be +0.0");
    let nz = unsafe {
        (c.c2Sub)(
            C2v { x: -0.0, y: -0.0 },
            C2v { x: 0.0, y: 0.0 },
        )
    };
    assert_eq!(fb(nz.x), fb(-0.0f32), "-0.0 - +0.0 must be -0.0");
    case.finish();
}

// ===========================================================================
// Row 24 — c2V must not canonicalise (sNaN stays signalling)
// ===========================================================================

#[test]
fn err24_c2v_no_canonicalisation() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E24 c2V preserves exact bit patterns");
    // Every "interesting" 32-bit pattern class, plus a dense sweep of NaN
    // payloads and the subnormal/normal boundary.
    let mut pats: Vec<u32> = vec![
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7f80_0001, // sNaN
        0xff80_0001,
        0x7fbf_ffff, // largest sNaN
        0x7fc0_0000, // smallest qNaN
        0x7fff_ffff,
        0xffff_ffff,
    ];
    for i in 0..64u32 {
        pats.push(0x7f80_0000 | (i * 0x0002_0001).max(1) & 0x007f_ffff);
        pats.push(0xff80_0000 | (i * 0x0003_0007).max(1) & 0x007f_ffff);
    }
    for &bx in &pats {
        for &by in &pats {
            let (x, y) = (f32::from_bits(bx), f32::from_bits(by));
            let cv = unsafe { (c.c2V)(x, y) };
            let rv = unsafe { (r.c2V)(x, y) };
            case.eq(vb(cv), vb(rv), || format!("c2V({bx:#010x}, {by:#010x})"));
            // c2V is pure construction: the bits must survive untouched.
            case.eq(fb(cv.x), bx, || format!("c2V must not alter x: {bx:#010x}"));
            case.eq(fb(cv.y), by, || format!("c2V must not alter y: {by:#010x}"));
        }
    }
    case.finish();
}

// ===========================================================================
// Rows 25, 26, 27, 28 — circle_collide
// ===========================================================================

#[test]
fn err25_circle_collide_nan() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E25 circle_collide NaN -> 0");
    let nans = [QNAN, SNAN, f32::from_bits(0xffc0_dead), f32::from_bits(0xffff_ffff)];
    for &n in &nans {
        for &other in &[0.0f32, -70.0, -27.5, -30.0, 20.0, 1e30] {
            for pos in 0..3 {
                let (x, y, rr) = match pos {
                    0 => (n, other, 20.0),
                    1 => (other, n, 20.0),
                    _ => (other, other, n),
                };
                let cv = unsafe { (c.circle_collide)(x, y, rr) };
                let rv = unsafe { (r.circle_collide)(x, y, rr) };
                case.eq(cv, rv, || format!("circle_collide({x}, {y}, {rr})"));
                case.eq(cv, 0, || {
                    format!("NaN input must give 0, got {cv} for ({x}, {y}, {rr})")
                });
            }
        }
    }
    // All three NaN at once.
    let cv = unsafe { (c.circle_collide)(QNAN, QNAN, QNAN) };
    let rv = unsafe { (r.circle_collide)(QNAN, QNAN, QNAN) };
    case.eq(cv, rv, || "all NaN".to_string());
    case.eq(cv, 0, || "all NaN must give 0".to_string());
    case.finish();
}

#[test]
fn err26_circle_collide_infinities() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("E26 circle_collide ±inf");
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    for &i in &infs {
        for &other in &[0.0f32, -0.0, -70.0, -27.5, 20.0, f32::MAX, -f32::MAX] {
            for pos in 0..3 {
                let (x, y, rr) = match pos {
                    0 => (i, other, 20.0),
                    1 => (other, i, 20.0),
                    _ => (other, other, i),
                };
                let cv = unsafe { (c.circle_collide)(x, y, rr) };
                let rv = unsafe { (r.circle_collide)(x, y, rr) };
                case.eq(cv, rv, || format!("circle_collide({x}, {y}, {rr})"));
            }
        }
        // Infinite centre -> every d2 is inf or NaN, so nothing collides.
        let cv = unsafe { (c.circle_collide)(i, i, 20.0) };
        case.eq(cv, 0, || format!("inf centre must give 0, got {cv}"));
    }
    case.finish();
}

#[test]
fn err27_circle_collide_zero_and_negative_r() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(127);
    let mut case = Case::new("E27 circle_collide r = 0 / -0 / negative");
    for _ in 0..8_000 {
        let x = g.sym(120.0);
        let y = g.sym(120.0);
        for rr in [
            0.0f32,
            -0.0,
            -1.0,
            -20.0,
            -60.0,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            -f32::MIN_POSITIVE,
        ] {
            let cv = unsafe { (c.circle_collide)(x, y, rr) };
            let rv = unsafe { (r.circle_collide)(x, y, rr) };
            case.eq(cv, rv, || format!("circle_collide({x}, {y}, {rr})"));
        }
    }
    // Deterministic sanity: r = 0 at the exact centre of the box still collides
    // with the box (d2 == 0 < 0 is false, so it must NOT) — assert whatever C says.
    for &(x, y) in &[(-27.5f32, -27.5f32), (-70.0, 0.0), (-30.0, 70.0)] {
        let cv = unsafe { (c.circle_collide)(x, y, 0.0) };
        let rv = unsafe { (r.circle_collide)(x, y, 0.0) };
        case.eq(cv, rv, || format!("r=0 at ({x},{y})"));
    }
    case.finish();
}

#[test]
fn err28_circle_collide_huge_r() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(128);
    let mut case = Case::new("E28 circle_collide huge r -> inf");
    for _ in 0..5_000 {
        let x = g.sym(200.0);
        let y = g.sym(200.0);
        for rr in [f32::MAX, 3.0e38, 3.4e38, 1e38, 1e30] {
            let cv = unsafe { (c.circle_collide)(x, y, rr) };
            let rv = unsafe { (r.circle_collide)(x, y, rr) };
            case.eq(cv, rv, || format!("circle_collide({x}, {y}, {rr})"));
        }
    }
    // FLT_MAX + 20.0f overflows to +inf, so the circle test (bit 0) must fire
    // for any finite centre, and the AABB test (bit 1) too since r*r == inf.
    let cv = unsafe { (c.circle_collide)(0.0, 0.0, f32::MAX) };
    let rv = unsafe { (r.circle_collide)(0.0, 0.0, f32::MAX) };
    case.eq(cv, rv, || "FLT_MAX radius".to_string());
    case.eq(cv, 7, || {
        format!("FLT_MAX radius should hit all three shapes, got {cv}")
    });
    case.finish();
}
