//! Phase C — error / rejection path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md`. Each block
//! constructs the exact invalid input, asserts the C produces the sentinel
//! `ERRORS.md` claims, and asserts the Rust produces the *same* sentinel and
//! the same out-param state — not merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Asserts the C returned exactly `want`, so a row cannot pass because the
/// trigger was never actually reached.
fn expect_c(row: &str, got: c_int, want: c_int) {
    assert_eq!(got, want, "[{row}] C did not take the expected branch");
}

fn untouched(row: &str, who: &str, out: c2Raycast) {
    assert!(
        cast_eq(out, POISON),
        "[{row}] {who} wrote to *out but ERRORS.md says it must be left untouched: {}",
        fmt_cast(out)
    );
}

// ===========================================================================
// Rows 1-5 — c2RaytoCircle
// ===========================================================================

#[test]
fn rows01_05_circle_rejections() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE01);
    unsafe {
        // Row 1: disc < 0 — ray line misses the circle entirely.
        for _ in 0..3000 {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.1, 3.0) };
            let dir = rng.v_dir();
            let perp = c2v { x: -dir.y, y: dir.x };
            let off = B.r + rng.range(0.5, 20.0);
            let A = c2Ray {
                p: c2v {
                    x: B.p.x + perp.x * off - dir.x * 10.0,
                    y: B.p.y + perp.y * off - dir.y * 10.0,
                },
                d: dir,
                t: 1000.0,
            };
            let cr = call_circle(&p.c, A, B);
            expect_c("row 1", cr.0, 0);
            untouched("row 1", "C", cr.1);
            d.ray("row 1: disc < 0", cr, call_circle(&p.rs, A, B));
        }

        // Row 2: t < 0 — the circle is behind the ray origin, or the origin is
        // inside it (`c < 0` makes `-b - sqrt(disc)` negative).
        for _ in 0..3000 {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.5, 6.0) };
            let dir = rng.v_dir();
            // origin strictly inside
            let A = c2Ray { p: B.p, d: dir, t: 1000.0 };
            let cr = call_circle(&p.c, A, B);
            expect_c("row 2 (inside)", cr.0, 0);
            untouched("row 2", "C", cr.1);
            d.ray("row 2: origin inside", cr, call_circle(&p.rs, A, B));

            // circle entirely behind the origin
            let back = B.r + rng.range(1.0, 20.0);
            let A2 = c2Ray {
                p: c2v { x: B.p.x + dir.x * back, y: B.p.y + dir.y * back },
                d: dir,
                t: 1000.0,
            };
            let cr2 = call_circle(&p.c, A2, B);
            expect_c("row 2 (behind)", cr2.0, 0);
            untouched("row 2", "C", cr2.1);
            d.ray("row 2: circle behind", cr2, call_circle(&p.rs, A2, B));
        }

        // Row 3: a hit exists but lies beyond A.t.
        for _ in 0..3000 {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.5, 6.0) };
            let dir = rng.v_dir();
            let dist = B.r + rng.range(2.0, 20.0);
            let A_full = c2Ray {
                p: c2v { x: B.p.x - dir.x * dist, y: B.p.y - dir.y * dist },
                d: dir,
                t: 1.0e6,
            };
            let (hit, out) = call_circle(&p.c, A_full, B);
            expect_c("row 3 (setup must hit)", hit, 1);
            // now shorten the ray to just under the hit distance
            let short = f32::from_bits(out.t.to_bits() - 1);
            let A = c2Ray { p: A_full.p, d: dir, t: short };
            let cr = call_circle(&p.c, A, B);
            expect_c("row 3", cr.0, 0);
            untouched("row 3", "C", cr.1);
            d.ray("row 3: t > A.t", cr, call_circle(&p.rs, A, B));
            // exactly at the hit distance must still hit (boundary is `<=`)
            let A_eq = c2Ray { p: A_full.p, d: dir, t: out.t };
            let ce = call_circle(&p.c, A_eq, B);
            expect_c("row 3 (t == hit)", ce.0, 1);
            d.ray("row 3: t == hit", ce, call_circle(&p.rs, A_eq, B));
        }

        // Row 4: negative radius — `B.r * B.r` is positive, so the C treats a
        // negative radius exactly like its absolute value.
        for _ in 0..2000 {
            let r = rng.range(0.5, 6.0);
            let centre = rng.v_small();
            let dir = rng.v_dir();
            let dist = r + rng.range(1.0, 20.0);
            let A = c2Ray {
                p: c2v { x: centre.x - dir.x * dist, y: centre.y - dir.y * dist },
                d: dir,
                t: 1.0e6,
            };
            let pos = c2Circle { p: centre, r };
            let neg = c2Circle { p: centre, r: -r };
            let cpos = call_circle(&p.c, A, pos);
            let cneg = call_circle(&p.c, A, neg);
            d.ray("row 4: C r == -r", cpos, cneg);
            d.ray("row 4: neg radius", cneg, call_circle(&p.rs, A, neg));
        }

        // Row 5: NaN inputs — `disc < 0` is false, `sqrtf(NaN)` is NaN and
        // `NaN >= 0` is false, so the C returns 0 without touching *out*.
        for _ in 0..3000 {
            let A = c2Ray { p: rng.v_mixed(), d: rng.v_mixed(), t: rng.f_mixed() };
            let B = c2Circle { p: rng.v_mixed(), r: rng.f_mixed() };
            let cr = call_circle(&p.c, A, B);
            d.ray("row 5: non-finite", cr, call_circle(&p.rs, A, B));
        }
        // targeted: force disc to be NaN via inf - inf
        for t in [0.0f32, 1.0, f32::INFINITY, f32::NAN] {
            let A = c2Ray {
                p: c2v { x: f32::INFINITY, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t,
            };
            let B = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::INFINITY };
            let cr = call_circle(&p.c, A, B);
            expect_c("row 5 (disc NaN)", cr.0, 0);
            untouched("row 5", "C", cr.1);
            d.ray("row 5: disc NaN", cr, call_circle(&p.rs, A, B));
        }
    }
    d.finish("ERRORS rows 1-5: c2RaytoCircle rejections");
}

// ===========================================================================
// Rows 6-10 — c2AABBtoAABB
// ===========================================================================

#[test]
fn rows06_10_aabbtoaabb_rejections() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE06);
    unsafe {
        let base = |rng: &mut Rng| {
            let x = rng.sym(10.0);
            let y = rng.sym(10.0);
            c2AABB {
                min: c2v { x, y },
                max: c2v { x: x + rng.range(0.5, 6.0), y: y + rng.range(0.5, 6.0) },
            }
        };
        for _ in 0..3000 {
            let A = base(&mut rng);
            let w = A.max.x - A.min.x;
            let h = A.max.y - A.min.y;
            let gap = rng.range(0.01, 10.0);
            // Row 6: B.max.x < A.min.x
            // Row 7: A.max.x < B.min.x
            // Row 8: B.max.y < A.min.y
            // Row 9: A.max.y < B.min.y
            let cases = [
                ("row 6", c2v { x: -(w + gap), y: 0.0 }),
                ("row 7", c2v { x: w + gap, y: 0.0 }),
                ("row 8", c2v { x: 0.0, y: -(h + gap) }),
                ("row 9", c2v { x: 0.0, y: h + gap }),
            ];
            for (row, sh) in cases {
                let B = c2AABB {
                    min: c2v { x: A.min.x + sh.x, y: A.min.y + sh.y },
                    max: c2v { x: A.max.x + sh.x, y: A.max.y + sh.y },
                };
                let cv = (p.c.c2AABBtoAABB)(A, B);
                expect_c(row, cv, 0);
                d.int(row, cv, (p.rs.c2AABBtoAABB)(A, B));
            }
        }
        // Row 10: any NaN coordinate makes all four `<` false, so the C
        // reports the boxes as OVERLAPPING.
        let sane = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        for slot in 0..8 {
            let mut A = sane;
            let mut B = c2AABB {
                min: c2v { x: 100.0, y: 100.0 },
                max: c2v { x: 200.0, y: 200.0 },
            };
            let n = f32::NAN;
            match slot {
                0 => A.min.x = n,
                1 => A.min.y = n,
                2 => A.max.x = n,
                3 => A.max.y = n,
                4 => B.min.x = n,
                5 => B.min.y = n,
                6 => B.max.x = n,
                _ => B.max.y = n,
            }
            let cv = (p.c.c2AABBtoAABB)(A, B);
            d.int(&format!("row 10 (NaN slot {slot})"), cv, (p.rs.c2AABBtoAABB)(A, B));
        }
        // all-NaN must be reported as overlapping (returns 1)
        let nanbox = c2AABB {
            min: c2v { x: f32::NAN, y: f32::NAN },
            max: c2v { x: f32::NAN, y: f32::NAN },
        };
        let cv = (p.c.c2AABBtoAABB)(nanbox, nanbox);
        expect_c("row 10 (all NaN)", cv, 1);
        d.int("row 10: all NaN", cv, (p.rs.c2AABBtoAABB)(nanbox, nanbox));
    }
    d.finish("ERRORS rows 6-10: c2AABBtoAABB rejections");
}

// ===========================================================================
// Rows 11-18 — c2RaytoAABB (incl. the static c2RayToPlane_OneDimensional,
// which is not exported and can only be driven through c2RaytoAABB)
// ===========================================================================

#[test]
fn rows11_18_aabb_ray_rejections() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE11);
    unsafe {
        // Row 14: the ray's own AABB does not overlap B at all.
        for _ in 0..3000 {
            let B = c2AABB {
                min: c2v { x: -1.0, y: -1.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            let far = 50.0 + rng.range(0.0, 100.0);
            let dir = rng.v_dir();
            let A = c2Ray {
                p: c2v { x: far, y: far },
                d: dir,
                t: 1.0,
            };
            let cr = call_aabb(&p.c, A, B);
            expect_c("row 14", cr.0, 0);
            untouched("row 14", "C", cr.1);
            d.ray("row 14: ray bbox disjoint", cr, call_aabb(&p.rs, A, B));
        }

        // Row 15: bboxes overlap but the separating-axis test `d > 0` rejects.
        // A long diagonal ray whose bbox contains the box but whose line
        // passes outside it.
        let B = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let mut row15_hits = 0;
        for _ in 0..4000 {
            let s = rng.range(3.0, 40.0);
            // start bottom-left, end top-right, but shifted so the line misses
            let shift = rng.range(2.5, 10.0) * if rng.bool() { 1.0 } else { -1.0 };
            let A = c2Ray {
                p: c2v { x: -s + shift, y: -s },
                d: c2Norm_local(c2v { x: 1.0, y: 1.0 }),
                t: 2.0 * s * 1.5,
            };
            let cr = call_aabb(&p.c, A, B);
            if cr.0 == 0 && cast_eq(cr.1, POISON) {
                row15_hits += 1;
            }
            d.ray("row 15: SAT reject", cr, call_aabb(&p.rs, A, B));
        }
        assert!(row15_hits > 0, "[row 15] never produced a SAT rejection");

        // Row 16: every t_i > 1.0 -> hit == 0.  Row 11/12/13: the three exits of
        // c2RayToPlane_OneDimensional. All are driven by placing the ray so
        // that `da < 0`, `da*db > 0`, or `da == db`.
        //   da == db happens whenever p0 and p1 have the same coordinate on the
        //   tested axis, i.e. for axis-aligned rays and for A.t == 0.
        for _ in 0..3000 {
            let bx = rng.sym(6.0);
            let by = rng.sym(6.0);
            let Bb = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + rng.range(0.5, 6.0), y: by + rng.range(0.5, 6.0) },
            };
            for dir in AXIS_DIRS {
                // rows 11-13: axis-aligned -> da == db on the perpendicular axis
                for off in [0.0f32, 0.5, 1.0, -1.0, 2.0] {
                    let cxx = (Bb.min.x + Bb.max.x) * 0.5;
                    let cyy = (Bb.min.y + Bb.max.y) * 0.5;
                    let origin = if dir.x != 0.0 {
                        c2v { x: cxx - dir.x * 10.0, y: cyy + off * (Bb.max.y - Bb.min.y) }
                    } else {
                        c2v { x: cxx + off * (Bb.max.x - Bb.min.x), y: cyy - dir.y * 10.0 }
                    };
                    for t in [0.0f32, 1.0, 10.0, 20.0] {
                        let A = c2Ray { p: origin, d: dir, t };
                        d.ray(
                            "rows 11-13,16: plane-1d exits",
                            call_aabb(&p.c, A, Bb),
                            call_aabb(&p.rs, A, Bb),
                        );
                    }
                }
            }
        }

        // Row 17: inverted box (min > max) — no validation in the C.
        for _ in 0..3000 {
            let Bi = c2AABB { min: rng.v_small(), max: rng.v_small() };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 30.0) };
            d.ray("row 17: inverted box", call_aabb(&p.c, A, Bi), call_aabb(&p.rs, A, Bi));
        }
        // guaranteed-inverted
        for _ in 0..2000 {
            let hi = rng.v_small();
            let Bi = c2AABB {
                min: c2v { x: hi.x + rng.range(0.1, 5.0), y: hi.y + rng.range(0.1, 5.0) },
                max: hi,
            };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 30.0) };
            d.ray("row 17: strictly inverted", call_aabb(&p.c, A, Bi), call_aabb(&p.rs, A, Bi));
        }

        // Row 18: A.t == 0 -> p1 == p0, ab == 0, n == 0, d = -dot(0, he) == -0.
        for _ in 0..3000 {
            let bx = rng.sym(6.0);
            let by = rng.sym(6.0);
            let Bb = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + rng.range(0.0, 6.0), y: by + rng.range(0.0, 6.0) },
            };
            for q in [
                c2v { x: (Bb.min.x + Bb.max.x) * 0.5, y: (Bb.min.y + Bb.max.y) * 0.5 },
                Bb.min,
                Bb.max,
                rng.v_small(),
            ] {
                let A = c2Ray { p: q, d: rng.v_dir(), t: 0.0 };
                d.ray("row 18: A.t == 0", call_aabb(&p.c, A, Bb), call_aabb(&p.rs, A, Bb));
            }
        }
    }
    d.finish("ERRORS rows 11-18: c2RaytoAABB rejections");
}

fn c2Norm_local(v: c2v) -> c2v {
    let l = (v.x * v.x + v.y * v.y).sqrt();
    c2v { x: v.x / l, y: v.y / l }
}

// ===========================================================================
// Rows 19-26 — point predicates
// ===========================================================================

#[test]
fn rows19_26_point_predicates() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE19);
    unsafe {
        for _ in 0..3000 {
            let x = rng.sym(10.0);
            let y = rng.sym(10.0);
            let A = c2AABB {
                min: c2v { x, y },
                max: c2v { x: x + rng.range(0.5, 6.0), y: y + rng.range(0.5, 6.0) },
            };
            let g = rng.range(0.01, 5.0);
            // Rows 19-22, one per rejecting comparison
            let cases = [
                ("row 19", c2v { x: A.min.x - g, y: A.min.y }),
                ("row 20", c2v { x: A.min.x, y: A.min.y - g }),
                ("row 21", c2v { x: A.max.x + g, y: A.max.y }),
                ("row 22", c2v { x: A.max.x, y: A.max.y + g }),
            ];
            for (row, q) in cases {
                let cv = (p.c.c2AABBtoPoint)(A, q);
                expect_c(row, cv, 0);
                d.int(row, cv, (p.rs.c2AABBtoPoint)(A, q));
            }
        }
        // Row 23: NaN in any slot -> all four comparisons false -> returns 1.
        let sane = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let outside = c2v { x: 100.0, y: 100.0 };
        for slot in 0..6 {
            let mut A = sane;
            let mut q = outside;
            let n = f32::NAN;
            match slot {
                0 => A.min.x = n,
                1 => A.min.y = n,
                2 => A.max.x = n,
                3 => A.max.y = n,
                4 => q.x = n,
                _ => q.y = n,
            }
            let cv = (p.c.c2AABBtoPoint)(A, q);
            d.int(&format!("row 23 (NaN slot {slot})"), cv, (p.rs.c2AABBtoPoint)(A, q));
        }
        let nq = c2v { x: f32::NAN, y: f32::NAN };
        let cv = (p.c.c2AABBtoPoint)(sane, nq);
        expect_c("row 23 (point all NaN)", cv, 1);
        d.int("row 23: point all NaN", cv, (p.rs.c2AABBtoPoint)(sane, nq));

        // Row 24: point exactly on the rim is REJECTED (strict `<`).
        for r in [1.0f32, 2.0, 3.0, 4.0, 0.5, 8.0, 16.0] {
            let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r };
            for q in [
                c2v { x: r, y: 0.0 },
                c2v { x: -r, y: 0.0 },
                c2v { x: 0.0, y: r },
                c2v { x: 0.0, y: -r },
            ] {
                let cv = (p.c.c2CircleToPoint)(A, q);
                expect_c("row 24 (on rim)", cv, 0);
                d.int("row 24: point on rim", cv, (p.rs.c2CircleToPoint)(A, q));
            }
            // strictly outside
            let q = c2v { x: r + 1.0, y: 0.0 };
            let cv = (p.c.c2CircleToPoint)(A, q);
            expect_c("row 24 (outside)", cv, 0);
            d.int("row 24: outside", cv, (p.rs.c2CircleToPoint)(A, q));
        }

        // Row 25: r == 0 can never contain a point (`d2 < 0` is impossible).
        for _ in 0..2000 {
            for r in [0.0f32, -0.0] {
                let A = c2Circle { p: rng.v_small(), r };
                for q in [A.p, rng.v_small(), rng.v_mixed()] {
                    let cv = (p.c.c2CircleToPoint)(A, q);
                    if q.x.is_finite() && q.y.is_finite() {
                        expect_c("row 25 (r == 0)", cv, 0);
                    }
                    d.int("row 25: r == 0", cv, (p.rs.c2CircleToPoint)(A, q));
                }
            }
        }

        // Row 26: r < 0 behaves like |r| because only `r*r` is used.
        for _ in 0..2000 {
            let centre = rng.v_small();
            let r = rng.range(0.5, 6.0);
            let pos = c2Circle { p: centre, r };
            let neg = c2Circle { p: centre, r: -r };
            let ang = rng.range(-7.0, 7.0);
            for k in [0.0f32, 0.5, 0.999, 1.0, 1.001, 2.0] {
                let q = c2v {
                    x: centre.x + r * k * ang.cos(),
                    y: centre.y + r * k * ang.sin(),
                };
                let cp = (p.c.c2CircleToPoint)(pos, q);
                let cn = (p.c.c2CircleToPoint)(neg, q);
                d.int("row 26: C r == -r", cp, cn);
                d.int("row 26: neg radius", cn, (p.rs.c2CircleToPoint)(neg, q));
            }
        }
    }
    d.finish("ERRORS rows 19-26: point predicates");
}

// ===========================================================================
// Rows 27-31 — c2RaytoCapsule
// ===========================================================================

#[test]
fn rows27_31_capsule_rejections() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE27);
    unsafe {
        // Row 27: a == b -> c2Norm(0) = (NaN, NaN), and `yBb`/`yAp` become NaN
        // too. c2AABBtoPoint then compares against NaN, every `<`/`>` is false,
        // and it reports the point as INSIDE -> the C returns 1 with
        // out = { t: 0, n: (NaN, NaN) }.
        for _ in 0..3000 {
            let q = rng.v_small();
            let B = c2Capsule { a: q, b: q, r: rng.range(-2.0, 6.0) };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 30.0) };
            let cr = call_capsule(&p.c, A, B);
            expect_c("row 27", cr.0, 1);
            assert!(
                cr.1.n.x.is_nan() && cr.1.n.y.is_nan() && cr.1.t.to_bits() == 0,
                "[row 27] expected C to leave out = {{t:0, n:(NaN,NaN)}}, got {}",
                fmt_cast(cr.1)
            );
            d.ray("row 27: a == b", cr, call_capsule(&p.rs, A, B));
            // also with the ray origin exactly at the degenerate point
            let A2 = c2Ray { p: q, d: rng.v_dir(), t: rng.range(0.0, 30.0) };
            let cr2 = call_capsule(&p.c, A2, B);
            expect_c("row 27 (origin at a)", cr2.0, 1);
            d.ray("row 27: a == b, origin at a", cr2, call_capsule(&p.rs, A2, B));
        }

        // Row 28: the final `return 0` — but *out* HAS been written with
        // n = c2Norm(b - a) and t = 0. This is the subtle one: a translation
        // that only wrote *out* on success would pass every hit test and fail
        // here.
        let mut row28_seen = 0;
        for _ in 0..8000 {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.5, 10.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r: rng.range(0.1, 2.0),
            };
            // far away, pointing away from the capsule
            let away = rng.v_dir();
            let A = c2Ray {
                p: c2v { x: a.x + away.x * 200.0, y: a.y + away.y * 200.0 },
                d: away,
                t: rng.range(0.0, 10.0),
            };
            let cr = call_capsule(&p.c, A, B);
            if cr.0 == 0 && !cast_eq(cr.1, POISON) {
                row28_seen += 1;
                assert_eq!(cr.1.t.to_bits(), 0, "[row 28] expected out.t == 0");
            }
            d.ray("row 28: fall-through writes *out*", cr, call_capsule(&p.rs, A, B));
        }
        assert!(row28_seen > 0, "[row 28] never reached the final `return 0`");

        // Row 29: delegates to c2RaytoCircle, which rejects.
        let mut row29_seen = 0;
        for _ in 0..8000 {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.5, 10.0);
            let r = rng.range(0.1, 2.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r,
            };
            // along the axis, beyond the far cap, pointing further away, so the
            // |yAp.x| < r branch is taken and the circle cast then rejects
            let axis = c2v { x: (B.b.x - a.x) / len, y: (B.b.y - a.y) / len };
            let A = c2Ray {
                p: c2v {
                    x: B.b.x + axis.x * (r + rng.range(0.5, 20.0)),
                    y: B.b.y + axis.y * (r + rng.range(0.5, 20.0)),
                },
                d: axis,
                t: rng.range(0.0, 20.0),
            };
            let cr = call_capsule(&p.c, A, B);
            if cr.0 == 0 {
                row29_seen += 1;
            }
            d.ray("row 29: delegate rejects", cr, call_capsule(&p.rs, A, B));
        }
        assert!(row29_seen > 0, "[row 29] delegate never rejected");

        // Row 30: d = yAe.x - yAp.x == 0 -> t = (c - yAp.x)/0.
        // Happens whenever the ray direction is parallel to the capsule axis
        // (so its local x-component is 0) or A.t == 0.
        for _ in 0..3000 {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.5, 10.0);
            let axis = c2v { x: ang.cos(), y: ang.sin() };
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * axis.x, y: a.y + len * axis.y },
                r: rng.range(0.1, 2.0),
            };
            for tv in [0.0f32, 1.0, 20.0] {
                // origin off to the side by more than r, direction along axis
                let perp = c2v { x: -axis.y, y: axis.x };
                let off = B.r + rng.range(0.01, 5.0);
                let A = c2Ray {
                    p: c2v { x: a.x + perp.x * off, y: a.y + perp.y * off },
                    d: axis,
                    t: tv,
                };
                d.ray("row 30: zero denominator", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                // and t == 0 with the origin anywhere
                let A2 = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: 0.0 };
                d.ray("row 30: A.t == 0", call_capsule(&p.c, A2, B), call_capsule(&p.rs, A2, B));
            }
        }

        // Row 31: r < 0 -> capsule_bb.min.x = -r > r = max.x (inverted), so the
        // c2AABBtoPoint early exit can never fire.
        for _ in 0..3000 {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.5, 10.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r: -rng.range(0.1, 4.0),
            };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 30.0) };
            d.ray("row 31: negative radius", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
    }
    d.finish("ERRORS rows 27-31: c2RaytoCapsule rejections");
}

// ===========================================================================
// Rows 32-40 — c2RaytoPoly
// ===========================================================================

#[test]
fn rows32_40_poly_rejections() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE32);
    unsafe {
        // Row 32: den == 0 && num < 0. An axis-aligned quad plus an
        // axis-aligned ray starting outside the perpendicular slab gives this
        // exactly.
        let mut row32 = 0;
        for _ in 0..4000 {
            let poly = make_axis_quad(&mut rng);
            let c = poly_centroid(&poly);
            let hh = poly.verts[1].y - c.y;
            let A = c2Ray {
                p: c2v { x: c.x - 50.0, y: c.y + hh * rng.range(1.5, 10.0) },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            };
            let cr = call_poly(&p.c, A, &poly, None);
            if cr.0 == 0 && cast_eq(cr.1, POISON) {
                row32 += 1;
            }
            expect_c("row 32", cr.0, 0);
            untouched("row 32", "C", cr.1);
            d.ray("row 32: den == 0 && num < 0", cr, call_poly(&p.rs, A, &poly, None));
        }
        assert!(row32 > 0, "[row 32] never triggered");

        // Row 33: hi < lo. Reached by rays that miss a convex hull sideways.
        let mut row33 = 0;
        for _ in 0..8000 {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            let cr = call_poly(&p.c, A, &poly, None);
            if cr.0 == 0 && cast_eq(cr.1, POISON) {
                row33 += 1;
            }
            d.ray("row 33: hi < lo", cr, call_poly(&p.rs, A, &poly, None));
        }
        assert!(row33 > 0, "[row 33] never produced a rejection");

        // Row 34: loop completes with index == ~0 (origin inside the hull).
        for _ in 0..4000 {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let A = c2Ray {
                p: poly_centroid(&poly),
                d: rng.v_dir(),
                t: rng.range(0.0, 40.0),
            };
            let cr = call_poly(&p.c, A, &poly, None);
            expect_c("row 34", cr.0, 0);
            untouched("row 34", "C", cr.1);
            d.ray("row 34: index == ~0", cr, call_poly(&p.rs, A, &poly, None));
        }

        // Row 35: count == 0.
        for _ in 0..2000 {
            let mut poly = make_convex_poly(&mut rng, 4);
            poly.count = 0;
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(-10.0, 40.0) };
            for bx in [None, Some(c2x { p: rng.v_small(), r: rng.rot_unit() })] {
                let cr = call_poly(&p.c, A, &poly, bx.as_ref());
                expect_c("row 35", cr.0, 0);
                untouched("row 35", "C", cr.1);
                d.ray("row 35: count == 0", cr, call_poly(&p.rs, A, &poly, bx.as_ref()));
            }
        }

        // Row 36: negative count, including INT_MIN.
        for cnt in [-1i32, -2, -100, i32::MIN, i32::MIN + 1] {
            for _ in 0..400 {
                let mut poly = make_convex_poly(&mut rng, 4);
                poly.count = cnt;
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(-10.0, 40.0) };
                let cr = call_poly(&p.c, A, &poly, None);
                expect_c("row 36", cr.0, 0);
                untouched("row 36", "C", cr.1);
                d.ray(
                    &format!("row 36: count == {cnt}"),
                    cr,
                    call_poly(&p.rs, A, &poly, None),
                );
            }
        }

        // Row 37: bx == NULL is the library's only null check.
        let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
        for _ in 0..4000 {
            let count = 1 + rng.below(8);
            let poly = make_convex_poly(&mut rng, count);
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            let c_null = call_poly(&p.c, A, &poly, None);
            let c_id = call_poly(&p.c, A, &poly, Some(&ident));
            d.ray("row 37: C NULL == identity", c_null, c_id);
            d.ray("row 37: RS NULL", c_null, call_poly(&p.rs, A, &poly, None));
            d.ray("row 37: RS identity", c_id, call_poly(&p.rs, A, &poly, Some(&ident)));
        }

        // Rows 38-39: A.t == 0 and A.t < 0 -> hi <= lo == 0.
        for _ in 0..3000 {
            let count = 1 + rng.below(8);
            let poly = make_convex_poly(&mut rng, count);
            for t in [0.0f32, -0.0, -1.0e-30, -1.0, -1.0e30] {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t };
                let cr = call_poly(&p.c, A, &poly, None);
                expect_c(
                    if t == 0.0 { "row 38" } else { "row 39" },
                    cr.0,
                    0,
                );
                untouched("rows 38-39", "C", cr.1);
                d.ray("rows 38-39: A.t <= 0", cr, call_poly(&p.rs, A, &poly, None));
            }
        }

        // Row 40: degenerate / non-normalised bx.r.
        for _ in 0..3000 {
            let count = 1 + rng.below(8);
            let poly = make_convex_poly(&mut rng, count);
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            let bxs = [
                c2x { p: rng.v_small(), r: c2r { c: 0.0, s: 0.0 } },
                c2x { p: rng.v_small(), r: c2r { c: rng.sym(5.0), s: rng.sym(5.0) } },
                c2x { p: rng.v_mixed(), r: c2r { c: f32::NAN, s: 1.0 } },
                c2x { p: c2v { x: f32::INFINITY, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
            ];
            for bx in bxs {
                d.ray(
                    "row 40: degenerate bx.r",
                    call_poly(&p.c, A, &poly, Some(&bx)),
                    call_poly(&p.rs, A, &poly, Some(&bx)),
                );
            }
        }
    }
    d.finish("ERRORS rows 32-40: c2RaytoPoly rejections");
}

// ===========================================================================
// Rows 41-43 — c2CastRay, including out-of-range enum values
// ===========================================================================

/// Row 41. `C2_TYPE` is a C enum, so *any* `int` is a valid argument at the FFI
/// boundary. The `switch` has no `default`, so an unknown value must fall
/// through to `return 0` WITHOUT dereferencing `B` — which is proved here by
/// passing a NULL shape pointer.
#[test]
fn row41_cast_ray_invalid_enum() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE41);
    let bad: [c_int; 14] = [
        4,
        5,
        6,
        100,
        -1,
        -2,
        -100,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX,
        c_int::MAX - 1,
        0x7fff_fffe,
        1 << 16,
        -(1 << 16),
    ];
    unsafe {
        for &ty in &bad {
            for _ in 0..200 {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
                // NULL shape: only safe because the C never dereferences it for
                // an unknown type. If it did, both would crash identically.
                let cr = call_cast(&p.c, A, std::ptr::null(), None, ty);
                expect_c("row 41", cr.0, 0);
                untouched("row 41", "C", cr.1);
                let rr = call_cast(&p.rs, A, std::ptr::null(), None, ty);
                d.ray(&format!("row 41: typeB = {ty}, B = NULL"), cr, rr);

                // and with a real shape behind B plus a real bx
                let poly = make_convex_poly(&mut rng, 4);
                let bx = c2x { p: rng.v_small(), r: rng.rot_unit() };
                let sp = &poly as *const c2Poly as *const c_void;
                let cr2 = call_cast(&p.c, A, sp, Some(&bx), ty);
                expect_c("row 41 (with shape)", cr2.0, 0);
                untouched("row 41", "C", cr2.1);
                d.ray(
                    &format!("row 41: typeB = {ty}, real shape"),
                    cr2,
                    call_cast(&p.rs, A, sp, Some(&bx), ty),
                );
            }
        }
        // exhaustive sweep over a window around the valid range
        for ty in -64i32..=64 {
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            let poly = make_axis_quad(&mut rng);
            let sp = &poly as *const c2Poly as *const c_void;
            let cr = call_cast(&p.c, A, sp, None, ty);
            if !(0..=3).contains(&ty) {
                expect_c("row 41 (sweep)", cr.0, 0);
                untouched("row 41", "C", cr.1);
            }
            d.ray(&format!("row 41: sweep typeB = {ty}"), cr, call_cast(&p.rs, A, sp, None, ty));
        }
    }
    d.finish("ERRORS row 41: c2CastRay out-of-range enum");
}

/// Rows 42-43: `bx == NULL` through the dispatcher, and faithful propagation
/// of every delegate's rejection sentinel.
#[test]
fn rows42_43_cast_ray_propagation() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE42);
    unsafe {
        for _ in 0..4000 {
            let A = c2Ray {
                p: if rng.below(5) == 0 { rng.v_mixed() } else { rng.v_small() },
                d: if rng.below(5) == 0 { rng.v_mixed() } else { rng.v_dir() },
                t: if rng.below(5) == 0 { rng.f_mixed() } else { rng.range(-5.0, 40.0) },
            };

            // Row 42: POLY + NULL bx
            let poly = { let n = 1 + rng.below(8); make_convex_poly(&mut rng, n) };
            let sp = &poly as *const c2Poly as *const c_void;
            d.ray(
                "row 42: POLY + NULL bx",
                call_cast(&p.c, A, sp, None, C2_TYPE_POLY),
                call_cast(&p.rs, A, sp, None, C2_TYPE_POLY),
            );

            // Row 43: each arm's rejection must propagate unchanged
            let circle = c2Circle { p: rng.v_small(), r: rng.range(-2.0, 4.0) };
            let cp = &circle as *const c2Circle as *const c_void;
            let cc = call_cast(&p.c, A, cp, None, C2_TYPE_CIRCLE);
            d.ray("row 43: CIRCLE propagate", cc, call_cast(&p.rs, A, cp, None, C2_TYPE_CIRCLE));
            d.ray("row 43: CIRCLE == direct", cc, call_circle(&p.c, A, circle));

            let boxx = c2AABB { min: rng.v_small(), max: rng.v_small() };
            let bp = &boxx as *const c2AABB as *const c_void;
            let cb = call_cast(&p.c, A, bp, None, C2_TYPE_AABB);
            d.ray("row 43: AABB propagate", cb, call_cast(&p.rs, A, bp, None, C2_TYPE_AABB));
            d.ray("row 43: AABB == direct", cb, call_aabb(&p.c, A, boxx));

            let q = rng.v_small();
            let cap = c2Capsule { a: q, b: q, r: rng.range(-2.0, 4.0) };
            let pp = &cap as *const c2Capsule as *const c_void;
            let ccap = call_cast(&p.c, A, pp, None, C2_TYPE_CAPSULE);
            d.ray("row 43: CAPSULE propagate", ccap, call_cast(&p.rs, A, pp, None, C2_TYPE_CAPSULE));
            d.ray("row 43: CAPSULE == direct", ccap, call_capsule(&p.c, A, cap));
        }
    }
    d.finish("ERRORS rows 42-43: c2CastRay propagation");
}

// ===========================================================================
// Rows 44-49 — arithmetic domain edges
// ===========================================================================

#[test]
fn rows44_49_arithmetic_edges() {
    let p = load_pair();
    let mut d = Diff::new();
    unsafe {
        // Row 44: c2Div by zero, and c2Norm of a zero-length vector.
        for &b in &[0.0f32, -0.0] {
            for &vx in WEIRD {
                for &vy in WEIRD {
                    let a = c2v { x: vx, y: vy };
                    d.vec("row 44: c2Div by 0", (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b));
                }
            }
        }
        let zeros = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: -0.0, y: -0.0 },
        ];
        for z in zeros {
            let cv = (p.c.c2Norm)(z);
            assert!(
                cv.x.is_nan() && cv.y.is_nan(),
                "[row 44/45] expected c2Norm({}) to be NaN, got {}",
                fmt_v(z),
                fmt_v(cv)
            );
            // Row 45 specifically: the -0.0 vector.
            d.vec(&format!("rows 44-45: c2Norm({})", fmt_v(z)), cv, (p.rs.c2Norm)(z));
            d.scalar("rows 44-45: c2Len(zero)", (p.c.c2Len)(z), (p.rs.c2Len)(z));
        }
        // denormals whose squares underflow to zero -> len == 0 -> 1/0 -> inf
        for &t in &[1.0e-45f32, -1.0e-45, f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 1.0e-30, 1.0e-23] {
            let a = c2v { x: t, y: t };
            d.vec("row 44: c2Norm(denormal)", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
            d.scalar("row 44: c2Len(denormal)", (p.c.c2Len)(a), (p.rs.c2Len)(a));
        }

        // Row 46: inf - inf inside c2Dot -> NaN -> sqrtf(NaN).
        // Row 47: any infinite component -> +inf.
        let inf_cases = [
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: f32::INFINITY, y: f32::INFINITY },
            c2v { x: f32::NEG_INFINITY, y: 0.0 },
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NAN },
            c2v { x: f32::MAX, y: f32::MAX },
        ];
        for a in inf_cases {
            let cv = (p.c.c2Len)(a);
            d.scalar(&format!("rows 46-47: c2Len({})", fmt_v(a)), cv, (p.rs.c2Len)(a));
            d.vec("rows 46-47: c2Norm(inf)", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
        }
        // c2Dot(inf, 0) is the invalid-operation NaN; pin its bit pattern
        let a = c2v { x: f32::INFINITY, y: 0.0 };
        let b = c2v { x: 0.0, y: 0.0 };
        let cv = (p.c.c2Dot)(a, b);
        assert!(cv.is_nan(), "[row 46] expected NaN from inf*0");
        d.scalar("row 46: inf * 0", cv, (p.rs.c2Dot)(a, b));

        // Row 48: `x < 0 ? -x : x` must NOT normalise -0.0, unlike fabsf.
        let mz = c2v { x: -0.0, y: -0.0 };
        let cv = (p.c.c2Absv)(mz);
        assert_eq!(
            (cv.x.to_bits(), cv.y.to_bits()),
            (0x8000_0000u32, 0x8000_0000u32),
            "[row 48] C's c2Absv is expected to preserve -0.0"
        );
        d.vec("row 48: c2Absv(-0.0)", cv, (p.rs.c2Absv)(mz));
        for &q in WEIRD {
            let a = c2v { x: q, y: -q };
            d.vec("row 48: c2Absv(weird)", (p.c.c2Absv)(a), (p.rs.c2Absv)(a));
        }

        // Row 49: ternary min/max returns the SECOND operand when the compare
        // is false, so NaN propagation differs from fminf/fmaxf.
        let n = f32::NAN;
        let cases = [
            (c2v { x: n, y: 1.0 }, c2v { x: 1.0, y: n }),
            (c2v { x: 1.0, y: n }, c2v { x: n, y: 1.0 }),
            (c2v { x: n, y: n }, c2v { x: 1.0, y: 2.0 }),
            (c2v { x: 1.0, y: 2.0 }, c2v { x: n, y: n }),
            (c2v { x: -n, y: n }, c2v { x: n, y: -n }),
            (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        ];
        for (a, b) in cases {
            let cmin = (p.c.c2Minv)(a, b);
            let cmax = (p.c.c2Maxv)(a, b);
            d.vec("row 49: c2Minv", cmin, (p.rs.c2Minv)(a, b));
            d.vec("row 49: c2Maxv", cmax, (p.rs.c2Maxv)(a, b));
        }
        // exhaustive over the weird pool
        for &ax in WEIRD {
            for &bx in WEIRD {
                let a = c2v { x: ax, y: bx };
                let b = c2v { x: bx, y: ax };
                d.vec("row 49: c2Minv(sweep)", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                d.vec("row 49: c2Maxv(sweep)", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
            }
        }
    }
    d.finish("ERRORS rows 44-49: arithmetic domain edges");
}

// ===========================================================================
// Generic FFI boundary cases required by Phase C beyond the table
// ===========================================================================

/// Zero / oversized / one-step-past-range values for every integer-ish input
/// the API accepts, plus NULL for the only pointer the C actually checks.
#[test]
fn generic_boundaries() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB0_11D);
    unsafe {
        // c2Poly.count: 0, 1, 8 and the int extremes.
        //
        // Counts ABOVE 8 make the C read past the declared arrays, so they are
        // only meaningful when both implementations are handed the SAME backing
        // bytes — that is done below with a shared oversized buffer, and more
        // thoroughly in CONFIGS row 56.
        for cnt in [0i32, 1, 7, 8, i32::MIN, i32::MIN + 1, -1] {
            for _ in 0..200 {
                let mut poly = make_convex_poly(&mut rng, 8);
                poly.count = cnt;
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 20.0) };
                d.ray(
                    &format!("boundary: count = {cnt}"),
                    call_poly(&p.c, A, &poly, None),
                    call_poly(&p.rs, A, &poly, None),
                );
            }
        }

        // count = 9 / INT_MAX / INT_MAX-1 in a SHARED buffer. The first edge is
        // rigged so that `den == 0 && num < 0`, which makes the C bail out on
        // iteration 0 — otherwise INT_MAX would walk off the end of memory.
        {
            let mut backing = vec![0f32; 1024];
            let base = backing.as_mut_ptr() as *mut u8;
            let pp = base as *mut c2Poly;
            for i in 0..8 {
                (*pp).verts[i] = c2v { x: 0.0, y: -10.0 };
                (*pp).norms[i] = c2v { x: 0.0, y: 1.0 };
            }
            let A = c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            };
            for cnt in [9i32, 100, i32::MAX - 1, i32::MAX] {
                (*pp).count = cnt;
                let cb = base as *const c2Poly;
                let cr = call_poly_raw(&p.c, A, cb, None);
                expect_c("boundary: oversized count bails on edge 0", cr.0, 0);
                untouched("boundary: oversized count", "C", cr.1);
                d.ray(
                    &format!("boundary: count = {cnt} (shared buffer)"),
                    cr,
                    call_poly_raw(&p.rs, A, cb, None),
                );
            }
        }

        // NULL bx on every dispatcher arm (the C's only null check).
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY] {
            for _ in 0..400 {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 20.0) };
                let poly = make_convex_poly(&mut rng, 4);
                let sp = &poly as *const c2Poly as *const c_void;
                d.ray(
                    &format!("boundary: NULL bx, type {ty}"),
                    call_cast(&p.c, A, sp, None, ty),
                    call_cast(&p.rs, A, sp, None, ty),
                );
            }
        }

        // Zero and oversized ray lengths on every entry point.
        for t in [0.0f32, -0.0, f32::MIN_POSITIVE, 1.0e-45, f32::MAX, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            for _ in 0..300 {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t };
                let circle = c2Circle { p: rng.v_small(), r: rng.range(0.0, 5.0) };
                d.ray("boundary: t sweep circle", call_circle(&p.c, A, circle), call_circle(&p.rs, A, circle));
                let boxx = c2AABB { min: rng.v_small(), max: rng.v_small() };
                d.ray("boundary: t sweep aabb", call_aabb(&p.c, A, boxx), call_aabb(&p.rs, A, boxx));
                let q = rng.v_small();
                let cap = c2Capsule { a: q, b: rng.v_small(), r: rng.range(0.0, 5.0) };
                d.ray("boundary: t sweep capsule", call_capsule(&p.c, A, cap), call_capsule(&p.rs, A, cap));
                let poly = { let n = 1 + rng.below(8); make_convex_poly(&mut rng, n) };
                d.ray("boundary: t sweep poly", call_poly(&p.c, A, &poly, None), call_poly(&p.rs, A, &poly, None));
            }
        }
    }
    d.finish("Phase C: generic FFI boundaries");
}
