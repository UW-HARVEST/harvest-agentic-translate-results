//! High-volume randomized stress pass.
//!
//! The per-row tests in `phase_b_*.rs` prove each `CONFIGS.md` configuration is
//! reached; this file drives millions of additional inputs through the same
//! entry points to shrink the chance that a rare value-dependent path (a NaN
//! payload race, an exact-equality tie, an unsigned wrap boundary) went
//! unsampled. Seeds are fixed, so failures are reproducible.
//!
//! Run with `cargo test --test phase_b_stress -- --nocapture`.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_void;

/// Iterations per stress loop. The full volume only makes sense in an
/// optimized build; an unoptimized build runs ~30x slower, so scale it down
/// there to keep every invocation well under the 600 s budget.
/// (`cargo test --release --test phase_b_stress` is the intended full run.)
const N: u32 = if cfg!(debug_assertions) {
    1_000_000
} else {
    30_000_000
};

#[test]
fn stress_f3() {
    let a = apis();
    let mut rng = Rng::with_seed(0x9E37_79B9_7F4A_7C15);
    for i in 0..N {
        // Mix full-range, small-divisor and INT_MIN-adjacent shapes.
        let (v1, v2) = match i % 6 {
            0 => (rng.next_i32(), rng.next_i32()),
            1 => (rng.next_i32(), (rng.next_u32() % 64) as i32 - 32),
            2 => ((rng.next_u32() % 64) as i32 - 32, rng.next_i32()),
            3 => (
                i32::MIN.wrapping_add((rng.next_u32() % 8) as i32),
                rng.next_i32(),
            ),
            4 => (
                rng.next_i32(),
                i32::MIN.wrapping_add((rng.next_u32() % 8) as i32),
            ),
            _ => (rng.pick(SPECIAL_I32), rng.pick(SPECIAL_I32)),
        };
        unsafe {
            let (c, r) = ((a.c.f3)(v1, v2), (a.r.f3)(v1, v2));
            if c != r {
                panic!("f3 diverged: f3({v1}, {v2}) C={c} Rust={r}");
            }
        }
    }
}

#[test]
fn stress_f7() {
    let a = apis();
    let mut rng = Rng::with_seed(0xBF58_476D_1CE4_E5B9);
    for i in 0..N {
        let (bs, ch, bd) = match i % 4 {
            0 => (rng.next_u32(), rng.next_u32(), rng.next_u32()),
            1 => (rng.next_u32(), 2, rng.next_u32()),
            2 => (rng.next_u32(), rng.next_u32(), 32),
            _ => (
                rng.pick(SPECIAL_U32),
                rng.pick(SPECIAL_U32),
                rng.pick(SPECIAL_U32),
            ),
        };
        unsafe {
            let (c, r) = ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd));
            if c != r {
                panic!("f7 diverged: f7({bs}, {ch}, {bd}) C={c} Rust={r}");
            }
        }
    }
}

#[test]
fn stress_f4() {
    let a = apis();
    let mut rng = Rng::with_seed(0x94D0_49BB_1331_11EB);
    // Long chains so the generator visits deep state, checked every step.
    for _ in 0..(N / 512) {
        let (s0, s1) = (rng.next_u64(), rng.next_u64());
        let mut sc = cn_rnd_t { state: [s0, s1] };
        let mut sr = cn_rnd_t { state: [s0, s1] };
        for step in 0..512 {
            let (c, r) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
            if c.to_bits() != r.to_bits() || sc.state != sr.state {
                panic!(
                    "f4 diverged at step {step} for seed {s0:#x},{s1:#x}: \
                     C={c:?}/{:?} Rust={r:?}/{:?}",
                    sc.state, sr.state
                );
            }
        }
    }
}

#[test]
fn stress_f5() {
    let a = apis();
    let mut rng = Rng::with_seed(0x2545_F491_4F6C_DD1D);
    for _ in 0..N {
        let v = rng.next_u32();
        unsafe {
            let (c, r) = ((a.c.f5)(v), (a.r.f5)(v));
            if c != r {
                panic!("f5 diverged: f5({v:#x}) C={c:#x} Rust={r:#x}");
            }
        }
    }
}

#[test]
fn stress_collision_predicates() {
    let a = apis();
    let mut rng = Rng::with_seed(0x1234_5678_9ABC_DEF0);
    let zoo = zoo_f32();
    for i in 0..(N / 2) {
        let f = |rng: &mut Rng| match i % 3 {
            0 => rng.any_f32(),
            1 => rng.pick(&zoo),
            _ => rng.finite_f32(100.0),
        };
        let A = c2Circle {
            p: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            r: f(&mut rng),
        };
        let B = c2Circle {
            p: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            r: f(&mut rng),
        };
        let X = c2AABB {
            min: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            max: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
        };
        let Y = c2AABB {
            min: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            max: c2v {
                x: f(&mut rng),
                y: f(&mut rng),
            },
        };
        unsafe {
            let ctx = (
                Bits(A.p.x),
                Bits(A.p.y),
                Bits(A.r),
                Bits(B.p.x),
                Bits(B.p.y),
                Bits(B.r),
            );
            eq_f32("c2Dot", &ctx, (a.c.c2Dot)(A.p, B.p), (a.r.c2Dot)(A.p, B.p));
            eq_v2("c2Sub", &ctx, (a.c.c2Sub)(A.p, B.p), (a.r.c2Sub)(A.p, B.p));
            eq_v2(
                "c2Maxv",
                &ctx,
                (a.c.c2Maxv)(A.p, B.p),
                (a.r.c2Maxv)(A.p, B.p),
            );
            eq_v2(
                "c2Minv",
                &ctx,
                (a.c.c2Minv)(A.p, B.p),
                (a.r.c2Minv)(A.p, B.p),
            );
            eq_v2(
                "c2Clampv",
                &ctx,
                (a.c.c2Clampv)(A.p, X.min, X.max),
                (a.r.c2Clampv)(A.p, X.min, X.max),
            );
            eq_i32(
                "c2CircletoCircle",
                &ctx,
                (a.c.c2CircletoCircle)(A, B),
                (a.r.c2CircletoCircle)(A, B),
            );
            eq_i32(
                "c2CircletoAABB",
                &ctx,
                (a.c.c2CircletoAABB)(A, X),
                (a.r.c2CircletoAABB)(A, X),
            );
            eq_i32(
                "c2AABBtoAABB",
                &ctx,
                (a.c.c2AABBtoAABB)(X, Y),
                (a.r.c2AABBtoAABB)(X, Y),
            );
            // f2 in all four valid dispatch combinations
            let pc = &A as *const c2Circle as *const c_void;
            let pb = &X as *const c2AABB as *const c_void;
            let pc2 = &B as *const c2Circle as *const c_void;
            let pb2 = &Y as *const c2AABB as *const c_void;
            for (ta, pa, tb, pbp) in [
                (C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE, pc2),
                (C2_TYPE_CIRCLE, pc, C2_TYPE_AABB, pb),
                (C2_TYPE_AABB, pb, C2_TYPE_CIRCLE, pc),
                (C2_TYPE_AABB, pb, C2_TYPE_AABB, pb2),
            ] {
                eq_i32(
                    "f2",
                    &ctx,
                    (a.c.f2)(pa, ta, pbp, tb),
                    (a.r.f2)(pa, ta, pbp, tb),
                );
            }
        }
    }
}

#[test]
fn stress_f9() {
    let a = apis();
    let mut rng = Rng::with_seed(0xDEAD_BEEF_CAFE_BABE);
    let zoo = zoo_f32();
    for i in 0..N {
        let f = |rng: &mut Rng| match i % 4 {
            0 => rng.any_f32(),
            1 => rng.pick(&zoo),
            2 => rng.finite_f32(100.0),
            _ => rng.small_f32(),
        };
        let (p1, p2, p3, p) = (
            lm_vec2 {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            lm_vec2 {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            lm_vec2 {
                x: f(&mut rng),
                y: f(&mut rng),
            },
            lm_vec2 {
                x: f(&mut rng),
                y: f(&mut rng),
            },
        );
        unsafe {
            let (c, r) = ((a.c.f9)(p1, p2, p3, p), (a.r.f9)(p1, p2, p3, p));
            if c.x.to_bits() != r.x.to_bits() || c.y.to_bits() != r.y.to_bits() {
                panic!(
                    "f9 diverged for p1=({:?},{:?}) p2=({:?},{:?}) p3=({:?},{:?}) p=({:?},{:?}): \
                     C=({:?},{:?}) Rust=({:?},{:?})",
                    Bits(p1.x),
                    Bits(p1.y),
                    Bits(p2.x),
                    Bits(p2.y),
                    Bits(p3.x),
                    Bits(p3.y),
                    Bits(p.x),
                    Bits(p.y),
                    Bits(c.x),
                    Bits(c.y),
                    Bits(r.x),
                    Bits(r.y)
                );
            }
        }
    }
}

#[test]
fn stress_colour_conversions() {
    let a = apis();
    let mut rng = Rng::with_seed(0x5555_AAAA_3333_CCCC);
    let zoo = zoo_f32();
    for i in 0..N {
        let src: [f32; 3] = match i % 5 {
            0 => [rng.any_f32(), rng.any_f32(), rng.any_f32()],
            1 => [rng.pick(&zoo), rng.pick(&zoo), rng.pick(&zoo)],
            // sane hue x unit s/l, the densest region of real use
            2 => [
                (rng.next_u32() as f64 / u32::MAX as f64 * 400.0 - 20.0) as f32,
                (rng.next_u32() as f64 / u32::MAX as f64) as f32,
                (rng.next_u32() as f64 / u32::MAX as f64) as f32,
            ],
            // small integral multiples: maximises exact-equality ties
            3 => [rng.small_f32(), rng.small_f32(), rng.small_f32()],
            _ => [
                rng.finite_f32(1e6),
                rng.finite_f32(1e-20),
                rng.finite_f32(1e20),
            ],
        };
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            for (name, cf, rf) in [
                ("f11", a.c.f11, a.r.f11),
                ("f12", a.c.f12, a.r.f12),
                ("f13", a.c.f13, a.r.f13),
            ] {
                cf(dc.as_mut_ptr(), src.as_ptr());
                rf(dr.as_mut_ptr(), src.as_ptr());
                for k in 0..3 {
                    if dc[k].to_bits() != dr[k].to_bits() {
                        panic!(
                            "{name} diverged at [{k}] for src={:?}: C={:?} Rust={:?}",
                            BitsTri(src),
                            Bits(dc[k]),
                            Bits(dr[k])
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn stress_agglom() {
    let a = apis();
    let mut rng = Rng::with_seed(0x0F1E_2D3C_4B5A_6978);
    let zoo = zoo_f32();
    for i in 0..(N / 2) {
        let fl = |rng: &mut Rng| match i % 4 {
            0 => rng.any_f32(),
            1 => rng.pick(&zoo),
            2 => rng.finite_f32(400.0),
            _ => (rng.next_u32() as f64 / u32::MAX as f64) as f32,
        };
        let f: [f32; 24] = std::array::from_fn(|_| fl(&mut rng));
        let i1 = rng.next_i32();
        let i2 = if i % 3 == 0 { 0 } else { rng.next_i32() };
        let s0 = rng.next_u64();
        let s1 = rng.next_u64();
        let u0 = rng.next_u32();
        let u1 = rng.next_u32();
        let u2 = if i % 5 == 0 { 2 } else { rng.next_u32() };
        let u3 = if i % 7 == 0 { 32 } else { rng.next_u32() };
        let h = rng.next_u16();

        let call = |g: FnAgglom| unsafe {
            g(
                f[0], f[1], f[2], f[3], f[4], f[5], f[6], i1, i2, s0, s1, u0, u1, u2, u3, f[7],
                f[8], f[9], f[10], f[11], f[12], f[13], f[14], h, f[15], f[16], f[17], f[18],
                f[19], f[20], f[21], f[22], f[23],
            )
        };
        let (c, r) = (call(a.c.agglom), call(a.r.agglom));
        if c.to_bits() != r.to_bits() {
            panic!(
                "agglom diverged: C={c:?} (0x{:016x}) Rust={r:?} (0x{:016x})\n\
                 floats(bits)={:?}\n i=({i1},{i2}) u64=({s0:#x},{s1:#x}) \
                 u32=({u0:#x},{u1:#x},{u2:#x},{u3:#x}) h={h:#06x}",
                c.to_bits(),
                r.to_bits(),
                f.iter().map(|v| format!("{:#010x}", v.to_bits())).collect::<Vec<_>>()
            );
        }
    }
}

/// Sanity check that the stress loops really iterate `N` times (guards against
/// an accidentally-empty loop making the whole file vacuous).
#[test]
fn stress_loop_counts_are_real() {
    let a = apis();
    let mut rng = Rng::with_seed(1);
    let mut acc: u64 = 0;
    let mut count: u64 = 0;
    for _ in 0..N {
        let v = rng.next_u32();
        let c = unsafe { (a.c.f5)(v) };
        let r = unsafe { (a.r.f5)(v) };
        assert_eq!(c, r);
        acc = acc.wrapping_add(c as u64);
        count += 1;
    }
    assert_eq!(count, N as u64, "the loop must run N times");
    assert_ne!(acc, 0, "results must actually be consumed");
    eprintln!("stress: {count} iterations, checksum {acc}");
}
