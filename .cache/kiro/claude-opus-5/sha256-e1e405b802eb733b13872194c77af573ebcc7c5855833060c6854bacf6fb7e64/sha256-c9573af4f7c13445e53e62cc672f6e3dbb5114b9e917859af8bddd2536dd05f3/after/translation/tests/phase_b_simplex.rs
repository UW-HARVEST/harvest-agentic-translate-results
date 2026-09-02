//! Phase B, Tier 3: `CONFIGS.md` rows 19-43.
//!
//! The simplex solvers, driven directly on a caller-built `c2Simplex` — this is
//! the lowest-level composed layer and it is where the GJK's real branching
//! lives. Every call compares the whole 152-byte `c2Simplex` afterwards,
//! including the unused `d` slot, so a stray write is caught.

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 3000;

/// A simplex whose every byte starts as 0xA5, then overwritten with real data.
/// Guarantees C and Rust begin from bit-identical state (the C reads fields the
/// caller is responsible for; leaving them indeterminate would be untestable).
fn poisoned_simplex() -> c2Simplex {
    unsafe { std::mem::transmute::<[u8; 152], c2Simplex>([0xA5u8; 152]) }
}

fn sv(p: c2v, u: f32, iA: c_int, iB: c_int) -> c2sv {
    c2sv { sA: c2v { x: p.x * 0.25, y: p.y * -0.5 }, sB: c2v { x: p.y, y: p.x }, p, u, iA, iB }
}

/// Build a simplex with the given vertex positions; all other bytes poisoned.
fn simplex_of(ps: &[c2v], div: f32) -> c2Simplex {
    let mut s = poisoned_simplex();
    for i in 0..4 {
        let p = ps.get(i).copied().unwrap_or(c2v { x: 7.5, y: -3.25 });
        s.verts[i] = sv(p, (i as f32) * 0.5 + 0.25, i as c_int, (3 - i) as c_int);
    }
    s.div = div;
    s.count = ps.len() as c_int;
    s
}

// ---------------------------------------------------------------------------
// Rows 19-21 — c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn row19_row20_row21_simplex_metric() {
    let l = libs();
    let (c, r) =
        (l.c.sym::<FnSimplexF>("c2GJKSimplexMetric"), l.rs.sym::<FnSimplexF>("c2GJKSimplexMetric"));
    let mut g = Rng::new(0x13);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, s: c2Simplex, tag: &str| {
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_f32(x, y), || {
            format!("c2GJKSimplexMetric[{tag}] count={}: C={} Rust={}", s.count, show_f32(x), show_f32(y))
        });
        // Neither side may mutate the simplex.
        rep.check(same_simplex(&a, &b), || {
            format!("c2GJKSimplexMetric[{tag}] mutated state differently:\n  C:    {}\n  Rust: {}", show_simplex(&a), show_simplex(&b))
        });
    };

    // Row 19: count == 1 (and the `default:` fall-through values).
    for count in [1, 0, 4, -1, 7, i32::MIN, i32::MAX] {
        for _ in 0..40 {
            let mut s = g.simplex(count);
            s.count = count;
            probe(&mut rep, s, "row19/default");
        }
    }
    // Row 20: count == 2.
    for i in 0..N {
        let mut s = g.simplex(2);
        if i % 5 == 0 {
            s.verts[1].p = s.verts[0].p; // duplicate -> metric 0
        }
        if i % 11 == 0 {
            s.verts[1].p = c2v { x: 1.0e30, y: -1.0e30 }; // dot overflows -> inf
        }
        probe(&mut rep, s, "row20");
    }
    // Row 21: count == 3, both winding directions and degenerate triangles.
    for _ in 0..N {
        probe(&mut rep, g.simplex(3), "row21");
    }
    for (a, b, cc) in [
        // counter-clockwise (positive det) and clockwise (negative det)
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }, c2v { x: 1.0, y: 0.0 }),
        // collinear -> det == 0
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }),
        // all identical
        (c2v { x: 3.0, y: 3.0 }, c2v { x: 3.0, y: 3.0 }, c2v { x: 3.0, y: 3.0 }),
    ] {
        probe(&mut rep, simplex_of(&[a, b, cc], 1.0), "row21/explicit");
    }
    rep.finish("row19_row20_row21_simplex_metric");
}

// ---------------------------------------------------------------------------
// Rows 22-25 — c22
// ---------------------------------------------------------------------------

fn probe_solver(
    rep: &mut Report,
    c: &libloading::Symbol<FnSimplexVoid>,
    r: &libloading::Symbol<FnSimplexVoid>,
    s: c2Simplex,
    tag: &str,
) {
    let (mut a, mut b) = (s, s);
    unsafe {
        c(&raw mut a);
        r(&raw mut b);
    }
    rep.check(same_simplex(&a, &b), || {
        format!(
            "[{tag}] input count={} div={} a.p={} b.p={} c.p={}\n  C:    {}\n  Rust: {}",
            s.count,
            show_f32(s.div),
            show_v(s.verts[0].p),
            show_v(s.verts[1].p),
            show_v(s.verts[2].p),
            show_simplex(&a),
            show_simplex(&b)
        )
    });
}

#[test]
fn row22_to_row25_c22() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexVoid>("c22"), l.rs.sym::<FnSimplexVoid>("c22"));
    let mut g = Rng::new(0x16);
    let mut rep = Report::new();

    // Rows 22/23/24: hand-built inputs that pin each of the three branches.
    // u = dot(b, b-a), v = dot(a, a-b).
    //
    // Row 22 (v <= 0): origin projects beyond a. e.g. a=(1,0) b=(2,0):
    //   v = dot((1,0),(-1,0)) = -1 <= 0
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }], 1.0), "row22 v<=0");
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: 0.5, y: 0.5 }, c2v { x: 4.0, y: 4.0 }], 3.0), "row22 v<=0");
    // Row 23 (u <= 0, v > 0): origin projects beyond b. e.g. a=(-2,0) b=(-1,0):
    //   u = dot((-1,0),(1,0)) = -1 <= 0 ; v = dot((-2,0),(-1,0)) = 2 > 0
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }], 1.0), "row23 u<=0");
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: -4.0, y: 1.0 }, c2v { x: -1.0, y: 0.25 }], 9.0), "row23 u<=0");
    // Row 24 (u > 0 && v > 0): origin projects inside the segment.
    //   a=(-1,1) b=(1,1): u = dot((1,1),(2,0)) = 2, v = dot((-1,1),(-2,0)) = 2
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }], 1.0), "row24 interior");
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: -3.0, y: -2.0 }, c2v { x: 5.0, y: -2.0 }], 0.0), "row24 interior");
    // Row 25: degenerate a.p == b.p -> u == v == 0 -> takes the v<=0 branch.
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: 2.0, y: 3.0 }, c2v { x: 2.0, y: 3.0 }], 1.0), "row25 dup");
    probe_solver(&mut rep, &c, &r, simplex_of(&[c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }], 0.0), "row25 origin dup");

    // Row 25 (randomized): a wide sweep so all three branches fire by chance,
    // including NaN / inf coordinates where every `<= 0` test is false.
    for i in 0..N {
        let mut s = g.simplex(2);
        match i % 8 {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[0].p = c2v { x: 0.0, y: 0.0 },
            2 => s.verts[1].p = c2v { x: 0.0, y: 0.0 },
            3 => s.verts[0].p = g.nasty_v(),
            4 => s.verts[1].p = g.nasty_v(),
            _ => {}
        }
        probe_solver(&mut rep, &c, &r, s, "row25 random");
        // c22 must also be exercised with counts it was not written for: the C
        // reads only a and b regardless of `count`, and always overwrites it.
        let mut s2 = g.simplex(2);
        s2.count = [0, 1, 3, 4, -1][i % 5];
        probe_solver(&mut rep, &c, &r, s2, "row25 odd count");
    }
    rep.finish("row22_to_row25_c22");
}

// ---------------------------------------------------------------------------
// Rows 26-34 — c23 (all seven regions)
// ---------------------------------------------------------------------------

#[test]
fn row26_to_row34_c23() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexVoid>("c23"), l.rs.sym::<FnSimplexVoid>("c23"));
    let mut g = Rng::new(0x1a);
    let mut rep = Report::new();

    // A triangle far from the origin in a chosen direction puts the origin in
    // that triangle's corresponding Voronoi region. Sweep the origin's relative
    // position over a dense grid so every one of the seven branches is hit many
    // times with many different values.
    let tri = [c2v { x: -1.0, y: -0.6 }, c2v { x: 1.2, y: -0.5 }, c2v { x: 0.1, y: 1.4 }];
    let mut hits = [0usize; 8];
    for gx in -12i32..=12 {
        for gy in -12i32..=12 {
            let off = c2v { x: gx as f32 * 0.45, y: gy as f32 * 0.45 };
            let ps: Vec<c2v> =
                tri.iter().map(|p| c2v { x: p.x + off.x, y: p.y + off.y }).collect();
            let s = simplex_of(&ps, 1.0);
            // Classify which branch the C will take, to prove coverage.
            hits[classify_c23(&ps)] += 1;
            probe_solver(&mut rep, &c, &r, s, "row26-32 grid");
        }
    }
    // Rows 26-32: assert the grid really did cover all seven branches.
    for b in 0..7 {
        assert!(hits[b] > 0, "c23 branch {b} was never exercised by the grid sweep: {hits:?}");
    }
    eprintln!("c23 branch coverage (vertA,vertB,vertC,edgeAB,edgeBC,edgeCA,interior) = {:?}", &hits[..7]);

    // Row 33: collinear / zero-area triangles -> area == 0 -> div == 0.
    for (a, b, cc) in [
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }),
        (c2v { x: -3.0, y: 1.0 }, c2v { x: 0.0, y: 1.0 }, c2v { x: 5.0, y: 1.0 }),
        (c2v { x: 2.0, y: 2.0 }, c2v { x: 2.0, y: 2.0 }, c2v { x: 2.0, y: 2.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 1.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }), // origin on segment
    ] {
        probe_solver(&mut rep, &c, &r, simplex_of(&[a, b, cc], 1.0), "row33 degenerate");
    }

    // Row 34: fully randomized triangles, repeated vertices, nasty floats.
    for i in 0..N {
        let mut s = g.simplex(3);
        match i % 9 {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[2].p = s.verts[0].p,
            2 => s.verts[2].p = s.verts[1].p,
            3 => s.verts[0].p = c2v { x: 0.0, y: 0.0 },
            4 => s.verts[0].p = g.nasty_v(),
            5 => s.verts[2].p = g.nasty_v(),
            6 => {
                // Huge coordinates: area*det2 overflows to +/-inf.
                s.verts[0].p = c2v { x: 1.0e20, y: -1.0e20 };
                s.verts[1].p = c2v { x: -1.0e20, y: 1.0e20 };
            }
            _ => {}
        }
        probe_solver(&mut rep, &c, &r, s, "row34 random");
        // Odd counts: the C ignores `count` on entry and always rewrites it.
        let mut s2 = g.simplex(3);
        s2.count = [0, 1, 2, 4, -1][i % 5];
        probe_solver(&mut rep, &c, &r, s2, "row34 odd count");
    }
    rep.finish("row26_to_row34_c23");
}

/// Mirror of the C's branch ladder in `c23`, used only to prove branch coverage.
fn classify_c23(p: &[c2v]) -> usize {
    let dot = |a: c2v, b: c2v| a.x * b.x + a.y * b.y;
    let sub = |a: c2v, b: c2v| c2v { x: a.x - b.x, y: a.y - b.y };
    let det = |a: c2v, b: c2v| a.x * b.y - a.y * b.x;
    let (a, b, c) = (p[0], p[1], p[2]);
    let (uab, vab) = (dot(b, sub(b, a)), dot(a, sub(a, b)));
    let (ubc, vbc) = (dot(c, sub(c, b)), dot(b, sub(b, c)));
    let (uca, vca) = (dot(a, sub(a, c)), dot(c, sub(c, a)));
    let area = det(sub(b, a), sub(c, a));
    let (uabc, vabc, wabc) = (det(b, c) * area, det(c, a) * area, det(a, b) * area);
    if vab <= 0.0 && uca <= 0.0 {
        0
    } else if uab <= 0.0 && vbc <= 0.0 {
        1
    } else if ubc <= 0.0 && vca <= 0.0 {
        2
    } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
        3
    } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
        4
    } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
        5
    } else {
        6
    }
}

// ---------------------------------------------------------------------------
// Row 35 — c2D
// ---------------------------------------------------------------------------

#[test]
fn row35_c2D() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexV>("c2D"), l.rs.sym::<FnSimplexV>("c2D"));
    let mut g = Rng::new(0x23);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, s: c2Simplex, tag: &str| {
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_v(x, y), || {
            format!(
                "c2D[{tag}] count={} a.p={} b.p={}: C={} Rust={}",
                s.count, show_v(s.verts[0].p), show_v(s.verts[1].p), show_v(x), show_v(y)
            )
        });
        rep.check(same_simplex(&a, &b), || format!("c2D[{tag}] mutated state differently"));
    };

    // count == 1
    for _ in 0..400 {
        probe(&mut rep, g.simplex(1), "count1");
    }
    // count == 2, both det2 signs plus the exact det2 == 0 boundary.
    for _ in 0..N {
        probe(&mut rep, g.simplex(2), "count2 random");
    }
    for (a, b) in [
        // det2(ab, -a) > 0  -> c2Skew
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 1.0 }),
        // det2(ab, -a) < 0  -> c2CCW90
        (c2v { x: 1.0, y: -1.0 }, c2v { x: 2.0, y: -1.0 }),
        // det2 == 0 exactly (origin collinear with ab) -> c2CCW90
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }),
        (c2v { x: -1.0, y: -1.0 }, c2v { x: 1.0, y: 1.0 }),
        // a.p == b.p -> ab == 0
        (c2v { x: 4.0, y: 4.0 }, c2v { x: 4.0, y: 4.0 }),
        // a.p == origin
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 2.0 }),
        // NaN
        (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 1.0 }),
    ] {
        probe(&mut rep, simplex_of(&[a, b], 1.0), "count2 explicit");
    }
    // count == 3 / default
    for count in [3, 0, 4, -1, i32::MAX, i32::MIN] {
        for _ in 0..60 {
            let mut s = g.simplex(3);
            s.count = count;
            probe(&mut rep, s, "default");
        }
    }
    rep.finish("row35_c2D");
}

// ---------------------------------------------------------------------------
// Row 36 — c2L
// ---------------------------------------------------------------------------

#[test]
fn row36_c2L() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexV>("c2L"), l.rs.sym::<FnSimplexV>("c2L"));
    let mut g = Rng::new(0x24);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, s: c2Simplex, tag: &str| {
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_v(x, y), || {
            format!(
                "c2L[{tag}] count={} div={} u=({},{}): C={} Rust={}",
                s.count, show_f32(s.div), show_f32(s.verts[0].u), show_f32(s.verts[1].u),
                show_v(x), show_v(y)
            )
        });
        rep.check(same_simplex(&a, &b), || format!("c2L[{tag}] mutated state differently"));
    };

    for count in [1, 2, 0, 3, 4, -1, i32::MIN] {
        for _ in 0..600 {
            let mut s = g.simplex(count);
            s.count = count;
            probe(&mut rep, s, "random");
        }
    }
    // div == 0 (den = inf), div negative, div = NaN/inf, u = 0.
    for div in [0.0f32, -0.0, -1.0, 2.0, f32::INFINITY, f32::NAN, f32::from_bits(1)] {
        for count in [1, 2, 3] {
            let mut s = simplex_of(&[c2v { x: 1.0, y: 2.0 }, c2v { x: -3.0, y: 4.0 }], div);
            s.count = count;
            probe(&mut rep, s, "div edge");
            s.verts[0].u = 0.0;
            s.verts[1].u = 0.0;
            probe(&mut rep, s, "div edge u=0");
        }
    }
    rep.finish("row36_c2L");
}

// ---------------------------------------------------------------------------
// Rows 37-39 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row37_to_row39_c2Witness() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnWitness>("c2Witness"), l.rs.sym::<FnWitness>("c2Witness"));
    let mut g = Rng::new(0x25);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, s: c2Simplex, tag: &str| {
        // Poison the out-params so a missing write is detected.
        let poison = c2v { x: f32::from_bits(0x5A5A5A5A), y: f32::from_bits(0xA5A5A5A5) };
        let (mut sc, mut sr) = (s, s);
        let (mut ac, mut bc) = (poison, poison);
        let (mut ar, mut br) = (poison, poison);
        unsafe {
            c(&raw mut sc, &raw mut ac, &raw mut bc);
            r(&raw mut sr, &raw mut ar, &raw mut br);
        }
        rep.check(same_v(ac, ar) && same_v(bc, br), || {
            format!(
                "c2Witness[{tag}] count={} div={}:\n  C:    a={} b={}\n  Rust: a={} b={}",
                s.count, show_f32(s.div), show_v(ac), show_v(bc), show_v(ar), show_v(br)
            )
        });
        rep.check(same_simplex(&sc, &sr), || format!("c2Witness[{tag}] mutated state differently"));
    };

    // Rows 37/38/39 plus the `default:` label.
    for count in [1, 2, 3, 0, 4, -1, i32::MAX] {
        for _ in 0..800 {
            let mut s = g.simplex(count);
            s.count = count;
            probe(&mut rep, s, "random");
        }
    }
    // div == 0 / negative / NaN with each count; and u values that do not sum to div.
    for div in [0.0f32, -0.0, -2.5, 1.0, f32::INFINITY, f32::NAN] {
        for count in [1, 2, 3, 4] {
            let mut s = simplex_of(
                &[c2v { x: 1.0, y: 2.0 }, c2v { x: -3.0, y: 4.0 }, c2v { x: 0.5, y: -0.25 }],
                div,
            );
            s.count = count;
            probe(&mut rep, s, "div edge");
            for k in 0..4 {
                s.verts[k].u = 0.0;
            }
            probe(&mut rep, s, "div edge u=0");
            for k in 0..4 {
                s.verts[k].u = f32::NAN;
            }
            probe(&mut rep, s, "div edge u=NaN");
        }
    }
    rep.finish("row37_to_row39_c2Witness");
}

// ---------------------------------------------------------------------------
// Rows 40-43 — c2Support at every vertex count
// ---------------------------------------------------------------------------

#[test]
fn row40_to_row43_c2Support() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSupport>("c2Support"), l.rs.sym::<FnSupport>("c2Support"));
    let mut g = Rng::new(0x28);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, verts: &[c2v; 8], count: c_int, d: c2v, tag: &str| {
        let (x, y) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
        rep.check(x == y, || {
            format!(
                "c2Support[{tag}] count={count} d={} verts={:?}: C={x} Rust={y}",
                show_v(d),
                &verts[..(count.max(0) as usize).min(8)]
                    .iter()
                    .map(|v| (v.x, v.y))
                    .collect::<Vec<_>>()
            )
        });
    };

    // Rows 40/41/42/43: counts 1, 2, 4 and the full 8-slot capacity.
    for count in [1, 2, 4, 8] {
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = g.finite_v();
            }
            probe(&mut rep, &verts, count, g.finite_v(), "random");
            // Direction aligned with an axis -> exact ties between vertices.
            for d in [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: -1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 0.0, y: -1.0 },
                c2v { x: 0.0, y: 0.0 }, // all dots are 0 -> no `dot > dmax` -> index 0
            ] {
                probe(&mut rep, &verts, count, d, "axis/zero d");
            }
        }
        // All vertices identical -> first index wins.
        let verts = [c2v { x: 2.0, y: -1.0 }; 8];
        probe(&mut rep, &verts, count, c2v { x: 1.0, y: 1.0 }, "all equal");
        // A NaN vertex: `dot > dmax` is false, so it never becomes the max.
        let mut verts = [c2v { x: 1.0, y: 1.0 }; 8];
        verts[count.min(7) as usize / 2] = c2v { x: f32::NAN, y: 0.0 };
        probe(&mut rep, &verts, count, c2v { x: 1.0, y: 0.0 }, "NaN vertex");
        // A NaN direction: dmax starts NaN and nothing beats it -> index 0.
        let verts = [c2v { x: 1.0, y: 1.0 }; 8];
        probe(&mut rep, &verts, count, c2v { x: f32::NAN, y: 1.0 }, "NaN d");
        // Infinite coordinates.
        let mut verts = [c2v { x: 1.0, y: 1.0 }; 8];
        verts[0] = c2v { x: f32::INFINITY, y: 0.0 };
        probe(&mut rep, &verts, count, c2v { x: 0.0, y: 1.0 }, "inf vertex");
    }

    // The four-vertex AABB proxy layout specifically (row 42), with directions
    // swept all the way round the circle so each of the 4 corners wins.
    let bb = [
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v::default(),
        c2v::default(),
        c2v::default(),
        c2v::default(),
    ];
    for k in 0..720 {
        let ang = k as f32 * std::f32::consts::TAU / 720.0;
        probe(&mut rep, &bb, 4, c2v { x: ang.cos(), y: ang.sin() }, "aabb sweep");
    }
    rep.finish("row40_to_row43_c2Support");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 25 preview: count <= 0 still reads verts[0] and returns 0.
// (Kept here because it needs the same fixture; also asserted in phase_c.)
// ---------------------------------------------------------------------------

#[test]
fn support_nonpositive_count() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSupport>("c2Support"), l.rs.sym::<FnSupport>("c2Support"));
    let mut rep = Report::new();
    let verts = [c2v { x: 3.0, y: -7.0 }; 8];
    for count in [0, -1, -100, i32::MIN] {
        for d in [c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }] {
            let (x, y) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            rep.check(x == y && x == 0, || {
                format!("c2Support(count={count}, d={}): C={x} Rust={y} (both must be 0)", show_v(d))
            });
        }
    }
    rep.finish("support_nonpositive_count");
}
