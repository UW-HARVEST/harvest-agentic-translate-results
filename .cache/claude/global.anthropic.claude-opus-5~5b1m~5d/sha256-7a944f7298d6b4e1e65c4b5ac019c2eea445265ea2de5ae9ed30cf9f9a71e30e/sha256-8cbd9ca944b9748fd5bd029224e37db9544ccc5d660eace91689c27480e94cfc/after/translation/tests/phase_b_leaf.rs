//! Phase B — CONFIGS.md rows 1..19 (leaf vector helpers + proxy construction).
//!
//! Every call goes through the `.so` export of BOTH libraries.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 4000;

#[test]
fn row01_c2V() {
    let a = api();
    let mut d = Diff::new("row 1: c2V");
    let r = Rng::new(1);
    for _ in 0..N {
        let (x, y) = (r.wild(), r.wild());
        let (c, s) = ((a.c2V.0)(x, y), (a.c2V.1)(x, y));
        d.check(veq(c, s), || format!("c2V({}, {}) C={} R={}", fmt_f(x), fmt_f(y), fmt_v(c), fmt_v(s)));
    }
    // sign of zero must survive
    for &(x, y) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        let (c, s) = ((a.c2V.0)(x, y), (a.c2V.1)(x, y));
        d.check(veq(c, s), || format!("c2V zero-sign C={} R={}", fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row02_c2Mulvs() {
    let a = api();
    let mut d = Diff::new("row 2: c2Mulvs");
    let r = Rng::new(2);
    for _ in 0..N {
        let v = r.wild_v();
        let b = match r.below(5) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => 1.0e30,
            _ => r.wild(),
        };
        let (c, s) = ((a.c2Mulvs.0)(v, b), (a.c2Mulvs.1)(v, b));
        d.check(veq(c, s), || format!("c2Mulvs({}, {}) C={} R={}", fmt_v(v), fmt_f(b), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row03_c2Add_c2Sub() {
    let a = api();
    let mut d = Diff::new("row 3: c2Add / c2Sub");
    let r = Rng::new(3);
    for _ in 0..N {
        let u = r.wild_v();
        let v = if r.below(4) == 0 { u } else { r.wild_v() };
        let (c, s) = ((a.c2Add.0)(u, v), (a.c2Add.1)(u, v));
        d.check(veq(c, s), || format!("c2Add({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
        let (c, s) = ((a.c2Sub.0)(u, v), (a.c2Sub.1)(u, v));
        d.check(veq(c, s), || format!("c2Sub({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row04_c2Dot() {
    let a = api();
    let mut d = Diff::new("row 4: c2Dot");
    let r = Rng::new(4);
    for _ in 0..N {
        let u = r.wild_v();
        let v = match r.below(4) {
            0 => u,
            1 => c2v { x: -u.y, y: u.x }, // orthogonal
            _ => r.wild_v(),
        };
        let (c, s) = ((a.c2Dot.0)(u, v), (a.c2Dot.1)(u, v));
        d.check(feq(c, s), || format!("c2Dot({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_f(c), fmt_f(s)));
    }
    d.finish();
}

#[test]
fn row05_c2Det2() {
    let a = api();
    let mut d = Diff::new("row 5: c2Det2");
    let r = Rng::new(5);
    for _ in 0..N {
        let u = r.wild_v();
        let v = match r.below(4) {
            0 => u,
            1 => c2v { x: u.x * 2.0, y: u.y * 2.0 }, // collinear
            _ => r.wild_v(),
        };
        for (p, q) in [(u, v), (v, u)] {
            let (c, s) = ((a.c2Det2.0)(p, q), (a.c2Det2.1)(p, q));
            d.check(feq(c, s), || format!("c2Det2({}, {}) C={} R={}", fmt_v(p), fmt_v(q), fmt_f(c), fmt_f(s)));
        }
    }
    d.finish();
}

#[test]
fn row06_c2Len() {
    let a = api();
    let mut d = Diff::new("row 6: c2Len");
    let r = Rng::new(6);
    let fixed = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: 1.0e30, y: 1.0e30 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: 1.0e-30, y: 1.0e-30 },
    ];
    for v in fixed {
        let (c, s) = ((a.c2Len.0)(v), (a.c2Len.1)(v));
        d.check(feq(c, s), || format!("c2Len({}) C={} R={}", fmt_v(v), fmt_f(c), fmt_f(s)));
    }
    for _ in 0..N {
        let v = r.wild_v();
        let (c, s) = ((a.c2Len.0)(v), (a.c2Len.1)(v));
        d.check(feq(c, s), || format!("c2Len({}) C={} R={}", fmt_v(v), fmt_f(c), fmt_f(s)));
    }
    d.finish();
}

#[test]
fn row07_c2Maxv_c2Minv() {
    let a = api();
    let mut d = Diff::new("row 7: c2Maxv / c2Minv (ternary NaN + tie semantics)");
    let r = Rng::new(7);
    let nan = f32::NAN;
    let fixed: &[(c2v, c2v)] = &[
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }), // exact tie -> C returns b
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: nan, y: nan }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
        (c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }, c2v { x: 0.0, y: 0.0 }),
    ];
    for &(u, v) in fixed {
        let (c, s) = ((a.c2Maxv.0)(u, v), (a.c2Maxv.1)(u, v));
        d.check(veq(c, s), || format!("c2Maxv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
        let (c, s) = ((a.c2Minv.0)(u, v), (a.c2Minv.1)(u, v));
        d.check(veq(c, s), || format!("c2Minv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    for _ in 0..N {
        let u = r.wild_v();
        let mut v = r.wild_v();
        if r.below(4) == 0 {
            v.x = u.x; // force a tie on x
        }
        if r.below(6) == 0 {
            v.y = nan;
        }
        let (c, s) = ((a.c2Maxv.0)(u, v), (a.c2Maxv.1)(u, v));
        d.check(veq(c, s), || format!("c2Maxv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
        let (c, s) = ((a.c2Minv.0)(u, v), (a.c2Minv.1)(u, v));
        d.check(veq(c, s), || format!("c2Minv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row08_c2Clampv() {
    let a = api();
    let mut d = Diff::new("row 8: c2Clampv (normal + inverted box)");
    let r = Rng::new(8);
    for _ in 0..N {
        let p = r.wild_v();
        let lo = r.geo_v();
        let hi = if r.below(3) == 0 {
            // inverted: hi < lo
            c2v { x: lo.x - r.unit() * 10.0, y: lo.y - r.unit() * 10.0 }
        } else {
            c2v { x: lo.x + r.unit() * 50.0, y: lo.y + r.unit() * 50.0 }
        };
        let (c, s) = ((a.c2Clampv.0)(p, lo, hi), (a.c2Clampv.1)(p, lo, hi));
        d.check(veq(c, s), || {
            format!("c2Clampv({}, {}, {}) C={} R={}", fmt_v(p), fmt_v(lo), fmt_v(hi), fmt_v(c), fmt_v(s))
        });
    }
    d.finish();
}

#[test]
fn row09_c2Neg_c2Skew_c2CCW90() {
    let a = api();
    let mut d = Diff::new("row 9: c2Neg / c2Skew / c2CCW90");
    let r = Rng::new(9);
    let fixed = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ];
    let mut all: Vec<c2v> = fixed.to_vec();
    for _ in 0..N {
        all.push(r.wild_v());
    }
    for v in all {
        for (name, f) in [("c2Neg", a.c2Neg), ("c2Skew", a.c2Skew), ("c2CCW90", a.c2CCW90)] {
            let (c, s) = ((f.0)(v), (f.1)(v));
            d.check(veq(c, s), || format!("{name}({}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
        }
    }
    d.finish();
}

#[test]
fn row10_c2Div() {
    let a = api();
    let mut d = Diff::new("row 10: c2Div");
    let r = Rng::new(10);
    for _ in 0..N {
        let v = r.wild_v();
        let b = match r.below(6) {
            0 => 1.0,
            1 => 1.0e-30,
            2 => -1.0e-30,
            3 => 1.0e30,
            _ => r.wild(),
        };
        let (c, s) = ((a.c2Div.0)(v, b), (a.c2Div.1)(v, b));
        d.check(veq(c, s), || format!("c2Div({}, {}) C={} R={}", fmt_v(v), fmt_f(b), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row11_c2Norm() {
    let a = api();
    let mut d = Diff::new("row 11: c2Norm");
    let r = Rng::new(11);
    let fixed = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.6, y: 0.8 },
        c2v { x: 1.0e30, y: 1.0e30 },
        c2v { x: 1.0e-30, y: 1.0e-30 },
    ];
    for v in fixed {
        let (c, s) = ((a.c2Norm.0)(v), (a.c2Norm.1)(v));
        d.check(veq(c, s), || format!("c2Norm({}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    for _ in 0..N {
        let v = r.wild_v();
        let (c, s) = ((a.c2Norm.0)(v), (a.c2Norm.1)(v));
        d.check(veq(c, s), || format!("c2Norm({}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row12_identities() {
    let a = api();
    let mut d = Diff::new("row 12: c2RotIdentity / c2xIdentity");
    let (c, s) = ((a.c2RotIdentity.0)(), (a.c2RotIdentity.1)());
    d.check(req(c, s), || format!("c2RotIdentity C={c:?} R={s:?}"));
    let (c, s) = ((a.c2xIdentity.0)(), (a.c2xIdentity.1)());
    d.check(xeq(c, s), || format!("c2xIdentity C={c:?} R={s:?}"));
    d.check(bytes_of(&c) == bytes_of(&s), || "c2xIdentity byte image".into());
    d.finish();
}

#[test]
fn row13_c2Mulrv_c2MulrvT() {
    let a = api();
    let mut d = Diff::new("row 13: c2Mulrv / c2MulrvT");
    let r = Rng::new(13);
    let fixed_rots = [
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: 0.0, s: -1.0 },
        c2r { c: 3.0, s: -7.0 }, // unnormalized
        c2r { c: 0.0, s: 0.0 },  // degenerate
    ];
    for rot in fixed_rots {
        for _ in 0..64 {
            let v = r.wild_v();
            let (c, s) = ((a.c2Mulrv.0)(rot, v), (a.c2Mulrv.1)(rot, v));
            d.check(veq(c, s), || format!("c2Mulrv({rot:?}, {}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
            let (c, s) = ((a.c2MulrvT.0)(rot, v), (a.c2MulrvT.1)(rot, v));
            d.check(veq(c, s), || format!("c2MulrvT({rot:?}, {}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
        }
    }
    for _ in 0..N {
        let rot = if r.below(3) == 0 { c2r { c: r.wild(), s: r.wild() } } else { r.rot() };
        let v = r.wild_v();
        let (c, s) = ((a.c2Mulrv.0)(rot, v), (a.c2Mulrv.1)(rot, v));
        d.check(veq(c, s), || format!("c2Mulrv({rot:?}, {}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
        let (c, s) = ((a.c2MulrvT.0)(rot, v), (a.c2MulrvT.1)(rot, v));
        d.check(veq(c, s), || format!("c2MulrvT({rot:?}, {}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row14_c2Mulxv() {
    let a = api();
    let mut d = Diff::new("row 14: c2Mulxv");
    let r = Rng::new(14);
    for _ in 0..N {
        let xf = match r.below(4) {
            0 => (a.c2xIdentity.0)(),
            1 => c2x { p: r.geo_v(), r: c2r { c: 1.0, s: 0.0 } }, // translation only
            2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: r.rot() },   // rotation only
            _ => r.xform(),
        };
        let v = r.wild_v();
        let (c, s) = ((a.c2Mulxv.0)(xf, v), (a.c2Mulxv.1)(xf, v));
        d.check(veq(c, s), || format!("c2Mulxv({xf:?}, {}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

#[test]
fn row15_16_c2BBVerts() {
    let a = api();
    let mut d = Diff::new("rows 15-16: c2BBVerts (normal / degenerate / inverted)");
    let r = Rng::new(1516);
    for i in 0..N {
        let mut bb = match i % 4 {
            0 => r.aabb(),
            1 => {
                let p = r.geo_v();
                c2AABB { min: p, max: p }
            }
            2 => {
                let bb = r.aabb();
                c2AABB { min: bb.max, max: bb.min } // inverted
            }
            _ => c2AABB { min: r.wild_v(), max: r.wild_v() },
        };
        let mut co = [c2v::default(); 4];
        let mut ro = [c2v::default(); 4];
        unsafe {
            (a.c2BBVerts.0)(co.as_mut_ptr(), &mut bb);
            (a.c2BBVerts.1)(ro.as_mut_ptr(), &mut bb);
        }
        d.check(bytes_of(&co) == bytes_of(&ro), || format!("c2BBVerts({bb:?}) C={co:?} R={ro:?}"));
    }
    d.finish();
}

/// Rows 17-19: `c2MakeProxy` for the three valid shape types.  The output
/// struct is pre-filled with a recognisable pattern so that *any* byte the C
/// leaves untouched is visible in the comparison.
#[test]
fn row17_19_c2MakeProxy_valid() {
    let a = api();
    let mut d = Diff::new("rows 17-19: c2MakeProxy CIRCLE / AABB / CAPSULE");
    let r = Rng::new(1719);

    fn seeded(tag: f32) -> c2Proxy {
        let mut p = c2Proxy { radius: tag, count: -12345, verts: [c2v::default(); 8] };
        for i in 0..8 {
            p.verts[i] = c2v { x: tag + i as f32, y: tag - i as f32 };
        }
        p
    }

    for _ in 0..N {
        // CIRCLE
        let circle = r.circle();
        let (mut cp, mut rp) = (seeded(7.5), seeded(7.5));
        unsafe {
            (a.c2MakeProxy.0)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut cp);
            (a.c2MakeProxy.1)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut rp);
        }
        d.check(proxyeq(&cp, &rp), || format!("c2MakeProxy CIRCLE {circle:?}\n C={cp:?}\n R={rp:?}"));

        // AABB
        let bb = r.aabb();
        let (mut cp, mut rp) = (seeded(-3.25), seeded(-3.25));
        unsafe {
            (a.c2MakeProxy.0)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut cp);
            (a.c2MakeProxy.1)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut rp);
        }
        d.check(proxyeq(&cp, &rp), || format!("c2MakeProxy AABB {bb:?}\n C={cp:?}\n R={rp:?}"));

        // CAPSULE
        let cap = r.capsule();
        let (mut cp, mut rp) = (seeded(11.0), seeded(11.0));
        unsafe {
            (a.c2MakeProxy.0)(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut cp);
            (a.c2MakeProxy.1)(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut rp);
        }
        d.check(proxyeq(&cp, &rp), || format!("c2MakeProxy CAPSULE {cap:?}\n C={cp:?}\n R={rp:?}"));
    }
    d.finish();
}
