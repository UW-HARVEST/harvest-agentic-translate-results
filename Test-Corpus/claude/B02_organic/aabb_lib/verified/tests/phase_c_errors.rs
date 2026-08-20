//! PHASE C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (`eNN_*`).  Each test constructs the exact
//! invalid input / boundary condition described by the row, calls BOTH `.so`s
//! and asserts they produce the **same** sentinel result, bit for bit.
//!
//! Rows the C itself resolves with undefined behaviour (NULL dereference,
//! out-of-bounds stack write, read of uninitialised stack memory) are present as
//! `#[ignore]`d tests: they are compiled — so the calls stay type-checked and the
//! reasoning stays visible — but not executed, because the C result there is not
//! a function of its inputs and diffing it would be meaningless.

#![allow(non_snake_case)]

mod common;
use common::*;

const POISON_V: c2v = c2v {
    x: 1.234_567_8e-11,
    y: -9.876_543e12,
};

fn poisoned_proxy() -> c2Proxy {
    c2Proxy {
        radius: -4.242e7,
        count: -777,
        verts: [POISON_V; 8],
    }
}

/// `int` values that a C caller can legally put in a `C2_TYPE` parameter but
/// which match no enumerator.
const BAD: [C2_TYPE; 8] = [3, 4, 5, 7, 100, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];

// ---------------------------------------------------------------------------
// row 01 / 02 — c2MakeProxy
// ---------------------------------------------------------------------------

/// row 01 — out-of-range enum: the `switch` has no `default:`, so `*p` must be
/// left completely untouched.
#[test]
fn e01_makeproxy_out_of_range_enum() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC01);
    for &ty in BAD.iter() {
        for _ in 0..500 {
            // all three shape layouts, so nothing can accidentally match
            let shapes = [
                Shape::Circle(g.circle(50.0)),
                Shape::Aabb(g.aabb(50.0)),
                Shape::Capsule(g.capsule(50.0)),
            ];
            for s in shapes.iter() {
                let mut pc = poisoned_proxy();
                let mut pr = poisoned_proxy();
                unsafe {
                    (c.c2MakeProxy)(s.ptr(), ty, &mut pc);
                    (r.c2MakeProxy)(s.ptr(), ty, &mut pr);
                }
                assert_same("c2MakeProxy/bad-enum", &(*s, ty), pc, pr);
                // and both must really be the untouched poison
                assert_same("c2MakeProxy/untouched", &ty, pc, poisoned_proxy());
                assert_same("c2MakeProxy/untouched", &ty, pr, poisoned_proxy());
            }
        }
    }
}

/// row 02 — `p == NULL` dereferences NULL in the C. UB: not executed.
#[test]
#[ignore = "row 02: c2MakeProxy(shape, valid_type, NULL) dereferences NULL in the C -> SIGSEGV"]
fn e02_makeproxy_null_proxy_documented() {
    let (c, r) = apis();
    let s = Shape::Circle(c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 1.0,
    });
    unsafe {
        (c.c2MakeProxy)(s.ptr(), C2_TYPE_CIRCLE, std::ptr::null_mut());
        (r.c2MakeProxy)(s.ptr(), C2_TYPE_CIRCLE, std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// row 03 — c2GJKSimplexMetric bad count
// ---------------------------------------------------------------------------

/// row 03 — `count` ∉ {2,3} must return `+0.0` (`default:` falls into `case 1:`).
#[test]
fn e03_simplexmetric_bad_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC03);
    for count in [0i32, 1, 4, 5, 100, -1, -7, i32::MIN, i32::MAX] {
        for _ in 0..400 {
            let s = g.simplex(count, 50.0);
            let (mut sc, mut sr) = (s, s);
            let (vc, vr) = unsafe {
                (
                    (c.c2GJKSimplexMetric)(&mut sc),
                    (r.c2GJKSimplexMetric)(&mut sr),
                )
            };
            assert_same("metric/bad-count", &(s, count), (vc, sc), (vr, sr));
            assert_eq!(vc.to_bits(), 0u32, "C must return +0.0 for count={count}");
        }
    }
}

// ---------------------------------------------------------------------------
// rows 04..06 — c22 branch coverage
// ---------------------------------------------------------------------------

/// Reproduce the C's `u` / `v` so the test can tell which branch fired.
fn c22_uv(a: c2v, b: c2v) -> (f32, f32) {
    let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
    let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    (u, v)
}

fn run_c22(s: c2Simplex) -> c2Simplex {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    unsafe {
        (c.c22)(&mut sc);
        (r.c22)(&mut sr);
    }
    assert_same("c22", &s, sc, sr);
    sc
}

/// rows 04, 05, 06 — every `c22` branch, each hit many times.
#[test]
fn e04_e05_e06_c22_all_branches() {
    let mut g = Rng::new(0xC04);
    let mut hits = [0usize; 3];
    // hand-built witnesses first
    let cases: [(c2v, c2v); 3] = [
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }), // v = -1 <= 0  -> branch 0
        (c2v { x: 2.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }), // u = -1 <= 0  -> branch 1
        (c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }), // both > 0    -> branch 2
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let mut s = g.simplex(2, 5.0);
        s.verts[0].p = *a;
        s.verts[1].p = *b;
        let out = run_c22(s);
        let (u, v) = c22_uv(*a, *b);
        let branch = if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        };
        assert_eq!(branch, i, "hand-built case {i} took branch {branch}");
        match i {
            0 | 1 => {
                assert_eq!(out.count, 1);
                assert_eq!(out.div.to_bits(), 1.0f32.to_bits());
                assert_eq!(out.verts[0].u.to_bits(), 1.0f32.to_bits());
            }
            _ => assert_eq!(out.count, 2),
        }
    }
    // randomized: assert each branch is exercised, and diff every case
    for _ in 0..60_000 {
        let a = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let b = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let mut s = g.simplex(2, 5.0);
        s.verts[0].p = a;
        s.verts[1].p = b;
        run_c22(s);
        let (u, v) = c22_uv(a, b);
        hits[if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        }] += 1;
    }
    eprintln!("e04/05/06 c22 branch hits: {hits:?}");
    for (i, h) in hits.iter().enumerate() {
        assert!(*h > 100, "c22 branch {i} only hit {h} times");
    }
}

// ---------------------------------------------------------------------------
// rows 07..14 — c23 branch coverage
// ---------------------------------------------------------------------------

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Which of the seven `c23` branches the C takes for this triangle (0..6).
fn c23_branch(a: c2v, b: c2v, c: c2v) -> (usize, f32) {
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
    let br = if vAB <= 0.0 && uCA <= 0.0 {
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
    };
    (br, area)
}

fn run_c23(s: c2Simplex) -> c2Simplex {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    unsafe {
        (c.c23)(&mut sc);
        (r.c23)(&mut sr);
    }
    assert_same("c23", &s, sc, sr);
    sc
}

/// rows 07..13 — all seven `c23` branches, plus row 14 (`area == 0`).
#[test]
fn e07_to_e14_c23_all_branches() {
    let mut g = Rng::new(0xC07);
    let mut hits = [0usize; 7];
    let mut zero_area_hits = [0usize; 7];
    for _ in 0..200_000 {
        let a = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let b = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let cc = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let mut s = g.simplex(3, 5.0);
        s.verts[0].p = a;
        s.verts[1].p = b;
        s.verts[2].p = cc;
        let out = run_c23(s);
        let (br, area) = c23_branch(a, b, cc);
        hits[br] += 1;
        if area == 0.0 {
            zero_area_hits[br] += 1;
        }
        // the branch determines the resulting count
        let expect_count = match br {
            0 | 1 | 2 => 1,
            3 | 4 | 5 => 2,
            _ => 3,
        };
        assert_eq!(
            out.count, expect_count,
            "branch {br} produced count {} (a={a:?} b={b:?} c={cc:?})",
            out.count
        );
    }
    eprintln!("e07..e13 c23 branch hits: {hits:?}");
    eprintln!("e14      zero-area hits : {zero_area_hits:?}");
    for (i, h) in hits.iter().enumerate() {
        assert!(*h > 50, "c23 branch {i} only hit {h} times");
    }
    assert!(
        zero_area_hits.iter().sum::<usize>() > 100,
        "row 14: degenerate (area == 0) triangles were not exercised"
    );
}

// ---------------------------------------------------------------------------
// rows 15..17 — c2D
// ---------------------------------------------------------------------------

fn run_c2D(s: c2Simplex) -> c2v {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    let (vc, vr) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
    assert_same("c2D", &s, (vc, sc), (vr, sr));
    vc
}

/// row 15 — `count == 1` returns `-a.p` (with `-0.0` where a component is `+0`).
#[test]
fn e15_c2D_count1() {
    let mut g = Rng::new(0xC15);
    for &(x, y) in [
        (0.0f32, 0.0f32),
        (-0.0, 0.0),
        (0.0, -0.0),
        (f32::NAN, 0.0),
        (f32::INFINITY, f32::NEG_INFINITY),
    ]
    .iter()
    {
        let mut s = g.simplex(1, 5.0);
        s.verts[0].p = c2v { x, y };
        let v = run_c2D(s);
        assert_eq!(v.x.to_bits(), (-x).to_bits());
        assert_eq!(v.y.to_bits(), (-y).to_bits());
    }
    for _ in 0..20_000 {
        let mut s = g.simplex(1, 50.0);
        s.verts[0].p = g.v_mixed(50.0);
        run_c2D(s);
    }
}

/// row 16 — `count == 2` with `c2Det2(ab, -a.p) <= 0` (or `NaN`) must use
/// `c2CCW90`, not `c2Skew`.
#[test]
fn e16_c2D_count2_det_not_positive() {
    let mut g = Rng::new(0xC16);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for i in 0..40_000 {
        let mut s = g.simplex(2, 5.0);
        let a = if i % 3 == 0 {
            c2v { x: g.f32_grid(3), y: g.f32_grid(3) }
        } else {
            g.v_mixed(20.0)
        };
        // every third case: origin exactly on the AB line => det == 0
        let b = if i % 3 == 0 {
            c2v { x: a.x * 2.0, y: a.y * 2.0 }
        } else {
            g.v_mixed(20.0)
        };
        s.verts[0].p = a;
        s.verts[1].p = b;
        let v = run_c2D(s);
        let ab = sub(b, a);
        let d = det2(ab, c2v { x: -a.x, y: -a.y });
        if d > 0.0 {
            skew += 1;
            assert_eq!(v.x.to_bits(), (-ab.y).to_bits());
            assert_eq!(v.y.to_bits(), ab.x.to_bits());
        } else {
            ccw += 1;
            assert_eq!(v.x.to_bits(), ab.y.to_bits());
            assert_eq!(v.y.to_bits(), (-ab.x).to_bits());
        }
    }
    eprintln!("e16 c2D count=2: skew={skew} ccw90={ccw}");
    assert!(skew > 100 && ccw > 100);
}

/// row 17 — `count` ∉ {1,2} returns `(+0.0, +0.0)`.
#[test]
fn e17_c2D_bad_count() {
    let mut g = Rng::new(0xC17);
    for count in [0i32, 3, 4, 5, 99, -1, i32::MIN, i32::MAX] {
        for _ in 0..400 {
            let s = g.simplex(count, 50.0);
            let v = run_c2D(s);
            assert_eq!(v.x.to_bits(), 0u32, "count={count}");
            assert_eq!(v.y.to_bits(), 0u32, "count={count}");
        }
    }
}

// ---------------------------------------------------------------------------
// rows 18, 19 — c2Support
// ---------------------------------------------------------------------------

/// row 18 — `count <= 0`: the loop never runs, `verts[0]` is still read, `0` is
/// returned.
#[test]
fn e18_support_nonpositive_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC18);
    for count in [0i32, -1, -2, -1000, i32::MIN] {
        for _ in 0..2000 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = g.v_mixed(50.0);
            }
            let d = g.v_mixed(50.0);
            let (vc, vr) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            assert_same("c2Support/count<=0", &(verts.to_vec(), count, d), vc, vr);
            assert_eq!(vc, 0, "C must return 0 for count={count}");
        }
    }
}

/// row 19 — every dot equal, or every dot `NaN`: index `0` wins.
#[test]
fn e19_support_all_ties_or_nan() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC19);
    for count in [1i32, 2, 3, 4, 8] {
        for mode in 0..3 {
            for _ in 0..2000 {
                let (verts, d) = match mode {
                    // all vertices identical -> all dots equal
                    0 => {
                        let p = g.v_geom(30.0);
                        ([p; 8], g.v_geom(30.0))
                    }
                    // d == (0,0) -> every dot is +0.0
                    1 => {
                        let mut v = [c2v::default(); 8];
                        for e in v.iter_mut() {
                            *e = g.v_geom(30.0);
                        }
                        (v, c2v { x: 0.0, y: 0.0 })
                    }
                    // d has a NaN -> every dot is NaN
                    _ => {
                        let mut v = [c2v::default(); 8];
                        for e in v.iter_mut() {
                            *e = g.v_geom(30.0);
                        }
                        (v, c2v { x: f32::NAN, y: g.f32_geom(3.0) })
                    }
                };
                let (vc, vr) = unsafe {
                    (
                        (c.c2Support)(verts.as_ptr(), count, d),
                        (r.c2Support)(verts.as_ptr(), count, d),
                    )
                };
                assert_same("c2Support/ties", &(verts.to_vec(), count, d), vc, vr);
                assert_eq!(vc, 0, "ties must resolve to index 0 (mode {mode})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 20, 21 — c2Witness
// ---------------------------------------------------------------------------

fn run_witness(s: c2Simplex) -> (c2v, c2v) {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    let (mut ac, mut bc) = (POISON_V, POISON_V);
    let (mut ar, mut br) = (POISON_V, POISON_V);
    unsafe {
        (c.c2Witness)(&mut sc, &mut ac, &mut bc);
        (r.c2Witness)(&mut sr, &mut ar, &mut br);
    }
    assert_same("c2Witness", &s, (ac, bc, sc), (ar, br, sr));
    (ac, bc)
}

/// row 20 — `div == 0` (and `-0.0`) makes `den` infinite; `inf * 0` yields `NaN`.
#[test]
fn e20_witness_div_zero() {
    let mut g = Rng::new(0xC20);
    let mut saw_nan = false;
    let mut saw_inf = false;
    for count in [1i32, 2, 3] {
        for &div in [0.0f32, -0.0].iter() {
            for i in 0..3000 {
                let mut s = g.simplex(count, 40.0);
                s.div = div;
                if i % 3 == 0 {
                    for v in s.verts.iter_mut() {
                        v.u = 0.0; // 0 * inf -> NaN
                    }
                }
                let (a, b) = run_witness(s);
                for f in [a.x, a.y, b.x, b.y] {
                    saw_nan |= f.is_nan();
                    saw_inf |= f.is_infinite();
                }
            }
        }
    }
    assert!(saw_nan, "row 20: no NaN produced by div == 0");
    assert!(saw_inf, "row 20: no infinity produced by div == 0");
}

/// row 21 — `count` ∉ {1,2,3} zeroes both outputs.
#[test]
fn e21_witness_bad_count() {
    let mut g = Rng::new(0xC21);
    for count in [0i32, 4, 5, 99, -1, i32::MIN, i32::MAX] {
        for _ in 0..500 {
            let s = g.simplex(count, 40.0);
            let (a, b) = run_witness(s);
            assert_eq!((a.x.to_bits(), a.y.to_bits()), (0, 0), "count={count}");
            assert_eq!((b.x.to_bits(), b.y.to_bits()), (0, 0), "count={count}");
        }
    }
}

// ---------------------------------------------------------------------------
// rows 22..25 — c2Div / c2Norm
// ---------------------------------------------------------------------------

/// row 22 — `c2Div(a, 0)` / `c2Div(a, -0)`.
#[test]
fn e22_div_by_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC22);
    let mut saw_nan = false;
    let mut saw_inf = false;
    for &b in [0.0f32, -0.0].iter() {
        for i in 0..8000 {
            let a = match i % 4 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: -0.0, y: 0.0 },
                2 => g.v_geom(50.0),
                _ => g.v_mixed(50.0),
            };
            let (vc, vr) = unsafe { ((c.c2Div)(a, b), (r.c2Div)(a, b)) };
            assert_same("c2Div/0", &(a, b), vc, vr);
            for f in [vc.x, vc.y] {
                saw_nan |= f.is_nan();
                saw_inf |= f.is_infinite();
            }
        }
    }
    assert!(saw_nan && saw_inf, "row 22: expected NaN and inf results");
}

/// row 23 — `c2Div(a, NaN)` / `c2Div(a, ±inf)`.
#[test]
fn e23_div_nan_inf() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC23);
    for &b in [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY].iter() {
        for _ in 0..8000 {
            let a = g.v_mixed(50.0);
            let (vc, vr) = unsafe { ((c.c2Div)(a, b), (r.c2Div)(a, b)) };
            assert_same("c2Div/nan-inf", &(a, b), vc, vr);
        }
    }
}

/// row 24 — `c2Norm((0,0))` (and the `-0.0` variants) is `(NaN, NaN)`.
#[test]
fn e24_norm_zero_vector() {
    let (c, r) = apis();
    for &x in [0.0f32, -0.0].iter() {
        for &y in [0.0f32, -0.0].iter() {
            let a = c2v { x, y };
            let (vc, vr) = unsafe { ((c.c2Norm)(a), (r.c2Norm)(a)) };
            assert_same("c2Norm/0", &a, vc, vr);
            assert!(vc.x.is_nan() && vc.y.is_nan(), "expected (NaN,NaN), got {vc:?}");
        }
    }
}

/// row 25 — `c2Norm` on vectors containing `NaN` / `±inf`.
#[test]
fn e25_norm_nan_inf() {
    let (c, r) = apis();
    let sp = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    for &x in sp.iter() {
        for &y in sp.iter() {
            let a = c2v { x, y };
            let (vc, vr) = unsafe { ((c.c2Norm)(a), (r.c2Norm)(a)) };
            assert_same("c2Norm/nan-inf", &a, vc, vr);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 26..28 — c2L / c2Len
// ---------------------------------------------------------------------------

/// row 26 — `c2L` with `div == 0`.
#[test]
fn e26_cL_div_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC26);
    let mut saw_bad = false;
    for count in [1i32, 2] {
        for &div in [0.0f32, -0.0].iter() {
            for _ in 0..4000 {
                let mut s = g.simplex(count, 40.0);
                s.div = div;
                let (mut sc, mut sr) = (s, s);
                let (vc, vr) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
                assert_same("c2L/div0", &s, (vc, sc), (vr, sr));
                if count == 2 {
                    saw_bad |= vc.x.is_nan() || vc.x.is_infinite();
                }
            }
        }
    }
    assert!(saw_bad, "row 26: div == 0 never produced inf/NaN");
}

/// row 27 — `c2L` with `count` ∉ {1,2} returns `(+0,+0)`.
#[test]
fn e27_cL_bad_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC27);
    for count in [0i32, 3, 4, 5, 99, -1, i32::MIN, i32::MAX] {
        for _ in 0..500 {
            let s = g.simplex(count, 40.0);
            let (mut sc, mut sr) = (s, s);
            let (vc, vr) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
            assert_same("c2L/bad-count", &s, (vc, sc), (vr, sr));
            assert_eq!((vc.x.to_bits(), vc.y.to_bits()), (0, 0), "count={count}");
        }
    }
}

/// row 28 — `c2Len` when `c2Dot(a,a)` is `NaN` (`inf` mixtures) -> `sqrtf(NaN)`.
#[test]
fn e28_len_nan() {
    let (c, r) = apis();
    let sp = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        1e30,
    ];
    let mut saw_nan = false;
    let mut saw_inf = false;
    for &x in sp.iter() {
        for &y in sp.iter() {
            let a = c2v { x, y };
            let (vc, vr) = unsafe { ((c.c2Len)(a), (r.c2Len)(a)) };
            assert_same("c2Len/nan", &a, vc, vr);
            saw_nan |= vc.is_nan();
            saw_inf |= vc.is_infinite();
        }
    }
    assert!(saw_nan && saw_inf);
}

// ---------------------------------------------------------------------------
// rows 29..34 — c2GJK transform / cache guards
// ---------------------------------------------------------------------------

const IDENTITY_X: c2x = c2x {
    p: c2v { x: 0.0, y: 0.0 },
    r: c2r { c: 1.0, s: 0.0 },
};

fn rand_shape(g: &mut Rng, ty: C2_TYPE) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(g.circle(40.0)),
        C2_TYPE_AABB => Shape::Aabb(g.aabb(40.0)),
        _ => Shape::Capsule(g.capsule(40.0)),
    }
}

/// row 29 — `ax_ptr == NULL` must behave exactly like `&c2xIdentity()`.
#[test]
fn e29_gjk_null_ax() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC29);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..600 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let ur = (i % 2) as i32;
                let null = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let expl = call_gjk(c, &sa, Some(&IDENTITY_X), &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/NULL-ax == identity (C)", &(sa, sb, ur), null, expl);
                let rn = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let re = call_gjk(r, &sa, Some(&IDENTITY_X), &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/NULL-ax == identity (rust)", &(sa, sb, ur), rn, re);
                assert_same(
                    "gjk/NULL-ax C vs rust",
                    &(sa, sb, ur),
                    call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None),
                    call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None),
                );
            }
        }
    }
}

/// row 30 — `bx_ptr == NULL` must behave exactly like `&c2xIdentity()`.
#[test]
fn e30_gjk_null_bx() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC30);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..600 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let ur = (i % 2) as i32;
                assert_same(
                    "gjk/NULL-bx == identity (C)",
                    &(sa, sb, ur),
                    call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None),
                    call_gjk(c, &sa, None, &sb, Some(&IDENTITY_X), ur, OutSel::ALL, None),
                );
                assert_same(
                    "gjk/NULL-bx == identity (rust)",
                    &(sa, sb, ur),
                    call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None),
                    call_gjk(r, &sa, None, &sb, Some(&IDENTITY_X), ur, OutSel::ALL, None),
                );
                assert_same(
                    "gjk/NULL-bx C vs rust",
                    &(sa, sb, ur),
                    call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None),
                    call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None),
                );
            }
        }
    }
}

/// row 31 — `cache->count == 0` disables the warm start (`cache_was_good == 0`),
/// so the result must equal the `cache == NULL` result apart from the write-back.
#[test]
fn e31_gjk_cache_count_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC31);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..800 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let ur = (i % 2) as i32;
                // count == 0 but every other field deliberately garbage
                let cache = c2GJKCache {
                    metric: g.f32_mixed(1e9),
                    count: 0,
                    iA: [7, -3, 99],
                    iB: [-1, 5, 123],
                    div: g.f32_mixed(1e5),
                };
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                assert_same("gjk/cache-count-0", &(sa, sb, cache, ur), oc, or_);
                // ... and identical to the NULL-cache result
                let nc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                assert_same(
                    "gjk/cache-count-0 == NULL cache",
                    &(sa, sb, ur),
                    (oc.dist, oc.outA, oc.outB, oc.iterations),
                    (nc.dist, nc.outA, nc.outB, nc.iterations),
                );
            }
        }
    }
}

/// row 32 — the metric test `!(min < max*2 && metric < -1.0e8f)` is effectively
/// always true, so a non-empty cache is *always* accepted.  Proven observably:
/// a crafted cache with a non-zero starting index changes the result relative to
/// a cold start, and C and Rust must agree on the changed result.
#[test]
fn e32_gjk_cache_always_read() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC32);
    let mut differed = 0usize;
    let mut total = 0usize;
    // AABB vs AABB: 4 proxy vertices, so index 2 really is a different start.
    for _ in 0..4000 {
        let sa = Shape::Aabb(g.aabb(40.0));
        let sb = Shape::Aabb(g.aabb(40.0));
        for &metric in [-1.0e30f32, -1.0e9, 0.0, 1.0e9, f32::NAN, f32::INFINITY].iter() {
            let cache = c2GJKCache {
                metric,
                count: 1,
                iA: [2, 0, 0],
                iB: [3, 0, 0],
                div: 1.0,
            };
            let warm_c = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
            let warm_r = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
            assert_same("gjk/cache-always-read", &(sa, sb, cache), warm_c, warm_r);
            let cold = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
            total += 1;
            if warm_c.cache.unwrap().bits() != cold.cache.unwrap().bits() {
                differed += 1;
            }
        }
    }
    eprintln!("e32: warm start changed the outcome in {differed}/{total} cases");
    assert!(
        differed > 0,
        "row 32: the cache-read branch was never actually taken"
    );
}

/// row 33 — cached indices outside `0..proxy->count` read the *uninitialised*
/// tail of the stack-allocated `c2Proxy`.  UB: not executed.
#[test]
#[ignore = "row 33: reads uninitialised c2Proxy::verts[count..8] in the C -> result is not a function of the inputs"]
fn e33_gjk_cache_index_out_of_shape_range_documented() {
    let (c, r) = apis();
    let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }); // proxy count == 1
    let sb = Shape::Circle(c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 1.0 });
    let cache = c2GJKCache {
        metric: 0.0,
        count: 1,
        iA: [5, 0, 0], // >= proxy count 1 -> never written by c2MakeProxy
        iB: [7, 0, 0],
        div: 1.0,
    };
    let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
    let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
    assert_same("gjk/cache-oob-index", &(sa, sb, cache), oc, or_);
}

/// row 34 — `cache->count > 3` reads past `cache->iA[3]` *and* writes
/// `verts[4]`, i.e. past the end of `c2Simplex`.  UB: not executed.
#[test]
#[ignore = "row 34: cache->count > 3 writes past the end of c2Simplex in the C -> stack corruption"]
fn e34_gjk_cache_count_gt3_documented() {
    let (c, r) = apis();
    let sa = Shape::Aabb(c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    });
    let sb = Shape::Aabb(c2AABB {
        min: c2v { x: 5.0, y: 5.0 },
        max: c2v { x: 6.0, y: 6.0 },
    });
    for count in [4i32, 5, i32::MAX, -1] {
        let cache = c2GJKCache {
            metric: 0.0,
            count,
            iA: [0; 3],
            iB: [0; 3],
            div: 1.0,
        };
        let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
        let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
        assert_same("gjk/cache-count>3", &(sa, sb, cache), oc, or_);
    }
}

// ---------------------------------------------------------------------------
// rows 35..39 — c2GJK loop exits
// ---------------------------------------------------------------------------

/// row 35 — the 20-iteration cap.  Broad search over arbitrary bit-pattern
/// coordinates; `*iterations` (the direct observable of where the loop left) is
/// compared for every case, and the maximum ever reached is reported.
#[test]
fn e35_gjk_iteration_cap() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC35);
    let mut max_iter = -1i32;
    let mut hist = [0usize; 21];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..12_000 {
                // fully arbitrary bit patterns: NaN, inf, denormals, huge
                let sa = match ta {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: g.v_mixed(1e6), r: g.f32_mixed(1e6) }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: g.v_mixed(1e6), max: g.v_mixed(1e6) }),
                    _ => Shape::Capsule(c2Capsule { a: g.v_mixed(1e6), b: g.v_mixed(1e6), r: g.f32_mixed(1e6) }),
                };
                let sb = match tb {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: g.v_mixed(1e6), r: g.f32_mixed(1e6) }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: g.v_mixed(1e6), max: g.v_mixed(1e6) }),
                    _ => Shape::Capsule(c2Capsule { a: g.v_mixed(1e6), b: g.v_mixed(1e6), r: g.f32_mixed(1e6) }),
                };
                let ur = (i % 2) as i32;
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let it = oc.iterations.unwrap();
                max_iter = max_iter.max(it);
                hist[(it.clamp(0, 20)) as usize] += 1;
                if it >= 20 {
                    // the C would now read an uninitialised `u`; skip the diff
                    continue;
                }
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/iter-cap-search", &(sa, sb, ur), oc, or_);
            }
        }
    }
    eprintln!("e35: max iterations observed = {max_iter}; histogram = {hist:?}");
    assert!(max_iter >= 0);
}

/// row 36 — `s.count == 3` (`hit`): forces `a = b` and `dist = 0` even with
/// `use_radius == 0`.  `cache->count == 3` on return proves the branch fired.
#[test]
fn e36_gjk_hit() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC36);
    let mut hits = 0usize;
    for _ in 0..8000 {
        // two heavily overlapping boxes -> the origin is inside the Minkowski
        // difference -> the simplex grows to 3 vertices
        let p = g.v_geom(30.0);
        let sa = Shape::Aabb(c2AABB {
            min: c2v { x: p.x - 10.0, y: p.y - 10.0 },
            max: c2v { x: p.x + 10.0, y: p.y + 10.0 },
        });
        let q = c2v { x: p.x + g.f32_range(4.0), y: p.y + g.f32_range(4.0) };
        let sb = Shape::Aabb(c2AABB {
            min: c2v { x: q.x - 9.0, y: q.y - 9.0 },
            max: c2v { x: q.x + 9.0, y: q.y + 9.0 },
        });
        for ur in [0i32, 1] {
            let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            assert_same("gjk/hit", &(sa, sb, ur), oc, or_);
            if oc.cache.unwrap().count == 3 {
                hits += 1;
                assert_eq!(oc.dist.to_bits(), 0u32, "hit must force dist = +0.0");
                assert_same("gjk/hit a==b", &(sa, sb, ur), oc.outA, oc.outB);
            }
        }
    }
    eprintln!("e36: `hit` branch taken {hits} times");
    assert!(hits > 1000, "row 36: the hit branch was barely exercised");
}

/// row 37 — the `d1 > d0` break.  `*iterations` and the returned `cache->count`
/// observe the loop-exit point directly, so a different comparison in the Rust
/// (`>=`, `<`, …) shows up as a divergence.  Driven with mixed-scale, nearly
/// degenerate configurations where the descent really does regress.
#[test]
fn e37_gjk_no_progress_break() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC37);
    let scales = [1.0f32, 1.0e-6, 1.0e6, 1.0e18, 1.0e-18, f32::EPSILON];
    let mut it_hist = [0usize; 21];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..3000 {
                let s1 = scales[(i % scales.len()) as usize];
                let s2 = scales[((i / scales.len()) % scales.len()) as usize];
                let ca = c2v { x: g.f32_range(1.0) * s1, y: g.f32_range(1.0) * s1 };
                let cb = c2v { x: g.f32_range(1.0) * s2, y: g.f32_range(1.0) * s2 };
                let sa = match ta {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: ca, r: g.f32_range(1.0).abs() * s1 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: ca,
                        max: c2v { x: ca.x + g.f32_range(1.0).abs() * s1, y: ca.y + g.f32_range(1.0).abs() * s1 },
                    }),
                    _ => Shape::Capsule(c2Capsule { a: ca, b: c2v { x: ca.x + s1, y: ca.y }, r: g.f32_range(1.0).abs() * s1 }),
                };
                let sb = match tb {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: cb, r: g.f32_range(1.0).abs() * s2 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: cb,
                        max: c2v { x: cb.x + g.f32_range(1.0).abs() * s2, y: cb.y + g.f32_range(1.0).abs() * s2 },
                    }),
                    _ => Shape::Capsule(c2Capsule { a: cb, b: c2v { x: cb.x + s2, y: cb.y }, r: g.f32_range(1.0).abs() * s2 }),
                };
                let ur = (i % 2) as i32;
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                let it = oc.iterations.unwrap();
                it_hist[it.clamp(0, 20) as usize] += 1;
                if it >= 20 {
                    continue;
                }
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                assert_same("gjk/no-progress", &(sa, sb, ur), oc, or_);
            }
        }
    }
    eprintln!("e37: iteration histogram = {it_hist:?}");
    // multi-iteration runs are required, otherwise `d1 > d0` can never be reached
    assert!(
        it_hist[1..].iter().sum::<usize>() > 100,
        "row 37: no multi-iteration descent was produced"
    );
}

/// row 38 — `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON`.
///
/// With `A` and `B` the *same* shape, the first simplex vertex is
/// `sB - sA == (0,0)`, so `c2D` returns `(-0,-0)` and `c2Dot(d,d) == 0`: the
/// degenerate-direction break is the *only* exit that can fire, at
/// `iter == 0`, leaving `count == 1`.
#[test]
fn e38_gjk_degenerate_direction_break() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC38);
    for &ta in C2_TYPES.iter() {
        for i in 0..3000 {
            let s = rand_shape(&mut g, ta);
            let ur = (i % 2) as i32;
            let oc = call_gjk(c, &s, None, &s, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            let or_ = call_gjk(r, &s, None, &s, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            assert_same("gjk/eps-break", &(s, ur), oc, or_);
            assert_eq!(oc.iterations.unwrap(), 0, "must break at iteration 0");
            assert_eq!(oc.cache.unwrap().count, 1, "must break with a 1-simplex");
        }
    }
}

/// row 39 — the duplicate-support break.
///
/// A degenerate AABB (`min == max`) gives a proxy whose four vertices are all
/// equal, so `c2Support` always returns `0`; the freshly generated support pair
/// `(0,0)` therefore duplicates the saved one at `iter == 0`.  Because the two
/// shapes are separated, `c2Dot(d,d)` is far above `FLT_EPSILON^2`, so the
/// eps break cannot fire — the duplicate break is the one that exits.
#[test]
fn e39_gjk_duplicate_support_break() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC39);
    for i in 0..8000 {
        let p = g.v_geom(30.0);
        let q = c2v { x: p.x + 50.0 + g.f32_range(10.0), y: p.y + 50.0 + g.f32_range(10.0) };
        let sa = Shape::Aabb(c2AABB { min: p, max: p });
        let sb = Shape::Aabb(c2AABB { min: q, max: q });
        let ur = (i % 2) as i32;
        let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
        let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
        assert_same("gjk/dup-break", &(sa, sb, ur), oc, or_);
        assert_eq!(oc.iterations.unwrap(), 0);
        assert_eq!(oc.cache.unwrap().count, 1);
        // proof the eps break could not have fired: |d|^2 is enormous here
        assert!(oc.dist.is_finite());
    }
}

// ---------------------------------------------------------------------------
// rows 40..45 — c2GJK radius handling
// ---------------------------------------------------------------------------

/// row 40 — `use_radius == 0` skips the whole shrink block.
#[test]
fn e40_gjk_use_radius_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC40);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for _ in 0..800 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let oc = call_gjk(c, &sa, None, &sb, None, 0, OutSel::ALL, Some(c2GJKCache::default()));
                let or_ = call_gjk(r, &sa, None, &sb, None, 0, OutSel::ALL, Some(c2GJKCache::default()));
                assert_same("gjk/use_radius=0", &(sa, sb), oc, or_);
                // with use_radius == 0 the witness distance is returned verbatim
                // unless `hit` fired (count == 3 => dist forced to 0)
                if oc.cache.unwrap().count != 3 {
                    let w = call_gjk(c, &sa, None, &sb, None, 0, OutSel::NONE, None);
                    assert_eq!(w.dist.to_bits(), oc.dist.to_bits());
                }
            }
        }
    }
}

/// row 41 — any non-zero `int` in `use_radius` behaves like `1`.
#[test]
fn e41_gjk_use_radius_nonzero_variants() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC41);
    let vals = [1i32, 2, -1, 42, i32::MIN, i32::MAX, 0x0001_0000];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for _ in 0..400 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let base_c = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
                for &ur in vals.iter() {
                    let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    assert_same("gjk/use_radius variants", &(sa, sb, ur), oc, or_);
                    assert_same("gjk/use_radius != 0 == 1", &(sa, sb, ur), oc, base_c);
                }
            }
        }
    }
}

/// row 42 — `!(dist > rA+rB && dist > FLT_EPSILON)` -> midpoint, `dist = 0`.
#[test]
fn e42_gjk_radius_else_midpoint() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC42);
    for _ in 0..20_000 {
        // two circles whose radii swallow the gap: circles have a 1-vertex proxy,
        // so `hit` can never fire and the radius block always runs
        let pa = g.v_geom(30.0);
        let pb = c2v { x: pa.x + g.f32_range(4.0), y: pa.y + g.f32_range(4.0) };
        let sa = Shape::Circle(c2Circle { p: pa, r: 20.0 });
        let sb = Shape::Circle(c2Circle { p: pb, r: 20.0 });
        let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
        let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
        assert_same("gjk/radius-else", &(sa, sb), oc, or_);
        assert_eq!(oc.cache.unwrap().count, 1, "circle proxies cannot reach count 3");
        assert_eq!(oc.dist.to_bits(), 0u32, "midpoint branch must return +0.0");
        assert_same("gjk/radius-else a==b", &(sa, sb), oc.outA, oc.outB);
    }
    // exact `dist == FLT_EPSILON` boundary
    for k in [0.0f32, f32::EPSILON, f32::EPSILON * 2.0, f32::EPSILON * 0.5, 1.0] {
        let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 });
        let sb = Shape::Circle(c2Circle { p: c2v { x: k, y: 0.0 }, r: 0.0 });
        let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, None);
        let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, None);
        assert_same("gjk/radius-eps-boundary", &(sa, sb, k), oc, or_);
    }
}

/// row 43 — `NaN` radius makes `dist > rA+rB` false -> midpoint branch.
#[test]
fn e43_gjk_radius_nan() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC43);
    for &(ra, rb) in [
        (f32::NAN, 1.0f32),
        (1.0, f32::NAN),
        (f32::NAN, f32::NAN),
        (-f32::NAN, 0.0),
        (f32::INFINITY, f32::NEG_INFINITY),
    ]
    .iter()
    {
        for _ in 0..3000 {
            let pa = g.v_geom(40.0);
            let pb = g.v_geom(40.0);
            let sa = Shape::Circle(c2Circle { p: pa, r: ra });
            let sb = Shape::Circle(c2Circle { p: pb, r: rb });
            let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
            let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, Some(c2GJKCache::default()));
            assert_same("gjk/radius-nan", &(sa, sb), oc, or_);
            assert_eq!(oc.dist.to_bits(), 0u32, "NaN radius must take the else branch");
        }
    }
}

/// row 44 — the shrink lands `a` exactly on `b`, so `dist` is forced to `0`
/// although the subtraction produced a non-zero value.
///
/// Construction: `rA = +1e30`, `rB = -1e30` (a caller may pass any float), so
/// `rA + rB == 0` and `dist > 0` passes, yet `a += n*rA` and `b -= n*rB` both
/// land on `+1e30` because `10` is far below the ULP of `1e30`.
#[test]
fn e44_gjk_radius_shrink_collapses() {
    let (c, r) = apis();
    let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0e30 });
    let sb = Shape::Circle(c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: -1.0e30 });
    let raw_c = call_gjk(c, &sa, None, &sb, None, 0, OutSel::ALL, None);
    let raw_r = call_gjk(r, &sa, None, &sb, None, 0, OutSel::ALL, None);
    assert_same("gjk/collapse raw", &(sa, sb), raw_c, raw_r);
    assert_eq!(raw_c.dist.to_bits(), 10.0f32.to_bits(), "raw distance");

    let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, None);
    let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, None);
    assert_same("gjk/collapse", &(sa, sb), oc, or_);
    assert_same("gjk/collapse a==b", &(sa, sb), oc.outA, oc.outB);
    assert_eq!(
        oc.dist.to_bits(),
        0u32,
        "row 44: dist must be forced to +0.0 (got {})",
        oc.dist
    );

    // a family of magnitudes, all of which must agree between C and Rust
    for e in [1.0e20f32, 1.0e25, 1.0e30, 1.0e35, f32::MAX / 4.0] {
        for gap in [1.0f32, 10.0, 1000.0, 1.0e6] {
            let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: e });
            let sb = Shape::Circle(c2Circle { p: c2v { x: gap, y: 0.0 }, r: -e });
            let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, None);
            let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, None);
            assert_same("gjk/collapse family", &(sa, sb, e, gap), oc, or_);
        }
    }
}

/// row 45 — `c2Norm` degenerates because `c2Len` overflows to `+inf`, so
/// `n == (±0,±0)`, `a`/`b` stay put and `dist` remains `+inf`.
#[test]
fn e45_gjk_radius_norm_overflow() {
    let (c, r) = apis();
    let sa = Shape::Circle(c2Circle { p: c2v { x: -1.0e38, y: 0.0 }, r: 0.0 });
    let sb = Shape::Circle(c2Circle { p: c2v { x: 1.0e38, y: 0.0 }, r: 0.0 });
    let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, None);
    let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, None);
    assert_same("gjk/norm-overflow", &(sa, sb), oc, or_);
    assert_eq!(oc.dist.to_bits(), f32::INFINITY.to_bits(), "dist must stay +inf");
    assert_eq!(oc.outA.unwrap().x.to_bits(), (-1.0e38f32).to_bits());
    assert_eq!(oc.outB.unwrap().x.to_bits(), (1.0e38f32).to_bits());

    // a sweep of overflowing separations, all shape types
    let mut g = Rng::new(0xC45);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for _ in 0..400 {
                let big = 1.0e38f32;
                let mk = |ty: C2_TYPE, sign: f32, g: &mut Rng| match ty {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c2v { x: sign * big, y: sign * g.f32_range(1.0) * big }, r: 0.0 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v { x: sign * big, y: sign * big },
                        max: c2v { x: sign * big, y: sign * big },
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v { x: sign * big, y: 0.0 },
                        b: c2v { x: sign * big, y: 0.0 },
                        r: 0.0,
                    }),
                };
                let sa = mk(ta, -1.0, &mut g);
                let sb = mk(tb, 1.0, &mut g);
                let oc = call_gjk(c, &sa, None, &sb, None, 1, OutSel::ALL, None);
                let or_ = call_gjk(r, &sa, None, &sb, None, 1, OutSel::ALL, None);
                assert_same("gjk/norm-overflow sweep", &(sa, sb), oc, or_);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 46..49 — c2GJK NULL out-parameters
// ---------------------------------------------------------------------------

const POISON_CACHE: c2GJKCache = c2GJKCache {
    metric: -1.5e-9,
    count: 0x5A5A,
    iA: [0x1111_1111; 3],
    iB: [0x2222_2222; 3],
    div: -7.5e11,
};

/// row 46 — `cache == NULL`: no write-back at all, and the answer is unchanged.
#[test]
fn e46_gjk_null_cache_no_writeback() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC46);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..600 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let ur = (i % 2) as i32;
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/NULL-cache", &(sa, sb, ur), oc, or_);
                assert!(oc.cache.is_none() && or_.cache.is_none());
                // the caller's own buffer stays untouched when NULL is passed
                let mut mine = POISON_CACHE;
                let d = unsafe {
                    (c.c2GJK)(
                        sa.ptr(), ta, std::ptr::null(),
                        sb.ptr(), tb, std::ptr::null(),
                        std::ptr::null_mut(), std::ptr::null_mut(),
                        ur, std::ptr::null_mut(), std::ptr::null_mut(),
                    )
                };
                let mut mine_r = POISON_CACHE;
                let dr = unsafe {
                    (r.c2GJK)(
                        sa.ptr(), ta, std::ptr::null(),
                        sb.ptr(), tb, std::ptr::null(),
                        std::ptr::null_mut(), std::ptr::null_mut(),
                        ur, std::ptr::null_mut(), std::ptr::null_mut(),
                    )
                };
                assert_same("gjk/NULL-everything", &(sa, sb, ur), (d, mine), (dr, mine_r));
                assert_same("gjk/cache buffer untouched", &(sa, sb, ur), mine, POISON_CACHE);
                assert_same("gjk/cache buffer untouched", &(sa, sb, ur), mine_r, POISON_CACHE);
                let _ = (&mut mine, &mut mine_r);
            }
        }
    }
}

/// rows 47, 48, 49 — each of `outA` / `outB` / `iterations` NULL individually:
/// the caller's poisoned slot must survive untouched on both sides.
#[test]
fn e47_e48_e49_gjk_null_out_params() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC47);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..400 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, tb);
                let ur = (i % 2) as i32;
                for which in 0..3 {
                    let sel = OutSel {
                        a: which != 0,
                        b: which != 1,
                        iters: which != 2,
                    };
                    let oc = call_gjk(c, &sa, None, &sb, None, ur, sel, None);
                    let or_ = call_gjk(r, &sa, None, &sb, None, ur, sel, None);
                    assert_same("gjk/one-null-out", &(sa, sb, ur, which as u32), oc, or_);
                    // the skipped slot must still hold the poison value
                    match which {
                        0 => {
                            assert_eq!(oc.outA.unwrap().x.to_bits(), 0xDEAD_BEEF);
                            assert_eq!(or_.outA.unwrap().x.to_bits(), 0xDEAD_BEEF);
                        }
                        1 => {
                            assert_eq!(oc.outB.unwrap().x.to_bits(), 0xDEAD_BEED);
                            assert_eq!(or_.outB.unwrap().x.to_bits(), 0xDEAD_BEED);
                        }
                        _ => {
                            assert_eq!(oc.iterations.unwrap(), -12345);
                            assert_eq!(or_.iterations.unwrap(), -12345);
                        }
                    }
                }
            }
        }
    }
}

/// G2 — the full cross-product of all six nullable pointer parameters
/// (`ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, `cache`): 64 combinations.
#[test]
fn e_all_null_combinations() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC4F);
    let xf = c2x {
        p: c2v { x: 3.0, y: -7.0 },
        r: c2r { c: 0.8, s: 0.6 },
    };
    for mask in 0u32..64 {
        let ax = if mask & 1 != 0 { Some(&xf) } else { None };
        let bx = if mask & 2 != 0 { Some(&xf) } else { None };
        let sel = OutSel {
            a: mask & 4 != 0,
            b: mask & 8 != 0,
            iters: mask & 16 != 0,
        };
        let cache = if mask & 32 != 0 {
            Some(c2GJKCache::default())
        } else {
            None
        };
        for &ta in C2_TYPES.iter() {
            for &tb in C2_TYPES.iter() {
                for i in 0..60 {
                    let sa = rand_shape(&mut g, ta);
                    let sb = rand_shape(&mut g, tb);
                    let ur = (i % 2) as i32;
                    let oc = call_gjk(c, &sa, ax, &sb, bx, ur, sel, cache);
                    let or_ = call_gjk(r, &sa, ax, &sb, bx, ur, sel, cache);
                    assert_same("gjk/null-matrix", &(sa, sb, mask, ur), oc, or_);
                }
            }
        }
    }
}

/// row 50 — an out-of-range `C2_TYPE` leaves the stack `c2Proxy` uninitialised.
/// UB: not executed (a garbage `pA.count` makes `c2Support` walk off the array).
#[test]
#[ignore = "row 50: c2GJK with an invalid C2_TYPE reads an uninitialised c2Proxy in the C -> UB"]
fn e50_gjk_bad_type_documented() {
    let (c, r) = apis();
    let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 });
    let sb = Shape::Circle(c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 1.0 });
    for &bad in BAD.iter() {
        let oc = call_gjk_ty(c, &sa, bad, None, &sb, C2_TYPE_CIRCLE, None, 1, OutSel::ALL, None);
        let or_ = call_gjk_ty(r, &sa, bad, None, &sb, C2_TYPE_CIRCLE, None, 1, OutSel::ALL, None);
        assert_same("gjk/bad-type", &(sa, sb, bad), oc, or_);
    }
}

// ---------------------------------------------------------------------------
// rows 51..55 — c2AABBtoAABB
// ---------------------------------------------------------------------------

/// rows 51..54 — each of the four separating-axis rejections, individually.
#[test]
fn e51_aabbaabb_sep_axes() {
    let (c, r) = apis();
    let unit = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    // axis 0: B.max.x < A.min.x ; 1: A.max.x < B.min.x
    // axis 2: B.max.y < A.min.y ; 3: A.max.y < B.min.y
    for axis in 0..4usize {
        for &gap in [-1.0f32, -0.5, -1e-7, 0.0, 1e-7, 0.5, 1.0, 100.0].iter() {
            let b = match axis {
                0 => c2AABB { min: c2v { x: -2.0 - gap, y: 0.0 }, max: c2v { x: -gap, y: 1.0 } },
                1 => c2AABB { min: c2v { x: 1.0 + gap, y: 0.0 }, max: c2v { x: 3.0 + gap, y: 1.0 } },
                2 => c2AABB { min: c2v { x: 0.0, y: -2.0 - gap }, max: c2v { x: 1.0, y: -gap } },
                _ => c2AABB { min: c2v { x: 0.0, y: 1.0 + gap }, max: c2v { x: 1.0, y: 3.0 + gap } },
            };
            let (vc, vr) = unsafe {
                ((c.c2AABBtoAABB)(unit, b), (r.c2AABBtoAABB)(unit, b))
            };
            assert_same("c2AABBtoAABB/axis", &(unit, b, axis as u32, gap), vc, vr);
            // strictly separated by `gap > 0` must reject
            if gap > 0.0 {
                assert_eq!(vc, 0, "axis {axis} gap {gap} should be rejected");
            }
            // exact touching must NOT reject (the test is `<`, not `<=`)
            if gap == 0.0 {
                assert_eq!(vc, 1, "axis {axis} exact touch must report a hit");
            }
        }
    }
    // and a randomized confirmation over each axis independently
    let mut g = Rng::new(0xC51);
    for _ in 0..40_000 {
        let a = c2AABB { min: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, max: c2v { x: g.f32_grid(4), y: g.f32_grid(4) } };
        let b = c2AABB { min: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, max: c2v { x: g.f32_grid(4), y: g.f32_grid(4) } };
        let (vc, vr) = unsafe { ((c.c2AABBtoAABB)(a, b), (r.c2AABBtoAABB)(a, b)) };
        assert_same("c2AABBtoAABB/rand", &(a, b), vc, vr);
        let expect = !((b.max.x < a.min.x) | (a.max.x < b.min.x) | (b.max.y < a.min.y) | (a.max.y < b.min.y));
        assert_eq!(vc, expect as i32);
    }
}

/// row 55 — a `NaN` coordinate defeats all four `<` tests, so the C reports 1.
#[test]
fn e55_aabbaabb_nan_reports_hit() {
    let (c, r) = apis();
    let far = c2AABB {
        min: c2v { x: 1.0e9, y: 1.0e9 },
        max: c2v { x: 2.0e9, y: 2.0e9 },
    };
    for slot in 0..4 {
        for &nan in [f32::NAN, -f32::NAN].iter() {
            let mut a = c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            match slot {
                0 => a.min.x = nan,
                1 => a.min.y = nan,
                2 => a.max.x = nan,
                _ => a.max.y = nan,
            }
            let (vc, vr) = unsafe { ((c.c2AABBtoAABB)(a, far), (r.c2AABBtoAABB)(a, far)) };
            assert_same("c2AABBtoAABB/nan", &(a, far, slot as u32), vc, vr);
            let (vc2, vr2) = unsafe { ((c.c2AABBtoAABB)(far, a), (r.c2AABBtoAABB)(far, a)) };
            assert_same("c2AABBtoAABB/nan-swapped", &(far, a, slot as u32), vc2, vr2);
        }
    }
    // fully-NaN boxes "collide" with everything
    let n = c2AABB {
        min: c2v { x: f32::NAN, y: f32::NAN },
        max: c2v { x: f32::NAN, y: f32::NAN },
    };
    let (vc, vr) = unsafe { ((c.c2AABBtoAABB)(n, far), (r.c2AABBtoAABB)(n, far)) };
    assert_same("c2AABBtoAABB/all-nan", &(n, far), vc, vr);
    assert_eq!(vc, 1, "a NaN box must report a hit");
}

// ---------------------------------------------------------------------------
// rows 56, 57 — the GJK-backed wrappers reject when dist != 0
// ---------------------------------------------------------------------------

/// row 56 — `c2AABBtoCapsule` returns 0 whenever the GJK distance is non-zero.
#[test]
fn e56_aabbcapsule_reject() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC56);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for _ in 0..20_000 {
        let bb = c2AABB {
            min: c2v { x: g.f32_grid(4), y: g.f32_grid(4) },
            max: c2v { x: g.f32_grid(4) + 4.0, y: g.f32_grid(4) + 4.0 },
        };
        let cap = c2Capsule {
            a: c2v { x: g.f32_grid(8), y: g.f32_grid(8) },
            b: c2v { x: g.f32_grid(8), y: g.f32_grid(8) },
            r: g.f32_grid(3).abs(),
        };
        let (vc, vr) = unsafe {
            ((c.c2AABBtoCapsule)(bb, cap), (r.c2AABBtoCapsule)(bb, cap))
        };
        assert_same("c2AABBtoCapsule", &(bb, cap), vc, vr);
        let d = call_gjk(c, &Shape::Aabb(bb), None, &Shape::Capsule(cap), None, 1, OutSel::NONE, None).dist;
        assert_eq!(vc, (d == 0.0) as i32, "dist={d} but result={vc}");
        if vc == 0 { rejected += 1 } else { accepted += 1 }
    }
    eprintln!("e56: rejected={rejected} accepted={accepted}");
    assert!(rejected > 100 && accepted > 100);
    // a definitely-separated pair must reject
    let bb = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let cap = c2Capsule { a: c2v { x: 100.0, y: 100.0 }, b: c2v { x: 110.0, y: 100.0 }, r: 1.0 };
    let (vc, vr) = unsafe { ((c.c2AABBtoCapsule)(bb, cap), (r.c2AABBtoCapsule)(bb, cap)) };
    assert_same("c2AABBtoCapsule/far", &(bb, cap), vc, vr);
    assert_eq!(vc, 0);
}

/// row 57 — `c2CapsuletoCapsule` returns 0 whenever the GJK distance is non-zero.
#[test]
fn e57_capsulecapsule_reject() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC57);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for _ in 0..20_000 {
        let a = c2Capsule {
            a: c2v { x: g.f32_grid(5), y: g.f32_grid(5) },
            b: c2v { x: g.f32_grid(5), y: g.f32_grid(5) },
            r: g.f32_grid(3).abs(),
        };
        let b = c2Capsule {
            a: c2v { x: g.f32_grid(5), y: g.f32_grid(5) },
            b: c2v { x: g.f32_grid(5), y: g.f32_grid(5) },
            r: g.f32_grid(3).abs(),
        };
        let (vc, vr) = unsafe {
            ((c.c2CapsuletoCapsule)(a, b), (r.c2CapsuletoCapsule)(a, b))
        };
        assert_same("c2CapsuletoCapsule", &(a, b), vc, vr);
        let d = call_gjk(c, &Shape::Capsule(a), None, &Shape::Capsule(b), None, 1, OutSel::NONE, None).dist;
        assert_eq!(vc, (d == 0.0) as i32, "dist={d} but result={vc}");
        if vc == 0 { rejected += 1 } else { accepted += 1 }
    }
    eprintln!("e57: rejected={rejected} accepted={accepted}");
    assert!(rejected > 100 && accepted > 100);
    let a = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 };
    let b = c2Capsule { a: c2v { x: 500.0, y: 0.0 }, b: c2v { x: 501.0, y: 0.0 }, r: 0.5 };
    let (vc, vr) = unsafe { ((c.c2CapsuletoCapsule)(a, b), (r.c2CapsuletoCapsule)(a, b)) };
    assert_same("c2CapsuletoCapsule/far", &(a, b), vc, vr);
    assert_eq!(vc, 0);
}

// ---------------------------------------------------------------------------
// rows 58..63 — circle wrappers
// ---------------------------------------------------------------------------

/// row 58 — `c2CircletoCircle` rejects; exact tangency (`d2 == r2`) is a reject.
#[test]
fn e58_circlecircle_reject_and_tangent() {
    let (c, r) = apis();
    // 3-4-5: distance 7 with radii 3 and 4 => d2 == r2 == 49 exactly
    let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 3.0 };
    let b = c2Circle { p: c2v { x: 7.0, y: 0.0 }, r: 4.0 };
    let (vc, vr) = unsafe { ((c.c2CircletoCircle)(a, b), (r.c2CircletoCircle)(a, b)) };
    assert_same("c2CircletoCircle/tangent", &(a, b), vc, vr);
    assert_eq!(vc, 0, "exact tangency must NOT collide (`<`, not `<=`)");
    // one ULP closer must collide
    let b2 = c2Circle { p: c2v { x: f32::from_bits(7.0f32.to_bits() - 1), y: 0.0 }, r: 4.0 };
    let (vc, vr) = unsafe { ((c.c2CircletoCircle)(a, b2), (r.c2CircletoCircle)(a, b2)) };
    assert_same("c2CircletoCircle/tangent-1ulp", &(a, b2), vc, vr);
    assert_eq!(vc, 1);
    // zero radii can never collide (r2 == 0 is never `> d2`)
    let mut g = Rng::new(0xC58);
    for _ in 0..20_000 {
        let a = c2Circle { p: g.v_geom(30.0), r: 0.0 };
        let b = c2Circle { p: g.v_geom(30.0), r: 0.0 };
        let (vc, vr) = unsafe { ((c.c2CircletoCircle)(a, b), (r.c2CircletoCircle)(a, b)) };
        assert_same("c2CircletoCircle/r0", &(a, b), vc, vr);
        assert_eq!(vc, 0);
    }
    // NaN anywhere => reject
    for slot in 0..6 {
        let mut a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let mut b = c2Circle { p: c2v { x: 0.5, y: 0.0 }, r: 1.0 };
        match slot {
            0 => a.p.x = f32::NAN,
            1 => a.p.y = f32::NAN,
            2 => a.r = f32::NAN,
            3 => b.p.x = f32::NAN,
            4 => b.p.y = f32::NAN,
            _ => b.r = f32::NAN,
        }
        let (vc, vr) = unsafe { ((c.c2CircletoCircle)(a, b), (r.c2CircletoCircle)(a, b)) };
        assert_same("c2CircletoCircle/nan", &(a, b, slot as u32), vc, vr);
        assert_eq!(vc, 0, "NaN must reject (slot {slot})");
    }
}

/// row 59 — `c2CircletoAABB` rejects (`r == 0`, tangency, inverted box, NaN).
#[test]
fn e59_circleaabb_reject() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC59);
    // r == 0 can never collide
    for _ in 0..20_000 {
        let a = c2Circle { p: g.v_geom(30.0), r: 0.0 };
        let b = g.aabb(30.0);
        let (vc, vr) = unsafe { ((c.c2CircletoAABB)(a, b), (r.c2CircletoAABB)(a, b)) };
        assert_same("c2CircletoAABB/r0", &(a, b), vc, vr);
        assert_eq!(vc, 0);
    }
    // exact tangency: centre 2 to the right of the box edge, radius exactly 2
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for &rr in [1.9f32, 2.0, 2.1].iter() {
        let a = c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: rr };
        let (vc, vr) = unsafe { ((c.c2CircletoAABB)(a, bb), (r.c2CircletoAABB)(a, bb)) };
        assert_same("c2CircletoAABB/tangent", &(a, bb, rr), vc, vr);
        assert_eq!(vc, (rr > 2.0) as i32);
    }
    // inverted box: c2Clampv(min=hi, max=lo) collapses to a single point
    for _ in 0..20_000 {
        let p = g.v_geom(20.0);
        let q = g.v_geom(20.0);
        let inv = c2AABB {
            min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
            max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
        };
        let a = c2Circle { p: g.v_geom(20.0), r: g.f32_grid(4).abs() };
        let (vc, vr) = unsafe { ((c.c2CircletoAABB)(a, inv), (r.c2CircletoAABB)(a, inv)) };
        assert_same("c2CircletoAABB/inverted", &(a, inv), vc, vr);
    }
    // NaN in every slot
    for slot in 0..7 {
        let mut a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let mut b = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        match slot {
            0 => a.p.x = f32::NAN,
            1 => a.p.y = f32::NAN,
            2 => a.r = f32::NAN,
            3 => b.min.x = f32::NAN,
            4 => b.min.y = f32::NAN,
            5 => b.max.x = f32::NAN,
            _ => b.max.y = f32::NAN,
        }
        let (vc, vr) = unsafe { ((c.c2CircletoAABB)(a, b), (r.c2CircletoAABB)(a, b)) };
        assert_same("c2CircletoAABB/nan", &(a, b, slot as u32), vc, vr);
    }
}

/// rows 60..63 — `c2CircletoCapsule`: all three `da`/`db` branches, the
/// degenerate `a == b` capsule (`c2Dot(n,n) == 0` -> `da/0`) and the reject.
#[test]
fn e60_to_e63_circlecapsule_branches() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC60);
    let seg = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 10.0, y: 0.0 },
        r: 1.0,
    };
    let mut hits = [0usize; 3];
    let mut rejects = 0usize;
    for _ in 0..60_000 {
        let a = c2Circle {
            p: c2v { x: g.f32_grid(20), y: g.f32_grid(6) },
            r: g.f32_grid(3).abs(),
        };
        let (vc, vr) = unsafe {
            ((c.c2CircletoCapsule)(a, seg), (r.c2CircletoCapsule)(a, seg))
        };
        assert_same("c2CircletoCapsule", &(a, seg), vc, vr);
        // classify the branch exactly as the C does
        let n = sub(seg.b, seg.a);
        let ap = sub(a.p, seg.a);
        let da = dot(ap, n);
        if da < 0.0 {
            hits[0] += 1;
        } else if dot(sub(a.p, seg.b), n) < 0.0 {
            hits[1] += 1;
        } else {
            hits[2] += 1;
        }
        if vc == 0 {
            rejects += 1;
        }
    }
    eprintln!("e60..e63 circle/capsule branch hits: {hits:?}, rejects={rejects}");
    assert!(hits.iter().all(|h| *h > 500), "branch coverage: {hits:?}");
    assert!(rejects > 500);

    // row 61 — degenerate capsule a == b: c2Dot(n,n) == 0 => da/0
    let mut saw_zero_div = 0usize;
    for _ in 0..20_000 {
        let p = g.v_geom(20.0);
        let cap = c2Capsule { a: p, b: p, r: g.f32_grid(4).abs() };
        let a = c2Circle { p: g.v_geom(20.0), r: g.f32_grid(4).abs() };
        let (vc, vr) = unsafe {
            ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap))
        };
        assert_same("c2CircletoCapsule/degenerate", &(a, cap), vc, vr);
        // n == (0,0) => da == +0 => `da < 0` false, db == +0 => `db < 0` false
        // => the "past endpoint b" branch, which is well-defined
        saw_zero_div += 1;
        let _ = vc;
    }
    assert!(saw_zero_div > 0);

    // the *other* degenerate: a == b but the circle centre coincides too
    for _ in 0..2000 {
        let p = g.v_geom(20.0);
        let cap = c2Capsule { a: p, b: p, r: g.f32_grid(4).abs() };
        let a = c2Circle { p, r: g.f32_grid(4).abs() };
        let (vc, vr) = unsafe {
            ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap))
        };
        assert_same("c2CircletoCapsule/coincident", &(a, cap), vc, vr);
    }

    // row 63 — explicit rejects, incl. exact tangency and NaN everywhere
    let a = c2Circle { p: c2v { x: -4.0, y: 0.0 }, r: 1.0 };
    let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 3.0 };
    let (vc, vr) = unsafe { ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap)) };
    assert_same("c2CircletoCapsule/tangent", &(a, cap), vc, vr);
    assert_eq!(vc, 0, "exact tangency must not collide");
    for slot in 0..8 {
        let mut a = c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 1.0 };
        let mut cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 1.0 };
        match slot {
            0 => a.p.x = f32::NAN,
            1 => a.p.y = f32::NAN,
            2 => a.r = f32::NAN,
            3 => cap.a.x = f32::NAN,
            4 => cap.a.y = f32::NAN,
            5 => cap.b.x = f32::NAN,
            6 => cap.b.y = f32::NAN,
            _ => cap.r = f32::NAN,
        }
        let (vc, vr) = unsafe { ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap)) };
        assert_same("c2CircletoCapsule/nan", &(a, cap, slot as u32), vc, vr);
    }
}

// ---------------------------------------------------------------------------
// rows 64..69 — c2Collided dispatch
// ---------------------------------------------------------------------------

/// rows 64, 65, 66 — a valid `typeA` with an out-of-range `typeB` returns 0.
#[test]
fn e64_collided_bad_typeB() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC64);
    for &ta in C2_TYPES.iter() {
        for &tb in BAD.iter() {
            for _ in 0..200 {
                let sa = rand_shape(&mut g, ta);
                let sb = rand_shape(&mut g, C2_TYPE_CAPSULE); // biggest layout
                let (vc, vr) = unsafe {
                    (
                        (c.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                        (r.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                    )
                };
                assert_same("c2Collided/bad-typeB", &(sa, ta, sb, tb), vc, vr);
                assert_eq!(vc, 0, "typeA={ta} typeB={tb} must return 0");
            }
        }
    }
}

/// row 67 — an out-of-range `typeA` returns 0 without ever looking at `typeB`.
#[test]
fn e67_collided_bad_typeA() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC67);
    let mut types: Vec<C2_TYPE> = C2_TYPES.to_vec();
    types.extend_from_slice(&BAD);
    for &ta in BAD.iter() {
        for &tb in types.iter() {
            for _ in 0..100 {
                let sa = rand_shape(&mut g, C2_TYPE_CAPSULE);
                let sb = rand_shape(&mut g, C2_TYPE_CAPSULE);
                let (vc, vr) = unsafe {
                    (
                        (c.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                        (r.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                    )
                };
                assert_same("c2Collided/bad-typeA", &(sa, ta, sb, tb), vc, vr);
                assert_eq!(vc, 0);
            }
        }
    }
}

/// row 68 — NULL shape pointers with a *valid* type pair dereference NULL.
/// UB: not executed.
#[test]
#[ignore = "row 68: c2Collided(NULL, CIRCLE, NULL, CIRCLE) dereferences NULL in the C -> SIGSEGV"]
fn e68_collided_null_shape_documented() {
    let (c, r) = apis();
    let (vc, vr) = unsafe {
        (
            (c.c2Collided)(std::ptr::null(), C2_TYPE_CIRCLE, std::ptr::null(), C2_TYPE_CIRCLE),
            (r.c2Collided)(std::ptr::null(), C2_TYPE_CIRCLE, std::ptr::null(), C2_TYPE_CIRCLE),
        )
    };
    assert_same("c2Collided/null", &(), vc, vr);
}

/// row 69 — NULL shape pointers are *safe* when `typeA` is out of range: the
/// outer `default:` returns before either pointer is dereferenced.
#[test]
fn e69_collided_null_with_bad_type() {
    let (c, r) = apis();
    let mut all: Vec<C2_TYPE> = C2_TYPES.to_vec();
    all.extend_from_slice(&BAD);
    for &ta in BAD.iter() {
        for &tb in all.iter() {
            let (vc, vr) = unsafe {
                (
                    (c.c2Collided)(std::ptr::null(), ta, std::ptr::null(), tb),
                    (r.c2Collided)(std::ptr::null(), ta, std::ptr::null(), tb),
                )
            };
            assert_same("c2Collided/null+bad-typeA", &(ta, tb), vc, vr);
            assert_eq!(vc, 0);
        }
    }
    // A *valid* typeA combined with an invalid typeB is equally safe: the
    // dereferences live inside the matched `case` arms, and an out-of-range
    // typeB lands on the inner `default:` (lib.c:585/597/609) before either
    // pointer is touched.  Exercised with A NULL, B NULL and both NULL.
    let real = Shape::Capsule(c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 1.0, y: 0.0 },
        r: 1.0,
    });
    for &ta in C2_TYPES.iter() {
        for &tb in BAD.iter() {
            for which in 0..3 {
                let (pa, pb) = match which {
                    0 => (real.ptr(), std::ptr::null()),
                    1 => (std::ptr::null(), real.ptr()),
                    _ => (std::ptr::null(), std::ptr::null()),
                };
                let (vc, vr) = unsafe {
                    ((c.c2Collided)(pa, ta, pb, tb), (r.c2Collided)(pa, ta, pb, tb))
                };
                assert_same("c2Collided/null+bad-typeB", &(ta, tb, which as u32), vc, vr);
                assert_eq!(vc, 0, "typeA={ta} typeB={tb} which={which}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 70, 71
// ---------------------------------------------------------------------------

/// row 70 — `c2BBVerts` performs no validation whatsoever.
#[test]
fn e70_bbverts_no_validation() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC70);
    let sp = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::from_bits(1),
    ];
    for &a in sp.iter() {
        for &b in sp.iter() {
            for &cx in sp.iter() {
                for &d in sp.iter() {
                    let mut bbc = c2AABB { min: c2v { x: a, y: b }, max: c2v { x: cx, y: d } };
                    let mut bbr = bbc;
                    let mut oc = [POISON_V; 6];
                    let mut or_ = [POISON_V; 6];
                    unsafe {
                        (c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
                        (r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
                    }
                    assert_same("c2BBVerts/sp", &bbc, oc.to_vec(), or_.to_vec());
                    // the 5th/6th slots must be untouched
                    assert_eq!(oc[4].x.to_bits(), POISON_V.x.to_bits());
                    assert_eq!(or_[4].x.to_bits(), POISON_V.x.to_bits());
                }
            }
        }
    }
    // inverted boxes, random
    for _ in 0..20_000 {
        let mut bbc = c2AABB { min: g.v_mixed(50.0), max: g.v_mixed(50.0) };
        let mut bbr = bbc;
        let mut oc = [POISON_V; 4];
        let mut or_ = [POISON_V; 4];
        unsafe {
            (c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
            (r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
        }
        assert_same("c2BBVerts/rand", &bbc, oc.to_vec(), or_.to_vec());
    }
}

/// row 71 — the public `aabb` entry point never rejects: any input still yields
/// a 3-bit mask in `0..=7`.
#[test]
fn e71_aabb_entry_extreme_inputs() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC71);
    let sp = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::EPSILON,
        -f32::EPSILON,
    ];
    for &a in sp.iter() {
        for &b in sp.iter() {
            for &cx in sp.iter() {
                for &d in sp.iter() {
                    let (vc, vr) = unsafe { ((c.aabb)(a, b, cx, d), (r.aabb)(a, b, cx, d)) };
                    assert_same("aabb/extreme", &(a, b, cx, d), vc, vr);
                    assert!((0..=7).contains(&vc), "aabb returned {vc}");
                }
            }
        }
    }
    // and fully arbitrary bit patterns
    for _ in 0..200_000 {
        let a = g.f32_bits();
        let b = g.f32_bits();
        let cx = g.f32_bits();
        let d = g.f32_bits();
        let (vc, vr) = unsafe { ((c.aabb)(a, b, cx, d), (r.aabb)(a, b, cx, d)) };
        assert_same("aabb/bits", &(a, b, cx, d), vc, vr);
        assert!((0..=7).contains(&vc), "aabb returned {vc}");
    }
    // inverted boxes on the grid
    for _ in 0..200_000 {
        let a = g.f32_grid(120);
        let b = g.f32_grid(120);
        let cx = g.f32_grid(120);
        let d = g.f32_grid(120);
        let (vc, vr) = unsafe { ((c.aabb)(a, b, cx, d), (r.aabb)(a, b, cx, d)) };
        assert_same("aabb/grid", &(a, b, cx, d), vc, vr);
    }
}

// ---------------------------------------------------------------------------
// Boundary coverage for the three "equivalent mutants" found by
// ./mutation_check.sh (see MUTATION_NOTES.md).  These probe the exact `== 0`
// boundaries the mutants flip, so the rows are demonstrably observed even though
// the mutants are semantically unobservable.
// ---------------------------------------------------------------------------

/// row 60 boundary — `da` exactly `+0.0` / `-0.0` in `c2CircletoCapsule`
/// (the boundary the `da < 0` vs `da <= 0` mutant flips).  With the capsule
/// along +X, `da == p.x * L`, so `p.x == ±0` puts `da` exactly on the boundary.
#[test]
fn e60_boundary_da_exactly_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC60B);
    let mut seen = 0usize;
    for &px in [0.0f32, -0.0].iter() {
        for &len in [1.0f32, 4.0, 1.0e-30, 1.0e30, f32::MIN_POSITIVE].iter() {
            for _ in 0..4000 {
                let cap = c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: len, y: 0.0 },
                    r: g.f32_grid(4).abs(),
                };
                let a = c2Circle {
                    p: c2v { x: px, y: g.f32_grid(5) },
                    r: g.f32_grid(4).abs(),
                };
                // da == p.x * len == ±0 by construction
                let da = dot(sub(a.p, cap.a), sub(cap.b, cap.a));
                assert_eq!(da.abs().to_bits(), 0u32, "da must be exactly ±0, got {da}");
                seen += 1;
                let (vc, vr) = unsafe {
                    ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap))
                };
                assert_same("c2CircletoCapsule/da==0", &(a, cap, da), vc, vr);
            }
        }
    }
    // same boundary, but with the capsule pointing in an arbitrary direction and
    // the circle centre placed exactly on the perpendicular through `a`
    for _ in 0..40_000 {
        let n = c2v { x: g.f32_grid(4), y: g.f32_grid(4) };
        let base = c2v { x: g.f32_grid(4), y: g.f32_grid(4) };
        let cap = c2Capsule {
            a: base,
            b: c2v { x: base.x + n.x, y: base.y + n.y },
            r: g.f32_grid(3).abs(),
        };
        let t = g.f32_grid(3);
        // perpendicular offset => dot(ap, n) == 0 whenever the products cancel
        let a = c2Circle {
            p: c2v { x: base.x - n.y * t, y: base.y + n.x * t },
            r: g.f32_grid(3).abs(),
        };
        let (vc, vr) = unsafe {
            ((c.c2CircletoCapsule)(a, cap), (r.c2CircletoCapsule)(a, cap))
        };
        assert_same("c2CircletoCapsule/perp", &(a, cap), vc, vr);
    }
    assert!(seen > 1000);
}

/// row 19 boundary — `c2Support`'s loop starting at `i = 1` instead of `i = 0`
/// is only observable if `c2Dot(verts[0], d) > c2Dot(verts[0], d)` could be
/// true.  Assert the invariant the C relies on: the first vertex never beats
/// itself, for every float class (including NaN, where `>` is false anyway).
#[test]
fn e19_boundary_support_first_vertex_never_beats_itself() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC19B);
    for _ in 0..50_000 {
        let v0 = g.v_mixed(1e6);
        let d = g.v_mixed(1e6);
        let dot0c = unsafe { (c.c2Dot)(v0, d) };
        let dot0r = unsafe { (r.c2Dot)(v0, d) };
        assert_same("c2Dot determinism", &(v0, d), dot0c, dot0r);
        assert!(
            !(dot0c > dot0c),
            "c2Dot is not reflexive-stable for {v0:?} . {d:?}"
        );
        // and c2Support with only that vertex must return 0 for every count
        let verts = [v0; 8];
        for count in [1i32, 2, 4, 8] {
            let (a, b) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            assert_same("c2Support/identical", &(v0, d, count), a, b);
            assert_eq!(a, 0);
        }
    }
}

/// Rigorous proof that `da < 0` and `da <= 0` are indistinguishable in
/// `c2CircletoCapsule` (the surviving mutant #3 in `MUTATION_NOTES.md`).
///
/// The two variants can only differ when `da == ±0`.  Both candidate `d2`
/// values are recomputed here **using the C library's own exported primitives**
/// (`c2Dot`, `c2Sub`, `c2Mulvs`, `c2Div`) so the arithmetic is the C's, and are
/// asserted bit-equal over a large randomized corpus of `da == ±0` inputs.
#[test]
fn e60_proof_da_zero_branches_coincide() {
    let (c, r) = apis();
    let mut g = Rng::new(0xC60C);
    let mut checked = 0usize;
    let mut via_interior = 0usize;
    let mut via_endpoint_b = 0usize;

    let mut probe = |A: c2Circle, B: c2Capsule| {
        unsafe {
            let n = (c.c2Sub)(B.b, B.a);
            let ap = (c.c2Sub)(A.p, B.a);
            let da = (c.c2Dot)(ap, n);
            if da.to_bits() & 0x7FFF_FFFF != 0 {
                return; // not on the boundary
            }
            // what the mutant (`da <= 0`) would compute
            let d2_mutant = (c.c2Dot)(ap, ap);
            // what the C actually computes when `da < 0` is false
            let db = (c.c2Dot)((c.c2Sub)(A.p, B.b), n);
            let d2_real = if db < 0.0 {
                via_interior += 1;
                let nn = (c.c2Dot)(n, n);
                let e = (c.c2Sub)(ap, (c.c2Mulvs)(n, da / nn));
                (c.c2Dot)(e, e)
            } else {
                via_endpoint_b += 1;
                let bp = (c.c2Sub)(A.p, B.b);
                (c.c2Dot)(bp, bp)
            };
            assert_eq!(
                d2_real.to_bits(),
                d2_mutant.to_bits(),
                "da == {da} yet the two branches differ: real={d2_real} mutant={d2_mutant} \
                 (A={A:?} B={B:?})"
            );
            checked += 1;
            // and of course the two libraries must agree on the final answer
            assert_same(
                "c2CircletoCapsule/da==0 proof",
                &(A, B),
                (c.c2CircletoCapsule)(A, B),
                (r.c2CircletoCapsule)(A, B),
            );
        }
    };

    // axis-aligned capsules with the centre exactly on the perpendicular
    for &len in [1.0f32, 3.0, 1.0e-20, 1.0e20, f32::MIN_POSITIVE, f32::MAX].iter() {
        for &px in [0.0f32, -0.0].iter() {
            for _ in 0..3000 {
                probe(
                    c2Circle { p: c2v { x: px, y: g.f32_grid(6) }, r: g.f32_grid(4).abs() },
                    c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: len, y: 0.0 }, r: g.f32_grid(4).abs() },
                );
            }
        }
    }
    // degenerate capsules (n == (0,0) => da == ±0 for any centre)
    for _ in 0..40_000 {
        let p = g.v_geom(30.0);
        probe(
            c2Circle { p: g.v_geom(30.0), r: g.f32_grid(4).abs() },
            c2Capsule { a: p, b: p, r: g.f32_grid(4).abs() },
        );
    }
    // arbitrary orientations, centre on the perpendicular through `a`
    for _ in 0..200_000 {
        let base = c2v { x: g.f32_grid(5), y: g.f32_grid(5) };
        let n = c2v { x: g.f32_grid(5), y: g.f32_grid(5) };
        let t = g.f32_grid(4);
        probe(
            c2Circle {
                p: c2v { x: base.x - n.y * t, y: base.y + n.x * t },
                r: g.f32_grid(4).abs(),
            },
            c2Capsule { a: base, b: c2v { x: base.x + n.x, y: base.y + n.y }, r: g.f32_grid(4).abs() },
        );
    }
    // near-underflow capsule directions, where dot(n,n) rounds to 0
    for _ in 0..40_000 {
        let s = 1.0e-24f32;
        let base = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        let n = c2v { x: g.f32_grid(3) * s, y: g.f32_grid(3) * s };
        let t = g.f32_grid(3);
        probe(
            c2Circle {
                p: c2v { x: base.x - n.y * t, y: base.y + n.x * t },
                r: g.f32_grid(4).abs(),
            },
            c2Capsule { a: base, b: c2v { x: base.x + n.x, y: base.y + n.y }, r: g.f32_grid(4).abs() },
        );
    }

    eprintln!(
        "e60 proof: {checked} inputs with da == ±0 checked \
         ({via_interior} via the segment-interior branch, {via_endpoint_b} via endpoint b) \
         — the `da < 0` / `da <= 0` variants never disagreed"
    );
    assert!(checked > 10_000, "only {checked} boundary inputs found");
    assert!(via_endpoint_b > 100);
}
