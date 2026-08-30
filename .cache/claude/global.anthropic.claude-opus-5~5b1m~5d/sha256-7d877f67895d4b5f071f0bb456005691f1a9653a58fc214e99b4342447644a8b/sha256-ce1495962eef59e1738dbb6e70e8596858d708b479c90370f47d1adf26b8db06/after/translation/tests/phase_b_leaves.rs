#![allow(non_snake_case)]
//! Phase B — valid-path differential tests for the LOW-LEVEL entry points.
//! CONFIGS.md rows 1..33.
//!
//! Every call goes through `.so` exports of both libraries; results are
//! compared BITWISE (`to_bits`), never with an epsilon.

mod common;
use common::*;

const N: usize = 4000;

// --------------------------------------------------------------------- row 1
#[test]
fn cfg_vec_unary() {
    let (cV, rV) = pair::<FnV_ff>("c2V");
    let (cN, rN) = pair::<FnV_v>("c2Neg");
    let (cS, rS) = pair::<FnV_v>("c2Skew");
    let (cW, rW) = pair::<FnV_v>("c2CCW90");
    let mut g = Rng::new(0x1001);
    for _ in 0..N {
        let (x, y) = (g.nasty(), g.nasty());
        same("c2V", cV(x, y), rV(x, y));
        let v = c2v { x, y };
        same("c2Neg", cN(v), rN(v));
        same("c2Skew", cS(v), rS(v));
        same("c2CCW90", cW(v), rW(v));
    }
}

// --------------------------------------------------------------------- row 2
#[test]
fn cfg_vec_binary() {
    let (cA, rA) = pair::<FnV_vv>("c2Add");
    let (cS, rS) = pair::<FnV_vv>("c2Sub");
    let (cD, rD) = pair::<FnF_vv>("c2Dot");
    let (cT, rT) = pair::<FnF_vv>("c2Det2");
    let mut g = Rng::new(0x1002);
    for i in 0..N {
        let a = g.nasty_vec();
        // every 4th pair is nearly equal, to force catastrophic cancellation
        let b = if i % 4 == 0 {
            c2v {
                x: a.x * (1.0 + 1e-7),
                y: a.y * (1.0 - 1e-7),
            }
        } else {
            g.nasty_vec()
        };
        same("c2Add", cA(a, b), rA(a, b));
        same("c2Sub", cS(a, b), rS(a, b));
        same("c2Dot", cD(a, b), rD(a, b));
        same("c2Det2", cT(a, b), rT(a, b));
    }
}

// --------------------------------------------------------------------- row 3
#[test]
fn cfg_vec_scalar() {
    let (cM, rM) = pair::<FnV_vf>("c2Mulvs");
    let (cD, rD) = pair::<FnV_vf>("c2Div");
    let mut g = Rng::new(0x1003);
    for _ in 0..N {
        let a = g.nasty_vec();
        let s = g.nasty();
        same("c2Mulvs", cM(a, s), rM(a, s));
        same("c2Div", cD(a, s), rD(a, s));
    }
}

// --------------------------------------------------------------------- row 4
#[test]
fn cfg_minmax_clamp() {
    let (cX, rX) = pair::<FnV_vv>("c2Maxv");
    let (cI, rI) = pair::<FnV_vv>("c2Minv");
    let (cC, rC) = pair::<FnV_vvv>("c2Clampv");
    let mut g = Rng::new(0x1004);
    for i in 0..N {
        let a = g.nasty_vec();
        let mut b = g.nasty_vec();
        if i % 5 == 0 {
            b = a; // exactly equal components
        }
        same("c2Maxv", cX(a, b), rX(a, b));
        same("c2Minv", cI(a, b), rI(a, b));
        // both the ordinary (lo <= hi) and the INVERTED (lo > hi) clamp box
        let (lo, hi) = if i % 2 == 0 { (a, b) } else { (b, a) };
        let q = g.nasty_vec();
        same("c2Clampv", cC(q, lo, hi), rC(q, lo, hi));
        let p = g.vec();
        same("c2Clampv/geom", cC(p, lo, hi), rC(p, lo, hi));
    }
}

// --------------------------------------------------------------------- row 5
#[test]
fn cfg_len_norm() {
    let (cL, rL) = pair::<FnF_v>("c2Len");
    let (cN, rN) = pair::<FnV_v>("c2Norm");
    let mut g = Rng::new(0x1005);
    let scales = [1.0f32, 1e-30, 1e-20, 1e-3, 1e3, 1e18, 1e20, 1e38];
    for _ in 0..N {
        for s in scales {
            let v = c2v {
                x: g.range(-1.0, 1.0) * s,
                y: g.range(-1.0, 1.0) * s,
            };
            same("c2Len", cL(v), rL(v));
            same("c2Norm", cN(v), rN(v));
        }
        let v = g.nasty_vec();
        same("c2Len/nasty", cL(v), rL(v));
        same("c2Norm/nasty", cN(v), rN(v));
    }
}

// --------------------------------------------------------------------- row 6
#[test]
fn cfg_identities() {
    let (c, r) = pair::<FnR_void>("c2RotIdentity");
    for _ in 0..64 {
        same("c2RotIdentity", c(), r());
    }
    let (c, r) = pair::<FnX_void>("c2xIdentity");
    for _ in 0..64 {
        same("c2xIdentity", c(), r());
    }
}

// --------------------------------------------------------------------- row 7
#[test]
fn cfg_rotations() {
    let (cM, rM) = pair::<FnV_rv>("c2Mulrv");
    let (cT, rT) = pair::<FnV_rv>("c2MulrvT");
    let (cX, rX) = pair::<FnV_xv>("c2Mulxv");
    let mut g = Rng::new(0x1007);
    for _ in 0..N {
        let rot = g.rot();
        let v = g.nasty_vec();
        same("c2Mulrv", cM(rot, v), rM(rot, v));
        same("c2MulrvT", cT(rot, v), rT(rot, v));
        // round trip: MulrvT(r, Mulrv(r, v)) for a normalised r
        let back = cM(rot, v);
        same("c2MulrvT o c2Mulrv", cT(rot, back), rT(rot, back));
        let x = g.xform();
        same("c2Mulxv", cX(x, v), rX(x, v));
        let p = g.vec();
        same("c2Mulxv/geom", cX(x, p), rX(x, p));
    }
}

// --------------------------------------------------------------------- row 8
#[test]
fn cfg_bbverts() {
    let (cf, rf) = pair::<FnBBVerts>("c2BBVerts");
    let mut g = Rng::new(0x1008);
    for i in 0..N {
        let mut bb = match i % 3 {
            0 => g.aabb(),
            1 => {
                let p = g.vec();
                c2AABB { min: p, max: p } // zero area
            }
            _ => c2AABB {
                min: g.nasty_vec(),
                max: g.nasty_vec(),
            },
        };
        let mut co = [c2v { x: 7.0, y: 7.0 }; 4];
        let mut ro = co;
        unsafe {
            cf(co.as_mut_ptr(), &mut bb);
            rf(ro.as_mut_ptr(), &mut bb);
        }
        same("c2BBVerts", co, ro);
    }
}

// ------------------------------------------------------------ rows 9, 10, 11
#[test]
fn cfg_makeproxy_circle() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(0x1009);
    for i in 0..N {
        let c = if i % 4 == 0 {
            c2Circle {
                p: g.nasty_vec(),
                r: g.nasty(),
            }
        } else {
            g.circle()
        };
        let blob = Blob::of_circle(c);
        let (mut cp, mut rp) = (sentinel_proxy(), sentinel_proxy());
        unsafe {
            cf(blob.ptr(), C2_TYPE_CIRCLE, &mut cp);
            rf(blob.ptr(), C2_TYPE_CIRCLE, &mut rp);
        }
        same("c2MakeProxy/circle", cp, rp);
        assert_eq!(cp.count, 1);
    }
}

#[test]
fn cfg_makeproxy_aabb() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(0x100a);
    for i in 0..N {
        let a = if i % 4 == 0 {
            c2AABB {
                min: g.nasty_vec(),
                max: g.nasty_vec(),
            }
        } else {
            g.aabb()
        };
        let blob = Blob::of_aabb(a);
        let (mut cp, mut rp) = (sentinel_proxy(), sentinel_proxy());
        unsafe {
            cf(blob.ptr(), C2_TYPE_AABB, &mut cp);
            rf(blob.ptr(), C2_TYPE_AABB, &mut rp);
        }
        same("c2MakeProxy/aabb", cp, rp);
        assert_eq!((cp.count, cp.radius), (4, 0.0));
    }
}

#[test]
fn cfg_makeproxy_capsule() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(0x100b);
    for i in 0..N {
        let c = if i % 4 == 0 {
            c2Capsule {
                a: g.nasty_vec(),
                b: g.nasty_vec(),
                r: g.nasty(),
            }
        } else {
            g.capsule()
        };
        let blob = Blob::of_capsule(c);
        let (mut cp, mut rp) = (sentinel_proxy(), sentinel_proxy());
        unsafe {
            cf(blob.ptr(), C2_TYPE_CAPSULE, &mut cp);
            rf(blob.ptr(), C2_TYPE_CAPSULE, &mut rp);
        }
        same("c2MakeProxy/capsule", cp, rp);
        assert_eq!(cp.count, 2);
    }
}

fn sentinel_proxy() -> c2Proxy {
    c2Proxy {
        radius: f32::from_bits(0x1234_5678),
        count: -999,
        verts: [c2v {
            x: f32::from_bits(0x0BAD_F00D),
            y: f32::from_bits(0x0BAD_BEEF),
        }; 8],
    }
}

// ---------------------------------------------------------- rows 12,13,14,15
fn support_case(seed: u64, count: i32) {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut g = Rng::new(seed);
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = if i % 5 == 0 { g.nasty_vec() } else { g.vec() };
        }
        // deliberate exact ties every 3rd iteration (dot > dmax is strict)
        if i % 3 == 0 && count >= 2 {
            verts[1] = verts[0];
            if count >= 4 {
                verts[3] = verts[2];
            }
        }
        let dirs = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            g.vec(),
            g.nasty_vec(),
        ];
        for d in dirs {
            let cv = unsafe { cf(verts.as_ptr(), count, d) };
            let rv = unsafe { rf(verts.as_ptr(), count, d) };
            same("c2Support", cv, rv);
        }
    }
}

#[test]
fn cfg_support_count1() {
    support_case(0x100c, 1);
}
#[test]
fn cfg_support_count2() {
    support_case(0x100d, 2);
}
#[test]
fn cfg_support_count4() {
    support_case(0x100e, 4);
}
#[test]
fn cfg_support_count8() {
    support_case(0x100f, 8);
}

// ---------------------------------------------------------- rows 16, 17, 18
fn metric_case(seed: u64, count: i32) {
    let (cf, rf) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut g = Rng::new(seed);
    for i in 0..N {
        let mut s = g.simplex(count);
        if i % 4 == 0 {
            // collinear / degenerate triangles (area ~ 0)
            s.verts[1].p = s.verts[0].p;
            s.verts[2].p = c2v {
                x: s.verts[0].p.x * 2.0,
                y: s.verts[0].p.y * 2.0,
            };
        }
        if i % 7 == 0 {
            for v in s.verts.iter_mut() {
                v.p = g.nasty_vec();
            }
        }
        let mut cs = s;
        let mut rs = s;
        let cv = unsafe { cf(&mut cs) };
        let rv = unsafe { rf(&mut rs) };
        same("c2GJKSimplexMetric", (cv, cs), (rv, rs));
    }
}

#[test]
fn cfg_metric_count1() {
    metric_case(0x1010, 1);
}
#[test]
fn cfg_metric_count2() {
    metric_case(0x1011, 2);
}
#[test]
fn cfg_metric_count3() {
    metric_case(0x1012, 3);
}

// ------------------------------------------------------------ rows 19,20,21
#[test]
fn cfg_c22_branches() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    // branch v<=0, branch u<=0, branch else
    let cases: [(c2v, c2v); 3] = [
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }),
        (c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }),
        (c2v { x: -1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
    ];
    let mut g = Rng::new(0x1013);
    let mut seen = [false; 3];
    for (k, (a, b)) in cases.into_iter().enumerate() {
        for _ in 0..200 {
            let mut s = g.simplex(2);
            s.verts[0].p = a;
            s.verts[1].p = b;
            let mut cs = s;
            let mut rs = s;
            unsafe {
                cf(&mut cs);
                rf(&mut rs);
            }
            same("c22 crafted", cs, rs);
        }
        seen[k] = true;
    }
    assert_eq!(seen, [true; 3]);
}

#[test]
fn cfg_c22_random() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x1014);
    let mut branch_hits = [0usize; 3];
    for i in 0..N * 2 {
        let mut s = g.simplex(2);
        match i % 4 {
            0 => {
                // origin-straddling segment
                s.verts[0].p = g.vec();
                s.verts[1].p = c2v {
                    x: -s.verts[0].p.x,
                    y: -s.verts[0].p.y,
                };
            }
            1 => {
                s.verts[1].p = s.verts[0].p; // degenerate segment
            }
            2 => {
                s.verts[0].p = g.nasty_vec();
                s.verts[1].p = g.nasty_vec();
            }
            _ => {}
        }
        let a = s.verts[0].p;
        let b = s.verts[1].p;
        let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
        let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
        branch_hits[if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        }] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        same("c22 random", cs, rs);
    }
    assert!(
        branch_hits.iter().all(|&h| h > 0),
        "c22 branch coverage: {branch_hits:?}"
    );
}

// ------------------------------------------------------------ rows 23..30
#[test]
fn cfg_c23_random() {
    let (cf, rf) = pair::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x1015);
    let mut hits = [0usize; 7];

    // Hand-crafted representatives of each of the 7 branches, plus their
    // clockwise mirrors (which flip the sign of `area`).
    let crafted: [[c2v; 3]; 7] = [
        // 7: interior
        [v(-1.0, -1.0), v(1.0, -1.0), v(0.0, 2.0)],
        // 1: vertex A region
        [v(1.0, 1.0), v(2.0, 1.0), v(1.0, 2.0)],
        // 2: vertex B region
        [v(2.0, 1.0), v(1.0, 1.0), v(1.0, 2.0)],
        // 3: vertex C region
        [v(1.0, 2.0), v(2.0, 1.0), v(1.0, 1.0)],
        // 4: edge AB
        [v(-1.0, 1.0), v(1.0, 1.0), v(0.0, 3.0)],
        // 5: edge BC
        [v(0.0, 3.0), v(-1.0, 1.0), v(1.0, 1.0)],
        // 6: edge CA
        [v(1.0, 1.0), v(0.0, 3.0), v(-1.0, 1.0)],
    ];

    let mut run = |s: c2Simplex, hits: &mut [usize; 7], tag: &str| {
        let br = classify_c23(&s);
        hits[br] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        same(tag, cs, rs);
    };

    for tri in crafted {
        for mirror in [false, true] {
            for _ in 0..100 {
                let mut s = g.simplex(3);
                for k in 0..3 {
                    s.verts[k].p = if mirror {
                        v(tri[k].x, -tri[k].y)
                    } else {
                        tri[k]
                    };
                }
                run(s, &mut hits, "c23 crafted");
            }
        }
    }

    for i in 0..N * 3 {
        let mut s = g.simplex(3);
        match i % 6 {
            0 => {
                // triangle centred on the origin
                for k in 0..3 {
                    let t = (k as f32) * 2.094_395 + g.range(0.0, 6.28);
                    let rad = g.range(0.1, 50.0);
                    s.verts[k].p = v(rad * t.cos(), rad * t.sin());
                }
            }
            1 => {
                // triangle far from the origin
                let o = v(g.range(50.0, 200.0), g.range(50.0, 200.0));
                for k in 0..3 {
                    s.verts[k].p = v(o.x + g.range(-5.0, 5.0), o.y + g.range(-5.0, 5.0));
                }
            }
            2 => {
                // collinear (area == 0)
                let a = g.vec();
                let d = g.vec();
                for k in 0..3 {
                    let t = g.range(-2.0, 2.0);
                    s.verts[k].p = v(a.x + d.x * t, a.y + d.y * t);
                }
            }
            3 => {
                // duplicated vertices
                s.verts[1].p = s.verts[0].p;
            }
            4 => {
                for k in 0..3 {
                    s.verts[k].p = g.nasty_vec();
                }
            }
            _ => {}
        }
        run(s, &mut hits, "c23 random");
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c23 branch coverage (7 branches): {hits:?}"
    );
    eprintln!("c23 branch hits: {hits:?}");
}

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// Mirrors the branch ladder in `c23` (lib.c:222-261) — bookkeeping only.
fn classify_c23(s: &c2Simplex) -> usize {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let c = s.verts[2].p;
    let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
    let sub = |p: c2v, q: c2v| v(p.x - q.x, p.y - q.y);
    let det = |p: c2v, q: c2v| p.x * q.y - p.y * q.x;
    let uab = dot(b, sub(b, a));
    let vab = dot(a, sub(a, b));
    let ubc = dot(c, sub(c, b));
    let vbc = dot(b, sub(b, c));
    let uca = dot(a, sub(a, c));
    let vca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uabc = det(b, c) * area;
    let vabc = det(c, a) * area;
    let wabc = det(a, b) * area;
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

// --------------------------------------------------------------------- row 31
#[test]
fn cfg_c2d() {
    let (cf, rf) = pair::<FnSimplexV>("c2D");
    let mut g = Rng::new(0x1016);
    let mut det_pos = 0usize;
    let mut det_neg = 0usize;
    for count in [1i32, 2] {
        for i in 0..N * 2 {
            let mut s = g.simplex(count);
            if i % 3 == 0 {
                s.verts[0].p = g.nasty_vec();
                s.verts[1].p = g.nasty_vec();
            }
            if count == 2 {
                let ab = v(
                    s.verts[1].p.x - s.verts[0].p.x,
                    s.verts[1].p.y - s.verts[0].p.y,
                );
                let d = ab.x * s.verts[0].p.y * -1.0 - ab.y * (-s.verts[0].p.x);
                if d > 0.0 {
                    det_pos += 1
                } else {
                    det_neg += 1
                }
            }
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&mut cs) };
            let rv = unsafe { rf(&mut rs) };
            same("c2D", (cv, cs), (rv, rs));
        }
    }
    assert!(det_pos > 0 && det_neg > 0, "c2D count=2: {det_pos}/{det_neg}");
}

// --------------------------------------------------------------------- row 32
#[test]
fn cfg_c2l() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x1017);
    for count in [1i32, 2] {
        for i in 0..N * 2 {
            let mut s = g.simplex(count);
            match i % 5 {
                0 => {
                    // barycentric-looking weights summing to div
                    s.verts[0].u = g.range(0.0, 1.0);
                    s.verts[1].u = g.range(0.0, 1.0);
                    s.div = s.verts[0].u + s.verts[1].u;
                }
                1 => {
                    s.div = 1.0;
                }
                2 => {
                    s.verts[0].u = g.nasty();
                    s.verts[1].u = g.nasty();
                    s.div = g.nasty();
                }
                _ => {}
            }
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&mut cs) };
            let rv = unsafe { rf(&mut rs) };
            same("c2L", (cv, cs), (rv, rs));
        }
    }
}

// --------------------------------------------------------------------- row 33
#[test]
fn cfg_witness() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x1018);
    for count in [1i32, 2, 3] {
        for i in 0..N * 2 {
            let mut s = g.simplex(count);
            match i % 5 {
                0 => {
                    let mut tot = 0.0f32;
                    for k in 0..count as usize {
                        s.verts[k].u = g.range(0.0, 1.0);
                        tot += s.verts[k].u;
                    }
                    s.div = tot;
                }
                1 => s.div = 1.0,
                2 => {
                    for k in 0..4 {
                        s.verts[k].u = g.nasty();
                        s.verts[k].sA = g.nasty_vec();
                        s.verts[k].sB = g.nasty_vec();
                    }
                    s.div = g.nasty();
                }
                _ => {}
            }
            let mut cs = s;
            let mut rs = s;
            let mut ca = v(f32::from_bits(0xDEAD), f32::from_bits(0xBEEF));
            let mut cb = ca;
            let mut ra = ca;
            let mut rb = ca;
            unsafe {
                cf(&mut cs, &mut ca, &mut cb);
                rf(&mut rs, &mut ra, &mut rb);
            }
            same("c2Witness", (ca, cb, cs), (ra, rb, rs));
        }
    }
}
