//! Phase B — rows 1..19: level-0 vector / rotation / transform primitives and
//! the proxy vertex builder. Everything is called through both `.so`s.

mod common;

use common::*;

const N: usize = 4000;

// ---- rows 1 & 2: c2V, c2Neg, c2Skew, c2CCW90 ------------------------------

#[test]
fn cfg01_02_unary_vector_ops() {
    let (c_v, r_v) = pair::<extern "C" fn(f32, f32) -> c2v>("c2V");
    let ops: [&str; 3] = ["c2Neg", "c2Skew", "c2CCW90"];
    let mut rng = Rng::new(0x1111);

    for spicy in [false, true] {
        for _ in 0..N {
            let (x, y) = if spicy {
                (rng.spicy_f32(), rng.spicy_f32())
            } else {
                (rng.coord(), rng.coord())
            };
            assert_same("c2V", c_v(x, y), r_v(x, y));
            let v = c2v { x, y };
            for name in ops {
                let (cf, rf) = pair::<FnVV>(name);
                assert_same(name, cf(v), rf(v));
            }
        }
    }
}

// ---- rows 3 & 4: c2Add, c2Sub, c2Mulvs -----------------------------------

#[test]
fn cfg03_04_binary_vector_ops() {
    let (c_add, r_add) = pair::<FnVVV>("c2Add");
    let (c_sub, r_sub) = pair::<FnVVV>("c2Sub");
    let (c_mul, r_mul) = pair::<extern "C" fn(c2v, f32) -> c2v>("c2Mulvs");
    let mut rng = Rng::new(0x2222);

    for spicy in [false, true] {
        for _ in 0..N {
            let (a, b, s) = if spicy {
                (rng.spicy_vec(), rng.spicy_vec(), rng.spicy_f32())
            } else {
                (rng.vec(), rng.vec(), rng.range(-8.0, 8.0))
            };
            assert_same("c2Add", c_add(a, b), r_add(a, b));
            assert_same("c2Sub", c_sub(a, b), r_sub(a, b));
            assert_same("c2Mulvs", c_mul(a, s), r_mul(a, s));
        }
    }
    // Explicit overflow / underflow magnitudes.
    for (a, b) in [
        (c2v { x: 1e30, y: 1e30 }, c2v { x: 1e30, y: 1e30 }),
        (c2v { x: 1e-30, y: 1e-30 }, c2v { x: 1e-30, y: 1e-30 }),
        (
            c2v {
                x: f32::MAX,
                y: f32::MIN,
            },
            c2v {
                x: -f32::MAX,
                y: f32::MAX,
            },
        ),
    ] {
        assert_same("c2Add extreme", c_add(a, b), r_add(a, b));
        assert_same("c2Sub extreme", c_sub(a, b), r_sub(a, b));
        for s in [1e30f32, 1e-30, f32::MAX, 0.0, -0.0, f32::INFINITY] {
            assert_same("c2Mulvs extreme", c_mul(a, s), r_mul(a, s));
        }
    }
}

// ---- rows 5 & 6: c2Dot, c2Det2 -------------------------------------------

#[test]
fn cfg05_06_dot_det() {
    let (c_dot, r_dot) = pair::<FnVVf>("c2Dot");
    let (c_det, r_det) = pair::<FnVVf>("c2Det2");
    let mut rng = Rng::new(0x3333);

    for spicy in [false, true] {
        for _ in 0..N {
            let (a, b) = if spicy {
                (rng.spicy_vec(), rng.spicy_vec())
            } else {
                (rng.vec(), rng.vec())
            };
            assert_same("c2Dot", c_dot(a, b), r_dot(a, b));
            assert_same("c2Det2", c_det(a, b), r_det(a, b));
        }
    }
    // Catastrophic-cancellation inputs (these are where an FMA contraction in
    // one implementation but not the other would show up).
    let cases: [(c2v, c2v); 4] = [
        (
            c2v { x: 1.0, y: 1.0 },
            c2v {
                x: 1.0 + f32::EPSILON,
                y: -1.0,
            },
        ),
        (
            c2v { x: 1e20, y: 1e20 },
            c2v { x: 1e-20, y: -1e-20 },
        ),
        (
            c2v {
                x: 16777217.0,
                y: 1.0,
            },
            c2v { x: 1.0, y: -16777216.0 },
        ),
        (
            c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
            c2v { x: 0.0, y: 1.0 },
        ),
    ];
    for (a, b) in cases {
        assert_same("c2Dot cancel", c_dot(a, b), r_dot(a, b));
        assert_same("c2Det2 cancel", c_det(a, b), r_det(a, b));
    }
}

// ---- rows 7 & 8: c2Len ---------------------------------------------------

#[test]
fn cfg07_08_len() {
    let (cf, rf) = pair::<FnVf>("c2Len");
    let mut rng = Rng::new(0x4444);
    for _ in 0..N {
        let v = rng.vec();
        assert_same("c2Len", cf(v), rf(v));
    }
    for _ in 0..N {
        let v = rng.spicy_vec();
        assert_same("c2Len spicy", cf(v), rf(v));
    }
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1e-30, y: 1e-30 },
        c2v { x: 1e30, y: 1e30 },
        c2v {
            x: f32::NAN,
            y: 0.0,
        },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        c2v {
            x: f32::MIN_POSITIVE,
            y: 0.0,
        },
    ] {
        assert_same("c2Len edge", cf(v), rf(v));
    }
}

// ---- rows 9 & 10: c2Div, c2Norm -----------------------------------------

#[test]
fn cfg09_10_div_norm() {
    let (c_div, r_div) = pair::<extern "C" fn(c2v, f32) -> c2v>("c2Div");
    let (c_norm, r_norm) = pair::<FnVV>("c2Norm");
    let mut rng = Rng::new(0x5555);

    for _ in 0..N {
        let v = rng.vec();
        let d = rng.range(-9.0, 9.0);
        assert_same("c2Div", c_div(v, d), r_div(v, d));
        assert_same("c2Norm", c_norm(v), r_norm(v));
    }
    for _ in 0..N {
        let v = rng.spicy_vec();
        let d = rng.spicy_f32();
        assert_same("c2Div spicy", c_div(v, d), r_div(v, d));
        assert_same("c2Norm spicy", c_norm(v), r_norm(v));
    }
    for d in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 1e-45] {
        let v = c2v { x: 3.0, y: -4.0 };
        assert_same("c2Div zero", c_div(v, d), r_div(v, d));
    }
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v {
            x: f32::NAN,
            y: 1.0,
        },
        c2v {
            x: f32::INFINITY,
            y: 1.0,
        },
    ] {
        assert_same("c2Norm zero", c_norm(v), r_norm(v));
    }
}

// ---- rows 11 & 12: c2Maxv, c2Minv, c2Clampv -----------------------------

#[test]
fn cfg11_12_minmax_clamp() {
    let (c_max, r_max) = pair::<FnVVV>("c2Maxv");
    let (c_min, r_min) = pair::<FnVVV>("c2Minv");
    let (c_cl, r_cl) = pair::<extern "C" fn(c2v, c2v, c2v) -> c2v>("c2Clampv");
    let mut rng = Rng::new(0x6666);

    for _ in 0..N {
        let a = rng.vec();
        let bb = rng.aabb();
        assert_same("c2Maxv", c_max(a, bb.min), r_max(a, bb.min));
        assert_same("c2Minv", c_min(a, bb.max), r_min(a, bb.max));
        assert_same(
            "c2Clampv",
            c_cl(a, bb.min, bb.max),
            r_cl(a, bb.min, bb.max),
        );
    }
    // NaN / signed-zero / inverted-range behaviour (ternary, not fmaxf).
    for _ in 0..N {
        let a = rng.spicy_vec();
        let lo = rng.spicy_vec();
        let hi = rng.spicy_vec();
        assert_same("c2Maxv spicy", c_max(a, lo), r_max(a, lo));
        assert_same("c2Minv spicy", c_min(a, hi), r_min(a, hi));
        assert_same("c2Clampv spicy", c_cl(a, lo, hi), r_cl(a, lo, hi));
    }
    let zeros = [
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (
            c2v {
                x: f32::NAN,
                y: 1.0,
            },
            c2v {
                x: 2.0,
                y: f32::NAN,
            },
        ),
    ];
    for (a, b) in zeros {
        assert_same("c2Maxv zero/nan", c_max(a, b), r_max(a, b));
        assert_same("c2Minv zero/nan", c_min(a, b), r_min(a, b));
        // Deliberately inverted: lo > hi.
        assert_same("c2Clampv inverted", c_cl(a, b, a), r_cl(a, b, a));
    }
}

// ---- row 13: c2RotIdentity, c2xIdentity ---------------------------------

#[test]
fn cfg13_identities() {
    let (c_ri, r_ri) = pair::<extern "C" fn() -> c2r>("c2RotIdentity");
    let (c_xi, r_xi) = pair::<extern "C" fn() -> c2x>("c2xIdentity");
    assert_same("c2RotIdentity", c_ri(), r_ri());
    assert_same("c2xIdentity", c_xi(), r_xi());
}

// ---- rows 14 & 15: c2Mulrv, c2MulrvT -----------------------------------

#[test]
fn cfg14_15_rotate() {
    let (c_m, r_m) = pair::<extern "C" fn(c2r, c2v) -> c2v>("c2Mulrv");
    let (c_t, r_t) = pair::<extern "C" fn(c2r, c2v) -> c2v>("c2MulrvT");
    let mut rng = Rng::new(0x7777);

    for _ in 0..N {
        let rot = rng.rot();
        let v = rng.vec();
        assert_same("c2Mulrv", c_m(rot, v), r_m(rot, v));
        assert_same("c2MulrvT", c_t(rot, v), r_t(rot, v));
    }
    for _ in 0..N {
        // Non-unit / non-finite c2r: the API never validates c*c + s*s == 1.
        let rot = c2r {
            c: rng.spicy_f32(),
            s: rng.spicy_f32(),
        };
        let v = rng.spicy_vec();
        assert_same("c2Mulrv nonunit", c_m(rot, v), r_m(rot, v));
        assert_same("c2MulrvT nonunit", c_t(rot, v), r_t(rot, v));
    }
}

// ---- rows 16 & 17: c2Mulxv ---------------------------------------------

#[test]
fn cfg16_17_mulxv() {
    let (cf, rf) = pair::<extern "C" fn(c2x, c2v) -> c2v>("c2Mulxv");
    let (_, r_xi) = pair::<extern "C" fn() -> c2x>("c2xIdentity");
    let ident = r_xi();
    let mut rng = Rng::new(0x8888);

    for _ in 0..N {
        let v = rng.vec();
        // identity
        assert_same("c2Mulxv identity", cf(ident, v), rf(ident, v));
        // translation only
        let t = c2x {
            p: rng.vec(),
            r: ident.r,
        };
        assert_same("c2Mulxv translate", cf(t, v), rf(t, v));
        // rotation only
        let ro = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: rng.rot(),
        };
        assert_same("c2Mulxv rotate", cf(ro, v), rf(ro, v));
        // both
        let x = rng.xform();
        assert_same("c2Mulxv rot+trans", cf(x, v), rf(x, v));
    }
    for _ in 0..N {
        let x = c2x {
            p: rng.spicy_vec(),
            r: c2r {
                c: rng.spicy_f32(),
                s: rng.spicy_f32(),
            },
        };
        let v = rng.spicy_vec();
        assert_same("c2Mulxv spicy", cf(x, v), rf(x, v));
    }
}

// ---- rows 18 & 19: c2BBVerts -------------------------------------------

#[test]
fn cfg18_19_bbverts() {
    let (cf, rf) = pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x9999);

    let run = |bb: c2AABB| {
        // 8 slots so an over-write past vert[3] would be caught.
        let mut cout = [c2v {
            x: f32::from_bits(0xA5A5_A5A5),
            y: f32::from_bits(0x5A5A_5A5A),
        }; 8];
        let mut rout = cout;
        let mut cbb = bb;
        let mut rbb = bb;
        unsafe {
            cf(cout.as_mut_ptr(), &raw mut cbb);
            rf(rout.as_mut_ptr(), &raw mut rbb);
        }
        assert_same("c2BBVerts out", cout, rout);
        assert_same("c2BBVerts input untouched", cbb, rbb);
    };

    for _ in 0..N {
        run(rng.aabb());
    }
    for _ in 0..N {
        // Arbitrary (often inverted / non-finite) boxes.
        run(c2AABB {
            min: rng.spicy_vec(),
            max: rng.spicy_vec(),
        });
    }
    let p = c2v { x: 7.0, y: -3.0 };
    run(c2AABB { min: p, max: p }); // zero extent
    run(c2AABB {
        min: c2v { x: 5.0, y: 5.0 },
        max: c2v { x: -5.0, y: -5.0 },
    }); // inverted
}
