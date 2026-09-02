//! High-volume differential fuzz sweep across ALL 12 exported entry points.
//!
//! This is the broad safety net behind the per-row Phase B/C tests: every call
//! uses fully random 32-bit patterns (so NaNs, subnormals, infinities and huge
//! exponents all occur naturally) and every `int` / `f32` result is compared
//! exactly. Deterministic: fixed seed, no external RNG crate.
//!
//! Volume is tunable via `FUZZ_ITERS` (default 300_000 per entry point) so the
//! suite stays well inside the 600 s budget.

#![allow(non_snake_case)]

mod common;
use common::*;

fn iters() -> u64 {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300_000)
}

fn bytes_of<T: Copy>(v: &T) -> Vec<u8> {
    let n = std::mem::size_of::<T>();
    let mut out = vec![0u8; n];
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, out.as_mut_ptr(), n);
    }
    out
}

#[test]
fn fuzz_all_leaf_entry_points() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(0xF122_0001);
    let n = iters();
    for i in 0..n {
        let a = rng.vec_raw();
        let b = rng.vec_raw();
        let d = rng.vec_raw();
        let s = rng.raw_f32();
        unsafe {
            let (x, y) = (a.x, a.y);
            diff_assert!(
                v_eq((c.c2V)(x, y), (r.c2V)(x, y)),
                "fuzz #{i} c2V({}, {})",
                show(x),
                show(y)
            );
            diff_assert!(
                v_eq((c.c2Sub)(a, b), (r.c2Sub)(a, b)),
                "fuzz #{i} c2Sub({}, {})",
                show_v(a),
                show_v(b)
            );
            diff_assert!(
                f32_eq_bits((c.c2Dot)(a, b), (r.c2Dot)(a, b)),
                "fuzz #{i} c2Dot({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show((c.c2Dot)(a, b)),
                show((r.c2Dot)(a, b))
            );
            diff_assert!(
                v_eq((c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s)),
                "fuzz #{i} c2Mulvs({}, {})",
                show_v(a),
                show(s)
            );
            diff_assert!(
                v_eq((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)),
                "fuzz #{i} c2Maxv({}, {})",
                show_v(a),
                show_v(b)
            );
            diff_assert!(
                v_eq((c.c2Minv)(a, b), (r.c2Minv)(a, b)),
                "fuzz #{i} c2Minv({}, {})",
                show_v(a),
                show_v(b)
            );
            diff_assert!(
                v_eq((c.c2Clampv)(a, b, d), (r.c2Clampv)(a, b, d)),
                "fuzz #{i} c2Clampv({}, {}, {})",
                show_v(a),
                show_v(b),
                show_v(d)
            );
        }
    }
}

#[test]
fn fuzz_all_shape_entry_points() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(0xF122_0002);
    let n = iters();
    let (mut hit_cc, mut hit_ca, mut hit_cp) = (0u64, 0u64, 0u64);
    for i in 0..n {
        // Alternate between "geometrically plausible" and "raw bit soup" inputs
        // so the collision predicates actually fire as well as being stressed.
        let plausible = i % 2 == 0;
        let A = if plausible {
            rng.circle()
        } else {
            c2Circle {
                p: rng.vec_raw(),
                r: rng.raw_f32(),
            }
        };
        let B = if plausible {
            rng.circle()
        } else {
            c2Circle {
                p: rng.vec_raw(),
                r: rng.raw_f32(),
            }
        };
        let bb = if plausible {
            rng.aabb_proper()
        } else {
            c2AABB {
                min: rng.vec_raw(),
                max: rng.vec_raw(),
            }
        };
        let cap = if plausible {
            rng.capsule()
        } else {
            c2Capsule {
                a: rng.vec_raw(),
                b: rng.vec_raw(),
                r: rng.raw_f32(),
            }
        };
        unsafe {
            let (cv, rv) = ((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
            diff_assert!(cv == rv, "fuzz #{i} c2CircletoCircle {A:?} {B:?}: {cv} vs {rv}");
            hit_cc += cv as u64;
            let (cv, rv) = ((c.c2CircletoAABB)(A, bb), (r.c2CircletoAABB)(A, bb));
            diff_assert!(cv == rv, "fuzz #{i} c2CircletoAABB {A:?} {bb:?}: {cv} vs {rv}");
            hit_ca += cv as u64;
            let (cv, rv) = (
                (c.c2CircletoCapsule)(A, cap),
                (r.c2CircletoCapsule)(A, cap),
            );
            diff_assert!(cv == rv, "fuzz #{i} c2CircletoCapsule {A:?} {cap:?}: {cv} vs {rv}");
            hit_cp += cv as u64;
        }
    }
    println!("fuzz shape hits: circle={hit_cc} aabb={hit_ca} capsule={hit_cp} of {n}");
    assert!(hit_cc > 0 && hit_ca > 0 && hit_cp > 0, "fuzz never hit a collision");
    assert!(hit_cc < n && hit_ca < n && hit_cp < n, "fuzz always hit a collision");
}

#[test]
fn fuzz_dispatcher_and_wrapper() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(0xF122_0003);
    let n = iters();
    for i in 0..n {
        let A = c2Circle {
            p: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        let ab = bytes_of(&A);
        // Random operand buffer big enough for the largest shape, plus a random
        // typeB drawn from a range that straddles the valid variants.
        let cap = c2Capsule {
            a: rng.vec_raw(),
            b: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        let bb = bytes_of(&cap);
        let ty = (rng.next_u32() % 9) as i32 - 3; // -3 ..= 5
        unsafe {
            let cv = (c.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            let rv = (r.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            diff_assert!(
                cv == rv,
                "fuzz #{i} c2Collided(ty={ty}) A={A:?} B={cap:?}: C={cv} RS={rv}"
            );
            if !(0..=2).contains(&ty) {
                diff_assert!(cv == 0, "fuzz #{i} invalid ty={ty} gave {cv}, expected 0");
            }

            let (x, y, rad) = (rng.raw_f32(), rng.raw_f32(), rng.raw_f32());
            let cw = (c.circle_collide)(x, y, rad);
            let rw = (r.circle_collide)(x, y, rad);
            diff_assert!(
                cw == rw,
                "fuzz #{i} circle_collide({}, {}, {}): C={cw} RS={rw}",
                show(x),
                show(y),
                show(rad)
            );
            diff_assert!(
                (0..8).contains(&cw),
                "fuzz #{i} C circle_collide returned out-of-range {cw}"
            );
        }
    }
}
