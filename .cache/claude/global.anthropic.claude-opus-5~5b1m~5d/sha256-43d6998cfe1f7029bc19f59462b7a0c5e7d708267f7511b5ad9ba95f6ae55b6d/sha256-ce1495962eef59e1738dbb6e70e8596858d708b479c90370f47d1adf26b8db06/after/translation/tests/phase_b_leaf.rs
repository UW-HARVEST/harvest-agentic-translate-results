//! Phase B — Group 1 & 2: leaf vector/scalar primitives and transforms.
//! CONFIGS.md rows 1..21.

mod common;
use common::*;

const N: usize = 4096;

#[test]
fn smoke_libraries_load() {
    let l = libs();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.r_path.display());
    // sanity: identity functions
    diff("c2RotIdentity", |s| c2RotIdentity(s));
    diff("c2xIdentity", |s| c2xIdentity(s));
}

// --- row 1 ---------------------------------------------------------------
#[test]
fn cfg_leaf_v() {
    let mut a = DiffAccum::new("cfg_leaf_v");
    let mut rng = Rng::new(0x5eed_0001);
    for i in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        a.check(format!("{i} any {x:e} {y:e}"), |s| c2V(s, x, y));
    }
    for i in 0..N {
        let (x, y) = (rng.special(), rng.special());
        a.check(format!("{i} special"), |s| c2V(s, x, y));
    }
    a.finish();
}

// --- row 2 ---------------------------------------------------------------
#[test]
fn cfg_leaf_mulvs() {
    let mut a = DiffAccum::new("cfg_leaf_mulvs");
    let mut rng = Rng::new(0x5eed_0002);
    for i in 0..N {
        let v = rng.vec();
        let k = rng.coord();
        a.check(format!("{i} plain"), |s| c2Mulvs(s, v, k));
    }
    for i in 0..N {
        let v = rng.special_vec();
        let k = rng.special();
        a.check(format!("{i} special"), |s| c2Mulvs(s, v, k));
    }
    for i in 0..N {
        let v = rng.any_vec();
        let k = rng.any_f32();
        a.check(format!("{i} any"), |s| c2Mulvs(s, v, k));
    }
    a.finish();
}

// --- row 3 ---------------------------------------------------------------
#[test]
fn cfg_leaf_addsub() {
    let mut a = DiffAccum::new("cfg_leaf_addsub");
    let mut rng = Rng::new(0x5eed_0003);
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        a.check(format!("{i} add plain"), |s| c2Add(s, u, v));
        a.check(format!("{i} sub plain"), |s| c2Sub(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.special_vec(), rng.special_vec());
        a.check(format!("{i} add special"), |s| c2Add(s, u, v));
        a.check(format!("{i} sub special"), |s| c2Sub(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        a.check(format!("{i} add any"), |s| c2Add(s, u, v));
        a.check(format!("{i} sub any"), |s| c2Sub(s, u, v));
    }
    a.finish();
}

// --- row 4 ---------------------------------------------------------------
#[test]
fn cfg_leaf_dot() {
    let mut a = DiffAccum::new("cfg_leaf_dot");
    let mut rng = Rng::new(0x5eed_0004);
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        a.check(format!("{i} plain"), |s| c2Dot(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.special_vec(), rng.special_vec());
        a.check(format!("{i} special {u:?} {v:?}"), |s| c2Dot(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        a.check(format!("{i} any {u:?} {v:?}"), |s| c2Dot(s, u, v));
    }
    a.finish();
}

// --- row 5 ---------------------------------------------------------------
#[test]
fn cfg_leaf_det2() {
    let mut a = DiffAccum::new("cfg_leaf_det2");
    let mut rng = Rng::new(0x5eed_0005);
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        a.check(format!("{i} plain"), |s| c2Det2(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.special_vec(), rng.special_vec());
        a.check(format!("{i} special {u:?} {v:?}"), |s| c2Det2(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        a.check(format!("{i} any {u:?} {v:?}"), |s| c2Det2(s, u, v));
    }
    a.finish();
}

// --- row 6 ---------------------------------------------------------------
#[test]
fn cfg_leaf_len() {
    let mut a = DiffAccum::new("cfg_leaf_len");
    let mut rng = Rng::new(0x5eed_0006);
    for i in 0..N {
        let v = rng.vec();
        a.check(format!("{i} plain"), |s| c2Len(s, v));
    }
    for i in 0..N {
        let v = rng.special_vec();
        a.check(format!("{i} special {v:?}"), |s| c2Len(s, v));
    }
    for i in 0..N {
        let v = rng.any_vec();
        a.check(format!("{i} any {v:?}"), |s| c2Len(s, v));
    }
    a.finish();
}

// --- row 7 ---------------------------------------------------------------
#[test]
fn cfg_leaf_unary() {
    let mut a = DiffAccum::new("cfg_leaf_unary");
    let mut rng = Rng::new(0x5eed_0007);
    for i in 0..(N * 2) {
        let v = if i % 3 == 0 {
            rng.any_vec()
        } else if i % 3 == 1 {
            rng.special_vec()
        } else {
            rng.vec()
        };
        a.check(format!("{i} neg {v:?}"), |s| c2Neg(s, v));
        a.check(format!("{i} skew {v:?}"), |s| c2Skew(s, v));
        a.check(format!("{i} ccw90 {v:?}"), |s| c2CCW90(s, v));
        a.check(format!("{i} absv {v:?}"), |s| c2Absv(s, v));
    }
    a.finish();
}

// --- row 8 ---------------------------------------------------------------
#[test]
fn cfg_leaf_minmax() {
    let mut a = DiffAccum::new("cfg_leaf_minmax");
    let mut rng = Rng::new(0x5eed_0008);
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        a.check(format!("{i} max plain"), |s| c2Maxv(s, u, v));
        a.check(format!("{i} min plain"), |s| c2Minv(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.special_vec(), rng.special_vec());
        a.check(format!("{i} max special {u:?} {v:?}"), |s| c2Maxv(s, u, v));
        a.check(format!("{i} min special {u:?} {v:?}"), |s| c2Minv(s, u, v));
    }
    for i in 0..N {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        a.check(format!("{i} max any"), |s| c2Maxv(s, u, v));
        a.check(format!("{i} min any"), |s| c2Minv(s, u, v));
    }
    // equal / signed zero
    for &(x, y) in &[
        (0.0f32, -0.0f32),
        (-0.0, 0.0),
        (1.0, 1.0),
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::NAN, f32::NAN),
    ] {
        let u = c2v { x, y };
        let v = c2v { x: y, y: x };
        a.check(format!("edge max {x:?} {y:?}"), |s| c2Maxv(s, u, v));
        a.check(format!("edge min {x:?} {y:?}"), |s| c2Minv(s, u, v));
    }
    a.finish();
}

// --- row 9 ---------------------------------------------------------------
#[test]
fn cfg_leaf_clampv() {
    let mut a = DiffAccum::new("cfg_leaf_clampv");
    let mut rng = Rng::new(0x5eed_0009);
    for i in 0..N {
        let (v, lo, hi) = (rng.vec(), rng.vec(), rng.vec());
        a.check(format!("{i} plain (lo may exceed hi)"), |s| {
            c2Clampv(s, v, lo, hi)
        });
    }
    for i in 0..N {
        let (v, lo, hi) = (rng.special_vec(), rng.special_vec(), rng.special_vec());
        a.check(format!("{i} special"), |s| c2Clampv(s, v, lo, hi));
    }
    for i in 0..N {
        let (v, lo, hi) = (rng.any_vec(), rng.any_vec(), rng.any_vec());
        a.check(format!("{i} any"), |s| c2Clampv(s, v, lo, hi));
    }
    a.finish();
}

// --- row 10 --------------------------------------------------------------
#[test]
fn cfg_leaf_div() {
    let mut a = DiffAccum::new("cfg_leaf_div");
    let mut rng = Rng::new(0x5eed_000a);
    for i in 0..N {
        let v = rng.vec();
        let k = rng.coord();
        a.check(format!("{i} plain k={k:?}"), |s| c2Div(s, v, k));
    }
    for i in 0..N {
        let v = rng.special_vec();
        let k = rng.special();
        a.check(format!("{i} special k={k:?}"), |s| c2Div(s, v, k));
    }
    for i in 0..N {
        let v = rng.any_vec();
        let k = rng.any_f32();
        a.check(format!("{i} any"), |s| c2Div(s, v, k));
    }
    a.finish();
}

// --- row 11 --------------------------------------------------------------
#[test]
fn cfg_leaf_norm() {
    let mut a = DiffAccum::new("cfg_leaf_norm");
    let mut rng = Rng::new(0x5eed_000b);
    for i in 0..N {
        let v = rng.vec();
        a.check(format!("{i} plain {v:?}"), |s| c2Norm(s, v));
    }
    for i in 0..N {
        let v = rng.special_vec();
        a.check(format!("{i} special {v:?}"), |s| c2Norm(s, v));
    }
    for i in 0..N {
        let v = rng.any_vec();
        a.check(format!("{i} any {v:?}"), |s| c2Norm(s, v));
    }
    a.finish();
}

// --- row 12 --------------------------------------------------------------
#[test]
fn cfg_leaf_dist() {
    let mut a = DiffAccum::new("cfg_leaf_dist");
    let mut rng = Rng::new(0x5eed_000c);
    for i in 0..N {
        let h = c2h {
            n: rng.vec(),
            d: rng.coord(),
        };
        let p = rng.vec();
        a.check(format!("{i} plain"), |s| c2Dist(s, h, p));
    }
    for i in 0..N {
        let h = c2h {
            n: rng.special_vec(),
            d: rng.special(),
        };
        let p = rng.special_vec();
        a.check(format!("{i} special {h:?} {p:?}"), |s| c2Dist(s, h, p));
    }
    for i in 0..N {
        let h = c2h {
            n: rng.any_vec(),
            d: rng.any_f32(),
        };
        let p = rng.any_vec();
        a.check(format!("{i} any {h:?} {p:?}"), |s| c2Dist(s, h, p));
    }
    a.finish();
}

// --- row 13 --------------------------------------------------------------
#[test]
fn cfg_leaf_intersect() {
    let mut a = DiffAccum::new("cfg_leaf_intersect");
    let mut rng = Rng::new(0x5eed_000d);
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        let (da, db) = (rng.coord(), rng.coord());
        a.check(format!("{i} plain da={da:?} db={db:?}"), |s| {
            c2Intersect(s, u, v, da, db)
        });
    }
    // degenerate da == db (incl. both zero)
    for i in 0..N {
        let (u, v) = (rng.vec(), rng.vec());
        let da = rng.coord();
        a.check(format!("{i} da==db={da:?}"), |s| c2Intersect(s, u, v, da, da));
    }
    for i in 0..N {
        let (u, v) = (rng.special_vec(), rng.special_vec());
        let (da, db) = (rng.special(), rng.special());
        a.check(format!("{i} special"), |s| c2Intersect(s, u, v, da, db));
    }
    for i in 0..N {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        let (da, db) = (rng.any_f32(), rng.any_f32());
        a.check(format!("{i} any"), |s| c2Intersect(s, u, v, da, db));
    }
    a.finish();
}

// --- row 14 --------------------------------------------------------------
#[test]
fn cfg_leaf_identity() {
    diff("c2RotIdentity", |s| c2RotIdentity(s));
    diff("c2xIdentity", |s| c2xIdentity(s));
}

// --- rows 15..18 ---------------------------------------------------------
#[test]
fn cfg_xform_rot() {
    let mut a = DiffAccum::new("cfg_xform_rot");
    let mut rng = Rng::new(0x5eed_000e);
    let ident = c2r { c: 1.0, s: 0.0 };
    for i in 0..N {
        let v = rng.vec();
        a.check(format!("{i} ident mulrv"), |s| c2Mulrv(s, ident, v));
        a.check(format!("{i} ident mulrvT"), |s| c2MulrvT(s, ident, v));
        let r = rng.rot();
        a.check(format!("{i} rand mulrv"), |s| c2Mulrv(s, r, v));
        a.check(format!("{i} rand mulrvT"), |s| c2MulrvT(s, r, v));
    }
    for i in 0..N {
        let r = c2r {
            c: rng.special(),
            s: rng.special(),
        };
        let v = rng.special_vec();
        a.check(format!("{i} special mulrv {r:?} {v:?}"), |s| c2Mulrv(s, r, v));
        a.check(format!("{i} special mulrvT {r:?} {v:?}"), |s| {
            c2MulrvT(s, r, v)
        });
    }
    for i in 0..N {
        let r = c2r {
            c: rng.any_f32(),
            s: rng.any_f32(),
        };
        let v = rng.any_vec();
        a.check(format!("{i} any mulrv {r:?} {v:?}"), |s| c2Mulrv(s, r, v));
        a.check(format!("{i} any mulrvT {r:?} {v:?}"), |s| c2MulrvT(s, r, v));
    }
    a.finish();
}

// --- rows 19..21 ---------------------------------------------------------
#[test]
fn cfg_xform_x() {
    let mut a = DiffAccum::new("cfg_xform_x");
    let mut rng = Rng::new(0x5eed_000f);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    for i in 0..N {
        let v = rng.vec();
        a.check(format!("{i} ident mulxv"), |s| c2Mulxv(s, ident, v));
        a.check(format!("{i} ident mulxvT"), |s| c2MulxvT(s, ident, v));
        // translation only
        let t = c2x {
            p: rng.vec(),
            r: c2r { c: 1.0, s: 0.0 },
        };
        a.check(format!("{i} trans mulxv"), |s| c2Mulxv(s, t, v));
        a.check(format!("{i} trans mulxvT"), |s| c2MulxvT(s, t, v));
        // rotation only
        let r = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: rng.rot(),
        };
        a.check(format!("{i} rot mulxv"), |s| c2Mulxv(s, r, v));
        a.check(format!("{i} rot mulxvT"), |s| c2MulxvT(s, r, v));
        // both
        let x = rng.xform();
        a.check(format!("{i} both mulxv"), |s| c2Mulxv(s, x, v));
        a.check(format!("{i} both mulxvT"), |s| c2MulxvT(s, x, v));
    }
    for i in 0..N {
        let x = c2x {
            p: rng.special_vec(),
            r: c2r {
                c: rng.special(),
                s: rng.special(),
            },
        };
        let v = rng.special_vec();
        a.check(format!("{i} special mulxv {x:?} {v:?}"), |s| c2Mulxv(s, x, v));
        a.check(format!("{i} special mulxvT {x:?} {v:?}"), |s| {
            c2MulxvT(s, x, v)
        });
    }
    for i in 0..N {
        let x = c2x {
            p: rng.any_vec(),
            r: c2r {
                c: rng.any_f32(),
                s: rng.any_f32(),
            },
        };
        let v = rng.any_vec();
        a.check(format!("{i} any mulxv {x:?} {v:?}"), |s| c2Mulxv(s, x, v));
        a.check(format!("{i} any mulxvT {x:?} {v:?}"), |s| c2MulxvT(s, x, v));
    }
    a.finish();
}
