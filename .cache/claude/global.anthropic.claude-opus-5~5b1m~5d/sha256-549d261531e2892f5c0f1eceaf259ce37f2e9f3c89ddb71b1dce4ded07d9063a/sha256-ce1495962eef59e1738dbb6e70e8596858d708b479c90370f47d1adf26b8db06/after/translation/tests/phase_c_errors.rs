//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1–57). Each test *constructs the exact
//! invalid input or condition* the C checks for, calls both libraries, and
//! asserts they agree on the same rejection sentinel — and, where the C's
//! contract says so, that `*out` was left untouched (the 32-byte out-buffer is
//! pre-poisoned with `0xA5`, so "untouched" is verifiable rather than assumed).
//!
//! The library has no error enum: its entire rejection vocabulary is
//! `return 0`. So "same error" means *same 0/1 sentinel* AND *same out-buffer
//! bytes* — a test that only checked "both returned 0" would pass even if one
//! library scribbled on `*out` and the other did not.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

/// Assert both libraries returned exactly `expect_ret` and left `*out` pristine.
fn assert_rejected_untouched(ctx: &str, expect_ret: c_int, c: &RayResult, r: &RayResult) {
    assert_eq!(c.ret, expect_ret, "{ctx}: C returned {} not {expect_ret}", c.ret);
    assert!(
        c.out.iter().all(|&b| b == POISON),
        "{ctx}: the C wrote to *out on a rejection path: {:02x?}",
        &c.out[..16]
    );
    assert!(
        c == r,
        "{ctx}: DIVERGENCE\n  C    = {c:?}\n  RUST = {r:?}"
    );
}

/// Assert both libraries agree bit-for-bit (return value + full out buffer).
fn assert_same(ctx: &str, c: &RayResult, r: &RayResult) {
    assert!(
        c == r,
        "{ctx}: DIVERGENCE\n  C    = {c:?}\n  RUST = {r:?}"
    );
}

const QNAN: f32 = f32::from_bits(0x7FC0_0000u32);

// ===========================================================================
// c2RaytoCircle — rows 1–6
// ===========================================================================

#[test]
fn err01_circle_disc_negative() {
    let l = libs();
    // Ray line passes well clear of the circle → b*b - c < 0.
    let cases = [
        (
            c2Ray {
                p: c2v { x: -10.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 2.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 50.0 },
                d: c2v { x: 0.0, y: -1.0 },
                t: 100.0,
            },
            c2Circle {
                p: c2v { x: 40.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 3.0, y: 3.0 },
                d: c2v { x: 1.0, y: 1.0 },
                t: 100.0,
            },
            c2Circle {
                p: c2v { x: -20.0, y: 20.0 },
                r: 0.5,
            },
        ),
    ];
    for (i, &(a, b)) in cases.iter().enumerate() {
        let c = run_circle(&l.c, a, b);
        let r = run_circle(&l.rs, a, b);
        assert_rejected_untouched(&format!("err01 #{i}"), 0, &c, &r);
    }
    // `out == NULL` is safe on this path: the C returns before touching it.
    for &(a, b) in &cases {
        let cr = unsafe { (l.c.c2RaytoCircle)(a, b, ptr::null_mut()) };
        let rr = unsafe { (l.rs.c2RaytoCircle)(a, b, ptr::null_mut()) };
        assert_eq!((cr, rr), (0, 0), "err01 null-out must return 0 in both");
    }
}

#[test]
fn err02_circle_t_negative() {
    let l = libs();
    // Origin strictly inside → nearest root is behind the origin → t < 0.
    let cases = [
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 3.0,
            },
        ),
        // Circle entirely behind the ray origin.
        (
            c2Ray {
                p: c2v { x: 10.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 2.0,
            },
        ),
    ];
    for (i, &(a, b)) in cases.iter().enumerate() {
        let c = run_circle(&l.c, a, b);
        let r = run_circle(&l.rs, a, b);
        assert_rejected_untouched(&format!("err02 #{i}"), 0, &c, &r);
    }
}

#[test]
fn err03_circle_t_beyond_ray_length() {
    let l = libs();
    // Hit is at t == 3, but A.t is shorter — including exactly one step short.
    let b = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    let base = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 3.0,
    };
    // t == A.t exactly → HIT (inclusive bound).
    let c = run_circle(&l.c, base, b);
    let r = run_circle(&l.rs, base, b);
    assert_eq!(c.ret, 1, "err03: t == A.t must be an inclusive hit");
    assert_same("err03 inclusive", &c, &r);
    // One ULP below → miss.
    let mut short = base;
    short.t = f32::from_bits(3.0f32.to_bits() - 1);
    let c = run_circle(&l.c, short, b);
    let r = run_circle(&l.rs, short, b);
    assert_rejected_untouched("err03 one-ulp-short", 0, &c, &r);
    for &t in &[0.0f32, -0.0, 1.0, 2.9, -5.0] {
        let mut a = base;
        a.t = t;
        let c = run_circle(&l.c, a, b);
        let r = run_circle(&l.rs, a, b);
        assert_rejected_untouched(&format!("err03 t={}", show(t)), 0, &c, &r);
    }
}

#[test]
fn err04_circle_nan_falls_through_both_checks() {
    let l = libs();
    // NaN anywhere makes `disc < 0` false and then `t >= 0` false, so the C
    // exits via the *final* `return 0` with `*out` untouched.
    let base_a = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let base_b = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        for slot in 0..8 {
            let mut a = base_a;
            let mut b = base_b;
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.d.x = nan,
                3 => a.d.y = nan,
                4 => a.t = nan,
                5 => b.p.x = nan,
                6 => b.p.y = nan,
                _ => b.r = nan,
            }
            let c = run_circle(&l.c, a, b);
            let r = run_circle(&l.rs, a, b);
            assert_rejected_untouched(
                &format!("err04 slot{slot} nan={nanbits:#010x}"),
                0,
                &c,
                &r,
            );
        }
    }
    // inf - inf inside `disc` also produces NaN from *finite-looking* inputs.
    let a = c2Ray {
        p: c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        d: c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        t: f32::MAX,
    };
    let b = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: f32::MAX,
    };
    assert_same("err04 overflow-nan", &run_circle(&l.c, a, b), &run_circle(&l.rs, a, b));
}

#[test]
fn err05_circle_negative_radius_behaves_like_positive() {
    let l = libs();
    // `c = dot(m,m) - r*r` squares the radius, so -r behaves as +|r|; the C
    // never validates the sign. Confirm both libraries agree AND that the
    // negative radius really does hit (so this row has teeth).
    let a = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    for &r in &[-2.0f32, 2.0, -0.5, 0.5, -4.0, 4.0] {
        let b = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r,
        };
        let c = run_circle(&l.c, a, b);
        let rr = run_circle(&l.rs, a, b);
        assert_same(&format!("err05 r={}", show(r)), &c, &rr);
    }
    let pos = run_circle(&l.c, a, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 });
    let neg = run_circle(&l.c, a, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -2.0 });
    assert_eq!(pos.ret, 1, "err05: the +r case should hit");
    assert_eq!(
        pos, neg,
        "err05: the C should treat -r exactly like +r (r is squared)"
    );
}

#[test]
fn err06_circle_zero_radius_at_origin_yields_nan_normal() {
    let l = libs();
    for &p in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 7.0, y: -3.5 },
    ] {
        for &r in &[0.0f32, -0.0] {
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -3.0, y: 4.0 },
            ] {
                let a = c2Ray { p, d, t: 4.0 };
                let b = c2Circle { p, r };
                let c = run_circle(&l.c, a, b);
                let rr = run_circle(&l.rs, a, b);
                assert_same(&format!("err06 r={} p={}", show(r), showv(p)), &c, &rr);
            }
        }
    }
    // The documented shape of the result: hit, t == 0, n == (NaN, NaN).
    let a = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let b = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    let c = run_circle(&l.c, a, b);
    assert_eq!(c.ret, 1, "err06: origin == centre with r == 0 must report a hit");
    let rc = unsafe { (c.out.as_ptr() as *const c2Raycast).read_unaligned() };
    // `t = -b - sqrtf(disc)` with `b == +0.0` and `disc == 0` is
    // `(-0.0) - 0.0 == -0.0`, so the sign bit is SET. This is exactly the kind
    // of detail a `t == 0.0` comparison would silently paper over.
    assert_eq!(
        fb(rc.t),
        fb(-0.0),
        "err06: t must be -0.0 (from -b - sqrt(0) with b == +0.0)"
    );
    assert!(
        rc.n.x.is_nan() && rc.n.y.is_nan(),
        "err06: c2Norm((0,0)) must give a NaN normal, got {}",
        showv(rc.n)
    );
}

// ===========================================================================
// c2AABBtoAABB — rows 7–12
// ===========================================================================

#[test]
fn err07_to_err10_aabbtoaabb_each_separating_axis() {
    let l = libs();
    let a = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    // One row per `d0..d3`, each isolated so exactly one disjunct is true.
    let rows: [(&str, c2AABB); 4] = [
        (
            "err07 d0: B.max.x < A.min.x",
            c2AABB {
                min: c2v { x: -5.0, y: 0.0 },
                max: c2v { x: -1.0, y: 2.0 },
            },
        ),
        (
            "err08 d1: A.max.x < B.min.x",
            c2AABB {
                min: c2v { x: 3.0, y: 0.0 },
                max: c2v { x: 5.0, y: 2.0 },
            },
        ),
        (
            "err09 d2: B.max.y < A.min.y",
            c2AABB {
                min: c2v { x: 0.0, y: -5.0 },
                max: c2v { x: 2.0, y: -1.0 },
            },
        ),
        (
            "err10 d3: A.max.y < B.min.y",
            c2AABB {
                min: c2v { x: 0.0, y: 3.0 },
                max: c2v { x: 2.0, y: 5.0 },
            },
        ),
    ];
    for (name, b) in rows {
        let cv = (l.c.c2AABBtoAABB)(a, b);
        let rv = (l.rs.c2AABBtoAABB)(a, b);
        assert_eq!(cv, 0, "{name}: expected the C to reject");
        assert_eq!(cv, rv, "{name}: DIVERGENCE C={cv} RUST={rv}");
    }
    // Exactly-touching versions of the same four axes must NOT reject.
    let touching: [c2AABB; 4] = [
        c2AABB {
            min: c2v { x: -2.0, y: 0.0 },
            max: c2v { x: 0.0, y: 2.0 },
        },
        c2AABB {
            min: c2v { x: 2.0, y: 0.0 },
            max: c2v { x: 4.0, y: 2.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: -2.0 },
            max: c2v { x: 2.0, y: 0.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: 2.0 },
            max: c2v { x: 2.0, y: 4.0 },
        },
    ];
    for (i, b) in touching.into_iter().enumerate() {
        let cv = (l.c.c2AABBtoAABB)(a, b);
        let rv = (l.rs.c2AABBtoAABB)(a, b);
        assert_eq!(cv, 1, "err07-10 touch #{i}: `<` is strict, so touching overlaps");
        assert_eq!(cv, rv);
    }
}

#[test]
fn err11_aabbtoaabb_nan_reports_overlap() {
    let l = libs();
    let a = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    // Separated on every axis, then poison one coordinate with NaN: all four
    // `<` comparisons go false, so the C reports OVERLAP.
    let far = c2AABB {
        min: c2v { x: 100.0, y: 100.0 },
        max: c2v { x: 200.0, y: 200.0 },
    };
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        let mut b = far;
        b.min.x = nan;
        b.min.y = nan;
        let cv = (l.c.c2AABBtoAABB)(a, b);
        let rv = (l.rs.c2AABBtoAABB)(a, b);
        assert_eq!(cv, rv, "err11 {nanbits:#010x}: DIVERGENCE C={cv} RUST={rv}");
    }
    // All-NaN boxes: every comparison is false → `!(0)` → 1.
    let nanbox = c2AABB {
        min: c2v { x: QNAN, y: QNAN },
        max: c2v { x: QNAN, y: QNAN },
    };
    let cv = (l.c.c2AABBtoAABB)(nanbox, nanbox);
    assert_eq!(cv, 1, "err11: an all-NaN box must report overlap in the C");
    assert_eq!(cv, (l.rs.c2AABBtoAABB)(nanbox, nanbox));
}

#[test]
fn err12_aabbtoaabb_inverted_boxes() {
    let l = libs();
    let inverted = [
        c2AABB {
            min: c2v { x: 2.0, y: 2.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
        c2AABB {
            min: c2v { x: 5.0, y: -5.0 },
            max: c2v { x: -5.0, y: 5.0 },
        },
        c2AABB {
            min: c2v {
                x: f32::INFINITY,
                y: f32::INFINITY,
            },
            max: c2v {
                x: f32::NEG_INFINITY,
                y: f32::NEG_INFINITY,
            },
        },
    ];
    let normal = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for (i, &a) in inverted.iter().enumerate() {
        for &b in inverted.iter().chain(std::iter::once(&normal)) {
            let cv = (l.c.c2AABBtoAABB)(a, b);
            let rv = (l.rs.c2AABBtoAABB)(a, b);
            assert_eq!(cv, rv, "err12 #{i}: DIVERGENCE C={cv} RUST={rv}");
            let cv = (l.c.c2AABBtoAABB)(b, a);
            let rv = (l.rs.c2AABBtoAABB)(b, a);
            assert_eq!(cv, rv, "err12 #{i} swapped: DIVERGENCE C={cv} RUST={rv}");
        }
    }
}

// ===========================================================================
// c2AABBtoPoint — rows 13–17
// ===========================================================================

#[test]
fn err13_to_err16_aabbtopoint_each_axis() {
    let l = libs();
    let a = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    let rows: [(&str, c2v); 4] = [
        ("err13 d0: B.x < A.min.x", c2v { x: -1.0, y: 1.0 }),
        ("err14 d1: B.y < A.min.y", c2v { x: 1.0, y: -1.0 }),
        ("err15 d2: B.x > A.max.x", c2v { x: 3.0, y: 1.0 }),
        ("err16 d3: B.y > A.max.y", c2v { x: 1.0, y: 3.0 }),
    ];
    for (name, p) in rows {
        let cv = (l.c.c2AABBtoPoint)(a, p);
        let rv = (l.rs.c2AABBtoPoint)(a, p);
        assert_eq!(cv, 0, "{name}: expected the C to reject");
        assert_eq!(cv, rv, "{name}: DIVERGENCE C={cv} RUST={rv}");
    }
    // Exactly on each edge must be INSIDE (`<`/`>` are strict).
    for (i, &p) in [
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 2.0, y: 1.0 },
        c2v { x: 1.0, y: 2.0 },
    ]
    .iter()
    .enumerate()
    {
        let cv = (l.c.c2AABBtoPoint)(a, p);
        assert_eq!(cv, 1, "err13-16 edge #{i}: strict comparison → on-edge is inside");
        assert_eq!(cv, (l.rs.c2AABBtoPoint)(a, p));
    }
}

#[test]
fn err17_aabbtopoint_nan_point() {
    let l = libs();
    let a = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        for p in [
            c2v { x: nan, y: nan },
            c2v { x: nan, y: 1.0 },
            c2v { x: 1.0, y: nan },
        ] {
            let cv = (l.c.c2AABBtoPoint)(a, p);
            let rv = (l.rs.c2AABBtoPoint)(a, p);
            assert_eq!(cv, rv, "err17 {nanbits:#010x}: DIVERGENCE C={cv} RUST={rv}");
        }
    }
    let cv = (l.c.c2AABBtoPoint)(a, c2v { x: QNAN, y: QNAN });
    assert_eq!(cv, 1, "err17: an all-NaN point must report inside in the C");
    assert_eq!(cv, (l.rs.c2AABBtoPoint)(a, c2v { x: QNAN, y: QNAN }));
}

// ===========================================================================
// c2CircleToPoint — rows 18–20
// ===========================================================================

#[test]
fn err18_circletopoint_on_rim_is_a_miss() {
    let l = libs();
    // Exact Pythagorean rim points: `d2 == r*r`, and the C uses a strict `<`.
    for &(cx, cy, r, px, py) in &[
        (0.0f32, 0.0, 5.0, 3.0, 4.0),
        (0.0f32, 0.0, 5.0, -4.0, 3.0),
        (0.0f32, 0.0, 13.0, 5.0, 12.0),
        (1.0f32, 2.0, 13.0, 6.0, 14.0),
        (0.0f32, 0.0, 1.0, 1.0, 0.0),
        (0.0f32, 0.0, 2.0, 0.0, -2.0),
    ] {
        let a = c2Circle {
            p: c2v { x: cx, y: cy },
            r,
        };
        let b = c2v { x: px, y: py };
        let cv = (l.c.c2CircleToPoint)(a, b);
        let rv = (l.rs.c2CircleToPoint)(a, b);
        assert_eq!(cv, 0, "err18: a point exactly on the rim must MISS (strict `<`)");
        assert_eq!(cv, rv, "err18: DIVERGENCE C={cv} RUST={rv}");
    }
    // Strictly outside.
    let a = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 5.0,
    };
    for &b in &[
        c2v { x: 6.0, y: 0.0 },
        c2v { x: 100.0, y: 100.0 },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
    ] {
        let cv = (l.c.c2CircleToPoint)(a, b);
        assert_eq!(cv, 0, "err18 outside {}", showv(b));
        assert_eq!(cv, (l.rs.c2CircleToPoint)(a, b));
    }
}

#[test]
fn err19_circletopoint_zero_radius_never_contains() {
    let l = libs();
    for &r in &[0.0f32, -0.0] {
        let a = c2Circle {
            p: c2v { x: 3.0, y: -4.0 },
            r,
        };
        for &b in &[
            c2v { x: 3.0, y: -4.0 }, // exactly the centre
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 3.0000001, y: -4.0 },
            c2v { x: -0.0, y: -0.0 },
        ] {
            let cv = (l.c.c2CircleToPoint)(a, b);
            let rv = (l.rs.c2CircleToPoint)(a, b);
            assert_eq!(cv, 0, "err19: r == 0 can never contain a point");
            assert_eq!(cv, rv, "err19: DIVERGENCE C={cv} RUST={rv}");
        }
    }
}

#[test]
fn err20_circletopoint_nan() {
    let l = libs();
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        for slot in 0..5 {
            let mut a = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 5.0,
            };
            let mut b = c2v { x: 1.0, y: 1.0 };
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.x = nan,
                _ => b.y = nan,
            }
            let cv = (l.c.c2CircleToPoint)(a, b);
            let rv = (l.rs.c2CircleToPoint)(a, b);
            assert_eq!(
                cv, 0,
                "err20 slot{slot}: `d2 < NaN` is false, so the C must reject"
            );
            assert_eq!(cv, rv, "err20 slot{slot}: DIVERGENCE C={cv} RUST={rv}");
        }
    }
}

// ===========================================================================
// c2RaytoAABB — rows 21–28
// ===========================================================================

#[test]
fn err21_raytoaabb_ray_bb_does_not_overlap() {
    let l = libs();
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    // Short rays whose own AABB is entirely off to one side (each of the 4 axes).
    let cases = [
        c2Ray {
            p: c2v { x: -10.0, y: 1.0 },
            d: c2v { x: -1.0, y: 0.0 },
            t: 1.0,
        },
        c2Ray {
            p: c2v { x: 10.0, y: 1.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 1.0,
        },
        c2Ray {
            p: c2v { x: 1.0, y: -10.0 },
            d: c2v { x: 0.0, y: -1.0 },
            t: 1.0,
        },
        c2Ray {
            p: c2v { x: 1.0, y: 10.0 },
            d: c2v { x: 0.0, y: 1.0 },
            t: 1.0,
        },
    ];
    for (i, &a) in cases.iter().enumerate() {
        let c = run_aabb(&l.c, a, b);
        let r = run_aabb(&l.rs, a, b);
        assert_rejected_untouched(&format!("err21 #{i}"), 0, &c, &r);
        // Safe on this path: the C returns before touching `out`.
        let cn = unsafe { (l.c.c2RaytoAABB)(a, b, ptr::null_mut()) };
        let rn = unsafe { (l.rs.c2RaytoAABB)(a, b, ptr::null_mut()) };
        assert_eq!((cn, rn), (0, 0), "err21 #{i} null-out");
    }
}

#[test]
fn err22_raytoaabb_sat_separation() {
    let l = libs();
    // A long diagonal segment whose bounding box overlaps the box but whose SAT
    // axis (the segment normal) separates it → the `d > 0` rejection.
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    let cases = [
        c2Ray {
            p: c2v { x: -8.0, y: 6.0 },
            d: c2v { x: 1.0, y: -1.0 },
            t: 20.0,
        },
        c2Ray {
            p: c2v { x: 6.0, y: -8.0 },
            d: c2v { x: -1.0, y: 1.0 },
            t: 20.0,
        },
        c2Ray {
            p: c2v { x: -8.0, y: 10.0 },
            d: c2v { x: 1.0, y: -1.0 },
            t: 30.0,
        },
    ];
    let mut saw_reject = 0;
    for (i, &a) in cases.iter().enumerate() {
        let c = run_aabb(&l.c, a, b);
        let r = run_aabb(&l.rs, a, b);
        if c.ret == 0 {
            saw_reject += 1;
            assert_rejected_untouched(&format!("err22 #{i}"), 0, &c, &r);
        } else {
            assert_same(&format!("err22 #{i}"), &c, &r);
        }
    }
    assert!(saw_reject > 0, "err22 never reached the `d > 0` rejection");

    // Sweep a diagonal segment past the box so `d` crosses zero, guaranteeing
    // both sides of the branch are exercised.
    for i in 0..4096 {
        let off = -12.0 + 24.0 * (i as f32) / 4096.0;
        let a = c2Ray {
            p: c2v { x: -8.0, y: off },
            d: c2v { x: 1.0, y: 1.0 },
            t: 20.0,
        };
        assert_same(
            &format!("err22 sweep {}", show(off)),
            &run_aabb(&l.c, a, b),
            &run_aabb(&l.rs, a, b),
        );
    }
}

#[test]
fn err23_raytoaabb_all_t_beyond_one() {
    let l = libs();
    // Reach the `hit == 0` exit: all four `c2RayToPlane_OneDimensional` results
    // must exceed 1.0. Sweep broadly and require the branch to be observed.
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    let mut rng = Rng::new(SEED ^ 123);
    let mut saw = 0usize;
    for i in 0..20000 {
        let a = c2Ray {
            p: rng.vec_grid(6),
            d: rng.vec_grid(3),
            t: rng.gridded(8),
        };
        let c = run_aabb(&l.c, a, b);
        let r = run_aabb(&l.rs, a, b);
        if c.ret == 0 && c.out.iter().all(|&x| x == POISON) {
            saw += 1;
        }
        assert_same(&format!("err23 #{i}"), &c, &r);
    }
    assert!(saw > 0, "err23 never observed a rejection with *out untouched");
}

#[test]
fn err24_raytoaabb_zero_length_ray() {
    let l = libs();
    let boxes = [
        c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 2.0, y: 2.0 },
        },
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        },
    ];
    for &b in &boxes {
        for &t in &[0.0f32, -0.0] {
            for i in 0..512 {
                let s = -3.0 + 6.0 * (i as f32) / 512.0;
                for &d in &[
                    c2v { x: 1.0, y: 0.0 },
                    c2v { x: 0.0, y: 0.0 },
                    c2v { x: 1.0, y: 1.0 },
                    c2v {
                        x: f32::INFINITY,
                        y: 0.0,
                    },
                ] {
                    let a = c2Ray {
                        p: c2v { x: s, y: s },
                        d,
                        t,
                    };
                    assert_same(
                        &format!("err24 t={} s={}", show(t), show(s)),
                        &run_aabb(&l.c, a, b),
                        &run_aabb(&l.rs, a, b),
                    );
                }
            }
        }
    }
}

#[test]
fn err25_raytoaabb_negative_ray_length() {
    let l = libs();
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    for &t in &[-1.0f32, -0.5, -100.0, f32::NEG_INFINITY, f32::MIN] {
        for i in 0..256 {
            let s = -5.0 + 10.0 * (i as f32) / 256.0;
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: -1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 1.0, y: 1.0 },
            ] {
                let a = c2Ray {
                    p: c2v { x: s, y: 1.0 },
                    d,
                    t,
                };
                assert_same(
                    &format!("err25 t={} s={}", show(t), show(s)),
                    &run_aabb(&l.c, a, b),
                    &run_aabb(&l.rs, a, b),
                );
            }
        }
    }
}

#[test]
fn err26_raytoaabb_infinite_t_and_zero_direction() {
    let l = libs();
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    // `A.t == inf` with `A.d == (0,0)` makes `p1` NaN, which flows through the
    // NaN ternaries in c2Minv/c2Maxv and then c2AABBtoAABB.
    for &t in &[f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        for &d in &[
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            c2v { x: QNAN, y: QNAN },
        ] {
            for &p in &[
                c2v { x: 1.0, y: 1.0 },
                c2v { x: -5.0, y: 1.0 },
                c2v { x: 0.0, y: 0.0 },
            ] {
                let a = c2Ray { p, d, t };
                assert_same(
                    &format!("err26 t={} d={}", show(t), showv(d)),
                    &run_aabb(&l.c, a, b),
                    &run_aabb(&l.rs, a, b),
                );
            }
        }
    }
}

#[test]
fn err27_and_err28_raytoaabb_degenerate_and_inverted_box() {
    let l = libs();
    let boxes = [
        // degenerate: min == max → half_extents == (0,0)
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        },
        c2AABB {
            min: c2v { x: -0.0, y: -0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
        // inverted: min > max → negative half_extents
        c2AABB {
            min: c2v { x: 2.0, y: 2.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
        c2AABB {
            min: c2v { x: 5.0, y: -5.0 },
            max: c2v { x: -5.0, y: 5.0 },
        },
        // zero-width in one axis only
        c2AABB {
            min: c2v { x: 1.0, y: 0.0 },
            max: c2v { x: 1.0, y: 2.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: 1.0 },
            max: c2v { x: 2.0, y: 1.0 },
        },
    ];
    for (bi, &b) in boxes.iter().enumerate() {
        for i in 0..512 {
            let s = -4.0 + 8.0 * (i as f32) / 512.0;
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 1.0, y: 1.0 },
                c2v { x: -1.0, y: 0.5 },
            ] {
                for &t in &[0.0f32, 4.0, 20.0, -4.0] {
                    let a = c2Ray {
                        p: c2v { x: s, y: s },
                        d,
                        t,
                    };
                    assert_same(
                        &format!("err27/28 box{bi} s={} t={}", show(s), show(t)),
                        &run_aabb(&l.c, a, b),
                        &run_aabb(&l.rs, a, b),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// c2RayToPlane_OneDimensional (static, observed through c2RaytoAABB) — 29–32
// ===========================================================================

#[test]
fn err29_to_err32_ray_to_plane_branches() {
    let l = libs();
    // Each of the helper's four exits is reachable from c2RaytoAABB:
    //  * `da < 0`              → ray origin on the outside of that plane
    //  * `da*db > 0`           → both endpoints on the same side
    //  * `da - db == 0`        → segment parallel to that plane
    //  * `da` NaN              → NaN coordinate
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    // Axis-parallel rays make `da == db` for the perpendicular pair (d == 0).
    for &(p, d) in &[
        (c2v { x: -5.0, y: 1.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 1.0, y: -5.0 }, c2v { x: 0.0, y: 1.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 2.0, y: 2.0 }, c2v { x: 0.0, y: -1.0 }),
        // both endpoints on the same side (da*db > 0)
        (c2v { x: -5.0, y: 1.0 }, c2v { x: -1.0, y: 0.0 }),
        (c2v { x: 5.0, y: 1.0 }, c2v { x: 1.0, y: 0.0 }),
        // origin outside on the min.x plane (da < 0 for that plane)
        (c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 0.0 }),
    ] {
        for &t in &[0.0f32, 1.0, 4.0, 10.0, 100.0, -4.0] {
            let a = c2Ray { p, d, t };
            assert_same(
                &format!("err29-32 p={} d={} t={}", showv(p), showv(d), show(t)),
                &run_aabb(&l.c, a, b),
                &run_aabb(&l.rs, a, b),
            );
        }
    }
    // NaN in each coordinate → NaN `da` → NaN/NaN division inside the helper.
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        for slot in 0..5 {
            let mut a = c2Ray {
                p: c2v { x: -5.0, y: 1.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 20.0,
            };
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.d.x = nan,
                3 => a.d.y = nan,
                _ => a.t = nan,
            }
            assert_same(
                &format!("err29-32 nan slot{slot} {nanbits:#010x}"),
                &run_aabb(&l.c, a, b),
                &run_aabb(&l.rs, a, b),
            );
        }
    }
    // Exhaustive small-integer grid: guarantees `da == 0`, `db == 0` and
    // `da - db == 0` ties are all produced.
    for px in -3..=5i32 {
        for py in -3..=5i32 {
            for dx in -2..=2i32 {
                for dy in -2..=2i32 {
                    for t in [0i32, 1, 2, 4] {
                        let a = c2Ray {
                            p: c2v {
                                x: px as f32,
                                y: py as f32,
                            },
                            d: c2v {
                                x: dx as f32,
                                y: dy as f32,
                            },
                            t: t as f32,
                        };
                        assert_same(
                            &format!("err29-32 grid {px},{py} {dx},{dy} t={t}"),
                            &run_aabb(&l.c, a, b),
                            &run_aabb(&l.rs, a, b),
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// c2RaytoCapsule — rows 33–39
// ===========================================================================

#[test]
fn err33_capsule_fallthrough_writes_out_before_rejecting() {
    let l = libs();
    // Ray well clear of the capsule on one side, so the final `return 0` is
    // taken — but `*out` has ALREADY been written with n = norm(b-a), t = 0.
    let b = c2Capsule {
        a: c2v { x: 0.0, y: -3.0 },
        b: c2v { x: 0.0, y: 3.0 },
        r: 1.0,
    };
    let cases = [
        c2Ray {
            p: c2v { x: 20.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 5.0,
        },
        c2Ray {
            p: c2v { x: -20.0, y: 0.0 },
            d: c2v { x: -1.0, y: 0.0 },
            t: 5.0,
        },
        c2Ray {
            p: c2v { x: 10.0, y: 10.0 },
            d: c2v { x: 0.0, y: 1.0 },
            t: 5.0,
        },
    ];
    let expect_n = (l.rs.c2Norm)((l.rs.c2Sub)(b.b, b.a));
    for (i, &a) in cases.iter().enumerate() {
        let c = run_capsule(&l.c, a, b);
        let r = run_capsule(&l.rs, a, b);
        assert_eq!(c.ret, 0, "err33 #{i}: expected the fall-through rejection");
        let got = unsafe { (c.out.as_ptr() as *const c2Raycast).read_unaligned() };
        assert_eq!(
            fb(got.t),
            fb(0.0),
            "err33 #{i}: the C must have pre-written out->t = 0"
        );
        assert_eq!(
            vb(got.n),
            vb(expect_n),
            "err33 #{i}: the C must have pre-written out->n = norm(b-a)"
        );
        assert!(
            !c.out.iter().all(|&x| x == POISON),
            "err33 #{i}: *out should NOT be pristine — the C writes before rejecting"
        );
        assert_same(&format!("err33 #{i}"), &c, &r);
    }
}

#[test]
fn err34_capsule_degenerate_zero_length_axis() {
    let l = libs();
    for &pt in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 3.0, y: -4.0 },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
    ] {
        let b = c2Capsule {
            a: pt,
            b: pt,
            r: 1.0,
        };
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -1.0, y: 2.0 },
        ] {
            for &t in &[0.0f32, 1.0, 8.0, -8.0, f32::INFINITY] {
                for &p in &[pt, c2v { x: -5.0, y: 0.0 }, c2v { x: 1.0, y: 1.0 }] {
                    let a = c2Ray { p, d, t };
                    assert_same(
                        &format!("err34 pt={} d={} t={}", showv(pt), showv(d), show(t)),
                        &run_capsule(&l.c, a, b),
                        &run_capsule(&l.rs, a, b),
                    );
                }
            }
        }
    }
    // The documented shape: c2Norm((0,0)) is NaN, so out->n is (NaN, NaN).
    let b = c2Capsule {
        a: c2v { x: 1.0, y: 1.0 },
        b: c2v { x: 1.0, y: 1.0 },
        r: 1.0,
    };
    let a = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let c = run_capsule(&l.c, a, b);
    let got = unsafe { (c.out.as_ptr() as *const c2Raycast).read_unaligned() };
    assert!(
        got.n.x.is_nan() && got.n.y.is_nan(),
        "err34: a == b must give a NaN pre-written normal, got {}",
        showv(got.n)
    );
    assert_same("err34 shape", &c, &run_capsule(&l.rs, a, b));
}

#[test]
fn err35_and_err36_capsule_zero_and_negative_radius() {
    let l = libs();
    let axes = [
        (c2v { x: 0.0, y: -3.0 }, c2v { x: 0.0, y: 3.0 }),
        (c2v { x: -3.0, y: 0.0 }, c2v { x: 3.0, y: 0.0 }),
        (c2v { x: 0.0, y: 3.0 }, c2v { x: 0.0, y: -3.0 }),
        (c2v { x: -1.0, y: -1.0 }, c2v { x: 2.0, y: 2.0 }),
    ];
    for &r in &[0.0f32, -0.0, -1.0, -3.0, f32::NEG_INFINITY] {
        for (ai, &(a0, b0)) in axes.iter().enumerate() {
            let b = c2Capsule { a: a0, b: b0, r };
            for i in 0..256 {
                let s = -5.0 + 10.0 * (i as f32) / 256.0;
                for &d in &[
                    c2v { x: 1.0, y: 0.0 },
                    c2v { x: 0.0, y: 1.0 },
                    c2v { x: 1.0, y: 1.0 },
                    c2v { x: 0.0, y: 0.0 },
                ] {
                    for &t in &[0.0f32, 4.0, 20.0, -4.0] {
                        let a = c2Ray {
                            p: c2v { x: s, y: s * 0.5 },
                            d,
                            t,
                        };
                        assert_same(
                            &format!("err35/36 r={} axis{ai} s={}", show(r), show(s)),
                            &run_capsule(&l.c, a, b),
                            &run_capsule(&l.rs, a, b),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn err37_capsule_side_plane_division_by_zero() {
    let l = libs();
    // Reach the side-plane branch with `d = yAe.x - yAp.x == 0`, i.e. a ray
    // travelling parallel to the capsule axis while offset beyond the radius.
    let b = c2Capsule {
        a: c2v { x: 0.0, y: -3.0 },
        b: c2v { x: 0.0, y: 3.0 },
        r: 1.0,
    };
    let mut reached = 0usize;
    for i in 0..4096 {
        let x = -4.0 + 8.0 * (i as f32) / 4096.0;
        // Direction purely along the axis ⇒ yAd.x == 0 ⇒ yAe.x == yAp.x.
        for &d in &[c2v { x: 0.0, y: 1.0 }, c2v { x: 0.0, y: -1.0 }] {
            for &t in &[0.0f32, 4.0, 20.0, f32::INFINITY] {
                let a = c2Ray {
                    p: c2v { x, y: -10.0 },
                    d,
                    t,
                };
                let c = run_capsule(&l.c, a, b);
                let r = run_capsule(&l.rs, a, b);
                if c.ret != 0 {
                    reached += 1;
                }
                assert_same(&format!("err37 x={} t={}", show(x), show(t)), &c, &r);
            }
        }
    }
    // Also with A.t == 0 so yAe == yAp exactly regardless of direction.
    for i in 0..1024 {
        let x = -4.0 + 8.0 * (i as f32) / 1024.0;
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 1.0, y: 1.0 },
        ] {
            let a = c2Ray {
                p: c2v { x, y: 0.5 },
                d,
                t: 0.0,
            };
            assert_same(
                &format!("err37 t0 x={}", show(x)),
                &run_capsule(&l.c, a, b),
                &run_capsule(&l.rs, a, b),
            );
        }
    }
    let _ = reached;
}

#[test]
fn err38_and_err39_capsule_delegates_to_circle() {
    let l = libs();
    // `|yAp.x| < r` with `yAp.y` on either side of 0 delegates to c2RaytoCircle
    // for cap A or cap B, inheriting that function's own rejections.
    let b = c2Capsule {
        a: c2v { x: 0.0, y: -3.0 },
        b: c2v { x: 0.0, y: 3.0 },
        r: 1.0,
    };
    let mut delegated_miss = 0usize;
    let mut delegated_hit = 0usize;
    let expect_n = (l.rs.c2Norm)((l.rs.c2Sub)(b.b, b.a));
    for i in 0..8192 {
        // Origin close to the axis (|x| < r) but outside the capsule body, so
        // the early bb / circle checks fail and the delegation happens.
        let x = -0.999 + 1.998 * ((i % 128) as f32) / 128.0;
        let y = if i % 2 == 0 { -20.0 } else { 20.0 };
        for &d in &[
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 1.0, y: 0.0 },
        ] {
            for &t in &[0.0f32, 1.0, 25.0] {
                let a = c2Ray {
                    p: c2v { x, y },
                    d,
                    t,
                };
                let c = run_capsule(&l.c, a, b);
                let r = run_capsule(&l.rs, a, b);
                let got = unsafe { (c.out.as_ptr() as *const c2Raycast).read_unaligned() };
                if c.ret == 0 {
                    // A delegated circle miss leaves the pre-written *out alone.
                    if vb(got.n) == vb(expect_n) && fb(got.t) == fb(0.0) {
                        delegated_miss += 1;
                    }
                } else if vb(got.n) != vb(expect_n) {
                    delegated_hit += 1;
                }
                assert_same(&format!("err38/39 x={} y={} t={}", show(x), show(y), show(t)), &c, &r);
            }
        }
    }
    assert!(
        delegated_miss > 0,
        "err38/39 never observed a delegated circle MISS"
    );
    assert!(
        delegated_hit > 0,
        "err38/39 never observed a delegated circle HIT"
    );
}

// ===========================================================================
// c2RaytoPoly — rows 40–49
// ===========================================================================

#[test]
fn err40_poly_parallel_plane_outside() {
    let l = libs();
    // `den == 0 && num < 0`: ray direction perpendicular to a normal (parallel
    // to that plane) with the origin on the outside of it.
    let p = poly_ray_box();
    let buf = PolyBuf::from_poly(&p);
    // norms[0] = (1,0); direction (0,±1) gives den == 0. Origin x > 0.875 makes
    // num = dot(n, verts[0]-p) = 0.875 - p.x < 0.
    let cases = [
        c2Ray {
            p: c2v { x: 5.0, y: 0.0 },
            d: c2v { x: 0.0, y: 1.0 },
            t: 10.0,
        },
        c2Ray {
            p: c2v { x: 5.0, y: 0.0 },
            d: c2v { x: 0.0, y: -1.0 },
            t: 10.0,
        },
        // norms[3] = (0,-1); direction (±1,0). Origin y < -11.5.
        c2Ray {
            p: c2v { x: 0.0, y: -20.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        },
        c2Ray {
            p: c2v { x: 0.0, y: 20.0 },
            d: c2v { x: -1.0, y: 0.0 },
            t: 10.0,
        },
    ];
    for (i, &a) in cases.iter().enumerate() {
        let c = run_poly_raw(&l.c, a, &buf, None);
        let r = run_poly_raw(&l.rs, a, &buf, None);
        assert_rejected_untouched(&format!("err40 #{i}"), 0, &c, &r);
        // Safe: the C returns before touching `out`.
        let cn = unsafe { (l.c.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
        let rn = unsafe { (l.rs.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
        assert_eq!((cn, rn), (0, 0), "err40 #{i} null-out");
    }
}

#[test]
fn err41_poly_interval_collapse() {
    let l = invariant_libs();
    // `hi < lo` — reachable immediately when A.t < 0 (hi starts negative while
    // lo starts at 0) as long as the first iteration runs.
    let p = poly_ray_box();
    let buf = PolyBuf::from_poly(&p);
    for &t in &[-1.0f32, -0.001, -100.0, f32::NEG_INFINITY, f32::MIN] {
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 1.0, y: 1.0 },
        ] {
            let a = c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d,
                t,
            };
            let c = run_poly_raw(&l.c, a, &buf, None);
            let r = run_poly_raw(&l.rs, a, &buf, None);
            assert_rejected_untouched(&format!("err41 t={} d={}", show(t), showv(d)), 0, &c, &r);
        }
    }
    // Also the genuine interval collapse: a ray that leaves before it enters.
    let mut rng = Rng::new(SEED ^ 41);
    let mut collapses = 0usize;
    for i in 0..20000 {
        let nverts = 3 + (rng.below(6) as i32);
        let ply = convex_ngon(&mut rng, nverts);
        let b2 = PolyBuf::from_poly(&ply);
        let a = c2Ray {
            p: rng.vec_grid(10),
            d: rng.vec_grid(2),
            t: rng.gridded(6),
        };
        let c = run_poly_raw(&l.c, a, &b2, None);
        let r = run_poly_raw(&l.rs, a, &b2, None);
        if c.ret == 0 {
            collapses += 1;
        }
        assert_same(&format!("err41 rand #{i}"), &c, &r);
    }
    assert!(collapses > 0, "err41 never rejected");
}

/// `libs()` under a different name, purely so each test reads independently.
fn invariant_libs() -> &'static Pair {
    libs()
}

#[test]
fn err42_poly_index_never_set() {
    let l = libs();
    // Ray origin strictly inside the polygon: every plane is entered from the
    // inside, so `lo` is never tightened and `index` stays ~0.
    let p = poly_ray_box();
    let buf = PolyBuf::from_poly(&p);
    let mut inside_rejects = 0usize;
    for i in 0..2048 {
        let fx = -0.8 + 1.6 * ((i % 64) as f32) / 64.0;
        let fy = -11.0 + 22.0 * ((i / 64) as f32) / 32.0;
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 1.0, y: 1.0 },
        ] {
            let a = c2Ray {
                p: c2v { x: fx, y: fy },
                d,
                t: 100.0,
            };
            let c = run_poly_raw(&l.c, a, &buf, None);
            let r = run_poly_raw(&l.rs, a, &buf, None);
            if c.ret == 0 {
                inside_rejects += 1;
                assert_rejected_untouched(&format!("err42 inside #{i}"), 0, &c, &r);
            } else {
                assert_same(&format!("err42 inside #{i}"), &c, &r);
            }
        }
    }
    assert!(
        inside_rejects > 0,
        "err42 never reached the `index == ~0` rejection from inside the polygon"
    );
}

#[test]
fn err43_poly_count_zero() {
    let l = libs();
    let mut p = poly_ray_box();
    p.count = 0;
    let buf = PolyBuf::from_poly(&p);
    let mut rng = Rng::new(SEED ^ 43);
    for i in 0..1024 {
        let a = any_ray(&mut rng);
        let c = run_poly_raw(&l.c, a, &buf, None);
        let r = run_poly_raw(&l.rs, a, &buf, None);
        assert_rejected_untouched(&format!("err43 #{i}"), 0, &c, &r);
        let bx = rng.any_x();
        let c = run_poly_raw(&l.c, a, &buf, Some(&bx));
        let r = run_poly_raw(&l.rs, a, &buf, Some(&bx));
        assert_rejected_untouched(&format!("err43 bx #{i}"), 0, &c, &r);
    }
    // `out == NULL` is safe with count == 0: the C never writes.
    let a = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    let cn = unsafe { (l.c.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
    let rn = unsafe { (l.rs.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
    assert_eq!((cn, rn), (0, 0), "err43 null-out with count == 0");
}

#[test]
fn err44_poly_negative_count() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 44);
    for &count in &[-1i32, -2, -7, -8, -9, -1000, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        let mut p = poly_ray_box();
        p.count = count;
        let buf = PolyBuf::from_poly(&p);
        for i in 0..256 {
            let a = any_ray(&mut rng);
            let c = run_poly_raw(&l.c, a, &buf, None);
            let r = run_poly_raw(&l.rs, a, &buf, None);
            assert_rejected_untouched(&format!("err44 count={count} #{i}"), 0, &c, &r);
        }
        // null out is safe here too — the loop body never executes.
        let a = c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 1.0,
        };
        let cn = unsafe { (l.c.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
        let rn = unsafe { (l.rs.c2RaytoPoly)(a, buf.as_ptr(), ptr::null(), ptr::null_mut()) };
        assert_eq!((cn, rn), (0, 0), "err44 count={count} null-out");
    }
}

#[test]
fn err45_poly_count_past_the_fixed_arrays() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 45);
    // count 9..=16 indexes past verts[8]/norms[8]. Both libraries index the
    // SAME 512-byte buffer, so the out-of-range bytes are identical and any
    // divergence is a genuine translation bug rather than uninitialised memory.
    for count in 9..=16i32 {
        for i in 0..512 {
            let mut p = convex_ngon(&mut rng, 8);
            p.count = count;
            let buf = PolyBuf::from_poly(&p);
            let a = any_ray(&mut rng);
            assert_same(
                &format!("err45 count={count} #{i}"),
                &run_poly_raw(&l.c, a, &buf, None),
                &run_poly_raw(&l.rs, a, &buf, None),
            );
            let bx = rng.any_x();
            assert_same(
                &format!("err45 count={count} bx #{i}"),
                &run_poly_raw(&l.c, a, &buf, Some(&bx)),
                &run_poly_raw(&l.rs, a, &buf, Some(&bx)),
            );
        }
    }
    // count exactly one past the array bound, with a controlled tail so the
    // out-of-range normal is a well-behaved unit vector.
    let tail: Vec<f32> = vec![0.0, -1.0, 1.0, 0.0, -1.0, 0.0, 0.0, 1.0];
    for count in [9i32, 10, 12] {
        let mut p = convex_ngon(&mut rng, 8);
        p.count = count;
        let buf = PolyBuf::from_poly_with_tail(&p, &tail);
        for i in 0..256 {
            let a = sane_ray(&mut rng);
            assert_same(
                &format!("err45 tail count={count} #{i}"),
                &run_poly_raw(&l.c, a, &buf, None),
                &run_poly_raw(&l.rs, a, &buf, None),
            );
        }
    }
}

#[test]
fn err46_poly_null_bx_means_identity() {
    let l = libs();
    let ident = (l.rs.c2xIdentity)();
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..2048 {
        let nv = 1 + (rng.below(8) as i32);
        let p = convex_ngon(&mut rng, nv);
        let buf = PolyBuf::from_poly(&p);
        let a = any_ray(&mut rng);
        let with_null_c = run_poly_raw(&l.c, a, &buf, None);
        let with_ident_c = run_poly_raw(&l.c, a, &buf, Some(&ident));
        assert_eq!(
            with_null_c, with_ident_c,
            "err46 #{i}: NULL bx must mean c2xIdentity() in the C"
        );
        let with_null_r = run_poly_raw(&l.rs, a, &buf, None);
        let with_ident_r = run_poly_raw(&l.rs, a, &buf, Some(&ident));
        assert_eq!(
            with_null_r, with_ident_r,
            "err46 #{i}: NULL bx must mean c2xIdentity() in the Rust too"
        );
        assert_same(&format!("err46 #{i} null"), &with_null_c, &with_null_r);
        assert_same(&format!("err46 #{i} ident"), &with_ident_c, &with_ident_r);
    }
}

#[test]
fn err47_poly_negative_ray_length() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 47);
    for &t in &[-0.0f32, -1e-30, -1.0, -1e30, f32::NEG_INFINITY, f32::MIN] {
        for count in [0i32, 1, 4, 8] {
            let mut p = convex_ngon(&mut rng, if count == 0 { 4 } else { count });
            p.count = count;
            let buf = PolyBuf::from_poly(&p);
            for i in 0..128 {
                let a = c2Ray {
                    p: rng.vec_grid(10),
                    d: rng.dir(),
                    t,
                };
                let c = run_poly_raw(&l.c, a, &buf, None);
                let r = run_poly_raw(&l.rs, a, &buf, None);
                assert_rejected_untouched(
                    &format!("err47 t={} count={count} #{i}", show(t)),
                    0,
                    &c,
                    &r,
                );
            }
        }
    }
}

#[test]
fn err48_poly_degenerate_transform() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 48);
    let degenerate = [
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 0.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: -0.0, y: -0.0 },
            r: c2r { c: -0.0, s: -0.0 },
        },
        c2x {
            p: c2v { x: 5.0, y: -5.0 },
            r: c2r { c: 0.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 100.0, s: -100.0 },
        },
        c2x {
            p: c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
            r: c2r { c: 1.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: QNAN, s: QNAN },
        },
    ];
    for (xi, bx) in degenerate.iter().enumerate() {
        for count in 1..=8i32 {
            for i in 0..64 {
                let p = convex_ngon(&mut rng, count);
                let buf = PolyBuf::from_poly(&p);
                let a = any_ray(&mut rng);
                assert_same(
                    &format!("err48 x{xi} count={count} #{i}"),
                    &run_poly_raw(&l.c, a, &buf, Some(bx)),
                    &run_poly_raw(&l.rs, a, &buf, Some(bx)),
                );
            }
        }
    }
}

#[test]
fn err49_poly_nan_makes_every_comparison_false() {
    let l = libs();
    // With NaN in the geometry, `den == 0`, `num < 0`, `den < 0`, `den > 0` and
    // `hi < lo` are all false, so the loop completes with index == ~0.
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        // NaN in every ray / transform slot.
        for slot in 0..9 {
            let mut a = c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            };
            let mut bx = c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 1.0, s: 0.0 },
            };
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.d.x = nan,
                3 => a.d.y = nan,
                4 => a.t = nan,
                5 => bx.p.x = nan,
                6 => bx.p.y = nan,
                7 => bx.r.c = nan,
                _ => bx.r.s = nan,
            }
            let buf = PolyBuf::from_poly(&poly_ray_box());
            assert_same(
                &format!("err49 ray slot{slot} {nanbits:#010x}"),
                &run_poly_raw(&l.c, a, &buf, Some(&bx)),
                &run_poly_raw(&l.rs, a, &buf, Some(&bx)),
            );
            assert_same(
                &format!("err49 ray-null slot{slot} {nanbits:#010x}"),
                &run_poly_raw(&l.c, a, &buf, None),
                &run_poly_raw(&l.rs, a, &buf, None),
            );
        }
        // Every polygon plane NaN → the whole loop is inert.
        let mut p = poly_ray_box();
        for k in 0..8 {
            p.verts[k] = c2v { x: nan, y: nan };
            p.norms[k] = c2v { x: nan, y: nan };
        }
        let buf = PolyBuf::from_poly(&p);
        let a = c2Ray {
            p: c2v { x: -5.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        let c = run_poly_raw(&l.c, a, &buf, None);
        let r = run_poly_raw(&l.rs, a, &buf, None);
        assert_rejected_untouched(&format!("err49 all-nan {nanbits:#010x}"), 0, &c, &r);
    }
}

// ===========================================================================
// c2CastRay — rows 50–52
// ===========================================================================

#[test]
fn err50_castray_out_of_range_type_enum() {
    let l = libs();
    // A C enum accepts any `int` across the FFI boundary, and the `switch` has
    // no `default`, so control falls through to `return 0` — WITHOUT touching
    // `B` or `*out`. This is the classic bug class happy-path tests miss.
    let bad: [c_int; 16] = [
        -1,
        4,
        5,
        6,
        7,
        8,
        100,
        1000,
        -100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7FFF_FFFE,
        -0x4000_0000,
        0x0001_0000,
    ];
    let mut rng = Rng::new(SEED ^ 50);
    let shape = ShapeBuf::from_circle(&c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 2.0,
    });
    for &ty in &bad {
        for i in 0..64 {
            let a = any_ray(&mut rng);
            let c = run_cast(&l.c, a, &shape, None, ty);
            let r = run_cast(&l.rs, a, &shape, None, ty);
            assert_eq!(c.ret, 0, "err50 typeB={ty}: the C must return 0");
            assert!(
                c.out.iter().all(|&x| x == POISON),
                "err50 typeB={ty}: the C must not touch *out"
            );
            assert_same(&format!("err50 typeB={ty} #{i}"), &c, &r);

            // And with a transform supplied, which must also be ignored.
            let bx = rng.any_x();
            let c = run_cast(&l.c, a, &shape, Some(&bx), ty);
            let r = run_cast(&l.rs, a, &shape, Some(&bx), ty);
            assert_same(&format!("err50 typeB={ty} bx #{i}"), &c, &r);
        }
        // `B == NULL` is safe for an out-of-range type: the C never dereferences
        // it, because no `case` matches.
        let a = c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 1.0,
        };
        let cn = unsafe { (l.c.c2CastRay)(a, ptr::null(), ptr::null(), ty, ptr::null_mut()) };
        let rn = unsafe { (l.rs.c2CastRay)(a, ptr::null(), ptr::null(), ty, ptr::null_mut()) };
        assert_eq!(
            (cn, rn),
            (0, 0),
            "err50 typeB={ty}: NULL B and NULL out must both give 0"
        );
    }
    // Exhaustive sweep of the immediate neighbourhood of the valid range.
    for ty in -8i32..=12 {
        let a = c2Ray {
            p: c2v { x: -5.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        let c = run_cast(&l.c, a, &shape, None, ty);
        let r = run_cast(&l.rs, a, &shape, None, ty);
        assert_same(&format!("err50 neighbourhood typeB={ty}"), &c, &r);
        if !(0..=3).contains(&ty) {
            assert_eq!(c.ret, 0, "err50 typeB={ty} must be rejected");
            assert!(c.out.iter().all(|&x| x == POISON));
        }
    }
}

#[test]
fn err51_castray_ignores_bx_for_non_poly_types() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 51);
    for i in 0..2048 {
        let a = any_ray(&mut rng);
        let bx = rng.any_x();
        for (ty, shape) in [
            (
                C2_TYPE_CIRCLE,
                ShapeBuf::from_circle(&any_circle(&mut rng)),
            ),
            (C2_TYPE_AABB, ShapeBuf::from_aabb(&any_aabb(&mut rng))),
            (
                C2_TYPE_CAPSULE,
                ShapeBuf::from_capsule(&any_capsule(&mut rng)),
            ),
        ] {
            for api in [&l.c, &l.rs] {
                let without = run_cast(api, a, &shape, None, ty);
                let with = run_cast(api, a, &shape, Some(&bx), ty);
                assert_eq!(
                    without, with,
                    "err51 {} typeB={ty} #{i}: bx must be ignored",
                    api.tag
                );
            }
            assert_same(
                &format!("err51 typeB={ty} #{i}"),
                &run_cast(&l.c, a, &shape, Some(&bx), ty),
                &run_cast(&l.rs, a, &shape, Some(&bx), ty),
            );
        }
    }
}

#[test]
fn err52_castray_poly_inherits_poly_rejections() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 52);
    // Every c2RaytoPoly rejection must surface identically through c2CastRay.
    for &count in &[0i32, -1, i32::MIN, 4, 9, 16] {
        for i in 0..256 {
            let mut p = convex_ngon(&mut rng, 4);
            p.count = count;
            let buf = PolyBuf::from_poly(&p);
            let a = any_ray(&mut rng);
            let bx = rng.any_x();
            for use_bx in [false, true] {
                let mut b1 = OutBuf::poisoned();
                let mut b2 = OutBuf::poisoned();
                let bxp = if use_bx {
                    &bx as *const c2x
                } else {
                    ptr::null()
                };
                let r1 = unsafe {
                    (l.c.c2CastRay)(
                        a,
                        buf.as_ptr() as *const c_void,
                        bxp,
                        C2_TYPE_POLY,
                        b1.as_ptr(),
                    )
                };
                let r2 = unsafe {
                    (l.rs.c2CastRay)(
                        a,
                        buf.as_ptr() as *const c_void,
                        bxp,
                        C2_TYPE_POLY,
                        b2.as_ptr(),
                    )
                };
                assert!(
                    r1 == r2 && b1.bytes() == b2.bytes(),
                    "err52 count={count} #{i} use_bx={use_bx}: C ret={r1} RUST ret={r2}"
                );
                // Cross-check against the direct low-level call.
                let direct = run_poly_raw(
                    &l.c,
                    a,
                    &buf,
                    if use_bx { Some(&bx) } else { None },
                );
                assert_eq!(
                    direct.ret, r1,
                    "err52: c2CastRay(POLY) must agree with c2RaytoPoly"
                );
                assert_eq!(direct.out, b1.bytes(), "err52: payload mismatch");
            }
        }
    }
}

// ===========================================================================
// Arithmetic-degeneracy rows 53–56
// ===========================================================================

#[test]
fn err53_and_err54_div_by_zero_and_signed_zero() {
    let l = libs();
    let vecs = [
        c2v { x: 1.0, y: -2.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        c2v { x: QNAN, y: 3.0 },
        c2v {
            x: f32::MAX,
            y: f32::MIN,
        },
        c2v {
            x: f32::MIN_POSITIVE,
            y: -f32::MIN_POSITIVE,
        },
        c2v {
            x: f32::from_bits(1),
            y: f32::from_bits(0x8000_0001),
        },
    ];
    let divisors = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        QNAN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        1.0,
        -1.0,
    ];
    for &a in &vecs {
        for &b in &divisors {
            diff_eq!(
                format!("err53/54 c2Div a={} b={}", showv(a), show(b)),
                vb((l.c.c2Div)(a, b)),
                vb((l.rs.c2Div)(a, b))
            );
        }
        diff_eq!(
            format!("err53 c2Norm a={}", showv(a)),
            vb((l.c.c2Norm)(a)),
            vb((l.rs.c2Norm)(a))
        );
    }
    // The documented shapes: 1/0 = +inf so 0 * inf = NaN, and 1/-0 = -inf.
    let z = c2v { x: 0.0, y: 0.0 };
    let n = (l.c.c2Norm)(z);
    assert!(
        n.x.is_nan() && n.y.is_nan(),
        "err53: c2Norm((0,0)) must be NaN in the C, got {}",
        showv(n)
    );
    assert_eq!(vb(n), vb((l.rs.c2Norm)(z)));
    let d = (l.c.c2Div)(c2v { x: 1.0, y: -1.0 }, -0.0);
    assert_eq!(
        (fb(d.x), fb(d.y)),
        (fb(f32::NEG_INFINITY), fb(f32::INFINITY)),
        "err54: 1.0f/-0.0 must be -inf"
    );
    assert_eq!(vb(d), vb((l.rs.c2Div)(c2v { x: 1.0, y: -1.0 }, -0.0)));
}

#[test]
fn err55_len_of_nonfinite() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            diff_eq!(
                format!("err55 c2Len a={}", showv(a)),
                fb((l.c.c2Len)(a)),
                fb((l.rs.c2Len)(a))
            );
        }
    }
    // Overflow to +inf then sqrt(inf) == inf.
    let big = c2v {
        x: f32::MAX,
        y: f32::MAX,
    };
    assert_eq!(
        fb((l.c.c2Len)(big)),
        fb(f32::INFINITY),
        "err55: dot overflow must give sqrt(inf) == inf"
    );
    assert_eq!(fb((l.c.c2Len)(big)), fb((l.rs.c2Len)(big)));
}

#[test]
fn err56_minv_maxv_absv_are_ternaries_not_libm() {
    let l = libs();
    let nan = QNAN;
    // The asymmetry that distinguishes `a<b ? a : b` from `fminf`.
    let a_nan = c2v { x: nan, y: nan };
    let one = c2v { x: 1.0, y: 1.0 };
    let min_nan_first = (l.c.c2Minv)(a_nan, one);
    let min_nan_second = (l.c.c2Minv)(one, a_nan);
    assert_eq!(
        vb(min_nan_first),
        vb(one),
        "err56: c2Minv(NaN, 1) must be 1 (ternary picks b)"
    );
    assert!(
        min_nan_second.x.is_nan(),
        "err56: c2Minv(1, NaN) must be NaN (ternary picks b == NaN)"
    );
    assert_eq!(vb(min_nan_first), vb((l.rs.c2Minv)(a_nan, one)));
    assert_eq!(vb(min_nan_second), vb((l.rs.c2Minv)(one, a_nan)));

    let max_nan_first = (l.c.c2Maxv)(a_nan, one);
    let max_nan_second = (l.c.c2Maxv)(one, a_nan);
    assert_eq!(vb(max_nan_first), vb(one), "err56: c2Maxv(NaN, 1) must be 1");
    assert!(max_nan_second.x.is_nan(), "err56: c2Maxv(1, NaN) must be NaN");
    assert_eq!(vb(max_nan_first), vb((l.rs.c2Maxv)(a_nan, one)));
    assert_eq!(vb(max_nan_second), vb((l.rs.c2Maxv)(one, a_nan)));

    // `-0.0` must survive c2Absv (fabsf would return +0.0).
    let nz = c2v { x: -0.0, y: -0.0 };
    let abs_nz = (l.c.c2Absv)(nz);
    assert_eq!(
        (fb(abs_nz.x), fb(abs_nz.y)),
        (fb(-0.0), fb(-0.0)),
        "err56: `x<0 ? -x : x` leaves -0.0 as -0.0"
    );
    assert_eq!(vb(abs_nz), vb((l.rs.c2Absv)(nz)));

    // A negative NaN keeps its sign bit (fabsf would clear it).
    for &bits in NANS {
        let v = c2v {
            x: f32::from_bits(bits),
            y: f32::from_bits(bits),
        };
        diff_eq!(
            format!("err56 c2Absv nan {bits:#010x}"),
            vb((l.c.c2Absv)(v)),
            vb((l.rs.c2Absv)(v))
        );
    }
    let neg_nan = c2v {
        x: f32::from_bits(0xFFC0_0000),
        y: f32::from_bits(0xFFC0_0000),
    };
    let r = (l.c.c2Absv)(neg_nan);
    assert_eq!(
        fb(r.x),
        0xFFC0_0000,
        "err56: c2Absv must NOT clear a NaN's sign bit"
    );
    assert_eq!(vb(r), vb((l.rs.c2Absv)(neg_nan)));

    // Exhaustive special × special for all three.
    let sp = special_wide();
    for &ax in &sp {
        for &bx in &sp {
            let a = c2v { x: ax, y: bx };
            let b = c2v { x: bx, y: ax };
            diff_eq!(
                format!("err56 min {} {}", show(ax), show(bx)),
                vb((l.c.c2Minv)(a, b)),
                vb((l.rs.c2Minv)(a, b))
            );
            diff_eq!(
                format!("err56 max {} {}", show(ax), show(bx)),
                vb((l.c.c2Maxv)(a, b)),
                vb((l.rs.c2Maxv)(a, b))
            );
            diff_eq!(
                format!("err56 abs {}", showv(a)),
                vb((l.c.c2Absv)(a)),
                vb((l.rs.c2Absv)(a))
            );
        }
    }
}

// ===========================================================================
// poly_ray — row 57
// ===========================================================================

#[test]
fn err57_poly_ray_null_out_pointers() {
    let l = libs();
    // In the hard-coded scenario BOTH rays miss, so `poly_ray` never
    // dereferences either out pointer — passing NULL is therefore well defined
    // and must give the same answer from both libraries.
    let (cret, c1, c2) = run_poly_ray(&l.c);
    assert_eq!(cret, 0, "err57 precondition: the fixed scenario should miss");
    assert!(
        c1.iter().all(|&b| b == POISON) && c2.iter().all(|&b| b == POISON),
        "err57 precondition: neither out buffer should have been written"
    );

    for _ in 0..16 {
        let cn = unsafe { (l.c.poly_ray)(ptr::null_mut(), ptr::null_mut()) };
        let rn = unsafe { (l.rs.poly_ray)(ptr::null_mut(), ptr::null_mut()) };
        assert_eq!(cn, rn, "err57: NULL out pointers must agree");
        assert_eq!(cn, 0);
    }
    // One NULL, one valid.
    for _ in 0..16 {
        let mut b = OutBuf::poisoned();
        let cn = unsafe { (l.c.poly_ray)(ptr::null_mut(), b.as_ptr()) };
        let cb = b.bytes();
        let mut b2 = OutBuf::poisoned();
        let rn = unsafe { (l.rs.poly_ray)(ptr::null_mut(), b2.as_ptr()) };
        assert_eq!(cn, rn);
        assert_eq!(cb, b2.bytes());

        let mut b = OutBuf::poisoned();
        let cn = unsafe { (l.c.poly_ray)(b.as_ptr(), ptr::null_mut()) };
        let cb = b.bytes();
        let mut b2 = OutBuf::poisoned();
        let rn = unsafe { (l.rs.poly_ray)(b2.as_ptr(), ptr::null_mut()) };
        assert_eq!(cn, rn);
        assert_eq!(cb, b2.bytes());
    }
    // Aliasing: the same buffer for both out params.
    let mut b = OutBuf::poisoned();
    let cn = unsafe { (l.c.poly_ray)(b.as_ptr(), b.as_ptr()) };
    let cb = b.bytes();
    let mut b2 = OutBuf::poisoned();
    let rn = unsafe { (l.rs.poly_ray)(b2.as_ptr(), b2.as_ptr()) };
    assert_eq!(cn, rn, "err57 aliased out params");
    assert_eq!(cb, b2.bytes());
}
