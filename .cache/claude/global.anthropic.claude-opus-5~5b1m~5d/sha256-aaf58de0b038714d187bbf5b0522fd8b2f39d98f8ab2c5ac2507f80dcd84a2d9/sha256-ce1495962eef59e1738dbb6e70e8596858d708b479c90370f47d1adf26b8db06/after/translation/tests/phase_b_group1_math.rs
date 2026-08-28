//! Phase B, Group 1 — CONFIGS.md rows C1..C15 (leaf vector maths).
//!
//! Every assertion compares the C `.so` against the Rust `.so`, both reached
//! through `libloading`.

#![allow(non_snake_case)]

mod common;

use common::*;

const N: usize = 4000;

#[test]
fn c1_c2V() {
    let p = load();
    let c: FnV = p.c.sym("c2V");
    let r: FnV = p.rs.sym("c2V");
    let mut rng = Rng::new(0xC1);
    unsafe {
        for x in specials() {
            for y in specials() {
                assert_bits_eq!(c(x, y), r(x, y), "c2V({}, {})", f32_hex(x), f32_hex(y));
            }
        }
        for _ in 0..N {
            let (x, y) = (rng.wild(), rng.wild());
            assert_bits_eq!(c(x, y), r(x, y), "c2V({}, {})", f32_hex(x), f32_hex(y));
        }
    }
}

#[test]
fn c2_c3_c2Mulvs() {
    let p = load();
    let c: FnMulvs = p.c.sym("c2Mulvs");
    let r: FnMulvs = p.rs.sym("c2Mulvs");
    let mut rng = Rng::new(0xC2);
    unsafe {
        // C2: random finite
        for _ in 0..N {
            let (a, b) = (rng.v(), rng.coord());
            assert_bits_eq!(c(a, b), r(a, b), "c2Mulvs({}, {})", v_hex(&a), f32_hex(b));
        }
        // C3: 0 * inf, NaN, denormals — full special x special sweep
        for xb in specials() {
            for yb in specials() {
                let a = c2v { x: xb, y: yb };
                for s in specials() {
                    assert_bits_eq!(
                        c(a, s),
                        r(a, s),
                        "c2Mulvs({}, {})",
                        v_hex(&a),
                        f32_hex(s)
                    );
                }
            }
        }
        for _ in 0..N {
            let (a, b) = (rng.v_wild(), rng.wild());
            assert_bits_eq!(c(a, b), r(a, b), "c2Mulvs({}, {})", v_hex(&a), f32_hex(b));
        }
    }
}

#[test]
fn c4_c2Maxv_c2Minv() {
    let p = load();
    let cmax: FnVV = p.c.sym("c2Maxv");
    let rmax: FnVV = p.rs.sym("c2Maxv");
    let cmin: FnVV = p.c.sym("c2Minv");
    let rmin: FnVV = p.rs.sym("c2Minv");
    let mut rng = Rng::new(0xC4);
    unsafe {
        // full special x special sweep on x, mirrored on y
        for xa in specials() {
            for xb in specials() {
                let a = c2v { x: xa, y: xb };
                let b = c2v { x: xb, y: xa };
                assert_bits_eq!(cmax(a, b), rmax(a, b), "c2Maxv({}, {})", v_hex(&a), v_hex(&b));
                assert_bits_eq!(cmin(a, b), rmin(a, b), "c2Minv({}, {})", v_hex(&a), v_hex(&b));
            }
        }
        for _ in 0..N {
            let (a, b) = (rng.v_wild(), rng.v_wild());
            assert_bits_eq!(cmax(a, b), rmax(a, b), "c2Maxv({}, {})", v_hex(&a), v_hex(&b));
            assert_bits_eq!(cmin(a, b), rmin(a, b), "c2Minv({}, {})", v_hex(&a), v_hex(&b));
            // equal operands / equal components
            assert_bits_eq!(cmax(a, a), rmax(a, a), "c2Maxv(a,a) {}", v_hex(&a));
            assert_bits_eq!(cmin(a, a), rmin(a, a), "c2Minv(a,a) {}", v_hex(&a));
        }
    }
}

#[test]
fn c5_c2Clampv() {
    let p = load();
    let c: FnVVV = p.c.sym("c2Clampv");
    let r: FnVVV = p.rs.sym("c2Clampv");
    let mut rng = Rng::new(0xC5);
    unsafe {
        for _ in 0..(N * 2) {
            let a = if rng.below(4) == 0 { rng.v_wild() } else { rng.v() };
            let bb = rng.aabb();
            let (lo, hi) = (bb.min, bb.max);
            assert_bits_eq!(
                c(a, lo, hi),
                r(a, lo, hi),
                "c2Clampv({}, {}, {})",
                v_hex(&a),
                v_hex(&lo),
                v_hex(&hi)
            );
            // inverted range on purpose
            assert_bits_eq!(
                c(a, hi, lo),
                r(a, hi, lo),
                "c2Clampv inverted ({}, {}, {})",
                v_hex(&a),
                v_hex(&hi),
                v_hex(&lo)
            );
        }
        for s in specials() {
            let a = c2v { x: s, y: s };
            for t in specials() {
                let lo = c2v { x: t, y: -t };
                let hi = c2v { x: -t, y: t };
                assert_bits_eq!(
                    c(a, lo, hi),
                    r(a, lo, hi),
                    "c2Clampv({}, {}, {})",
                    v_hex(&a),
                    v_hex(&lo),
                    v_hex(&hi)
                );
            }
        }
    }
}

#[test]
fn c6_c2Sub_c2Add() {
    let p = load();
    let csub: FnVV = p.c.sym("c2Sub");
    let rsub: FnVV = p.rs.sym("c2Sub");
    let cadd: FnVV = p.c.sym("c2Add");
    let radd: FnVV = p.rs.sym("c2Add");
    let mut rng = Rng::new(0xC6);
    unsafe {
        for xa in specials() {
            for xb in specials() {
                let a = c2v { x: xa, y: xb };
                let b = c2v { x: xb, y: xa };
                assert_bits_eq!(csub(a, b), rsub(a, b), "c2Sub({}, {})", v_hex(&a), v_hex(&b));
                assert_bits_eq!(cadd(a, b), radd(a, b), "c2Add({}, {})", v_hex(&a), v_hex(&b));
            }
        }
        for _ in 0..(N * 2) {
            let (a, b) = (rng.v_wild(), rng.v_wild());
            assert_bits_eq!(csub(a, b), rsub(a, b), "c2Sub({}, {})", v_hex(&a), v_hex(&b));
            assert_bits_eq!(cadd(a, b), radd(a, b), "c2Add({}, {})", v_hex(&a), v_hex(&b));
        }
        // deliberate overflow to +-inf
        let big = c2v { x: f32::MAX, y: -f32::MAX };
        let big2 = c2v { x: f32::MAX, y: f32::MAX };
        assert_bits_eq!(cadd(big, big2), radd(big, big2), "c2Add overflow");
        assert_bits_eq!(csub(big, big2), rsub(big, big2), "c2Sub overflow");
    }
}

#[test]
fn c7_c2Dot() {
    let p = load();
    let c: FnVVf = p.c.sym("c2Dot");
    let r: FnVVf = p.rs.sym("c2Dot");
    let mut rng = Rng::new(0xC7);
    unsafe {
        // Exhaustive 20^4 = 160_000 special-value sweep: this is exactly where a
        // wrong SSE destination operand for NaN propagation shows up.
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                for xb in specials() {
                    for yb in specials() {
                        let b = c2v { x: xb, y: yb };
                        assert_f32_bits_eq!(
                            c(a, b),
                            r(a, b),
                            "c2Dot({}, {})",
                            v_hex(&a),
                            v_hex(&b)
                        );
                    }
                }
            }
        }
        for _ in 0..(N * 4) {
            let (a, b) = (rng.v_wild(), rng.v_wild());
            assert_f32_bits_eq!(c(a, b), r(a, b), "c2Dot({}, {})", v_hex(&a), v_hex(&b));
        }
    }
}

#[test]
fn c8_c2Det2() {
    let p = load();
    let c: FnVVf = p.c.sym("c2Det2");
    let r: FnVVf = p.rs.sym("c2Det2");
    let mut rng = Rng::new(0xC8);
    unsafe {
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                for xb in specials() {
                    for yb in specials() {
                        let b = c2v { x: xb, y: yb };
                        assert_f32_bits_eq!(
                            c(a, b),
                            r(a, b),
                            "c2Det2({}, {})",
                            v_hex(&a),
                            v_hex(&b)
                        );
                    }
                }
            }
        }
        for _ in 0..(N * 4) {
            let (a, b) = (rng.v_wild(), rng.v_wild());
            assert_f32_bits_eq!(c(a, b), r(a, b), "c2Det2({}, {})", v_hex(&a), v_hex(&b));
            // parallel / degenerate: det == +-0
            let k = rng.coord();
            let par = c2v { x: a.x * k, y: a.y * k };
            assert_f32_bits_eq!(
                c(a, par),
                r(a, par),
                "c2Det2 parallel({}, {})",
                v_hex(&a),
                v_hex(&par)
            );
        }
    }
}

#[test]
fn c9_c2Len() {
    let p = load();
    let c: FnVf = p.c.sym("c2Len");
    let r: FnVf = p.rs.sym("c2Len");
    let mut rng = Rng::new(0xC9);
    unsafe {
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                assert_f32_bits_eq!(c(a), r(a), "c2Len({})", v_hex(&a));
            }
        }
        for _ in 0..(N * 4) {
            let a = rng.v_wild();
            assert_f32_bits_eq!(c(a), r(a), "c2Len({})", v_hex(&a));
        }
        let zero = c2v { x: 0.0, y: 0.0 };
        assert_f32_bits_eq!(c(zero), r(zero), "c2Len(zero)");
    }
}

#[test]
fn c10_c2Div() {
    let p = load();
    let c: FnDiv = p.c.sym("c2Div");
    let r: FnDiv = p.rs.sym("c2Div");
    let mut rng = Rng::new(0xCA);
    unsafe {
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                for b in specials() {
                    assert_bits_eq!(c(a, b), r(a, b), "c2Div({}, {})", v_hex(&a), f32_hex(b));
                }
            }
        }
        for _ in 0..(N * 2) {
            let (a, b) = (rng.v_wild(), rng.wild());
            assert_bits_eq!(c(a, b), r(a, b), "c2Div({}, {})", v_hex(&a), f32_hex(b));
        }
    }
}

#[test]
fn c11_c2Norm() {
    let p = load();
    let c: FnV1 = p.c.sym("c2Norm");
    let r: FnV1 = p.rs.sym("c2Norm");
    let mut rng = Rng::new(0xCB);
    unsafe {
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                assert_bits_eq!(c(a), r(a), "c2Norm({})", v_hex(&a));
            }
        }
        for _ in 0..(N * 4) {
            let a = rng.v_wild();
            assert_bits_eq!(c(a), r(a), "c2Norm({})", v_hex(&a));
        }
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: f32::MAX, y: f32::MAX },
        ] {
            assert_bits_eq!(c(a), r(a), "c2Norm fixed({})", v_hex(&a));
        }
    }
}

#[test]
fn c12_c2Neg_c2Skew_c2CCW90() {
    let p = load();
    let cn: FnV1 = p.c.sym("c2Neg");
    let rn: FnV1 = p.rs.sym("c2Neg");
    let cs: FnV1 = p.c.sym("c2Skew");
    let rs_: FnV1 = p.rs.sym("c2Skew");
    let cc: FnV1 = p.c.sym("c2CCW90");
    let rc: FnV1 = p.rs.sym("c2CCW90");
    let mut rng = Rng::new(0xCC);
    unsafe {
        for xa in specials() {
            for ya in specials() {
                let a = c2v { x: xa, y: ya };
                assert_bits_eq!(cn(a), rn(a), "c2Neg({})", v_hex(&a));
                assert_bits_eq!(cs(a), rs_(a), "c2Skew({})", v_hex(&a));
                assert_bits_eq!(cc(a), rc(a), "c2CCW90({})", v_hex(&a));
            }
        }
        for _ in 0..(N * 2) {
            let a = rng.v_wild();
            assert_bits_eq!(cn(a), rn(a), "c2Neg({})", v_hex(&a));
            assert_bits_eq!(cs(a), rs_(a), "c2Skew({})", v_hex(&a));
            assert_bits_eq!(cc(a), rc(a), "c2CCW90({})", v_hex(&a));
        }
    }
}

#[test]
fn c13_identities() {
    let p = load();
    let crot: FnRotIdentity = p.c.sym("c2RotIdentity");
    let rrot: FnRotIdentity = p.rs.sym("c2RotIdentity");
    let cx: FnxIdentity = p.c.sym("c2xIdentity");
    let rx: FnxIdentity = p.rs.sym("c2xIdentity");
    unsafe {
        assert_bits_eq!(crot(), rrot(), "c2RotIdentity()");
        assert_bits_eq!(cx(), rx(), "c2xIdentity()");
    }
}

#[test]
fn c14_c2Mulrv_c2MulrvT() {
    let p = load();
    let cm: FnMulrv = p.c.sym("c2Mulrv");
    let rm: FnMulrv = p.rs.sym("c2Mulrv");
    let ct: FnMulrv = p.c.sym("c2MulrvT");
    let rt: FnMulrv = p.rs.sym("c2MulrvT");
    let mut rng = Rng::new(0xCE);
    unsafe {
        // exhaustive specials on the rotation and the vector
        for rc in specials() {
            for rsv in specials() {
                let rot = c2r { c: rc, s: rsv };
                for vx in specials() {
                    let v = c2v { x: vx, y: rc };
                    assert_bits_eq!(cm(rot, v), rm(rot, v), "c2Mulrv");
                    assert_bits_eq!(ct(rot, v), rt(rot, v), "c2MulrvT");
                }
            }
        }
        for _ in 0..(N * 4) {
            let rot = if rng.below(4) == 0 {
                c2r { c: rng.wild(), s: rng.wild() }
            } else {
                rng.rot()
            };
            let v = if rng.below(4) == 0 { rng.v_wild() } else { rng.v() };
            assert_bits_eq!(
                cm(rot, v),
                rm(rot, v),
                "c2Mulrv(c={} s={}, {})",
                f32_hex(rot.c),
                f32_hex(rot.s),
                v_hex(&v)
            );
            assert_bits_eq!(
                ct(rot, v),
                rt(rot, v),
                "c2MulrvT(c={} s={}, {})",
                f32_hex(rot.c),
                f32_hex(rot.s),
                v_hex(&v)
            );
        }
    }
}

#[test]
fn c15_c2Mulxv() {
    let p = load();
    let c: FnMulxv = p.c.sym("c2Mulxv");
    let r: FnMulxv = p.rs.sym("c2Mulxv");
    let mut rng = Rng::new(0xCF);
    unsafe {
        let ident = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        };
        for _ in 0..(N * 2) {
            let v = if rng.below(4) == 0 { rng.v_wild() } else { rng.v() };
            assert_bits_eq!(c(ident, v), r(ident, v), "c2Mulxv identity {}", v_hex(&v));
            // translation only
            let t = c2x { p: rng.v(), r: c2r { c: 1.0, s: 0.0 } };
            assert_bits_eq!(c(t, v), r(t, v), "c2Mulxv translation {}", v_hex(&v));
            // rotation only
            let ro = c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() };
            assert_bits_eq!(c(ro, v), r(ro, v), "c2Mulxv rotation {}", v_hex(&v));
            // both, and a fully wild transform
            let both = rng.x();
            assert_bits_eq!(c(both, v), r(both, v), "c2Mulxv both {}", v_hex(&v));
            let wild = c2x {
                p: rng.v_wild(),
                r: c2r { c: rng.wild(), s: rng.wild() },
            };
            assert_bits_eq!(c(wild, v), r(wild, v), "c2Mulxv wild {}", v_hex(&v));
        }
    }
}
