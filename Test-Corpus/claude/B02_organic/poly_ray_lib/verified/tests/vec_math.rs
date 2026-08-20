//! Phase B — CONFIGS.md rows 1..15 (leaf vector math).
//!
//! Every call goes through `dlopen`'d symbols of both the C `.so` and the Rust
//! `cdylib`.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4096;

// --- row 1 -----------------------------------------------------------------
#[test]
fn row01_c2V() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x0101);
    for i in 0..N {
        let (x, y) = if i < SPECIALS.len() * SPECIALS.len() {
            (SPECIALS[i / SPECIALS.len()], SPECIALS[i % SPECIALS.len()])
        } else {
            (rng.wild(), rng.wild())
        };
        let a = unsafe { (c.c2V)(x, y) };
        let b = unsafe { (r.c2V)(x, y) };
        assert!(
            veq(a, b),
            "c2V({}, {}): C={} RUST={}",
            fshow(x),
            fshow(y),
            vshow(a),
            vshow(b)
        );
    }
}

// --- rows 2 & 3 ------------------------------------------------------------
#[test]
fn row02_row03_c2Dot() {
    let (c, r) = (c(), rs());
    // row 3: full special x special cross product on all four components
    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            for &bx in SPECIALS.iter() {
                for &by in SPECIALS.iter() {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ra = unsafe { (c.c2Dot)(a, b) };
                    let rb = unsafe { (r.c2Dot)(a, b) };
                    assert!(
                        feq(ra, rb),
                        "c2Dot({}, {}): C={} RUST={}",
                        vshow(a),
                        vshow(b),
                        fshow(ra),
                        fshow(rb)
                    );
                }
            }
        }
    }
    // row 2: randomized
    let mut rng = Rng::new(0x0202);
    for _ in 0..N {
        let (a, b) = (rng.wild_v(), rng.wild_v());
        let ra = unsafe { (c.c2Dot)(a, b) };
        let rb = unsafe { (r.c2Dot)(a, b) };
        assert!(feq(ra, rb), "c2Dot({}, {})", vshow(a), vshow(b));
    }
    for _ in 0..N {
        let (a, b) = (rng.geom_v(), rng.geom_v());
        let ra = unsafe { (c.c2Dot)(a, b) };
        let rb = unsafe { (r.c2Dot)(a, b) };
        assert!(feq(ra, rb), "c2Dot({}, {})", vshow(a), vshow(b));
    }
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn row04_c2Len() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<C2v> = Vec::new();
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            cases.push(v(x, y));
        }
    }
    for &xb in SPECIAL_BITS.iter() {
        for &yb in SPECIAL_BITS.iter() {
            cases.push(v(f32::from_bits(xb), f32::from_bits(yb)));
        }
    }
    // overflow: dot(a,a) -> +inf
    cases.push(v(1.0e30, 1.0e30));
    cases.push(v(f32::MAX, f32::MAX));
    cases.push(v(-f32::MAX, 0.0));
    let mut rng = Rng::new(0x0404);
    for _ in 0..N {
        cases.push(rng.wild_v());
    }
    for _ in 0..N {
        cases.push(rng.geom_v());
    }
    for a in cases {
        let ra = unsafe { (c.c2Len)(a) };
        let rb = unsafe { (r.c2Len)(a) };
        assert!(
            feq(ra, rb),
            "c2Len({}): C={} RUST={}",
            vshow(a),
            fshow(ra),
            fshow(rb)
        );
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn row05_c2Add_c2Sub() {
    let (c, r) = (c(), rs());
    let mut pairs: Vec<(C2v, C2v)> = Vec::new();
    for &ax in SPECIALS.iter() {
        for &bx in SPECIALS.iter() {
            for &ay in SPECIALS.iter() {
                pairs.push((v(ax, ay), v(bx, ay)));
                pairs.push((v(ax, ay), v(bx, ax)));
            }
        }
    }
    let mut rng = Rng::new(0x0505);
    for _ in 0..N {
        pairs.push((rng.wild_v(), rng.wild_v()));
    }
    for _ in 0..N {
        pairs.push((rng.geom_v(), rng.geom_v()));
    }
    for (a, b) in pairs {
        let ca = unsafe { (c.c2Add)(a, b) };
        let ra = unsafe { (r.c2Add)(a, b) };
        assert!(
            veq(ca, ra),
            "c2Add({}, {}): C={} RUST={}",
            vshow(a),
            vshow(b),
            vshow(ca),
            vshow(ra)
        );
        let cs = unsafe { (c.c2Sub)(a, b) };
        let rss = unsafe { (r.c2Sub)(a, b) };
        assert!(
            veq(cs, rss),
            "c2Sub({}, {}): C={} RUST={}",
            vshow(a),
            vshow(b),
            vshow(cs),
            vshow(rss)
        );
    }
}

// --- rows 6 & 7 ------------------------------------------------------------
#[test]
fn row06_row07_c2Mulvs_c2Div() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<(C2v, f32)> = Vec::new();
    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            for &s in SPECIALS.iter() {
                cases.push((v(ax, ay), s));
            }
        }
    }
    for &b in SPECIAL_BITS.iter() {
        cases.push((v(1.0, -2.0), f32::from_bits(b)));
        cases.push((v(0.0, -0.0), f32::from_bits(b)));
    }
    let mut rng = Rng::new(0x0607);
    for _ in 0..N {
        cases.push((rng.wild_v(), rng.wild()));
    }
    for _ in 0..N {
        cases.push((rng.geom_v(), rng.geom()));
    }
    for (a, s) in cases {
        let cm = unsafe { (c.c2Mulvs)(a, s) };
        let rm = unsafe { (r.c2Mulvs)(a, s) };
        assert!(
            veq(cm, rm),
            "c2Mulvs({}, {}): C={} RUST={}",
            vshow(a),
            fshow(s),
            vshow(cm),
            vshow(rm)
        );
        let cd = unsafe { (c.c2Div)(a, s) };
        let rd = unsafe { (r.c2Div)(a, s) };
        assert!(
            veq(cd, rd),
            "c2Div({}, {}): C={} RUST={}",
            vshow(a),
            fshow(s),
            vshow(cd),
            vshow(rd)
        );
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn row08_c2Norm() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<C2v> = vec![
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(0.0, -0.0),
        v(1.0, 0.0),
        v(3.0, 4.0),
        v(f32::INFINITY, 1.0),
        v(f32::INFINITY, f32::INFINITY),
        v(f32::NAN, 1.0),
        v(1.0e30, 1.0e30),
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        v(f32::from_bits(1), f32::from_bits(1)),
    ];
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            cases.push(v(x, y));
        }
    }
    for &xb in SPECIAL_BITS.iter() {
        for &yb in SPECIAL_BITS.iter() {
            cases.push(v(f32::from_bits(xb), f32::from_bits(yb)));
        }
    }
    let mut rng = Rng::new(0x0808);
    for _ in 0..N {
        cases.push(rng.wild_v());
    }
    for _ in 0..N {
        cases.push(rng.geom_v());
    }
    for a in cases {
        let ca = unsafe { (c.c2Norm)(a) };
        let ra = unsafe { (r.c2Norm)(a) };
        assert!(
            veq(ca, ra),
            "c2Norm({}): C={} RUST={}",
            vshow(a),
            vshow(ca),
            vshow(ra)
        );
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn row09_c2Minv_c2Maxv() {
    let (c, r) = (c(), rs());
    let mut pairs: Vec<(C2v, C2v)> = Vec::new();
    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            for &bx in SPECIALS.iter() {
                for &by in SPECIALS.iter() {
                    pairs.push((v(ax, ay), v(bx, by)));
                }
            }
        }
    }
    let mut rng = Rng::new(0x0909);
    for _ in 0..N {
        pairs.push((rng.wild_v(), rng.wild_v()));
    }
    for (a, b) in pairs {
        let cm = unsafe { (c.c2Minv)(a, b) };
        let rm = unsafe { (r.c2Minv)(a, b) };
        assert!(
            veq(cm, rm),
            "c2Minv({}, {}): C={} RUST={}",
            vshow(a),
            vshow(b),
            vshow(cm),
            vshow(rm)
        );
        let cx = unsafe { (c.c2Maxv)(a, b) };
        let rx = unsafe { (r.c2Maxv)(a, b) };
        assert!(
            veq(cx, rx),
            "c2Maxv({}, {}): C={} RUST={}",
            vshow(a),
            vshow(b),
            vshow(cx),
            vshow(rx)
        );
    }
}

// --- rows 10 & 11 ----------------------------------------------------------
#[test]
fn row10_row11_c2Skew_c2CCW90_c2Absv() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<C2v> = Vec::new();
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            cases.push(v(x, y));
        }
    }
    for &xb in SPECIAL_BITS.iter() {
        for &yb in SPECIAL_BITS.iter() {
            cases.push(v(f32::from_bits(xb), f32::from_bits(yb)));
        }
    }
    let mut rng = Rng::new(0x1011);
    for _ in 0..N {
        cases.push(rng.wild_v());
    }
    for a in cases {
        for (nm, cf, rf) in [
            ("c2Skew", c.c2Skew, r.c2Skew),
            ("c2CCW90", c.c2CCW90, r.c2CCW90),
            ("c2Absv", c.c2Absv, r.c2Absv),
        ] {
            let ca = unsafe { cf(a) };
            let ra = unsafe { rf(a) };
            assert!(
                veq(ca, ra),
                "{nm}({}): C={} RUST={}",
                vshow(a),
                vshow(ca),
                vshow(ra)
            );
        }
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn row12_c2MulmvT() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<(C2m, C2v)> = Vec::new();
    for &a in SPECIALS.iter() {
        for &b in SPECIALS.iter() {
            cases.push((
                C2m {
                    x: v(a, b),
                    y: v(b, a),
                },
                v(a, b),
            ));
            cases.push((
                C2m {
                    x: v(a, a),
                    y: v(b, b),
                },
                v(b, a),
            ));
        }
    }
    let mut rng = Rng::new(0x1212);
    for _ in 0..N {
        cases.push((
            C2m {
                x: rng.wild_v(),
                y: rng.wild_v(),
            },
            rng.wild_v(),
        ));
    }
    for _ in 0..N {
        cases.push((
            C2m {
                x: rng.geom_v(),
                y: rng.geom_v(),
            },
            rng.geom_v(),
        ));
    }
    for (m, b) in cases {
        let ca = unsafe { (c.c2MulmvT)(m, b) };
        let ra = unsafe { (r.c2MulmvT)(m, b) };
        assert!(
            veq(ca, ra),
            "c2MulmvT({{{}, {}}}, {}): C={} RUST={}",
            vshow(m.x),
            vshow(m.y),
            vshow(b),
            vshow(ca),
            vshow(ra)
        );
    }
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn row13_identities() {
    let (c, r) = (c(), rs());
    for _ in 0..8 {
        let cr = unsafe { (c.c2RotIdentity)() };
        let rr = unsafe { (r.c2RotIdentity)() };
        assert!(
            feq(cr.c, rr.c) && feq(cr.s, rr.s),
            "c2RotIdentity: C={{{},{}}} RUST={{{},{}}}",
            fshow(cr.c),
            fshow(cr.s),
            fshow(rr.c),
            fshow(rr.s)
        );
        let cx = unsafe { (c.c2xIdentity)() };
        let rx = unsafe { (r.c2xIdentity)() };
        assert!(
            veq(cx.p, rx.p) && feq(cx.r.c, rx.r.c) && feq(cx.r.s, rx.r.s),
            "c2xIdentity mismatch"
        );
    }
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn row14_c2Mulrv_c2MulrvT() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<(C2r, C2v)> = Vec::new();
    // unit rotation sweep
    for i in 0..64 {
        let ang = (i as f32) * std::f32::consts::TAU / 64.0;
        let rot = C2r {
            c: ang.cos(),
            s: ang.sin(),
        };
        for k in 0..8 {
            let a = (k as f32) * 0.37;
            cases.push((rot, v(a, -a * 2.0)));
        }
    }
    // non-unit / degenerate rotations
    for &a in SPECIALS.iter() {
        for &b in SPECIALS.iter() {
            cases.push((C2r { c: a, s: b }, v(b, a)));
            cases.push((C2r { c: a, s: b }, v(1.0, -1.0)));
        }
    }
    cases.push((C2r { c: 0.0, s: 0.0 }, v(1.0, 2.0)));
    cases.push((C2r { c: 5.0, s: 7.0 }, v(1.0, 2.0)));
    let mut rng = Rng::new(0x1414);
    for _ in 0..N {
        cases.push((
            C2r {
                c: rng.wild(),
                s: rng.wild(),
            },
            rng.wild_v(),
        ));
    }
    for _ in 0..N {
        cases.push((
            C2r {
                c: rng.geom(),
                s: rng.geom(),
            },
            rng.geom_v(),
        ));
    }
    for (rot, b) in cases {
        let ca = unsafe { (c.c2Mulrv)(rot, b) };
        let ra = unsafe { (r.c2Mulrv)(rot, b) };
        assert!(
            veq(ca, ra),
            "c2Mulrv({{{},{}}}, {}): C={} RUST={}",
            fshow(rot.c),
            fshow(rot.s),
            vshow(b),
            vshow(ca),
            vshow(ra)
        );
        let ct = unsafe { (c.c2MulrvT)(rot, b) };
        let rt = unsafe { (r.c2MulrvT)(rot, b) };
        assert!(
            veq(ct, rt),
            "c2MulrvT({{{},{}}}, {}): C={} RUST={}",
            fshow(rot.c),
            fshow(rot.s),
            vshow(b),
            vshow(ct),
            vshow(rt)
        );
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn row15_c2MulxvT() {
    let (c, r) = (c(), rs());
    let mut cases: Vec<(C2x, C2v)> = Vec::new();
    let ident = C2x {
        p: v(0.0, 0.0),
        r: C2r { c: 1.0, s: 0.0 },
    };
    cases.push((ident, v(3.0, -4.0)));
    // pure translation
    for i in 0..16 {
        let t = i as f32 - 8.0;
        cases.push((
            C2x {
                p: v(t, -t),
                r: C2r { c: 1.0, s: 0.0 },
            },
            v(t * 2.0, t * 0.5),
        ));
    }
    // pure rotation + rotation & translation
    for i in 0..32 {
        let ang = (i as f32) * std::f32::consts::TAU / 32.0;
        let rot = C2r {
            c: ang.cos(),
            s: ang.sin(),
        };
        cases.push((C2x { p: v(0.0, 0.0), r: rot }, v(1.0, 2.0)));
        cases.push((
            C2x {
                p: v(i as f32 * 0.25, -(i as f32) * 0.75),
                r: rot,
            },
            v(-3.5, 7.25),
        ));
    }
    // non-unit / special
    for &a in SPECIALS.iter() {
        for &b in SPECIALS.iter() {
            cases.push((
                C2x {
                    p: v(a, b),
                    r: C2r { c: b, s: a },
                },
                v(a, b),
            ));
        }
    }
    let mut rng = Rng::new(0x1515);
    for _ in 0..N {
        cases.push((
            C2x {
                p: rng.wild_v(),
                r: C2r {
                    c: rng.wild(),
                    s: rng.wild(),
                },
            },
            rng.wild_v(),
        ));
    }
    for _ in 0..N {
        cases.push((
            C2x {
                p: rng.geom_v(),
                r: C2r {
                    c: rng.geom(),
                    s: rng.geom(),
                },
            },
            rng.geom_v(),
        ));
    }
    for (x, b) in cases {
        let ca = unsafe { (c.c2MulxvT)(x, b) };
        let ra = unsafe { (r.c2MulxvT)(x, b) };
        assert!(
            veq(ca, ra),
            "c2MulxvT({{p:{}, r:{{{},{}}}}}, {}): C={} RUST={}",
            vshow(x.p),
            fshow(x.r.c),
            fshow(x.r.s),
            vshow(b),
            vshow(ca),
            vshow(ra)
        );
    }
}
