//! Phase B — CONFIGS.md rows 22..28: the simplex-level entry points
//! (`c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
//! `c2Support`).  These are the low-level functions `c2GJK` composes; they are
//! driven here DIRECTLY with hand-built `c2Simplex` state so that branch
//! combinations unreachable from the one-shot wrapper are still covered.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: u32 = 6000;

fn rand_sv(rng: &mut Rng, mag: f32) -> c2sv {
    c2sv {
        sA: rng.v(mag),
        sB: rng.v(mag),
        p: rng.v(mag),
        u: rng.sym(mag),
        iA: (rng.below(8)) as i32,
        iB: (rng.below(8)) as i32,
    }
}

fn spicy_sv(rng: &mut Rng, mag: f32) -> c2sv {
    c2sv {
        sA: rng.v_spicy(mag),
        sB: rng.v_spicy(mag),
        p: rng.v_spicy(mag),
        u: rng.spicy(mag),
        iA: rng.next_u32() as i32,
        iB: rng.next_u32() as i32,
    }
}

/// Build a simplex whose `p` values are drawn from a generator, with the given
/// `count` and `div`.
fn simplex(rng: &mut Rng, count: i32, div: f32, mag: f32, spicy: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.verts[i] = if spicy { spicy_sv(rng, mag) } else { rand_sv(rng, mag) };
    }
    s.div = div;
    s.count = count;
    s
}

/// Same as `simplex` but draws `div` from the RNG itself (avoids a double
/// mutable borrow at the call sites).
fn simplex_d(rng: &mut Rng, count: i32, mag: f32, spicy: bool) -> c2Simplex {
    let div = rng.spicy(10.0);
    simplex(rng, count, div, mag, spicy)
}

/// `p` values that force degenerate geometry.
fn set_ps(s: &mut c2Simplex, ps: &[c2v]) {
    for (i, p) in ps.iter().enumerate() {
        s.verts[i].p = *p;
    }
}

const ODD_COUNTS: [i32; 6] = [0, 4, -1, 5, i32::MIN, i32::MAX];

// -------------------------------------------------------------------- row 22
#[test]
fn row22_c2GJKSimplexMetric() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3022);
    for _ in 0..N {
        for count in [1i32, 2, 3] {
            for spicy in [false, true] {
                let s = simplex_d(&mut rng, count, 1e3, spicy);
                let mut sc = s;
                let mut sr = s;
                let (a, b) = unsafe {
                    ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr))
                };
                diff_eq!(format!("metric count={count} spicy={spicy}"), fb(a), fb(b));
                // the function must not mutate the simplex
                diff_eq!("metric no-mutate", simplex_bits(&sc), simplex_bits(&sr));
            }
        }
        // degenerate: equal points
        let mut s = simplex(&mut rng, 2, 1.0, 10.0, false);
        let p = rng.v(10.0);
        set_ps(&mut s, &[p, p, p]);
        for count in [2i32, 3] {
            s.count = count;
            let mut sc = s;
            let mut sr = s;
            let (a, b) =
                unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
            diff_eq!(format!("metric degenerate count={count}"), fb(a), fb(b));
        }
    }
    for &count in ODD_COUNTS.iter() {
        for _ in 0..64 {
            let s = simplex_d(&mut rng, count, 1e3, true);
            let mut sc = s;
            let mut sr = s;
            let (a, b) =
                unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
            diff_eq!(format!("metric odd count={count}"), fb(a), fb(b));
        }
    }
}

// -------------------------------------------------------------------- row 23
#[test]
fn row23_c22() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3023);
    let mut hits = [0u32; 3]; // branch coverage: count==1(v<=0), count==1(u<=0), count==2
    for _ in 0..N {
        for mag in [1.0f32, 100.0, 1e-6, 1e18] {
            let mut s = simplex_d(&mut rng, 2, mag, false);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (c.c22)(&mut sc);
                (r.c22)(&mut sr);
            }
            diff_eq!(format!("c22 mag={mag}"), simplex_bits(&sc), simplex_bits(&sr));
            if sc.count == 2 {
                hits[2] += 1;
            } else {
                // distinguish the two collapse branches by the surviving vertex
                if sc.verts[0].iA == s.verts[0].iA && sc.verts[0].iB == s.verts[0].iB {
                    hits[0] += 1;
                } else {
                    hits[1] += 1;
                }
            }
            let _ = &mut s;
        }
        // degenerate a.p == b.p
        let mut s = simplex(&mut rng, 2, 1.0, 10.0, false);
        let p = rng.v(10.0);
        set_ps(&mut s, &[p, p]);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
        }
        diff_eq!("c22 a.p==b.p", simplex_bits(&sc), simplex_bits(&sr));

        // collinear with the origin (u or v exactly 0)
        let d = rng.v(10.0);
        set_ps(&mut s, &[d, c2v { x: d.x * 2.0, y: d.y * 2.0 }]);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
        }
        diff_eq!("c22 collinear", simplex_bits(&sc), simplex_bits(&sr));

        // spicy / NaN
        let s = simplex_d(&mut rng, 2, 1e3, true);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
        }
        diff_eq!("c22 spicy", simplex_bits(&sc), simplex_bits(&sr));
    }
    assert!(hits.iter().all(|&h| h > 0), "c22 branch coverage incomplete: {hits:?}");
}

// -------------------------------------------------------------------- row 24
#[test]
fn row24_c23() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3024);
    // Track which of the 7 branches fired, identified by (count, div-source).
    let mut seen_count = [0u32; 4]; // index by resulting count 1..3
    for _ in 0..N {
        for mag in [1.0f32, 100.0, 1e-6, 1e18] {
            let s = simplex_d(&mut rng, 3, mag, false);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (c.c23)(&mut sc);
                (r.c23)(&mut sr);
            }
            diff_eq!(format!("c23 mag={mag}"), simplex_bits(&sc), simplex_bits(&sr));
            if (1..=3).contains(&sc.count) {
                seen_count[sc.count as usize] += 1;
            }
        }
        // deliberately place the origin inside / outside the triangle
        let ctr = rng.v(5.0);
        let rad = 1.0 + rng.unit() * 20.0;
        for k in 0..3 {
            // CCW and CW windings
            for cw in [false, true] {
                let mut ps = [c2v::default(); 3];
                for i in 0..3 {
                    let mut t = (i as f32) * std::f32::consts::TAU / 3.0 + (k as f32) * 0.7;
                    if cw {
                        t = -t;
                    }
                    ps[i] = c2v { x: ctr.x + rad * t.cos(), y: ctr.y + rad * t.sin() };
                }
                let mut s = simplex_d(&mut rng, 3, 1.0, false);
                set_ps(&mut s, &ps);
                let mut sc = s;
                let mut sr = s;
                unsafe {
                    (c.c23)(&mut sc);
                    (r.c23)(&mut sr);
                }
                diff_eq!(format!("c23 tri k={k} cw={cw}"), simplex_bits(&sc), simplex_bits(&sr));
                if (1..=3).contains(&sc.count) {
                    seen_count[sc.count as usize] += 1;
                }
            }
        }
        // all three points equal
        let mut s = simplex(&mut rng, 3, 1.0, 10.0, false);
        let p = rng.v(10.0);
        set_ps(&mut s, &[p, p, p]);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("c23 all-equal", simplex_bits(&sc), simplex_bits(&sr));

        // collinear (area == 0)
        let d = rng.v(10.0);
        let o = rng.v(10.0);
        set_ps(
            &mut s,
            &[
                o,
                c2v { x: o.x + d.x, y: o.y + d.y },
                c2v { x: o.x + d.x * 2.0, y: o.y + d.y * 2.0 },
            ],
        );
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("c23 collinear", simplex_bits(&sc), simplex_bits(&sr));

        // spicy / NaN -> falls through every guard to the `else`
        let s = simplex_d(&mut rng, 3, 1e3, true);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("c23 spicy", simplex_bits(&sc), simplex_bits(&sr));
    }
    assert!(
        seen_count[1] > 0 && seen_count[2] > 0 && seen_count[3] > 0,
        "c23 result-count coverage incomplete: {seen_count:?}"
    );
}

// -------------------------------------------------------------------- row 25
#[test]
fn row25_c2D() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3025);
    let mut skew = 0u32;
    let mut ccw = 0u32;
    for _ in 0..N {
        for count in [1i32, 2, 3] {
            for spicy in [false, true] {
                let s = simplex_d(&mut rng, count, 1e3, spicy);
                let mut sc = s;
                let mut sr = s;
                let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
                diff_eq!(format!("c2D count={count} spicy={spicy}"), vb(a), vb(b));
                diff_eq!("c2D no-mutate", simplex_bits(&sc), simplex_bits(&sr));
                if count == 2 && !spicy {
                    let ab = c2v {
                        x: s.verts[1].p.x - s.verts[0].p.x,
                        y: s.verts[1].p.y - s.verts[0].p.y,
                    };
                    let det = ab.x * -s.verts[0].p.y - ab.y * -s.verts[0].p.x;
                    if det > 0.0 {
                        skew += 1;
                    } else {
                        ccw += 1;
                    }
                }
            }
        }
        // det2 == 0 exactly: a.p and b.p collinear with the origin
        let d = rng.v(10.0);
        let mut s = simplex(&mut rng, 2, 1.0, 10.0, false);
        set_ps(&mut s, &[d, c2v { x: d.x * 3.0, y: d.y * 3.0 }]);
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
        diff_eq!("c2D det==0", vb(a), vb(b));
        // a.p == b.p
        let p = rng.v(10.0);
        set_ps(&mut s, &[p, p]);
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
        diff_eq!("c2D degenerate edge", vb(a), vb(b));
    }
    for &count in ODD_COUNTS.iter() {
        for _ in 0..64 {
            let s = simplex_d(&mut rng, count, 1e3, true);
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
            diff_eq!(format!("c2D odd count={count}"), vb(a), vb(b));
        }
    }
    assert!(skew > 0 && ccw > 0, "c2D branch coverage incomplete: skew={skew} ccw={ccw}");
}

// -------------------------------------------------------------------- row 26
#[test]
fn row26_c2L() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3026);
    let divs = [
        1.0f32,
        0.0,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        FLT_EPSILON,
        -3.5,
        FLT_MAX,
    ];
    for _ in 0..N {
        for count in [1i32, 2, 3] {
            for &div in divs.iter() {
                let s = simplex(&mut rng, count, div, 1e3, false);
                let mut sc = s;
                let mut sr = s;
                let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
                diff_eq!(format!("c2L count={count} div={div:?}"), vb(a), vb(b));
                diff_eq!("c2L no-mutate", simplex_bits(&sc), simplex_bits(&sr));
            }
            // u == 0 exactly (with div == 0 -> inf * 0 -> NaN)
            let mut s = simplex(&mut rng, count, 0.0, 1e3, false);
            for i in 0..4 {
                s.verts[i].u = 0.0;
            }
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
            diff_eq!(format!("c2L u=0 count={count}"), vb(a), vb(b));
        }
        let s = simplex_d(&mut rng, 2, 1e3, true);
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
        diff_eq!("c2L spicy", vb(a), vb(b));
    }
    for &count in ODD_COUNTS.iter() {
        for _ in 0..64 {
            let s = simplex_d(&mut rng, count, 1e3, true);
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
            diff_eq!(format!("c2L odd count={count}"), vb(a), vb(b));
        }
    }
}

// -------------------------------------------------------------------- row 27
#[test]
fn row27_c2Witness() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3027);
    let divs = [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, 2.5, FLT_EPSILON, FLT_MAX];
    for _ in 0..N {
        for count in [1i32, 2, 3] {
            for &div in divs.iter() {
                let s = simplex(&mut rng, count, div, 1e3, false);
                let mut sc = s;
                let mut sr = s;
                let mut ac = c2v { x: 9.5, y: -9.5 };
                let mut bc = c2v { x: -9.5, y: 9.5 };
                let mut ar = ac;
                let mut br = bc;
                unsafe {
                    (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                diff_eq!(
                    format!("witness count={count} div={div:?}"),
                    (vb(ac), vb(bc)),
                    (vb(ar), vb(br))
                );
                diff_eq!("witness no-mutate", simplex_bits(&sc), simplex_bits(&sr));
            }
        }
        let s = simplex_d(&mut rng, 3, 1e3, true);
        let mut sc = s;
        let mut sr = s;
        let (mut ac, mut bc) = (c2v::default(), c2v::default());
        let (mut ar, mut br) = (c2v::default(), c2v::default());
        unsafe {
            (c.c2Witness)(&mut sc, &mut ac, &mut bc);
            (r.c2Witness)(&mut sr, &mut ar, &mut br);
        }
        diff_eq!("witness spicy", (vb(ac), vb(bc)), (vb(ar), vb(br)));
    }
    for &count in ODD_COUNTS.iter() {
        for _ in 0..64 {
            let s = simplex_d(&mut rng, count, 1e3, true);
            let mut sc = s;
            let mut sr = s;
            let mut ac = c2v { x: 7.0, y: 7.0 };
            let mut bc = c2v { x: 8.0, y: 8.0 };
            let mut ar = ac;
            let mut br = bc;
            unsafe {
                (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            diff_eq!(
                format!("witness odd count={count}"),
                (vb(ac), vb(bc)),
                (vb(ar), vb(br))
            );
        }
    }
}

// -------------------------------------------------------------------- row 28
#[test]
fn row28_c2Support() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3028);
    for _ in 0..N {
        let mut verts = [c2v::default(); 8];
        for count in 1..=8i32 {
            for spicy in [false, true] {
                for i in 0..8 {
                    verts[i] = if spicy { rng.v_spicy(1e3) } else { rng.v(1e3) };
                }
                let d = if spicy { rng.v_spicy(1e3) } else { rng.v(1e3) };
                let (a, b) = unsafe {
                    (
                        (c.c2Support)(verts.as_ptr(), count, d),
                        (r.c2Support)(verts.as_ptr(), count, d),
                    )
                };
                diff_eq!(format!("support count={count} spicy={spicy} d={d:?}"), a, b);
            }
        }
        // d == (0,0): every dot is 0 -> index 0 wins
        for i in 0..8 {
            verts[i] = rng.v(1e3);
        }
        let z = c2v { x: 0.0, y: 0.0 };
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                ((c.c2Support)(verts.as_ptr(), count, z), (r.c2Support)(verts.as_ptr(), count, z))
            };
            diff_eq!(format!("support d=0 count={count}"), a, b);
        }
        // all verts equal -> tie, first index wins
        let p = rng.v(1e3);
        let eq = [p; 8];
        let d = rng.v(1e3);
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                ((c.c2Support)(eq.as_ptr(), count, d), (r.c2Support)(eq.as_ptr(), count, d))
            };
            diff_eq!(format!("support ties count={count}"), a, b);
        }
        // coarse values -> frequent exact ties on the dot products
        for i in 0..8 {
            verts[i] = rng.v_coarse(4.0);
        }
        let d = rng.v_coarse(4.0);
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            diff_eq!(format!("support coarse count={count}"), a, b);
        }
        // verts[0] is NaN -> dmax is NaN, no later index can win
        verts[0] = c2v { x: f32::NAN, y: f32::NAN };
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            diff_eq!(format!("support nan0 count={count}"), a, b);
        }
    }
}
