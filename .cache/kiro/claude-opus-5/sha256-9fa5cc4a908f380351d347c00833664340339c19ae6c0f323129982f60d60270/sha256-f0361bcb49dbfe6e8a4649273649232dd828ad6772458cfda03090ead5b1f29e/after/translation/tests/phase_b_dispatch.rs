//! Phase B — valid-path differential tests for the DISPATCHER, the top-level
//! entry point, and the composed pipeline.
//!
//! Covers `CONFIGS.md` rows 25-34.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 20_000;

/// Rows 25-27 — `c2Collided` with each valid `C2_TYPE`, cross-checked two ways:
/// C vs Rust, and each library's dispatcher against *its own* direct predicate
/// (so a mis-wired switch arm cannot hide behind a matching pair).
#[test]
fn row25_27_collided_valid_types() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let a = rng.circle();

        // C2_TYPE_CIRCLE
        let b = rng.circle();
        let (c, rs) = unsafe {
            (
                (p.c.c2Collided)(
                    &a as *const _ as *const c_void,
                    &b as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                ),
                (p.rs.c2Collided)(
                    &a as *const _ as *const c_void,
                    &b as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                ),
            )
        };
        assert_int_eq("c2Collided CIRCLE", c, rs);
        assert_int_eq(
            "c2Collided CIRCLE vs direct (C)",
            c,
            unsafe { (p.c.c2CircletoCircle)(a, b) },
        );
        assert_int_eq(
            "c2Collided CIRCLE vs direct (Rust)",
            rs,
            unsafe { (p.rs.c2CircletoCircle)(a, b) },
        );

        // C2_TYPE_AABB
        let bb = rng.aabb_raw();
        let (c, rs) = unsafe {
            (
                (p.c.c2Collided)(
                    &a as *const _ as *const c_void,
                    &bb as *const _ as *const c_void,
                    C2_TYPE_AABB,
                ),
                (p.rs.c2Collided)(
                    &a as *const _ as *const c_void,
                    &bb as *const _ as *const c_void,
                    C2_TYPE_AABB,
                ),
            )
        };
        assert_int_eq("c2Collided AABB", c, rs);
        assert_int_eq("c2Collided AABB vs direct (C)", c, unsafe {
            (p.c.c2CircletoAABB)(a, bb)
        });
        assert_int_eq("c2Collided AABB vs direct (Rust)", rs, unsafe {
            (p.rs.c2CircletoAABB)(a, bb)
        });

        // C2_TYPE_CAPSULE
        let cap = rng.capsule();
        let (c, rs) = unsafe {
            (
                (p.c.c2Collided)(
                    &a as *const _ as *const c_void,
                    &cap as *const _ as *const c_void,
                    C2_TYPE_CAPSULE,
                ),
                (p.rs.c2Collided)(
                    &a as *const _ as *const c_void,
                    &cap as *const _ as *const c_void,
                    C2_TYPE_CAPSULE,
                ),
            )
        };
        assert_int_eq("c2Collided CAPSULE", c, rs);
        assert_int_eq("c2Collided CAPSULE vs direct (C)", c, unsafe {
            (p.c.c2CircletoCapsule)(a, cap)
        });
        assert_int_eq("c2Collided CAPSULE vs direct (Rust)", rs, unsafe {
            (p.rs.c2CircletoCapsule)(a, cap)
        });
    }
}

/// Row 28 — aliasing: the SAME buffer passed as both `A` and `B`. For the AABB
/// and capsule arms this means a `c2Circle` is reinterpreted as a larger type,
/// so `B` reads bytes past the circle — which is exactly what the C does when a
/// caller aliases, and is reproduced here with a buffer large enough for the
/// widest type so no read is out of bounds.
#[test]
fn row28_collided_aliasing() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        // 32-byte buffer: fits c2Circle (12), c2AABB (16) and c2Capsule (20).
        let mut buf = [0u32; 8];
        for w in buf.iter_mut() {
            *w = rng.next_u32();
        }
        // Keep the leading floats in a sane range so real branches are hit,
        // while the tail stays arbitrary.
        let head = [rng.coord(), rng.coord(), rng.radius(), rng.coord(), rng.coord()];
        for (i, f) in head.iter().enumerate() {
            buf[i] = f.to_bits();
        }
        let ptr = buf.as_ptr() as *const c_void;

        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let (c, rs) = unsafe { ((p.c.c2Collided)(ptr, ptr, ty), (p.rs.c2Collided)(ptr, ptr, ty)) };
            assert_int_eq(&format!("c2Collided aliased ty={ty}"), c, rs);
        }
    }
}

/// Row 29 — `B` points into an over-sized buffer whose tail holds garbage. The
/// C only reads `sizeof(type)` bytes, so the tail must not affect the result:
/// asserted by running the same logical shape with two different tails.
#[test]
fn row29_collided_oversized_buffer() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let a = rng.circle();
        let a_ptr = &a as *const _ as *const c_void;

        for (ty, used_words) in [
            (C2_TYPE_CIRCLE, 3usize),
            (C2_TYPE_AABB, 4),
            (C2_TYPE_CAPSULE, 5),
        ] {
            let mut buf1 = [0u32; 8];
            for i in 0..used_words {
                buf1[i] = rng.coord().to_bits();
            }
            let mut buf2 = buf1;
            // Differing garbage tails.
            for i in used_words..8 {
                buf1[i] = rng.next_u32();
                buf2[i] = rng.next_u32();
            }

            let r1c = unsafe { (p.c.c2Collided)(a_ptr, buf1.as_ptr() as *const c_void, ty) };
            let r1r = unsafe { (p.rs.c2Collided)(a_ptr, buf1.as_ptr() as *const c_void, ty) };
            let r2c = unsafe { (p.c.c2Collided)(a_ptr, buf2.as_ptr() as *const c_void, ty) };
            let r2r = unsafe { (p.rs.c2Collided)(a_ptr, buf2.as_ptr() as *const c_void, ty) };

            assert_int_eq(&format!("oversized buf1 ty={ty}"), r1c, r1r);
            assert_int_eq(&format!("oversized buf2 ty={ty}"), r2c, r2r);
            assert_eq!(r1c, r2c, "C read past the used bytes for ty={ty}");
            assert_eq!(r1r, r2r, "Rust read past the used bytes for ty={ty}");
        }
    }
}

/// Rows 30-32 — `circle_collide`: targeted inputs for each bit, multi-bit
/// overlaps, a dense grid sweep, and a large randomized sweep. Asserts that all
/// 8 packed results are observed, so the row really covers the cross-product.
#[test]
fn row30_32_circle_collide_sweep() {
    let p = load();
    let mut rng = Rng::default_seeded();
    let mut seen = [0usize; 8];

    let check = |x: f32, y: f32, r: f32, what: &str, seen: &mut [usize; 8]| {
        let (c, rs) = unsafe { ((p.c.circle_collide)(x, y, r), (p.rs.circle_collide)(x, y, r)) };
        assert_int_eq(&format!("circle_collide({x:?},{y:?},{r:?}) [{what}]"), c, rs);
        if (0..8).contains(&c) {
            seen[c as usize] += 1;
        }
        c
    };

    // Row 31 — each bit alone.
    // bit 0: hard-coded circle at (-70, 0) r 20.
    check(-70.0, 0.0, 1.0, "bit0 centre", &mut seen);
    check(-55.0, 0.0, 1.0, "bit0 rim", &mut seen);
    // bit 1: hard-coded AABB [-40,-15]^2.
    check(-27.5, -27.5, 1.0, "bit1 centre", &mut seen);
    check(-45.0, -27.5, 6.0, "bit1 left edge", &mut seen);
    // bit 2: hard-coded capsule (-40,40)-(-20,100) r 10.
    check(-30.0, 70.0, 1.0, "bit2 middle", &mut seen);
    check(-40.0, 40.0, 1.0, "bit2 endpoint a", &mut seen);
    check(-20.0, 100.0, 1.0, "bit2 endpoint b", &mut seen);
    // Row 32 — multi-bit overlaps via a huge radius, and the empty result.
    check(0.0, 0.0, 1000.0, "all bits", &mut seen);
    check(-55.0, -30.0, 20.0, "bits 0+1", &mut seen);
    check(-40.0, 10.0, 40.0, "bits 1+2", &mut seen);
    check(1000.0, 1000.0, 1.0, "no bits", &mut seen);

    // Row 32 — dense grid sweep over the region containing all three shapes.
    let mut x = -120.0f32;
    while x <= 40.0 {
        let mut y = -120.0f32;
        while y <= 140.0 {
            for &r in &[0.0f32, 1.0, 5.0, 20.0, 100.0] {
                check(x, y, r, "grid", &mut seen);
            }
            y += 2.5;
        }
        x += 2.5;
    }

    // Row 30 — randomized sweep.
    for _ in 0..N {
        let x = rng.range(-160.0, 60.0);
        let y = rng.range(-160.0, 180.0);
        let r = rng.range(0.0, 80.0);
        check(x, y, r, "random", &mut seen);
    }

    let missing: Vec<usize> = (0..8).filter(|&i| seen[i] == 0).collect();
    assert!(
        missing.is_empty(),
        "packed results never observed: {missing:?} (counts {seen:?})"
    );
}

/// Row 33 — `circle_collide` with extreme scalars on every axis.
#[test]
fn row33_circle_collide_extremes() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            for &r in EDGE_F32 {
                let (c, rs) =
                    unsafe { ((p.c.circle_collide)(x, y, r), (p.rs.circle_collide)(x, y, r)) };
                assert_int_eq(&format!("circle_collide extreme({x:?},{y:?},{r:?})"), c, rs);
            }
        }
    }

    for _ in 0..N {
        let (x, y, r) = (rng.any_bits_f32(), rng.any_bits_f32(), rng.any_bits_f32());
        let (c, rs) = unsafe { ((p.c.circle_collide)(x, y, r), (p.rs.circle_collide)(x, y, r)) };
        assert_int_eq("circle_collide random-bits", c, rs);
    }
}

/// Row 34 — the composed pipeline, built out of the LOW-LEVEL exports and
/// compared against the high-level predicate of the *other* library. This
/// catches a divergence that only appears once the primitives are chained
/// (`c2Clampv` -> `c2Sub` -> `c2Dot`), which per-function tests cannot see.
#[test]
fn row34_composed_pipeline_cross_library() {
    let p = load();
    let mut rng = Rng::default_seeded();

    // Re-implement c2CircletoAABB using one library's primitives.
    let compose_aabb = |api: &Api, a: C2Circle, b: C2Aabb| -> i32 {
        unsafe {
            let l = (api.c2Clampv)(a.p, b.min, b.max);
            let ab = (api.c2Sub)(a.p, l);
            let d2 = (api.c2Dot)(ab, ab);
            let r2 = (api.c2Dot)((api.c2V)(a.r, 0.0), (api.c2V)(a.r, 0.0));
            // NB: c2Dot((r,0),(r,0)) == 0*0 + r*r == r*r, matching `A.r * A.r`.
            (d2 < r2) as i32
        }
    };

    // Re-implement c2CircletoCapsule's side arm using primitives.
    let compose_capsule_side = |api: &Api, a: C2Circle, b: C2Capsule| -> C2v {
        unsafe {
            let n = (api.c2Sub)(b.b, b.a);
            let ap = (api.c2Sub)(a.p, b.a);
            let da = (api.c2Dot)(ap, n);
            let nn = (api.c2Dot)(n, n);
            (api.c2Sub)(ap, (api.c2Mulvs)(n, da / nn))
        }
    };

    for _ in 0..N {
        let a = rng.circle();
        let bb = rng.aabb_raw();

        let cc = compose_aabb(&p.c, a, bb);
        let cr = compose_aabb(&p.rs, a, bb);
        assert_int_eq("composed aabb C vs Rust", cc, cr);
        // Cross: C-composed vs Rust's built-in, and vice versa.
        assert_int_eq("C-composed vs Rust builtin", cc, unsafe {
            (p.rs.c2CircletoAABB)(a, bb)
        });
        assert_int_eq("Rust-composed vs C builtin", cr, unsafe {
            (p.c.c2CircletoAABB)(a, bb)
        });

        let cap = rng.capsule();
        let ec = compose_capsule_side(&p.c, a, cap);
        let er = compose_capsule_side(&p.rs, a, cap);
        assert_v_bits_eq("composed capsule projection", ec, er);
        // Cross-library composition: mix C and Rust primitives in one chain.
        let mixed = unsafe {
            let n = (p.c.c2Sub)(cap.b, cap.a);
            let ap = (p.rs.c2Sub)(a.p, cap.a);
            let da = (p.c.c2Dot)(ap, n);
            let nn = (p.rs.c2Dot)(n, n);
            (p.c.c2Sub)(ap, (p.rs.c2Mulvs)(n, da / nn))
        };
        assert_v_bits_eq("mixed C/Rust chain", ec, mixed);
    }
}
