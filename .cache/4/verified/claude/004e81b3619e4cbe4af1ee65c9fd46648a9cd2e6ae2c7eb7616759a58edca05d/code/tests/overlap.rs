//! Phase B — CONFIGS.md rows 16..24 (overlap predicates).

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4096;

fn cmp_aabb_aabb(A: C2AABB, B: C2AABB) {
    let (c, r) = (c(), rs());
    let ca = unsafe { (c.c2AABBtoAABB)(A, B) };
    let ra = unsafe { (r.c2AABBtoAABB)(A, B) };
    assert_eq!(
        ca,
        ra,
        "c2AABBtoAABB(min{} max{}, min{} max{}): C={ca} RUST={ra}",
        vshow(A.min),
        vshow(A.max),
        vshow(B.min),
        vshow(B.max)
    );
}

fn cmp_aabb_point(A: C2AABB, B: C2v) {
    let (c, r) = (c(), rs());
    let ca = unsafe { (c.c2AABBtoPoint)(A, B) };
    let ra = unsafe { (r.c2AABBtoPoint)(A, B) };
    assert_eq!(
        ca,
        ra,
        "c2AABBtoPoint(min{} max{}, {}): C={ca} RUST={ra}",
        vshow(A.min),
        vshow(A.max),
        vshow(B)
    );
}

fn cmp_circle_point(A: C2Circle, B: C2v) {
    let (c, r) = (c(), rs());
    let ca = unsafe { (c.c2CircleToPoint)(A, B) };
    let ra = unsafe { (r.c2CircleToPoint)(A, B) };
    assert_eq!(
        ca,
        ra,
        "c2CircleToPoint({{p:{}, r:{}}}, {}): C={ca} RUST={ra}",
        vshow(A.p),
        fshow(A.r),
        vshow(B)
    );
}

fn box_of(cx: f32, cy: f32, hx: f32, hy: f32) -> C2AABB {
    C2AABB {
        min: v(cx - hx, cy - hy),
        max: v(cx + hx, cy + hy),
    }
}

// --- row 16: random overlapping boxes --------------------------------------
#[test]
fn row16_aabbtoaabb_overlapping() {
    let mut rng = Rng::new(0x1616);
    for _ in 0..N {
        // Build B so that it definitely overlaps A.
        let a = box_of(rng.unit(10.0), rng.unit(10.0), rng.unit(5.0).abs() + 0.5, rng.unit(5.0).abs() + 0.5);
        let px = a.min.x + (a.max.x - a.min.x) * ((rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32);
        let py = a.min.y + (a.max.y - a.min.y) * ((rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32);
        let b = box_of(px, py, rng.unit(3.0).abs() + 0.01, rng.unit(3.0).abs() + 0.01);
        cmp_aabb_aabb(a, b);
        cmp_aabb_aabb(b, a);
    }
}

// --- row 17: separated on each of the 4 axes -------------------------------
#[test]
fn row17_aabbtoaabb_separated_each_axis() {
    let mut rng = Rng::new(0x1717);
    for _ in 0..N {
        let a = box_of(rng.unit(10.0), rng.unit(10.0), 1.0 + rng.unit(2.0).abs(), 1.0 + rng.unit(2.0).abs());
        let gap = 0.001 + rng.unit(5.0).abs();
        let w = 1.0 + rng.unit(2.0).abs();
        let h = 1.0 + rng.unit(2.0).abs();
        // B entirely to the -x of A  (B.max.x < A.min.x)
        cmp_aabb_aabb(a, box_of(a.min.x - gap - w, a.min.y, w, h));
        // B entirely to the +x of A  (A.max.x < B.min.x)
        cmp_aabb_aabb(a, box_of(a.max.x + gap + w, a.min.y, w, h));
        // B entirely to the -y of A
        cmp_aabb_aabb(a, box_of(a.min.x, a.min.y - gap - h, w, h));
        // B entirely to the +y of A
        cmp_aabb_aabb(a, box_of(a.min.x, a.max.y + gap + h, w, h));
    }
}

// --- row 18: exact touching -----------------------------------------------
#[test]
fn row18_aabbtoaabb_touching() {
    let mut rng = Rng::new(0x1818);
    for _ in 0..N {
        let x0 = (rng.below(41) as f32 - 20.0) * 0.5;
        let y0 = (rng.below(41) as f32 - 20.0) * 0.5;
        let w = (rng.below(9) as f32 + 1.0) * 0.5;
        let h = (rng.below(9) as f32 + 1.0) * 0.5;
        let a = C2AABB {
            min: v(x0, y0),
            max: v(x0 + w, y0 + h),
        };
        // shares the +x face exactly
        cmp_aabb_aabb(
            a,
            C2AABB {
                min: v(a.max.x, y0),
                max: v(a.max.x + w, y0 + h),
            },
        );
        // shares the -x face exactly
        cmp_aabb_aabb(
            a,
            C2AABB {
                min: v(a.min.x - w, y0),
                max: v(a.min.x, y0 + h),
            },
        );
        // shares the +y face exactly
        cmp_aabb_aabb(
            a,
            C2AABB {
                min: v(x0, a.max.y),
                max: v(x0 + w, a.max.y + h),
            },
        );
        // shares the -y face exactly
        cmp_aabb_aabb(
            a,
            C2AABB {
                min: v(x0, a.min.y - h),
                max: v(x0 + w, a.min.y),
            },
        );
        // corner-only touch
        cmp_aabb_aabb(
            a,
            C2AABB {
                min: v(a.max.x, a.max.y),
                max: v(a.max.x + w, a.max.y + h),
            },
        );
    }
}

// --- row 19: degenerate & inverted boxes ----------------------------------
#[test]
fn row19_aabbtoaabb_degenerate_inverted() {
    let mut rng = Rng::new(0x1919);
    for _ in 0..N {
        let p = rng.geom_v();
        let q = rng.geom_v();
        let degen_a = C2AABB { min: p, max: p };
        let degen_b = C2AABB { min: q, max: q };
        cmp_aabb_aabb(degen_a, degen_b);
        cmp_aabb_aabb(degen_a, box_of(q.x, q.y, 1.0, 1.0));
        cmp_aabb_aabb(box_of(p.x, p.y, 1.0, 1.0), degen_b);
        // inverted (min > max)
        let inv = C2AABB {
            min: v(p.x.max(q.x) + 1.0, p.y.max(q.y) + 1.0),
            max: v(p.x.min(q.x) - 1.0, p.y.min(q.y) - 1.0),
        };
        cmp_aabb_aabb(inv, degen_b);
        cmp_aabb_aabb(degen_a, inv);
        cmp_aabb_aabb(inv, inv);
        // line boxes (zero width / zero height)
        cmp_aabb_aabb(
            C2AABB {
                min: p,
                max: v(p.x, p.y + 3.0),
            },
            C2AABB {
                min: v(q.x - 3.0, q.y),
                max: v(q.x + 3.0, q.y),
            },
        );
    }
}

// --- row 20: NaN / inf ----------------------------------------------------
#[test]
fn row20_aabbtoaabb_nan_inf() {
    // exhaustive over the specials in a single coordinate at a time
    let base = box_of(0.0, 0.0, 1.0, 1.0);
    for &s in SPECIALS.iter() {
        for slot in 0..4 {
            let mut b = box_of(0.5, 0.5, 1.0, 1.0);
            match slot {
                0 => b.min.x = s,
                1 => b.min.y = s,
                2 => b.max.x = s,
                _ => b.max.y = s,
            }
            cmp_aabb_aabb(base, b);
            cmp_aabb_aabb(b, base);
            cmp_aabb_aabb(b, b);
        }
    }
    let mut rng = Rng::new(0x2020);
    for _ in 0..N {
        let a = C2AABB {
            min: rng.wild_v(),
            max: rng.wild_v(),
        };
        let b = C2AABB {
            min: rng.wild_v(),
            max: rng.wild_v(),
        };
        cmp_aabb_aabb(a, b);
    }
}

// --- row 21: point in / on / outside each side ----------------------------
#[test]
fn row21_aabbtopoint_sides() {
    let mut rng = Rng::new(0x2121);
    for _ in 0..N {
        let x0 = (rng.below(41) as f32 - 20.0) * 0.5;
        let y0 = (rng.below(41) as f32 - 20.0) * 0.5;
        let w = (rng.below(9) as f32 + 1.0) * 0.5;
        let h = (rng.below(9) as f32 + 1.0) * 0.5;
        let a = C2AABB {
            min: v(x0, y0),
            max: v(x0 + w, y0 + h),
        };
        // inside
        cmp_aabb_point(a, v(x0 + w * 0.5, y0 + h * 0.5));
        // exactly on each corner / edge
        cmp_aabb_point(a, a.min);
        cmp_aabb_point(a, a.max);
        cmp_aabb_point(a, v(a.min.x, a.max.y));
        cmp_aabb_point(a, v(a.max.x, a.min.y));
        cmp_aabb_point(a, v(a.min.x, y0 + h * 0.5));
        cmp_aabb_point(a, v(a.max.x, y0 + h * 0.5));
        cmp_aabb_point(a, v(x0 + w * 0.5, a.min.y));
        cmp_aabb_point(a, v(x0 + w * 0.5, a.max.y));
        // just outside each of the 4 sides
        let e = 0.0009765625f32; // exact power of two
        cmp_aabb_point(a, v(a.min.x - e, y0 + h * 0.5));
        cmp_aabb_point(a, v(a.max.x + e, y0 + h * 0.5));
        cmp_aabb_point(a, v(x0 + w * 0.5, a.min.y - e));
        cmp_aabb_point(a, v(x0 + w * 0.5, a.max.y + e));
        // random
        cmp_aabb_point(a, rng.geom_v());
    }
}

// --- row 22: inverted / degenerate box, NaN point ------------------------
#[test]
fn row22_aabbtopoint_degenerate_nan() {
    let inv = C2AABB {
        min: v(1.0, 1.0),
        max: v(-1.0, -1.0),
    };
    let degen = C2AABB {
        min: v(2.0, 3.0),
        max: v(2.0, 3.0),
    };
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            cmp_aabb_point(inv, v(x, y));
            cmp_aabb_point(degen, v(x, y));
            cmp_aabb_point(
                C2AABB {
                    min: v(x, y),
                    max: v(y, x),
                },
                v(0.0, 0.0),
            );
        }
    }
    cmp_aabb_point(degen, degen.min);
    let mut rng = Rng::new(0x2222);
    for _ in 0..N {
        cmp_aabb_point(
            C2AABB {
                min: rng.wild_v(),
                max: rng.wild_v(),
            },
            rng.wild_v(),
        );
    }
}

// --- row 23: NaN sweep on c2AABBtoPoint ---------------------------------
#[test]
fn row23_aabbtopoint_nan_each_slot() {
    let base = C2AABB {
        min: v(-1.0, -1.0),
        max: v(1.0, 1.0),
    };
    for &s in SPECIALS.iter() {
        for slot in 0..6 {
            let mut a = base;
            let mut p = v(0.0, 0.0);
            match slot {
                0 => a.min.x = s,
                1 => a.min.y = s,
                2 => a.max.x = s,
                3 => a.max.y = s,
                4 => p.x = s,
                _ => p.y = s,
            }
            cmp_aabb_point(a, p);
        }
    }
}

// --- row 24: point inside / on rim / outside ----------------------------
#[test]
fn row24_circletopoint_boundary() {
    let mut rng = Rng::new(0x2424);
    for _ in 0..N {
        let cx = (rng.below(21) as f32) - 10.0;
        let cy = (rng.below(21) as f32) - 10.0;
        let rad = (rng.below(8) as f32) + 1.0;
        let circ = C2Circle {
            p: v(cx, cy),
            r: rad,
        };
        // exactly on the rim on the 4 axes (r*r == d2 -> exclusive -> 0)
        cmp_circle_point(circ, v(cx + rad, cy));
        cmp_circle_point(circ, v(cx - rad, cy));
        cmp_circle_point(circ, v(cx, cy + rad));
        cmp_circle_point(circ, v(cx, cy - rad));
        // centre & strictly inside
        cmp_circle_point(circ, circ.p);
        cmp_circle_point(circ, v(cx + rad * 0.5, cy));
        // just outside
        cmp_circle_point(circ, v(cx + rad + 0.03125, cy));
        // 3-4-5 exact rim
        cmp_circle_point(C2Circle { p: v(0.0, 0.0), r: 5.0 }, v(3.0, 4.0));
        cmp_circle_point(C2Circle { p: v(0.0, 0.0), r: 5.0 }, v(3.0, 3.9375));
        // random
        cmp_circle_point(circ, rng.geom_v());
    }
}

// --- row 25 (table row 24 continued): degenerate radii -----------------
#[test]
fn row24b_circletopoint_degenerate_radii() {
    for &rad in SPECIALS.iter() {
        for &x in SPECIALS.iter() {
            for &y in SPECIALS.iter() {
                cmp_circle_point(C2Circle { p: v(x, y), r: rad }, v(y, x));
                cmp_circle_point(C2Circle { p: v(0.0, 0.0), r: rad }, v(x, y));
            }
        }
    }
    for &rb in SPECIAL_BITS.iter() {
        cmp_circle_point(
            C2Circle {
                p: v(0.0, 0.0),
                r: f32::from_bits(rb),
            },
            v(0.0, 0.0),
        );
        cmp_circle_point(
            C2Circle {
                p: v(0.0, 0.0),
                r: f32::from_bits(rb),
            },
            v(1.0, 1.0),
        );
    }
    let mut rng = Rng::new(0x2425);
    for _ in 0..N {
        cmp_circle_point(
            C2Circle {
                p: rng.wild_v(),
                r: rng.wild(),
            },
            rng.wild_v(),
        );
    }
    for _ in 0..N {
        cmp_circle_point(
            C2Circle {
                p: rng.geom_v(),
                r: rng.geom(),
            },
            rng.geom_v(),
        );
    }
}
