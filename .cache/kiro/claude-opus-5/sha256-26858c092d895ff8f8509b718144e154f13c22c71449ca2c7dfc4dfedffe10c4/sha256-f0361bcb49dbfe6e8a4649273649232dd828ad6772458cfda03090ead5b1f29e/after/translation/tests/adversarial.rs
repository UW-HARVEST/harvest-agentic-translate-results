//! Adversarial brute-force sweeps.
//!
//! The riskiest part of the translation is the hand-written SSE emulation in
//! `mulss` / `addss` / `subss` (NaN quieting, operand priority, and the
//! `0xFFC00000` "indefinite" result). These tests hammer exactly those paths
//! with exhaustive bit-pattern cross-products and multi-million-iteration
//! random sweeps.
//!
//! Comparisons here are done on raw bits with no eager string formatting, so
//! the iteration counts can be large.

#![allow(non_snake_case)]

mod harness;
use harness::*;
use std::ffi::c_void;

/// A dense pool of NaN payloads (quiet & signalling, both signs, boundary
/// payloads) plus the finite/infinite neighbours that interact with them.
const NAN_POOL: &[u32] = &[
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7F80_0001, // +SNaN, min payload
    0xFF80_0001, // -SNaN, min payload
    0x7F80_1234,
    0xFF80_1234,
    0x7FBF_FFFF, // +SNaN, max payload
    0xFFBF_FFFF, // -SNaN, max payload
    0x7FC0_0000, // +QNaN, empty payload
    0xFFC0_0000, // -QNaN indefinite
    0x7FC0_0001,
    0xFFC0_0001,
    0x7FDE_AD00,
    0xFFDE_AD00,
    0x7FFF_FFFF, // +QNaN, max payload
    0xFFFF_FFFF, // -QNaN, max payload
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x0000_0001, // min subnormal
    0x0080_0000, // min normal
];

#[track_caller]
fn chk_f(name: &str, a: C2v, b: C2v, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{name} diverged\n  a = (0x{:08x}, 0x{:08x})\n  b = (0x{:08x}, 0x{:08x})\n  C    = \
             0x{:08x}\n  Rust = 0x{:08x}",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits(),
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
fn chk_v(name: &str, a: C2v, b: C2v, c: C2v, r: C2v) {
    if bits_v(c) != bits_v(r) {
        panic!(
            "{name} diverged\n  a = (0x{:08x}, 0x{:08x})\n  b = (0x{:08x}, 0x{:08x})\n  C    = \
             (0x{:08x}, 0x{:08x})\n  Rust = (0x{:08x}, 0x{:08x})",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits(),
            c.x.to_bits(),
            c.y.to_bits(),
            r.x.to_bits(),
            r.y.to_bits()
        );
    }
}

// ---------------------------------------------------------------------------
// Exhaustive 4-slot cross-product over NAN_POOL (24^4 = 331 776 argument sets)
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_nan_cross_product_dot_and_sub() {
    let (c, r) = pair();
    for &ax in NAN_POOL {
        for &ay in NAN_POOL {
            let a = v(f32::from_bits(ax), f32::from_bits(ay));
            for &bx in NAN_POOL {
                for &by in NAN_POOL {
                    let b = v(f32::from_bits(bx), f32::from_bits(by));
                    unsafe {
                        chk_f("c2Dot", a, b, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                        chk_v("c2Sub", a, b, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                        chk_v("c2Maxv", a, b, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                        chk_v("c2Minv", a, b, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                    }
                }
            }
        }
    }
}

/// Same pool, but through `c2Clampv` (three vectors ⇒ prune to the x component
/// varying over the full pool while y is driven by a rotating index, so all
/// six slots see every pattern).
#[test]
fn exhaustive_nan_cross_product_clampv() {
    let (c, r) = pair();
    let n = NAN_POOL.len();
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                for rot in 0..n {
                    let a = v(f32::from_bits(NAN_POOL[i]), f32::from_bits(NAN_POOL[(i + rot) % n]));
                    let lo = v(f32::from_bits(NAN_POOL[j]), f32::from_bits(NAN_POOL[(j + rot) % n]));
                    let hi = v(f32::from_bits(NAN_POOL[k]), f32::from_bits(NAN_POOL[(k + rot) % n]));
                    unsafe {
                        let cv = (c.c2Clampv)(a, lo, hi);
                        let rv = (r.c2Clampv)(a, lo, hi);
                        if bits_v(cv) != bits_v(rv) {
                            panic!(
                                "c2Clampv diverged: a=({:08x},{:08x}) lo=({:08x},{:08x}) \
                                 hi=({:08x},{:08x}) C=({:08x},{:08x}) Rust=({:08x},{:08x})",
                                a.x.to_bits(),
                                a.y.to_bits(),
                                lo.x.to_bits(),
                                lo.y.to_bits(),
                                hi.x.to_bits(),
                                hi.y.to_bits(),
                                cv.x.to_bits(),
                                cv.y.to_bits(),
                                rv.x.to_bits(),
                                rv.y.to_bits()
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Exhaustive predicate sweep over the same pool
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_nan_cross_product_predicates() {
    let (c, r) = pair();
    let n = NAN_POOL.len();
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                for l in 0..n {
                    let f = |x: usize| f32::from_bits(NAN_POOL[x]);
                    let A = circle(f(i), f(j), f(k));
                    let B = circle(f(l), f(k), f(j));
                    let bx = aabb(f(i), f(j), f(k), f(l));
                    let by = aabb(f(l), f(k), f(j), f(i));
                    unsafe {
                        let (cv, rv) = ((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
                        assert_eq!(cv, rv, "c2CircletoCircle diverged A={A:?} B={B:?}");
                        let (cv, rv) = ((c.c2CircletoAABB)(A, bx), (r.c2CircletoAABB)(A, bx));
                        assert_eq!(cv, rv, "c2CircletoAABB diverged A={A:?} B={bx:?}");
                        let (cv, rv) = ((c.c2AABBtoAABB)(bx, by), (r.c2AABBtoAABB)(bx, by));
                        assert_eq!(cv, rv, "c2AABBtoAABB diverged A={bx:?} B={by:?}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-million-iteration random sweeps
// ---------------------------------------------------------------------------

/// Fully random 32-bit words: every float class, every NaN payload.
#[test]
fn random_sweep_uniform_bits() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xA5A5_5A5A);
    for _ in 0..1_000_000 {
        let a = v(rng.any_f32(), rng.any_f32());
        let b = v(rng.any_f32(), rng.any_f32());
        unsafe {
            chk_f("c2Dot", a, b, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            chk_v("c2Sub", a, b, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            chk_v("c2Maxv", a, b, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            chk_v("c2Minv", a, b, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
        }
    }
}

/// Biased towards NaN / inf / subnormal / boundary values, which uniform
/// random 32-bit words hit only ~0.4% of the time.
#[test]
fn random_sweep_biased_specials() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x1234_5678);
    let pick = |rng: &mut Rng| -> f32 {
        if rng.next_u32() % 4 == 0 {
            rng.interesting_f32()
        } else {
            f32::from_bits(NAN_POOL[(rng.next_u32() as usize) % NAN_POOL.len()])
        }
    };
    for _ in 0..500_000 {
        let a = v(pick(&mut rng), pick(&mut rng));
        let b = v(pick(&mut rng), pick(&mut rng));
        let ci = v(pick(&mut rng), pick(&mut rng));
        unsafe {
            chk_f("c2Dot", a, b, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            chk_v("c2Sub", a, b, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            chk_v("c2Maxv", a, b, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            chk_v("c2Minv", a, b, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
            chk_v("c2Clampv", a, b, (c.c2Clampv)(a, b, ci), (r.c2Clampv)(a, b, ci));
            let A = C2Circle { p: a, r: ci.x };
            let B = C2Circle { p: b, r: ci.y };
            let bx = C2Aabb { min: a, max: b };
            let by = C2Aabb { min: b, max: ci };
            assert_eq!((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B), "cc {A:?} {B:?}");
            assert_eq!((c.c2CircletoAABB)(A, bx), (r.c2CircletoAABB)(A, bx), "ca {A:?} {bx:?}");
            assert_eq!((c.c2AABBtoAABB)(bx, by), (r.c2AABBtoAABB)(bx, by), "aa {bx:?} {by:?}");
        }
    }
}

/// Random raw byte buffers through the public `collided` entry point, with
/// every tag pair including out-of-range ones.
#[test]
fn random_sweep_collided_raw_bytes() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x0FED_CBA9);
    let tags = [-2i32, -1, 0, 1, 2, 3, i32::MAX, i32::MIN];
    for _ in 0..200_000 {
        let mut ba = [0u32; 4];
        let mut bb = [0u32; 4];
        for i in 0..4 {
            ba[i] = if rng.next_u32() % 3 == 0 {
                NAN_POOL[(rng.next_u32() as usize) % NAN_POOL.len()]
            } else {
                rng.next_u32()
            };
            bb[i] = if rng.next_u32() % 3 == 0 {
                NAN_POOL[(rng.next_u32() as usize) % NAN_POOL.len()]
            } else {
                rng.next_u32()
            };
        }
        let pa = ba.as_ptr() as *const c_void;
        let pb = bb.as_ptr() as *const c_void;
        let ta = tags[(rng.next_u32() as usize) % tags.len()];
        let tb = tags[(rng.next_u32() as usize) % tags.len()];
        unsafe {
            let cv = (c.collided)(pa, ta, pb, tb);
            let rv = (r.collided)(pa, ta, pb, tb);
            if cv != rv {
                panic!(
                    "collided diverged: A={ba:08x?} typeA={ta} B={bb:08x?} typeB={tb} -> C={cv} \
                     Rust={rv}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rounding: dense grids that stress the mulss/addss rounding path
// ---------------------------------------------------------------------------

/// Values spread across the whole exponent range with random mantissas, so
/// `a.x*b.x + a.y*b.y` overflows, underflows, and cancels in every regime.
#[test]
fn exponent_grid_dot_rounding() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xBEEF_0001);
    for ea in 0..255u32 {
        for eb in 0..255u32 {
            for _ in 0..12 {
                let mk = |e: u32, rng: &mut Rng| -> f32 {
                    let m = rng.next_u32() & 0x007F_FFFF;
                    let s = (rng.next_u32() & 1) << 31;
                    f32::from_bits(s | (e << 23) | m)
                };
                let a = v(mk(ea, &mut rng), mk(eb, &mut rng));
                let b = v(mk(eb, &mut rng), mk(ea, &mut rng));
                unsafe {
                    chk_f("c2Dot", a, b, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                    chk_v("c2Sub", a, b, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                }
            }
        }
    }
}

/// Adjacent-ULP pairs around the `d2 < r2` decision boundary, swept across
/// many magnitudes: catches any rounding difference that would flip the
/// predicate.
#[test]
fn ulp_boundary_sweep_predicates() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xBEEF_0002);
    for _ in 0..50_000 {
        let scale = f32::from_bits(((rng.next_u32() % 200 + 20) << 23) | (rng.next_u32() & 0x7FFFFF));
        let ra = scale;
        let rb = scale * 0.5;
        let d = ra + rb;
        for delta in -4i32..=4 {
            let dd = f32::from_bits((d.to_bits() as i32 + delta).max(0) as u32);
            let A = circle(0.0, 0.0, ra);
            let B = circle(dd, 0.0, rb);
            unsafe {
                assert_eq!(
                    (c.c2CircletoCircle)(A, B),
                    (r.c2CircletoCircle)(A, B),
                    "cc ulp boundary A={A:?} B={B:?}"
                );
            }
            let bx = aabb(dd, 0.0, dd + scale, scale);
            let Ac = circle(0.0, 0.0, ra);
            unsafe {
                assert_eq!(
                    (c.c2CircletoAABB)(Ac, bx),
                    (r.c2CircletoAABB)(Ac, bx),
                    "ca ulp boundary A={Ac:?} B={bx:?}"
                );
                let ay = aabb(0.0, 0.0, scale, scale);
                assert_eq!(
                    (c.c2AABBtoAABB)(ay, bx),
                    (r.c2AABBtoAABB)(ay, bx),
                    "aa ulp boundary"
                );
            }
        }
    }
}
