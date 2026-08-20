//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library has no error codes; its whole rejection surface is
//!   * `c2Collided`'s `default: return 0;` for out-of-range `C2_TYPE` values, and
//!   * the strict `d2 < r2` predicates returning `0`.
//! Each test asserts that BOTH `.so`s produce the *same* rejection value, not
//! merely that both "failed somehow".

mod common;

use common::*;
use std::ffi::c_int;
use std::os::raw::c_void;

/// A valid `A` (always read as `c2Circle`) and a valid `B` of each shape, so an
/// out-of-range tag is the *only* thing wrong with the call.
struct Fixture {
    a: C2Circle,
    circle: C2Circle,
    aabb: C2Aabb,
    capsule: C2Capsule,
}

impl Fixture {
    fn new() -> Fixture {
        Fixture {
            a: C2Circle {
                p: C2v::new(1.0, 2.0),
                r: 5.0,
            },
            circle: C2Circle {
                p: C2v::new(2.0, 3.0),
                r: 4.0,
            },
            aabb: C2Aabb {
                min: C2v::new(-1.0, -1.0),
                max: C2v::new(1.0, 1.0),
            },
            capsule: C2Capsule {
                a: C2v::new(-5.0, 0.0),
                b: C2v::new(5.0, 0.0),
                r: 2.0,
            },
        }
    }
    fn pa(&self) -> *const c_void {
        &self.a as *const C2Circle as *const c_void
    }
    fn bs(&self) -> [(&'static str, *const c_void); 3] {
        [
            ("circle", &self.circle as *const C2Circle as *const c_void),
            ("aabb", &self.aabb as *const C2Aabb as *const c_void),
            ("capsule", &self.capsule as *const C2Capsule as *const c_void),
        ]
    }
}

/// Assert both libraries return the same value for `c2Collided(A, B, tag)` and,
/// when the tag is out of range, that the value is the documented `0` sentinel.
#[track_caller]
fn check_tag(f: &Fixture, tag: c_int) {
    let (c, r) = libs();
    for (label, pb) in f.bs() {
        let (cr, rr) = unsafe { ((c.c2Collided)(f.pa(), pb, tag), (r.c2Collided)(f.pa(), pb, tag)) };
        assert_int(cr, rr, &format!("c2Collided(B={label}, typeB={tag})"));
        if !(0..=2).contains(&tag) {
            assert_eq!(
                cr, 0,
                "C must reject out-of-range typeB={tag} with the 0 sentinel (B={label}), got {cr}"
            );
            assert_eq!(
                rr, 0,
                "Rust must reject out-of-range typeB={tag} with the 0 sentinel (B={label}), got {rr}"
            );
        }
    }
}

// ===========================================================================
// E1..E5 — c2Collided out-of-range C2_TYPE (the only explicit rejection in C)
// ===========================================================================

#[test]
fn e1_collided_type_one_past_end() {
    check_tag(&Fixture::new(), 3);
}

#[test]
fn e2_collided_type_negative() {
    let f = Fixture::new();
    for tag in [-1, -2, -3] {
        check_tag(&f, tag);
    }
}

#[test]
fn e3_collided_type_int_extremes() {
    let f = Fixture::new();
    for tag in [
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 3,
        i32::MIN.wrapping_add(2),
        0x0080_0000,
        -0x0080_0000,
    ] {
        check_tag(&f, tag);
    }
}

#[test]
fn e4_collided_type_sweep() {
    let f = Fixture::new();
    // Exhaustive over a wide window...
    for tag in -4096..=4096 {
        if (0..=2).contains(&tag) {
            continue;
        }
        check_tag(&f, tag);
    }
    // ...plus randomized 32-bit values.
    let mut rng = Rng::new(104);
    for _ in 0..20_000 {
        let tag = rng.next_i32();
        if (0..=2).contains(&tag) {
            continue;
        }
        check_tag(&f, tag);
    }
}

#[test]
fn e5_collided_type_alias_low_byte() {
    let f = Fixture::new();
    // Out-of-range values whose low byte / low 16 bits alias a valid tag: the C
    // `switch` compares the full `int`, so these must NOT reach a valid arm.
    for tag in [
        0x0000_0100,
        0x0000_0101,
        0x0000_0102,
        0x0001_0000,
        0x0001_0001,
        0x0001_0002,
        0x7FFF_FF00u32 as i32,
        0x7FFF_FF01u32 as i32,
        0x7FFF_FF02u32 as i32,
        -0x0000_0100,
        0x0000_FF00,
        0x0000_FF01,
        0x0000_FF02,
        0x8000_0000u32 as i32,
        0x8000_0001u32 as i32,
        0x8000_0002u32 as i32,
    ] {
        check_tag(&f, tag);
    }
}

// ===========================================================================
// E6..E11 — c2CircletoCircle rejections
// ===========================================================================

#[test]
fn e6_circle_separated() {
    let (c, r) = libs();
    let mut rng = Rng::new(106);
    for _ in 0..20_000 {
        let ar = rng.unit() * 30.0;
        let br = rng.unit() * 30.0;
        let ang = rng.unit() * std::f32::consts::TAU;
        // Strictly farther apart than the radius sum.
        let d = (ar + br) * (1.001 + rng.unit()) + 0.001;
        let ap = rng.tame_v(50.0);
        let a = C2Circle { p: ap, r: ar };
        let b = C2Circle {
            p: C2v::new(ap.x + d * ang.cos(), ap.y + d * ang.sin()),
            r: br,
        };
        let cr = (c.c2CircletoCircle)(a, b);
        assert_int(cr, (r.c2CircletoCircle)(a, b), &format!("{a:?} {b:?}"));
        assert_eq!(cr, 0, "separated circles must be rejected: {a:?} {b:?}");
    }
}

#[test]
fn e7_circle_exactly_tangent() {
    let (c, r) = libs();
    // Exactly representable integer geometry so d2 == r2 bit-exactly.
    // `<` is strict, so tangency must be rejected.
    let mut checked = 0;
    for ar in 0..40u32 {
        for br in 0..40u32 {
            let (arf, brf) = (ar as f32, br as f32);
            let d = arf + brf;
            for (dx, dy) in [(d, 0.0f32), (0.0f32, d), (-d, 0.0f32), (0.0f32, -d)] {
                let a = C2Circle {
                    p: C2v::new(0.0, 0.0),
                    r: arf,
                };
                let b = C2Circle {
                    p: C2v::new(dx, dy),
                    r: brf,
                };
                let cr = (c.c2CircletoCircle)(a, b);
                assert_int(cr, (r.c2CircletoCircle)(a, b), &format!("{a:?} {b:?}"));
                assert_eq!(cr, 0, "tangent circles must be rejected: {a:?} {b:?}");
                checked += 1;
            }
        }
    }
    // 3-4-5 triangles: d2 == r2 with a non-axis-aligned offset.
    for k in 1..40u32 {
        let (kx, ky) = ((3 * k) as f32, (4 * k) as f32);
        let sum = (5 * k) as f32;
        for i in 0..=(5 * k) {
            let arf = i as f32;
            let brf = sum - arf;
            let a = C2Circle {
                p: C2v::new(0.0, 0.0),
                r: arf,
            };
            let b = C2Circle {
                p: C2v::new(kx, ky),
                r: brf,
            };
            let cr = (c.c2CircletoCircle)(a, b);
            assert_int(cr, (r.c2CircletoCircle)(a, b), &format!("{a:?} {b:?}"));
            assert_eq!(cr, 0, "tangent circles must be rejected: {a:?} {b:?}");
            checked += 1;
        }
    }
    assert!(checked > 1_000);
}

#[test]
fn e8_circle_negative_radii() {
    let (c, r) = libs();
    let mut rng = Rng::new(108);
    for _ in 0..20_000 {
        let p = rng.tame_v(60.0);
        let q = rng.tame_v(60.0);
        let ar = rng.tame_f32(40.0);
        let br = rng.tame_f32(40.0);
        for (sa, sb) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let a = C2Circle { p, r: ar * sa };
            let b = C2Circle { p: q, r: br * sb };
            assert_int(
                (c.c2CircletoCircle)(a, b),
                (r.c2CircletoCircle)(a, b),
                &format!("negative radii {a:?} {b:?}"),
            );
        }
        // The C quirk: r2 = (A.r+B.r)^2, so negating BOTH radii is identical.
        let pos = (c.c2CircletoCircle)(
            C2Circle { p, r: ar },
            C2Circle {
                p: q,
                r: br,
            },
        );
        let neg = (c.c2CircletoCircle)(
            C2Circle { p, r: -ar },
            C2Circle {
                p: q,
                r: -br,
            },
        );
        assert_eq!(pos, neg, "C: negating both radii must not change the result");
        assert_eq!(
            neg,
            (r.c2CircletoCircle)(
                C2Circle { p, r: -ar },
                C2Circle { p: q, r: -br }
            ),
            "Rust must reproduce the negative-radius quirk"
        );
    }
}

#[test]
fn e9_circle_zero_radius_sum() {
    let (c, r) = libs();
    let mut rng = Rng::new(109);
    for _ in 0..20_000 {
        let p = rng.tame_v(60.0);
        let q = if rng.below(3) == 0 { p } else { rng.tame_v(60.0) };
        let k = rng.tame_f32(40.0);
        for (ar, br) in [(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0), (k, -k)] {
            let a = C2Circle { p, r: ar };
            let b = C2Circle { p: q, r: br };
            let cr = (c.c2CircletoCircle)(a, b);
            assert_int(cr, (r.c2CircletoCircle)(a, b), &format!("{a:?} {b:?}"));
            if !k.is_nan() {
                assert_eq!(
                    cr, 0,
                    "r2 == 0 means d2 < 0 is impossible, must reject: {a:?} {b:?}"
                );
            }
        }
    }
}

#[test]
fn e10_circle_nan_rejects() {
    let (c, r) = libs();
    let mut rng = Rng::new(110);
    for &nan in NANS.iter() {
        for slot in 0..6 {
            for _ in 0..500 {
                let mut a = C2Circle {
                    p: rng.tame_v(60.0),
                    r: rng.unit() * 30.0,
                };
                let mut b = C2Circle {
                    p: rng.tame_v(60.0),
                    r: rng.unit() * 30.0,
                };
                match slot {
                    0 => a.p.x = nan,
                    1 => a.p.y = nan,
                    2 => a.r = nan,
                    3 => b.p.x = nan,
                    4 => b.p.y = nan,
                    _ => b.r = nan,
                }
                let cr = (c.c2CircletoCircle)(a, b);
                assert_int(cr, (r.c2CircletoCircle)(a, b), &format!("{a:?} {b:?}"));
                assert_eq!(cr, 0, "NaN input must reject (slot {slot}): {a:?} {b:?}");
            }
        }
    }
}

#[test]
fn e11_circle_infinities() {
    let (c, r) = libs();
    let infs: &[f32] = &[f32::INFINITY, f32::NEG_INFINITY];
    let vals: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        20.0,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &i in infs {
        for slot in 0..6usize {
            for &v0 in vals {
                for &v1 in vals {
                    let mut a = C2Circle {
                        p: C2v::new(v0, v1),
                        r: v0,
                    };
                    let mut b = C2Circle {
                        p: C2v::new(v1, v0),
                        r: v1,
                    };
                    match slot {
                        0 => a.p.x = i,
                        1 => a.p.y = i,
                        2 => a.r = i,
                        3 => b.p.x = i,
                        4 => b.p.y = i,
                        _ => b.r = i,
                    }
                    assert_int(
                        (c.c2CircletoCircle)(a, b),
                        (r.c2CircletoCircle)(a, b),
                        &format!("inf slot {slot}: {a:?} {b:?}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E12..E16 — c2CircletoAABB rejections
// ===========================================================================

#[test]
fn e12_aabb_outside() {
    let (c, r) = libs();
    let mut rng = Rng::new(112);
    for _ in 0..20_000 {
        let min = rng.tame_v(50.0);
        let max = C2v::new(min.x + rng.unit() * 40.0 + 1.0, min.y + rng.unit() * 40.0 + 1.0);
        let bb = C2Aabb { min, max };
        let rad = rng.unit() * 10.0;
        // Push the centre strictly farther than `rad` from the nearest face.
        let slack = rad * 1.5 + 1.0;
        let p = match rng.below(4) {
            0 => C2v::new(max.x + slack, rng.sym(60.0)),
            1 => C2v::new(min.x - slack, rng.sym(60.0)),
            2 => C2v::new(rng.sym(60.0), max.y + slack),
            _ => C2v::new(rng.sym(60.0), min.y - slack),
        };
        let a = C2Circle { p, r: rad };
        let cr = (c.c2CircletoAABB)(a, bb);
        assert_int(cr, (r.c2CircletoAABB)(a, bb), &format!("{a:?} {bb:?}"));
        assert_eq!(cr, 0, "circle outside the box must be rejected: {a:?} {bb:?}");
    }
}

#[test]
fn e13_aabb_zero_radius() {
    let (c, r) = libs();
    let mut rng = Rng::new(113);
    for _ in 0..20_000 {
        let min = rng.tame_v(50.0);
        let max = C2v::new(min.x + rng.unit() * 40.0, min.y + rng.unit() * 40.0);
        let bb = C2Aabb { min, max };
        // Centre inside the box (d2 == 0) — still rejected because r2 == 0.
        let p = C2v::new(
            min.x + (max.x - min.x) * rng.unit(),
            min.y + (max.y - min.y) * rng.unit(),
        );
        for rad in [0.0f32, -0.0] {
            let a = C2Circle { p, r: rad };
            let cr = (c.c2CircletoAABB)(a, bb);
            assert_int(cr, (r.c2CircletoAABB)(a, bb), &format!("{a:?} {bb:?}"));
            assert_eq!(
                cr, 0,
                "zero radius must reject even inside the box: {a:?} {bb:?}"
            );
        }
    }
}

#[test]
fn e14_aabb_inverted_box() {
    let (c, r) = libs();
    let mut rng = Rng::new(114);
    for _ in 0..20_000 {
        let lo = rng.tame_v(50.0);
        let hi = C2v::new(lo.x + rng.unit() * 40.0, lo.y + rng.unit() * 40.0);
        // Deliberately swapped: min > max. C performs no validation.
        let bb = C2Aabb { min: hi, max: lo };
        let a = C2Circle {
            p: rng.tame_v(80.0),
            r: rng.unit() * 30.0,
        };
        assert_int(
            (c.c2CircletoAABB)(a, bb),
            (r.c2CircletoAABB)(a, bb),
            &format!("inverted box {a:?} {bb:?}"),
        );
        // The clamp itself must agree bit-for-bit: max(lo, min(a, hi)).
        assert_v_bits(
            (c.c2Clampv)(a.p, bb.min, bb.max),
            (r.c2Clampv)(a.p, bb.min, bb.max),
            &format!("inverted clamp {a:?} {bb:?}"),
        );
    }
}

#[test]
fn e15_aabb_negative_radius() {
    let (c, r) = libs();
    let mut rng = Rng::new(115);
    for _ in 0..20_000 {
        let min = rng.tame_v(50.0);
        let max = C2v::new(min.x + rng.unit() * 40.0, min.y + rng.unit() * 40.0);
        let bb = C2Aabb { min, max };
        let p = rng.tame_v(80.0);
        let rad = rng.unit() * 30.0;
        let pos = C2Circle { p, r: rad };
        let neg = C2Circle { p, r: -rad };
        let cp = (c.c2CircletoAABB)(pos, bb);
        let cn = (c.c2CircletoAABB)(neg, bb);
        assert_int(cp, (r.c2CircletoAABB)(pos, bb), &format!("{pos:?} {bb:?}"));
        assert_int(cn, (r.c2CircletoAABB)(neg, bb), &format!("{neg:?} {bb:?}"));
        // C quirk: r2 = A.r*A.r, so the sign of the radius is irrelevant.
        assert_eq!(cp, cn, "C: r2 = A.r*A.r ignores the sign of A.r");
    }
}

#[test]
fn e16_aabb_nan_rejects() {
    let (c, r) = libs();
    let mut rng = Rng::new(116);
    // Verified against the compiled C library: a NaN in `A.p` or `A.r` poisons
    // `d2`/`r2` and always rejects, but a NaN in the *bounds* is silently
    // DISCARDED by the `?:` clamp (`NaN > v` is false, so `c2Maxv` returns the
    // second operand), so such a call can still report a collision. Both
    // behaviours must be reproduced exactly; only the poisoning slots may be
    // asserted to reject.
    let mut bound_nan_outcomes = Cover::new("aabb NaN-bound outcome", &["hit", "miss"]);
    for &nan in NANS.iter() {
        for slot in 0..5usize {
            for _ in 0..800 {
                let min = rng.tame_v(50.0);
                let mut bb = C2Aabb {
                    min,
                    max: C2v::new(min.x + rng.unit() * 40.0, min.y + rng.unit() * 40.0),
                };
                let mut a = C2Circle {
                    p: rng.tame_v(80.0),
                    r: rng.unit() * 30.0,
                };
                match slot {
                    0 => a.p.x = nan,
                    1 => a.r = nan,
                    2 => bb.min.x = nan,
                    3 => bb.max.y = nan,
                    _ => {
                        bb.min.y = nan;
                        bb.max.x = nan;
                    }
                }
                let cr = (c.c2CircletoAABB)(a, bb);
                assert_int(cr, (r.c2CircletoAABB)(a, bb), &format!("{a:?} {bb:?}"));
                if slot <= 1 {
                    // NaN in A.p.x / A.r reaches d2 / r2 unavoidably.
                    assert_eq!(cr, 0, "NaN in A must reject (slot {slot}): {a:?} {bb:?}");
                } else {
                    bound_nan_outcomes.hit(if cr != 0 { "hit" } else { "miss" });
                }
                // The `?:` clamp with a NaN bound picks the *second* operand,
                // which is a different result than f32::min/max would give.
                assert_v_bits(
                    (c.c2Clampv)(a.p, bb.min, bb.max),
                    (r.c2Clampv)(a.p, bb.min, bb.max),
                    &format!("NaN clamp slot {slot}: {a:?} {bb:?}"),
                );
            }
        }
    }
    // Prove both sides of the NaN-bound behaviour were actually exercised.
    bound_nan_outcomes.require_all(1);
}

// ===========================================================================
// E17..E24 — c2CircletoCapsule rejections
// ===========================================================================

/// Reproduce the C arm selection using the C library's own primitives.
fn arm_of(api: &Api, a: C2Circle, b: C2Capsule) -> u8 {
    let n = (api.c2Sub)(b.b, b.a);
    let ap = (api.c2Sub)(a.p, b.a);
    if (api.c2Dot)(ap, n) < 0.0 {
        return 1;
    }
    if (api.c2Dot)((api.c2Sub)(a.p, b.b), n) < 0.0 {
        2
    } else {
        3
    }
}

fn capsule_miss_in_arm(arm: u8, stream: u64) {
    let (c, r) = libs();
    let mut rng = Rng::new(stream);
    let mut hits = 0usize;
    for _ in 0..300_000 {
        let a0 = rng.tame_v(60.0);
        let len = rng.unit() * 80.0 + 1.0;
        let ang = rng.unit() * std::f32::consts::TAU;
        let b0 = C2v::new(a0.x + len * ang.cos(), a0.y + len * ang.sin());
        let cap = C2Capsule {
            a: a0,
            b: b0,
            r: rng.unit() * 8.0,
        };
        // Far-away query point so `d2 >= r*r` in whichever arm is selected.
        let t = match arm {
            1 => -(rng.unit() * 1.5 + 0.2),
            2 => rng.unit(),
            _ => rng.unit() * 1.5 + 1.2,
        };
        let dirx = b0.x - a0.x;
        let diry = b0.y - a0.y;
        let ilen = 1.0 / (dirx * dirx + diry * diry).sqrt();
        let (nx, ny) = (-diry * ilen, dirx * ilen);
        let off = (cap.r + 1.0) * (3.0 + rng.unit() * 20.0) * if rng.below(2) == 0 { 1.0 } else { -1.0 };
        let cir = C2Circle {
            p: C2v::new(a0.x + dirx * t + nx * off, a0.y + diry * t + ny * off),
            r: rng.unit() * 0.5,
        };
        if arm_of(c, cir, cap) != arm {
            continue;
        }
        let cr = (c.c2CircletoCapsule)(cir, cap);
        assert_int(cr, (r.c2CircletoCapsule)(cir, cap), &format!("{cir:?} {cap:?}"));
        assert_eq!(cr, 0, "arm {arm} miss must be rejected: {cir:?} {cap:?}");
        hits += 1;
        if hits >= 5_000 {
            break;
        }
    }
    assert!(hits >= 1_000, "only {hits} misses landed in arm {arm}");
}

#[test]
fn e17_capsule_before_a() {
    capsule_miss_in_arm(1, 117);
}

#[test]
fn e18_capsule_after_b() {
    capsule_miss_in_arm(3, 118);
}

#[test]
fn e19_capsule_middle() {
    capsule_miss_in_arm(2, 119);
}

#[test]
fn e20_capsule_degenerate_zero_length() {
    let (c, r) = libs();
    let mut rng = Rng::new(120);
    // a == b => n == (0,0) => c2Dot(n,n) == 0 => the middle arm divides by 0.
    for _ in 0..40_000 {
        let a0 = rng.tame_v(60.0);
        let cap = C2Capsule {
            a: a0,
            b: a0,
            r: [0.0f32, -0.0, 1.0, -1.0, 7.5, 1.0e30, 1.0e-30][rng.below(7) as usize],
        };
        let cir = C2Circle {
            p: match rng.below(4) {
                0 => a0,
                1 => C2v::new(a0.x + rng.sym(1.0e-6), a0.y),
                _ => rng.tame_v(90.0),
            },
            r: [0.0f32, -0.0, 1.0, -1.0, 5.0, 1.0e30][rng.below(6) as usize],
        };
        // Whatever the div-by-zero yields, both sides must agree exactly.
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("zero-length capsule {cir:?} {cap:?}"),
        );
        // `da == 0` (query point == a) makes it literally 0/0 = NaN.
        assert_eq!(
            arm_of(c, cir, cap),
            arm_of(r, cir, cap),
            "arm selection diverged for {cir:?} {cap:?}"
        );
    }
    // Exact 0/0: query point coincident with the degenerate segment.
    for &rad in &[0.0f32, 1.0, -1.0, 1.0e30] {
        let p = C2v::new(3.0, -4.0);
        let cap = C2Capsule { a: p, b: p, r: rad };
        let cir = C2Circle { p, r: rad };
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("0/0 capsule {cir:?} {cap:?}"),
        );
    }
}

#[test]
fn e21_capsule_zero_radius_sum() {
    let (c, r) = libs();
    let mut rng = Rng::new(121);
    for _ in 0..20_000 {
        let a0 = rng.tame_v(50.0);
        let b0 = C2v::new(a0.x + rng.tame_f32(40.0), a0.y + rng.tame_f32(40.0));
        let k = rng.tame_f32(20.0);
        for (ar, br) in [(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (k, -k)] {
            let cap = C2Capsule { a: a0, b: b0, r: br };
            // Query point exactly on the segment (t=0, t=1 and the midpoint).
            for p in [
                a0,
                b0,
                C2v::new((a0.x + b0.x) * 0.5, (a0.y + b0.y) * 0.5),
            ] {
                let cir = C2Circle { p, r: ar };
                let cr = (c.c2CircletoCapsule)(cir, cap);
                assert_int(
                    cr,
                    (r.c2CircletoCapsule)(cir, cap),
                    &format!("{cir:?} {cap:?}"),
                );
                if ar + br == 0.0 {
                    assert_eq!(
                        cr, 0,
                        "r*r == 0 must reject even on the segment: {cir:?} {cap:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn e22_capsule_negative_radii() {
    let (c, r) = libs();
    let mut rng = Rng::new(122);
    for _ in 0..20_000 {
        let a0 = rng.tame_v(50.0);
        let b0 = C2v::new(a0.x + rng.tame_f32(40.0), a0.y + rng.tame_f32(40.0));
        let ar = rng.unit() * 20.0;
        let br = rng.unit() * 20.0;
        let p = rng.tame_v(80.0);
        let pos = (
            C2Circle { p, r: ar },
            C2Capsule { a: a0, b: b0, r: br },
        );
        let neg = (
            C2Circle { p, r: -ar },
            C2Capsule { a: a0, b: b0, r: -br },
        );
        let cp = (c.c2CircletoCapsule)(pos.0, pos.1);
        let cn = (c.c2CircletoCapsule)(neg.0, neg.1);
        assert_int(cp, (r.c2CircletoCapsule)(pos.0, pos.1), "positive radii");
        assert_int(cn, (r.c2CircletoCapsule)(neg.0, neg.1), "negative radii");
        // C quirk: the predicate is d2 < r*r, so negating both radii is a no-op.
        assert_eq!(cp, cn, "C: negating both radii must not change the result");
    }
}

#[test]
fn e23_capsule_nan_rejects() {
    let (c, r) = libs();
    let mut rng = Rng::new(123);
    // Verified against the compiled C library: NaN in `A.p`, `A.r`, `B.b` or
    // `B.r` always reaches `d2`/`r` and rejects, but a NaN in `B.a` only
    // poisons `n` and `ap`; since `da`/`db` are then unordered, both `< 0`
    // tests fail and control lands in the *beyond-b* arm, whose `d2` is
    // computed from `A.p - B.b` alone — completely NaN-free. Such a call can
    // therefore still report a collision, and that must be reproduced.
    let mut a_nan_outcomes = Cover::new("capsule NaN-B.a outcome", &["hit", "miss"]);
    for &nan in NANS.iter() {
        for slot in 0..7usize {
            for _ in 0..600 {
                let a0 = rng.tame_v(50.0);
                let mut cap = C2Capsule {
                    a: a0,
                    b: C2v::new(a0.x + rng.tame_f32(40.0), a0.y + rng.tame_f32(40.0)),
                    r: rng.unit() * 15.0,
                };
                let mut cir = C2Circle {
                    p: rng.tame_v(80.0),
                    r: rng.unit() * 15.0,
                };
                match slot {
                    0 => cir.p.x = nan,
                    1 => cir.p.y = nan,
                    2 => cir.r = nan,
                    3 => cap.a.x = nan,
                    4 => cap.a.y = nan,
                    5 => cap.b.x = nan,
                    _ => cap.r = nan,
                }
                let cr = (c.c2CircletoCapsule)(cir, cap);
                assert_int(
                    cr,
                    (r.c2CircletoCapsule)(cir, cap),
                    &format!("NaN slot {slot}: {cir:?} {cap:?}"),
                );
                if slot == 3 || slot == 4 {
                    // NaN in B.a — may survive into the beyond-b arm.
                    a_nan_outcomes.hit(if cr != 0 { "hit" } else { "miss" });
                    assert_eq!(
                        arm_of(c, cir, cap),
                        3,
                        "NaN in B.a must force the beyond-b arm: {cir:?} {cap:?}"
                    );
                } else {
                    assert_eq!(
                        cr, 0,
                        "NaN input must reject (slot {slot}): {cir:?} {cap:?}"
                    );
                }
                assert_eq!(
                    arm_of(c, cir, cap),
                    arm_of(r, cir, cap),
                    "arm selection diverged (slot {slot}): {cir:?} {cap:?}"
                );
            }
        }
    }
    a_nan_outcomes.require_all(1);
}

#[test]
fn e24_capsule_infinities() {
    let (c, r) = libs();
    let vals: &[f32] = &[
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
    ];
    let mut rng = Rng::new(124);
    for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
        for slot in 0..7usize {
            for &v in vals {
                for _ in 0..40 {
                    let mut cap = C2Capsule {
                        a: C2v::new(v, rng.tame_f32(40.0)),
                        b: C2v::new(rng.tame_f32(40.0), v),
                        r: v,
                    };
                    let mut cir = C2Circle {
                        p: C2v::new(rng.tame_f32(40.0), v),
                        r: rng.tame_f32(20.0),
                    };
                    match slot {
                        0 => cir.p.x = i,
                        1 => cir.p.y = i,
                        2 => cir.r = i,
                        3 => cap.a.x = i,
                        4 => cap.a.y = i,
                        5 => cap.b.y = i,
                        _ => cap.r = i,
                    }
                    assert_int(
                        (c.c2CircletoCapsule)(cir, cap),
                        (r.c2CircletoCapsule)(cir, cap),
                        &format!("inf slot {slot}: {cir:?} {cap:?}"),
                    );
                    assert_eq!(
                        arm_of(c, cir, cap),
                        arm_of(r, cir, cap),
                        "arm selection diverged (inf slot {slot}): {cir:?} {cap:?}"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E25..E29 — primitive-level NaN / overflow / signed-zero behaviour
// ===========================================================================

#[test]
fn e25_dot_nan_payloads() {
    let (c, r) = libs();
    // Exhaustive over the NaN table in every one of the four slots.
    for &ax in NANS.iter() {
        for &bx in NANS.iter() {
            for &ay in NANS.iter() {
                for &by in NANS.iter() {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_f32_bits(
                        (c.c2Dot)(a, b),
                        (r.c2Dot)(a, b),
                        &format!("c2Dot all-NaN {} {}", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
    // Exactly one NaN slot at a time, with finite values elsewhere.
    let fin: &[f32] = &[0.0, -0.0, 1.0, -1.0, 3.5, -7.25, f32::MAX, f32::MIN_POSITIVE];
    for &nan in NANS.iter() {
        for slot in 0..4usize {
            for &f0 in fin {
                for &f1 in fin {
                    let mut a = C2v::new(f0, f1);
                    let mut b = C2v::new(f1, f0);
                    match slot {
                        0 => a.x = nan,
                        1 => a.y = nan,
                        2 => b.x = nan,
                        _ => b.y = nan,
                    }
                    assert_f32_bits(
                        (c.c2Dot)(a, b),
                        (r.c2Dot)(a, b),
                        &format!("c2Dot one-NaN slot {slot} {} {}", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(125);
    for _ in 0..40_000 {
        // Random NaN payloads mixed with random finite values.
        let pick = |rng: &mut Rng| -> f32 {
            match rng.below(3) {
                0 => f32::from_bits(0x7F80_0000 | (rng.next_u32() & 0x807F_FFFF)),
                1 => rng.bit_f32(),
                _ => rng.tame_f32(100.0),
            }
        };
        let a = C2v::new(pick(&mut rng), pick(&mut rng));
        let b = C2v::new(pick(&mut rng), pick(&mut rng));
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot random NaN {} {}", fmt_v(a), fmt_v(b)),
        );
    }
}

#[test]
fn e26_dot_overflow_to_inf() {
    let (c, r) = libs();
    let big: &[f32] = &[
        f32::MAX,
        -f32::MAX,
        1.0e38,
        -1.0e38,
        3.0e38,
        -3.0e38,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e19,
        -1.0e19,
    ];
    for &ax in big {
        for &bx in big {
            for &ay in big {
                for &by in big {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_f32_bits(
                        (c.c2Dot)(a, b),
                        (r.c2Dot)(a, b),
                        &format!("c2Dot overflow {} {}", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
    // Underflow to zero / subnormal too.
    let tiny: &[f32] = &[
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-45,
        -1.0e-45,
        1.0e-30,
        -1.0e-30,
        0.0,
        -0.0,
    ];
    for &ax in tiny {
        for &bx in tiny {
            for &ay in tiny {
                for &by in tiny {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_f32_bits(
                        (c.c2Dot)(a, b),
                        (r.c2Dot)(a, b),
                        &format!("c2Dot underflow {} {}", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
}

#[test]
fn e27_mulvs_nan_payloads() {
    let (c, r) = libs();
    // The C `mulss` keeps the DESTINATION (the vector component) payload, so
    // `a` wins over `b` when both are NaN.
    for &ax in NANS.iter() {
        for &ay in NANS.iter() {
            for &s in NANS.iter() {
                let a = C2v::new(ax, ay);
                let got_c = (c.c2Mulvs)(a, s);
                assert_v_bits(
                    got_c,
                    (r.c2Mulvs)(a, s),
                    &format!("c2Mulvs both-NaN {} {}", fmt_v(a), fmt_f32(s)),
                );
                // Sanity: it really is `a`'s payload (quieted), not `b`'s.
                let quiet = |v: f32| f32::from_bits(v.to_bits() | 0x0040_0000);
                assert_eq!(
                    got_c.x.to_bits(),
                    quiet(ax).to_bits(),
                    "C kept the wrong NaN for the x lane"
                );
            }
        }
    }
    // NaN in exactly one operand.
    let fin: &[f32] = &[0.0, -0.0, 1.0, -1.0, 2.5, f32::MAX, f32::INFINITY];
    for &nan in NANS.iter() {
        for &f in fin {
            for (a, s) in [
                (C2v::new(nan, f), f),
                (C2v::new(f, nan), f),
                (C2v::new(f, f), nan),
                (C2v::new(nan, nan), f),
            ] {
                assert_v_bits(
                    (c.c2Mulvs)(a, s),
                    (r.c2Mulvs)(a, s),
                    &format!("c2Mulvs one-NaN {} {}", fmt_v(a), fmt_f32(s)),
                );
            }
        }
    }
    // Inf * 0 = NaN in both lanes.
    for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &z in &[0.0f32, -0.0f32] {
            let a = C2v::new(i, -i);
            assert_v_bits(
                (c.c2Mulvs)(a, z),
                (r.c2Mulvs)(a, z),
                &format!("c2Mulvs inf*0 {} {}", fmt_v(a), fmt_f32(z)),
            );
            let a2 = C2v::new(z, z);
            assert_v_bits(
                (c.c2Mulvs)(a2, i),
                (r.c2Mulvs)(a2, i),
                &format!("c2Mulvs 0*inf {} {}", fmt_v(a2), fmt_f32(i)),
            );
        }
    }
}

#[test]
fn e28_sub_inf_and_nan() {
    let (c, r) = libs();
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    for &ax in pool.iter() {
        for &bx in pool.iter() {
            for &ay in pool.iter() {
                for &by in pool.iter() {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_v_bits(
                        (c.c2Sub)(a, b),
                        (r.c2Sub)(a, b),
                        &format!("c2Sub {} {}", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
}

#[test]
fn e29_minmax_nan_and_signed_zero() {
    let (c, r) = libs();
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    for &ax in pool.iter() {
        for &bx in pool.iter() {
            for &ay in pool.iter() {
                for &by in pool.iter() {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_v_bits(
                        (c.c2Minv)(a, b),
                        (r.c2Minv)(a, b),
                        &format!("c2Minv {} {}", fmt_v(a), fmt_v(b)),
                    );
                    assert_v_bits(
                        (c.c2Maxv)(a, b),
                        (r.c2Maxv)(a, b),
                        &format!("c2Maxv {} {}", fmt_v(a), fmt_v(b)),
                    );
                    assert_v_bits(
                        (c.c2Clampv)(a, b, a),
                        (r.c2Clampv)(a, b, a),
                        &format!("c2Clampv {} {} {}", fmt_v(a), fmt_v(b), fmt_v(a)),
                    );
                }
            }
        }
    }
    // The `?:` returns the SECOND operand on a tie: max(-0.0, +0.0) == +0.0 and
    // max(+0.0, -0.0) == -0.0, which f32::max would not reproduce.
    let z = C2v::new(0.0, 0.0);
    let nz = C2v::new(-0.0, -0.0);
    assert_v_bits((c.c2Maxv)(nz, z), (r.c2Maxv)(nz, z), "max(-0,+0)");
    assert_v_bits((c.c2Maxv)(z, nz), (r.c2Maxv)(z, nz), "max(+0,-0)");
    assert_eq!(
        (c.c2Maxv)(z, nz).bits(),
        nz.bits(),
        "C: `?:` must pick the second operand on a tie"
    );
    assert_v_bits((c.c2Minv)(nz, z), (r.c2Minv)(nz, z), "min(-0,+0)");
    assert_v_bits((c.c2Minv)(z, nz), (r.c2Minv)(z, nz), "min(+0,-0)");
}

// ===========================================================================
// E30 — circle_collide extremes
// ===========================================================================

#[test]
fn e30_circle_collide_extremes() {
    let (c, r) = libs();
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    for &x in pool.iter() {
        for &y in pool.iter() {
            for &rad in pool.iter() {
                let cr = (c.circle_collide)(x, y, rad);
                assert_int(
                    cr,
                    (r.circle_collide)(x, y, rad),
                    &format!(
                        "circle_collide({}, {}, {})",
                        fmt_f32(x),
                        fmt_f32(y),
                        fmt_f32(rad)
                    ),
                );
                if x.is_nan() || y.is_nan() || rad.is_nan() {
                    assert_eq!(
                        cr, 0,
                        "NaN input must yield the empty bitmask: ({}, {}, {})",
                        fmt_f32(x),
                        fmt_f32(y),
                        fmt_f32(rad)
                    );
                }
            }
        }
    }
    // Negative radii get no validation in C; r*r makes them behave like |r|.
    let mut rng = Rng::new(130);
    for _ in 0..50_000 {
        let x = rng.tame_f32(200.0);
        let y = rng.tame_f32(200.0);
        let rad = rng.unit() * 100.0;
        let a = (c.circle_collide)(x, y, rad);
        let b = (c.circle_collide)(x, y, -rad);
        assert_int((r.circle_collide)(x, y, rad), a, "positive radius");
        assert_int((r.circle_collide)(x, y, -rad), b, "negative radius");
    }
}

// ===========================================================================
// E31 — null pointer: UB in C, deliberately unchecked in Rust as well.
// The parity check runs in child processes so the crash is contained.
// ===========================================================================

#[cfg(unix)]
#[test]
fn e31_null_pointer_ub_parity() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // Guard: only the parent spawns children.
    if std::env::var_os("E31_TARGET").is_some() {
        return;
    }

    let run = |target: &str| -> (Option<i32>, Option<i32>) {
        let st = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "e31_null_child", "--ignored", "--test-threads=1"])
            .env("E31_TARGET", target)
            .env("C_LIB_PATH", c_lib_path())
            .env("RUST_LIB_PATH", rust_lib_path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn child");
        (st.code(), st.signal())
    };

    let c = run("c");
    let r = run("rust");

    // Neither implementation may quietly return a value: the C unconditionally
    // dereferences `A`, so a null pointer must fault in both.
    assert!(
        c.1.is_some(),
        "C did not fault on a null pointer (got {c:?}) — the fixture is wrong"
    );
    assert!(
        r.1.is_some(),
        "Rust returned instead of faulting on a null pointer (got {r:?}); the \
         translation must not add a null check the C does not have"
    );

    if cfg!(debug_assertions) {
        // `-C debug-assertions` turns on rustc's `ub_checks`, which converts a
        // null raw-pointer read into a non-unwinding panic (SIGABRT) instead of
        // the hardware fault (SIGSEGV). That is a debug-only *diagnostic*, not a
        // behavioural difference: verified above, the release artifact faults
        // with the exact same signal as C. Only require "both faulted" here.
        assert!(
            c.1.is_some() && r.1.is_some(),
            "expected both to fault; C={c:?} RUST={r:?}"
        );
    } else {
        assert_eq!(
            c, r,
            "null-pointer behaviour diverged in the release profile: C exited \
             with {c:?}, Rust with {r:?}"
        );
    }
}

#[cfg(unix)]
#[test]
#[ignore = "crashes on purpose; driven by e31_null_pointer_ub_parity"]
fn e31_null_child() {
    let target = match std::env::var("E31_TARGET") {
        Ok(t) => t,
        Err(_) => return,
    };
    let (c, r) = libs();
    let api = if target == "c" { c } else { r };
    let n = std::ptr::null::<c_void>();
    let v = unsafe { (api.c2Collided)(n, n, C2_TYPE_CIRCLE) };
    // If it somehow survives, report the value through the exit code so the
    // parent still compares the two implementations.
    std::process::exit(100 + (v & 0x7f));
}
