//! High-volume randomized differential fuzz over EVERY exported symbol.
//!
//! Phases B and C target specific branches; this file is the safety net that
//! blankets the whole input space with uniformly random bit patterns, which is
//! what catches value-dependent bugs (NaN-operand selection, wrap-around,
//! table indices) that a per-branch test can miss.
//!
//! Iteration count is `FUZZ_ITERS` (default 200_000 per function). Raise it for
//! a longer soak:
//!
//! ```text
//! FUZZ_ITERS=5000000 cargo test --release --test fuzz_deep
//! ```

mod common;

use common::*;
use std::ffi::{c_uint, c_void};

fn iters() -> usize {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000)
}

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct ShapeBuf([f32; 4]);

impl ShapeBuf {
    fn ptr(&self) -> *const c_void {
        self as *const ShapeBuf as *const c_void
    }
}

#[test]
fn fuzz_c2_primitives() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0xF00D);
    for _ in 0..n {
        let a = r.raw_c2v();
        let b = r.raw_c2v();
        let c = r.raw_c2v();
        let ctx = (
            (a.x.to_bits(), a.y.to_bits()),
            (b.x.to_bits(), b.y.to_bits()),
            (c.x.to_bits(), c.y.to_bits()),
        );
        same("fuzz c2V", ctx, unsafe { (p.c.c2V)(a.x, a.y) }, unsafe {
            (p.rs.c2V)(a.x, a.y)
        });
        same("fuzz c2Maxv", ctx, unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
            (p.rs.c2Maxv)(a, b)
        });
        same("fuzz c2Minv", ctx, unsafe { (p.c.c2Minv)(a, b) }, unsafe {
            (p.rs.c2Minv)(a, b)
        });
        same(
            "fuzz c2Clampv",
            ctx,
            unsafe { (p.c.c2Clampv)(a, b, c) },
            unsafe { (p.rs.c2Clampv)(a, b, c) },
        );
        same("fuzz c2Sub", ctx, unsafe { (p.c.c2Sub)(a, b) }, unsafe {
            (p.rs.c2Sub)(a, b)
        });
        same("fuzz c2Dot", ctx, unsafe { (p.c.c2Dot)(a, b) }, unsafe {
            (p.rs.c2Dot)(a, b)
        });
    }
}

#[test]
fn fuzz_c2_shape_tests() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0xBEEF);
    for i in 0..n {
        // alternate between raw bit patterns and clustered "plausible" geometry
        let (ca, cb, ba, bb);
        if i % 2 == 0 {
            ca = C2Circle {
                p: r.raw_c2v(),
                r: r.raw_f32(),
            };
            cb = C2Circle {
                p: r.raw_c2v(),
                r: r.raw_f32(),
            };
            ba = C2Aabb {
                min: r.raw_c2v(),
                max: r.raw_c2v(),
            };
            bb = C2Aabb {
                min: r.raw_c2v(),
                max: r.raw_c2v(),
            };
        } else {
            ca = r.circle(4.0);
            cb = r.circle(4.0);
            ba = r.aabb(4.0);
            bb = r.aabb(4.0);
        }
        let ctx = (
            (ca.p.x.to_bits(), ca.p.y.to_bits(), ca.r.to_bits()),
            (cb.p.x.to_bits(), cb.p.y.to_bits(), cb.r.to_bits()),
            (
                ba.min.x.to_bits(),
                ba.min.y.to_bits(),
                ba.max.x.to_bits(),
                ba.max.y.to_bits(),
            ),
        );
        same(
            "fuzz c2CircletoCircle",
            ctx,
            unsafe { (p.c.c2CircletoCircle)(ca, cb) },
            unsafe { (p.rs.c2CircletoCircle)(ca, cb) },
        );
        same(
            "fuzz c2CircletoAABB",
            ctx,
            unsafe { (p.c.c2CircletoAABB)(ca, ba) },
            unsafe { (p.rs.c2CircletoAABB)(ca, ba) },
        );
        same(
            "fuzz c2AABBtoAABB",
            ctx,
            unsafe { (p.c.c2AABBtoAABB)(ba, bb) },
            unsafe { (p.rs.c2AABBtoAABB)(ba, bb) },
        );
    }
}

#[test]
fn fuzz_f2_dispatch() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0xCAFE);
    for _ in 0..n {
        let a = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        let b = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        // mostly valid enum values, sometimes out-of-range ones
        let pick = |r: &mut Rng| -> c_uint {
            match r.below(8) {
                0..=2 => 0,
                3..=5 => 1,
                6 => 2,
                _ => r.next_u32(),
            }
        };
        let ta = pick(&mut r);
        let tb = pick(&mut r);
        same(
            "fuzz f2",
            (a.0.map(f32::to_bits), b.0.map(f32::to_bits), ta, tb),
            unsafe { (p.c.f2)(a.ptr(), ta, b.ptr(), tb) },
            unsafe { (p.rs.f2)(a.ptr(), ta, b.ptr(), tb) },
        );
    }
}

#[test]
fn fuzz_f3() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0x1234);
    for i in 0..n {
        let (v1, v2) = match i % 4 {
            0 => (r.next_i32(), r.next_i32()),
            1 => (r.edgy_i32(), r.edgy_i32()),
            2 => (r.next_i32() % 1000, r.next_i32() % 13),
            _ => (r.edgy_i32(), r.next_i32() % 7),
        };
        same(
            "fuzz f3",
            (v1, v2),
            unsafe { (p.c.f3)(v1, v2) },
            unsafe { (p.rs.f3)(v1, v2) },
        );
    }
}

#[test]
fn fuzz_f4() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0x5678);
    // long chains from random seeds so the state update is stressed
    let mut left = n;
    while left > 0 {
        let seed = [r.edgy_u64(), r.edgy_u64()];
        let mut sc = CnRnd { state: seed };
        let mut sr = CnRnd { state: seed };
        let steps = 64.min(left);
        for i in 0..steps {
            let cv = unsafe { (p.c.f4)(&mut sc) };
            let rv = unsafe { (p.rs.f4)(&mut sr) };
            same("fuzz f4", (seed, i), cv, rv);
            same("fuzz f4/state", (seed, i), sc.state, sr.state);
        }
        left -= steps;
    }
}

#[test]
fn fuzz_f5_f7() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0x9ABC);
    for i in 0..n {
        let a = if i % 2 == 0 { r.next_u32() } else { r.edgy_u32() };
        same("fuzz f5", a, unsafe { (p.c.f5)(a) }, unsafe { (p.rs.f5)(a) });

        let (bs, ch, bd) = match i % 3 {
            0 => (r.next_u32(), r.next_u32(), r.next_u32()),
            1 => (r.edgy_u32(), r.below(6), r.edgy_u32()),
            _ => (r.edgy_u32(), r.edgy_u32(), r.edgy_u32()),
        };
        same(
            "fuzz f7",
            (bs, ch, bd),
            unsafe { (p.c.f7)(bs, ch, bd) },
            unsafe { (p.rs.f7)(bs, ch, bd) },
        );
    }
}

#[test]
fn fuzz_f9() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0xDEF0);
    for i in 0..n {
        let (a, b, c, q);
        match i % 3 {
            0 => {
                a = r.raw_lmv();
                b = r.raw_lmv();
                c = r.raw_lmv();
                q = r.raw_lmv();
            }
            1 => {
                a = r.lmv(4.0);
                b = r.lmv(4.0);
                c = r.lmv(4.0);
                q = r.lmv(4.0);
            }
            _ => {
                // near-degenerate: c is almost collinear with a,b
                a = LmVec2 {
                    x: r.finite_f32(8.0),
                    y: r.finite_f32(8.0),
                };
                b = LmVec2 {
                    x: r.finite_f32(8.0),
                    y: r.finite_f32(8.0),
                };
                let t = r.finite_f32(2.0);
                let eps = f32::from_bits(r.next_u32() % 1024);
                c = LmVec2 {
                    x: a.x + t * (b.x - a.x) + eps,
                    y: a.y + t * (b.y - a.y),
                };
                q = LmVec2 {
                    x: r.finite_f32(8.0),
                    y: r.finite_f32(8.0),
                };
            }
        }
        same(
            "fuzz f9",
            (
                (a.x.to_bits(), a.y.to_bits()),
                (b.x.to_bits(), b.y.to_bits()),
                (c.x.to_bits(), c.y.to_bits()),
                (q.x.to_bits(), q.y.to_bits()),
            ),
            unsafe { (p.c.f9)(a, b, c, q) },
            unsafe { (p.rs.f9)(a, b, c, q) },
        );
    }
}

#[test]
fn fuzz_f10() {
    let p = pair();
    // the entire domain is only 65536 values, so sweep it repeatedly with
    // different call ordering to also catch any accidental statefulness
    for _round in 0..4 {
        let mut h: u32 = 0;
        while h <= 0xFFFF {
            let hh = h as u16;
            same("fuzz f10", hh, unsafe { (p.c.f10)(hh) }, unsafe {
                (p.rs.f10)(hh)
            });
            h += 1;
        }
    }
}

#[test]
fn fuzz_f11_f12_f13() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0x2468);
    for i in 0..n {
        let src = match i % 4 {
            0 => [r.raw_f32(), r.raw_f32(), r.raw_f32()],
            1 => [r.nice_f32(400.0), r.nice_f32(2.0), r.nice_f32(2.0)],
            2 => [
                r.range_f32(-720.0, 720.0),
                r.range_f32(-2.0, 2.0),
                r.range_f32(-2.0, 2.0),
            ],
            _ => [r.nice_f32(2.0), r.nice_f32(2.0), r.nice_f32(2.0)],
        };
        let ctx = src.map(f32::to_bits);
        same("fuzz f11", ctx, call_f1x(p.c.f11, src), call_f1x(p.rs.f11, src));
        same("fuzz f12", ctx, call_f1x(p.c.f12, src), call_f1x(p.rs.f12, src));
        same("fuzz f13", ctx, call_f1x(p.c.f13, src), call_f1x(p.rs.f13, src));
        // and aliased
        same(
            "fuzz f11/alias",
            ctx,
            call_f1x_aliased(p.c.f11, src),
            call_f1x_aliased(p.rs.f11, src),
        );
        same(
            "fuzz f12/alias",
            ctx,
            call_f1x_aliased(p.c.f12, src),
            call_f1x_aliased(p.rs.f12, src),
        );
        same(
            "fuzz f13/alias",
            ctx,
            call_f1x_aliased(p.c.f13, src),
            call_f1x_aliased(p.rs.f13, src),
        );
    }
}

#[test]
fn fuzz_agglom() {
    let p = pair();
    let n = iters();
    let mut r = Rng::new(SEED ^ 0x1357);
    for i in 0..n {
        let raw = i % 2 == 0;
        macro_rules! f {
            () => {
                if raw {
                    r.raw_f32()
                } else {
                    r.nice_f32(4.0)
                }
            };
        }
        #[rustfmt::skip]
        let a = (
            f!(), f!(), f!(), f!(), f!(), f!(), f!(),
            r.edgy_i32(), r.edgy_i32(),
            r.edgy_u64(), r.edgy_u64(),
            r.edgy_u32(),
            r.edgy_u32(), r.edgy_u32(), r.edgy_u32(),
            f!(), f!(), f!(), f!(), f!(), f!(), f!(), f!(),
            r.next_u16(),
            f!(), f!(), f!(),
            f!(), f!(), f!(),
            f!(), f!(), f!(),
        );
        let cv = unsafe {
            (p.c.agglom)(
                a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7, a.8, a.9, a.10, a.11, a.12, a.13, a.14,
                a.15, a.16, a.17, a.18, a.19, a.20, a.21, a.22, a.23, a.24, a.25, a.26, a.27,
                a.28, a.29, a.30, a.31, a.32,
            )
        };
        let rv = unsafe {
            (p.rs.agglom)(
                a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7, a.8, a.9, a.10, a.11, a.12, a.13, a.14,
                a.15, a.16, a.17, a.18, a.19, a.20, a.21, a.22, a.23, a.24, a.25, a.26, a.27,
                a.28, a.29, a.30, a.31, a.32,
            )
        };
        if cv.to_bits() != rv.to_bits() {
            // Tuples wider than 12 elements have no `Debug`, so dump the raw
            // bit patterns of every argument by hand for reproducibility.
            let args = format!(
                "f2=[{:#010x},{:#010x},{:#010x},{:#010x},{:#010x},{:#010x},{:#010x}] \
                 f3=[{},{}] f4=[{:#018x},{:#018x}] f5={:#010x} f7=[{},{},{}] \
                 f9=[{:#010x},{:#010x},{:#010x},{:#010x},{:#010x},{:#010x},{:#010x},{:#010x}] \
                 f10={:#06x} f11=[{:#010x},{:#010x},{:#010x}] \
                 f12=[{:#010x},{:#010x},{:#010x}] f13=[{:#010x},{:#010x},{:#010x}]",
                a.0.to_bits(), a.1.to_bits(), a.2.to_bits(), a.3.to_bits(),
                a.4.to_bits(), a.5.to_bits(), a.6.to_bits(),
                a.7, a.8,
                a.9, a.10,
                a.11,
                a.12, a.13, a.14,
                a.15.to_bits(), a.16.to_bits(), a.17.to_bits(), a.18.to_bits(),
                a.19.to_bits(), a.20.to_bits(), a.21.to_bits(), a.22.to_bits(),
                a.23,
                a.24.to_bits(), a.25.to_bits(), a.26.to_bits(),
                a.27.to_bits(), a.28.to_bits(), a.29.to_bits(),
                a.30.to_bits(), a.31.to_bits(), a.32.to_bits(),
            );
            panic!(
                "DIVERGENCE in fuzz agglom\n  iter  : {i}\n  args  : {args}\n  C     : {:#018x}\n  Rust  : {:#018x}",
                cv.to_bits(),
                rv.to_bits()
            );
        }
    }
}
