//! Phase B — valid-path differential tests for the DISPATCHER and the one-shot
//! wrapper, plus the composed-pipeline cross-check.
//!
//! Covers `CONFIGS.md` rows 32..40: `c2Collided` under each valid `C2_TYPE`
//! (including the 20-byte MEMORY-class `c2Capsule` by-value pass, raw-byte
//! operands and unaligned operand addresses), `circle_collide` over its full
//! 3-bit result space, and a pipeline cross-check that recomputes
//! `circle_collide` from the low-level exports.

#![allow(non_snake_case)]

mod common;
use common::*;

fn specials() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.extend(SPECIAL_BITS.iter().map(|&b| f32::from_bits(b)));
    v
}

/// Raw little-endian bytes of a `#[repr(C)]` value.
fn bytes_of<T: Copy>(v: &T) -> Vec<u8> {
    let n = std::mem::size_of::<T>();
    let mut out = vec![0u8; n];
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, out.as_mut_ptr(), n);
    }
    out
}

/// Call `c2Collided` on both `.so`s with the given raw operand buffers.
fn cmp_collided(a: &[u8], b: &[u8], ty: i32, ctx: &str) -> i32 {
    let (c, r) = libs();
    unsafe {
        let cv = (c.c2Collided)(a.as_ptr(), b.as_ptr(), ty);
        let rv = (r.c2Collided)(a.as_ptr(), b.as_ptr(), ty);
        diff_assert!(
            cv == rv,
            "{ctx} c2Collided(A={:02x?}, B={:02x?}, typeB={ty}): C={cv} RS={rv}",
            a,
            b
        );
        cv
    }
}

// ===========================================================================
// Rows 32..34 — c2Collided under each valid C2_TYPE
// ===========================================================================

#[test]
fn row32_collided_type_circle() {
    let mut rng = Rng::seeded(32);
    let mut hits = 0usize;
    for i in 0..4096 {
        let A = rng.circle();
        let B = rng.circle();
        hits += cmp_collided(
            &bytes_of(&A),
            &bytes_of(&B),
            C2_TYPE_CIRCLE,
            &format!("row32 #{i}"),
        ) as usize;
    }
    assert!(hits > 0 && hits < 4096, "row32 coverage {hits}/4096");
}

#[test]
fn row33_collided_type_aabb() {
    let mut rng = Rng::seeded(33);
    let mut hits = 0usize;
    for i in 0..4096 {
        let A = rng.circle();
        // Mix proper, inverted and degenerate boxes.
        let bb = match i % 3 {
            0 => rng.aabb_proper(),
            1 => {
                let p = rng.aabb_proper();
                c2AABB {
                    min: p.max,
                    max: p.min,
                }
            }
            _ => {
                let p = rng.vec_coord();
                c2AABB { min: p, max: p }
            }
        };
        hits += cmp_collided(
            &bytes_of(&A),
            &bytes_of(&bb),
            C2_TYPE_AABB,
            &format!("row33 #{i}"),
        ) as usize;
    }
    assert!(hits > 0, "row33 never collided");
}

#[test]
fn row34_collided_type_capsule() {
    // Also exercises the 20-byte c2Capsule => SysV MEMORY class (stack) pass
    // performed inside c2Collided.
    let mut rng = Rng::seeded(34);
    let mut hits = 0usize;
    for i in 0..4096 {
        let A = rng.circle();
        let cap = if i % 4 == 0 {
            let p = rng.vec_coord();
            c2Capsule {
                a: p,
                b: p,
                r: rng.radius(),
            } // degenerate
        } else {
            rng.capsule()
        };
        hits += cmp_collided(
            &bytes_of(&A),
            &bytes_of(&cap),
            C2_TYPE_CAPSULE,
            &format!("row34 #{i}"),
        ) as usize;
    }
    assert!(hits > 0, "row34 never collided");
}

// ===========================================================================
// Row 35 — arbitrary raw operand bytes under each valid type
// ===========================================================================

#[test]
fn row35_collided_raw_bytes_all_types() {
    let mut rng = Rng::seeded(35);
    for &(ty, bsize) in &[
        (C2_TYPE_CIRCLE, 12usize),
        (C2_TYPE_AABB, 16),
        (C2_TYPE_CAPSULE, 20),
    ] {
        for i in 0..2048 {
            // 8-byte-aligned buffers holding fully random bit patterns, so the
            // floats include NaNs, subnormals, infinities and huge exponents.
            let mut a: Vec<u64> = (0..2).map(|_| rng.next_u64()).collect();
            let mut b: Vec<u64> = (0..4).map(|_| rng.next_u64()).collect();
            let abytes = unsafe {
                std::slice::from_raw_parts(a.as_mut_ptr() as *const u8, 12)
            };
            let bbytes = unsafe {
                std::slice::from_raw_parts(b.as_mut_ptr() as *const u8, bsize)
            };
            cmp_collided(abytes, bbytes, ty, &format!("row35 ty={ty} #{i}"));
        }
    }
}

// ===========================================================================
// Row 36 — unaligned operand pointers
// ===========================================================================

#[test]
fn row36_collided_unaligned_pointers() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(36);
    for &(ty, bsize) in &[
        (C2_TYPE_CIRCLE, 12usize),
        (C2_TYPE_AABB, 16),
        (C2_TYPE_CAPSULE, 20),
    ] {
        for i in 0..1024 {
            // 32-byte scratch buffers; place the structs at offsets 1..=7 so the
            // struct copy inside c2Collided is unaligned.
            let mut abuf = [0u8; 40];
            let mut bbuf = [0u8; 40];
            for k in 0..40 {
                abuf[k] = (rng.next_u32() & 0xFF) as u8;
                bbuf[k] = (rng.next_u32() & 0xFF) as u8;
            }
            let off = 1 + (i % 7);
            let ap = abuf[off..off + 12].as_ptr();
            let bp = bbuf[off..off + bsize].as_ptr();
            let (cv, rv) = unsafe {
                ((c.c2Collided)(ap, bp, ty), (r.c2Collided)(ap, bp, ty))
            };
            diff_assert!(
                cv == rv,
                "row36 ty={ty} #{i} off={off}: C={cv} RS={rv} A={:02x?} B={:02x?}",
                &abuf[off..off + 12],
                &bbuf[off..off + bsize]
            );
            // Cross-check: the same bytes copied to an aligned location must
            // give the same answer in both libs.
            let a_al = abuf[off..off + 12].to_vec();
            let b_al = bbuf[off..off + bsize].to_vec();
            let ca = cmp_collided(&a_al, &b_al, ty, &format!("row36-aligned ty={ty} #{i}"));
            diff_assert!(
                ca == cv,
                "row36 alignment changed the answer (ty={ty} #{i}): unaligned={cv} aligned={ca}"
            );
        }
    }
}

// ===========================================================================
// Rows 37..39 — circle_collide
// ===========================================================================

#[test]
fn row37_circle_collide_random_region() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(37);
    let mut seen = [0usize; 8];
    for i in 0..8192 {
        // The three hard-coded shapes live roughly in x in [-90,-5], y in [-50,110].
        let x = -100.0 + rng.unit() * 110.0;
        let y = -60.0 + rng.unit() * 180.0;
        let rad = rng.unit() * 40.0;
        unsafe {
            let (cv, rv) = ((c.circle_collide)(x, y, rad), (r.circle_collide)(x, y, rad));
            diff_assert!(
                cv == rv,
                "row37 #{i} circle_collide({}, {}, {}): C={cv} RS={rv}",
                show(x),
                show(y),
                show(rad)
            );
            if (0..8).contains(&cv) {
                seen[cv as usize] += 1;
            }
        }
    }
    let distinct = seen.iter().filter(|&&n| n > 0).count();
    assert!(
        distinct >= 4,
        "row37 too few distinct result bit-patterns: {seen:?}"
    );
}

#[test]
fn row38_circle_collide_targeted_bit_patterns() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(38);
    // Hard-coded shapes from c_src/src/lib.c:124..135.
    let circle_p = (-70.0f32, 0.0f32);
    let circle_r = 20.0f32;
    let aabb_min = (-40.0f32, -40.0f32);
    let aabb_max = (-15.0f32, -15.0f32);
    let cap_a = (-40.0f32, 40.0f32);
    let cap_b = (-20.0f32, 100.0f32);

    let anchors: Vec<(f32, f32)> = vec![
        circle_p,
        (circle_p.0 - circle_r, circle_p.1),
        (circle_p.0 + circle_r, circle_p.1),
        aabb_min,
        aabb_max,
        ((aabb_min.0 + aabb_max.0) * 0.5, (aabb_min.1 + aabb_max.1) * 0.5),
        (aabb_min.0, aabb_max.1),
        (aabb_max.0, aabb_min.1),
        cap_a,
        cap_b,
        ((cap_a.0 + cap_b.0) * 0.5, (cap_a.1 + cap_b.1) * 0.5),
        (0.0, 0.0),
        (-30.0, 0.0),
        (-50.0, 20.0),
        (1000.0, 1000.0),
    ];
    let radii = [
        0.0f32, -0.0, 1.0, 5.0, 20.0, 50.0, 200.0, 1000.0, 1.0e6, -5.0, 1.0e-30,
    ];

    let mut seen = [0usize; 8];
    for (k, &(x, y)) in anchors.iter().enumerate() {
        for &rad in &radii {
            unsafe {
                let (cv, rv) = ((c.circle_collide)(x, y, rad), (r.circle_collide)(x, y, rad));
                diff_assert!(
                    cv == rv,
                    "row38 anchor{k} circle_collide({}, {}, {}): C={cv} RS={rv}",
                    show(x),
                    show(y),
                    show(rad)
                );
                if (0..8).contains(&cv) {
                    seen[cv as usize] += 1;
                }
            }
        }
    }
    // Randomized jitter around each anchor to catch ULP-level boundary flips.
    for i in 0..2048 {
        let (ax, ay) = anchors[i % anchors.len()];
        let x = ax + rng.sym(2.0);
        let y = ay + rng.sym(2.0);
        let rad = rng.unit() * 60.0;
        unsafe {
            let (cv, rv) = ((c.circle_collide)(x, y, rad), (r.circle_collide)(x, y, rad));
            diff_assert!(
                cv == rv,
                "row38 jitter #{i} circle_collide({}, {}, {}): C={cv} RS={rv}",
                show(x),
                show(y),
                show(rad)
            );
            if (0..8).contains(&cv) {
                seen[cv as usize] += 1;
            }
        }
    }
    assert!(seen[0] > 0, "row38 never saw result 0b000");
    assert!(seen[7] > 0, "row38 never saw result 0b111 (huge radius)");
    let distinct = seen.iter().filter(|&&n| n > 0).count();
    assert!(distinct >= 6, "row38 too few distinct patterns: {seen:?}");
}

#[test]
fn row39_circle_collide_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &x in &sp {
        for &y in &sp {
            for &rad in &sp {
                unsafe {
                    let (cv, rv) =
                        ((c.circle_collide)(x, y, rad), (r.circle_collide)(x, y, rad));
                    diff_assert!(
                        cv == rv,
                        "row39 circle_collide({}, {}, {}): C={cv} RS={rv}",
                        show(x),
                        show(y),
                        show(rad)
                    );
                }
            }
        }
    }
}

#[test]
fn row39b_circle_collide_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(390);
    for i in 0..500_000u64 {
        let (x, y, rad) = (rng.raw_f32(), rng.raw_f32(), rng.raw_f32());
        unsafe {
            let (cv, rv) = ((c.circle_collide)(x, y, rad), (r.circle_collide)(x, y, rad));
            diff_assert!(
                cv == rv,
                "row39b #{i} circle_collide({}, {}, {}): C={cv} RS={rv}",
                show(x),
                show(y),
                show(rad)
            );
        }
    }
}

// ===========================================================================
// Row 40 — composed-pipeline cross-check
// ===========================================================================

/// Rebuild `circle_collide` out of the *low-level* exports of one library and
/// compare against that same library's one-shot wrapper. Run against BOTH
/// libraries, so a divergence in either the composition or the wrapper shows up
/// even when the two wrappers happen to agree with each other.
#[test]
fn row40_pipeline_recomposition() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(40);

    let circle = c2Circle {
        p: c2v { x: -70.0, y: 0.0 },
        r: 20.0,
    };
    let aabb = c2AABB {
        min: c2v { x: -40.0, y: -40.0 },
        max: c2v { x: -15.0, y: -15.0 },
    };
    let capsule = c2Capsule {
        a: c2v { x: -40.0, y: 40.0 },
        b: c2v { x: -20.0, y: 100.0 },
        r: 10.0,
    };

    for i in 0..4096 {
        let (x, y, rad) = match i % 3 {
            0 => (
                -100.0 + rng.unit() * 110.0,
                -60.0 + rng.unit() * 180.0,
                rng.unit() * 40.0,
            ),
            1 => (rng.coord(), rng.coord(), rng.radius()),
            _ => (rng.raw_f32(), rng.raw_f32(), rng.raw_f32()),
        };
        let circle_in = c2Circle {
            p: c2v { x, y },
            r: rad,
        };
        let ab = bytes_of(&circle_in);

        for lib in [c, r] {
            unsafe {
                // Path 1: low-level dispatcher, composed exactly as the C does.
                let b0 = (lib.c2Collided)(
                    ab.as_ptr(),
                    bytes_of(&circle).as_ptr(),
                    C2_TYPE_CIRCLE,
                );
                let b1 = (lib.c2Collided)(
                    ab.as_ptr(),
                    bytes_of(&aabb).as_ptr(),
                    C2_TYPE_AABB,
                );
                let b2 = (lib.c2Collided)(
                    ab.as_ptr(),
                    bytes_of(&capsule).as_ptr(),
                    C2_TYPE_CAPSULE,
                );
                let composed = b0 + (b1 << 1) + (b2 << 2);

                // Path 2: the shape predicates called directly (level 3).
                let d0 = (lib.c2CircletoCircle)(circle_in, circle);
                let d1 = (lib.c2CircletoAABB)(circle_in, aabb);
                let d2 = (lib.c2CircletoCapsule)(circle_in, capsule);
                let direct = d0 + (d1 << 1) + (d2 << 2);

                // Path 3: the one-shot wrapper.
                let oneshot = (lib.circle_collide)(x, y, rad);

                diff_assert!(
                    composed == oneshot && direct == oneshot,
                    "row40 #{i} [{}] internal inconsistency for ({}, {}, {}): \
                     dispatcher={composed} direct={direct} circle_collide={oneshot}",
                    lib.name,
                    show(x),
                    show(y),
                    show(rad)
                );
            }
        }

        // And the two libraries must agree with each other on every path.
        unsafe {
            diff_assert!(
                (c.circle_collide)(x, y, rad) == (r.circle_collide)(x, y, rad),
                "row40 #{i} circle_collide mismatch for ({}, {}, {})",
                show(x),
                show(y),
                show(rad)
            );
            diff_assert!(
                (c.c2CircletoCapsule)(circle_in, capsule)
                    == (r.c2CircletoCapsule)(circle_in, capsule),
                "row40 #{i} capsule mismatch"
            );
        }
    }
}
