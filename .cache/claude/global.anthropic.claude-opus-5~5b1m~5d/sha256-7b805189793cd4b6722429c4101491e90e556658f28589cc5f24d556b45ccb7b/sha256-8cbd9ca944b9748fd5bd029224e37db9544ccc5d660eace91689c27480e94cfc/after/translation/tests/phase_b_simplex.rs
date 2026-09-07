//! Phase B, Group 3 — `CONFIGS.md` rows 23-44.
//!
//! The simplex machinery driven **directly** through the low-level exports
//! (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`), not via the
//! `c2GJK` convenience pipeline. The whole 152-byte `c2Simplex` is compared
//! after every call, so a wrong slot copy or a stale field shows up even when
//! the "interesting" outputs happen to agree.

mod common;
use common::*;

const N: usize = 3000;

// ---------------------------------------------------------------------------
// Rows 23-25 — c2GJKSimplexMetric at each count
// ---------------------------------------------------------------------------

fn metric_at(count: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let wild = i % 3 == 0;
        let mut s = rand_simplex(&mut rng, count, wild);
        // Force coincident / collinear special cases sometimes.
        match i % 7 {
            1 => s.verts[1].p = s.verts[0].p, // coincident a,b
            2 => s.verts[2].p = s.verts[0].p, // coincident a,c
            3 => {
                // collinear: c = a + 2*(b-a)
                let (a, b) = (s.verts[0].p, s.verts[1].p);
                s.verts[2].p = c2v::new(a.x + 2.0 * (b.x - a.x), a.y + 2.0 * (b.y - a.y));
            }
            4 => {
                s.verts[0].p = ZV;
                s.verts[1].p = ZV;
                s.verts[2].p = ZV;
            }
            _ => {}
        }
        diff_metric(&format!("count={count} i={i} s={s:?}"), &s);
    }
}

#[test]
fn row23_metric_count1() {
    metric_at(1, 0x21);
}

#[test]
fn row24_metric_count2() {
    metric_at(2, 0x22);
}

#[test]
fn row25_metric_count3() {
    metric_at(3, 0x23);
}

// ---------------------------------------------------------------------------
// Rows 26-29 — c22
// ---------------------------------------------------------------------------

/// Row 26: general position — `u > 0 && v > 0` ⇒ the `else` (count 2) branch.
#[test]
fn row26_c22_general() {
    let mut rng = Rng::new(0x26);
    let mut hit = 0usize;
    for i in 0..N * 2 {
        let mut s = rand_simplex(&mut rng, 2, false);
        // Straddle the origin so the projection lands strictly inside AB.
        let dir = rng.vec();
        let n = (dir.x * dir.x + dir.y * dir.y).sqrt();
        if n < 1e-3 {
            continue;
        }
        let (ux, uy) = (dir.x / n, dir.y / n);
        let ta = -rng.uniform(0.2, 5.0);
        let tb = rng.uniform(0.2, 5.0);
        // Offset perpendicular so the origin is off the segment line.
        let off = rng.uniform(-3.0, 3.0);
        let (px, py) = (-uy * off, ux * off);
        s.verts[0].p = c2v::new(px + ux * ta, py + uy * ta);
        s.verts[1].p = c2v::new(px + ux * tb, py + uy * tb);
        hit += 1;
        diff_c22(&format!("general i={i} s={s:?}"), &s);
    }
    assert!(hit > N, "not enough general-position samples: {hit}");
}

/// Row 27: origin beyond `a` ⇒ `v <= 0` ⇒ count 1, `a` kept.
#[test]
fn row27_c22_vertex_a() {
    let mut rng = Rng::new(0x27);
    for i in 0..N {
        let mut s = rand_simplex(&mut rng, 2, false);
        // Place a and b both "past" the origin along +dir so that
        // v = dot(a, a-b) <= 0, i.e. the origin is on a's outer side.
        let dir = rng.vec();
        let n = (dir.x * dir.x + dir.y * dir.y).sqrt();
        if n < 1e-3 {
            continue;
        }
        let (ux, uy) = (dir.x / n, dir.y / n);
        let ta = rng.uniform(0.5, 4.0);
        let tb = ta + rng.uniform(0.1, 4.0);
        s.verts[0].p = c2v::new(ux * ta, uy * ta);
        s.verts[1].p = c2v::new(ux * tb, uy * tb);
        diff_c22(&format!("vertexA i={i} s={s:?}"), &s);
    }
}

/// Row 28: origin beyond `b` ⇒ `u <= 0` ⇒ count 1 with `s->a = s->b`.
#[test]
fn row28_c22_vertex_b() {
    let mut rng = Rng::new(0x28);
    for i in 0..N {
        let mut s = rand_simplex(&mut rng, 2, false);
        let dir = rng.vec();
        let n = (dir.x * dir.x + dir.y * dir.y).sqrt();
        if n < 1e-3 {
            continue;
        }
        let (ux, uy) = (dir.x / n, dir.y / n);
        let tb = rng.uniform(0.5, 4.0);
        let ta = tb + rng.uniform(0.1, 4.0);
        s.verts[0].p = c2v::new(ux * ta, uy * ta);
        s.verts[1].p = c2v::new(ux * tb, uy * tb);
        diff_c22(&format!("vertexB i={i} s={s:?}"), &s);
    }
}

/// Row 29: degenerate and non-finite inputs.
#[test]
fn row29_c22_degenerate() {
    let mut rng = Rng::new(0x29);
    // Hand-picked degeneracies.
    let mut fixed: Vec<c2Simplex> = Vec::new();
    for &(ap, bp) in &[
        (ZV, ZV),
        (c2v::new(1.0, 1.0), c2v::new(1.0, 1.0)),
        (c2v::new(-0.0, 0.0), c2v::new(0.0, -0.0)),
        (c2v::new(f32::NAN, 1.0), c2v::new(2.0, 3.0)),
        (c2v::new(1.0, 2.0), c2v::new(f32::NAN, f32::NAN)),
        (
            c2v::new(f32::INFINITY, 0.0),
            c2v::new(f32::NEG_INFINITY, 0.0),
        ),
        (c2v::new(FLT_MAX, FLT_MAX), c2v::new(-FLT_MAX, -FLT_MAX)),
        (
            c2v::new(f32::MIN_POSITIVE, 0.0),
            c2v::new(f32::from_bits(1), 0.0),
        ),
    ] {
        let mut s = simplex_with(2, 1.0, [ap, bp, ZV], [0.0; 3]);
        s.verts[0].sA = c2v::new(11.0, 12.0);
        s.verts[0].sB = c2v::new(13.0, 14.0);
        s.verts[1].sA = c2v::new(21.0, 22.0);
        s.verts[1].sB = c2v::new(23.0, 24.0);
        s.verts[0].iA = 5;
        s.verts[0].iB = 6;
        s.verts[1].iA = 7;
        s.verts[1].iB = 8;
        fixed.push(s);
    }
    for s in &fixed {
        diff_c22(&format!("fixed {s:?}"), s);
    }
    // Fully random, including wild floats and every count value.
    for count in [-2i32, -1, 0, 1, 2, 3, 4, 7, i32::MIN, i32::MAX] {
        for i in 0..400 {
            let s = rand_simplex(&mut rng, count, i % 2 == 0);
            diff_c22(&format!("count={count} i={i} s={s:?}"), &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 30-37 — c23
// ---------------------------------------------------------------------------

/// Build a `c2Simplex` with count 3 and fully populated slots.
fn tri(rng: &mut Rng, a: c2v, b: c2v, c: c2v) -> c2Simplex {
    let mut s = rand_simplex(rng, 3, false);
    s.verts[0].p = a;
    s.verts[1].p = b;
    s.verts[2].p = c;
    s
}

/// Row 30: general position — origin strictly inside the triangle ⇒ the final
/// `else` (count 3, barycentric) branch.
#[test]
fn row30_c23_interior() {
    let mut rng = Rng::new(0x30);
    let mut n_interior = 0usize;
    for i in 0..N * 2 {
        // Random triangle guaranteed to contain the origin: pick three
        // directions spanning the circle with random radii.
        let base = rng.uniform(0.0, 6.28);
        let mut ang = [0.0f32; 3];
        for k in 0..3 {
            ang[k] = base + k as f32 * 2.094_395 + rng.uniform(-0.5, 0.5);
        }
        let mut ps = [ZV; 3];
        for k in 0..3 {
            let r = rng.uniform(0.5, 5.0);
            ps[k] = c2v::new(r * ang[k].cos(), r * ang[k].sin());
        }
        let s = tri(&mut rng, ps[0], ps[1], ps[2]);
        n_interior += 1;
        diff_c23(&format!("interior i={i} s={s:?}"), &s);
    }
    assert!(n_interior > N);
}

/// Rows 31-36: each vertex and edge region of `c23`, reached by placing the
/// origin in the corresponding Voronoi region of a triangle.
#[test]
fn row31_36_c23_all_regions() {
    let mut rng = Rng::new(0x31);
    // A triangle with vertices at known positions; the origin's location
    // relative to it is varied by translating the triangle.
    for i in 0..N * 3 {
        // Random triangle, then translate it so the origin sits in each
        // region in turn: far outside each vertex, outside each edge, inside.
        let a = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
        let b = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
        let c = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
        // 8 shifts: towards/away from each vertex and each edge midpoint,
        // plus the centroid (interior) and no shift at all.
        let cen = c2v::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0);
        let mid = |p: c2v, q: c2v| c2v::new((p.x + q.x) * 0.5, (p.y + q.y) * 0.5);
        let targets = [
            a,
            b,
            c,
            mid(a, b),
            mid(b, c),
            mid(c, a),
            cen,
            c2v::new(cen.x * 3.0, cen.y * 3.0),
        ];
        for (ti, t) in targets.iter().enumerate() {
            // Push the origin outside by scaling how far past `t` we shift.
            for &k in &[0.0f32, 0.9, 1.0, 1.1, 3.0, -1.0] {
                let d = c2v::new(-t.x * k, -t.y * k);
                let sh = |p: c2v| c2v::new(p.x + d.x, p.y + d.y);
                let s = tri(&mut rng, sh(a), sh(b), sh(c));
                diff_c23(&format!("regions i={i} t={ti} k={k} s={s:?}"), &s);
            }
        }
    }
}

/// Row 37: collinear / duplicate vertices, all-zero, and non-finite.
#[test]
fn row37_c23_degenerate() {
    let mut rng = Rng::new(0x37);
    let fixed: Vec<[c2v; 3]> = vec![
        [ZV, ZV, ZV],
        [c2v::new(1.0, 1.0), c2v::new(1.0, 1.0), c2v::new(1.0, 1.0)],
        // exactly collinear -> area == 0
        [c2v::new(0.0, 0.0), c2v::new(1.0, 1.0), c2v::new(2.0, 2.0)],
        [c2v::new(-1.0, 0.0), c2v::new(0.0, 0.0), c2v::new(1.0, 0.0)],
        // duplicate pairs
        [c2v::new(1.0, 0.0), c2v::new(1.0, 0.0), c2v::new(0.0, 1.0)],
        [c2v::new(1.0, 0.0), c2v::new(0.0, 1.0), c2v::new(1.0, 0.0)],
        // non-finite
        [c2v::new(f32::NAN, 0.0), c2v::new(1.0, 1.0), c2v::new(2.0, 0.0)],
        [
            c2v::new(1.0, 1.0),
            c2v::new(f32::INFINITY, f32::NEG_INFINITY),
            c2v::new(0.0, 2.0),
        ],
        [
            c2v::new(f32::from_bits(0xFFC0_1234), 1.0),
            c2v::new(f32::from_bits(0x7F80_0005), 2.0),
            c2v::new(3.0, f32::NAN),
        ],
        // overflow: products of these overflow inside c2Det2
        [
            c2v::new(FLT_MAX, FLT_MAX),
            c2v::new(-FLT_MAX, FLT_MAX),
            c2v::new(0.0, -FLT_MAX),
        ],
        // subnormals
        [
            c2v::new(f32::from_bits(1), 0.0),
            c2v::new(0.0, f32::from_bits(2)),
            c2v::new(f32::from_bits(3), f32::from_bits(4)),
        ],
        // signed zeros
        [
            c2v::new(-0.0, 0.0),
            c2v::new(0.0, -0.0),
            c2v::new(-0.0, -0.0),
        ],
    ];
    for ps in &fixed {
        let s = tri(&mut rng, ps[0], ps[1], ps[2]);
        diff_c23(&format!("fixed {ps:?} s={s:?}"), &s);
    }
    // Random collinear triples (area exactly or nearly 0).
    for i in 0..N {
        let a = rng.vec();
        let b = rng.vec();
        let t = rng.uniform(-3.0, 3.0);
        let c = c2v::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        let s = tri(&mut rng, a, b, c);
        diff_c23(&format!("collinear i={i} s={s:?}"), &s);
    }
    // Random wild, all counts (incl. out-of-range, which `c23` ignores since
    // it never reads `count`, only writes it).
    for count in [-1i32, 0, 1, 2, 3, 4, i32::MIN, i32::MAX] {
        for i in 0..300 {
            let s = rand_simplex(&mut rng, count, i % 2 == 0);
            diff_c23(&format!("wild count={count} i={i} s={s:?}"), &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 38-40 — c2D
// ---------------------------------------------------------------------------

#[test]
fn row38_c2d_count1() {
    let mut rng = Rng::new(0x38);
    for i in 0..N {
        let s = rand_simplex(&mut rng, 1, i % 3 == 0);
        diff_c2D(&format!("count=1 i={i} s={s:?}"), &s);
    }
    // Exhaustive sign/NaN patterns for a.p.
    let pats: [u32; 10] = [
        0x0000_0000,
        0x8000_0000,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001,
        0x0000_0001,
        0x3F80_0000,
        0xBF80_0000,
    ];
    for &x in &pats {
        for &y in &pats {
            let s = simplex_with(
                1,
                1.0,
                [c2v::new(f32::from_bits(x), f32::from_bits(y)), ZV, ZV],
                [1.0, 0.0, 0.0],
            );
            diff_c2D(&format!("pat {x:08x},{y:08x}"), &s);
        }
    }
}

#[test]
fn row39_40_c2d_count2_both_branches() {
    let mut rng = Rng::new(0x39);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for i in 0..N * 2 {
        let s = rand_simplex(&mut rng, 2, false);
        let (a, b) = (s.verts[0].p, s.verts[1].p);
        // det2(b-a, -a) decides the branch; count how many of each we get.
        let ab = c2v::new(b.x - a.x, b.y - a.y);
        let det = ab.x * (-a.y) - ab.y * (-a.x);
        if det > 0.0 {
            skew += 1;
        } else {
            ccw += 1;
        }
        diff_c2D(&format!("count=2 i={i} s={s:?}"), &s);
    }
    assert!(skew > 100 && ccw > 100, "branch coverage: skew={skew} ccw={ccw}");

    // det == 0 exactly (origin on the AB line) ⇒ the `<= 0` / CCW90 branch.
    for i in 0..500 {
        let d = rng.vec();
        let ta = rng.uniform(-4.0, 4.0);
        let tb = rng.uniform(-4.0, 4.0);
        let s = simplex_with(
            2,
            1.0,
            [
                c2v::new(d.x * ta, d.y * ta),
                c2v::new(d.x * tb, d.y * tb),
                ZV,
            ],
            [1.0, 1.0, 0.0],
        );
        diff_c2D(&format!("collinear-origin i={i} s={s:?}"), &s);
    }
    // NaN det ⇒ comparison false ⇒ CCW90 branch.
    for i in 0..500 {
        let s = rand_simplex(&mut rng, 2, true);
        diff_c2D(&format!("wild count=2 i={i} s={s:?}"), &s);
    }
    // All other counts (default arm).
    for count in [-1i32, 0, 3, 4, 5, i32::MIN, i32::MAX] {
        for i in 0..300 {
            let s = rand_simplex(&mut rng, count, i % 2 == 0);
            diff_c2D(&format!("count={count} i={i} s={s:?}"), &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 41 — c2L
// ---------------------------------------------------------------------------

#[test]
fn row41_c2l() {
    let mut rng = Rng::new(0x41);
    for count in [1i32, 2] {
        for i in 0..N * 2 {
            let mut s = rand_simplex(&mut rng, count, i % 4 == 0);
            // Interesting `div` values, including the ones that make `den`
            // infinite or NaN.
            s.div = match i % 10 {
                0 => 0.0,
                1 => -0.0,
                2 => f32::NAN,
                3 => f32::INFINITY,
                4 => f32::MIN_POSITIVE,
                5 => f32::from_bits(1),
                6 => FLT_MAX,
                7 => 1.0,
                _ => s.div,
            };
            diff_c2L(&format!("count={count} i={i} s={s:?}"), &s);
        }
    }
    // Default arm.
    for count in [-1i32, 0, 3, 4, i32::MIN, i32::MAX] {
        for i in 0..500 {
            let mut s = rand_simplex(&mut rng, count, i % 2 == 0);
            if i % 5 == 0 {
                s.div = 0.0;
            }
            diff_c2L(&format!("count={count} i={i} s={s:?}"), &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 42-44 — c2Witness
// ---------------------------------------------------------------------------

fn witness_at(count: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for i in 0..N * 2 {
        let mut s = rand_simplex(&mut rng, count, i % 4 == 0);
        s.div = match i % 12 {
            0 => 0.0,
            1 => -0.0,
            2 => f32::NAN,
            3 => f32::INFINITY,
            4 => f32::NEG_INFINITY,
            5 => f32::MIN_POSITIVE,
            6 => f32::from_bits(1),
            7 => FLT_MAX,
            8 => 1.0,
            9 => 3.0,
            _ => s.div,
        };
        // Occasionally make the u's sum to div exactly (the realistic case).
        if i % 6 == 0 && count >= 1 && count <= 3 {
            let mut sum = 0.0f32;
            for k in 0..count as usize {
                s.verts[k].u = rng.uniform(0.1, 3.0);
                sum += s.verts[k].u;
            }
            s.div = sum;
        }
        diff_witness(&format!("count={count} i={i} s={s:?}"), &s);
    }
}

#[test]
fn row42_witness_count1() {
    witness_at(1, 0x42);
}

#[test]
fn row43_witness_count2() {
    witness_at(2, 0x43);
}

#[test]
fn row44_witness_count3() {
    witness_at(3, 0x44);
}
