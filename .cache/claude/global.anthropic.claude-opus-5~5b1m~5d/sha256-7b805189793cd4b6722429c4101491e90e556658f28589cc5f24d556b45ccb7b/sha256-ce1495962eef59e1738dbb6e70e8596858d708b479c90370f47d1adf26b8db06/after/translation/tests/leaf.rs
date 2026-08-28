//! Phase B — `CONFIGS.md` rows 1..15: the leaf arithmetic, called through both
//! `.so`s. These are the lowest-level entry points and everything else is built
//! out of them, so they are checked first and hardest.
//!
//! Each row runs `N` randomized inputs from the fixed seed, drawn from the
//! `A14` value-class mix (normals, subnormals, `±0`, `±inf`, `FLT_MAX`,
//! `FLT_EPSILON`, and quiet/signalling NaNs with random payloads and signs) —
//! plus an explicit NaN-placement matrix, because *which* NaN survives an
//! operation depends on which operand GCC put in the SSE destination register
//! and a single scalar input would never reveal that.

mod common;
use common::*;

const N: usize = 20_000;

/// The NaN-placement matrix: for a two-operand op, drive (clean, clean),
/// (NaN, clean), (clean, NaN) and (NaN1, NaN2) with *distinct* payloads.
fn nan_matrix(rng: &mut Rng) -> [(f32, f32); 6] {
    let a = rng.coord();
    let b = rng.coord();
    let n1 = rng.nan();
    let n2 = rng.nan();
    [
        (a, b),
        (n1, b),
        (a, n2),
        (n1, n2),
        (n2, n1),
        (f32::from_bits(0x7f80_0001), f32::from_bits(0xffbf_ffff)),
    ]
}

// ---------------------------------------------------------------------------
// Row 1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_c2v() {
    let (c, r) = both();
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        let cv = unsafe { (c.c2V)(x, y) };
        let rv = unsafe { (r.c2V)(x, y) };
        assert_bits_eq(&format!("c2V #{i} ({x:?},{y:?})"), &cv, &rv);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — c2Sub / c2Add
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_add_sub() {
    let (c, r) = both();
    let mut rng = Rng::new(2);
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        assert_bits_eq(
            &format!("c2Sub #{i} {a:?} {b:?}"),
            &unsafe { (c.c2Sub)(a, b) },
            &unsafe { (r.c2Sub)(a, b) },
        );
        assert_bits_eq(
            &format!("c2Add #{i} {a:?} {b:?}"),
            &unsafe { (c.c2Add)(a, b) },
            &unsafe { (r.c2Add)(a, b) },
        );
    }
    // NaN-placement matrix, per component, and the inf-inf / inf+-inf cases.
    let mut rng = Rng::new(0x2a);
    for i in 0..N {
        for (p, q) in nan_matrix(&mut rng) {
            for (a, b) in [
                (c2v { x: p, y: q }, c2v { x: q, y: p }),
                (
                    c2v {
                        x: f32::INFINITY,
                        y: f32::NEG_INFINITY,
                    },
                    c2v {
                        x: f32::INFINITY,
                        y: f32::INFINITY,
                    },
                ),
                (
                    c2v { x: 0.0, y: -0.0 },
                    c2v { x: -0.0, y: 0.0 },
                ),
            ] {
                assert_bits_eq(
                    &format!("c2Sub nan #{i} {a:?} {b:?}"),
                    &unsafe { (c.c2Sub)(a, b) },
                    &unsafe { (r.c2Sub)(a, b) },
                );
                assert_bits_eq(
                    &format!("c2Add nan #{i} {a:?} {b:?}"),
                    &unsafe { (c.c2Add)(a, b) },
                    &unsafe { (r.c2Add)(a, b) },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_mulvs() {
    let (c, r) = both();
    let mut rng = Rng::new(3);
    for i in 0..N {
        let a = rng.any_v();
        let s = rng.any_f32();
        assert_bits_eq(
            &format!("c2Mulvs #{i} {a:?} * {s:?}"),
            &unsafe { (c.c2Mulvs)(a, s) },
            &unsafe { (r.c2Mulvs)(a, s) },
        );
    }
    // NaN in the vector vs NaN in the scalar: which one wins pins the
    // destination operand of `mulss`.
    let mut rng = Rng::new(0x3a);
    for i in 0..N {
        for (p, q) in nan_matrix(&mut rng) {
            let a = c2v { x: p, y: p };
            assert_bits_eq(
                &format!("c2Mulvs nan #{i} {a:?} * {q:?}"),
                &unsafe { (c.c2Mulvs)(a, q) },
                &unsafe { (r.c2Mulvs)(a, q) },
            );
        }
        // 0 * inf in both orders
        for (v, s) in [
            (c2v { x: 0.0, y: -0.0 }, f32::INFINITY),
            (
                c2v {
                    x: f32::INFINITY,
                    y: f32::NEG_INFINITY,
                },
                0.0,
            ),
        ] {
            assert_bits_eq(
                &format!("c2Mulvs zeroinf #{i}"),
                &unsafe { (c.c2Mulvs)(v, s) },
                &unsafe { (r.c2Mulvs)(v, s) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 4, 5 — c2Dot / c2Det2
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_dot() {
    let (c, r) = both();
    let mut rng = Rng::new(4);
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        assert_f32_bits_eq(
            &format!("c2Dot #{i} {a:?}.{b:?}"),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
    // Independent NaN in each of the four scalar slots.
    let mut rng = Rng::new(0x4a);
    for i in 0..N {
        let clean = [rng.coord(), rng.coord(), rng.coord(), rng.coord()];
        for slot in 0..4 {
            let n = rng.nan();
            let mut v = clean;
            v[slot] = n;
            let a = c2v { x: v[0], y: v[1] };
            let b = c2v { x: v[2], y: v[3] };
            assert_f32_bits_eq(
                &format!("c2Dot nan slot{slot} #{i} {a:?}.{b:?}"),
                unsafe { (c.c2Dot)(a, b) },
                unsafe { (r.c2Dot)(a, b) },
            );
        }
        // Two different NaNs in the two products: only the destination
        // operand's payload survives the addss.
        let (n1, n2) = (rng.nan(), rng.nan());
        let a = c2v { x: n1, y: n2 };
        let b = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        assert_f32_bits_eq(
            &format!("c2Dot 2nan #{i}"),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
}

#[test]
fn cfg_leaf_det2() {
    let (c, r) = both();
    let mut rng = Rng::new(5);
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        assert_f32_bits_eq(
            &format!("c2Det2 #{i} {a:?} {b:?}"),
            unsafe { (c.c2Det2)(a, b) },
            unsafe { (r.c2Det2)(a, b) },
        );
    }
    let mut rng = Rng::new(0x5a);
    for i in 0..N {
        let clean = [rng.coord(), rng.coord(), rng.coord(), rng.coord()];
        for slot in 0..4 {
            let n = rng.nan();
            let mut v = clean;
            v[slot] = n;
            let a = c2v { x: v[0], y: v[1] };
            let b = c2v { x: v[2], y: v[3] };
            assert_f32_bits_eq(
                &format!("c2Det2 nan slot{slot} #{i}"),
                unsafe { (c.c2Det2)(a, b) },
                unsafe { (r.c2Det2)(a, b) },
            );
        }
        let (n1, n2) = (rng.nan(), rng.nan());
        let a = c2v { x: n1, y: n2 };
        let b = c2v { x: n2, y: n1 };
        assert_f32_bits_eq(
            &format!("c2Det2 2nan #{i}"),
            unsafe { (c.c2Det2)(a, b) },
            unsafe { (r.c2Det2)(a, b) },
        );
        // Exactly-zero determinant (collinear) and inf*0 in one term.
        let k = rng.coord();
        for (a, b) in [
            (c2v { x: k, y: k }, c2v { x: k, y: k }),
            (
                c2v {
                    x: f32::INFINITY,
                    y: 1.0,
                },
                c2v { x: 1.0, y: 0.0 },
            ),
        ] {
            assert_f32_bits_eq(
                &format!("c2Det2 degen #{i}"),
                unsafe { (c.c2Det2)(a, b) },
                unsafe { (r.c2Det2)(a, b) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — c2Len (the only libm call in the library: `sqrtf@plt`)
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_len() {
    let (c, r) = both();
    let mut rng = Rng::new(6);
    for i in 0..N {
        let a = rng.any_v();
        assert_f32_bits_eq(
            &format!("c2Len #{i} {a:?}"),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    // Exact squares, subnormals (whose square underflows to zero), values whose
    // square overflows to inf, and NaN payloads.
    let mut fixed: Vec<c2v> = vec![
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: -3.0, y: -4.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: FLT_MAX, y: FLT_MAX },
        c2v { x: FLT_MAX, y: 0.0 },
        c2v {
            x: f32::from_bits(1),
            y: f32::from_bits(1),
        },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        c2v {
            x: FLT_EPSILON,
            y: FLT_EPSILON,
        },
    ];
    let mut rng = Rng::new(0x6a);
    for _ in 0..2000 {
        let n = rng.nan();
        fixed.push(c2v { x: n, y: rng.coord() });
        fixed.push(c2v { x: rng.coord(), y: n });
        fixed.push(c2v { x: n, y: rng.nan() });
    }
    for (i, a) in fixed.iter().enumerate() {
        assert_f32_bits_eq(
            &format!("c2Len fixed #{i} {a:?}"),
            unsafe { (c.c2Len)(*a) },
            unsafe { (r.c2Len)(*a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 7, 8 — c2Maxv / c2Minv / c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_minmax() {
    let (c, r) = both();
    let mut rng = Rng::new(7);
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        assert_bits_eq(
            &format!("c2Maxv #{i} {a:?} {b:?}"),
            &unsafe { (c.c2Maxv)(a, b) },
            &unsafe { (r.c2Maxv)(a, b) },
        );
        assert_bits_eq(
            &format!("c2Minv #{i} {a:?} {b:?}"),
            &unsafe { (c.c2Minv)(a, b) },
            &unsafe { (r.c2Minv)(a, b) },
        );
    }
    // +0 vs -0 (compare equal, so the ternary's else-arm decides the sign of
    // the result), equal values, and NaN in either slot.
    let interesting = [0.0f32, -0.0f32, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY];
    let mut rng = Rng::new(0x7a);
    for &p in &interesting {
        for &q in &interesting {
            let a = c2v { x: p, y: q };
            let b = c2v { x: q, y: p };
            assert_bits_eq(
                &format!("c2Maxv zeros {p:?} {q:?}"),
                &unsafe { (c.c2Maxv)(a, b) },
                &unsafe { (r.c2Maxv)(a, b) },
            );
            assert_bits_eq(
                &format!("c2Minv zeros {p:?} {q:?}"),
                &unsafe { (c.c2Minv)(a, b) },
                &unsafe { (r.c2Minv)(a, b) },
            );
        }
    }
    for i in 0..N {
        for (p, q) in nan_matrix(&mut rng) {
            let a = c2v { x: p, y: q };
            let b = c2v { x: q, y: p };
            assert_bits_eq(
                &format!("c2Maxv nan #{i}"),
                &unsafe { (c.c2Maxv)(a, b) },
                &unsafe { (r.c2Maxv)(a, b) },
            );
            assert_bits_eq(
                &format!("c2Minv nan #{i}"),
                &unsafe { (c.c2Minv)(a, b) },
                &unsafe { (r.c2Minv)(a, b) },
            );
        }
    }
}

#[test]
fn cfg_leaf_clampv() {
    let (c, r) = both();
    let mut rng = Rng::new(8);
    for i in 0..N {
        // Both a sane range and an inverted one (lo > hi), which the C never
        // validates.
        let a = rng.any_v();
        let (lo, hi) = if rng.bool() {
            (rng.any_v(), rng.any_v())
        } else {
            let l = rng.v();
            (
                l,
                c2v {
                    x: l.x + rng.radius(),
                    y: l.y + rng.radius(),
                },
            )
        };
        assert_bits_eq(
            &format!("c2Clampv #{i} {a:?} lo={lo:?} hi={hi:?}"),
            &unsafe { (c.c2Clampv)(a, lo, hi) },
            &unsafe { (r.c2Clampv)(a, lo, hi) },
        );
    }
    // NaN in each of the three arguments independently.
    let mut rng = Rng::new(0x8a);
    for i in 0..N {
        let base = [rng.v(), rng.v(), rng.v()];
        for slot in 0..3 {
            let mut v = base;
            v[slot] = c2v {
                x: rng.nan(),
                y: rng.nan(),
            };
            assert_bits_eq(
                &format!("c2Clampv nan slot{slot} #{i}"),
                &unsafe { (c.c2Clampv)(v[0], v[1], v[2]) },
                &unsafe { (r.c2Clampv)(v[0], v[1], v[2]) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — c2Neg / c2Skew / c2CCW90 (pure `xorps` sign flips)
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_neg_skew() {
    let (c, r) = both();
    let mut rng = Rng::new(9);
    for i in 0..N {
        let a = rng.any_v();
        assert_bits_eq(
            &format!("c2Neg #{i} {a:?}"),
            &unsafe { (c.c2Neg)(a) },
            &unsafe { (r.c2Neg)(a) },
        );
        assert_bits_eq(
            &format!("c2Skew #{i} {a:?}"),
            &unsafe { (c.c2Skew)(a) },
            &unsafe { (r.c2Skew)(a) },
        );
        assert_bits_eq(
            &format!("c2CCW90 #{i} {a:?}"),
            &unsafe { (c.c2CCW90)(a) },
            &unsafe { (r.c2CCW90)(a) },
        );
    }
    // A signalling NaN must have its sign flipped WITHOUT being quieted.
    for bits in [
        0x7f80_0001u32,
        0xff80_0001,
        0x7fc0_0000,
        0xffc0_0000,
        0x7fbf_ffff,
        0x0000_0000,
        0x8000_0000,
    ] {
        let a = c2v {
            x: f32::from_bits(bits),
            y: f32::from_bits(bits ^ 0x8000_0000),
        };
        assert_bits_eq(
            &format!("c2Neg snan 0x{bits:08x}"),
            &unsafe { (c.c2Neg)(a) },
            &unsafe { (r.c2Neg)(a) },
        );
        assert_bits_eq(
            &format!("c2Skew snan 0x{bits:08x}"),
            &unsafe { (c.c2Skew)(a) },
            &unsafe { (r.c2Skew)(a) },
        );
        assert_bits_eq(
            &format!("c2CCW90 snan 0x{bits:08x}"),
            &unsafe { (c.c2CCW90)(a) },
            &unsafe { (r.c2CCW90)(a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 10, 11 — c2Div / c2Norm
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_div() {
    let (c, r) = both();
    let mut rng = Rng::new(10);
    for i in 0..N {
        let a = rng.any_v();
        let d = rng.any_f32();
        assert_bits_eq(
            &format!("c2Div #{i} {a:?} / {d:?}"),
            &unsafe { (c.c2Div)(a, d) },
            &unsafe { (r.c2Div)(a, d) },
        );
    }
}

#[test]
fn cfg_leaf_norm() {
    let (c, r) = both();
    let mut rng = Rng::new(11);
    for i in 0..N {
        let a = rng.any_v();
        assert_bits_eq(
            &format!("c2Norm #{i} {a:?}"),
            &unsafe { (c.c2Norm)(a) },
            &unsafe { (r.c2Norm)(a) },
        );
    }
    // Every magnitude class, including the ones where 1/len is inf or 0.
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: FLT_MAX, y: FLT_MAX },
        c2v {
            x: f32::from_bits(1),
            y: 0.0,
        },
        c2v {
            x: f32::INFINITY,
            y: 1.0,
        },
        c2v {
            x: f32::NEG_INFINITY,
            y: f32::INFINITY,
        },
        c2v {
            x: FLT_EPSILON,
            y: -FLT_EPSILON,
        },
    ] {
        assert_bits_eq(
            &format!("c2Norm fixed {a:?}"),
            &unsafe { (c.c2Norm)(a) },
            &unsafe { (r.c2Norm)(a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — the two identity constructors
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_identities() {
    let (c, r) = both();
    assert_bits_eq(
        "c2RotIdentity",
        &unsafe { (c.c2RotIdentity)() },
        &unsafe { (r.c2RotIdentity)() },
    );
    assert_bits_eq(
        "c2xIdentity",
        &unsafe { (c.c2xIdentity)() },
        &unsafe { (r.c2xIdentity)() },
    );
    // And the exact expected bit patterns, so a matching pair of wrong answers
    // would still fail.
    let ri = unsafe { (c.c2RotIdentity)() };
    assert_eq!((ri.c.to_bits(), ri.s.to_bits()), (0x3f80_0000, 0));
    let xi = unsafe { (c.c2xIdentity)() };
    assert_eq!(
        (
            xi.p.x.to_bits(),
            xi.p.y.to_bits(),
            xi.r.c.to_bits(),
            xi.r.s.to_bits()
        ),
        (0, 0, 0x3f80_0000, 0)
    );
}

// ---------------------------------------------------------------------------
// Rows 13, 14, 15 — c2Mulrv / c2MulrvT / c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn cfg_leaf_mulrv() {
    let (c, r) = both();
    let mut rng = Rng::new(13);
    for i in 0..N {
        let (rot, v) = (rng.any_r(), rng.any_v());
        assert_bits_eq(
            &format!("c2Mulrv #{i} {rot:?} {v:?}"),
            &unsafe { (c.c2Mulrv)(rot, v) },
            &unsafe { (r.c2Mulrv)(rot, v) },
        );
        let (rot, v) = (rng.rot(), rng.v());
        assert_bits_eq(
            &format!("c2Mulrv unit #{i} {rot:?} {v:?}"),
            &unsafe { (c.c2Mulrv)(rot, v) },
            &unsafe { (r.c2Mulrv)(rot, v) },
        );
    }
    // NaN in each of the four scalar slots independently: c2Mulrv has four
    // products and two sums, each with its own destination operand.
    let mut rng = Rng::new(0xd0);
    for i in 0..N {
        let clean = [rng.coord(), rng.coord(), rng.coord(), rng.coord()];
        for slot in 0..4 {
            let mut s = clean;
            s[slot] = rng.nan();
            let rot = c2r { c: s[0], s: s[1] };
            let v = c2v { x: s[2], y: s[3] };
            assert_bits_eq(
                &format!("c2Mulrv nan slot{slot} #{i}"),
                &unsafe { (c.c2Mulrv)(rot, v) },
                &unsafe { (r.c2Mulrv)(rot, v) },
            );
            assert_bits_eq(
                &format!("c2MulrvT nan slot{slot} #{i}"),
                &unsafe { (c.c2MulrvT)(rot, v) },
                &unsafe { (r.c2MulrvT)(rot, v) },
            );
        }
    }
}

#[test]
fn cfg_leaf_mulrvt() {
    let (c, r) = both();
    let mut rng = Rng::new(14);
    for i in 0..N {
        let (rot, v) = (rng.any_r(), rng.any_v());
        assert_bits_eq(
            &format!("c2MulrvT #{i} {rot:?} {v:?}"),
            &unsafe { (c.c2MulrvT)(rot, v) },
            &unsafe { (r.c2MulrvT)(rot, v) },
        );
        let (rot, v) = (rng.rot(), rng.v());
        assert_bits_eq(
            &format!("c2MulrvT unit #{i} {rot:?} {v:?}"),
            &unsafe { (c.c2MulrvT)(rot, v) },
            &unsafe { (r.c2MulrvT)(rot, v) },
        );
    }
    // `-a.s * b.x + a.c * b.y` is NOT compiled as `a.c*b.y - a.s*b.x` at -O0:
    // the sign flip is a separate xorps and the addss destination is the first
    // term. Signed zeros and NaN sign bits make that observable.
    for (rc, rs, vx, vy) in [
        (1.0f32, 0.0f32, 0.0f32, 0.0f32),
        (1.0, -0.0, 0.0, 0.0),
        (0.0, 0.0, -0.0, -0.0),
        (-0.0, -0.0, 0.0, -0.0),
        (1.0, f32::from_bits(0x7f80_0001), 1.0, 1.0),
        (1.0, f32::from_bits(0xff80_0001), 1.0, 1.0),
        (f32::from_bits(0x7fc0_1234), 1.0, 1.0, 1.0),
    ] {
        let rot = c2r { c: rc, s: rs };
        let v = c2v { x: vx, y: vy };
        assert_bits_eq(
            &format!("c2MulrvT signed {rot:?} {v:?}"),
            &unsafe { (c.c2MulrvT)(rot, v) },
            &unsafe { (r.c2MulrvT)(rot, v) },
        );
        assert_bits_eq(
            &format!("c2Mulrv signed {rot:?} {v:?}"),
            &unsafe { (c.c2Mulrv)(rot, v) },
            &unsafe { (r.c2Mulrv)(rot, v) },
        );
    }
}

#[test]
fn cfg_leaf_mulxv() {
    let (c, r) = both();
    let mut rng = Rng::new(15);
    let ident = unsafe { (c.c2xIdentity)() };
    for i in 0..N {
        let v = rng.any_v();
        // identity / pure translation / pure rotation / both / garbage
        let xs = [
            ident,
            c2x {
                p: rng.v(),
                r: c2r { c: 1.0, s: 0.0 },
            },
            c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: rng.rot(),
            },
            c2x {
                p: rng.v(),
                r: rng.rot(),
            },
            c2x {
                p: rng.any_v(),
                r: rng.any_r(),
            },
        ];
        for (k, x) in xs.iter().enumerate() {
            assert_bits_eq(
                &format!("c2Mulxv #{i} case{k} {x:?} {v:?}"),
                &unsafe { (c.c2Mulxv)(*x, v) },
                &unsafe { (r.c2Mulxv)(*x, v) },
            );
        }
    }
}
