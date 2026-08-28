//! Phase B, Group 3 — CONFIGS.md rows C21..C38.
//!
//! The `c2Simplex` is constructed by hand (all 152 bytes controlled) so that
//! every arm of `c22` / `c23` / `c2D` / `c2L` / `c2Witness` /
//! `c2GJKSimplexMetric` is reached directly, without going through `c2GJK`.
//! After each call the ENTIRE struct image is compared, which catches partial
//! `c2sv` copies (`s->a = s->b` copies six fields, not just `.u`).

mod common;
use common::*;

use std::collections::BTreeMap;
use std::sync::Mutex;

const N: u32 = 4096;

/// Build a fully-populated simplex with traceable per-vertex contents.
fn mk(seed: u64, pts: &[C2v], count: i32, div: f32) -> C2Simplex {
    let rng = &mut Rng::new(seed);
    let mut s = C2Simplex {
        verts: [C2sv::default(); 4],
        div,
        count,
    };
    for i in 0..4 {
        s.verts[i] = C2sv {
            sA: rng.v_range(-1e3, 1e3),
            sB: rng.v_range(-1e3, 1e3),
            p: if i < pts.len() { pts[i] } else { rng.v_range(-1e3, 1e3) },
            u: rng.range(-10.0, 10.0),
            iA: 100 + i as i32,
            iB: 200 + i as i32,
        };
    }
    s
}

fn run22(base: &C2Simplex, ctx: &str) -> C2Simplex {
    let (c, r): (FnSimplex, FnSimplex) = sym(b"c22");
    let mut sc = *base;
    let mut sr = *base;
    unsafe { c(&mut sc) };
    unsafe { r(&mut sr) };
    assert!(
        raw_same(&sc, &sr),
        "c22 mismatch [{ctx}]\nINPUT: {}\n  C   : {}\n  Rust: {}",
        fmt_simplex(base),
        fmt_simplex(&sc),
        fmt_simplex(&sr)
    );
    sc
}

fn run23(base: &C2Simplex, ctx: &str) -> C2Simplex {
    let (c, r): (FnSimplex, FnSimplex) = sym(b"c23");
    let mut sc = *base;
    let mut sr = *base;
    unsafe { c(&mut sc) };
    unsafe { r(&mut sr) };
    assert!(
        raw_same(&sc, &sr),
        "c23 mismatch [{ctx}]\nINPUT: {}\n  C   : {}\n  Rust: {}",
        fmt_simplex(base),
        fmt_simplex(&sc),
        fmt_simplex(&sr)
    );
    sc
}

// ---------------------------------------------------------------------------
// C21..C24  c22
// ---------------------------------------------------------------------------

#[test]
fn c21_c24_c22_all_arms() {
    let mut rng = Rng::new(0xC21);
    let mut hits = [0u32; 3];
    for i in 0..N {
        // Random 2-simplexes.
        let a = rng.v_range(-100.0, 100.0);
        let b = rng.v_range(-100.0, 100.0);
        let s = mk(rng.next_u64(), &[a, b], 2, rng.range(0.5, 4.0));
        let out = run22(&s, &format!("random #{i}"));
        hits[(out.count.clamp(1, 2) - 1) as usize] += 1;

        // C21: targeted `v <= 0` arm -- origin is beyond b along ba, i.e.
        // dot(a, a-b) <= 0.  Put the origin "past" a.
        let dir = rng.v_range(-1.0, 1.0);
        let a1 = C2v { x: dir.x, y: dir.y };
        let b1 = C2v {
            x: dir.x * 3.0,
            y: dir.y * 3.0,
        };
        // origin, a1, b1 collinear with origin before a1 => dot(a1, a1-b1) < 0
        let s1 = mk(rng.next_u64(), &[a1, b1], 2, 1.0);
        let o1 = run22(&s1, "C21 v<=0 arm");
        assert_eq!(o1.count, 1, "expected the v<=0 arm");

        // C22: targeted `u <= 0` arm -- mirror of the above.
        let s2 = mk(rng.next_u64(), &[b1, a1], 2, 1.0);
        let o2 = run22(&s2, "C22 u<=0 arm");
        assert_eq!(o2.count, 1, "expected the u<=0 arm");
        // the whole c2sv must have been copied from b into a
        assert_eq!(o2.verts[0].iA, s2.verts[1].iA);
        assert_eq!(o2.verts[0].iB, s2.verts[1].iB);
        assert!(v_same(o2.verts[0].sA, s2.verts[1].sA));
        assert!(v_same(o2.verts[0].sB, s2.verts[1].sB));
        assert!(v_same(o2.verts[0].p, s2.verts[1].p));

        // C23: interior arm -- origin strictly between a and b.
        let t = rng.range(-1.0, 1.0);
        let n = C2v { x: -dir.y, y: dir.x };
        let a3 = C2v {
            x: -dir.x + n.x * t,
            y: -dir.y + n.y * t,
        };
        let b3 = C2v {
            x: dir.x + n.x * t,
            y: dir.y + n.y * t,
        };
        let s3 = mk(rng.next_u64(), &[a3, b3], 2, 1.0);
        let o3 = run22(&s3, "C23 interior arm");
        assert_eq!(o3.count, 2, "expected the interior arm");
    }
    assert!(hits[0] > 0 && hits[1] > 0, "random c22 hit both counts: {hits:?}");
}

#[test]
fn c24_c22_degenerate() {
    let mut rng = Rng::new(0xC24);
    for i in 0..N {
        // a.p == b.p
        let p = rng.v_spicy();
        let mut s = mk(rng.next_u64(), &[p, p], 2, rng.spicy());
        run22(&s, &format!("a==b #{i}"));
        // spicy points
        s = mk(rng.next_u64(), &[rng.v_spicy(), rng.v_spicy()], 2, rng.spicy());
        run22(&s, &format!("spicy #{i}"));
        // spicy counts too
        for count in [0i32, 1, 3, 4, -1, i32::MIN, i32::MAX] {
            let mut t = s;
            t.count = count;
            run22(&t, &format!("count={count} #{i}"));
        }
    }
    // fixed degenerates
    let mut rng = Rng::new(0xD24);
    for p in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: f32::NAN, y: 0.0 },
        C2v {
            x: f32::INFINITY,
            y: f32::INFINITY,
        },
        C2v { x: FLT_MAX, y: FLT_MAX },
        C2v { x: 1e-45, y: 1e-45 },
    ] {
        for q in [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: f32::NAN, y: f32::NAN },
            C2v {
                x: f32::NEG_INFINITY,
                y: 1.0,
            },
            C2v { x: 1.0, y: 1.0 },
        ] {
            for div in [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY] {
                let s = mk(rng.next_u64(), &[p, q], 2, div);
                run22(&s, "c22 fixed degenerate");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C25..C33  c23 — one test per arm plus a random sweep
// ---------------------------------------------------------------------------

/// Given a triangle in "shape space" plus a point that should act as the
/// origin, translate so the origin lands where we want it, i.e. build the
/// simplex points as (vertex - origin).
fn tri_about(o: C2v, v: [C2v; 3]) -> [C2v; 3] {
    [
        C2v { x: v[0].x - o.x, y: v[0].y - o.y },
        C2v { x: v[1].x - o.x, y: v[1].y - o.y },
        C2v { x: v[2].x - o.x, y: v[2].y - o.y },
    ]
}

#[test]
fn c25_c33_c23_arms_and_random() {
    let mut rng = Rng::new(0xC25);
    // Track which resulting (count, layout) combinations we reached so the test
    // can assert that all seven arms of the C `if` chain were exercised.
    let arm_hits: Mutex<BTreeMap<String, u32>> = Mutex::new(BTreeMap::new());

    let classify = |before: &C2Simplex, after: &C2Simplex| -> String {
        match after.count {
            1 => {
                if after.verts[0].iA == before.verts[0].iA {
                    "vertexA".to_string()
                } else if after.verts[0].iA == before.verts[1].iA {
                    "vertexB".to_string()
                } else {
                    "vertexC".to_string()
                }
            }
            2 => {
                if after.verts[0].iA == before.verts[0].iA
                    && after.verts[1].iA == before.verts[1].iA
                {
                    "edgeAB".to_string()
                } else if after.verts[0].iA == before.verts[1].iA
                    && after.verts[1].iA == before.verts[2].iA
                {
                    "edgeBC".to_string()
                } else {
                    "edgeCA".to_string()
                }
            }
            3 => "interior".to_string(),
            n => format!("count{n}"),
        }
    };

    // Random triangles with the origin placed all over the plane -- this hits
    // every arm in its natural proportion.
    for i in 0..N {
        let v = [
            rng.v_range(-100.0, 100.0),
            rng.v_range(-100.0, 100.0),
            rng.v_range(-100.0, 100.0),
        ];
        let o = rng.v_range(-150.0, 150.0);
        let pts = tri_about(o, v);
        let s = mk(rng.next_u64(), &pts, 3, rng.range(0.5, 4.0));
        let out = run23(&s, &format!("random #{i}"));
        *arm_hits
            .lock()
            .unwrap()
            .entry(classify(&s, &out))
            .or_insert(0) += 1;
    }

    // Deliberately aim at each region of a fixed triangle so the arms are hit
    // even if the random draw is unlucky.  Both windings are used.
    let base_tri = [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: 10.0, y: 0.0 },
        C2v { x: 0.0, y: 10.0 },
    ];
    let flipped = [base_tri[0], base_tri[2], base_tri[1]];
    let probes = [
        C2v { x: -5.0, y: -5.0 },  // vertex A region
        C2v { x: 20.0, y: -5.0 },  // vertex B region
        C2v { x: -5.0, y: 20.0 },  // vertex C region
        C2v { x: 5.0, y: -5.0 },   // edge AB region
        C2v { x: 9.0, y: 9.0 },    // edge BC region
        C2v { x: -5.0, y: 5.0 },   // edge CA region
        C2v { x: 2.0, y: 2.0 },    // interior
        C2v { x: 3.3, y: 3.3 },    // interior
    ];
    for tri in [base_tri, flipped] {
        for o in probes {
            for _ in 0..64 {
                let pts = tri_about(o, tri);
                let s = mk(rng.next_u64(), &pts, 3, rng.range(0.5, 4.0));
                let out = run23(&s, "targeted arm probe");
                *arm_hits
                    .lock()
                    .unwrap()
                    .entry(classify(&s, &out))
                    .or_insert(0) += 1;
            }
        }
    }

    let hits = arm_hits.lock().unwrap();
    for arm in [
        "vertexA", "vertexB", "vertexC", "edgeAB", "edgeBC", "edgeCA", "interior",
    ] {
        assert!(
            hits.get(arm).copied().unwrap_or(0) > 0,
            "c23 arm {arm} never reached; hits = {hits:?}"
        );
    }
}

#[test]
fn c32_c23_degenerate() {
    let mut rng = Rng::new(0xC32);
    for i in 0..N {
        // collinear (area == 0)
        let a = rng.v_range(-100.0, 100.0);
        let d = rng.v_range(-10.0, 10.0);
        let t1 = rng.range(-5.0, 5.0);
        let t2 = rng.range(-5.0, 5.0);
        let pts = [
            a,
            C2v { x: a.x + d.x * t1, y: a.y + d.y * t1 },
            C2v { x: a.x + d.x * t2, y: a.y + d.y * t2 },
        ];
        run23(&mk(rng.next_u64(), &pts, 3, 1.0), &format!("collinear #{i}"));

        // all equal
        run23(&mk(rng.next_u64(), &[a, a, a], 3, 1.0), &format!("all-equal #{i}"));

        // spicy
        let sp = [rng.v_spicy(), rng.v_spicy(), rng.v_spicy()];
        run23(&mk(rng.next_u64(), &sp, 3, rng.spicy()), &format!("spicy #{i}"));

        // out-of-range counts
        for count in [0i32, 1, 2, 4, -1, i32::MIN, i32::MAX] {
            let mut s = mk(rng.next_u64(), &sp, count, rng.spicy());
            s.count = count;
            run23(&s, &format!("count={count} #{i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C34  c2D
// ---------------------------------------------------------------------------

#[test]
fn c34_c2d() {
    let (c, r): (FnVSimplex, FnVSimplex) = sym(b"c2D");
    let mut rng = Rng::new(0xC34);
    let mut skew = 0u32;
    let mut ccw = 0u32;
    for i in 0..N {
        for count in [1i32, 2, 3, 0, 4, -1, i32::MIN, i32::MAX] {
            let pts = [rng.v_range(-100.0, 100.0), rng.v_range(-100.0, 100.0)];
            let mut sc = mk(rng.next_u64(), &pts, count, rng.range(0.5, 4.0));
            let mut sr = sc;
            let dc = unsafe { c(&mut sc) };
            let dr = unsafe { r(&mut sr) };
            assert_v(dc, dr, &format!("c2D count={count} #{i}"));
            assert_raw(&sc, &sr, "c2D must not mutate the simplex");
            if count == 2 {
                // classify which sub-branch was taken
                let ab = C2v {
                    x: pts[1].x - pts[0].x,
                    y: pts[1].y - pts[0].y,
                };
                if f32_same(dc.x, -ab.y) && f32_same(dc.y, ab.x) {
                    skew += 1;
                } else {
                    ccw += 1;
                }
            }
        }
        // spicy
        let sp = [rng.v_spicy(), rng.v_spicy()];
        for count in [1i32, 2, 3] {
            let mut sc = mk(rng.next_u64(), &sp, count, rng.spicy());
            let mut sr = sc;
            let dc = unsafe { c(&mut sc) };
            let dr = unsafe { r(&mut sr) };
            assert_v(dc, dr, "c2D spicy");
        }
        // det2 exactly zero: origin on the segment's line
        let a = rng.v_range(-50.0, 50.0);
        let k = rng.range(-3.0, 3.0);
        let b = C2v { x: a.x * k, y: a.y * k };
        let mut sc = mk(rng.next_u64(), &[a, b], 2, 1.0);
        let mut sr = sc;
        let dc = unsafe { c(&mut sc) };
        let dr = unsafe { r(&mut sr) };
        assert_v(dc, dr, "c2D collinear-with-origin");
    }
    assert!(skew > 0 && ccw > 0, "c2D branches: skew={skew} ccw={ccw}");
}

// ---------------------------------------------------------------------------
// C35  c2L
// ---------------------------------------------------------------------------

#[test]
fn c35_c2l() {
    let (c, r): (FnVSimplex, FnVSimplex) = sym(b"c2L");
    let mut rng = Rng::new(0xC35);
    for i in 0..N {
        for count in [1i32, 2, 3, 0, 4, -1, i32::MIN, i32::MAX] {
            for div in [
                rng.range(0.25, 8.0),
                0.0,
                -0.0,
                f32::NAN,
                f32::INFINITY,
                1e-45,
                FLT_MAX,
            ] {
                let pts = [rng.v_range(-100.0, 100.0), rng.v_range(-100.0, 100.0)];
                let mut sc = mk(rng.next_u64(), &pts, count, div);
                let mut sr = sc;
                let vc = unsafe { c(&mut sc) };
                let vr = unsafe { r(&mut sr) };
                assert_v(vc, vr, &format!("c2L count={count} div={} #{i}", fmt_f32(div)));
                assert_raw(&sc, &sr, "c2L must not mutate the simplex");
            }
        }
        // spicy u values
        let mut sc = mk(rng.next_u64(), &[rng.v_spicy(), rng.v_spicy()], 2, rng.spicy());
        sc.verts[0].u = rng.spicy();
        sc.verts[1].u = rng.spicy();
        let mut sr = sc;
        let vc = unsafe { c(&mut sc) };
        let vr = unsafe { r(&mut sr) };
        assert_v(vc, vr, "c2L spicy u");
    }
}

// ---------------------------------------------------------------------------
// C36  c2Witness
// ---------------------------------------------------------------------------

#[test]
fn c36_c2witness() {
    let (c, r): (FnWitness, FnWitness) = sym(b"c2Witness");
    let mut rng = Rng::new(0xC36);
    for i in 0..N {
        for count in [1i32, 2, 3, 0, 4, -1, i32::MIN, i32::MAX] {
            for div in [rng.range(0.25, 8.0), 0.0, f32::NAN, f32::INFINITY] {
                let pts = [
                    rng.v_range(-100.0, 100.0),
                    rng.v_range(-100.0, 100.0),
                    rng.v_range(-100.0, 100.0),
                ];
                let mut sc = mk(rng.next_u64(), &pts, count, div);
                if i % 3 == 0 {
                    for v in sc.verts.iter_mut() {
                        v.u = rng.spicy();
                    }
                }
                let mut sr = sc;
                let (mut ac, mut bc) = (C2v { x: 42.0, y: 43.0 }, C2v { x: 44.0, y: 45.0 });
                let (mut ar, mut br) = (ac, bc);
                unsafe { c(&mut sc, &mut ac, &mut bc) };
                unsafe { r(&mut sr, &mut ar, &mut br) };
                assert_v(ac, ar, &format!("c2Witness a count={count} #{i}"));
                assert_v(bc, br, &format!("c2Witness b count={count} #{i}"));
                assert_raw(&sc, &sr, "c2Witness must not mutate the simplex");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C37  c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn c37_metric() {
    let (c, r): (FnFSimplex, FnFSimplex) = sym(b"c2GJKSimplexMetric");
    let mut rng = Rng::new(0xC37);
    for i in 0..N {
        for count in [1i32, 2, 3, 0, 4, 5, -1, i32::MIN, i32::MAX] {
            let pts = [
                rng.v_range(-1e3, 1e3),
                rng.v_range(-1e3, 1e3),
                rng.v_range(-1e3, 1e3),
            ];
            let mut sc = mk(rng.next_u64(), &pts, count, rng.range(0.5, 4.0));
            let mut sr = sc;
            let mc = unsafe { c(&mut sc) };
            let mr = unsafe { r(&mut sr) };
            assert_f32(mc, mr, &format!("c2GJKSimplexMetric count={count} #{i}"));
            assert_raw(&sc, &sr, "metric must not mutate the simplex");
        }
        // spicy points
        let sp = [rng.v_spicy(), rng.v_spicy(), rng.v_spicy()];
        for count in [1i32, 2, 3] {
            let mut sc = mk(rng.next_u64(), &sp, count, rng.spicy());
            let mut sr = sc;
            assert_f32(
                unsafe { c(&mut sc) },
                unsafe { r(&mut sr) },
                "c2GJKSimplexMetric spicy",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C38  c2Support
// ---------------------------------------------------------------------------

#[test]
fn c38_c2support() {
    let (c, r): (FnSupport, FnSupport) = sym(b"c2Support");
    let mut rng = Rng::new(0xC38);
    for i in 0..N {
        let mut verts = [C2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v_range(-1e3, 1e3);
        }
        for count in [0i32, 1, 2, 3, 4, 5, 8, -1, -100] {
            let d = rng.v_range(-1.0, 1.0);
            let cc = unsafe { c(verts.as_ptr(), count, d) };
            let rr = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cc, rr, "c2Support count={count} #{i}");
        }
        // d == 0 (all dots equal 0, ties)
        let z = C2v { x: 0.0, y: 0.0 };
        assert_eq!(
            unsafe { c(verts.as_ptr(), 8, z) },
            unsafe { r(verts.as_ptr(), 8, z) },
            "c2Support d==0"
        );
        // tie-inducing: all verts identical
        let same = [verts[0]; 8];
        let d = rng.v_range(-1.0, 1.0);
        assert_eq!(
            unsafe { c(same.as_ptr(), 8, d) },
            unsafe { r(same.as_ptr(), 8, d) },
            "c2Support all-equal verts"
        );
        // spicy verts and directions
        let mut sp = [C2v::default(); 8];
        for v in sp.iter_mut() {
            *v = rng.v_spicy();
        }
        let sd = rng.v_spicy();
        for count in [1i32, 2, 4, 8] {
            assert_eq!(
                unsafe { c(sp.as_ptr(), count, sd) },
                unsafe { r(sp.as_ptr(), count, sd) },
                "c2Support spicy count={count}"
            );
        }
    }
}
