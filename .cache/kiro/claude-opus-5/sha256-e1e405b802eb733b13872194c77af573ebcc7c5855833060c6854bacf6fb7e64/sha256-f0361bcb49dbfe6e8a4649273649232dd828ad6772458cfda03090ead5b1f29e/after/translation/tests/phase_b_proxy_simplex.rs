//! Phase B — rows 20..46: proxy construction, support function, and the
//! simplex machinery (`c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`,
//! `c2Witness`). These are the LOWEST-level entry points below `c2GJK`; they
//! are driven directly through both `.so`s rather than only via the wrappers.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 3000;

/// A `c2Proxy` pre-seeded with a recognisable pattern, so that any slot the C
/// leaves untouched must also be left untouched by the Rust.
fn seeded_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0x1234_5678),
        count: -999,
        verts: [c2v {
            x: f32::from_bits(0x0BAD_0001),
            y: f32::from_bits(0x0BAD_0002),
        }; 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0x0BAD_0000 + i as u32 * 2);
        v.y = f32::from_bits(0x0BAD_0001 + i as u32 * 2);
    }
    p
}

fn run_make_proxy(
    cf: &FnMakeProxy,
    rf: &FnMakeProxy,
    shape: *const c_void,
    ty: c_int,
    label: &str,
) {
    let mut cp = seeded_proxy();
    let mut rp = seeded_proxy();
    unsafe {
        cf(shape, ty, &raw mut cp);
        rf(shape, ty, &raw mut rp);
    }
    assert_same(label, cp, rp);
}

// ---- rows 20..23: c2MakeProxy -------------------------------------------

#[test]
fn cfg20_23_make_proxy() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xAAAA);

    for _ in 0..N {
        // row 20 — circle
        let mut c = rng.circle();
        run_make_proxy(&cf, &rf, &raw const c as *const c_void, C2_TYPE_CIRCLE, "proxy circle");
        c.r = 0.0;
        run_make_proxy(&cf, &rf, &raw const c as *const c_void, C2_TYPE_CIRCLE, "proxy circle r=0");
        c.r = -rng.radius();
        run_make_proxy(&cf, &rf, &raw const c as *const c_void, C2_TYPE_CIRCLE, "proxy circle r<0");
        let cs = c2Circle {
            p: rng.spicy_vec(),
            r: rng.spicy_f32(),
        };
        run_make_proxy(&cf, &rf, &raw const cs as *const c_void, C2_TYPE_CIRCLE, "proxy circle spicy");

        // row 21 — AABB (radius forced to 0, count 4, 4 verts written)
        let bb = rng.aabb();
        run_make_proxy(&cf, &rf, &raw const bb as *const c_void, C2_TYPE_AABB, "proxy aabb");
        let deg = c2AABB { min: bb.min, max: bb.min };
        run_make_proxy(&cf, &rf, &raw const deg as *const c_void, C2_TYPE_AABB, "proxy aabb degenerate");
        let inv = c2AABB { min: bb.max, max: bb.min };
        run_make_proxy(&cf, &rf, &raw const inv as *const c_void, C2_TYPE_AABB, "proxy aabb inverted");
        let sp = c2AABB {
            min: rng.spicy_vec(),
            max: rng.spicy_vec(),
        };
        run_make_proxy(&cf, &rf, &raw const sp as *const c_void, C2_TYPE_AABB, "proxy aabb spicy");

        // row 22 — capsule
        let mut cap = rng.capsule();
        run_make_proxy(&cf, &rf, &raw const cap as *const c_void, C2_TYPE_CAPSULE, "proxy capsule");
        cap.b = cap.a;
        run_make_proxy(&cf, &rf, &raw const cap as *const c_void, C2_TYPE_CAPSULE, "proxy capsule a==b");
        let spc = c2Capsule {
            a: rng.spicy_vec(),
            b: rng.spicy_vec(),
            r: rng.spicy_f32(),
        };
        run_make_proxy(&cf, &rf, &raw const spc as *const c_void, C2_TYPE_CAPSULE, "proxy capsule spicy");
    }
    // row 23 is implicit in every call above: the whole 72-byte c2Proxy,
    // including the trailing verts the C never writes, is compared.
}

// ---- rows 24..28: c2Support --------------------------------------------

#[test]
fn cfg24_28_support() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let (c_mp, r_mp) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xBBBB);

    // Rows 24/25/26: exactly the vertex counts c2GJK produces (1, 2, 4),
    // sourced from a real proxy built by the library itself.
    for _ in 0..N {
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, C2_TYPE_AABB] {
            let shape = rand_shape(&mut rng, ty);
            let mut cp = seeded_proxy();
            let mut rp = seeded_proxy();
            unsafe {
                c_mp(shape.ptr(), ty, &raw mut cp);
                r_mp(shape.ptr(), ty, &raw mut rp);
            }
            assert_same("proxy for support", cp, rp);
            for d in [
                rng.vec(),
                c2v { x: 1.0, y: 0.0 },
                c2v { x: -1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 0.0, y: -1.0 },
                c2v { x: 0.0, y: 0.0 },
            ] {
                let ci = unsafe { cf(cp.verts.as_ptr(), cp.count, d) };
                let ri = unsafe { rf(rp.verts.as_ptr(), rp.count, d) };
                assert_same("c2Support proxy", ci, ri);
            }
        }
    }

    // Row 27: full verts[8], arbitrary directions.
    for _ in 0..N {
        let verts: [c2v; 8] = std::array::from_fn(|_| rng.vec());
        for count in 1..=8 {
            let d = rng.vec();
            let ci = unsafe { cf(verts.as_ptr(), count, d) };
            let ri = unsafe { rf(verts.as_ptr(), count, d) };
            assert_same("c2Support n=8", ci, ri);
        }
    }

    // Row 28: ties, zero direction, NaN verts.
    let dup = [c2v { x: 3.0, y: 4.0 }; 8];
    for count in 1..=8 {
        for d in [c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 0.0 }] {
            let ci = unsafe { cf(dup.as_ptr(), count, d) };
            let ri = unsafe { rf(dup.as_ptr(), count, d) };
            assert_same("c2Support ties", ci, ri);
        }
    }
    for _ in 0..N {
        let verts: [c2v; 8] = std::array::from_fn(|_| rng.spicy_vec());
        let d = rng.spicy_vec();
        for count in 1..=8 {
            let ci = unsafe { cf(verts.as_ptr(), count, d) };
            let ri = unsafe { rf(verts.as_ptr(), count, d) };
            assert_same("c2Support spicy", ci, ri);
        }
    }
}

// ---- rows 29..31: c2GJKSimplexMetric -----------------------------------

#[test]
fn cfg29_31_simplex_metric() {
    let (cf, rf) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0xCCCC);

    for count in [1, 2, 3] {
        for _ in 0..N {
            let mut cs = rng.simplex(count);
            let mut rs = cs;
            let cv = unsafe { cf(&raw mut cs) };
            let rv = unsafe { rf(&raw mut rs) };
            assert_same(&format!("c2GJKSimplexMetric count={count}"), cv, rv);
            assert_same("metric must not mutate", cs, rs);
        }
    }
    // Collinear (det == 0) and non-finite simplices.
    for _ in 0..N {
        let mut s = rng.simplex(3);
        let a = s.verts[0].p;
        let d = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        let t = rng.range(-3.0, 3.0);
        s.verts[1].p = c2v { x: a.x + d.x, y: a.y + d.y };
        s.verts[2].p = c2v {
            x: a.x + d.x * t,
            y: a.y + d.y * t,
        };
        let mut cs = s;
        let mut rs = s;
        assert_same(
            "metric collinear",
            unsafe { cf(&raw mut cs) },
            unsafe { rf(&raw mut rs) },
        );
        let mut sp = rng.simplex(3);
        sp.verts[0].p = rng.spicy_vec();
        sp.verts[1].p = rng.spicy_vec();
        sp.verts[2].p = rng.spicy_vec();
        let mut cs = sp;
        let mut rs = sp;
        assert_same(
            "metric spicy",
            unsafe { cf(&raw mut cs) },
            unsafe { rf(&raw mut rs) },
        );
    }
}

// ---- rows 32..35: c22 ---------------------------------------------------

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Classify which of the three `c22` branches an input takes (for coverage
/// accounting only — correctness comes from the C-vs-Rust comparison).
fn c22_branch(s: &c2Simplex) -> usize {
    let (a, b) = (s.verts[0].p, s.verts[1].p);
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn cfg32_35_c22() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(0xDDDD);
    let mut hits = [0usize; 3];

    for _ in 0..(N * 8) {
        // Row 35: every byte of the simplex is randomised.
        let s = rng.simplex(2);
        hits[c22_branch(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c22", cs, rs);
    }
    // Exact zero / signed-zero boundaries of `v <= 0` and `u <= 0`.
    for p in [
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }),
        (c2v { x: f32::NAN, y: 1.0 }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: f32::INFINITY, y: 1.0 }, c2v { x: 1.0, y: 2.0 }),
    ] {
        let mut s = rng.simplex(2);
        s.verts[0].p = p.0;
        s.verts[1].p = p.1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c22 boundary", cs, rs);
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c22 branch coverage incomplete: {hits:?}"
    );
    eprintln!("c22 branch hits: {hits:?}");
}

// ---- rows 36..43: c23 ---------------------------------------------------

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
fn cfg36_43_c23() {
    let (cf, rf) = pair::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(0xEEEE);
    let mut hits = [0usize; 7];

    // Random triangles around the origin hit every branch: the branch chosen
    // depends on which Voronoi region the origin falls in.
    for scale in [0.5f32, 2.0, 30.0, 200.0] {
        for _ in 0..(N * 4) {
            let mut s = rng.simplex(3);
            for k in 0..3 {
                s.verts[k].p = c2v {
                    x: rng.range(-scale, scale),
                    y: rng.range(-scale, scale),
                };
            }
            hits[c23_branch(&s)] += 1;
            let mut cs = s;
            let mut rs = s;
            unsafe {
                cf(&raw mut cs);
                rf(&raw mut rs);
            }
            assert_same("c23", cs, rs);
        }
    }
    // Row 43: fully arbitrary bytes, degenerate area, non-finite.
    for _ in 0..(N * 2) {
        let mut s = rng.simplex(3);
        let a = rng.vec();
        let d = rng.vec();
        let t = rng.range(-3.0, 3.0);
        s.verts[0].p = a;
        s.verts[1].p = c2v { x: a.x + d.x, y: a.y + d.y };
        s.verts[2].p = c2v { x: a.x + d.x * t, y: a.y + d.y * t };
        hits[c23_branch(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 collinear", cs, rs);

        let mut sp = rng.simplex(3);
        sp.verts[0].p = rng.spicy_vec();
        sp.verts[1].p = rng.spicy_vec();
        sp.verts[2].p = rng.spicy_vec();
        let mut cs = sp;
        let mut rs = sp;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 spicy", cs, rs);
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c23 branch coverage incomplete: {hits:?}"
    );
    eprintln!("c23 branch hits: {hits:?}");
}

// ---- row 44: c2D --------------------------------------------------------

#[test]
fn cfg44_c2D() {
    let (cf, rf) = pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(0xF00D);
    let mut skew = 0usize;
    let mut ccw = 0usize;

    for count in [0, 1, 2, 3, 4, -1, 77] {
        for _ in 0..N {
            let mut s = rng.simplex(count);
            if count == 2 {
                s.verts[0].p = rng.vec();
                s.verts[1].p = rng.vec();
                let ab = sub(s.verts[1].p, s.verts[0].p);
                let neg = c2v { x: -s.verts[0].p.x, y: -s.verts[0].p.y };
                if det2(ab, neg) > 0.0 {
                    skew += 1;
                } else {
                    ccw += 1;
                }
            }
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&raw mut cs) };
            let rv = unsafe { rf(&raw mut rs) };
            assert_same(&format!("c2D count={count}"), cv, rv);
            assert_same("c2D must not mutate", cs, rs);
        }
    }
    assert!(skew > 0 && ccw > 0, "c2D branch coverage: skew={skew} ccw={ccw}");
}

// ---- row 45: c2L --------------------------------------------------------

#[test]
fn cfg45_c2L() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(0xBEEF);
    for count in [0, 1, 2, 3, 4, -1, 99] {
        for _ in 0..N {
            let mut s = rng.simplex(count);
            if rng.next_u32() % 8 == 0 {
                s.div = 0.0; // exercise den = +Inf
            }
            let mut cs = s;
            let mut rs = s;
            assert_same(
                &format!("c2L count={count}"),
                unsafe { cf(&raw mut cs) },
                unsafe { rf(&raw mut rs) },
            );
            assert_same("c2L must not mutate", cs, rs);
        }
    }
}

// ---- row 46: c2Witness -------------------------------------------------

#[test]
fn cfg46_witness() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0xFACE);
    for count in [0, 1, 2, 3, 4, -1, 123] {
        for _ in 0..N {
            let s = rng.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let mut ca = c2v { x: f32::from_bits(0xDEAD_0001), y: f32::from_bits(0xDEAD_0002) };
            let mut cb = ca;
            let mut ra = ca;
            let mut rb = ca;
            unsafe {
                cf(&raw mut cs, &raw mut ca, &raw mut cb);
                rf(&raw mut rs, &raw mut ra, &raw mut rb);
            }
            assert_same(&format!("c2Witness a count={count}"), ca, ra);
            assert_same(&format!("c2Witness b count={count}"), cb, rb);
            assert_same("c2Witness must not mutate", cs, rs);
        }
    }
    // div == 0 → den = +Inf
    for count in [1, 2, 3] {
        for _ in 0..N {
            let mut s = rng.simplex(count);
            s.div = if rng.next_u32() % 2 == 0 { 0.0 } else { -0.0 };
            let mut cs = s;
            let mut rs = s;
            let mut ca = c2v::default();
            let mut cb = c2v::default();
            let mut ra = c2v::default();
            let mut rb = c2v::default();
            unsafe {
                cf(&raw mut cs, &raw mut ca, &raw mut cb);
                rf(&raw mut rs, &raw mut ra, &raw mut rb);
            }
            assert_same("c2Witness zero div a", ca, ra);
            assert_same("c2Witness zero div b", cb, rb);
        }
    }
}
