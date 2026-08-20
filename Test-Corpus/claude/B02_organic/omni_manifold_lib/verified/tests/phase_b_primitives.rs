#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]
//! Phase B, CONFIGS.md rows 1-20: the scalar / vector / rotation primitives.
//!
//! These are the bottom of the call hierarchy, so a divergence here poisons
//! everything above it. Every row is driven with many pseudo-random inputs from a
//! fixed seed, and results are compared as raw bytes so `-0.0` and NaN payloads
//! count.

mod common;
use common::*;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Row 1: c2V
// ---------------------------------------------------------------------------

#[test]
fn row01_c2V() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_ff>("c2V");
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = if i < 8 {
            // fixed corner cases first
            let p = [(0.0f32, -0.0f32), (-0.0, 0.0), (f32::INFINITY, f32::NEG_INFINITY),
                     (f32::NAN, 1.0), (1.0, f32::NAN), (FLT_MAX, -FLT_MAX),
                     (f32::from_bits(0x7f80_0001), f32::from_bits(0xff80_0001)),
                     (f32::from_bits(1), f32::from_bits(0x8000_0001))][i];
            p
        } else {
            (rng.f_bits(), rng.f_bits())
        };
        let (c, r) = unsafe { (cf(x, y), rf(x, y)) };
        eq("c2V", &format!("x=0x{:08x} y=0x{:08x}", x.to_bits(), y.to_bits()), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Row 2: c2Neg, c2CCW90, c2Skew, c2Absv -- sign / payload preservation
// ---------------------------------------------------------------------------

#[test]
fn row02_unary_vector_ops() {
    let l = libs();
    let mut rng = Rng::new(2);
    let names = ["c2Neg", "c2CCW90", "c2Skew", "c2Absv"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnV_v>(n)).collect();
    for i in 0..N {
        let a = if i < SPECIAL_BITS.len() * SPECIAL_BITS.len() {
            v(
                f32::from_bits(SPECIAL_BITS[i / SPECIAL_BITS.len()]),
                f32::from_bits(SPECIAL_BITS[i % SPECIAL_BITS.len()]),
            )
        } else {
            rng.vec_bits()
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(a), rf(a)) };
            eq(name, &format!("a=(0x{:08x},0x{:08x})", a.x.to_bits(), a.y.to_bits()), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 3-4: c2Sub, c2Add
// ---------------------------------------------------------------------------

#[test]
fn row03_04_add_sub() {
    let l = libs();
    let names = ["c2Sub", "c2Add"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnV_vv>(n)).collect();
    let mut rng = Rng::new(3);
    for i in 0..N {
        // Row 3: well behaved; Row 4: arbitrary bit patterns.
        let (a, b) = if i % 2 == 0 {
            (rng.vec_norm(1e3), rng.vec_norm(1e3))
        } else {
            (rng.vec_special(), rng.vec_special())
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
            eq(name, &format!("a={a:?} b={b:?}"), &c, &r);
        }
    }
    // exhaustive special x special for both, so every NaN/inf operand pair is hit
    for &ax in SPECIAL_BITS.iter() {
        for &ay in SPECIAL_BITS.iter() {
            for &bx in SPECIAL_BITS.iter() {
                let a = v(f32::from_bits(ax), f32::from_bits(ay));
                let b = v(f32::from_bits(bx), f32::from_bits(ax));
                for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
                    let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                    eq(name, &format!("exhaustive a={a:?} b={b:?}"), &c, &r);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5-6: c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn row05_06_c2Mulvs() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vf>("c2Mulvs");
    let mut rng = Rng::new(5);
    for i in 0..N {
        let (a, s) = match i % 3 {
            0 => (rng.vec_norm(1e3), rng.f_norm(1e3)),
            1 => (rng.vec_norm(1e3), rng.f_special()),
            _ => (rng.vec_bits(), rng.f_bits()),
        };
        let (c, r) = unsafe { (cf(a, s), rf(a, s)) };
        eq("c2Mulvs", &format!("a={a:?} s=0x{:08x}", s.to_bits()), &c, &r);
    }
    // exhaustive: every special vector component x every special scalar
    for &ax in SPECIAL_BITS.iter() {
        for &ay in SPECIAL_BITS.iter() {
            for &sb in SPECIAL_BITS.iter() {
                let a = v(f32::from_bits(ax), f32::from_bits(ay));
                let s = f32::from_bits(sb);
                let (c, r) = unsafe { (cf(a, s), rf(a, s)) };
                eq("c2Mulvs", &format!("exhaustive a={a:?} s=0x{sb:08x}"), &c, &r);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7: c2Div
// ---------------------------------------------------------------------------

#[test]
fn row07_c2Div() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vf>("c2Div");
    let mut rng = Rng::new(7);
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.vec_norm(1e3), rng.f_norm(1e3)),
            1 => (rng.vec_norm(1e3), 0.0),
            2 => (rng.vec_norm(1e3), -0.0),
            _ => (rng.vec_bits(), rng.f_special()),
        };
        let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
        eq("c2Div", &format!("a={a:?} b=0x{:08x}", b.to_bits()), &c, &r);
    }
    for &ax in SPECIAL_BITS.iter() {
        for &ay in SPECIAL_BITS.iter() {
            for &bb in SPECIAL_BITS.iter() {
                let a = v(f32::from_bits(ax), f32::from_bits(ay));
                let b = f32::from_bits(bb);
                let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                eq("c2Div", &format!("exhaustive a={a:?} b=0x{bb:08x}"), &c, &r);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8-9: c2Dot, c2Det2
// ---------------------------------------------------------------------------

#[test]
fn row08_09_dot_det2() {
    let l = libs();
    let names = ["c2Dot", "c2Det2"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnF_vv>(n)).collect();
    let mut rng = Rng::new(8);
    for i in 0..N {
        let (a, b) = if i % 2 == 0 {
            (rng.vec_norm(1e3), rng.vec_norm(1e3))
        } else {
            (rng.vec_special(), rng.vec_special())
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
            eq_f32(name, &format!("a={a:?} b={b:?}"), c, r);
        }
    }
    // exhaustive over special components: catches `inf*0` and two-NaN addss/subss
    for &ax in SPECIAL_BITS.iter() {
        for &ay in SPECIAL_BITS.iter() {
            for &bx in SPECIAL_BITS.iter() {
                for &by in SPECIAL_BITS.iter() {
                    let a = v(f32::from_bits(ax), f32::from_bits(ay));
                    let b = v(f32::from_bits(bx), f32::from_bits(by));
                    for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
                        let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                        eq_f32(name, &format!("exhaustive a={a:?} b={b:?}"), c, r);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11: c2Len, c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row10_11_len_norm() {
    let l = libs();
    let (clen, rlen) = l.get::<FnF_v>("c2Len");
    let (cn, rn) = l.get::<FnV_v>("c2Norm");
    let mut rng = Rng::new(10);
    for i in 0..N {
        let a = match i % 5 {
            0 => rng.vec_norm(1e3),
            1 => rng.vec_norm(1e-20), // underflows when squared
            2 => rng.vec_norm(1e30),  // overflows when squared
            3 => v(0.0, 0.0),
            _ => rng.vec_special(),
        };
        let ctx = format!("a=(0x{:08x},0x{:08x})", a.x.to_bits(), a.y.to_bits());
        let (c, r) = unsafe { (clen(a), rlen(a)) };
        eq_f32("c2Len", &ctx, c, r);
        let (c, r) = unsafe { (cn(a), rn(a)) };
        eq("c2Norm", &ctx, &c, &r);
    }
    for &ax in SPECIAL_BITS.iter() {
        for &ay in SPECIAL_BITS.iter() {
            let a = v(f32::from_bits(ax), f32::from_bits(ay));
            let ctx = format!("exhaustive a={a:?}");
            let (c, r) = unsafe { (clen(a), rlen(a)) };
            eq_f32("c2Len", &ctx, c, r);
            let (c, r) = unsafe { (cn(a), rn(a)) };
            eq("c2Norm", &ctx, &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12: c2Maxv, c2Minv -- NaN picks the SECOND operand in C
// ---------------------------------------------------------------------------

#[test]
fn row12_maxv_minv() {
    let l = libs();
    let names = ["c2Maxv", "c2Minv"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnV_vv>(n)).collect();
    let mut rng = Rng::new(12);
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.vec_norm(10.0), rng.vec_norm(10.0)),
            1 => {
                let x = rng.vec_norm(10.0);
                (x, x) // equal
            }
            2 => (v(0.0, -0.0), v(-0.0, 0.0)),
            _ => (rng.vec_special(), rng.vec_special()),
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
            eq(name, &format!("a={a:?} b={b:?}"), &c, &r);
        }
    }
    for &ax in SPECIAL_BITS.iter() {
        for &bx in SPECIAL_BITS.iter() {
            for &ay in SPECIAL_BITS.iter() {
                let a = v(f32::from_bits(ax), f32::from_bits(ay));
                let b = v(f32::from_bits(bx), f32::from_bits(bx));
                for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
                    let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                    eq(name, &format!("exhaustive a={a:?} b={b:?}"), &c, &r);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13: c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn row13_c2Clampv() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vvv>("c2Clampv");
    let mut rng = Rng::new(13);
    for i in 0..N {
        let (a, lo, hi) = match i % 6 {
            0 => {
                // proper range, a inside
                let lo = rng.vec_norm(10.0);
                let hi = v(lo.x + rng.f_pos(10.0), lo.y + rng.f_pos(10.0));
                (v(lo.x + rng.f_pos(hi.x - lo.x), lo.y + rng.f_pos(hi.y - lo.y)), lo, hi)
            }
            1 => {
                let lo = rng.vec_norm(10.0);
                let hi = v(lo.x + rng.f_pos(10.0), lo.y + rng.f_pos(10.0));
                (v(lo.x - rng.f_pos(5.0), lo.y - rng.f_pos(5.0)), lo, hi) // below
            }
            2 => {
                let lo = rng.vec_norm(10.0);
                let hi = v(lo.x + rng.f_pos(10.0), lo.y + rng.f_pos(10.0));
                (v(hi.x + rng.f_pos(5.0), hi.y + rng.f_pos(5.0)), lo, hi) // above
            }
            3 => {
                // inverted range
                let hi = rng.vec_norm(10.0);
                let lo = v(hi.x + rng.f_pos(10.0), hi.y + rng.f_pos(10.0));
                (rng.vec_norm(10.0), lo, hi)
            }
            4 => (rng.vec_special(), rng.vec_norm(10.0), rng.vec_norm(10.0)),
            _ => (rng.vec_special(), rng.vec_special(), rng.vec_special()),
        };
        let (c, r) = unsafe { (cf(a, lo, hi), rf(a, lo, hi)) };
        eq("c2Clampv", &format!("a={a:?} lo={lo:?} hi={hi:?}"), &c, &r);
    }
    // NaN in each position
    for &nan in [0x7fc0_1234u32, 0xffc0_5678, 0x7f80_0001].iter() {
        let n = f32::from_bits(nan);
        let cases = [
            (v(n, 1.0), v(-1.0, -1.0), v(2.0, 2.0)),
            (v(1.0, n), v(-1.0, -1.0), v(2.0, 2.0)),
            (v(1.0, 1.0), v(n, -1.0), v(2.0, 2.0)),
            (v(1.0, 1.0), v(-1.0, n), v(2.0, 2.0)),
            (v(1.0, 1.0), v(-1.0, -1.0), v(n, 2.0)),
            (v(1.0, 1.0), v(-1.0, -1.0), v(2.0, n)),
            (v(n, n), v(n, n), v(n, n)),
        ];
        for (a, lo, hi) in cases {
            let (c, r) = unsafe { (cf(a, lo, hi), rf(a, lo, hi)) };
            eq("c2Clampv", &format!("nan a={a:?} lo={lo:?} hi={hi:?}"), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14: c2Dist
// ---------------------------------------------------------------------------

#[test]
fn row14_c2Dist() {
    let l = libs();
    let (cf, rf) = l.get::<FnF_hv>("c2Dist");
    let mut rng = Rng::new(14);
    for i in 0..N {
        let (h, p) = match i % 4 {
            0 => (c2h { n: rng.vec_norm(1.0), d: rng.f_norm(10.0) }, rng.vec_norm(10.0)),
            1 => (c2h { n: v(0.0, 0.0), d: rng.f_norm(10.0) }, rng.vec_norm(10.0)),
            2 => (c2h { n: rng.vec_norm(1.0), d: rng.f_special() }, rng.vec_norm(10.0)),
            _ => (c2h { n: rng.vec_special(), d: rng.f_special() }, rng.vec_special()),
        };
        let (c, r) = unsafe { (cf(h, p), rf(h, p)) };
        eq_f32("c2Dist", &format!("h={h:?} p={p:?}"), c, r);
    }
    for &nx in SPECIAL_BITS.iter() {
        for &d in SPECIAL_BITS.iter() {
            for &px in SPECIAL_BITS.iter() {
                let h = c2h { n: v(f32::from_bits(nx), f32::from_bits(px)), d: f32::from_bits(d) };
                let p = v(f32::from_bits(px), f32::from_bits(nx));
                let (c, r) = unsafe { (cf(h, p), rf(h, p)) };
                eq_f32("c2Dist", &format!("exhaustive h={h:?} p={p:?}"), c, r);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15: c2Intersect
// ---------------------------------------------------------------------------

#[test]
fn row15_c2Intersect() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vvff>("c2Intersect");
    let mut rng = Rng::new(15);
    for i in 0..N {
        let (a, b, da, db) = match i % 6 {
            0 => (rng.vec_norm(10.0), rng.vec_norm(10.0), rng.f_norm(10.0), rng.f_norm(10.0)),
            1 => {
                let d = rng.f_norm(10.0);
                (rng.vec_norm(10.0), rng.vec_norm(10.0), d, d) // da == db -> x/0
            }
            2 => (rng.vec_norm(10.0), rng.vec_norm(10.0), 0.0, 0.0), // 0/0
            3 => (rng.vec_norm(10.0), rng.vec_norm(10.0), 0.0, -0.0),
            4 => (rng.vec_norm(10.0), rng.vec_norm(10.0), rng.f_special(), rng.f_special()),
            _ => (rng.vec_special(), rng.vec_special(), rng.f_special(), rng.f_special()),
        };
        let (c, r) = unsafe { (cf(a, b, da, db), rf(a, b, da, db)) };
        eq(
            "c2Intersect",
            &format!("a={a:?} b={b:?} da=0x{:08x} db=0x{:08x}", da.to_bits(), db.to_bits()),
            &c,
            &r,
        );
    }
    for &dab in SPECIAL_BITS.iter() {
        for &dbb in SPECIAL_BITS.iter() {
            let (a, b) = (v(1.0, -2.0), v(-3.0, 4.0));
            let (da, db) = (f32::from_bits(dab), f32::from_bits(dbb));
            let (c, r) = unsafe { (cf(a, b, da, db), rf(a, b, da, db)) };
            eq("c2Intersect", &format!("exhaustive da=0x{dab:08x} db=0x{dbb:08x}"), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16: c2RotIdentity, c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn row16_identities() {
    let l = libs();
    let (cr, rr) = l.get::<FnR_void>("c2RotIdentity");
    let (c, r) = unsafe { (cr(), rr()) };
    eq("c2RotIdentity", "no args", &c, &r);
    assert_eq!((c.c, c.s), (1.0, 0.0));

    let (cx, rx) = l.get::<FnX_void>("c2xIdentity");
    let (c, r) = unsafe { (cx(), rx()) };
    eq("c2xIdentity", "no args", &c, &r);
    assert_eq!((c.p.x, c.p.y, c.r.c, c.r.s), (0.0, 0.0, 1.0, 0.0));
}

// ---------------------------------------------------------------------------
// Row 17: c2Mulrv, c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn row17_rotate() {
    let l = libs();
    let names = ["c2Mulrv", "c2MulrvT"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnV_rv>(n)).collect();
    let mut rng = Rng::new(17);
    for i in 0..N {
        let (rot, b) = match i % 5 {
            0 => (c2r { c: 1.0, s: 0.0 }, rng.vec_norm(1e3)),
            1 => {
                let t = rng.f_pos(std::f32::consts::TAU);
                (c2r { c: t.cos(), s: t.sin() }, rng.vec_norm(1e3))
            }
            2 => (c2r { c: rng.f_norm(5.0), s: rng.f_norm(5.0) }, rng.vec_norm(1e3)),
            3 => (c2r { c: 0.0, s: 0.0 }, rng.vec_norm(1e3)),
            _ => (c2r { c: rng.f_special(), s: rng.f_special() }, rng.vec_special()),
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(rot, b), rf(rot, b)) };
            eq(name, &format!("rot={rot:?} b={b:?}"), &c, &r);
        }
    }
    for &rc in SPECIAL_BITS.iter() {
        for &rs in SPECIAL_BITS.iter() {
            for &bx in SPECIAL_BITS.iter() {
                let rot = c2r { c: f32::from_bits(rc), s: f32::from_bits(rs) };
                let b = v(f32::from_bits(bx), f32::from_bits(rs));
                for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
                    let (c, r) = unsafe { (cf(rot, b), rf(rot, b)) };
                    eq(name, &format!("exhaustive rot={rot:?} b={b:?}"), &c, &r);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18: c2Mulxv, c2MulxvT
// ---------------------------------------------------------------------------

#[test]
fn row18_transform() {
    let l = libs();
    let names = ["c2Mulxv", "c2MulxvT"];
    let fns: Vec<_> = names.iter().map(|n| l.get::<FnV_xv>(n)).collect();
    let mut rng = Rng::new(18);
    for i in 0..N {
        let (x, b) = match i % 6 {
            0 => (x_identity(), rng.vec_norm(1e3)),
            1 => (c2x { p: rng.vec_norm(1e3), r: c2r { c: 1.0, s: 0.0 } }, rng.vec_norm(1e3)),
            2 => {
                let t = rng.f_pos(std::f32::consts::TAU);
                (c2x { p: v(0.0, 0.0), r: c2r { c: t.cos(), s: t.sin() } }, rng.vec_norm(1e3))
            }
            3 => (rng.xform(1e3), rng.vec_norm(1e3)),
            4 => (
                c2x { p: rng.vec_special(), r: c2r { c: rng.f_special(), s: rng.f_special() } },
                rng.vec_norm(1e3),
            ),
            _ => (
                c2x { p: rng.vec_special(), r: c2r { c: rng.f_special(), s: rng.f_special() } },
                rng.vec_special(),
            ),
        };
        for (name, (cf, rf)) in names.iter().zip(fns.iter()) {
            let (c, r) = unsafe { (cf(x, b), rf(x, b)) };
            eq(name, &format!("x={x:?} b={b:?}"), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19: c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn row19_c2BBVerts() {
    let l = libs();
    let (cf, rf) = l.get::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(19);
    for i in 0..N {
        let bb = match i % 5 {
            0 => {
                let min = rng.vec_norm(100.0);
                c2AABB { min, max: v(min.x + rng.f_pos(50.0), min.y + rng.f_pos(50.0)) }
            }
            1 => {
                // inverted
                let max = rng.vec_norm(100.0);
                c2AABB { min: v(max.x + rng.f_pos(50.0), max.y + rng.f_pos(50.0)), max }
            }
            2 => {
                let p = rng.vec_norm(100.0);
                c2AABB { min: p, max: p } // zero extent
            }
            3 => c2AABB { min: v(f32::NEG_INFINITY, f32::NEG_INFINITY), max: v(f32::INFINITY, f32::INFINITY) },
            _ => c2AABB { min: rng.vec_special(), max: rng.vec_special() },
        };
        // 8 slots so an off-by-one write would be caught
        let mut cout = [poison_v(3); 8];
        let mut rout = [poison_v(3); 8];
        let (mut cbb, mut rbb) = (bb, bb);
        unsafe {
            cf(cout.as_mut_ptr(), &mut cbb);
            rf(rout.as_mut_ptr(), &mut rbb);
        }
        eq("c2BBVerts out", &format!("bb={bb:?}"), &cout, &rout);
        eq("c2BBVerts bb (must not be modified)", &format!("bb={bb:?}"), &cbb, &rbb);
    }
}

// ---------------------------------------------------------------------------
// Row 20: c2PlaneAt over all in-range indices
// ---------------------------------------------------------------------------

#[test]
fn row20_c2PlaneAt() {
    let l = libs();
    let (cf, rf) = l.get::<FnH_polyi>("c2PlaneAt");
    let mut rng = Rng::new(20);
    for trial in 0..N / 8 {
        let mut p = c2Poly::default();
        p.count = 1 + rng.below(8) as i32;
        for k in 0..8 {
            p.verts[k] = if trial % 3 == 0 { rng.vec_special() } else { rng.vec_norm(50.0) };
            p.norms[k] = if trial % 3 == 0 { rng.vec_special() } else { rng.vec_norm(1.0) };
        }
        for i in 0..8i32 {
            let (c, r) = unsafe { (cf(&p, i), rf(&p, i)) };
            eq("c2PlaneAt", &format!("trial={trial} i={i}"), &c, &r);
        }
    }
}
