//! Phase B rows 1-21: lowest-level entry points (vector math, proxy building,
#![allow(non_snake_case)]
//! support function). Every call goes through both `.so`s via libloading.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 4096;

fn pair() -> Pair {
    load_pair()
}

// --- row 1 -----------------------------------------------------------------
#[test]
fn row01_c2V() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..N {
        let (x, y) = (rng.wild_f32(), rng.wild_f32());
        unsafe {
            ck_v((p.c.c2V)(x, y), (p.r.c2V)(x, y), &format!("row01 i={i} x={x} y={y}"));
        }
    }
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn row02_c2Mulvs() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild_f32();
        unsafe {
            ck_v(
                (p.c.c2Mulvs)(a, b),
                (p.r.c2Mulvs)(a, b),
                &format!("row02 i={i} a=({},{}) b={b}", a.x, a.y),
            );
        }
    }
    // Overflow: huge * huge
    unsafe {
        let a = c2v { x: 3.0e38, y: -3.0e38 };
        ck_v((p.c.c2Mulvs)(a, 10.0), (p.r.c2Mulvs)(a, 10.0), "row02 overflow");
        ck_v((p.c.c2Mulvs)(a, 0.0), (p.r.c2Mulvs)(a, 0.0), "row02 times zero");
    }
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn row03_c2Add_c2Sub() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let a = rng.wild_v();
        let b = if i % 7 == 0 { a } else { rng.wild_v() };
        let ctx = format!("row03 i={i} a=({},{}) b=({},{})", a.x, a.y, b.x, b.y);
        unsafe {
            ck_v((p.c.c2Add)(a, b), (p.r.c2Add)(a, b), &format!("{ctx} add"));
            ck_v((p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b), &format!("{ctx} sub"));
        }
    }
    unsafe {
        let hi = c2v { x: f32::MAX, y: f32::MAX };
        ck_v((p.c.c2Add)(hi, hi), (p.r.c2Add)(hi, hi), "row03 overflow add");
        let inf = c2v { x: f32::INFINITY, y: f32::NEG_INFINITY };
        ck_v((p.c.c2Sub)(inf, inf), (p.r.c2Sub)(inf, inf), "row03 inf-inf");
    }
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn row04_c2Dot() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..N {
        let a = rng.wild_v();
        // orthogonal / antiparallel companions
        let b = match i % 5 {
            0 => c2v { x: -a.y, y: a.x },
            1 => c2v { x: -a.x, y: -a.y },
            _ => rng.wild_v(),
        };
        unsafe {
            ck_f(
                (p.c.c2Dot)(a, b),
                (p.r.c2Dot)(a, b),
                &format!("row04 i={i} a=({},{}) b=({},{})", a.x, a.y, b.x, b.y),
            );
        }
    }
    unsafe {
        let inf = c2v { x: f32::INFINITY, y: 0.0 };
        let z = c2v { x: 0.0, y: 1.0 };
        ck_f((p.c.c2Dot)(inf, z), (p.r.c2Dot)(inf, z), "row04 inf*0");
        let big = c2v { x: 2.0e19, y: 2.0e19 };
        ck_f((p.c.c2Dot)(big, big), (p.r.c2Dot)(big, big), "row04 overflow");
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn row05_c2Det2() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let a = rng.wild_v();
        let b = match i % 5 {
            0 => a,                                            // collinear -> 0
            1 => c2v { x: -a.x, y: -a.y },                      // antiparallel
            2 => unsafe { (p.c.c2Mulvs)(a, 2.5) },              // parallel
            _ => rng.wild_v(),
        };
        unsafe {
            ck_f(
                (p.c.c2Det2)(a, b),
                (p.r.c2Det2)(a, b),
                &format!("row05 i={i} a=({},{}) b=({},{})", a.x, a.y, b.x, b.y),
            );
        }
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn row06_c2Len() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let a = rng.wild_v();
        unsafe {
            ck_f((p.c.c2Len)(a), (p.r.c2Len)(a), &format!("row06 i={i} a=({},{})", a.x, a.y));
        }
    }
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 2.0e19, y: 2.0e19 },   // dot overflows -> inf
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 }, // subnormal
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ] {
        unsafe {
            ck_f((p.c.c2Len)(a), (p.r.c2Len)(a), &format!("row06 special ({},{})", a.x, a.y));
        }
    }
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn row07_c2Maxv_c2Minv() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let a = rng.wild_v();
        let b = if i % 6 == 0 { a } else { rng.wild_v() };
        let ctx = format!("row07 i={i} a=({},{}) b=({},{})", a.x, a.y, b.x, b.y);
        unsafe {
            ck_v((p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b), &format!("{ctx} max"));
            ck_v((p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b), &format!("{ctx} min"));
        }
    }
    // Signed-zero and NaN pairs: the C ternary picks `b` whenever the compare fails.
    let specials = [
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (c2v { x: f32::NAN, y: 1.0 }, c2v { x: 1.0, y: f32::NAN }),
        (c2v { x: f32::NAN, y: f32::NAN }, c2v { x: f32::NAN, y: f32::NAN }),
    ];
    for (i, (a, b)) in specials.iter().enumerate() {
        unsafe {
            ck_v((p.c.c2Maxv)(*a, *b), (p.r.c2Maxv)(*a, *b), &format!("row07 sp{i} max"));
            ck_v((p.c.c2Minv)(*a, *b), (p.r.c2Minv)(*a, *b), &format!("row07 sp{i} min"));
        }
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn row08_c2Clampv() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..N {
        let lo = rng.wild_v();
        let hi = match i % 4 {
            0 => lo,                // lo == hi
            1 => rng.wild_v(),      // possibly inverted
            _ => unsafe { (p.c.c2Add)(lo, c2v { x: rng.unit() * 10.0, y: rng.unit() * 10.0 }) },
        };
        let a = rng.wild_v();
        unsafe {
            ck_v(
                (p.c.c2Clampv)(a, lo, hi),
                (p.r.c2Clampv)(a, lo, hi),
                &format!(
                    "row08 i={i} a=({},{}) lo=({},{}) hi=({},{})",
                    a.x, a.y, lo.x, lo.y, hi.x, hi.y
                ),
            );
        }
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn row09_c2Neg_c2Skew_c2CCW90() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let a = rng.wild_v();
        let ctx = format!("row09 i={i} a=({},{})", a.x, a.y);
        unsafe {
            ck_v((p.c.c2Neg)(a), (p.r.c2Neg)(a), &format!("{ctx} neg"));
            ck_v((p.c.c2Skew)(a), (p.r.c2Skew)(a), &format!("{ctx} skew"));
            ck_v((p.c.c2CCW90)(a), (p.r.c2CCW90)(a), &format!("{ctx} ccw90"));
        }
    }
    // Sign of zero must survive negation identically.
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::NAN, y: f32::INFINITY },
    ] {
        unsafe {
            ck_v((p.c.c2Neg)(a), (p.r.c2Neg)(a), "row09 zero-sign neg");
            ck_v((p.c.c2Skew)(a), (p.r.c2Skew)(a), "row09 zero-sign skew");
            ck_v((p.c.c2CCW90)(a), (p.r.c2CCW90)(a), "row09 zero-sign ccw90");
        }
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn row10_c2Div_c2Norm() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild_f32();
        unsafe {
            ck_v(
                (p.c.c2Div)(a, b),
                (p.r.c2Div)(a, b),
                &format!("row10 i={i} div a=({},{}) b={b}", a.x, a.y),
            );
            ck_v(
                (p.c.c2Norm)(a),
                (p.r.c2Norm)(a),
                &format!("row10 i={i} norm a=({},{})", a.x, a.y),
            );
        }
    }
    for (a, b) in [
        (c2v { x: 1.0, y: 2.0 }, 0.0f32),
        (c2v { x: 1.0, y: 2.0 }, -0.0f32),
        (c2v { x: 0.0, y: 0.0 }, 0.0f32),
        (c2v { x: 1.0, y: 2.0 }, f32::INFINITY),
        (c2v { x: 1.0, y: 2.0 }, f32::NAN),
    ] {
        unsafe {
            ck_v((p.c.c2Div)(a, b), (p.r.c2Div)(a, b), "row10 div special");
        }
    }
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 3.0e38, y: 3.0e38 },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
        c2v { x: f32::NAN, y: 1.0 },
    ] {
        unsafe {
            ck_v((p.c.c2Norm)(a), (p.r.c2Norm)(a), "row10 norm special");
        }
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn row11_identities() {
    let p = pair();
    unsafe {
        let rc = (p.c.c2RotIdentity)();
        let rr = (p.r.c2RotIdentity)();
        ck_b(&rc, &rr, "row11 c2RotIdentity");
        let xc = (p.c.c2xIdentity)();
        let xr = (p.r.c2xIdentity)();
        ck_b(&xc, &xr, "row11 c2xIdentity");
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn row12_c2Mulrv_c2MulrvT() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..N {
        let r = rng.rot();
        let v = rng.wild_v();
        let ctx = format!("row12 i={i} r=({},{}) v=({},{})", r.c, r.s, v.x, v.y);
        unsafe {
            ck_v((p.c.c2Mulrv)(r, v), (p.r.c2Mulrv)(r, v), &format!("{ctx} mulrv"));
            ck_v((p.c.c2MulrvT)(r, v), (p.r.c2MulrvT)(r, v), &format!("{ctx} mulrvT"));
        }
    }
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn row13_c2Mulxv() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        let x = match i % 4 {
            0 => unsafe { (p.c.c2xIdentity)() },
            1 => c2x { p: c2v { x: 1.0e30, y: -1.0e30 }, r: rng.rot() },
            _ => rng.xform(1000.0),
        };
        let v = rng.wild_v();
        unsafe {
            ck_v(
                (p.c.c2Mulxv)(x, v),
                (p.r.c2Mulxv)(x, v),
                &format!(
                    "row13 i={i} x=(p=({},{}),r=({},{})) v=({},{})",
                    x.p.x, x.p.y, x.r.c, x.r.s, v.x, v.y
                ),
            );
        }
    }
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn row14_c2BBVerts() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 14);
    // Pre-filled 8-slot buffer: verifies both that the 4 written slots match
    // and that the untouched tail is left exactly as-is.
    let fill = [c2v { x: 7.5, y: -3.25 }; 8];
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild_v();
        let bb = match i % 5 {
            0 => c2AABB { min: a, max: a },
            1 => c2AABB { min: a, max: c2v { x: a.x, y: b.y } },
            2 => c2AABB { min: a, max: c2v { x: b.x, y: a.y } },
            3 => c2AABB { min: b, max: a },
            _ => c2AABB { min: a, max: b },
        };
        let mut oc = fill;
        let mut or = fill;
        let mut bbc = bb;
        let mut bbr = bb;
        unsafe {
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
            (p.r.c2BBVerts)(or.as_mut_ptr(), &mut bbr);
        }
        let ctx = format!(
            "row14 i={i} bb=min({},{}) max({},{})",
            bb.min.x, bb.min.y, bb.max.x, bb.max.y
        );
        ck_verts(&oc, &or, &ctx);
        ck_b(&bbc, &bbr, &format!("{ctx} (input must not change)"));
    }
}

// --- rows 15/16/17 ---------------------------------------------------------
fn proxy_prefill() -> c2Proxy {
    // A recognisable non-zero pattern so untouched fields are detectable.
    let mut pr = c2Proxy {
        radius: -12345.678,
        count: 0x7B7B_7B7B,
        verts: [c2v { x: 0.0, y: 0.0 }; 8],
    };
    for (i, v) in pr.verts.iter_mut().enumerate() {
        v.x = 1000.0 + i as f32;
        v.y = -1000.0 - i as f32;
    }
    pr
}

fn make_proxy_diff(p: &Pair, shape_ptr: *const c_void, ty: u32, ctx: &str) {
    let mut pc = proxy_prefill();
    let mut pr = proxy_prefill();
    unsafe {
        (p.c.c2MakeProxy)(shape_ptr, ty, &mut pc);
        (p.r.c2MakeProxy)(shape_ptr, ty, &mut pr);
    }
    ck_proxy(&pc, &pr, ctx);
}

#[test]
fn row15_c2MakeProxy_circle() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..N {
        let c = c2Circle {
            p: rng.wild_v(),
            r: rng.wild_f32(),
        };
        make_proxy_diff(
            &p,
            &c as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &format!("row15 i={i} circle p=({},{}) r={}", c.p.x, c.p.y, c.r),
        );
    }
}

#[test]
fn row16_c2MakeProxy_aabb() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild_v();
        let bb = match i % 4 {
            0 => c2AABB { min: a, max: a },
            1 => c2AABB { min: b, max: a },
            _ => c2AABB { min: a, max: b },
        };
        make_proxy_diff(
            &p,
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &format!("row16 i={i} aabb"),
        );
    }
}

#[test]
fn row17_c2MakeProxy_capsule() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N {
        let a = rng.wild_v();
        let cap = c2Capsule {
            a,
            b: if i % 5 == 0 { a } else { rng.wild_v() },
            r: rng.wild_f32(),
        };
        make_proxy_diff(
            &p,
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &format!("row17 i={i} capsule"),
        );
    }
}

// --- rows 18-21 ------------------------------------------------------------
fn support_sweep(count: i32, seed: u64, row: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        // Occasionally make all verts identical to exercise the tie path.
        let tie = i % 11 == 0;
        let base = rng.wild_v();
        for v in verts.iter_mut() {
            *v = if tie { base } else { rng.wild_v() };
        }
        let d = match i % 7 {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => c2v { x: f32::NAN, y: 1.0 },
            _ => rng.wild_v(),
        };
        unsafe {
            let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
            let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
            ck_i(
                rc,
                rr,
                &format!("{row} i={i} count={count} d=({},{})", d.x, d.y),
            );
        }
    }
}

#[test]
fn row18_c2Support_count1() {
    support_sweep(1, SEED ^ 18, "row18");
}

#[test]
fn row19_c2Support_count2() {
    support_sweep(2, SEED ^ 19, "row19");
}

#[test]
fn row20_c2Support_count4() {
    support_sweep(4, SEED ^ 20, "row20");
}

#[test]
fn row21_c2Support_count8() {
    support_sweep(8, SEED ^ 21, "row21");
}

/// Rows 18-21, but with the vertex array actually produced by `c2MakeProxy`,
/// i.e. the exact composition `c2GJK` performs.
#[test]
fn rows18_21_c2Support_over_real_proxies() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x1821);
    for i in 0..N {
        let ty = ALL_TYPES[(i % 3) as usize];
        let shape = rand_shape(&mut rng, ty, 200.0, 4);
        let mut pc = proxy_prefill();
        let mut pr = proxy_prefill();
        unsafe {
            (p.c.c2MakeProxy)(shape.as_ptr(), ty, &mut pc);
            (p.r.c2MakeProxy)(shape.as_ptr(), ty, &mut pr);
        }
        ck_proxy(&pc, &pr, &format!("rows18_21 proxy i={i} {}", shape.describe()));
        let d = rng.wild_v();
        unsafe {
            let ic = (p.c.c2Support)(pc.verts.as_ptr(), pc.count, d);
            let ir = (p.r.c2Support)(pr.verts.as_ptr(), pr.count, d);
            ck_i(
                ic,
                ir,
                &format!("rows18_21 support i={i} {} d=({},{})", shape.describe(), d.x, d.y),
            );
        }
    }
}
