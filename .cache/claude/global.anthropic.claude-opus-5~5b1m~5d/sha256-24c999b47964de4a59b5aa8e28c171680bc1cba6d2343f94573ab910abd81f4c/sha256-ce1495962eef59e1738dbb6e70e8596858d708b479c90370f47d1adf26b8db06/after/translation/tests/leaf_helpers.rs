//! Phase B, Group 1 — CONFIGS.md rows C01..C16.
//!
//! Every leaf vector helper exported by the C `.so`, driven through the FFI on
//! BOTH libraries with seeded randomized inputs (including NaN/inf/denormal/
//! signed-zero bit patterns) and compared bit-for-bit.

mod common;
use common::*;

const N: u32 = 4096;

// ---------------------------------------------------------------------------
// C01 c2V
// ---------------------------------------------------------------------------

#[test]
fn c01_c2v() {
    let (c, r): (FnV2, FnV2) = sym(b"c2V");
    let mut rng = Rng::new(0xC01);
    for i in 0..N {
        let (x, y) = (rng.spicy(), rng.spicy());
        let cv = unsafe { c(x, y) };
        let rv = unsafe { r(x, y) };
        assert_v(cv, rv, &format!("c2V #{i} x={} y={}", fmt_f32(x), fmt_f32(y)));
    }
    // Boundary sweep over raw bit patterns.
    for &x in &[0.0f32, -0.0, f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, FLT_MAX, FLT_MIN, 1e-45] {
        for &y in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, FLT_MAX] {
            assert_v(unsafe { c(x, y) }, unsafe { r(x, y) }, "c2V boundary");
        }
    }
}

// ---------------------------------------------------------------------------
// C02 c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn c02_c2mulvs() {
    let (c, r): (FnVvf, FnVvf) = sym(b"c2Mulvs");
    let mut rng = Rng::new(0xC02);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.spicy();
        assert_v(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Mulvs #{i} a={} b={}", fmt_v(a), fmt_f32(b)),
        );
    }
    let scalars = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e-45,
        FLT_MAX,
    ];
    for &b in &scalars {
        for &a in &[
            C2v { x: 0.0, y: -0.0 },
            C2v {
                x: f32::INFINITY,
                y: f32::NAN,
            },
            C2v { x: FLT_MAX, y: FLT_MIN },
            C2v { x: 1.0, y: -1.0 },
        ] {
            assert_v(unsafe { c(a, b) }, unsafe { r(a, b) }, "c2Mulvs boundary");
        }
    }
}

// ---------------------------------------------------------------------------
// C03 c2Add / c2Sub
// ---------------------------------------------------------------------------

#[test]
fn c03_c2add_c2sub() {
    let (ca, ra): (FnVvv, FnVvv) = sym(b"c2Add");
    let (cs, rs): (FnVvv, FnVvv) = sym(b"c2Sub");
    let mut rng = Rng::new(0xC03);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.v_spicy();
        assert_v(
            unsafe { ca(a, b) },
            unsafe { ra(a, b) },
            &format!("c2Add #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
        assert_v(
            unsafe { cs(a, b) },
            unsafe { rs(a, b) },
            &format!("c2Sub #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
    }
    // Overflow, inf-inf, signed-zero mixes.
    let pairs = [
        (C2v { x: FLT_MAX, y: FLT_MAX }, C2v { x: FLT_MAX, y: FLT_MAX }),
        (
            C2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            C2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
        ),
        (C2v { x: 0.0, y: -0.0 }, C2v { x: -0.0, y: 0.0 }),
        (C2v { x: 1e-45, y: -1e-45 }, C2v { x: 1e-45, y: 1e-45 }),
    ];
    for (a, b) in pairs {
        assert_v(unsafe { ca(a, b) }, unsafe { ra(a, b) }, "c2Add boundary");
        assert_v(unsafe { cs(a, b) }, unsafe { rs(a, b) }, "c2Sub boundary");
    }
}

// ---------------------------------------------------------------------------
// C04 c2Dot / C05 c2Det2
// ---------------------------------------------------------------------------

#[test]
fn c04_c2dot() {
    let (c, r): (FnFvv, FnFvv) = sym(b"c2Dot");
    let mut rng = Rng::new(0xC04);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.v_spicy();
        assert_f32(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Dot #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
    }
    // Exact cancellation, overflow and 0*inf.
    let pairs = [
        (C2v { x: 1.0, y: 1.0 }, C2v { x: 1.0, y: -1.0 }),
        (C2v { x: FLT_MAX, y: FLT_MAX }, C2v { x: FLT_MAX, y: FLT_MAX }),
        (C2v { x: 0.0, y: 1.0 }, C2v { x: f32::INFINITY, y: 1.0 }),
        (
            C2v { x: FLT_MAX, y: FLT_MAX },
            C2v {
                x: FLT_MAX,
                y: -FLT_MAX,
            },
        ),
        (C2v { x: 1e-45, y: 1e-45 }, C2v { x: 1e-45, y: 1e-45 }),
    ];
    for (a, b) in pairs {
        assert_f32(unsafe { c(a, b) }, unsafe { r(a, b) }, "c2Dot boundary");
    }
}

#[test]
fn c05_c2det2() {
    let (c, r): (FnFvv, FnFvv) = sym(b"c2Det2");
    let mut rng = Rng::new(0xC05);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.v_spicy();
        assert_f32(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Det2 #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
    }
    // Collinear (det 0), antiparallel, overflow.
    for i in 0..512 {
        let mut rng2 = Rng::new(0xD05 + i);
        let a = rng2.v_finite();
        let k = rng2.finite();
        let b = C2v { x: a.x * k, y: a.y * k };
        assert_f32(unsafe { c(a, b) }, unsafe { r(a, b) }, "c2Det2 collinear");
        let nb = C2v { x: -a.x, y: -a.y };
        assert_f32(unsafe { c(a, nb) }, unsafe { r(a, nb) }, "c2Det2 antiparallel");
    }
    let pairs = [
        (C2v { x: FLT_MAX, y: FLT_MAX }, C2v { x: FLT_MAX, y: FLT_MAX }),
        (
            C2v { x: f32::INFINITY, y: 0.0 },
            C2v { x: 0.0, y: f32::INFINITY },
        ),
        (C2v { x: f32::NAN, y: 1.0 }, C2v { x: 1.0, y: f32::NAN }),
        (C2v { x: -0.0, y: 0.0 }, C2v { x: 0.0, y: -0.0 }),
    ];
    for (a, b) in pairs {
        assert_f32(unsafe { c(a, b) }, unsafe { r(a, b) }, "c2Det2 boundary");
    }
}

// ---------------------------------------------------------------------------
// C06 c2Len
// ---------------------------------------------------------------------------

#[test]
fn c06_c2len() {
    let (c, r): (FnFv, FnFv) = sym(b"c2Len");
    let mut rng = Rng::new(0xC06);
    for i in 0..N {
        let a = rng.v_spicy();
        assert_f32(
            unsafe { c(a) },
            unsafe { r(a) },
            &format!("c2Len #{i} {}", fmt_v(a)),
        );
    }
    for a in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 3.0, y: 4.0 },
        C2v { x: FLT_MAX, y: FLT_MAX },
        C2v { x: FLT_MIN, y: FLT_MIN },
        C2v { x: 1e-45, y: 1e-45 },
        C2v { x: f32::NAN, y: 0.0 },
        C2v {
            x: f32::INFINITY,
            y: f32::NAN,
        },
        C2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
    ] {
        assert_f32(unsafe { c(a) }, unsafe { r(a) }, "c2Len boundary");
    }
}

// ---------------------------------------------------------------------------
// C07 c2Div / C08 c2Norm
// ---------------------------------------------------------------------------

#[test]
fn c07_c2div() {
    let (c, r): (FnVvf, FnVvf) = sym(b"c2Div");
    let mut rng = Rng::new(0xC07);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.spicy();
        assert_v(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Div #{i} {} / {}", fmt_v(a), fmt_f32(b)),
        );
    }
    for &b in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, FLT_MIN, 1e-45] {
        for a in [
            C2v { x: 0.0, y: -0.0 },
            C2v { x: 1.0, y: -1.0 },
            C2v {
                x: f32::INFINITY,
                y: f32::NAN,
            },
            C2v { x: FLT_MAX, y: FLT_MIN },
        ] {
            assert_v(unsafe { c(a, b) }, unsafe { r(a, b) }, "c2Div boundary");
        }
    }
}

#[test]
fn c08_c2norm() {
    let (c, r): (FnVv, FnVv) = sym(b"c2Norm");
    let mut rng = Rng::new(0xC08);
    for i in 0..N {
        let a = rng.v_spicy();
        assert_v(
            unsafe { c(a) },
            unsafe { r(a) },
            &format!("c2Norm #{i} {}", fmt_v(a)),
        );
    }
    for a in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: 0.0 },
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 3.0, y: 4.0 },
        C2v { x: FLT_MAX, y: FLT_MAX },
        C2v { x: 1e-45, y: 0.0 },
        C2v {
            x: f32::INFINITY,
            y: 1.0,
        },
        C2v { x: f32::NAN, y: 1.0 },
    ] {
        assert_v(unsafe { c(a) }, unsafe { r(a) }, "c2Norm boundary");
    }
}

// ---------------------------------------------------------------------------
// C09 c2Neg / c2Skew / c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn c09_unary_sign_helpers() {
    let (cn, rn): (FnVv, FnVv) = sym(b"c2Neg");
    let (ck, rk): (FnVv, FnVv) = sym(b"c2Skew");
    let (cw, rw): (FnVv, FnVv) = sym(b"c2CCW90");
    let mut rng = Rng::new(0xC09);
    for i in 0..N {
        let a = rng.v_bits();
        assert_v(unsafe { cn(a) }, unsafe { rn(a) }, &format!("c2Neg #{i}"));
        assert_v(unsafe { ck(a) }, unsafe { rk(a) }, &format!("c2Skew #{i}"));
        assert_v(unsafe { cw(a) }, unsafe { rw(a) }, &format!("c2CCW90 #{i}"));
    }
    for a in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: f32::NAN, y: -f32::NAN },
        C2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        // signalling NaN payloads
        C2v {
            x: f32::from_bits(0x7F80_0001),
            y: f32::from_bits(0xFF80_0001),
        },
    ] {
        assert_v(unsafe { cn(a) }, unsafe { rn(a) }, "c2Neg boundary");
        assert_v(unsafe { ck(a) }, unsafe { rk(a) }, "c2Skew boundary");
        assert_v(unsafe { cw(a) }, unsafe { rw(a) }, "c2CCW90 boundary");
    }
}

// ---------------------------------------------------------------------------
// C10 c2Maxv / c2Minv
// ---------------------------------------------------------------------------

#[test]
fn c10_minmax() {
    let (cx, rx): (FnVvv, FnVvv) = sym(b"c2Maxv");
    let (cm, rm): (FnVvv, FnVvv) = sym(b"c2Minv");
    let mut rng = Rng::new(0xC10);
    for i in 0..N {
        let a = rng.v_spicy();
        let b = rng.v_spicy();
        assert_v(
            unsafe { cx(a, b) },
            unsafe { rx(a, b) },
            &format!("c2Maxv #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
        assert_v(
            unsafe { cm(a, b) },
            unsafe { rm(a, b) },
            &format!("c2Minv #{i} {} {}", fmt_v(a), fmt_v(b)),
        );
    }
    // a == b, +0/-0, one NaN, both NaN.
    let specials = [0.0f32, -0.0, f32::NAN, -f32::NAN, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY];
    for &ax in &specials {
        for &bx in &specials {
            for &ay in &specials {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: ay };
                assert_v(unsafe { cx(a, b) }, unsafe { rx(a, b) }, "c2Maxv special");
                assert_v(unsafe { cm(a, b) }, unsafe { rm(a, b) }, "c2Minv special");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11 c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn c11_clampv() {
    let (c, r): (FnVvvv, FnVvvv) = sym(b"c2Clampv");
    let mut rng = Rng::new(0xC11);
    // random, incl. inverted ranges and NaNs
    for i in 0..N {
        let a = rng.v_spicy();
        let lo = rng.v_spicy();
        let hi = rng.v_spicy();
        assert_v(
            unsafe { c(a, lo, hi) },
            unsafe { r(a, lo, hi) },
            &format!("c2Clampv #{i} a={} lo={} hi={}", fmt_v(a), fmt_v(lo), fmt_v(hi)),
        );
    }
    // well-ordered lo<hi with a inside/below/above
    for i in 0..1024 {
        let mut g = Rng::new(0xD11 + i);
        let p = g.v_range(-100.0, 100.0);
        let q = g.v_range(-100.0, 100.0);
        let lo = C2v { x: p.x.min(q.x), y: p.y.min(q.y) };
        let hi = C2v { x: p.x.max(q.x), y: p.y.max(q.y) };
        for a in [g.v_range(-200.0, 200.0), lo, hi, C2v { x: lo.x - 1.0, y: hi.y + 1.0 }] {
            assert_v(
                unsafe { c(a, lo, hi) },
                unsafe { r(a, lo, hi) },
                "c2Clampv ordered",
            );
            // inverted
            assert_v(
                unsafe { c(a, hi, lo) },
                unsafe { r(a, hi, lo) },
                "c2Clampv inverted",
            );
        }
    }
    // NaN in each slot
    let nanv = C2v { x: f32::NAN, y: f32::NAN };
    let one = C2v { x: 1.0, y: 1.0 };
    let two = C2v { x: 2.0, y: 2.0 };
    for (a, lo, hi) in [
        (nanv, one, two),
        (one, nanv, two),
        (one, two, nanv),
        (nanv, nanv, nanv),
    ] {
        assert_v(unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) }, "c2Clampv NaN");
    }
}

// ---------------------------------------------------------------------------
// C12 c2RotIdentity / c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn c12_identities() {
    let (cr, rr): (FnR, FnR) = sym(b"c2RotIdentity");
    let (cx, rx): (FnX, FnX) = sym(b"c2xIdentity");
    for _ in 0..64 {
        let a = unsafe { cr() };
        let b = unsafe { rr() };
        assert!(r_same(a, b), "c2RotIdentity mismatch: C {a:?} Rust {b:?}");
        assert_eq!(a.c.to_bits(), 1.0f32.to_bits());
        assert_eq!(a.s.to_bits(), 0.0f32.to_bits());
        let a = unsafe { cx() };
        let b = unsafe { rx() };
        assert!(x_same(a, b), "c2xIdentity mismatch: C {a:?} Rust {b:?}");
    }
}

// ---------------------------------------------------------------------------
// C13 c2Mulrv / C14 c2MulrvT / C15 round-trip
// ---------------------------------------------------------------------------

#[test]
fn c13_mulrv() {
    let (c, r): (FnMulrv, FnMulrv) = sym(b"c2Mulrv");
    let mut rng = Rng::new(0xC13);
    for i in 0..N {
        let a = rng.rot_spicy();
        let b = rng.v_spicy();
        assert_v(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Mulrv #{i} rot=({},{}) v={}", fmt_f32(a.c), fmt_f32(a.s), fmt_v(b)),
        );
    }
    exact_rotation_sweep(c, r, "c2Mulrv");
}

#[test]
fn c14_mulrvt() {
    let (c, r): (FnMulrv, FnMulrv) = sym(b"c2MulrvT");
    let mut rng = Rng::new(0xC14);
    for i in 0..N {
        let a = rng.rot_spicy();
        let b = rng.v_spicy();
        assert_v(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2MulrvT #{i} rot=({},{}) v={}", fmt_f32(a.c), fmt_f32(a.s), fmt_v(b)),
        );
    }
    exact_rotation_sweep(c, r, "c2MulrvT");
    // The C spells the second component `-a.s * b.x + a.c * b.y`; the negation
    // must happen on `a.s` BEFORE the multiply, which is observable through
    // NaN sign bits and signed zeros.
    let rots = [
        C2r { c: 1.0, s: f32::NAN },
        C2r { c: 1.0, s: -f32::NAN },
        C2r {
            c: f32::NAN,
            s: f32::NAN,
        },
        C2r { c: 0.0, s: 0.0 },
        C2r { c: -0.0, s: -0.0 },
        C2r { c: 0.0, s: -0.0 },
        C2r {
            c: f32::INFINITY,
            s: f32::NEG_INFINITY,
        },
        C2r {
            c: 1.0,
            s: f32::from_bits(0x7F80_0001),
        },
        C2r {
            c: 1.0,
            s: f32::from_bits(0xFFC0_1234),
        },
    ];
    let vs = [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 1.0, y: 1.0 },
        C2v { x: -1.0, y: 2.0 },
        C2v { x: f32::NAN, y: 1.0 },
        C2v {
            x: f32::INFINITY,
            y: 0.0,
        },
    ];
    for &a in &rots {
        for &b in &vs {
            assert_v(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                &format!(
                    "c2MulrvT signbit rot=({},{}) v={}",
                    fmt_f32(a.c),
                    fmt_f32(a.s),
                    fmt_v(b)
                ),
            );
        }
    }
}

fn exact_rotation_sweep(c: FnMulrv, r: FnMulrv, name: &str) {
    let rots = [
        C2r { c: 1.0, s: 0.0 },
        C2r { c: 0.0, s: 1.0 },
        C2r { c: -1.0, s: 0.0 },
        C2r { c: 0.0, s: -1.0 },
        C2r { c: 0.5, s: 0.5 },
        C2r { c: 2.0, s: 3.0 },
        C2r { c: -0.0, s: -0.0 },
    ];
    let mut rng = Rng::new(0x517);
    for &a in &rots {
        for _ in 0..256 {
            let b = rng.v_range(-1e6, 1e6);
            assert_v(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                &format!("{name} exact rotation"),
            );
        }
    }
}

#[test]
fn c15_mulrv_roundtrip() {
    let (cf, rf): (FnMulrv, FnMulrv) = sym(b"c2Mulrv");
    let (ct, rt): (FnMulrv, FnMulrv) = sym(b"c2MulrvT");
    let mut rng = Rng::new(0xC15);
    for i in 0..N {
        let rot = rng.rot_unit();
        let v = rng.v_range(-1e4, 1e4);
        let c1 = unsafe { cf(rot, v) };
        let r1 = unsafe { rf(rot, v) };
        assert_v(c1, r1, &format!("roundtrip fwd #{i}"));
        let c2 = unsafe { ct(rot, c1) };
        let r2 = unsafe { rt(rot, r1) };
        assert_v(c2, r2, &format!("roundtrip back #{i}"));
    }
}

// ---------------------------------------------------------------------------
// C16 c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn c16_mulxv() {
    let (c, r): (FnMulxv, FnMulxv) = sym(b"c2Mulxv");
    let (cxi, _): (FnX, FnX) = sym(b"c2xIdentity");
    let mut rng = Rng::new(0xC16);
    let ident = unsafe { cxi() };
    for i in 0..N {
        let x = rng.x_spicy();
        let v = rng.v_spicy();
        assert_v(
            unsafe { c(x, v) },
            unsafe { r(x, v) },
            &format!("c2Mulxv #{i}"),
        );
        // identity / pure translation / pure rotation variants
        let pure_t = C2x {
            p: rng.v_range(-1e3, 1e3),
            r: C2r { c: 1.0, s: 0.0 },
        };
        let pure_r = C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: rng.rot_unit(),
        };
        for xx in [ident, pure_t, pure_r] {
            assert_v(
                unsafe { c(xx, v) },
                unsafe { r(xx, v) },
                "c2Mulxv structured",
            );
        }
    }
}
