//! Phase B — valid-path differential tests, CONFIGS.md rows 47..=59.
//!
//! L3 (`c2Collided`, the `void*` + `C2_TYPE` dispatcher) and L4
//! (`circle_collide`, the one-shot wrapper), plus composed-pipeline and
//! whole-surface sweeps.

mod common;
use common::*;

const SEED: u64 = 0x243F_6A88_85A3_08D3;
const N: usize = 20_000;

// ===========================================================================
// c2Collided — rows 47..=52
// ===========================================================================

/// Row 47 — `typeB = C2_TYPE_CIRCLE`, random finite buffers.
#[test]
fn cfg47_collided_type_circle_finite() {
    let mut rng = Rng::new(SEED ^ 47);
    let mut hits = 0usize;
    for _ in 0..N {
        let a = C2Circle {
            p: C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) },
            r: rng.range(0.0, 30.0),
        };
        let b = C2Circle {
            p: C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) },
            r: rng.range(0.0, 30.0),
        };
        let got = diff(
            || format!("c2Collided({a:?}, {b:?}, CIRCLE)"),
            |api| unsafe {
                (api.c2Collided)(
                    (&raw const a).cast(),
                    (&raw const b).cast(),
                    C2_TYPE_CIRCLE,
                )
            },
        );
        hits += (got != 0) as usize;
    }
    assert!(hits > N / 20 && hits < N - N / 20, "unbalanced: {hits}/{N}");
}

/// Row 48 — `typeB = C2_TYPE_AABB`, random finite buffers.
#[test]
fn cfg48_collided_type_aabb_finite() {
    let mut rng = Rng::new(SEED ^ 48);
    let mut hits = 0usize;
    for _ in 0..N {
        let a = C2Circle {
            p: C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) },
            r: rng.range(0.0, 30.0),
        };
        let x0 = rng.range(-50.0, 50.0);
        let y0 = rng.range(-50.0, 50.0);
        let b = C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + rng.range(0.0, 50.0), y: y0 + rng.range(0.0, 50.0) },
        };
        let got = diff(
            || format!("c2Collided({a:?}, {b:?}, AABB)"),
            |api| unsafe {
                (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), C2_TYPE_AABB)
            },
        );
        hits += (got != 0) as usize;
    }
    assert!(hits > N / 20 && hits < N - N / 20, "unbalanced: {hits}/{N}");
}

/// Row 49 — `typeB = C2_TYPE_CAPSULE`, random finite buffers (all three
/// internal regions are reached; asserted).
#[test]
fn cfg49_collided_type_capsule_finite() {
    let mut rng = Rng::new(SEED ^ 49);
    let mut hits = 0usize;
    let mut regions = [0usize; 3];
    for _ in 0..N {
        let a = C2Circle {
            p: C2v { x: rng.range(-90.0, 90.0), y: rng.range(-90.0, 90.0) },
            r: rng.range(0.0, 30.0),
        };
        let ax = rng.range(-50.0, 50.0);
        let ay = rng.range(-50.0, 50.0);
        let b = C2Capsule {
            a: C2v { x: ax, y: ay },
            b: C2v { x: ax + rng.range(-60.0, 60.0), y: ay + rng.range(-60.0, 60.0) },
            r: rng.range(0.0, 25.0),
        };
        regions[capsule_region(a.p, b) as usize] += 1;
        let got = diff(
            || format!("c2Collided({a:?}, {b:?}, CAPSULE)"),
            |api| unsafe {
                (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), C2_TYPE_CAPSULE)
            },
        );
        hits += (got != 0) as usize;
    }
    assert!(regions.iter().all(|r| *r > 0), "region coverage: {regions:?}");
    assert!(hits > 0, "no collision ever reported");
}

/// Row 50 — each valid `typeB` with fully random *bytes* in both buffers, so
/// NaN/inf/denormal reach the kernels through the dispatch layer.
#[test]
fn cfg50_collided_all_types_random_bytes() {
    let mut rng = Rng::new(SEED ^ 50);
    for _ in 0..N {
        let a_bytes: [u32; 3] = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let b_bytes: [u32; 5] = [
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        ];
        for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            diff(
                || format!("c2Collided(a={a_bytes:08x?}, b={b_bytes:08x?}, t={t})"),
                |api| unsafe {
                    (api.c2Collided)(
                        (&raw const a_bytes).cast(),
                        (&raw const b_bytes).cast(),
                        t,
                    )
                },
            );
        }
    }
    // pathological-weighted variant so NaN/inf dominate
    let mut rng = Rng::new(SEED ^ 0x5050);
    for _ in 0..N {
        let a = C2Circle { p: rng.v_path(), r: rng.pathological_f32() };
        let bb = [
            rng.pathological_f32(),
            rng.pathological_f32(),
            rng.pathological_f32(),
            rng.pathological_f32(),
            rng.pathological_f32(),
        ];
        for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            diff(
                || {
                    format!(
                        "c2Collided({a:?}, b={:08x?}, t={t})",
                        bb.map(|f| f.to_bits())
                    )
                },
                |api| unsafe {
                    (api.c2Collided)((&raw const a).cast(), (&raw const bb).cast(), t)
                },
            );
        }
    }
}

/// Row 51 — misaligned `A`/`B` pointers. GCC lowers `*(c2Circle *)A` to plain
/// `mov`/`movss` with no alignment requirement, so the C accepts offsets 1..3;
/// the Rust must read the same bytes (hence `read_unaligned`, not `read`).
#[test]
fn cfg51_collided_unaligned_pointers() {
    let mut rng = Rng::new(SEED ^ 51);
    for _ in 0..(N / 2) {
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let circle = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let x0 = rng.range(-50.0, 50.0);
        let y0 = rng.range(-50.0, 50.0);
        let aabb = C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + rng.range(0.0, 40.0), y: y0 + rng.range(0.0, 40.0) },
        };
        let cap = C2Capsule {
            a: rng.v_finite(),
            b: rng.v_finite(),
            r: rng.radius(),
        };
        for off_a in 0..4usize {
            for off_b in 0..4usize {
                let mut buf_a = [0u8; 32];
                let mut buf_b = [0u8; 32];
                let pa = unsafe { place(&mut buf_a, off_a, a) };
                for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                    let pb = match t {
                        C2_TYPE_CIRCLE => unsafe { place(&mut buf_b, off_b, circle) },
                        C2_TYPE_AABB => unsafe { place(&mut buf_b, off_b, aabb) },
                        _ => unsafe { place(&mut buf_b, off_b, cap) },
                    };
                    diff(
                        || {
                            format!(
                                "c2Collided(A@+{off_a}, B@+{off_b}, t={t}) a={a:?} \
                                 circle={circle:?} aabb={aabb:?} cap={cap:?}"
                            )
                        },
                        |api| unsafe { (api.c2Collided)(pa, pb, t) },
                    );
                }
            }
        }
    }
}

/// Row 52 — the dispatcher agrees with the kernel called directly, for all
/// three valid `typeB` values, on identical bytes. Compared *across* libraries
/// and *within* each library.
#[test]
fn cfg52_collided_matches_direct_kernels() {
    let mut rng = Rng::new(SEED ^ 52);
    for i in 0..N {
        let a = if i % 2 == 0 {
            C2Circle { p: rng.v_finite(), r: rng.radius() }
        } else {
            C2Circle { p: rng.v_path(), r: rng.pathological_f32() }
        };
        let circle = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let aabb = C2Aabb { min: rng.v_finite(), max: rng.v_finite() };
        let cap = C2Capsule { a: rng.v_finite(), b: rng.v_finite(), r: rng.radius() };

        diff(
            || format!("dispatch-vs-direct CIRCLE a={a:?} b={circle:?}"),
            |api| {
                let via = unsafe {
                    (api.c2Collided)(
                        (&raw const a).cast(),
                        (&raw const circle).cast(),
                        C2_TYPE_CIRCLE,
                    )
                };
                let direct = (api.c2CircletoCircle)(a, circle);
                assert_eq!(via, direct, "[{}] dispatcher disagrees with kernel", api.name);
                (via, direct)
            },
        );
        diff(
            || format!("dispatch-vs-direct AABB a={a:?} b={aabb:?}"),
            |api| {
                let via = unsafe {
                    (api.c2Collided)(
                        (&raw const a).cast(),
                        (&raw const aabb).cast(),
                        C2_TYPE_AABB,
                    )
                };
                let direct = (api.c2CircletoAABB)(a, aabb);
                assert_eq!(via, direct, "[{}] dispatcher disagrees with kernel", api.name);
                (via, direct)
            },
        );
        diff(
            || format!("dispatch-vs-direct CAPSULE a={a:?} b={cap:?}"),
            |api| {
                let via = unsafe {
                    (api.c2Collided)(
                        (&raw const a).cast(),
                        (&raw const cap).cast(),
                        C2_TYPE_CAPSULE,
                    )
                };
                let direct = (api.c2CircletoCapsule)(a, cap);
                assert_eq!(via, direct, "[{}] dispatcher disagrees with kernel", api.name);
                (via, direct)
            },
        );
    }
}

// ===========================================================================
// circle_collide — rows 53..=57
// ===========================================================================

/// Row 53 — random `(x, y, r)` over the geometric window of the three
/// hard-coded shapes; asserts every one of the 8 result bit patterns occurs.
#[test]
fn cfg53_circle_collide_random_window() {
    let mut rng = Rng::new(SEED ^ 53);
    let mut seen = [0usize; 8];
    for _ in 0..(N * 5) {
        let x = rng.range(-150.0, 150.0);
        let y = rng.range(-150.0, 150.0);
        let r = rng.range(0.0, 60.0);
        let got = diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
        assert!((0..8).contains(&got), "unexpected result {got}");
        seen[got as usize] += 1;
    }
    assert!(
        seen.iter().all(|s| *s > 0),
        "result-value coverage gap: {seen:?}"
    );
}

/// Row 54 — targeted values reaching each of the 8 possible return values.
#[test]
fn cfg54_circle_collide_all_eight_results() {
    // circle  : (-70,   0)          r=20
    // aabb    : (-40,-40)..(-15,-15)
    // capsule : (-40,  40)..(-20,100) r=10
    let cases: &[(f32, f32, f32)] = &[
        (200.0, 200.0, 1.0),    // 0: nothing
        (-70.0, 0.0, 1.0),      // 1: circle only
        (-27.0, -27.0, 1.0),    // 2: aabb only
        (-30.0, 60.0, 1.0),     // 4: capsule only
        (-55.0, -20.0, 20.0),   // 3: circle+aabb
        (-70.0, 20.0, 22.0),    // 5: circle+capsule
        (-35.0, 10.0, 25.0),    // 6: aabb+capsule
        (-45.0, 10.0, 40.0),    // 7: all three
    ];
    let mut seen = [false; 8];
    for &(x, y, r) in cases {
        let got = diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
        seen[got as usize] = true;
    }
    // Brute-force sweep guarantees all 8 are actually attainable and compared.
    let mut r = 0.0f32;
    while r <= 80.0 {
        let mut x = -160.0f32;
        while x <= 60.0 {
            let mut y = -120.0f32;
            while y <= 160.0 {
                let got = diff(
                    || format!("circle_collide({x}, {y}, {r})"),
                    |api| (api.circle_collide)(x, y, r),
                );
                seen[got as usize] = true;
                y += 7.5;
            }
            x += 7.5;
        }
        r += 6.5;
    }
    assert!(seen.iter().all(|s| *s), "not all 8 results reached: {seen:?}");
}

/// Row 55 — exact-boundary values: circle centre exactly touching each
/// hard-coded shape, and one ULP either side.
#[test]
fn cfg55_circle_collide_exact_boundaries() {
    let mut cases: Vec<(f32, f32, f32)> = Vec::new();

    // --- against circle ((-70,0), r=20): touch when |p-(-70,0)| == r+20
    for r in [0.0f32, 1.0, 4.0, 20.0, 32.0, 64.0] {
        let d = r + 20.0;
        for (dx, dy) in [(d, 0.0f32), (-d, 0.0), (0.0, d), (0.0, -d)] {
            cases.push((-70.0 + dx, dy, r));
            cases.push((-70.0 + ulp_up(dx), dy, r));
            cases.push((-70.0 + ulp_down(dx), dy, r));
        }
    }
    // --- against aabb (-40,-40)..(-15,-15): touch when distance to box == r
    for r in [0.0f32, 1.0, 4.0, 16.0, 64.0] {
        cases.push((-15.0 + r, -27.5, r)); // +x face
        cases.push((-40.0 - r, -27.5, r)); // -x face
        cases.push((-27.5, -15.0 + r, r)); // +y face
        cases.push((-27.5, -40.0 - r, r)); // -y face
        cases.push((ulp_up(-15.0 + r), -27.5, r));
        cases.push((ulp_down(-15.0 + r), -27.5, r));
        // corners
        let dd = r * std::f32::consts::FRAC_1_SQRT_2;
        cases.push((-15.0 + dd, -15.0 + dd, r));
        cases.push((-40.0 - dd, -40.0 - dd, r));
    }
    // --- against capsule (-40,40)..(-20,100) r=10
    for r in [0.0f32, 1.0, 4.0, 16.0, 64.0] {
        let d = r + 10.0;
        // beyond each cap along the axis direction (20,60)/|(20,60)|
        let len = (20.0f32 * 20.0 + 60.0 * 60.0).sqrt();
        let (ux, uy) = (20.0 / len, 60.0 / len);
        cases.push((-40.0 - ux * d, 40.0 - uy * d, r));
        cases.push((-20.0 + ux * d, 100.0 + uy * d, r));
        // perpendicular from the shaft midpoint
        cases.push((-30.0 - uy * d, 70.0 + ux * d, r));
        cases.push((-30.0 + uy * d, 70.0 - ux * d, r));
    }
    // exact hard-coded shape parameters themselves
    for &(x, y) in &[
        (-70.0f32, 0.0f32),
        (-40.0, -40.0),
        (-15.0, -15.0),
        (-40.0, 40.0),
        (-20.0, 100.0),
        (0.0, 0.0),
        (-0.0, -0.0),
    ] {
        for r in [0.0f32, -0.0, 1.0, 10.0, 20.0] {
            cases.push((x, y, r));
        }
    }

    for (x, y, r) in cases {
        diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
    }
}

/// Row 56 — arbitrary bit patterns for `x`, `y`, `r`.
#[test]
fn cfg56_circle_collide_any_bits() {
    let mut rng = Rng::new(SEED ^ 56);
    for i in 0..(N * 2) {
        let (x, y, r) = if i % 2 == 0 {
            (rng.any_f32(), rng.any_f32(), rng.any_f32())
        } else {
            (
                rng.pathological_f32(),
                rng.pathological_f32(),
                rng.pathological_f32(),
            )
        };
        diff(
            || {
                format!(
                    "circle_collide({:#010x}, {:#010x}, {:#010x})",
                    x.to_bits(),
                    y.to_bits(),
                    r.to_bits()
                )
            },
            |api| (api.circle_collide)(x, y, r),
        );
    }
    // full special-value grid
    for &x in special_values() {
        for &y in special_values() {
            for &r in special_values() {
                diff(
                    || {
                        format!(
                            "circle_collide({:#010x}, {:#010x}, {:#010x})",
                            x.to_bits(),
                            y.to_bits(),
                            r.to_bits()
                        )
                    },
                    |api| (api.circle_collide)(x, y, r),
                );
            }
        }
    }
}

/// Row 57 — wide random sweep in the hot geometric window.
#[test]
fn cfg57_circle_collide_wide_sweep() {
    let mut rng = Rng::new(SEED ^ 57);
    for _ in 0..200_000 {
        let x = rng.range(-200.0, 200.0);
        let y = rng.range(-200.0, 200.0);
        let r = match rng.below(4) {
            0 => rng.range(0.0, 5.0),
            1 => rng.range(0.0, 30.0),
            2 => rng.range(0.0, 120.0),
            _ => rng.range(-30.0, 30.0),
        };
        diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
    }
}

// ===========================================================================
// Composed pipeline & whole-surface sweep — rows 58..=59
// ===========================================================================

/// Row 58 — the composed pipeline that `c2CircletoAABB` runs internally,
/// rebuilt from the individual exported primitives and compared step by step,
/// then checked against `c2Clampv` and the kernel itself.
#[test]
fn cfg58_composed_pipeline() {
    let mut rng = Rng::new(SEED ^ 58);
    for i in 0..N {
        let (p, r, lo, hi) = if i % 2 == 0 {
            (rng.v_finite(), rng.radius(), rng.v_finite(), rng.v_finite())
        } else {
            (rng.v_path(), rng.pathological_f32(), rng.v_path(), rng.v_path())
        };
        diff(
            || format!("pipeline p={p:?} r={r:#x?} lo={lo:?} hi={hi:?}", r = r.to_bits()),
            |api| {
                // c2Clampv(p, lo, hi) == c2Maxv(lo, c2Minv(p, hi))
                let mid = (api.c2Minv)(p, hi);
                let clamped_manual = (api.c2Maxv)(lo, mid);
                let clamped = (api.c2Clampv)(p, lo, hi);
                assert_eq!(
                    vbits(clamped_manual),
                    vbits(clamped),
                    "[{}] c2Clampv != c2Maxv(lo, c2Minv(a, hi))",
                    api.name
                );
                // c2CircletoAABB's remaining steps
                let ab = (api.c2Sub)(p, clamped);
                let d2 = (api.c2Dot)(ab, ab);
                let r2 = r * r;
                let manual = (d2 < r2) as std::ffi::c_int;
                let kernel = (api.c2CircletoAABB)(
                    C2Circle { p, r },
                    C2Aabb { min: lo, max: hi },
                );
                assert_eq!(
                    manual, kernel,
                    "[{}] recomposed pipeline != c2CircletoAABB",
                    api.name
                );
                (
                    vbits(mid),
                    vbits(clamped),
                    vbits(ab),
                    d2.to_bits(),
                    manual,
                    kernel,
                )
            },
        );
    }
}

/// Row 59 — monolithic sweep: every one of the 12 exported symbols is called
/// with a shared random state and all outputs compared bit-for-bit.
#[test]
fn cfg59_all_symbols_sweep() {
    let mut rng = Rng::new(SEED ^ 59);
    for i in 0..100_000 {
        let pick = |rng: &mut Rng| -> C2v {
            match i % 3 {
                0 => rng.v_finite(),
                1 => rng.v_any(),
                _ => rng.v_path(),
            }
        };
        let a = pick(&mut rng);
        let b = pick(&mut rng);
        let cc = pick(&mut rng);
        let s = match i % 3 {
            0 => rng.finite_f32(),
            1 => rng.any_f32(),
            _ => rng.pathological_f32(),
        };
        let r1 = match i % 3 {
            0 => rng.radius(),
            1 => rng.any_f32(),
            _ => rng.pathological_f32(),
        };
        let r2 = match i % 3 {
            0 => rng.radius(),
            1 => rng.any_f32(),
            _ => rng.pathological_f32(),
        };
        let ty = (rng.next_u32() % 5) as std::ffi::c_int - 1; // -1..3 (incl. invalid)

        let circle_a = C2Circle { p: a, r: r1 };
        let circle_b = C2Circle { p: b, r: r2 };
        let aabb = C2Aabb { min: b, max: cc };
        let cap = C2Capsule { a: b, b: cc, r: r2 };

        diff(
            || {
                format!(
                    "sweep#{i} a={a:?} b={b:?} c={cc:?} s={s:#010x} r1={r1:#010x} \
                     r2={r2:#010x} ty={ty}",
                    s = s.to_bits(),
                    r1 = r1.to_bits(),
                    r2 = r2.to_bits()
                )
            },
            |api| {
                (
                    vbits((api.c2V)(a.x, a.y)),
                    vbits((api.c2Mulvs)(a, s)),
                    vbits((api.c2Maxv)(a, b)),
                    vbits((api.c2Minv)(a, b)),
                    vbits((api.c2Clampv)(a, b, cc)),
                    vbits((api.c2Sub)(a, b)),
                    (api.c2Dot)(a, b).to_bits(),
                    (api.c2CircletoCircle)(circle_a, circle_b),
                    (api.c2CircletoAABB)(circle_a, aabb),
                    (api.c2CircletoCapsule)(circle_a, cap),
                    unsafe {
                        (api.c2Collided)(
                            (&raw const circle_a).cast(),
                            (&raw const cap).cast(),
                            ty,
                        )
                    },
                    (api.circle_collide)(a.x, a.y, r1),
                )
            },
        );
    }
}
