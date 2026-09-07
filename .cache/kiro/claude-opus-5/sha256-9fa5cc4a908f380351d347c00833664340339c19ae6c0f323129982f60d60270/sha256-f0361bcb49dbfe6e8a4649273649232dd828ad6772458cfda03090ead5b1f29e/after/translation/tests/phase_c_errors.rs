//! Phase C — error / rejection-path differential tests.
//!
//! One test per group of `ERRORS.md` rows. Each constructs the exact invalid
//! input or degenerate condition and asserts that BOTH libraries produce the
//! SAME concrete result (the same sentinel `0`, or the same bit pattern) — not
//! merely "both failed".
//!
//! The library has no error codes, no `assert`, and no null checks; the single
//! true rejection branch is the `default:` arm of `c2Collided`'s switch. See
//! `ERRORS.md` for how that was established from the C source.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Rows 1-5 — the only real rejection branch: out-of-range C2_TYPE.
// ---------------------------------------------------------------------------

/// Rows 1-3 — `typeB` one past the last valid variant, negative, and the `int`
/// extremes. A C enum accepts any `int`, so these are real inputs. Both must
/// return the sentinel `0`.
#[test]
fn rows01_03_collided_out_of_range_enum() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let bad: Vec<c_int> = vec![
        3, // one past C2_TYPE_CAPSULE
        4,
        -1,
        -2,
        100,
        255,
        256,
        1 << 16,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX - 1,
    ];

    for &ty in &bad {
        for _ in 0..200 {
            let a = rng.circle();
            let cap = rng.capsule();
            let ac = &a as *const _ as *const c_void;
            let bc = &cap as *const _ as *const c_void;
            let (c, rs) = unsafe { ((p.c.c2Collided)(ac, bc, ty), (p.rs.c2Collided)(ac, bc, ty)) };
            assert_int_eq(&format!("c2Collided bad type {ty}"), c, rs);
            assert_eq!(c, 0, "C did not return the sentinel 0 for typeB={ty}");
            assert_eq!(rs, 0, "Rust did not return the sentinel 0 for typeB={ty}");
        }
    }
}

/// Row 4 — out-of-range `typeB` together with NULL `A` and `B`. The C provably
/// never dereferences on the `default:` arm, so this is a defined input and the
/// Rust must not dereference either.
#[test]
fn row04_collided_out_of_range_enum_null_pointers() {
    let p = load();
    let nul = std::ptr::null::<c_void>();
    let a = C2Circle { p: v(1.0, 2.0), r: 3.0 };
    let ap = &a as *const _ as *const c_void;

    for ty in [3, 4, -1, 7, c_int::MAX, c_int::MIN] {
        for &(x, y) in &[(nul, nul), (nul, ap), (ap, nul)] {
            let (c, rs) = unsafe { ((p.c.c2Collided)(x, y, ty), (p.rs.c2Collided)(x, y, ty)) };
            assert_int_eq(&format!("c2Collided null + bad type {ty}"), c, rs);
            assert_eq!(c, 0);
            assert_eq!(rs, 0);
        }
    }
}

/// Row 5 — exhaustive sweep of every `typeB` in `[-256, 256]`: the three valid
/// values must dispatch identically, every other value must yield `0`.
#[test]
fn row05_collided_enum_sweep() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..40 {
        let a = rng.circle();
        // Buffer wide enough for the widest shape so the valid arms are safe.
        let mut buf = [0u32; 8];
        for (i, w) in buf.iter_mut().enumerate() {
            *w = if i < 5 { rng.coord().to_bits() } else { rng.next_u32() };
        }
        let ap = &a as *const _ as *const c_void;
        let bp = buf.as_ptr() as *const c_void;

        for ty in -256i32..=256i32 {
            let (c, rs) = unsafe { ((p.c.c2Collided)(ap, bp, ty), (p.rs.c2Collided)(ap, bp, ty)) };
            assert_int_eq(&format!("c2Collided sweep ty={ty}"), c, rs);
            if !(0..=2).contains(&ty) {
                assert_eq!(c, 0, "C returned {c} for invalid typeB={ty}");
                assert_eq!(rs, 0, "Rust returned {rs} for invalid typeB={ty}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6-9 — c2CircletoCircle degenerate inputs.
// ---------------------------------------------------------------------------

/// Rows 6-9 — negative radii (silently treated like their magnitude), radius
/// sum / square overflow to `+inf`, NaN anywhere, and `inf - inf = NaN` from
/// infinite coordinates.
#[test]
fn rows06_09_circle_circle_degenerate() {
    let p = load();
    let mut rng = Rng::default_seeded();
    let check = |a: C2Circle, b: C2Circle, what: &str| -> c_int {
        let (c, rs) = unsafe { ((p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b)) };
        assert_int_eq(what, c, rs);
        c
    };

    // Row 6 — negative radii.
    for _ in 0..2000 {
        let a = C2Circle { p: rng.vec_coord(), r: -rng.range(0.0, 50.0) };
        let b = C2Circle { p: rng.vec_coord(), r: -rng.range(0.0, 50.0) };
        check(a, b, "neg radii both");
        check(C2Circle { r: -a.r, ..a }, b, "neg radii one");
    }

    // Row 7 — overflow of A.r + B.r and of r2 * r2.
    for &r in &[1.0e30f32, 1.0e20, f32::MAX, f32::MAX / 2.0, 1.0e38] {
        for _ in 0..200 {
            let a = C2Circle { p: rng.vec_coord(), r };
            let b = C2Circle { p: rng.vec_coord(), r };
            let res = check(a, b, &format!("overflow r={r:?}"));
            // r2 = (r+r)^2 overflows to +inf, and d2 is finite here, so the
            // strict `<` is satisfied: the C returns 1.
            assert_eq!(res, 1, "expected overflow-to-inf collision for r={r:?}");
        }
    }

    // Row 8 — NaN in each of the six scalar slots.
    let nan = f32::NAN;
    for slot in 0..6 {
        for _ in 0..500 {
            let mut a = rng.circle();
            let mut b = rng.circle();
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.p.x = nan,
                4 => b.p.y = nan,
                _ => b.r = nan,
            }
            let res = check(a, b, &format!("nan slot {slot}"));
            assert_eq!(res, 0, "NaN must make the strict `<` false (slot {slot})");
        }
    }

    // Row 9 — inf - inf = NaN inside c2Sub.
    for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..500 {
            let a = C2Circle { p: v(s, s), r: rng.radius() };
            let b = C2Circle { p: v(s, s), r: rng.radius() };
            let res = check(a, b, &format!("inf-inf {s:?}"));
            assert_eq!(res, 0, "inf-inf NaN must reject");
            // Opposite infinities: inf - (-inf) = inf, d2 = inf, r2 finite.
            let b2 = C2Circle { p: v(-s, -s), r: rng.radius() };
            check(a, b2, "opposite infinities");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-14 — c2CircletoAABB degenerate inputs.
// ---------------------------------------------------------------------------

/// Rows 10-14 — inverted box, degenerate (point) box, zero radius (can never
/// collide), NaN in every slot, and `A.r` infinite / overflowing to `inf`.
#[test]
fn rows10_14_circle_aabb_degenerate() {
    let p = load();
    let mut rng = Rng::default_seeded();
    let check = |a: C2Circle, b: C2Aabb, what: &str| -> c_int {
        let (c, rs) = unsafe { ((p.c.c2CircletoAABB)(a, b), (p.rs.c2CircletoAABB)(a, b)) };
        assert_int_eq(what, c, rs);
        c
    };

    for _ in 0..3000 {
        let s = rng.aabb_sorted();
        let a = rng.circle();

        // Row 10 — inverted box: clamp collapses to `min`.
        check(a, C2Aabb { min: s.max, max: s.min }, "inverted box");
        check(
            a,
            C2Aabb { min: v(s.max.x, s.min.y), max: v(s.min.x, s.max.y) },
            "inverted x only",
        );

        // Row 11 — degenerate point box.
        check(a, C2Aabb { min: s.min, max: s.min }, "point box");

        // Row 12 — zero radius can never collide.
        let z = check(C2Circle { p: a.p, r: 0.0 }, s, "zero radius");
        assert_eq!(z, 0, "r == 0 must never collide");
        let z = check(C2Circle { p: a.p, r: -0.0 }, s, "negative zero radius");
        assert_eq!(z, 0, "r == -0.0 must never collide");
        // A negative radius squares to a positive r2, so it CAN collide —
        // pinned here so the asymmetry with row 12 stays honest.
        check(C2Circle { p: a.p, r: -a.r.abs() }, s, "negative radius");
    }

    // Row 13 — NaN in each of the six scalar slots.
    let nan = f32::NAN;
    for slot in 0..6 {
        for _ in 0..500 {
            let mut a = rng.circle();
            let mut b = rng.aabb_sorted();
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.min.x = nan,
                4 => b.max.y = nan,
                _ => b.min.y = nan,
            }
            check(a, b, &format!("aabb nan slot {slot}"));
        }
    }

    // Row 14 — infinite / overflowing radius.
    for &r in &[f32::INFINITY, f32::NEG_INFINITY, 1.0e30, -1.0e30, f32::MAX] {
        for _ in 0..500 {
            let b = rng.aabb_sorted();
            let res = check(C2Circle { p: rng.vec_coord(), r }, b, &format!("aabb r={r:?}"));
            assert_eq!(res, 1, "r^2 == inf must collide with a finite box (r={r:?})");
        }
        // Infinite box bounds -> clamp yields inf -> inf - inf = NaN.
        for &e in &[f32::INFINITY, f32::NEG_INFINITY] {
            let b = C2Aabb { min: v(e, e), max: v(e, e) };
            check(C2Circle { p: v(0.0, 0.0), r }, b, "aabb inf bounds");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-19 — c2CircletoCapsule degenerate inputs.
// ---------------------------------------------------------------------------

/// Rows 15-19 — zero-length segment (`dot(n,n) == 0`, the division-by-zero
/// candidate), `da / dot(n,n)` overflow, negative radii, NaN anywhere, and
/// infinite endpoints.
#[test]
fn rows15_19_circle_capsule_degenerate() {
    let p = load();
    let mut rng = Rng::default_seeded();
    let check = |a: C2Circle, b: C2Capsule, what: &str| -> c_int {
        let (c, rs) = unsafe {
            (
                (p.c.c2CircletoCapsule)(a, b),
                (p.rs.c2CircletoCapsule)(a, b),
            )
        };
        assert_int_eq(what, c, rs);
        c
    };

    // Row 15 — degenerate segment a == b, so n == (0,0) and dot(n,n) == 0.
    for _ in 0..3000 {
        let q = rng.vec_coord();
        let cap = C2Capsule { a: q, b: q, r: rng.radius() };
        check(rng.circle(), cap, "degenerate segment");
        check(C2Circle { p: q, r: rng.radius() }, cap, "degenerate coincident");
        // With a NaN/inf mixed in, `da >= 0 && db < 0` can become reachable.
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let cap2 = C2Capsule { a: q, b: q, r: bad };
            check(rng.circle(), cap2, "degenerate segment bad r");
            let cap3 = C2Capsule { a: v(bad, q.y), b: v(bad, q.y), r: rng.radius() };
            check(rng.circle(), cap3, "degenerate segment bad endpoint");
        }
    }

    // Row 16 — tiny n so dot(n,n) underflows and da/dot(n,n) overflows.
    for &t in &[1.0e-45f32, 1.0e-44, f32::MIN_POSITIVE, 1.0e-30, 1.0e-22] {
        for _ in 0..400 {
            let q = rng.vec_coord();
            let cap = C2Capsule { a: q, b: v(q.x + t, q.y), r: rng.radius() };
            // Place the circle to the side so `da >= 0 && db < 0` holds and the
            // division is actually performed.
            let circ = C2Circle { p: v(q.x + t * 0.5, q.y + rng.range(-10.0, 10.0)), r: rng.radius() };
            check(circ, cap, &format!("capsule overflow div t={t:?}"));
            check(rng.circle(), cap, &format!("capsule tiny t={t:?}"));
        }
    }

    // Row 17 — both radii negative.
    for _ in 0..2000 {
        let a = C2Circle { p: rng.vec_coord(), r: -rng.range(0.0, 50.0) };
        let cap = C2Capsule { a: rng.vec_coord(), b: rng.vec_coord(), r: -rng.range(0.0, 50.0) };
        check(a, cap, "capsule neg radii");
    }

    // Row 18 — NaN in each of the nine scalar slots.
    let nan = f32::NAN;
    for slot in 0..9 {
        for _ in 0..400 {
            let mut a = rng.circle();
            let mut cap = rng.capsule();
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => cap.a.x = nan,
                4 => cap.a.y = nan,
                5 => cap.b.x = nan,
                6 => cap.b.y = nan,
                7 => cap.r = nan,
                _ => {
                    a.p.x = nan;
                    cap.r = nan;
                }
            }
            let res = check(a, cap, &format!("capsule nan slot {slot}"));
            // Only SOME NaN placements force a rejection. With NaN in `B.a`
            // (slots 3/4) both `da < 0` and `db < 0` are false, so the C takes
            // the endpoint-`b` arm, whose `bp = A.p - B.b` never touches `B.a`
            // and stays finite — so a NaN in `B.a` can still report a hit.
            if !matches!(slot, 3 | 4) {
                assert_eq!(res, 0, "NaN must make the final `<` false (slot {slot})");
            }
        }
    }

    // Row 19 — infinite endpoints -> inf - inf = NaN in c2Sub.
    for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..400 {
            let cap = C2Capsule { a: v(s, s), b: v(s, s), r: rng.radius() };
            check(rng.circle(), cap, "capsule inf endpoints equal");
            let cap2 = C2Capsule { a: v(s, s), b: v(-s, -s), r: rng.radius() };
            check(rng.circle(), cap2, "capsule inf endpoints opposite");
            let a = C2Circle { p: v(s, -s), r: rng.radius() };
            check(a, rng.capsule(), "capsule inf circle");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 20-25 — primitive degenerate arithmetic.
// ---------------------------------------------------------------------------

/// Rows 20 + 21 — `c2Dot` / `c2Mulvs` with `inf * 0` and `inf + (-inf)`, i.e.
/// NaN produced by the operation rather than supplied as input; the resulting
/// NaN bit pattern (sign included) must match.
#[test]
fn rows20_21_nan_producing_arithmetic() {
    let p = load();
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    let specials = [0.0f32, -0.0, inf, ninf, f32::NAN, -f32::NAN, 1.0, -1.0];

    for &ax in &specials {
        for &ay in &specials {
            for &bx in &specials {
                for &by in &specials {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    assert_f32_bits_eq(
                        &format!("c2Dot special(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        unsafe { (p.c.c2Dot)(a, b) },
                        unsafe { (p.rs.c2Dot)(a, b) },
                    );
                    assert_v_bits_eq(
                        &format!("c2Mulvs special(({ax:?},{ay:?}) * {bx:?})"),
                        unsafe { (p.c.c2Mulvs)(a, bx) },
                        unsafe { (p.rs.c2Mulvs)(a, bx) },
                    );
                    assert_v_bits_eq(
                        &format!("c2Sub special(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        unsafe { (p.c.c2Sub)(a, b) },
                        unsafe { (p.rs.c2Sub)(a, b) },
                    );
                }
            }
        }
    }

    // inf + (-inf) in the dot product's sum: x term +inf, y term -inf.
    let cases = [
        (v(inf, inf), v(1.0, -1.0)),
        (v(inf, ninf), v(1.0, 1.0)),
        (v(ninf, inf), v(1.0, 1.0)),
        (v(inf, 1.0), v(1.0, ninf)),
    ];
    for (a, b) in cases {
        assert_f32_bits_eq(
            "c2Dot inf + -inf",
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.rs.c2Dot)(a, b) },
        );
    }
}

/// Rows 22 + 23 — `c2Maxv` / `c2Minv` NaN and signed-zero behaviour: the bare
/// ternary returns the SECOND operand whenever the comparison is false, so the
/// result is neither `fmax`/`fmin` nor NaN-quieting.
#[test]
fn rows22_23_maxv_minv_nan_and_signed_zero() {
    let p = load();
    let vals = [
        0.0f32,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xffc0_1234), // negative quiet NaN with payload
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];

    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    assert_v_bits_eq(
                        &format!("c2Maxv(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        unsafe { (p.c.c2Maxv)(a, b) },
                        unsafe { (p.rs.c2Maxv)(a, b) },
                    );
                    assert_v_bits_eq(
                        &format!("c2Minv(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        unsafe { (p.c.c2Minv)(a, b) },
                        unsafe { (p.rs.c2Minv)(a, b) },
                    );
                }
            }
        }
    }

    // The specific documented quirk: equal operands (incl. +0 vs -0) select `b`.
    for &(ax, bx) in &[(0.0f32, -0.0f32), (-0.0, 0.0), (1.0, 1.0), (-0.0, -0.0)] {
        let a = v(ax, ax);
        let b = v(bx, bx);
        let c_max = unsafe { (p.c.c2Maxv)(a, b) };
        assert_v_bits_eq("maxv equal picks b", c_max, unsafe { (p.rs.c2Maxv)(a, b) });
        assert_eq!(
            bits_v(c_max),
            bits_v(b),
            "C did not return the second operand for equal inputs"
        );
    }
}

/// Rows 24 + 25 — `c2Clampv` with contradictory bounds (`lo > hi`) and NaN in
/// any slot. No validation exists, so the result is whatever the nested
/// ternaries select; both libraries must agree bit-for-bit.
#[test]
fn rows24_25_clampv_contradictory_and_nan() {
    let p = load();
    let mut rng = Rng::default_seeded();

    // Contradictory bounds: result must equal `lo` for finite inputs.
    for _ in 0..N {
        let s = rng.aabb_sorted();
        let a = rng.vec_coord();
        let (lo, hi) = (s.max, s.min); // deliberately inverted
        let c = unsafe { (p.c.c2Clampv)(a, lo, hi) };
        let rs = unsafe { (p.rs.c2Clampv)(a, lo, hi) };
        assert_v_bits_eq("clampv inverted", c, rs);
    }

    let vals = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::from_bits(0xffc0_00ff),
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &a0 in &vals {
        for &l0 in &vals {
            for &h0 in &vals {
                let a = v(a0, l0);
                let lo = v(l0, h0);
                let hi = v(h0, a0);
                assert_v_bits_eq(
                    &format!("clampv nan({a0:?},{l0:?},{h0:?})"),
                    unsafe { (p.c.c2Clampv)(a, lo, hi) },
                    unsafe { (p.rs.c2Clampv)(a, lo, hi) },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 26-29 — top-level entry point and c2V.
// ---------------------------------------------------------------------------

/// Rows 26-28 — `circle_collide` with NaN (must return 0), with negative /
/// infinite arguments, and with a zero / negative-zero radius.
#[test]
fn rows26_28_circle_collide_invalid_scalars() {
    let p = load();
    let mut rng = Rng::default_seeded();

    // Row 26 — NaN in any argument makes every predicate false.
    let nan = f32::NAN;
    for slot in 0..3 {
        for _ in 0..1000 {
            let mut arg = [rng.range(-100.0, 100.0), rng.range(-100.0, 100.0), rng.range(0.0, 50.0)];
            arg[slot] = nan;
            let (c, rs) = unsafe {
                (
                    (p.c.circle_collide)(arg[0], arg[1], arg[2]),
                    (p.rs.circle_collide)(arg[0], arg[1], arg[2]),
                )
            };
            assert_int_eq(&format!("circle_collide nan slot {slot}"), c, rs);
            assert_eq!(c, 0, "NaN argument must yield 0 (slot {slot})");
        }
    }

    // Row 27 — negative / infinite arguments.
    for &r in &[-1.0f32, -100.0, f32::INFINITY, f32::NEG_INFINITY, 1.0e30, f32::MAX] {
        for &x in &[0.0f32, -70.0, f32::INFINITY, f32::NEG_INFINITY, 1.0e30] {
            for &y in &[0.0f32, 70.0, f32::INFINITY, f32::NEG_INFINITY, -1.0e30] {
                let (c, rs) =
                    unsafe { ((p.c.circle_collide)(x, y, r), (p.rs.circle_collide)(x, y, r)) };
                assert_int_eq(&format!("circle_collide extreme({x:?},{y:?},{r:?})"), c, rs);
            }
        }
    }

    // Row 28 — zero radius: the AABB bit can never be set (r2 == 0).
    for &r in &[0.0f32, -0.0] {
        for _ in 0..2000 {
            let x = rng.range(-120.0, 40.0);
            let y = rng.range(-120.0, 140.0);
            let (c, rs) = unsafe { ((p.c.circle_collide)(x, y, r), (p.rs.circle_collide)(x, y, r)) };
            assert_int_eq(&format!("circle_collide r={r:?}"), c, rs);
            assert_eq!(c & 2, 0, "zero radius must never set the AABB bit");
        }
    }
}

/// Row 29 — `c2V` stores its arguments verbatim: signalling NaNs, denormals and
/// signed zeros must not be normalised or quieted by either library.
#[test]
fn row29_c2v_verbatim_bit_patterns() {
    let p = load();

    let patterns: Vec<u32> = vec![
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // smallest denormal
        0x8000_0001,
        0x007f_ffff, // largest denormal
        0x7f7f_ffff, // FLT_MAX
        0x7f80_0000, // +inf
        0xff80_0000, // -inf
        0x7f80_0001, // signalling NaN, minimal payload
        0xff80_0001,
        0x7fbf_ffff, // signalling NaN, maximal payload
        0x7fc0_0000, // canonical quiet NaN
        0xffc0_0000,
        0x7fff_ffff, // quiet NaN, maximal payload
        0xffff_ffff,
        0x3f80_0000, // 1.0
        0xbf80_0000, // -1.0
    ];

    for &bx in &patterns {
        for &by in &patterns {
            let (x, y) = (f32::from_bits(bx), f32::from_bits(by));
            let c = unsafe { (p.c.c2V)(x, y) };
            let rs = unsafe { (p.rs.c2V)(x, y) };
            assert_v_bits_eq(&format!("c2V(0x{bx:08x}, 0x{by:08x})"), c, rs);
            assert_eq!(
                bits_v(c),
                (bx, by),
                "C c2V did not store the bits verbatim"
            );
            assert_eq!(
                bits_v(rs),
                (bx, by),
                "Rust c2V did not store the bits verbatim"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30 — struct tail padding / trap representations.
// ---------------------------------------------------------------------------

/// Row 30 — `c2Circle` (12 B) and `c2Capsule` (20 B, MEMORY class) passed with
/// arbitrary garbage in the bytes beyond the declared fields. Neither library
/// may read them: verified by running the same logical value with two different
/// garbage fills and requiring an identical result.
#[test]
fn row30_struct_tail_padding_ignored() {
    let p = load();
    let mut rng = Rng::default_seeded();

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CircleWithTail {
        c: C2Circle,
        tail: [u32; 5],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CapsuleWithTail {
        c: C2Capsule,
        tail: [u32; 5],
    }

    for _ in 0..N {
        let circle = rng.circle();
        let capsule = rng.capsule();

        let mut c1 = CircleWithTail { c: circle, tail: [0; 5] };
        let mut c2 = CircleWithTail { c: circle, tail: [0; 5] };
        for i in 0..5 {
            c1.tail[i] = rng.next_u32();
            c2.tail[i] = rng.next_u32();
        }
        let mut k1 = CapsuleWithTail { c: capsule, tail: [0; 5] };
        let mut k2 = CapsuleWithTail { c: capsule, tail: [0; 5] };
        for i in 0..5 {
            k1.tail[i] = rng.next_u32();
            k2.tail[i] = rng.next_u32();
        }

        // By-value calls: the struct is copied out of a buffer whose tail
        // differs, so any read past the declared fields would show up here.
        let a1 = unsafe { (p.c.c2CircletoCircle)(c1.c, c2.c) };
        let a2 = unsafe { (p.rs.c2CircletoCircle)(c1.c, c2.c) };
        assert_int_eq("padded circle-circle", a1, a2);

        let b1 = unsafe { (p.c.c2CircletoCapsule)(c1.c, k1.c) };
        let b2 = unsafe { (p.rs.c2CircletoCapsule)(c1.c, k1.c) };
        let b3 = unsafe { (p.c.c2CircletoCapsule)(c2.c, k2.c) };
        let b4 = unsafe { (p.rs.c2CircletoCapsule)(c2.c, k2.c) };
        assert_int_eq("padded circle-capsule 1", b1, b2);
        assert_int_eq("padded circle-capsule 2", b3, b4);
        assert_eq!(b1, b3, "C result depended on tail garbage");
        assert_eq!(b2, b4, "Rust result depended on tail garbage");

        // Pointer calls through c2Collided: `B` deliberately points at a
        // capsule followed by garbage.
        let ap = &c1 as *const _ as *const c_void;
        let kp1 = &k1 as *const _ as *const c_void;
        let kp2 = &k2 as *const _ as *const c_void;
        let d1 = unsafe { (p.c.c2Collided)(ap, kp1, C2_TYPE_CAPSULE) };
        let d2 = unsafe { (p.rs.c2Collided)(ap, kp1, C2_TYPE_CAPSULE) };
        let d3 = unsafe { (p.c.c2Collided)(ap, kp2, C2_TYPE_CAPSULE) };
        let d4 = unsafe { (p.rs.c2Collided)(ap, kp2, C2_TYPE_CAPSULE) };
        assert_int_eq("padded collided 1", d1, d2);
        assert_int_eq("padded collided 2", d3, d4);
        assert_eq!(d1, d3);
        assert_eq!(d2, d4);
    }
}
