//! Phase B — CONFIGS.md rows 1..17: the pure vector helpers, the lowest level
//! of the public API.  Every call goes through both `.so`s via libloading.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: u32 = 4000;

/// Interesting float values that must be hit exactly (not just randomly).
fn corner_floats() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        3.0,
        1e-30,
        -1e-30,
        f32::MIN_POSITIVE,
        f32::MIN_POSITIVE / 7.0, // denormal
        FLT_EPSILON,
        -FLT_EPSILON,
        FLT_MAX,
        -FLT_MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234), // NaN, custom payload
        f32::from_bits(0xFFC0_1234), // -NaN, custom payload
        1e18,
        -1e18,
        1e-18,
    ]
}

fn corner_vecs() -> Vec<c2v> {
    let f = corner_floats();
    let mut out = Vec::new();
    for &x in &f {
        out.push(c2v { x, y: x });
    }
    for i in 0..f.len() {
        out.push(c2v { x: f[i], y: f[(i + 5) % f.len()] });
    }
    out.push(c2v { x: 3.0, y: 4.0 });
    out.push(c2v { x: -3.0, y: 4.0 });
    out.push(c2v { x: 1.0, y: 0.0 });
    out.push(c2v { x: 0.0, y: 1.0 });
    out
}

// --------------------------------------------------------------------- row 1
#[test]
fn row01_c2V() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1001);
    for &x in &corner_floats() {
        for &y in &corner_floats() {
            diff_eq!(format!("c2V({x:?},{y:?})"), vb((c.c2V)(x, y)), vb((r.c2V)(x, y)));
        }
    }
    for _ in 0..N {
        let (x, y) = (rng.spicy(1e3), rng.spicy(1e3));
        diff_eq!(format!("c2V({x:?},{y:?})"), vb((c.c2V)(x, y)), vb((r.c2V)(x, y)));
        let (x, y) = (rng.any_bits(), rng.any_bits());
        diff_eq!(format!("c2V bits({x:?},{y:?})"), vb((c.c2V)(x, y)), vb((r.c2V)(x, y)));
    }
}

// --------------------------------------------------------------------- row 2
#[test]
fn row02_c2Mulvs() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1002);
    for &v in &corner_vecs() {
        for &s in &corner_floats() {
            diff_eq!(
                format!("c2Mulvs({v:?},{s:?})"),
                vb((c.c2Mulvs)(v, s)),
                vb((r.c2Mulvs)(v, s))
            );
        }
    }
    for _ in 0..N {
        let v = rng.v_spicy(1e3);
        let s = rng.spicy(1e3);
        diff_eq!(format!("c2Mulvs({v:?},{s:?})"), vb((c.c2Mulvs)(v, s)), vb((r.c2Mulvs)(v, s)));
        // overflow-producing pair
        let v2 = c2v { x: 1e30, y: -1e30 };
        let s2 = rng.sym(1e12);
        diff_eq!(
            format!("c2Mulvs ovf({v2:?},{s2:?})"),
            vb((c.c2Mulvs)(v2, s2)),
            vb((r.c2Mulvs)(v2, s2))
        );
        let v3 = rng.v_bits();
        let s3 = rng.any_bits();
        diff_eq!(
            format!("c2Mulvs bits({v3:?},{s3:?})"),
            vb((c.c2Mulvs)(v3, s3)),
            vb((r.c2Mulvs)(v3, s3))
        );
    }
}

// ------------------------------------------------------------------ rows 3,4
#[test]
fn row03_04_c2Sub_c2Add() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1034);
    let cv = corner_vecs();
    for &a in &cv {
        for &b in &cv {
            diff_eq!(format!("c2Sub({a:?},{b:?})"), vb((c.c2Sub)(a, b)), vb((r.c2Sub)(a, b)));
            diff_eq!(format!("c2Add({a:?},{b:?})"), vb((c.c2Add)(a, b)), vb((r.c2Add)(a, b)));
        }
    }
    for _ in 0..N {
        let a = rng.v_spicy(1e3);
        let b = rng.v_spicy(1e3);
        diff_eq!(format!("c2Sub({a:?},{b:?})"), vb((c.c2Sub)(a, b)), vb((r.c2Sub)(a, b)));
        diff_eq!(format!("c2Add({a:?},{b:?})"), vb((c.c2Add)(a, b)), vb((r.c2Add)(a, b)));
        // equal vectors -> exact zeros with a defined sign
        diff_eq!(format!("c2Sub eq({a:?})"), vb((c.c2Sub)(a, a)), vb((r.c2Sub)(a, a)));
        // x + (-x)
        let na = c2v { x: -a.x, y: -a.y };
        diff_eq!(format!("c2Add neg({a:?})"), vb((c.c2Add)(a, na)), vb((r.c2Add)(a, na)));
        let a2 = rng.v_bits();
        let b2 = rng.v_bits();
        diff_eq!(format!("c2Sub bits"), vb((c.c2Sub)(a2, b2)), vb((r.c2Sub)(a2, b2)));
        diff_eq!(format!("c2Add bits"), vb((c.c2Add)(a2, b2)), vb((r.c2Add)(a2, b2)));
    }
}

// ------------------------------------------------------------------ rows 5,6
#[test]
fn row05_06_c2Dot_c2Det2() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1056);
    let cv = corner_vecs();
    for &a in &cv {
        for &b in &cv {
            diff_eq!(format!("c2Dot({a:?},{b:?})"), fb((c.c2Dot)(a, b)), fb((r.c2Dot)(a, b)));
            diff_eq!(format!("c2Det2({a:?},{b:?})"), fb((c.c2Det2)(a, b)), fb((r.c2Det2)(a, b)));
        }
    }
    for _ in 0..N {
        let a = rng.v_spicy(1e3);
        let b = rng.v_spicy(1e3);
        diff_eq!(format!("c2Dot({a:?},{b:?})"), fb((c.c2Dot)(a, b)), fb((r.c2Dot)(a, b)));
        diff_eq!(format!("c2Det2({a:?},{b:?})"), fb((c.c2Det2)(a, b)), fb((r.c2Det2)(a, b)));

        // orthogonal pair -> dot is exactly 0
        let o = c2v { x: -a.y, y: a.x };
        diff_eq!(format!("c2Dot orth"), fb((c.c2Dot)(a, o)), fb((r.c2Dot)(a, o)));
        // parallel pair -> det2 is exactly 0
        let k = rng.sym(4.0);
        let par = c2v { x: a.x * k, y: a.y * k };
        diff_eq!(format!("c2Det2 par"), fb((c.c2Det2)(a, par)), fb((r.c2Det2)(a, par)));
        // catastrophic cancellation: a.x*b.x == -(a.y*b.y)
        let ca = c2v { x: rng.sym(1e8), y: rng.sym(1e8) };
        let cb = c2v { x: ca.y, y: -ca.x };
        diff_eq!(format!("c2Dot cancel"), fb((c.c2Dot)(ca, cb)), fb((r.c2Dot)(ca, cb)));
        diff_eq!(format!("c2Det2 cancel"), fb((c.c2Det2)(ca, ca)), fb((r.c2Det2)(ca, ca)));
        // overflow
        let ha = c2v { x: 1e30, y: 1e30 };
        let hb = c2v { x: rng.sym(1e20), y: rng.sym(1e20) };
        diff_eq!(format!("c2Dot ovf"), fb((c.c2Dot)(ha, hb)), fb((r.c2Dot)(ha, hb)));
        diff_eq!(format!("c2Det2 ovf"), fb((c.c2Det2)(ha, hb)), fb((r.c2Det2)(ha, hb)));

        let a2 = rng.v_bits();
        let b2 = rng.v_bits();
        diff_eq!("c2Dot bits", fb((c.c2Dot)(a2, b2)), fb((r.c2Dot)(a2, b2)));
        diff_eq!("c2Det2 bits", fb((c.c2Det2)(a2, b2)), fb((r.c2Det2)(a2, b2)));
    }
}

// --------------------------------------------------------------------- row 7
#[test]
fn row07_c2Len() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1007);
    for &v in &corner_vecs() {
        diff_eq!(format!("c2Len({v:?})"), fb((c.c2Len)(v)), fb((r.c2Len)(v)));
    }
    for _ in 0..N {
        for mag in [1.0f32, 1e3, 1e-6, 1e-30, 1e18, 1e30] {
            let v = rng.v(mag);
            diff_eq!(format!("c2Len({v:?})"), fb((c.c2Len)(v)), fb((r.c2Len)(v)));
        }
        let v = rng.v_spicy(1e3);
        diff_eq!(format!("c2Len spicy({v:?})"), fb((c.c2Len)(v)), fb((r.c2Len)(v)));
        let v = rng.v_bits();
        diff_eq!(format!("c2Len bits({v:?})"), fb((c.c2Len)(v)), fb((r.c2Len)(v)));
    }
}

// ------------------------------------------------------------------ rows 8,9
#[test]
fn row08_09_c2Neg_c2Skew_c2CCW90() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1089);
    for &v in &corner_vecs() {
        diff_eq!(format!("c2Neg({v:?})"), vb((c.c2Neg)(v)), vb((r.c2Neg)(v)));
        diff_eq!(format!("c2Skew({v:?})"), vb((c.c2Skew)(v)), vb((r.c2Skew)(v)));
        diff_eq!(format!("c2CCW90({v:?})"), vb((c.c2CCW90)(v)), vb((r.c2CCW90)(v)));
    }
    for _ in 0..N {
        let v = rng.v_spicy(1e3);
        diff_eq!(format!("c2Neg({v:?})"), vb((c.c2Neg)(v)), vb((r.c2Neg)(v)));
        diff_eq!(format!("c2Skew({v:?})"), vb((c.c2Skew)(v)), vb((r.c2Skew)(v)));
        diff_eq!(format!("c2CCW90({v:?})"), vb((c.c2CCW90)(v)), vb((r.c2CCW90)(v)));
        let v = rng.v_bits();
        diff_eq!("c2Neg bits", vb((c.c2Neg)(v)), vb((r.c2Neg)(v)));
        diff_eq!("c2Skew bits", vb((c.c2Skew)(v)), vb((r.c2Skew)(v)));
        diff_eq!("c2CCW90 bits", vb((c.c2CCW90)(v)), vb((r.c2CCW90)(v)));
    }
}

// --------------------------------------------------------------- rows 10, 11
#[test]
fn row10_11_c2Div_c2Norm() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1011);
    for &v in &corner_vecs() {
        diff_eq!(format!("c2Norm({v:?})"), vb((c.c2Norm)(v)), vb((r.c2Norm)(v)));
        for &d in &corner_floats() {
            diff_eq!(
                format!("c2Div({v:?},{d:?})"),
                vb((c.c2Div)(v, d)),
                vb((r.c2Div)(v, d))
            );
        }
    }
    for _ in 0..N {
        let v = rng.v_spicy(1e3);
        let d = rng.spicy(1e3);
        diff_eq!(format!("c2Div({v:?},{d:?})"), vb((c.c2Div)(v, d)), vb((r.c2Div)(v, d)));
        diff_eq!(format!("c2Norm({v:?})"), vb((c.c2Norm)(v)), vb((r.c2Norm)(v)));
        for mag in [1.0f32, 1e-20, 1e20] {
            let v = rng.v(mag);
            diff_eq!(format!("c2Norm mag({v:?})"), vb((c.c2Norm)(v)), vb((r.c2Norm)(v)));
        }
        let v = rng.v_bits();
        let d = rng.any_bits();
        diff_eq!("c2Div bits", vb((c.c2Div)(v, d)), vb((r.c2Div)(v, d)));
        diff_eq!("c2Norm bits", vb((c.c2Norm)(v)), vb((r.c2Norm)(v)));
    }
}

// -------------------------------------------------------------------- row 12
#[test]
fn row12_c2Maxv_c2Minv() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1012);
    let cv = corner_vecs();
    for &a in &cv {
        for &b in &cv {
            diff_eq!(format!("c2Maxv({a:?},{b:?})"), vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
            diff_eq!(format!("c2Minv({a:?},{b:?})"), vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
        }
    }
    // explicit signed-zero and NaN-ordering matrix, both argument orders
    let odd = [
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: 1.0, y: f32::NAN },
        c2v { x: f32::from_bits(0xFFC0_0001), y: f32::from_bits(0x7FC0_0007) },
        c2v { x: 2.0, y: 2.0 },
    ];
    for &a in &odd {
        for &b in &odd {
            diff_eq!(format!("max odd({a:?},{b:?})"), vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
            diff_eq!(format!("min odd({a:?},{b:?})"), vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
        }
    }
    for _ in 0..N {
        let a = rng.v_spicy(10.0);
        let b = rng.v_spicy(10.0);
        diff_eq!(format!("c2Maxv({a:?},{b:?})"), vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
        diff_eq!(format!("c2Minv({a:?},{b:?})"), vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
        // coarse values -> frequent exact ties
        let a = rng.v_coarse(10.0);
        let b = rng.v_coarse(10.0);
        diff_eq!(format!("c2Maxv tie({a:?},{b:?})"), vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
        diff_eq!(format!("c2Minv tie({a:?},{b:?})"), vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
        let a = rng.v_bits();
        let b = rng.v_bits();
        diff_eq!("c2Maxv bits", vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
        diff_eq!("c2Minv bits", vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
    }
}

// -------------------------------------------------------------------- row 13
#[test]
fn row13_c2Clampv() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1013);
    for _ in 0..N {
        // normal lo<hi, a below/inside/above
        let lo = rng.v(10.0);
        let hi = c2v { x: lo.x + rng.unit() * 10.0, y: lo.y + rng.unit() * 10.0 };
        for a in [rng.v(30.0), lo, hi, c2v { x: lo.x - 1.0, y: hi.y + 1.0 }] {
            diff_eq!(
                format!("clamp({a:?},{lo:?},{hi:?})"),
                vb((c.c2Clampv)(a, lo, hi)),
                vb((r.c2Clampv)(a, lo, hi))
            );
        }
        // lo == hi
        diff_eq!(
            format!("clamp lo==hi"),
            vb((c.c2Clampv)(rng.v(10.0), lo, lo)),
            vb((r.c2Clampv)(rng.v(10.0), lo, lo))
        );
        // inverted range lo > hi
        let a = rng.v(10.0);
        diff_eq!(
            format!("clamp inverted({a:?},{hi:?},{lo:?})"),
            vb((c.c2Clampv)(a, hi, lo)),
            vb((r.c2Clampv)(a, hi, lo))
        );
        // NaN in each slot
        let n = c2v { x: f32::NAN, y: f32::from_bits(0xFFC0_0009) };
        for (a, l, h) in [(n, lo, hi), (a, n, hi), (a, lo, n), (n, n, n)] {
            diff_eq!(
                format!("clamp nan({a:?},{l:?},{h:?})"),
                vb((c.c2Clampv)(a, l, h)),
                vb((r.c2Clampv)(a, l, h))
            );
        }
        // fully arbitrary bits
        let (a, l, h) = (rng.v_bits(), rng.v_bits(), rng.v_bits());
        diff_eq!("clamp bits", vb((c.c2Clampv)(a, l, h)), vb((r.c2Clampv)(a, l, h)));
        // coarse (exact ties)
        let (a, l, h) = (rng.v_coarse(10.0), rng.v_coarse(10.0), rng.v_coarse(10.0));
        diff_eq!("clamp coarse", vb((c.c2Clampv)(a, l, h)), vb((r.c2Clampv)(a, l, h)));
    }
}

// -------------------------------------------------------------------- row 14
#[test]
fn row14_identities() {
    let (c, r) = apis();
    diff_eq!("c2RotIdentity", rb((c.c2RotIdentity)()), rb((r.c2RotIdentity)()));
    diff_eq!("c2xIdentity", xb((c.c2xIdentity)()), xb((r.c2xIdentity)()));
    // call repeatedly: must be stateless
    for _ in 0..100 {
        diff_eq!("c2RotIdentity rep", rb((c.c2RotIdentity)()), rb((r.c2RotIdentity)()));
        diff_eq!("c2xIdentity rep", xb((c.c2xIdentity)()), xb((r.c2xIdentity)()));
    }
}

// ---------------------------------------------------------------- rows 15,16
#[test]
fn row15_16_c2Mulrv_c2MulrvT() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1516);

    let mut rots: Vec<c2r> = vec![
        c2r { c: 1.0, s: 0.0 },        // identity
        c2r { c: 0.0, s: 1.0 },        // +90
        c2r { c: 0.0, s: -1.0 },       // -90
        c2r { c: -1.0, s: 0.0 },       // 180
        c2r { c: 0.0, s: 0.0 },        // degenerate
        c2r { c: -0.0, s: -0.0 },      // signed-zero degenerate
        c2r { c: 3.0, s: -7.5 },       // non-unit
        c2r { c: f32::NAN, s: 1.0 },
        c2r { c: 1.0, s: f32::NAN },
        c2r { c: f32::INFINITY, s: f32::NEG_INFINITY },
        c2r { c: 1e30, s: -1e30 },
        c2r { c: f32::from_bits(0xFFC0_0003), s: f32::from_bits(0x7FC0_0005) },
    ];
    for _ in 0..64 {
        rots.push(rng.rot());
        rots.push(c2r { c: rng.spicy(10.0), s: rng.spicy(10.0) });
    }

    let mut vs = corner_vecs();
    for _ in 0..64 {
        vs.push(rng.v(1e3));
        vs.push(rng.v_spicy(1e3));
    }

    for &m in &rots {
        for &v in &vs {
            diff_eq!(
                format!("c2Mulrv({m:?},{v:?})"),
                vb((c.c2Mulrv)(m, v)),
                vb((r.c2Mulrv)(m, v))
            );
            diff_eq!(
                format!("c2MulrvT({m:?},{v:?})"),
                vb((c.c2MulrvT)(m, v)),
                vb((r.c2MulrvT)(m, v))
            );
        }
    }
    for _ in 0..N {
        let m = c2r { c: rng.any_bits(), s: rng.any_bits() };
        let v = rng.v_bits();
        diff_eq!(format!("c2Mulrv bits({m:?},{v:?})"), vb((c.c2Mulrv)(m, v)), vb((r.c2Mulrv)(m, v)));
        diff_eq!(
            format!("c2MulrvT bits({m:?},{v:?})"),
            vb((c.c2MulrvT)(m, v)),
            vb((r.c2MulrvT)(m, v))
        );
    }
}

// -------------------------------------------------------------------- row 17
#[test]
fn row17_c2Mulxv() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1017);
    let mut xs: Vec<c2x> = vec![
        c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }, // identity
        c2x { p: c2v { x: 5.0, y: -7.0 }, r: c2r { c: 1.0, s: 0.0 } }, // pure translation
        c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 0.0, s: 1.0 } }, // pure rotation
        c2x { p: c2v { x: f32::NAN, y: 1.0 }, r: c2r { c: 1.0, s: 0.0 } },
        c2x { p: c2v { x: 1e30, y: -1e30 }, r: c2r { c: 1e30, s: 1e30 } },
        c2x { p: c2v { x: -0.0, y: -0.0 }, r: c2r { c: -0.0, s: -0.0 } },
    ];
    for _ in 0..96 {
        xs.push(c2x { p: rng.v(1e3), r: rng.rot() });
        xs.push(c2x { p: rng.v_spicy(1e3), r: c2r { c: rng.spicy(4.0), s: rng.spicy(4.0) } });
    }
    let mut vs = corner_vecs();
    for _ in 0..64 {
        vs.push(rng.v(1e3));
        vs.push(rng.v_spicy(1e3));
    }
    for &x in &xs {
        for &v in &vs {
            diff_eq!(
                format!("c2Mulxv({x:?},{v:?})"),
                vb((c.c2Mulxv)(x, v)),
                vb((r.c2Mulxv)(x, v))
            );
        }
    }
    for _ in 0..N {
        let x = c2x {
            p: rng.v_bits(),
            r: c2r { c: rng.any_bits(), s: rng.any_bits() },
        };
        let v = rng.v_bits();
        diff_eq!(format!("c2Mulxv bits({x:?},{v:?})"), vb((c.c2Mulxv)(x, v)), vb((r.c2Mulxv)(x, v)));
    }
}
