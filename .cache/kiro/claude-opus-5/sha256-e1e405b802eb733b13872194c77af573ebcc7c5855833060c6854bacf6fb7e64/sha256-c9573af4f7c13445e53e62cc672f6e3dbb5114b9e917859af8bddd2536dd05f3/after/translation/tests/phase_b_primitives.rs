//! Phase B, Tier 1 + Tier 2: `CONFIGS.md` rows 1-18.
//!
//! Vector / rotation primitives and proxy construction, driven directly through
//! the exported C symbols of both shared objects.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 4000;

// ---------------------------------------------------------------------------
// Row 1
// ---------------------------------------------------------------------------

#[test]
fn row01_c2V() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnV>("c2V"), l.rs.sym::<FnV>("c2V"));
    let mut g = Rng::new(0x01);
    let mut rep = Report::new();
    for _ in 0..N {
        let (x, y) = (g.nasty_f32(), g.nasty_f32());
        let (a, b) = (c(x, y), r(x, y));
        rep.check(same_v(a, b), || {
            format!("c2V({}, {}): C={} Rust={}", show_f32(x), show_f32(y), show_v(a), show_v(b))
        });
    }
    rep.finish("row01_c2V");
}

// ---------------------------------------------------------------------------
// Row 2
// ---------------------------------------------------------------------------

#[test]
fn row02_c2Mulvs() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnVsV>("c2Mulvs"), l.rs.sym::<FnVsV>("c2Mulvs"));
    let mut g = Rng::new(0x02);
    let mut rep = Report::new();
    for _ in 0..N {
        let v = g.nasty_v();
        let s = g.nasty_f32();
        let (a, b) = (c(v, s), r(v, s));
        rep.check(same_v(a, b), || {
            format!("c2Mulvs({}, {}): C={} Rust={}", show_v(v), show_f32(s), show_v(a), show_v(b))
        });
    }
    rep.finish("row02_c2Mulvs");
}

// ---------------------------------------------------------------------------
// Row 3
// ---------------------------------------------------------------------------

#[test]
fn row03_c2Add_c2Sub() {
    let l = libs();
    let add = (l.c.sym::<FnVVV>("c2Add"), l.rs.sym::<FnVVV>("c2Add"));
    let sub = (l.c.sym::<FnVVV>("c2Sub"), l.rs.sym::<FnVVV>("c2Sub"));
    let mut g = Rng::new(0x03);
    let mut rep = Report::new();
    for i in 0..N {
        let u = g.nasty_v();
        // Every 4th case: exact cancellation.
        let v = if i % 4 == 0 { u } else { g.nasty_v() };
        let (a, b) = (add.0(u, v), add.1(u, v));
        rep.check(same_v(a, b), || {
            format!("c2Add({}, {}): C={} Rust={}", show_v(u), show_v(v), show_v(a), show_v(b))
        });
        let (a, b) = (sub.0(u, v), sub.1(u, v));
        rep.check(same_v(a, b), || {
            format!("c2Sub({}, {}): C={} Rust={}", show_v(u), show_v(v), show_v(a), show_v(b))
        });
    }
    rep.finish("row03_c2Add_c2Sub");
}

// ---------------------------------------------------------------------------
// Rows 4 + 5
// ---------------------------------------------------------------------------

#[test]
fn row04_row05_c2Dot_c2Det2() {
    let l = libs();
    let dot = (l.c.sym::<FnVVf>("c2Dot"), l.rs.sym::<FnVVf>("c2Dot"));
    let det = (l.c.sym::<FnVVf>("c2Det2"), l.rs.sym::<FnVVf>("c2Det2"));
    let mut g = Rng::new(0x04);
    let mut rep = Report::new();
    for i in 0..N {
        let u = g.nasty_v();
        let v = match i % 5 {
            0 => u,                                             // dot = |u|^2, det = 0
            1 => c2v { x: -u.y, y: u.x },                        // dot = 0
            2 => c2v { x: u.x * 2.0, y: u.y * 2.0 },             // collinear -> det = 0
            _ => g.nasty_v(),
        };
        let (a, b) = (dot.0(u, v), dot.1(u, v));
        rep.check(same_f32(a, b), || {
            format!("c2Dot({}, {}): C={} Rust={}", show_v(u), show_v(v), show_f32(a), show_f32(b))
        });
        let (a, b) = (det.0(u, v), det.1(u, v));
        rep.check(same_f32(a, b), || {
            format!("c2Det2({}, {}): C={} Rust={}", show_v(u), show_v(v), show_f32(a), show_f32(b))
        });
    }
    rep.finish("row04_row05_c2Dot_c2Det2");
}

// ---------------------------------------------------------------------------
// Row 6
// ---------------------------------------------------------------------------

#[test]
fn row06_c2Len() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnVf>("c2Len"), l.rs.sym::<FnVf>("c2Len"));
    let mut g = Rng::new(0x06);
    let mut rep = Report::new();
    for _ in 0..N {
        let v = g.nasty_v();
        let (a, b) = (c(v), r(v));
        rep.check(same_f32(a, b), || {
            format!("c2Len({}): C={} Rust={}", show_v(v), show_f32(a), show_f32(b))
        });
    }
    // Explicit corner cases: zero, huge (dot overflows to inf), tiny.
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ] {
        let (a, b) = (c(v), r(v));
        rep.check(same_f32(a, b), || {
            format!("c2Len({}): C={} Rust={}", show_v(v), show_f32(a), show_f32(b))
        });
    }
    rep.finish("row06_c2Len");
}

// ---------------------------------------------------------------------------
// Row 7 — the ternary-select (not fmaxf) NaN asymmetry
// ---------------------------------------------------------------------------

#[test]
fn row07_c2Maxv_c2Minv() {
    let l = libs();
    let mx = (l.c.sym::<FnVVV>("c2Maxv"), l.rs.sym::<FnVVV>("c2Maxv"));
    let mn = (l.c.sym::<FnVVV>("c2Minv"), l.rs.sym::<FnVVV>("c2Minv"));
    let mut g = Rng::new(0x07);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, u: c2v, v: c2v| {
        let (a, b) = (mx.0(u, v), mx.1(u, v));
        rep.check(same_v(a, b), || {
            format!("c2Maxv({}, {}): C={} Rust={}", show_v(u), show_v(v), show_v(a), show_v(b))
        });
        let (a, b) = (mn.0(u, v), mn.1(u, v));
        rep.check(same_v(a, b), || {
            format!("c2Minv({}, {}): C={} Rust={}", show_v(u), show_v(v), show_v(a), show_v(b))
        });
    };

    for i in 0..N {
        let u = g.nasty_v();
        let v = if i % 6 == 0 { u } else { g.nasty_v() };
        probe(&mut rep, u, v);
    }
    // NaN in the first vs second operand, and +0 vs -0.
    let nan = f32::NAN;
    for (u, v) in [
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: -1.0 }),
        (c2v { x: 1.0, y: -1.0 }, c2v { x: nan, y: nan }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: nan, y: 0.0 }, c2v { x: nan, y: -0.0 }),
    ] {
        probe(&mut rep, u, v);
    }
    rep.finish("row07_c2Maxv_c2Minv");
}

// ---------------------------------------------------------------------------
// Row 8
// ---------------------------------------------------------------------------

#[test]
fn row08_c2Clampv() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnVVVV>("c2Clampv"), l.rs.sym::<FnVVVV>("c2Clampv"));
    let mut g = Rng::new(0x08);
    let mut rep = Report::new();
    for i in 0..N {
        let a = g.nasty_v();
        let p = g.nasty_v();
        let q = g.nasty_v();
        // Mix proper ranges, inverted ranges (lo > hi) and lo == hi.
        let (lo, hi) = match i % 4 {
            0 => (c2v { x: p.x.min(q.x), y: p.y.min(q.y) }, c2v { x: p.x.max(q.x), y: p.y.max(q.y) }),
            1 => (q, p),
            2 => (p, p),
            _ => (p, q),
        };
        let (x, y) = (c(a, lo, hi), r(a, lo, hi));
        rep.check(same_v(x, y), || {
            format!(
                "c2Clampv({}, lo={}, hi={}): C={} Rust={}",
                show_v(a),
                show_v(lo),
                show_v(hi),
                show_v(x),
                show_v(y)
            )
        });
    }
    rep.finish("row08_c2Clampv");
}

// ---------------------------------------------------------------------------
// Row 9
// ---------------------------------------------------------------------------

#[test]
fn row09_c2Neg_c2Skew_c2CCW90() {
    let l = libs();
    let pairs: [(&str, (libloading::Symbol<FnVV>, libloading::Symbol<FnVV>)); 3] = [
        ("c2Neg", (l.c.sym("c2Neg"), l.rs.sym("c2Neg"))),
        ("c2Skew", (l.c.sym("c2Skew"), l.rs.sym("c2Skew"))),
        ("c2CCW90", (l.c.sym("c2CCW90"), l.rs.sym("c2CCW90"))),
    ];
    let mut g = Rng::new(0x09);
    let mut rep = Report::new();
    for _ in 0..N {
        let v = g.nasty_v();
        for (name, (fc, fr)) in &pairs {
            let (a, b) = (fc(v), fr(v));
            rep.check(same_v(a, b), || {
                format!("{name}({}): C={} Rust={}", show_v(v), show_v(a), show_v(b))
            });
        }
    }
    // Signed zero: -(+0.0) must be -0.0 on both sides.
    for v in [c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: 0.0 }, c2v { x: 0.0, y: -0.0 }] {
        for (name, (fc, fr)) in &pairs {
            let (a, b) = (fc(v), fr(v));
            rep.check(same_v(a, b), || {
                format!("{name}({}): C={} Rust={}", show_v(v), show_v(a), show_v(b))
            });
        }
    }
    rep.finish("row09_c2Neg_c2Skew_c2CCW90");
}

// ---------------------------------------------------------------------------
// Rows 10 + 11
// ---------------------------------------------------------------------------

#[test]
fn row10_row11_c2Div_c2Norm() {
    let l = libs();
    let div = (l.c.sym::<FnVsV>("c2Div"), l.rs.sym::<FnVsV>("c2Div"));
    let norm = (l.c.sym::<FnVV>("c2Norm"), l.rs.sym::<FnVV>("c2Norm"));
    let mut g = Rng::new(0x0a);
    let mut rep = Report::new();
    for _ in 0..N {
        let v = g.nasty_v();
        let s = g.nasty_f32();
        let (a, b) = (div.0(v, s), div.1(v, s));
        rep.check(same_v(a, b), || {
            format!("c2Div({}, {}): C={} Rust={}", show_v(v), show_f32(s), show_v(a), show_v(b))
        });
        let (a, b) = (norm.0(v), norm.1(v));
        rep.check(same_v(a, b), || {
            format!("c2Norm({}): C={} Rust={}", show_v(v), show_v(a), show_v(b))
        });
    }
    // Divide by exactly zero, and normalize the zero vector (0 * inf -> NaN).
    for v in [c2v { x: 3.0, y: 4.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: f32::MAX, y: f32::MAX }] {
        for s in [0.0f32, -0.0, 1.0, f32::INFINITY, f32::NAN, f32::from_bits(1)] {
            let (a, b) = (div.0(v, s), div.1(v, s));
            rep.check(same_v(a, b), || {
                format!("c2Div({}, {}): C={} Rust={}", show_v(v), show_f32(s), show_v(a), show_v(b))
            });
        }
        let (a, b) = (norm.0(v), norm.1(v));
        rep.check(same_v(a, b), || {
            format!("c2Norm({}): C={} Rust={}", show_v(v), show_v(a), show_v(b))
        });
    }
    rep.finish("row10_row11_c2Div_c2Norm");
}

// ---------------------------------------------------------------------------
// Row 12 — nullary struct returns (checks the c2x SSE,SSE return class)
// ---------------------------------------------------------------------------

#[test]
fn row12_identities() {
    let l = libs();
    let rot = (l.c.sym::<FnR>("c2RotIdentity"), l.rs.sym::<FnR>("c2RotIdentity"));
    let xf = (l.c.sym::<FnX>("c2xIdentity"), l.rs.sym::<FnX>("c2xIdentity"));
    let mut rep = Report::new();
    // Called repeatedly: a wrong return-value class often only shows up when
    // register state is dirty from a previous call.
    for _ in 0..64 {
        let (a, b) = (rot.0(), rot.1());
        rep.check(same_r(a, b), || {
            format!("c2RotIdentity: C=({},{}) Rust=({},{})", show_f32(a.c), show_f32(a.s), show_f32(b.c), show_f32(b.s))
        });
        let (a, b) = (xf.0(), xf.1());
        rep.check(same_x(a, b), || {
            format!("c2xIdentity: C p={} r=({},{}) | Rust p={} r=({},{})",
                show_v(a.p), show_f32(a.r.c), show_f32(a.r.s),
                show_v(b.p), show_f32(b.r.c), show_f32(b.r.s))
        });
    }
    rep.finish("row12_identities");
}

// ---------------------------------------------------------------------------
// Row 13
// ---------------------------------------------------------------------------

#[test]
fn row13_c2Mulrv_c2MulrvT() {
    let l = libs();
    let fwd = (l.c.sym::<FnRVV>("c2Mulrv"), l.rs.sym::<FnRVV>("c2Mulrv"));
    let bwd = (l.c.sym::<FnRVV>("c2MulrvT"), l.rs.sym::<FnRVV>("c2MulrvT"));
    let mut g = Rng::new(0x0d);
    let mut rep = Report::new();
    for _ in 0..N {
        let rr = g.rot();
        let v = g.nasty_v();
        let (a, b) = (fwd.0(rr, v), fwd.1(rr, v));
        rep.check(same_v(a, b), || {
            format!("c2Mulrv(({},{}), {}): C={} Rust={}", show_f32(rr.c), show_f32(rr.s), show_v(v), show_v(a), show_v(b))
        });
        let (a2, b2) = (bwd.0(rr, v), bwd.1(rr, v));
        rep.check(same_v(a2, b2), || {
            format!("c2MulrvT(({},{}), {}): C={} Rust={}", show_f32(rr.c), show_f32(rr.s), show_v(v), show_v(a2), show_v(b2))
        });
        // Round-trip through both libraries (composition must agree too).
        let (rc, rr_) = (bwd.0(rr, fwd.0(rr, v)), bwd.1(rr, fwd.1(rr, v)));
        rep.check(same_v(rc, rr_), || {
            format!("MulrvT(Mulrv) round trip: C={} Rust={}", show_v(rc), show_v(rr_))
        });
    }
    // NaN rotation.
    for rr in [c2r { c: f32::NAN, s: 0.0 }, c2r { c: 0.0, s: f32::NAN }, c2r { c: 0.0, s: 0.0 }] {
        let v = c2v { x: 1.0, y: 2.0 };
        let (a, b) = (fwd.0(rr, v), fwd.1(rr, v));
        rep.check(same_v(a, b), || format!("c2Mulrv NaN: C={} Rust={}", show_v(a), show_v(b)));
        let (a, b) = (bwd.0(rr, v), bwd.1(rr, v));
        rep.check(same_v(a, b), || format!("c2MulrvT NaN: C={} Rust={}", show_v(a), show_v(b)));
    }
    rep.finish("row13_c2Mulrv_c2MulrvT");
}

// ---------------------------------------------------------------------------
// Row 14 — c2x passed by value (16-byte SSE,SSE argument class)
// ---------------------------------------------------------------------------

#[test]
fn row14_c2Mulxv() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnXVV>("c2Mulxv"), l.rs.sym::<FnXVV>("c2Mulxv"));
    let mut g = Rng::new(0x0e);
    let mut rep = Report::new();
    for i in 0..N {
        let v = g.nasty_v();
        let x = match i % 4 {
            0 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }, // identity
            1 => c2x { p: g.finite_v(), r: c2r { c: 1.0, s: 0.0 } },           // translation only
            2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: g.rot() },                // rotation only
            _ => g.xform(),
        };
        let (a, b) = (c(x, v), r(x, v));
        rep.check(same_v(a, b), || {
            format!(
                "c2Mulxv(p={} r=({},{}), {}): C={} Rust={}",
                show_v(x.p), show_f32(x.r.c), show_f32(x.r.s), show_v(v), show_v(a), show_v(b)
            )
        });
    }
    rep.finish("row14_c2Mulxv");
}

// ---------------------------------------------------------------------------
// Row 15 — c2BBVerts writes 4 vertices through an out-pointer
// ---------------------------------------------------------------------------

#[test]
fn row15_c2BBVerts() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnBBVerts>("c2BBVerts"), l.rs.sym::<FnBBVerts>("c2BBVerts"));
    let mut g = Rng::new(0x0f);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, mut bb: c2AABB| {
        // 8 slots with a poison pattern, so an over-write past vertex 3 shows up.
        let poison = c2v { x: f32::from_bits(0xDEADBEEF), y: f32::from_bits(0xCAFEBABE) };
        let mut oc = [poison; 8];
        let mut or_ = [poison; 8];
        let mut bb2 = bb;
        unsafe {
            c(oc.as_mut_ptr(), &raw mut bb);
            r(or_.as_mut_ptr(), &raw mut bb2);
        }
        for i in 0..8 {
            rep.check(same_v(oc[i], or_[i]), || {
                format!(
                    "c2BBVerts(min={} max={}) vert[{i}]: C={} Rust={}",
                    show_v(bb.min), show_v(bb.max), show_v(oc[i]), show_v(or_[i])
                )
            });
        }
        // The input struct must not be mutated by either side.
        rep.check(same_v(bb.min, bb2.min) && same_v(bb.max, bb2.max), || {
            "c2BBVerts mutated its input differently".to_string()
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.aabb());
    }
    for bb in [
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } }, // inverted
        c2AABB { min: c2v { x: f32::NAN, y: 1.0 }, max: c2v { x: 2.0, y: f32::INFINITY } },
        c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
    ] {
        probe(&mut rep, bb);
    }
    rep.finish("row15_c2BBVerts");
}

// ---------------------------------------------------------------------------
// Rows 16 + 17 + 18 — c2MakeProxy for each valid type
// ---------------------------------------------------------------------------

/// A fully-poisoned proxy: every byte 0xA5, so *any* field the C leaves
/// untouched is visible, and both sides start from the identical state.
fn poison_proxy() -> c2Proxy {
    unsafe { std::mem::transmute::<[u8; 72], c2Proxy>([0xA5u8; 72]) }
}

#[test]
fn row16_makeproxy_circle() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnMakeProxy>("c2MakeProxy"), l.rs.sym::<FnMakeProxy>("c2MakeProxy"));
    let mut g = Rng::new(0x10);
    let mut rep = Report::new();
    for i in 0..N {
        let mut sh = g.circle();
        if i % 7 == 0 {
            sh.r = [0.0f32, -3.0, f32::INFINITY, f32::NAN][i % 4];
        }
        let (mut pc, mut pr) = (poison_proxy(), poison_proxy());
        unsafe {
            c(&raw const sh as *const c_void, C2_TYPE_CIRCLE, &raw mut pc);
            r(&raw const sh as *const c_void, C2_TYPE_CIRCLE, &raw mut pr);
        }
        rep.check(same_proxy(&pc, &pr), || {
            format!(
                "c2MakeProxy(CIRCLE p={} r={}):\n  C:    {}\n  Rust: {}",
                show_v(sh.p), show_f32(sh.r), show_proxy(&pc), show_proxy(&pr)
            )
        });
    }
    rep.finish("row16_makeproxy_circle");
}

#[test]
fn row17_makeproxy_aabb() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnMakeProxy>("c2MakeProxy"), l.rs.sym::<FnMakeProxy>("c2MakeProxy"));
    let mut g = Rng::new(0x11);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, sh: c2AABB| {
        let (mut pc, mut pr) = (poison_proxy(), poison_proxy());
        unsafe {
            c(&raw const sh as *const c_void, C2_TYPE_AABB, &raw mut pc);
            r(&raw const sh as *const c_void, C2_TYPE_AABB, &raw mut pr);
        }
        rep.check(same_proxy(&pc, &pr), || {
            format!(
                "c2MakeProxy(AABB min={} max={}):\n  C:    {}\n  Rust: {}",
                show_v(sh.min), show_v(sh.max), show_proxy(&pc), show_proxy(&pr)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.aabb());
    }
    for bb in [
        c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB { min: c2v { x: 9.0, y: 9.0 }, max: c2v { x: -9.0, y: -9.0 } },
        c2AABB { min: c2v { x: f32::NEG_INFINITY, y: 0.0 }, max: c2v { x: f32::INFINITY, y: f32::NAN } },
    ] {
        probe(&mut rep, bb);
    }
    rep.finish("row17_makeproxy_aabb");
}

#[test]
fn row18_makeproxy_capsule() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnMakeProxy>("c2MakeProxy"), l.rs.sym::<FnMakeProxy>("c2MakeProxy"));
    let mut g = Rng::new(0x12);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, sh: c2Capsule| {
        let (mut pc, mut pr) = (poison_proxy(), poison_proxy());
        unsafe {
            c(&raw const sh as *const c_void, C2_TYPE_CAPSULE, &raw mut pc);
            r(&raw const sh as *const c_void, C2_TYPE_CAPSULE, &raw mut pr);
        }
        rep.check(same_proxy(&pc, &pr), || {
            format!(
                "c2MakeProxy(CAPSULE a={} b={} r={}):\n  C:    {}\n  Rust: {}",
                show_v(sh.a), show_v(sh.b), show_f32(sh.r), show_proxy(&pc), show_proxy(&pr)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.capsule());
    }
    for cap in [
        c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 1.0, y: 2.0 }, r: 0.0 }, // degenerate
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: -4.0 }, // negative r
        c2Capsule { a: c2v { x: f32::NAN, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: f32::INFINITY },
    ] {
        probe(&mut rep, cap);
    }
    rep.finish("row18_makeproxy_capsule");
}
