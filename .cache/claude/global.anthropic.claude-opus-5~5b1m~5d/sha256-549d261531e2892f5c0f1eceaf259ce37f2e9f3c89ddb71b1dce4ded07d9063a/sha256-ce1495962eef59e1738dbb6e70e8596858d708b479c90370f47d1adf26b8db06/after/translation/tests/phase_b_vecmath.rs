//! Phase B — valid-path differential tests for the leaf vector / transform math.
//!
//! Covers `CONFIGS.md` rows 1–18. Every call goes through `dlsym`'d function
//! pointers on both shared objects, so the struct-passing and struct-returning
//! ABI of the `#[no_mangle]` wrappers is exercised too.
//!
//! All comparisons are on raw `f32` bit patterns, so `+0.0` vs `-0.0` and
//! distinct NaN encodings are treated as divergences.

#![allow(non_snake_case)]

mod common;

use common::*;

const N: usize = 4096;

// ---------------------------------------------------------------------------
// Row 1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn row01_c2V() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        diff_eq!(
            format!("row01 c2V #{i} x={} y={}", show(x), show(y)),
            vb((l.c.c2V)(x, y)),
            vb((l.rs.c2V)(x, y))
        );
    }
    // Exhaustive special × special.
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            diff_eq!(
                format!("row01 c2V special x={} y={}", show(x), show(y)),
                vb((l.c.c2V)(x, y)),
                vb((l.rs.c2V)(x, y))
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2, 3 — c2Dot
// ---------------------------------------------------------------------------

#[test]
fn row02_c2Dot_finite() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let (a, b) = (rng.vec_sym(1.0e3), rng.vec_sym(1.0e3));
        diff_eq!(
            format!("row02 c2Dot #{i} a={} b={}", showv(a), showv(b)),
            fb((l.c.c2Dot)(a, b)),
            fb((l.rs.c2Dot)(a, b))
        );
    }
    for i in 0..N {
        let (a, b) = (rng.vec_grid(12), rng.vec_grid(12));
        diff_eq!(
            format!("row02 c2Dot grid #{i} a={} b={}", showv(a), showv(b)),
            fb((l.c.c2Dot)(a, b)),
            fb((l.rs.c2Dot)(a, b))
        );
    }
}

#[test]
fn row03_c2Dot_special() {
    let l = libs();
    let sp = special_wide();
    // Exhaustive over all 4 lanes would be sp^4; sample the cross-product of
    // (x-pair) × (y-pair) which is what the two mul/add actually see.
    for &ax in &sp {
        for &bx in &sp {
            for &ay in &sp {
                for by in [0.0f32, 1.0, -1.0, f32::INFINITY, f32::from_bits(0x7FC0_0000)] {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    diff_eq!(
                        format!("row03 c2Dot a={} b={}", showv(a), showv(b)),
                        fb((l.c.c2Dot)(a, b)),
                        fb((l.rs.c2Dot)(a, b))
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let (a, b) = (rng.any_vec(), rng.any_vec());
        diff_eq!(
            format!("row03 c2Dot rand #{i} a={} b={}", showv(a), showv(b)),
            fb((l.c.c2Dot)(a, b)),
            fb((l.rs.c2Dot)(a, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 4, 5 — c2Len
// ---------------------------------------------------------------------------

#[test]
fn row04_c2Len_finite() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..N {
        let a = rng.vec_sym(1.0e3);
        diff_eq!(
            format!("row04 c2Len #{i} a={}", showv(a)),
            fb((l.c.c2Len)(a)),
            fb((l.rs.c2Len)(a))
        );
    }
    for i in 0..N {
        let a = rng.vec_grid(10);
        diff_eq!(
            format!("row04 c2Len grid #{i} a={}", showv(a)),
            fb((l.c.c2Len)(a)),
            fb((l.rs.c2Len)(a))
        );
    }
}

#[test]
fn row05_c2Len_special() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            diff_eq!(
                format!("row05 c2Len a={}", showv(a)),
                fb((l.c.c2Len)(a)),
                fb((l.rs.c2Len)(a))
            );
        }
    }
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let a = rng.any_vec();
        diff_eq!(
            format!("row05 c2Len rand #{i} a={}", showv(a)),
            fb((l.c.c2Len)(a)),
            fb((l.rs.c2Len)(a))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 6 — c2Add / c2Sub
// ---------------------------------------------------------------------------

#[test]
fn row06_c2Add_c2Sub() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let (a, b) = (rng.vec_sym(1.0e3), rng.vec_sym(1.0e3));
        diff_eq!(
            format!("row06 c2Add #{i}"),
            vb((l.c.c2Add)(a, b)),
            vb((l.rs.c2Add)(a, b))
        );
        diff_eq!(
            format!("row06 c2Sub #{i}"),
            vb((l.c.c2Sub)(a, b)),
            vb((l.rs.c2Sub)(a, b))
        );
    }
    let sp = special_wide();
    for &ax in &sp {
        for &bx in &sp {
            for ay in [0.0f32, -0.0, 1.0, f32::INFINITY, f32::NEG_INFINITY] {
                for by in [0.0f32, -0.0, -1.0, f32::INFINITY] {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    diff_eq!(
                        format!("row06 c2Add sp a={} b={}", showv(a), showv(b)),
                        vb((l.c.c2Add)(a, b)),
                        vb((l.rs.c2Add)(a, b))
                    );
                    diff_eq!(
                        format!("row06 c2Sub sp a={} b={}", showv(a), showv(b)),
                        vb((l.c.c2Sub)(a, b)),
                        vb((l.rs.c2Sub)(a, b))
                    );
                }
            }
        }
    }
    for i in 0..N {
        let (a, b) = (rng.any_vec(), rng.any_vec());
        diff_eq!(
            format!("row06 c2Add rand #{i}"),
            vb((l.c.c2Add)(a, b)),
            vb((l.rs.c2Add)(a, b))
        );
        diff_eq!(
            format!("row06 c2Sub rand #{i}"),
            vb((l.c.c2Sub)(a, b)),
            vb((l.rs.c2Sub)(a, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 7 — c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn row07_c2Mulvs() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let a = rng.vec_sym(1.0e3);
        let s = rng.sym(100.0);
        diff_eq!(
            format!("row07 c2Mulvs #{i} a={} s={}", showv(a), show(s)),
            vb((l.c.c2Mulvs)(a, s)),
            vb((l.rs.c2Mulvs)(a, s))
        );
    }
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            for &s in &sp {
                let a = c2v { x, y };
                diff_eq!(
                    format!("row07 c2Mulvs sp a={} s={}", showv(a), show(s)),
                    vb((l.c.c2Mulvs)(a, s)),
                    vb((l.rs.c2Mulvs)(a, s))
                );
            }
        }
    }
    for i in 0..N {
        let a = rng.any_vec();
        let s = rng.any_f32();
        diff_eq!(
            format!("row07 c2Mulvs rand #{i}"),
            vb((l.c.c2Mulvs)(a, s)),
            vb((l.rs.c2Mulvs)(a, s))
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 8, 9 — c2Div  (must be `a * (1.0f/b)`, NOT `a / b`)
// ---------------------------------------------------------------------------

#[test]
fn row08_c2Div_nonzero() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    let mut differs_from_true_division = 0usize;
    for i in 0..(N * 4) {
        let a = rng.vec_sym(1.0e3);
        let mut b = rng.sym(100.0);
        if b == 0.0 {
            b = 3.0;
        }
        let cv = (l.c.c2Div)(a, b);
        let rv = (l.rs.c2Div)(a, b);
        diff_eq!(
            format!("row08 c2Div #{i} a={} b={}", showv(a), show(b)),
            vb(cv),
            vb(rv)
        );
        // Sanity: confirm the reciprocal form really is observably different
        // from true division, i.e. this row has teeth.
        if cv.x.to_bits() != (a.x / b).to_bits() {
            differs_from_true_division += 1;
        }
    }
    assert!(
        differs_from_true_division > 0,
        "expected `a*(1/b)` to differ from `a/b` for at least one input"
    );
}

#[test]
fn row09_c2Div_special_divisors() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            for &b in &sp {
                let a = c2v { x, y };
                diff_eq!(
                    format!("row09 c2Div sp a={} b={}", showv(a), show(b)),
                    vb((l.c.c2Div)(a, b)),
                    vb((l.rs.c2Div)(a, b))
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let a = rng.any_vec();
        let b = rng.any_f32();
        diff_eq!(
            format!("row09 c2Div rand #{i} a={} b={}", showv(a), show(b)),
            vb((l.c.c2Div)(a, b)),
            vb((l.rs.c2Div)(a, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 10, 11 — c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row10_c2Norm_finite() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let a = rng.vec_sym(1.0e3);
        diff_eq!(
            format!("row10 c2Norm #{i} a={}", showv(a)),
            vb((l.c.c2Norm)(a)),
            vb((l.rs.c2Norm)(a))
        );
    }
    for i in 0..N {
        let a = rng.dir(); // already unit
        diff_eq!(
            format!("row10 c2Norm unit #{i} a={}", showv(a)),
            vb((l.c.c2Norm)(a)),
            vb((l.rs.c2Norm)(a))
        );
    }
    for i in 0..N {
        let a = rng.vec_grid(10);
        diff_eq!(
            format!("row10 c2Norm grid #{i} a={}", showv(a)),
            vb((l.c.c2Norm)(a)),
            vb((l.rs.c2Norm)(a))
        );
    }
}

#[test]
fn row11_c2Norm_degenerate() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            diff_eq!(
                format!("row11 c2Norm sp a={}", showv(a)),
                vb((l.c.c2Norm)(a)),
                vb((l.rs.c2Norm)(a))
            );
        }
    }
    // The zero vector explicitly: 0 * (1/0) == NaN.
    let z = c2v { x: 0.0, y: 0.0 };
    diff_eq!("row11 c2Norm zero", vb((l.c.c2Norm)(z)), vb((l.rs.c2Norm)(z)));
    let nz = c2v { x: -0.0, y: -0.0 };
    diff_eq!(
        "row11 c2Norm negzero",
        vb((l.c.c2Norm)(nz)),
        vb((l.rs.c2Norm)(nz))
    );
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..N {
        let a = rng.any_vec();
        diff_eq!(
            format!("row11 c2Norm rand #{i} a={}", showv(a)),
            vb((l.c.c2Norm)(a)),
            vb((l.rs.c2Norm)(a))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — c2Minv / c2Maxv  (ternary semantics, asymmetric for NaN)
// ---------------------------------------------------------------------------

#[test]
fn row12_c2Minv_c2Maxv() {
    let l = libs();
    let sp = special_wide();
    for &ax in &sp {
        for &bx in &sp {
            for &ay in &sp {
                for &by in &sp {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    diff_eq!(
                        format!("row12 c2Minv a={} b={}", showv(a), showv(b)),
                        vb((l.c.c2Minv)(a, b)),
                        vb((l.rs.c2Minv)(a, b))
                    );
                    diff_eq!(
                        format!("row12 c2Maxv a={} b={}", showv(a), showv(b)),
                        vb((l.c.c2Maxv)(a, b)),
                        vb((l.rs.c2Maxv)(a, b))
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..(N * 2) {
        let (a, b) = (rng.any_vec(), rng.any_vec());
        diff_eq!(
            format!("row12 c2Minv rand #{i} a={} b={}", showv(a), showv(b)),
            vb((l.c.c2Minv)(a, b)),
            vb((l.rs.c2Minv)(a, b))
        );
        diff_eq!(
            format!("row12 c2Maxv rand #{i} a={} b={}", showv(a), showv(b)),
            vb((l.c.c2Maxv)(a, b)),
            vb((l.rs.c2Maxv)(a, b))
        );
    }
    // Explicit NaN asymmetry probes: the C ternary is NOT fminf/fmaxf.
    let nan = f32::from_bits(0x7FC0_0000);
    for &(ax, bx) in &[(nan, 1.0f32), (1.0f32, nan), (0.0f32, -0.0f32), (-0.0f32, 0.0f32)] {
        let a = c2v { x: ax, y: ax };
        let b = c2v { x: bx, y: bx };
        diff_eq!(
            format!("row12 asym min {} {}", show(ax), show(bx)),
            vb((l.c.c2Minv)(a, b)),
            vb((l.rs.c2Minv)(a, b))
        );
        diff_eq!(
            format!("row12 asym max {} {}", show(ax), show(bx)),
            vb((l.c.c2Maxv)(a, b)),
            vb((l.rs.c2Maxv)(a, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — c2Skew / c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn row13_c2Skew_c2CCW90() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            diff_eq!(
                format!("row13 c2Skew a={}", showv(a)),
                vb((l.c.c2Skew)(a)),
                vb((l.rs.c2Skew)(a))
            );
            diff_eq!(
                format!("row13 c2CCW90 a={}", showv(a)),
                vb((l.c.c2CCW90)(a)),
                vb((l.rs.c2CCW90)(a))
            );
        }
    }
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..(N * 2) {
        let a = rng.any_vec();
        diff_eq!(
            format!("row13 c2Skew rand #{i}"),
            vb((l.c.c2Skew)(a)),
            vb((l.rs.c2Skew)(a))
        );
        diff_eq!(
            format!("row13 c2CCW90 rand #{i}"),
            vb((l.c.c2CCW90)(a)),
            vb((l.rs.c2CCW90)(a))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 14 — c2Absv  (`x<0 ? -x : x`, NOT fabsf)
// ---------------------------------------------------------------------------

#[test]
fn row14_c2Absv() {
    let l = libs();
    let sp = special_wide();
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            diff_eq!(
                format!("row14 c2Absv a={}", showv(a)),
                vb((l.c.c2Absv)(a)),
                vb((l.rs.c2Absv)(a))
            );
        }
    }
    // -0.0 must stay -0.0 (fabsf would give +0.0); -NaN must keep its sign.
    for &x in &[
        -0.0f32,
        0.0f32,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0000),
        f32::NEG_INFINITY,
    ] {
        let a = c2v { x, y: x };
        diff_eq!(
            format!("row14 c2Absv edge {}", show(x)),
            vb((l.c.c2Absv)(a)),
            vb((l.rs.c2Absv)(a))
        );
    }
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..(N * 2) {
        let a = rng.any_vec();
        diff_eq!(
            format!("row14 c2Absv rand #{i} a={}", showv(a)),
            vb((l.c.c2Absv)(a)),
            vb((l.rs.c2Absv)(a))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 — c2RotIdentity / c2xIdentity (struct-return ABI, 8 and 16 bytes)
// ---------------------------------------------------------------------------

#[test]
fn row15_identities() {
    let l = libs();
    for _ in 0..64 {
        diff_eq!(
            "row15 c2RotIdentity",
            rb((l.c.c2RotIdentity)()),
            rb((l.rs.c2RotIdentity)())
        );
        diff_eq!(
            "row15 c2xIdentity",
            xb((l.c.c2xIdentity)()),
            xb((l.rs.c2xIdentity)())
        );
    }
    // And the documented values.
    let r = (l.rs.c2RotIdentity)();
    assert_eq!((fb(r.c), fb(r.s)), (fb(1.0), fb(0.0)));
    let x = (l.rs.c2xIdentity)();
    assert_eq!(xb(x), (fb(0.0), fb(0.0), fb(1.0), fb(0.0)));
}

// ---------------------------------------------------------------------------
// Row 16 — c2Mulrv / c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn row16_c2Mulrv_c2MulrvT() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);

    // 64 exact angles × random vectors.
    for k in 0..64u32 {
        let a = std::f32::consts::TAU * (k as f32) / 64.0;
        let r = c2r {
            c: a.cos(),
            s: a.sin(),
        };
        for i in 0..64 {
            let b = rng.vec_sym(100.0);
            diff_eq!(
                format!("row16 c2Mulrv angle{k} #{i}"),
                vb((l.c.c2Mulrv)(r, b)),
                vb((l.rs.c2Mulrv)(r, b))
            );
            diff_eq!(
                format!("row16 c2MulrvT angle{k} #{i}"),
                vb((l.c.c2MulrvT)(r, b)),
                vb((l.rs.c2MulrvT)(r, b))
            );
        }
    }

    // Identity, zero, non-normalised, and structured special rotations.
    let rots = [
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: 2.0, s: -3.0 },
        c2r { c: -0.0, s: -0.0 },
    ];
    let sp = special_wide();
    for r in rots {
        for &x in &sp {
            for &y in &sp {
                let b = c2v { x, y };
                diff_eq!(
                    format!("row16 c2Mulrv sp r=({},{}) b={}", show(r.c), show(r.s), showv(b)),
                    vb((l.c.c2Mulrv)(r, b)),
                    vb((l.rs.c2Mulrv)(r, b))
                );
                diff_eq!(
                    format!("row16 c2MulrvT sp r=({},{}) b={}", show(r.c), show(r.s), showv(b)),
                    vb((l.c.c2MulrvT)(r, b)),
                    vb((l.rs.c2MulrvT)(r, b))
                );
            }
        }
    }

    // Fully random, compared strictly on every lane. NaN payload selection is
    // reproduced exactly (see the `addss`/`mulss` helpers in src/lib.rs), so no
    // lane is exempt.
    for i in 0..(N * 8) {
        let r = rng.any_rot();
        let b = rng.any_vec();
        diff_eq!(
            format!("row16 c2Mulrv rand #{i} r=({},{}) b={}", show(r.c), show(r.s), showv(b)),
            vb((l.c.c2Mulrv)(r, b)),
            vb((l.rs.c2Mulrv)(r, b))
        );
        diff_eq!(
            format!("row16 c2MulrvT rand #{i} r=({},{}) b={}", show(r.c), show(r.s), showv(b)),
            vb((l.c.c2MulrvT)(r, b)),
            vb((l.rs.c2MulrvT)(r, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 17 — c2MulmvT
// ---------------------------------------------------------------------------

#[test]
fn row17_c2MulmvT() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 17);

    // Orthonormal frames exactly as c2RaytoCapsule builds them: M.y = norm(v),
    // M.x = c2CCW90(M.y).
    for i in 0..N {
        let y = (l.rs.c2Norm)(rng.vec_sym(10.0));
        let x = (l.rs.c2CCW90)(y);
        let m = c2m { x, y };
        let b = rng.vec_sym(100.0);
        diff_eq!(
            format!("row17 c2MulmvT ortho #{i}"),
            vb((l.c.c2MulmvT)(m, b)),
            vb((l.rs.c2MulmvT)(m, b))
        );
    }

    let frames = [
        c2m {
            x: c2v { x: 1.0, y: 0.0 },
            y: c2v { x: 0.0, y: 1.0 },
        },
        c2m {
            x: c2v { x: 0.0, y: 0.0 },
            y: c2v { x: 0.0, y: 0.0 },
        },
        c2m {
            x: c2v { x: 1.0, y: 1.0 },
            y: c2v { x: 1.0, y: 1.0 },
        }, // singular
        c2m {
            x: c2v { x: -0.0, y: -0.0 },
            y: c2v { x: -0.0, y: -0.0 },
        },
    ];
    let sp = special_wide();
    for m in frames {
        for &x in &sp {
            for &y in &sp {
                let b = c2v { x, y };
                diff_eq!(
                    format!("row17 c2MulmvT frame b={}", showv(b)),
                    vb((l.c.c2MulmvT)(m, b)),
                    vb((l.rs.c2MulmvT)(m, b))
                );
            }
        }
    }

    for i in 0..(N * 4) {
        let m = c2m {
            x: rng.any_vec(),
            y: rng.any_vec(),
        };
        let b = rng.any_vec();
        diff_eq!(
            format!(
                "row17 c2MulmvT rand #{i} m=({},{}) b={}",
                showv(m.x),
                showv(m.y),
                showv(b)
            ),
            vb((l.c.c2MulmvT)(m, b)),
            vb((l.rs.c2MulmvT)(m, b))
        );
    }
}

// ---------------------------------------------------------------------------
// Row 18 — c2MulxvT (16-byte struct argument)
// ---------------------------------------------------------------------------

#[test]
fn row18_c2MulxvT() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);

    let ident = (l.rs.c2xIdentity)();
    for i in 0..N {
        let b = rng.vec_sym(100.0);
        diff_eq!(
            format!("row18 c2MulxvT identity #{i}"),
            vb((l.c.c2MulxvT)(ident, b)),
            vb((l.rs.c2MulxvT)(ident, b))
        );
    }
    // Pure translation.
    for i in 0..N {
        let x = c2x {
            p: rng.vec_sym(50.0),
            r: c2r { c: 1.0, s: 0.0 },
        };
        let b = rng.vec_sym(100.0);
        diff_eq!(
            format!("row18 c2MulxvT translate #{i}"),
            vb((l.c.c2MulxvT)(x, b)),
            vb((l.rs.c2MulxvT)(x, b))
        );
    }
    // Pure rotation, 64 angles.
    for k in 0..64u32 {
        let a = std::f32::consts::TAU * (k as f32) / 64.0;
        let x = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r {
                c: a.cos(),
                s: a.sin(),
            },
        };
        for i in 0..32 {
            let b = rng.vec_sym(100.0);
            diff_eq!(
                format!("row18 c2MulxvT rot{k} #{i}"),
                vb((l.c.c2MulxvT)(x, b)),
                vb((l.rs.c2MulxvT)(x, b))
            );
        }
    }
    // Translation + rotation, and non-unit / zero c2r.
    for i in 0..(N * 4) {
        let x = rng.any_x();
        let b = rng.any_vec();
        diff_eq!(
            format!("row18 c2MulxvT rand #{i} x={:?} b={}", xb(x), showv(b)),
            vb((l.c.c2MulxvT)(x, b)),
            vb((l.rs.c2MulxvT)(x, b))
        );
    }
    // Structured special sweep with a finite transform.
    let sp = special_wide();
    let x = c2x {
        p: c2v { x: 3.0, y: -0.0 },
        r: c2r { c: 0.5, s: -0.25 },
    };
    for &bx in &sp {
        for &by in &sp {
            let b = c2v { x: bx, y: by };
            diff_eq!(
                format!("row18 c2MulxvT sp b={}", showv(b)),
                vb((l.c.c2MulxvT)(x, b)),
                vb((l.rs.c2MulxvT)(x, b))
            );
        }
    }
    // And with fully arbitrary transforms crossed against the special vectors.
    for _ in 0..64 {
        let x = rng.any_x();
        for &bx in &sp {
            for &by in &sp {
                let b = c2v { x: bx, y: by };
                diff_eq!(
                    format!("row18 c2MulxvT wild x={:?} b={}", xb(x), showv(b)),
                    vb((l.c.c2MulxvT)(x, b)),
                    vb((l.rs.c2MulxvT)(x, b))
                );
            }
        }
    }
}
