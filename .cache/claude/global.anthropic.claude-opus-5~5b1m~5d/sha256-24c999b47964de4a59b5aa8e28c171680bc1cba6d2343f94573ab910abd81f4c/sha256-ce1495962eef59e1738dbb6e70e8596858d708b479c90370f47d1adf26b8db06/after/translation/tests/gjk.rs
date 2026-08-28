//! Phase B, Group 4 — CONFIGS.md rows C39..C65 (`c2GJK`).
//!
//! `c2GJK` is the lowest-level *composed* entry point: it drives
//! `c2MakeProxy`, the simplex reduction, `c2Support`, `c2Witness`, the radius
//! shrink and the GJK cache.  Each row below runs the full operation end to end
//! on BOTH `.so`s and compares the return value, `*outA`, `*outB`,
//! `*iterations` and the complete 36-byte cache image bit-for-bit.

mod common;
use common::*;

const N: u32 = 512;

/// Which optional out-parameters to pass.
#[derive(Copy, Clone, Debug)]
struct OutMask {
    a: bool,
    b: bool,
    iters: bool,
}

const ALL_OUT: OutMask = OutMask {
    a: true,
    b: true,
    iters: true,
};

#[derive(Clone, Debug)]
struct Result {
    dist: f32,
    a: C2v,
    b: C2v,
    iters: i32,
    cache: Option<C2GJKCache>,
}

#[allow(clippy::too_many_arguments)]
fn call_one(
    f: FnGJK,
    sa: &ShapeBlob,
    ta: u32,
    ax: Option<&C2x>,
    sb: &ShapeBlob,
    tb: u32,
    bx: Option<&C2x>,
    use_radius: i32,
    mask: OutMask,
    cache: Option<&mut C2GJKCache>,
) -> Result {
    // Poison the out-params so a missing write is detectable.
    let mut oa = C2v {
        x: f32::from_bits(0xDEAD_BEEF),
        y: f32::from_bits(0xCAFE_BABE),
    };
    let mut ob = C2v {
        x: f32::from_bits(0xFEED_FACE),
        y: f32::from_bits(0xBAAD_F00D),
    };
    let mut it: i32 = -12345;
    let cache_ptr: *mut C2GJKCache = match cache {
        Some(c) => c as *mut C2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = unsafe {
        f(
            sa.as_ptr(),
            ta,
            ax.map_or(std::ptr::null(), |v| v as *const C2x),
            sb.as_ptr(),
            tb,
            bx.map_or(std::ptr::null(), |v| v as *const C2x),
            if mask.a { &mut oa } else { std::ptr::null_mut() },
            if mask.b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if mask.iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            cache_ptr,
        )
    };
    Result {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache: if cache_ptr.is_null() {
            None
        } else {
            Some(unsafe { *cache_ptr })
        },
    }
}

#[track_caller]
fn cmp(cr: &Result, rr: &Result, ctx: &str) {
    assert!(
        f32_same(cr.dist, rr.dist),
        "c2GJK return mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_f32(cr.dist),
        fmt_f32(rr.dist)
    );
    assert!(
        v_same(cr.a, rr.a),
        "c2GJK outA mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_v(cr.a),
        fmt_v(rr.a)
    );
    assert!(
        v_same(cr.b, rr.b),
        "c2GJK outB mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_v(cr.b),
        fmt_v(rr.b)
    );
    assert_eq!(
        cr.iters, rr.iters,
        "c2GJK iterations mismatch [{ctx}]: C {} Rust {}",
        cr.iters, rr.iters
    );
    match (&cr.cache, &rr.cache) {
        (Some(a), Some(b)) => assert!(
            raw_same(a, b),
            "c2GJK cache mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
            fmt_cache(a),
            fmt_cache(b)
        ),
        (None, None) => {}
        _ => panic!("cache presence mismatch [{ctx}]"),
    }
}

/// Run one configuration on both libraries and compare everything.
#[allow(clippy::too_many_arguments)]
#[track_caller]
fn diff(
    sa: &ShapeBlob,
    ta: u32,
    ax: Option<&C2x>,
    sb: &ShapeBlob,
    tb: u32,
    bx: Option<&C2x>,
    use_radius: i32,
    mask: OutMask,
    cache0: Option<C2GJKCache>,
    ctx: &str,
) {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut cc = cache0;
    let mut rc = cache0;
    let cr = call_one(c, sa, ta, ax, sb, tb, bx, use_radius, mask, cc.as_mut());
    let rr = call_one(r, sa, ta, ax, sb, tb, bx, use_radius, mask, rc.as_mut());
    cmp(&cr, &rr, ctx);
}

/// Random shapes in one of three separation regimes.
fn regime_shapes(rng: &mut Rng, ta: u32, tb: u32, regime: u32) -> (ShapeBlob, ShapeBlob) {
    // `span` controls the shape extent; `offset` how far apart the two shapes
    // are placed.  regime 0 = well separated, 1 = touching-ish, 2 = overlapping.
    let span = 20.0;
    let offset = match regime {
        0 => rng.range(60.0, 400.0),
        1 => rng.range(18.0, 42.0),
        _ => rng.range(0.0, 10.0),
    };
    let dirx = rng.range(-1.0, 1.0);
    let diry = rng.range(-1.0, 1.0);
    let len = (dirx * dirx + diry * diry).sqrt().max(1e-6);
    let dx = dirx / len * offset;
    let dy = diry / len * offset;
    let a = shape_at(rng, ta, C2v { x: 0.0, y: 0.0 }, span);
    let b = shape_at(rng, tb, C2v { x: dx, y: dy }, span);
    (a, b)
}

fn shape_at(rng: &mut Rng, ty: u32, at: C2v, span: f32) -> ShapeBlob {
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
            p: C2v {
                x: at.x + rng.range(-span * 0.2, span * 0.2),
                y: at.y + rng.range(-span * 0.2, span * 0.2),
            },
            r: rng.range(0.0, span * 0.5),
        }),
        C2_TYPE_AABB => {
            let hx = rng.range(0.0, span * 0.5);
            let hy = rng.range(0.0, span * 0.5);
            ShapeBlob::aabb(C2AABB {
                min: C2v { x: at.x - hx, y: at.y - hy },
                max: C2v { x: at.x + hx, y: at.y + hy },
            })
        }
        _ => ShapeBlob::capsule(C2Capsule {
            a: C2v {
                x: at.x + rng.range(-span * 0.5, span * 0.5),
                y: at.y + rng.range(-span * 0.5, span * 0.5),
            },
            b: C2v {
                x: at.x + rng.range(-span * 0.5, span * 0.5),
                y: at.y + rng.range(-span * 0.5, span * 0.5),
            },
            r: rng.range(0.0, span * 0.3),
        }),
    }
}

fn cold_cache() -> C2GJKCache {
    C2GJKCache {
        metric: f32::from_bits(0xDEAD_0001),
        count: 0,
        iA: [-7, -8, -9],
        iB: [-10, -11, -12],
        div: f32::from_bits(0xDEAD_0002),
    }
}

// ---------------------------------------------------------------------------
// C39..C47 — the 3x3 shape-type grid x 3 separation regimes x use_radius
// ---------------------------------------------------------------------------

fn grid_baseline(ta: u32, tb: u32, seed: u64) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        for regime in 0..3u32 {
            let (sa, sb) = regime_shapes(&mut rng, ta, tb, regime);
            for ur in [0i32, 1] {
                diff(
                    &sa,
                    ta,
                    None,
                    &sb,
                    tb,
                    None,
                    ur,
                    ALL_OUT,
                    None,
                    &format!(
                        "{} vs {} regime={regime} ur={ur} #{i}",
                        type_name(ta),
                        type_name(tb)
                    ),
                );
            }
        }
    }
}

#[test]
fn c39_circle_circle() {
    grid_baseline(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, 0xC39);
}
#[test]
fn c40_circle_aabb() {
    grid_baseline(C2_TYPE_CIRCLE, C2_TYPE_AABB, 0xC40);
}
#[test]
fn c41_circle_capsule() {
    grid_baseline(C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, 0xC41);
}
#[test]
fn c42_aabb_circle() {
    grid_baseline(C2_TYPE_AABB, C2_TYPE_CIRCLE, 0xC42);
}
#[test]
fn c43_aabb_aabb() {
    grid_baseline(C2_TYPE_AABB, C2_TYPE_AABB, 0xC43);
}
#[test]
fn c44_aabb_capsule() {
    grid_baseline(C2_TYPE_AABB, C2_TYPE_CAPSULE, 0xC44);
}
#[test]
fn c45_capsule_circle() {
    grid_baseline(C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, 0xC45);
}
#[test]
fn c46_capsule_aabb() {
    grid_baseline(C2_TYPE_CAPSULE, C2_TYPE_AABB, 0xC46);
}
#[test]
fn c47_capsule_capsule() {
    grid_baseline(C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, 0xC47);
}

// ---------------------------------------------------------------------------
// C48..C51 — transforms
// ---------------------------------------------------------------------------

#[test]
fn c48_transform_a_only() {
    let mut rng = Rng::new(0xC48);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 4 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                let ax = rng.x_unit(300.0);
                for ur in [0i32, 1] {
                    diff(
                        &sa, ta, Some(&ax), &sb, tb, None, ur, ALL_OUT, None,
                        &format!("axA {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

#[test]
fn c49_transform_b_only() {
    let mut rng = Rng::new(0xC49);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 4 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                let bx = rng.x_unit(300.0);
                for ur in [0i32, 1] {
                    diff(
                        &sa, ta, None, &sb, tb, Some(&bx), ur, ALL_OUT, None,
                        &format!("bxB {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

#[test]
fn c50_transform_both_unit() {
    let mut rng = Rng::new(0xC50);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 4 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                let ax = rng.x_unit(300.0);
                let bx = rng.x_unit(300.0);
                for ur in [0i32, 1] {
                    diff(
                        &sa,
                        ta,
                        Some(&ax),
                        &sb,
                        tb,
                        Some(&bx),
                        ur,
                        ALL_OUT,
                        None,
                        &format!("both {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

#[test]
fn c51_transform_degenerate() {
    let mut rng = Rng::new(0xC51);
    let weird = [
        C2r { c: 0.0, s: 0.0 },
        C2r { c: 2.0, s: 3.0 },
        C2r { c: -1.0, s: 0.0 },
        C2r {
            c: f32::INFINITY,
            s: 0.0,
        },
        C2r {
            c: f32::NAN,
            s: f32::NAN,
        },
        C2r { c: 1e-45, s: 1e-45 },
        C2r { c: FLT_MAX, s: FLT_MAX },
    ];
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for &ra in &weird {
                for &rb in &weird {
                    for i in 0..4u32 {
                        let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                        let ax = C2x {
                            p: rng.v_range(-100.0, 100.0),
                            r: ra,
                        };
                        let bx = C2x {
                            p: rng.v_range(-100.0, 100.0),
                            r: rb,
                        };
                        for ur in [0i32, 1] {
                            diff(
                                &sa,
                                ta,
                                Some(&ax),
                                &sb,
                                tb,
                                Some(&bx),
                                ur,
                                ALL_OUT,
                                Some(cold_cache()),
                                &format!("degenerate rot {} {}", type_name(ta), type_name(tb)),
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C52..C55 — cache behaviour
// ---------------------------------------------------------------------------

#[test]
fn c52_cache_cold() {
    let mut rng = Rng::new(0xC52);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 2 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                for ur in [0i32, 1] {
                    diff(
                        &sa,
                        ta,
                        None,
                        &sb,
                        tb,
                        None,
                        ur,
                        ALL_OUT,
                        Some(cold_cache()),
                        &format!("cold cache {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

/// Call `c2GJK` `reps` times against the SAME cache on both libraries,
/// comparing after every call.  `perturb` optionally moves the shapes between
/// calls so the cached indices are replayed against changed geometry.
fn cache_sequence(ta: u32, tb: u32, seed: u64, reps: usize, perturb: bool) {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(seed);
    for i in 0..(N / 8) {
        let (mut sa, mut sb) = regime_shapes(&mut rng, ta, tb, i % 3);
        let mut cc = cold_cache();
        let mut rc = cold_cache();
        for k in 0..reps {
            let cr = call_one(
                c,
                &sa,
                ta,
                None,
                &sb,
                tb,
                None,
                1,
                ALL_OUT,
                Some(&mut cc),
            );
            let rr = call_one(
                r,
                &sa,
                ta,
                None,
                &sb,
                tb,
                None,
                1,
                ALL_OUT,
                Some(&mut rc),
            );
            cmp(
                &cr,
                &rr,
                &format!(
                    "cache seq {} {} call {k} #{i}",
                    type_name(ta),
                    type_name(tb)
                ),
            );
            if perturb {
                let (na, nb) = regime_shapes(&mut rng, ta, tb, (i + k as u32) % 3);
                sa = na;
                sb = nb;
            }
        }
    }
}

#[test]
fn c53_cache_reuse_twice() {
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            cache_sequence(ta, tb, 0xC530 + ta as u64 * 16 + tb as u64, 2, false);
        }
    }
}

#[test]
fn c54_cache_reuse_perturbed() {
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            cache_sequence(ta, tb, 0xC540 + ta as u64 * 16 + tb as u64, 8, true);
        }
    }
}

#[test]
fn c55_null_out_params() {
    let mut rng = Rng::new(0xC55);
    let masks = [
        OutMask {
            a: false,
            b: true,
            iters: true,
        },
        OutMask {
            a: true,
            b: false,
            iters: true,
        },
        OutMask {
            a: true,
            b: true,
            iters: false,
        },
        OutMask {
            a: false,
            b: false,
            iters: false,
        },
    ];
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 8 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                for mask in masks {
                    for cache in [None, Some(cold_cache())] {
                        diff(
                            &sa,
                            ta,
                            None,
                            &sb,
                            tb,
                            None,
                            1,
                            mask,
                            cache,
                            &format!("null-out {mask:?} {} {}", type_name(ta), type_name(tb)),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C56..C58 — the use_radius sub-branches
// ---------------------------------------------------------------------------

#[test]
fn c56_radius_midpoint_collapse() {
    let mut rng = Rng::new(0xC56);
    // Huge radii vs small separations force `dist <= rA + rB`.
    for i in 0..N {
        let sep = rng.range(0.0, 30.0);
        let a = ShapeBlob::circle(C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: rng.range(20.0, 200.0),
        });
        let b = ShapeBlob::circle(C2Circle {
            p: C2v { x: sep, y: 0.0 },
            r: rng.range(20.0, 200.0),
        });
        diff(
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            1,
            ALL_OUT,
            Some(cold_cache()),
            &format!("radius collapse #{i}"),
        );
        // capsule/aabb variants
        let cap = ShapeBlob::capsule(C2Capsule {
            a: C2v { x: -5.0, y: 0.0 },
            b: C2v { x: 5.0, y: 0.0 },
            r: rng.range(50.0, 300.0),
        });
        let bb = ShapeBlob::aabb(C2AABB {
            min: C2v { x: sep, y: -1.0 },
            max: C2v { x: sep + 2.0, y: 1.0 },
        });
        diff(
            &cap,
            C2_TYPE_CAPSULE,
            None,
            &bb,
            C2_TYPE_AABB,
            None,
            1,
            ALL_OUT,
            Some(cold_cache()),
            &format!("radius collapse cap/bb #{i}"),
        );
    }
}

#[test]
fn c57_radius_epsilon_boundary() {
    let mut rng = Rng::new(0xC57);
    // Zero-radius shapes whose separation straddles FLT_EPSILON.
    let seps = [
        0.0f32,
        1e-45,
        FLT_EPSILON * 0.5,
        FLT_EPSILON,
        FLT_EPSILON * (1.0 + f32::EPSILON),
        FLT_EPSILON * 2.0,
        1e-6,
    ];
    for &sep in &seps {
        for _ in 0..64 {
            let y = rng.range(-1.0, 1.0);
            let a = ShapeBlob::circle(C2Circle {
                p: C2v { x: 0.0, y },
                r: 0.0,
            });
            let b = ShapeBlob::circle(C2Circle {
                p: C2v { x: sep, y },
                r: 0.0,
            });
            for ur in [0i32, 1] {
                diff(
                    &a,
                    C2_TYPE_CIRCLE,
                    None,
                    &b,
                    C2_TYPE_CIRCLE,
                    None,
                    ur,
                    ALL_OUT,
                    Some(cold_cache()),
                    &format!("eps boundary sep={}", fmt_f32(sep)),
                );
            }
            // and with tiny non-zero radii so `dist > rA+rB` is also borderline
            let a2 = ShapeBlob::circle(C2Circle {
                p: C2v { x: 0.0, y },
                r: sep * 0.25,
            });
            let b2 = ShapeBlob::circle(C2Circle {
                p: C2v { x: sep * 4.0, y },
                r: sep * 0.25,
            });
            diff(
                &a2,
                C2_TYPE_CIRCLE,
                None,
                &b2,
                C2_TYPE_CIRCLE,
                None,
                1,
                ALL_OUT,
                Some(cold_cache()),
                "eps boundary tiny radii",
            );
        }
    }
}

#[test]
fn c58_use_radius_non_boolean() {
    let mut rng = Rng::new(0xC58);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 8 {
                let (sa, sb) = regime_shapes(&mut rng, ta, tb, i % 3);
                for ur in [-1i32, 2, 1000, i32::MIN, i32::MAX] {
                    diff(
                        &sa,
                        ta,
                        None,
                        &sb,
                        tb,
                        None,
                        ur,
                        ALL_OUT,
                        Some(cold_cache()),
                        &format!("use_radius={ur} {} {}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C59..C64 — degenerate / extreme geometry
// ---------------------------------------------------------------------------

fn degenerate_shape(ty: u32, which: u32, at: C2v) -> ShapeBlob {
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(match which % 3 {
            0 => C2Circle { p: at, r: 0.0 },
            1 => C2Circle { p: at, r: -5.0 },
            _ => C2Circle { p: at, r: 1e-45 },
        }),
        C2_TYPE_AABB => ShapeBlob::aabb(match which % 3 {
            0 => C2AABB { min: at, max: at },
            1 => C2AABB {
                min: C2v { x: at.x + 5.0, y: at.y + 5.0 },
                max: C2v { x: at.x - 5.0, y: at.y - 5.0 },
            },
            _ => C2AABB {
                min: at,
                max: C2v { x: at.x, y: at.y + 10.0 },
            },
        }),
        _ => ShapeBlob::capsule(match which % 3 {
            0 => C2Capsule { a: at, b: at, r: 0.0 },
            1 => C2Capsule { a: at, b: at, r: -3.0 },
            _ => C2Capsule {
                a: at,
                b: C2v { x: at.x, y: at.y },
                r: 1e-45,
            },
        }),
    }
}

#[test]
fn c59_degenerate_shapes() {
    let mut rng = Rng::new(0xC59);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for wa in 0..3u32 {
                for wb in 0..3u32 {
                    for _ in 0..16 {
                        let pa = rng.v_range(-50.0, 50.0);
                        let pb = rng.v_range(-50.0, 50.0);
                        let sa = degenerate_shape(ta, wa, pa);
                        let sb = degenerate_shape(tb, wb, pb);
                        for ur in [0i32, 1] {
                            diff(
                                &sa,
                                ta,
                                None,
                                &sb,
                                tb,
                                None,
                                ur,
                                ALL_OUT,
                                Some(cold_cache()),
                                &format!(
                                    "degenerate {}/{wa} vs {}/{wb}",
                                    type_name(ta),
                                    type_name(tb)
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn c60_coincident_shapes() {
    let mut rng = Rng::new(0xC60);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 4 {
                let at = rng.v_range(-100.0, 100.0);
                let sa = shape_at(&mut rng, ta, at, 20.0);
                // exactly the same placement (and, when the types match, the
                // same shape bytes) so the simplex collapses / duplicates
                let sb = if ta == tb {
                    sa
                } else {
                    shape_at(&mut rng, tb, at, 20.0)
                };
                for ur in [0i32, 1] {
                    diff(
                        &sa,
                        ta,
                        None,
                        &sb,
                        tb,
                        None,
                        ur,
                        ALL_OUT,
                        Some(cold_cache()),
                        &format!("coincident {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

#[test]
fn c61_huge_coordinates() {
    let mut rng = Rng::new(0xC61);
    let mags = [1e18f32, 1e30, FLT_MAX * 0.5, FLT_MAX, f32::INFINITY];
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for &m in &mags {
                for _ in 0..8 {
                    let sa = shape_at(&mut rng, ta, C2v { x: -m, y: m }, m * 0.1);
                    let sb = shape_at(&mut rng, tb, C2v { x: m, y: -m }, m * 0.1);
                    for ur in [0i32, 1] {
                        diff(
                            &sa,
                            ta,
                            None,
                            &sb,
                            tb,
                            None,
                            ur,
                            ALL_OUT,
                            Some(cold_cache()),
                            &format!("huge {} {} m={}", type_name(ta), type_name(tb), fmt_f32(m)),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn c62_nan_coordinates() {
    let mut rng = Rng::new(0xC62);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..N / 4 {
                let sa = match ta {
                    C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
                        p: rng.v_spicy(),
                        r: rng.spicy(),
                    }),
                    C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
                        min: rng.v_spicy(),
                        max: rng.v_spicy(),
                    }),
                    _ => ShapeBlob::capsule(C2Capsule {
                        a: rng.v_spicy(),
                        b: rng.v_spicy(),
                        r: rng.spicy(),
                    }),
                };
                let sb = match tb {
                    C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
                        p: rng.v_spicy(),
                        r: rng.spicy(),
                    }),
                    C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
                        min: rng.v_spicy(),
                        max: rng.v_spicy(),
                    }),
                    _ => ShapeBlob::capsule(C2Capsule {
                        a: rng.v_spicy(),
                        b: rng.v_spicy(),
                        r: rng.spicy(),
                    }),
                };
                for ur in [0i32, 1] {
                    diff(
                        &sa,
                        ta,
                        None,
                        &sb,
                        tb,
                        None,
                        ur,
                        ALL_OUT,
                        Some(cold_cache()),
                        &format!("spicy shapes {} {} #{i}", type_name(ta), type_name(tb)),
                    );
                }
            }
        }
    }
}

#[test]
fn c63_denormal_scale_shapes() {
    let mut rng = Rng::new(0xC63);
    let scales = [1e-45f32, 1e-40, FLT_MIN, 1e-30, 1e-20];
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for &s in &scales {
                for _ in 0..16 {
                    let sa = shape_at(&mut rng, ta, C2v { x: 0.0, y: 0.0 }, s);
                    let sb = shape_at(&mut rng, tb, C2v { x: s, y: s }, s);
                    for ur in [0i32, 1] {
                        diff(
                            &sa,
                            ta,
                            None,
                            &sb,
                            tb,
                            None,
                            ur,
                            ALL_OUT,
                            Some(cold_cache()),
                            &format!("tiny {} {} s={}", type_name(ta), type_name(tb), fmt_f32(s)),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn c64_many_iterations() {
    let mut rng = Rng::new(0xC64);
    // Long thin shapes at awkward angles make GJK take more steps; assert the
    // iteration counts match exactly and that high counts are actually reached.
    let mut max_iters = 0i32;
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    for i in 0..N * 4 {
        let l = rng.range(1.0, 1e4);
        let t = rng.range(1e-4, 1e-1);
        let sa = ShapeBlob::aabb(C2AABB {
            min: C2v { x: -l, y: -t },
            max: C2v { x: l, y: t },
        });
        let sb = ShapeBlob::capsule(C2Capsule {
            a: C2v {
                x: rng.range(-l, l),
                y: rng.range(-l, l),
            },
            b: C2v {
                x: rng.range(-l, l),
                y: rng.range(-l, l),
            },
            r: rng.range(0.0, t * 2.0),
        });
        let ax = rng.x_unit(l);
        let bx = rng.x_unit(l);
        let mut cc = cold_cache();
        let mut rc = cold_cache();
        let cr = call_one(
            c,
            &sa,
            C2_TYPE_AABB,
            Some(&ax),
            &sb,
            C2_TYPE_CAPSULE,
            Some(&bx),
            1,
            ALL_OUT,
            Some(&mut cc),
        );
        let rr = call_one(
            r,
            &sa,
            C2_TYPE_AABB,
            Some(&ax),
            &sb,
            C2_TYPE_CAPSULE,
            Some(&bx),
            1,
            ALL_OUT,
            Some(&mut rc),
        );
        cmp(&cr, &rr, &format!("many-iter #{i}"));
        max_iters = max_iters.max(cr.iters);
    }
    assert!(max_iters >= 2, "expected multi-iteration GJK runs, got {max_iters}");
}

// ---------------------------------------------------------------------------
// C65 — full randomized cross-product sweep
// ---------------------------------------------------------------------------

#[test]
fn c65_full_random_sweep() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xC65);
    for i in 0..4096u32 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let regime = rng.below(3);
        let (sa, sb) = regime_shapes(&mut rng, ta, tb, regime);
        let ax = if rng.bool() { Some(rng.x_spicy()) } else { None };
        let bx = if rng.bool() {
            Some(rng.x_unit(200.0))
        } else {
            None
        };
        let ur = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => -1,
            _ => rng.next_u32() as i32,
        };
        let mask = OutMask {
            a: rng.bool(),
            b: rng.bool(),
            iters: rng.bool(),
        };
        let use_cache = rng.bool();
        let reps = 1 + rng.below(3) as usize;

        let mut cc = if use_cache { Some(cold_cache()) } else { None };
        let mut rc = cc;
        for k in 0..reps {
            let cr = call_one(
                c,
                &sa,
                ta,
                ax.as_ref(),
                &sb,
                tb,
                bx.as_ref(),
                ur,
                mask,
                cc.as_mut(),
            );
            let rr = call_one(
                r,
                &sa,
                ta,
                ax.as_ref(),
                &sb,
                tb,
                bx.as_ref(),
                ur,
                mask,
                rc.as_mut(),
            );
            cmp(
                &cr,
                &rr,
                &format!(
                    "random sweep #{i} rep {k} ta={} tb={} ur={ur} mask={mask:?} cache={use_cache}",
                    type_name(ta),
                    type_name(tb)
                ),
            );
        }
    }
}
