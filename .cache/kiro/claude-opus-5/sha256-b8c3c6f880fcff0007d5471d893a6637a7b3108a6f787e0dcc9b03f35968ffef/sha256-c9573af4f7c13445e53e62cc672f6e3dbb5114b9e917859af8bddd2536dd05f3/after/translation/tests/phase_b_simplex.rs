//! Phase B rows 22-27 and 55: the simplex-level entry points (`c22`, `c23`,
#![allow(non_snake_case)]
//! `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`) plus a hand-driven
//! reproduction of `c2GJK`'s pipeline built only from the low-level exports.

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 6000;

fn sv(p: c2v, iA: c_int, iB: c_int, u: f32) -> c2sv {
    c2sv {
        sA: c2v { x: p.x * 0.25 + 1.0, y: p.y * 0.5 - 2.0 },
        sB: c2v { x: p.x * 0.75 - 3.0, y: p.y * 0.125 + 4.0 },
        p,
        u,
        iA,
        iB,
    }
}

/// Random simplex whose three `p` values cover the interesting geometries.
fn rand_simplex(rng: &mut Rng, count: c_int) -> c2Simplex {
    let mode = rng.below(10);
    let a = match mode {
        8 => rng.wild_v(),
        _ => rng.tame_v(50.0),
    };
    let b = match mode {
        0 => a,                                     // duplicate
        1 => c2v { x: -a.x, y: -a.y },              // origin on segment
        2 => c2v { x: a.x * 2.0, y: a.y * 2.0 },    // collinear with origin
        8 => rng.wild_v(),
        _ => rng.tame_v(50.0),
    };
    let c = match mode {
        3 => a,
        4 => b,
        5 => c2v { x: a.x + (b.x - a.x) * 2.0, y: a.y + (b.y - a.y) * 2.0 }, // collinear
        6 => c2v { x: -(a.x + b.x), y: -(a.y + b.y) }, // often encloses origin
        8 => rng.wild_v(),
        _ => rng.tame_v(50.0),
    };
    let d = rng.tame_v(50.0);
    let mut s = c2Simplex {
        verts: [
            sv(a, rng.below(8) as c_int, rng.below(8) as c_int, rng.sym(5.0)),
            sv(b, rng.below(8) as c_int, rng.below(8) as c_int, rng.sym(5.0)),
            sv(c, rng.below(8) as c_int, rng.below(8) as c_int, rng.sym(5.0)),
            sv(d, rng.below(8) as c_int, rng.below(8) as c_int, rng.sym(5.0)),
        ],
        div: match rng.below(8) {
            0 => 0.0,
            1 => 1.0,
            2 => f32::MAX,
            _ => rng.unit() * 10.0 + 0.001,
        },
        count,
    };
    if mode == 9 {
        // NaN barycentrics / NaN points
        s.verts[0].p.x = f32::NAN;
        s.verts[1].u = f32::NAN;
        s.div = f32::NAN;
    }
    s
}

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

// --- row 22 ----------------------------------------------------------------
#[test]
fn row22_c22() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 22);
    // branch histogram: 0 = v<=0, 1 = u<=0, 2 = interior
    let mut hist = [0usize; 3];
    for i in 0..N {
        let s = rand_simplex(&mut rng, 2);
        let (a, b) = (s.verts[0].p, s.verts[1].p);
        let u = dot(b, sub(b, a));
        let v = dot(a, sub(a, b));
        hist[if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        }] += 1;

        let mut sc = s;
        let mut sr = s;
        unsafe {
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
        }
        ck_simplex(&sc, &sr, &format!("row22 i={i} u={u} v={v} in={s:?}"));
    }
    assert!(
        hist.iter().all(|&n| n > 0),
        "row22 did not cover all three c22 branches: {hist:?}"
    );
    println!("row22 branch histogram: {hist:?}");

    // Explicit degenerate cases.
    let cases: [(c2v, c2v); 6] = [
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }),   // a == b -> u=v=0
        (c2v { x: 1.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }),  // origin on segment
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),   // a at origin
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),   // b at origin
        (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: f32::INFINITY, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
    ];
    for (k, (a, b)) in cases.iter().enumerate() {
        for div in [0.0f32, 1.0, 7.5] {
            let s = c2Simplex {
                verts: [sv(*a, 1, 2, 3.0), sv(*b, 4, 5, 6.0), sv(*b, 6, 7, 8.0), sv(*a, 0, 1, 2.0)],
                div,
                count: 2,
            };
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c22)(&mut sc);
                (p.r.c22)(&mut sr);
            }
            ck_simplex(&sc, &sr, &format!("row22 degenerate k={k} div={div}"));
        }
    }
}

// --- row 23 ----------------------------------------------------------------
fn c23_branch(s: &c2Simplex) -> usize {
    let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let uABC = det2(b, c) * area;
    let vABC = det2(c, a) * area;
    let wABC = det2(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn row23_c23() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 23);
    let mut hist = [0usize; 7];
    for i in 0..N {
        let s = rand_simplex(&mut rng, 3);
        hist[c23_branch(&s)] += 1;
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
        }
        ck_simplex(&sc, &sr, &format!("row23 i={i} branch={} in={s:?}", c23_branch(&s)));
    }
    assert!(
        hist.iter().all(|&n| n > 0),
        "row23 did not cover all seven c23 branches: {hist:?}"
    );
    println!("row23 branch histogram: {hist:?}");

    // Explicit degenerate triangles.
    let tris: [[c2v; 3]; 8] = [
        // collinear (area == 0)
        [c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }, c2v { x: 3.0, y: 3.0 }],
        // all identical
        [c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }],
        // two identical
        [c2v { x: 1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }],
        // origin strictly inside
        [c2v { x: -1.0, y: -1.0 }, c2v { x: 2.0, y: -1.0 }, c2v { x: 0.0, y: 2.0 }],
        // origin on an edge
        [c2v { x: -1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 3.0 }],
        // origin is a vertex
        [c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }],
        // NaN
        [c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }],
        // huge (area overflows)
        [c2v { x: 3.0e38, y: 0.0 }, c2v { x: 0.0, y: 3.0e38 }, c2v { x: -3.0e38, y: -3.0e38 }],
    ];
    for (k, t) in tris.iter().enumerate() {
        for div in [0.0f32, 1.0, 3.0e38] {
            let s = c2Simplex {
                verts: [sv(t[0], 1, 2, 9.0), sv(t[1], 3, 4, 8.0), sv(t[2], 5, 6, 7.0), sv(t[0], 7, 0, 6.0)],
                div,
                count: 3,
            };
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c23)(&mut sc);
                (p.r.c23)(&mut sr);
            }
            ck_simplex(&sc, &sr, &format!("row23 degenerate k={k} div={div}"));
        }
    }
}

// --- row 24 ----------------------------------------------------------------
#[test]
fn row24_c2D() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 24);
    let mut hist = [0usize; 4]; // count1, count2/skew, count2/ccw90, other
    for i in 0..N {
        let count = [1i32, 2, 2, 3, 0, -1, 4, 7][(i % 8) as usize];
        let s = rand_simplex(&mut rng, count);
        match count {
            1 => hist[0] += 1,
            2 => {
                let ab = sub(s.verts[1].p, s.verts[0].p);
                let neg = c2v { x: -s.verts[0].p.x, y: -s.verts[0].p.y };
                if det2(ab, neg) > 0.0 {
                    hist[1] += 1
                } else {
                    hist[2] += 1
                }
            }
            _ => hist[3] += 1,
        }
        let mut sc = s;
        let mut sr = s;
        unsafe {
            let vc = (p.c.c2D)(&mut sc);
            let vr = (p.r.c2D)(&mut sr);
            ck_v(vc, vr, &format!("row24 i={i} count={count} in={s:?}"));
        }
        // c2D must not mutate the simplex.
        ck_simplex(&sc, &sr, &format!("row24 i={i} simplex untouched"));
    }
    assert!(hist.iter().all(|&n| n > 0), "row24 branch coverage: {hist:?}");
    println!("row24 branch histogram: {hist:?}");
}

// --- row 25 ----------------------------------------------------------------
#[test]
fn row25_c2L() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..N {
        let count = [1i32, 2, 3, 0, -1, 4, 5, 2][(i % 8) as usize];
        let mut s = rand_simplex(&mut rng, count);
        if i % 9 == 0 {
            s.div = 0.0;
        }
        let mut sc = s;
        let mut sr = s;
        unsafe {
            let vc = (p.c.c2L)(&mut sc);
            let vr = (p.r.c2L)(&mut sr);
            ck_v(vc, vr, &format!("row25 i={i} count={count} div={} in={s:?}", s.div));
        }
        ck_simplex(&sc, &sr, &format!("row25 i={i} simplex untouched"));
    }
}

// --- row 26 ----------------------------------------------------------------
#[test]
fn row26_c2Witness() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..N {
        let count = [1i32, 2, 3, 0, -1, 4, 8, 3][(i % 8) as usize];
        let mut s = rand_simplex(&mut rng, count);
        match i % 11 {
            0 => s.div = 0.0,
            1 => s.div = f32::MAX,
            2 => s.div = f32::MIN_POSITIVE,
            3 => s.div = f32::NAN,
            _ => {}
        }
        let (mut ac, mut bc) = (sentinel_v(), sentinel_v());
        let (mut ar, mut br) = (sentinel_v(), sentinel_v());
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
            (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
        }
        let ctx = format!("row26 i={i} count={count} div={} in={s:?}", s.div);
        ck_v(ac, ar, &format!("{ctx} outA"));
        ck_v(bc, br, &format!("{ctx} outB"));
        ck_simplex(&sc, &sr, &format!("{ctx} simplex untouched"));
    }
}

// --- row 27 ----------------------------------------------------------------
#[test]
fn row27_c2GJKSimplexMetric() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..N {
        let count = [1i32, 2, 3, 0, -1, 4, 100, -100][(i % 8) as usize];
        let s = rand_simplex(&mut rng, count);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            let fc = (p.c.c2GJKSimplexMetric)(&mut sc);
            let fr = (p.r.c2GJKSimplexMetric)(&mut sr);
            ck_f(fc, fr, &format!("row27 i={i} count={count} in={s:?}"));
        }
        ck_simplex(&sc, &sr, &format!("row27 i={i} simplex untouched"));
    }
}

// --- row 55: hand-driven pipeline built only from the low-level exports ----
/// Reimplements `c2GJK`'s loop in the test, calling `c2MakeProxy`, `c2Support`,
/// `c22`, `c23`, `c2D`, `c2L`, `c2Witness` and `c2GJKSimplexMetric` through the
/// `.so` under test, and compares the whole simplex after every single step.
/// This catches divergences that only show up in the composed pipeline.
#[test]
fn row55_manual_pipeline() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 55);
    const FLT_EPS: f32 = 1.192_092_895_507_812_5e-7;

    for iterc in 0..900 {
        let tyA = ALL_TYPES[(rng.below(3)) as usize];
        let tyB = ALL_TYPES[(rng.below(3)) as usize];
        let mag = [1.0f32, 50.0, 1.0e6, 1.0e-6][rng.below(4) as usize];
        let sa = rand_shape(&mut rng, tyA, mag, 5);
        let sb = rand_shape(&mut rng, tyB, mag, 5);
        let ax = rng.xform(mag);
        let bx = rng.xform(mag);
        let ctx0 = format!(
            "row55 it={iterc} A={} B={} tyA={} tyB={}",
            sa.describe(),
            sb.describe(),
            type_name(tyA),
            type_name(tyB)
        );

        // 1. proxies
        let mut pAc = c2Proxy::default();
        let mut pAr = c2Proxy::default();
        let mut pBc = c2Proxy::default();
        let mut pBr = c2Proxy::default();
        unsafe {
            (p.c.c2MakeProxy)(sa.as_ptr(), tyA, &mut pAc);
            (p.r.c2MakeProxy)(sa.as_ptr(), tyA, &mut pAr);
            (p.c.c2MakeProxy)(sb.as_ptr(), tyB, &mut pBc);
            (p.r.c2MakeProxy)(sb.as_ptr(), tyB, &mut pBr);
        }
        ck_proxy(&pAc, &pAr, &format!("{ctx0} proxyA"));
        ck_proxy(&pBc, &pBr, &format!("{ctx0} proxyB"));

        // 2. initial simplex, built through c2Mulxv / c2Sub
        let mut sc = c2Simplex::default();
        let mut sr = c2Simplex::default();
        unsafe {
            sc.verts[0].sA = (p.c.c2Mulxv)(ax, pAc.verts[0]);
            sc.verts[0].sB = (p.c.c2Mulxv)(bx, pBc.verts[0]);
            sc.verts[0].p = (p.c.c2Sub)(sc.verts[0].sB, sc.verts[0].sA);
            sr.verts[0].sA = (p.r.c2Mulxv)(ax, pAr.verts[0]);
            sr.verts[0].sB = (p.r.c2Mulxv)(bx, pBr.verts[0]);
            sr.verts[0].p = (p.r.c2Sub)(sr.verts[0].sB, sr.verts[0].sA);
        }
        sc.verts[0].u = 1.0;
        sr.verts[0].u = 1.0;
        sc.div = 1.0;
        sr.div = 1.0;
        sc.count = 1;
        sr.count = 1;
        ck_simplex(&sc, &sr, &format!("{ctx0} initial simplex"));

        let mut d0 = f32::MAX;
        let mut iter = 0i32;
        while iter < 20 {
            let save_count = sc.count;
            let mut saveA = [0i32; 3];
            let mut saveB = [0i32; 3];
            for i in 0..save_count.max(0).min(3) as usize {
                saveA[i] = sc.verts[i].iA;
                saveB[i] = sc.verts[i].iB;
            }
            unsafe {
                match sc.count {
                    2 => {
                        (p.c.c22)(&mut sc);
                        (p.r.c22)(&mut sr);
                    }
                    3 => {
                        (p.c.c23)(&mut sc);
                        (p.r.c23)(&mut sr);
                    }
                    _ => {}
                }
            }
            ck_simplex(&sc, &sr, &format!("{ctx0} after c22/c23 iter={iter}"));
            if sc.count == 3 {
                break;
            }

            let (lc, lr) = unsafe { ((p.c.c2L)(&mut sc), (p.r.c2L)(&mut sr)) };
            ck_v(lc, lr, &format!("{ctx0} c2L iter={iter}"));
            let (d1c, d1r) = unsafe { ((p.c.c2Dot)(lc, lc), (p.r.c2Dot)(lr, lr)) };
            ck_f(d1c, d1r, &format!("{ctx0} c2Dot(L,L) iter={iter}"));
            if d1c > d0 {
                break;
            }
            d0 = d1c;

            let (dc, dr) = unsafe { ((p.c.c2D)(&mut sc), (p.r.c2D)(&mut sr)) };
            ck_v(dc, dr, &format!("{ctx0} c2D iter={iter}"));
            let (ddc, ddr) = unsafe { ((p.c.c2Dot)(dc, dc), (p.r.c2Dot)(dr, dr)) };
            ck_f(ddc, ddr, &format!("{ctx0} c2Dot(d,d) iter={iter}"));
            if ddc < FLT_EPS * FLT_EPS {
                break;
            }

            let (negc, negr) = unsafe { ((p.c.c2Neg)(dc), (p.r.c2Neg)(dr)) };
            ck_v(negc, negr, &format!("{ctx0} c2Neg iter={iter}"));
            let (tac, tar) = unsafe { ((p.c.c2MulrvT)(ax.r, negc), (p.r.c2MulrvT)(ax.r, negr)) };
            ck_v(tac, tar, &format!("{ctx0} c2MulrvT(ax) iter={iter}"));
            let (tbc, tbr) = unsafe { ((p.c.c2MulrvT)(bx.r, dc), (p.r.c2MulrvT)(bx.r, dr)) };
            ck_v(tbc, tbr, &format!("{ctx0} c2MulrvT(bx) iter={iter}"));

            let (iAc, iAr) = unsafe {
                (
                    (p.c.c2Support)(pAc.verts.as_ptr(), pAc.count, tac),
                    (p.r.c2Support)(pAr.verts.as_ptr(), pAr.count, tar),
                )
            };
            ck_i(iAc, iAr, &format!("{ctx0} supportA iter={iter}"));
            let (iBc, iBr) = unsafe {
                (
                    (p.c.c2Support)(pBc.verts.as_ptr(), pBc.count, tbc),
                    (p.r.c2Support)(pBr.verts.as_ptr(), pBr.count, tbr),
                )
            };
            ck_i(iBc, iBr, &format!("{ctx0} supportB iter={iter}"));

            let slot = sc.count.clamp(0, 3) as usize;
            unsafe {
                sc.verts[slot].iA = iAc;
                sc.verts[slot].sA = (p.c.c2Mulxv)(ax, pAc.verts[iAc as usize & 7]);
                sc.verts[slot].iB = iBc;
                sc.verts[slot].sB = (p.c.c2Mulxv)(bx, pBc.verts[iBc as usize & 7]);
                sc.verts[slot].p = (p.c.c2Sub)(sc.verts[slot].sB, sc.verts[slot].sA);
                sr.verts[slot].iA = iAr;
                sr.verts[slot].sA = (p.r.c2Mulxv)(ax, pAr.verts[iAr as usize & 7]);
                sr.verts[slot].iB = iBr;
                sr.verts[slot].sB = (p.r.c2Mulxv)(bx, pBr.verts[iBr as usize & 7]);
                sr.verts[slot].p = (p.r.c2Sub)(sr.verts[slot].sB, sr.verts[slot].sA);
            }
            ck_simplex(&sc, &sr, &format!("{ctx0} after support push iter={iter}"));

            let mut dup = false;
            for i in 0..save_count.max(0).min(3) as usize {
                if iAc == saveA[i] && iBc == saveB[i] {
                    dup = true;
                    break;
                }
            }
            if dup {
                break;
            }
            sc.count += 1;
            sr.count += 1;
            iter += 1;
        }

        // 3. witness + metric
        let (mut ac, mut bc) = (sentinel_v(), sentinel_v());
        let (mut ar, mut br) = (sentinel_v(), sentinel_v());
        unsafe {
            (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
            (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
        }
        ck_v(ac, ar, &format!("{ctx0} witness a"));
        ck_v(bc, br, &format!("{ctx0} witness b"));
        unsafe {
            let dc = (p.c.c2Len)((p.c.c2Sub)(ac, bc));
            let dr = (p.r.c2Len)((p.r.c2Sub)(ar, br));
            ck_f(dc, dr, &format!("{ctx0} final dist"));
            let mc = (p.c.c2GJKSimplexMetric)(&mut sc);
            let mr = (p.r.c2GJKSimplexMetric)(&mut sr);
            ck_f(mc, mr, &format!("{ctx0} final metric"));
            // radius shrink, again through the exports
            let nc = (p.c.c2Norm)((p.c.c2Sub)(bc, ac));
            let nr = (p.r.c2Norm)((p.r.c2Sub)(br, ar));
            ck_v(nc, nr, &format!("{ctx0} normal"));
            let a2c = (p.c.c2Add)(ac, (p.c.c2Mulvs)(nc, pAc.radius));
            let a2r = (p.r.c2Add)(ar, (p.r.c2Mulvs)(nr, pAr.radius));
            ck_v(a2c, a2r, &format!("{ctx0} shrunk a"));
            let b2c = (p.c.c2Sub)(bc, (p.c.c2Mulvs)(nc, pBc.radius));
            let b2r = (p.r.c2Sub)(br, (p.r.c2Mulvs)(nr, pBr.radius));
            ck_v(b2c, b2r, &format!("{ctx0} shrunk b"));
            let mc2 = (p.c.c2Mulvs)((p.c.c2Add)(ac, bc), 0.5);
            let mr2 = (p.r.c2Mulvs)((p.r.c2Add)(ar, br), 0.5);
            ck_v(mc2, mr2, &format!("{ctx0} midpoint"));
        }
        ck_simplex(&sc, &sr, &format!("{ctx0} final simplex"));
    }
}

fn pair() -> Pair {
    load_pair()
}
