//! Adversarial bit-pattern fuzz.
//!
//! The C is compiled at `-O0`, so each float expression is one SSE instruction
//! with a fixed destination/source assignment; that assignment decides which
//! NaN payload and sign survives. LLVM is free to commute `fadd`/`fmul` and to
//! rewrite `-a*b + c` as `c - a*b`, which silently changes the answer for NaN
//! inputs. These tests feed *arbitrary 32-bit patterns* (signalling NaNs,
//! negative NaNs, infinities, denormals, QNaN-indefinite) into every exported
//! function so any remaining ordering mismatch shows up as a bit diff.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

fn pair() -> Pair {
    load_pair()
}

const ITERS: usize = 40_000;

/// Pool of the float values most likely to expose an ordering difference.
const POOL: [u32; 24] = [
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // +QNaN (Rust's f32::NAN)
    0xFFC0_0000, // -QNaN (x86 "indefinite")
    0x7F80_0001, // +SNaN
    0xFF80_0001, // -SNaN
    0x7FFF_FFFF, // +QNaN, all-ones payload
    0xFFFF_FFFF, // -QNaN, all-ones payload
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x0080_0000, // FLT_MIN
    0x8080_0000, // -FLT_MIN
    0x0000_0001, // smallest denormal
    0x8000_0001, // -smallest denormal
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x3400_0000, // FLT_EPSILON-ish
    0x4000_0000, // 2.0
    0x4348_0000, // 200.0
    0xC348_0000, // -200.0
    0x3F00_0000, // 0.5
    0x0000_0000,
];

/// Mostly-pool values with a sprinkling of fully random bit patterns.
fn f(g: &mut Rng) -> f32 {
    match g.below(10) {
        0..=6 => f32::from_bits(POOL[g.below(POOL.len() as u32) as usize]),
        7 => f32::from_bits(g.next_u32()),
        8 => f32::from_bits(0x7F80_0000 | g.below(0x0080_0000)), // some NaN/inf
        _ => f32::from_bits(0xFF80_0000 | g.below(0x0080_0000)),
    }
}

fn v(g: &mut Rng) -> c2v {
    c2v { x: f(g), y: f(g) }
}
fn r(g: &mut Rng) -> c2r {
    c2r { c: f(g), s: f(g) }
}
fn xf(g: &mut Rng) -> c2x {
    c2x { p: v(g), r: r(g) }
}

// ---------------------------------------------------------------------------
// Leaf vector algebra
// ---------------------------------------------------------------------------

#[test]
fn fuzz_leaf_vector_ops() {
    let p = pair();
    let mut g = Rng::new(0xF117);
    for i in 0..ITERS {
        let a = v(&mut g);
        let b = v(&mut g);
        let s = f(&mut g);
        let ctx = format!(
            "fuzz_leaf #{i} a=({:#010x},{:#010x}) b=({:#010x},{:#010x}) s={:#010x}",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits(),
            s.to_bits()
        );
        diff(&ctx, &p, |x| unsafe {
            (
                ((x.c2V)(a.x, a.y), (x.c2Sub)(a, b), (x.c2Add)(a, b)),
                ((x.c2Neg)(a), (x.c2Skew)(a), (x.c2CCW90)(a)),
                ((x.c2Mulvs)(a, s), (x.c2Div)(a, s), (x.c2Len)(a)),
                ((x.c2Dot)(a, b), (x.c2Det2)(a, b), (x.c2Norm)(a)),
            )
        });
        diff(&format!("{ctx} [minmax]"), &p, |x| unsafe {
            (
                (x.c2Maxv)(a, b),
                (x.c2Minv)(a, b),
                (x.c2Clampv)(a, b, v(&mut Rng::new(i as u64))),
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Rotations / transforms
// ---------------------------------------------------------------------------

#[test]
fn fuzz_rotations_and_transforms() {
    let p = pair();
    let mut g = Rng::new(0xF118);
    for i in 0..ITERS {
        let rot = r(&mut g);
        let b = v(&mut g);
        let t = xf(&mut g);
        let ctx = format!(
            "fuzz_rot #{i} r=({:#010x},{:#010x}) b=({:#010x},{:#010x})",
            rot.c.to_bits(),
            rot.s.to_bits(),
            b.x.to_bits(),
            b.y.to_bits()
        );
        diff(&ctx, &p, |x| unsafe {
            (
                (x.c2Mulrv)(rot, b),
                (x.c2MulrvT)(rot, b),
                (x.c2Mulxv)(t, b),
                ((x.c2RotIdentity)(), (x.c2xIdentity)()),
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Simplex reductions with adversarial bit patterns
// ---------------------------------------------------------------------------

fn fuzz_simplex(g: &mut Rng, count: c_int) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.v[i].sA = v(g);
        s.v[i].sB = v(g);
        s.v[i].p = v(g);
        s.v[i].u = f(g);
        s.v[i].iA = g.below(8) as c_int;
        s.v[i].iB = g.below(8) as c_int;
    }
    s.div = f(g);
    s.count = count;
    s
}

#[test]
fn fuzz_simplex_reductions() {
    let p = pair();
    let mut g = Rng::new(0xF119);
    for i in 0..ITERS {
        let count = [1i32, 2, 3][i % 3];
        let s = fuzz_simplex(&mut g, count);
        let ctx = format!("fuzz_simplex #{i} count={count} s={s:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut s22 = s;
            let mut s23 = s;
            let mut sd = s;
            let mut sl = s;
            let mut sm = s;
            let mut sw = s;
            let mut wa = c2v { x: 1.5, y: -2.5 };
            let mut wb = c2v { x: -3.5, y: 4.5 };
            (x.c22)(&mut s22);
            (x.c23)(&mut s23);
            (x.c2Witness)(&mut sw, &mut wa, &mut wb);
            (
                (s22, s23),
                ((x.c2D)(&mut sd), (x.c2L)(&mut sl)),
                ((x.c2GJKSimplexMetric)(&mut sm), sm),
                ((wa, wb), sw),
            )
        });
    }
}

// ---------------------------------------------------------------------------
// c2Support / c2BBVerts / c2MakeProxy
// ---------------------------------------------------------------------------

#[test]
fn fuzz_support_bbverts_makeproxy() {
    let p = pair();
    let mut g = Rng::new(0xF11A);
    for i in 0..ITERS {
        let mut verts = [c2v::default(); 8];
        for k in 0..8 {
            verts[k] = v(&mut g);
        }
        let d = v(&mut g);
        let count = 1 + (i % 8) as c_int;
        let bb = c2AABB {
            min: v(&mut g),
            max: v(&mut g),
        };
        let ci = c2Circle {
            p: v(&mut g),
            r: f(&mut g),
        };
        let ka = c2Capsule {
            a: v(&mut g),
            b: v(&mut g),
            r: f(&mut g),
        };
        let ctx = format!("fuzz_proxy #{i} count={count}");
        diff(&ctx, &p, |x| unsafe {
            let mut out = [c2v::default(); 4];
            let mut bbm = bb;
            (x.c2BBVerts)(out.as_mut_ptr(), &mut bbm);
            let mut p1 = c2Proxy::default();
            let mut p2 = c2Proxy::default();
            let mut p3 = c2Proxy::default();
            let mut cim = ci;
            let mut bbm2 = bb;
            let mut kam = ka;
            (x.c2MakeProxy)(&mut cim as *mut _ as *const c_void, C2_TYPE_CIRCLE, &mut p1);
            (x.c2MakeProxy)(&mut bbm2 as *mut _ as *const c_void, C2_TYPE_AABB, &mut p2);
            (x.c2MakeProxy)(&mut kam as *mut _ as *const c_void, C2_TYPE_CAPSULE, &mut p3);
            (
                ((x.c2Support)(verts.as_ptr(), count, d), out),
                (p1, p2),
                p3,
                bbm,
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Whole-pipeline fuzz: c2GJK and every boolean wrapper
// ---------------------------------------------------------------------------

fn fuzz_shape(g: &mut Rng, ty: c_int) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: v(g),
            r: f(g),
        }),
        C2_TYPE_AABB => Shape::Aabb(c2AABB {
            min: v(g),
            max: v(g),
        }),
        _ => Shape::Capsule(c2Capsule {
            a: v(g),
            b: v(g),
            r: f(g),
        }),
    }
}

#[test]
fn fuzz_gjk_all_type_pairs() {
    let p = pair();
    let mut g = Rng::new(0xF11B);
    for i in 0..ITERS {
        let ta = TYPES[(i / 3) % 3];
        let tb = TYPES[i % 3];
        let sa = fuzz_shape(&mut g, ta);
        let sb = fuzz_shape(&mut g, tb);
        // Cycle the whole option surface alongside the fuzzed geometry.
        let o = GjkOpts {
            use_radius: (i % 2) as c_int,
            ax: if i % 3 == 0 { None } else { Some(xf(&mut g)) },
            bx: if i % 5 == 0 { None } else { Some(xf(&mut g)) },
            want_a: i % 7 != 0,
            want_b: i % 11 != 0,
            want_iters: i % 13 != 0,
            cache: if i % 4 == 0 {
                None
            } else {
                Some(c2GJKCache {
                    metric: f(&mut g),
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: f(&mut g),
                })
            },
        };
        let ctx = format!("fuzz_gjk #{i} A={sa:?} B={sb:?} opts={o:?}");
        diff(&ctx, &p, |x| call_gjk(x, &sa, &sb, &o));
    }
}

#[test]
fn fuzz_boolean_wrappers_and_collided() {
    let p = pair();
    let mut g = Rng::new(0xF11C);
    for i in 0..ITERS {
        let ca = c2Circle {
            p: v(&mut g),
            r: f(&mut g),
        };
        let cb = c2Circle {
            p: v(&mut g),
            r: f(&mut g),
        };
        let ba = c2AABB {
            min: v(&mut g),
            max: v(&mut g),
        };
        let bb = c2AABB {
            min: v(&mut g),
            max: v(&mut g),
        };
        let ka = c2Capsule {
            a: v(&mut g),
            b: v(&mut g),
            r: f(&mut g),
        };
        let kb = c2Capsule {
            a: v(&mut g),
            b: v(&mut g),
            r: f(&mut g),
        };
        let ctx = format!("fuzz_bool #{i} ca={ca:?} cb={cb:?} ba={ba:?} bb={bb:?} ka={ka:?} kb={kb:?}");
        diff(&ctx, &p, |x| unsafe {
            (
                (
                    (x.c2CircletoCircle)(ca, cb),
                    (x.c2CircletoAABB)(ca, ba),
                    (x.c2CircletoCapsule)(ca, ka),
                ),
                (
                    (x.c2AABBtoAABB)(ba, bb),
                    (x.c2AABBtoCapsule)(ba, ka),
                    (x.c2CapsuletoCapsule)(ka, kb),
                ),
            )
        });
        // c2Collided across all 9 pairs with the same fuzzed shapes.
        let shapes = [Shape::Circle(ca), Shape::Aabb(ba), Shape::Capsule(ka)];
        let others = [Shape::Circle(cb), Shape::Aabb(bb), Shape::Capsule(kb)];
        for sa in shapes.iter() {
            for sb in others.iter() {
                let (sa, sb) = (*sa, *sb);
                let ctx = format!("fuzz_collided #{i} {}x{}", sa.ty(), sb.ty());
                diff(&ctx, &p, |x| unsafe {
                    with_shape(&sa, |pa| {
                        with_shape(&sb, |pb| (x.c2Collided)(pa, sa.ty(), pb, sb.ty()))
                    })
                });
            }
        }
    }
}

#[test]
fn fuzz_public_capsule_entry_point() {
    let p = pair();
    let mut g = Rng::new(0xF11D);
    for i in 0..ITERS * 4 {
        let a = f(&mut g);
        let b = f(&mut g);
        let c = f(&mut g);
        let d = f(&mut g);
        let e = f(&mut g);
        let ctx = format!(
            "fuzz_capsule #{i} ({:#010x},{:#010x},{:#010x},{:#010x},{:#010x})",
            a.to_bits(),
            b.to_bits(),
            c.to_bits(),
            d.to_bits(),
            e.to_bits()
        );
        diff(&ctx, &p, |x| unsafe { (x.capsule)(a, b, c, d, e) });
    }
}
