//! Phase B/C reinforcement: adversarial sweeps aimed at the parts of `c2GJK`
//! that random floating-point inputs almost never reach —
//!   * exact ties in `c2Support` (`dot > dmax` is strict),
//!   * `d1 == d0` exactly (the `if (d1 > d0) break;` boundary),
//!   * the highest reachable value of `iter` (the `while (iter < 20)` cap),
//!   * exact `dist == rA + rB` (the `dist > rA + rB` boundary).
//!
//! Everything uses small integers and exact powers of two so the float
//! arithmetic is exact and the boundaries are actually hit rather than
//! approached. Also prints the reachable-iteration histogram so the claim
//! "the 20-iteration cap is unreachable for these proxies" is measured, not
//! assumed.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::collections::BTreeMap;
use std::ffi::{c_int, c_void};

#[repr(C, align(16))]
struct Blob([u8; 32]);

fn blob_of<T: Copy>(v: &T) -> Blob {
    let mut b = Blob([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

struct Out {
    d: f32,
    a: c2v,
    b: c2v,
    it: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
unsafe fn run(
    f: &libloading::Symbol<FnGJK>,
    A: &Blob,
    tA: u32,
    ax: *const c2x,
    B: &Blob,
    tB: u32,
    bx: *const c2x,
    use_radius: c_int,
    cache_in: c2GJKCache,
    with_cache: bool,
) -> Out {
    unsafe {
        let mut a = c2v { x: 7777.0, y: -7777.0 };
        let mut b = c2v { x: -8888.0, y: 8888.0 };
        let mut it: c_int = -1;
        let mut cache = cache_in;
        let d = f(
            A.0.as_ptr() as *const c_void,
            tA,
            ax,
            B.0.as_ptr() as *const c_void,
            tB,
            bx,
            &mut a,
            &mut b,
            use_radius,
            &mut it,
            if with_cache { &mut cache } else { std::ptr::null_mut() },
        );
        Out { d, a, b, it, cache }
    }
}

fn same(c: &Out, r: &Out) -> bool {
    feq(c.d, r.d) && veq(c.a, r.a) && veq(c.b, r.b) && c.it == r.it && cache_eq(&c.cache, &r.cache)
}

fn describe(c: &Out, r: &Out) -> String {
    format!(
        "C{{d={} a={} b={} it={} cache={:?}}} R{{d={} a={} b={} it={} cache={:?}}}",
        fdesc(c.d), vdesc(c.a), vdesc(c.b), c.it, c.cache,
        fdesc(r.d), vdesc(r.a), vdesc(r.b), r.it, r.cache
    )
}

/// Exhaustive integer-grid sweep. All coordinates are small integers, so every
/// dot product, determinant and distance is computed exactly — this is what
/// makes the strict `>` / `<=` boundaries in the C reachable.
#[test]
fn adversarial_exact_integer_grid() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut it_hist: BTreeMap<c_int, usize> = BTreeMap::new();
    let mut zero_dist = 0usize;
    let mut exact_touch = 0usize;
    let mut cases = 0usize;

    // Integer AABBs, circles and capsules on a small lattice.
    let coords: [f32; 9] = [-4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0];
    let radii: [f32; 5] = [0.0, 1.0, 2.0, 3.0, 4.0];

    for &dx in &coords {
        for &dy in &coords {
            // A: unit square at the origin. B: unit square / circle / capsule,
            // translated by exactly (dx, dy) — so many configurations produce
            // exact ties in c2Support and exact `dist == rA + rB`.
            let aabb_a = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
            for &ra in &radii {
                let circ_a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra };
                let cap_a = c2Capsule {
                    a: c2v { x: -2.0, y: 0.0 },
                    b: c2v { x: 2.0, y: 0.0 },
                    r: ra,
                };
                for &rb in &radii {
                    let variants_a: [(u32, Blob); 3] = [
                        (C2_TYPE_AABB, blob_of(&aabb_a)),
                        (C2_TYPE_CIRCLE, blob_of(&circ_a)),
                        (C2_TYPE_CAPSULE, blob_of(&cap_a)),
                    ];
                    let aabb_b = c2AABB {
                        min: c2v { x: dx - 1.0, y: dy - 1.0 },
                        max: c2v { x: dx + 1.0, y: dy + 1.0 },
                    };
                    let circ_b = c2Circle { p: c2v { x: dx, y: dy }, r: rb };
                    let cap_b = c2Capsule {
                        a: c2v { x: dx, y: dy - 2.0 },
                        b: c2v { x: dx, y: dy + 2.0 },
                        r: rb,
                    };
                    let variants_b: [(u32, Blob); 3] = [
                        (C2_TYPE_AABB, blob_of(&aabb_b)),
                        (C2_TYPE_CIRCLE, blob_of(&circ_b)),
                        (C2_TYPE_CAPSULE, blob_of(&cap_b)),
                    ];

                    for (tA, bA) in &variants_a {
                        for (tB, bB) in &variants_b {
                            for use_radius in [0i32, 1] {
                                for with_cache in [false, true] {
                                    let fresh = c2GJKCache {
                                        metric: 0.0,
                                        count: 0,
                                        iA: [0; 3],
                                        iB: [0; 3],
                                        div: 0.0,
                                    };
                                    unsafe {
                                        let oc = run(&g_c, bA, *tA, std::ptr::null(), bB, *tB,
                                            std::ptr::null(), use_radius, fresh, with_cache);
                                        let or = run(&g_r, bA, *tA, std::ptr::null(), bB, *tB,
                                            std::ptr::null(), use_radius, fresh, with_cache);
                                        assert!(
                                            same(&oc, &or),
                                            "grid dx{dx} dy{dy} ra{ra} rb{rb} tA{tA} tB{tB} ur{use_radius} cache{with_cache}: {}",
                                            describe(&oc, &or)
                                        );
                                        cases += 1;
                                        *it_hist.entry(oc.it).or_default() += 1;
                                        if oc.d == 0.0 {
                                            zero_dist += 1;
                                        }
                                        if use_radius == 1 && oc.d == 0.0 && veq_strict(oc.a, oc.b) {
                                            exact_touch += 1;
                                        }

                                        // Feed the cache back in (the warm path)
                                        // twice, still on exact integers.
                                        if with_cache {
                                            let mut kc = oc.cache;
                                            let mut kr = or.cache;
                                            for gn in 0..2 {
                                                let oc2 = run(&g_c, bA, *tA, std::ptr::null(), bB,
                                                    *tB, std::ptr::null(), use_radius, kc, true);
                                                let or2 = run(&g_r, bA, *tA, std::ptr::null(), bB,
                                                    *tB, std::ptr::null(), use_radius, kr, true);
                                                assert!(
                                                    same(&oc2, &or2),
                                                    "grid warm gn{gn} dx{dx} dy{dy} ra{ra} rb{rb} tA{tA} tB{tB}: {}",
                                                    describe(&oc2, &or2)
                                                );
                                                *it_hist.entry(oc2.it).or_default() += 1;
                                                kc = oc2.cache;
                                                kr = or2.cache;
                                                cases += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    eprintln!("exact-integer grid: {cases} differential calls");
    eprintln!("  reachable `iterations` histogram: {it_hist:?}");
    eprintln!("  dist == 0: {zero_dist}, exact witness collapse: {exact_touch}");
    assert!(cases > 100_000, "grid too small: {cases}");
    assert!(zero_dist > 100, "no overlap cases reached");
    assert!(exact_touch > 100, "no exact collapse cases reached");
    // The C's iteration cap is 20; record what is actually reachable with
    // proxies of 1, 2 and 4 vertices.
    let max_it = *it_hist.keys().max().unwrap();
    eprintln!("  max reachable iterations = {max_it} (C's cap is 20)");
}

/// Symmetric / tie-heavy configurations: identical shapes, mirrored shapes and
/// shapes offset along an axis of symmetry. These are the cases where
/// `c2Support` sees `dot == dmax` and where `d1 == d0` can hold exactly.
#[test]
fn adversarial_symmetric_and_ties() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut it_hist: BTreeMap<c_int, usize> = BTreeMap::new();
    let mut cases = 0usize;

    // Offsets along exact symmetry axes, powers of two, and zero.
    let offs: [f32; 13] = [
        0.0, -0.0, 0.5, -0.5, 1.0, -1.0, 2.0, -2.0, 4.0, -4.0, 8.0, 1.5, 3.0,
    ];
    let sizes: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

    for &s in &sizes {
        for &ox in &offs {
            for &oy in &offs {
                let shapes_a: [(u32, Blob); 5] = [
                    (C2_TYPE_AABB, blob_of(&c2AABB {
                        min: c2v { x: -s, y: -s },
                        max: c2v { x: s, y: s },
                    })),
                    (C2_TYPE_CIRCLE, blob_of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: s })),
                    (C2_TYPE_CAPSULE, blob_of(&c2Capsule {
                        a: c2v { x: -s, y: 0.0 },
                        b: c2v { x: s, y: 0.0 },
                        r: s,
                    })),
                    // zero-length capsule (degenerate, two identical verts ->
                    // guaranteed tie in c2Support)
                    (C2_TYPE_CAPSULE, blob_of(&c2Capsule {
                        a: c2v { x: 0.0, y: 0.0 },
                        b: c2v { x: 0.0, y: 0.0 },
                        r: s,
                    })),
                    // zero-area AABB (four identical verts -> total tie)
                    (C2_TYPE_AABB, blob_of(&c2AABB {
                        min: c2v { x: 0.0, y: 0.0 },
                        max: c2v { x: 0.0, y: 0.0 },
                    })),
                ];
                let shapes_b: [(u32, Blob); 5] = [
                    (C2_TYPE_AABB, blob_of(&c2AABB {
                        min: c2v { x: ox - s, y: oy - s },
                        max: c2v { x: ox + s, y: oy + s },
                    })),
                    (C2_TYPE_CIRCLE, blob_of(&c2Circle { p: c2v { x: ox, y: oy }, r: s })),
                    (C2_TYPE_CAPSULE, blob_of(&c2Capsule {
                        a: c2v { x: ox - s, y: oy },
                        b: c2v { x: ox + s, y: oy },
                        r: s,
                    })),
                    (C2_TYPE_CAPSULE, blob_of(&c2Capsule {
                        a: c2v { x: ox, y: oy },
                        b: c2v { x: ox, y: oy },
                        r: s,
                    })),
                    (C2_TYPE_AABB, blob_of(&c2AABB {
                        min: c2v { x: ox, y: oy },
                        max: c2v { x: ox, y: oy },
                    })),
                ];
                for (tA, bA) in &shapes_a {
                    for (tB, bB) in &shapes_b {
                        for use_radius in [0i32, 1] {
                            // Also exercise exact 90/180/270-degree rotations,
                            // which keep the arithmetic exact.
                            let rots = [
                                c2r { c: 1.0, s: 0.0 },
                                c2r { c: 0.0, s: 1.0 },
                                c2r { c: -1.0, s: 0.0 },
                                c2r { c: 0.0, s: -1.0 },
                            ];
                            for (k, rr) in rots.iter().enumerate() {
                                let xa = c2x { p: c2v { x: 0.0, y: 0.0 }, r: *rr };
                                let xb = c2x { p: c2v { x: ox, y: oy }, r: rots[(k + 1) % 4] };
                                let fresh = c2GJKCache {
                                    metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0,
                                };
                                unsafe {
                                    let oc = run(&g_c, bA, *tA, &xa, bB, *tB, &xb, use_radius, fresh, true);
                                    let or = run(&g_r, bA, *tA, &xa, bB, *tB, &xb, use_radius, fresh, true);
                                    assert!(
                                        same(&oc, &or),
                                        "sym s{s} ox{ox} oy{oy} tA{tA} tB{tB} ur{use_radius} rot{k}: {}",
                                        describe(&oc, &or)
                                    );
                                    *it_hist.entry(oc.it).or_default() += 1;
                                    cases += 1;

                                    let mut kc = oc.cache;
                                    let mut kr = or.cache;
                                    for gn in 0..3 {
                                        let oc2 = run(&g_c, bA, *tA, &xa, bB, *tB, &xb, use_radius, kc, true);
                                        let or2 = run(&g_r, bA, *tA, &xa, bB, *tB, &xb, use_radius, kr, true);
                                        assert!(
                                            same(&oc2, &or2),
                                            "sym warm gn{gn} s{s} ox{ox} oy{oy} tA{tA} tB{tB} rot{k}: {}",
                                            describe(&oc2, &or2)
                                        );
                                        *it_hist.entry(oc2.it).or_default() += 1;
                                        kc = oc2.cache;
                                        kr = or2.cache;
                                        cases += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("symmetric/tie sweep: {cases} differential calls");
    eprintln!("  reachable `iterations` histogram: {it_hist:?}");
    assert!(cases > 50_000, "symmetric sweep too small: {cases}");
}

/// Exact `dist == rA + rB`: build circle-vs-circle pairs where the centre
/// separation is exactly the radius sum (3-4-5 triples, so `sqrtf` is exact).
#[test]
fn adversarial_exact_radius_boundary() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    // (dx, dy, exact distance) Pythagorean triples scaled by powers of two.
    let triples: [(f32, f32, f32); 6] = [
        (3.0, 4.0, 5.0),
        (6.0, 8.0, 10.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (0.0, 16.0, 16.0),
        (16.0, 0.0, 16.0),
    ];
    let mut boundary_hits = 0usize;
    for (dx, dy, dist) in triples {
        // Split the exact distance into rA + rB in several exact ways, plus one
        // step below and above so the strict `>` boundary is straddled.
        for k in 0..=8 {
            let rA = dist * (k as f32) / 8.0;
            for delta in [0.0f32, -f32::EPSILON * dist, f32::EPSILON * dist] {
                let rB = dist - rA + delta;
                let ca = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA };
                let cb = c2Circle { p: c2v { x: dx, y: dy }, r: rB };
                let bA = blob_of(&ca);
                let bB = blob_of(&cb);
                for use_radius in [0i32, 1] {
                    let fresh =
                        c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 };
                    unsafe {
                        let oc = run(&g_c, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB,
                            C2_TYPE_CIRCLE, std::ptr::null(), use_radius, fresh, true);
                        let or = run(&g_r, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB,
                            C2_TYPE_CIRCLE, std::ptr::null(), use_radius, fresh, true);
                        assert!(
                            same(&oc, &or),
                            "radius boundary d{dist} rA{rA} rB{rB} ur{use_radius}: {}",
                            describe(&oc, &or)
                        );
                        if delta == 0.0 {
                            boundary_hits += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(boundary_hits > 50, "exact radius boundary under-covered ({boundary_hits})");
}

/// Randomised sweep over exact half-integer coordinates, in bulk. Half-integers
/// keep the arithmetic exact while still covering a large configuration space.
#[test]
fn adversarial_bulk_half_integer_random() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut rng = Rng::new(SEED ^ 0xABCD);
    let half = |rng: &mut Rng| -> f32 { (rng.below(65) as f32 - 32.0) * 0.5 };
    let mut it_hist: BTreeMap<c_int, usize> = BTreeMap::new();

    for i in 0..40_000usize {
        let tA = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][(i % 3) as usize];
        let tB = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][((i / 3) % 3) as usize];
        let mk = |rng: &mut Rng, ty: u32| -> Blob {
            match ty {
                C2_TYPE_CIRCLE => blob_of(&c2Circle {
                    p: c2v { x: half(rng), y: half(rng) },
                    r: (rng.below(9) as f32) * 0.5,
                }),
                C2_TYPE_AABB => blob_of(&c2AABB {
                    min: c2v { x: half(rng), y: half(rng) },
                    max: c2v { x: half(rng), y: half(rng) },
                }),
                _ => blob_of(&c2Capsule {
                    a: c2v { x: half(rng), y: half(rng) },
                    b: c2v { x: half(rng), y: half(rng) },
                    r: (rng.below(9) as f32) * 0.5,
                }),
            }
        };
        let bA = mk(&mut rng, tA);
        let bB = mk(&mut rng, tB);
        let use_radius = (i % 2) as c_int;
        let with_cache = i % 3 != 0;
        let fresh = c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 };
        unsafe {
            let mut kc = fresh;
            let mut kr = fresh;
            for gn in 0..4 {
                let oc = run(&g_c, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    use_radius, kc, with_cache);
                let or = run(&g_r, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    use_radius, kr, with_cache);
                assert!(
                    same(&oc, &or),
                    "bulk i{i} gn{gn} tA{tA} tB{tB} ur{use_radius} cache{with_cache}: {}",
                    describe(&oc, &or)
                );
                *it_hist.entry(oc.it).or_default() += 1;
                kc = oc.cache;
                kr = or.cache;
            }
        }
    }
    eprintln!("bulk half-integer sweep: 160000 differential calls");
    eprintln!("  reachable `iterations` histogram: {it_hist:?}");
}

// ===========================================================================
// The `d1 > d0` boundary (`c2GJK`'s no-progress guard).
//
// `d0` starts at FLT_MAX and is then the previous iteration's `Dot(p,p)`.
// `d1 == d0` exactly is reachable — e.g. when the reduced simplex yields a `p`
// that is a permutation/mirror of the previous one (float addition is
// commutative, so `x*x + y*y` repeats bit-for-bit) — and it flips the loop's
// exit point, changing `iterations`, the witness points, the returned distance
// and the written-back cache.
//
// Random small-magnitude shapes never reach it. What does reach it is the
// COMBINATION of extreme magnitudes with a hand-built warm cache, fed through
// several generations. This test reproduces that space with a fixed seed, plus
// the exact bit-pattern witnesses found by a mutation search.
//
// IMPORTANT: cache indices are always kept `< proxy vertex count`. A larger
// index makes the C read `c2Proxy.verts[i]` that `c2MakeProxy` never wrote —
// uninitialised stack, i.e. UB (see ERRORS.md row 46), not a valid input.
// ===========================================================================

fn nverts(ty: u32) -> u32 {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    }
}

#[test]
fn adversarial_extremes_with_warm_cache() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    // Mixed pool: half-integers plus the extreme / awkward magnitudes that make
    // `Dot` and `Det2` overflow, underflow and repeat exactly.
    let mut vals: Vec<f32> = (-16i32..=16).map(|v| v as f32 * 0.5).collect();
    vals.extend_from_slice(&[
        0.0, -0.0, 1.0 / 3.0, -1.0 / 3.0, 1e-20, -1e-20, 1e18, -1e18,
        f32::MAX, -f32::MAX, f32::MIN_POSITIVE, f32::EPSILON, 1e4, -1e4, 1e8, -1e8,
        65536.0, -65536.0, 16_777_216.0, 0.1, -0.1, 3.4028e38,
    ]);

    let rots = [
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: 0.0, s: -1.0 },
        c2r { c: 0.6, s: 0.8 },
        c2r { c: -0.8, s: 0.6 },
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 2.0, s: -2.0 },
    ];

    let mut rng = Rng::new(SEED ^ 0xD1D0);
    let mut it_hist: BTreeMap<c_int, usize> = BTreeMap::new();

    for i in 0..200_000usize {
        let mut pick = |rng: &mut Rng| vals[rng.below(vals.len() as u32) as usize];
        let tA = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][rng.below(3) as usize];
        let tB = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][rng.below(3) as usize];
        let mk = |rng: &mut Rng, ty: u32, pick: &mut dyn FnMut(&mut Rng) -> f32| -> Blob {
            match ty {
                C2_TYPE_CIRCLE => blob_of(&c2Circle {
                    p: c2v { x: pick(rng), y: pick(rng) },
                    r: pick(rng).abs(),
                }),
                C2_TYPE_AABB => blob_of(&c2AABB {
                    min: c2v { x: pick(rng), y: pick(rng) },
                    max: c2v { x: pick(rng), y: pick(rng) },
                }),
                _ => blob_of(&c2Capsule {
                    a: c2v { x: pick(rng), y: pick(rng) },
                    b: c2v { x: pick(rng), y: pick(rng) },
                    r: pick(rng).abs(),
                }),
            }
        };
        let bA = mk(&mut rng, tA, &mut pick);
        let bB = mk(&mut rng, tB, &mut pick);
        let use_radius = rng.below(2) as c_int;

        let xa = c2x {
            p: c2v { x: pick(&mut rng), y: pick(&mut rng) },
            r: rots[rng.below(8) as usize],
        };
        let xb = c2x {
            p: c2v { x: pick(&mut rng), y: pick(&mut rng) },
            r: rots[rng.below(8) as usize],
        };
        let (pax, pbx): (*const c2x, *const c2x) = match rng.below(4) {
            0 => (std::ptr::null(), std::ptr::null()),
            1 => (&xa, std::ptr::null()),
            2 => (std::ptr::null(), &xb),
            _ => (&xa, &xb),
        };

        let (na, nb) = (nverts(tA), nverts(tB));
        let base = match rng.below(4) {
            0 => c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 },
            1 => c2GJKCache {
                metric: 0.0,
                count: 1,
                iA: [rng.below(na) as c_int; 3],
                iB: [rng.below(nb) as c_int; 3],
                div: 1.0,
            },
            2 => c2GJKCache {
                metric: pick(&mut rng),
                count: 2,
                iA: [rng.below(na) as c_int, rng.below(na) as c_int, 0],
                iB: [rng.below(nb) as c_int, rng.below(nb) as c_int, 0],
                div: 1.0,
            },
            _ => c2GJKCache {
                metric: pick(&mut rng) * 1e6,
                count: 3,
                iA: [rng.below(na) as c_int, rng.below(na) as c_int, rng.below(na) as c_int],
                iB: [rng.below(nb) as c_int, rng.below(nb) as c_int, rng.below(nb) as c_int],
                div: pick(&mut rng),
            },
        };

        let mut kc = base;
        let mut kr = base;
        for gn in 0..3 {
            unsafe {
                let oc = run(&g_c, &bA, tA, pax, &bB, tB, pbx, use_radius, kc, true);
                let or = run(&g_r, &bA, tA, pax, &bB, tB, pbx, use_radius, kr, true);
                assert!(
                    same(&oc, &or),
                    "extremes+warm-cache i{i} gn{gn} tA{tA} tB{tB} ur{use_radius} cacheIn={kc:?}: {}",
                    describe(&oc, &or)
                );
                *it_hist.entry(oc.it).or_default() += 1;
                kc = oc.cache;
                kr = or.cache;
            }
        }
    }
    eprintln!("extremes+warm-cache sweep: 600000 differential calls");
    eprintln!("  reachable `iterations` histogram: {it_hist:?}");
}

/// Exact bit-pattern regressions: configurations found by a mutation search to
/// sit precisely on the `d1 > d0` boundary. Kept as literals so they are
/// checked on every run regardless of the RNG.
#[test]
fn regression_d1_eq_d0_boundary_witnesses() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    // (tA, A field bits, tB, B field bits, use_radius, cache metric/count/iA/iB/div bits)
    #[allow(clippy::type_complexity)]
    let cases: [(u32, [u32; 5], u32, [u32; 5], c_int, (u32, c_int, [c_int; 3], [c_int; 3], u32)); 4] = [
        (
            C2_TYPE_CAPSULE, [0xc61c4000, 0x4b800000, 0x40a00000, 0x40a00000, 0x40200000],
            C2_TYPE_CAPSULE, [0xc0c00000, 0x00800000, 0x40d00000, 0x5d5e0b6b, 0x41000000],
            0, (0x4a989680, 3, [0, 0, 0], [0, 0, 1], 0xff7fffff),
        ),
        (
            C2_TYPE_CAPSULE, [0xc61c4000, 0x4b800000, 0x40a00000, 0x40a00000, 0x40200000],
            C2_TYPE_CAPSULE, [0xc0c00000, 0x00800000, 0x40d00000, 0x5d5e0b6b, 0x41000000],
            0, (0xe40797d1, 3, [1, 0, 1], [1, 0, 0], 0x7f800000),
        ),
        (
            C2_TYPE_CAPSULE, [0xbeaaaaab, 0x9e3ce508, 0xdd5e0b6b, 0xbdcccccd, 0x4b800000],
            C2_TYPE_AABB, [0xc0c00000, 0xbf800000, 0x4cbebc20, 0x461c4000, 0x00000000],
            0, (0xc61c4000, 2, [1, 0, 0], [2, 2, 0], 0x3f800000),
        ),
        (
            C2_TYPE_AABB, [0x00000000, 0xc0e00000, 0x4cbebc20, 0x5d5e0b6b, 0x00000000],
            C2_TYPE_CAPSULE, [0xc0a00000, 0xccbebc20, 0x3f000000, 0x3fc00000, 0x40800000],
            0, (0x40200000, 2, [1, 1, 0], [1, 0, 0], 0x3f800000),
        ),
    ];

    for (i, (tA, ab, tB, bb, use_radius, (mb, count, iA, iB, db))) in cases.iter().enumerate() {
        // Rebuild the shape blobs from the raw field bit patterns.
        let mut bloba = Blob([0u8; 32]);
        let mut blobb = Blob([0u8; 32]);
        for k in 0..5 {
            bloba.0[k * 4..k * 4 + 4].copy_from_slice(&ab[k].to_le_bytes());
            blobb.0[k * 4..k * 4 + 4].copy_from_slice(&bb[k].to_le_bytes());
        }
        // `iB` may exceed the capsule vertex count in the recorded witness only
        // where the shape is an AABB; assert the invariant explicitly.
        for k in 0..(*count as usize).min(3) {
            assert!(
                (iA[k] as u32) < nverts(*tA),
                "witness {i}: iA[{k}]={} out of range for type {tA}", iA[k]
            );
            assert!(
                (iB[k] as u32) < nverts(*tB),
                "witness {i}: iB[{k}]={} out of range for type {tB}", iB[k]
            );
        }

        let base = c2GJKCache {
            metric: f32::from_bits(*mb),
            count: *count,
            iA: *iA,
            iB: *iB,
            div: f32::from_bits(*db),
        };
        let mut kc = base;
        let mut kr = base;
        for gn in 0..4 {
            unsafe {
                let oc = run(&g_c, &bloba, *tA, std::ptr::null(), &blobb, *tB,
                    std::ptr::null(), *use_radius, kc, true);
                let or = run(&g_r, &bloba, *tA, std::ptr::null(), &blobb, *tB,
                    std::ptr::null(), *use_radius, kr, true);
                assert!(
                    same(&oc, &or),
                    "d1==d0 witness {i} gn{gn}: cacheIn={kc:?} -> {}",
                    describe(&oc, &or)
                );
                kc = oc.cache;
                kr = or.cache;
            }
        }
    }
}

// ===========================================================================
// Deep-iteration regressions.
//
// A 60M-call oracle search over the full input space found the maximum
// reachable value of `iter` to be **7** — the `while (iter < 20)` cap in the C
// is therefore unreachable for proxies of at most 4 vertices (the duplicate
// support-point test terminates the loop first). These are the exact
// configurations that reach `iterations` 5, 6 and 7, kept as literals so the
// deepest loop paths are exercised on every run rather than by RNG luck.
// ===========================================================================

#[test]
fn regression_deep_iteration_paths() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    // (expected iterations, tA, A bits, tB, B bits, use_radius, cache,
    //  optional ax bits (p.x, p.y, r.c, r.s), optional bx bits)
    type AxBits = Option<(u32, u32, u32, u32)>;
    #[allow(clippy::type_complexity)]
    let cases: [(c_int, u32, [u32; 5], u32, [u32; 5], c_int,
                 (u32, c_int, [c_int; 3], [c_int; 3], u32), AxBits, AxBits); 5] = [
        // iter == 7
        (
            7,
            C2_TYPE_CAPSULE, [0xdd5e0b6b, 0xc0a00000, 0x3f800000, 0xc0900000, 0x34000000],
            C2_TYPE_AABB, [0xbeaaaaab, 0x40d00000, 0x7f7fff8b, 0xc0400000, 0x00000000],
            0, (0x00000000, 0, [0, 0, 0], [0, 0, 0], 0x00000000), None, None,
        ),
        // iter == 6
        (
            6,
            C2_TYPE_AABB, [0x3eaaaaab, 0x40400000, 0xff7fffff, 0xc0000000, 0x00000000],
            C2_TYPE_AABB, [0x5d5e0b6b, 0xc0800000, 0x9e3ce508, 0xc0600000, 0x00000000],
            0, (0x00000000, 1, [0, 0, 0], [3, 3, 3], 0x3f800000), None, None,
        ),
        // iter == 5
        (
            5,
            C2_TYPE_CAPSULE, [0x3dcccccd, 0x00800000, 0x5d5e0b6b, 0xc0c00000, 0x40a00000],
            C2_TYPE_AABB, [0x3f000000, 0xc0f00000, 0xc0f00000, 0xc0400000, 0x00000000],
            1, (0x00000000, 1, [1, 1, 1], [0, 0, 0], 0x3f800000), None, None,
        ),
        // iter == 4
        (
            4,
            C2_TYPE_AABB, [0x40f00000, 0xc0600000, 0xc0c00000, 0xc0b00000, 0x00000000],
            C2_TYPE_CAPSULE, [0x40600000, 0x3dcccccd, 0x40200000, 0xbeaaaaab, 0x34000000],
            1, (0x00000000, 1, [2, 2, 2], [0, 0, 0], 0x3f800000),
            // this witness only goes deep with a non-identity transform on A
            Some((0x00800000, 0xbeaaaaab, 0x00000000, 0x3f800000)), None,
        ),
        // iter == 3
        (
            3,
            C2_TYPE_AABB, [0xc0600000, 0x00000000, 0x41000000, 0x00000000, 0x00000000],
            C2_TYPE_CAPSULE, [0x40400000, 0x461c4000, 0xff7fffff, 0x4cbebc20, 0x40b00000],
            0, (0x7f7fffff, 2, [3, 3, 0], [0, 0, 0], 0x3f800000), None, None,
        ),
    ];

    let mut deepest = -1;
    for (i, (want_it, tA, ab, tB, bb, use_radius, (mb, count, iA, iB, db), axb, bxb)) in
        cases.iter().enumerate()
    {
        let mk_x = |b: &AxBits| -> Option<c2x> {
            b.map(|(px, py, rc, rs)| c2x {
                p: c2v { x: f32::from_bits(px), y: f32::from_bits(py) },
                r: c2r { c: f32::from_bits(rc), s: f32::from_bits(rs) },
            })
        };
        let xa = mk_x(axb);
        let xb = mk_x(bxb);
        let pax: *const c2x = xa.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());
        let pbx: *const c2x = xb.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());
        let mut bloba = Blob([0u8; 32]);
        let mut blobb = Blob([0u8; 32]);
        for k in 0..5 {
            bloba.0[k * 4..k * 4 + 4].copy_from_slice(&ab[k].to_le_bytes());
            blobb.0[k * 4..k * 4 + 4].copy_from_slice(&bb[k].to_le_bytes());
        }
        for k in 0..(*count as usize).min(3) {
            assert!((iA[k] as u32) < nverts(*tA), "case {i}: iA[{k}] out of range");
            assert!((iB[k] as u32) < nverts(*tB), "case {i}: iB[{k}] out of range");
        }
        let base = c2GJKCache {
            metric: f32::from_bits(*mb),
            count: *count,
            iA: *iA,
            iB: *iB,
            div: f32::from_bits(*db),
        };
        // Also run the warm-cache continuation so the deep path is re-entered
        // with a seeded simplex. The recorded `want_it` is the deepest value
        // reached across the generations (some witnesses only go deep once the
        // cache has been written back once).
        let mut kc = base;
        let mut kr = base;
        let mut case_deepest = -1;
        for gn in 0..4 {
            unsafe {
                let oc = run(&g_c, &bloba, *tA, pax, &blobb, *tB, pbx, *use_radius, kc, true);
                let or = run(&g_r, &bloba, *tA, pax, &blobb, *tB, pbx, *use_radius, kr, true);
                assert!(
                    same(&oc, &or),
                    "deep-iteration case {i} gn{gn}: cacheIn={kc:?} -> {}",
                    describe(&oc, &or)
                );
                case_deepest = case_deepest.max(oc.it);
                deepest = deepest.max(oc.it);
                kc = oc.cache;
                kr = or.cache;
            }
        }
        assert!(
            case_deepest >= *want_it,
            "deep-iteration case {i}: the C reaches only {case_deepest} iterations, expected >= {want_it}"
        );
    }
    assert_eq!(
        deepest, 7,
        "the deepest loop path (iterations == 7) is no longer covered"
    );
}
