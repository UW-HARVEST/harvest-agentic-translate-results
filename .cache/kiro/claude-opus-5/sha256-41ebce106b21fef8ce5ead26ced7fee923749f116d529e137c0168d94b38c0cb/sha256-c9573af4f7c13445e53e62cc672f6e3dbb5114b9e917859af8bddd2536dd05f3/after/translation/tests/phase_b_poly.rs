//! Phase B — CONFIGS.md rows 44..62: `c2RaytoPoly`, called directly with
//! every `bx` transform state and every vertex count, including counts past
//! the declared array capacity.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 2500;

fn rand_ray(rng: &mut Rng) -> c2Ray {
    c2Ray {
        p: rng.v_small(),
        d: if rng.below(4) == 0 { AXIS_DIRS[rng.below(4)] } else { rng.v_dir() },
        t: rng.range(0.0, 40.0),
    }
}

fn rand_bx(rng: &mut Rng, kind: usize) -> c2x {
    match kind {
        0 => c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        },
        1 => c2x {
            p: rng.v_small(),
            r: c2r { c: 1.0, s: 0.0 },
        },
        2 => c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: rng.rot_unit(),
        },
        3 => c2x {
            p: rng.v_small(),
            r: rng.rot_unit(),
        },
        4 => c2x {
            p: rng.v_small(),
            r: c2r { c: 0.0, s: 0.0 },
        },
        _ => c2x {
            p: rng.v_small(),
            r: c2r {
                c: rng.sym(3.0),
                s: rng.sym(3.0),
            },
        },
    }
}

/// Row 44: `bx = NULL`, random convex polygons of every count 3..8.
#[test]
fn row44_poly_null_bx() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x44);
    unsafe {
        for _ in 0..(N * 8) {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let A = rand_ray(&mut rng);
            d.ray(
                "c2RaytoPoly(null bx)",
                call_poly(&p.c, A, &poly, None),
                call_poly(&p.rs, A, &poly, None),
            );
        }
    }
    d.finish("row 44: c2RaytoPoly bx = NULL");
}

/// Row 45: an explicit identity `c2x` must give a bit-identical result to
/// `NULL` (the C substitutes `c2xIdentity()`), in BOTH implementations.
#[test]
fn row45_poly_explicit_identity() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x45);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    unsafe {
        for _ in 0..(N * 4) {
            let count = 1 + rng.below(8);
            let poly = make_convex_poly(&mut rng, count);
            let A = rand_ray(&mut rng);
            let c_id = call_poly(&p.c, A, &poly, Some(&ident));
            let r_id = call_poly(&p.rs, A, &poly, Some(&ident));
            d.ray("c2RaytoPoly(identity)", c_id, r_id);
            // NULL and explicit identity must agree inside each implementation
            let c_null = call_poly(&p.c, A, &poly, None);
            let r_null = call_poly(&p.rs, A, &poly, None);
            d.ray("c2RaytoPoly(C: null == identity)", c_null, c_id);
            d.ray("c2RaytoPoly(RS: null == identity)", r_null, r_id);
        }
    }
    d.finish("row 45: c2RaytoPoly explicit identity == NULL");
}

/// Rows 46-49: every `bx` shape — translation only, rotation only,
/// rotation+translation, and non-unit / zero rotations.
#[test]
fn row46_to_row49_poly_transforms() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x4649);
    let labels = [
        "identity",
        "translation-only",
        "rotation-only",
        "rot+trans",
        "zero-rot",
        "non-unit-rot",
    ];
    unsafe {
        for kind in 0..6 {
            for _ in 0..(N * 3) {
                let count = 1 + rng.below(8);
                let poly = make_convex_poly(&mut rng, count);
                let bx = rand_bx(&mut rng, kind);
                let A = rand_ray(&mut rng);
                d.ray(
                    &format!("c2RaytoPoly(bx={})", labels[kind]),
                    call_poly(&p.c, A, &poly, Some(&bx)),
                    call_poly(&p.rs, A, &poly, Some(&bx)),
                );
            }
        }
        // non-finite transforms
        for _ in 0..(N * 2) {
            let poly = make_convex_poly(&mut rng, 4);
            let bx = c2x {
                p: rng.v_mixed(),
                r: c2r { c: rng.f_mixed(), s: rng.f_mixed() },
            };
            let A = rand_ray(&mut rng);
            d.ray(
                "c2RaytoPoly(bx=non-finite)",
                call_poly(&p.c, A, &poly, Some(&bx)),
                call_poly(&p.rs, A, &poly, Some(&bx)),
            );
        }
    }
    d.finish("rows 46-49: c2RaytoPoly bx transforms");
}

/// Rows 50-55: every vertex count 1..8, each with all `bx` kinds.
#[test]
fn row50_to_row55_poly_counts() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5055);
    unsafe {
        for count in 1..=8usize {
            for _ in 0..(N * 2) {
                let poly = make_convex_poly(&mut rng, count);
                let A = rand_ray(&mut rng);
                d.ray(
                    &format!("c2RaytoPoly(count={count},bx=NULL)"),
                    call_poly(&p.c, A, &poly, None),
                    call_poly(&p.rs, A, &poly, None),
                );
                let bx = { let k = rng.below(6); rand_bx(&mut rng, k) };
                d.ray(
                    &format!("c2RaytoPoly(count={count},bx=rand)"),
                    call_poly(&p.c, A, &poly, Some(&bx)),
                    call_poly(&p.rs, A, &poly, Some(&bx)),
                );
            }
        }
    }
    d.finish("rows 50-55: c2RaytoPoly counts 1..8");
}

/// Row 56: `count > 8`. The C loops past the declared `verts`/`norms` arrays,
/// so BOTH implementations are handed the same oversized backing buffer and
/// must read the same trailing bytes and produce the same answer.
#[test]
fn row56_poly_count_over_capacity() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x56);

    // 4 KiB of deterministic, fully-initialised backing store. `c2Poly` is
    // 132 bytes, so counts up to ~120 stay inside the allocation.
    const WORDS: usize = 1024;
    let mut backing = vec![0f32; WORDS];
    unsafe {
        for count in [9i32, 10, 12, 16, 24, 32, 64] {
            for _ in 0..600 {
                for w in backing.iter_mut() {
                    *w = rng.sym(8.0);
                }
                // lay out a c2Poly header at the start of the buffer
                let base = backing.as_mut_ptr() as *mut u8;
                std::ptr::write(base as *mut c_int, count);
                // fill the declared arrays with a sane convex quad so the
                // early iterations behave like a real polygon
                let poly_ptr = base as *mut c2Poly;
                let quad = make_axis_quad(&mut rng);
                for i in 0..8 {
                    (*poly_ptr).verts[i] = quad.verts[i % 4];
                    (*poly_ptr).norms[i] = quad.norms[i % 4];
                }
                (*poly_ptr).count = count;

                let A = rand_ray(&mut rng);
                let cb: *const c2Poly = base as *const c2Poly;
                for bx in [None, Some({ let k = rng.below(6); rand_bx(&mut rng, k) })] {
                    d.ray(
                        &format!("c2RaytoPoly(count={count})"),
                        call_poly_raw(&p.c, A, cb, bx.as_ref()),
                        call_poly_raw(&p.rs, A, cb, bx.as_ref()),
                    );
                }
            }
        }
    }
    d.finish("row 56: c2RaytoPoly count > 8");
}

/// Row 57: ray origin inside the polygon — every `den < 0` edge has
/// `num > 0`, so `lo` stays 0 and `index` usually stays `~0`.
#[test]
fn row57_poly_origin_inside() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x57);
    unsafe {
        for _ in 0..(N * 6) {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let c = poly_centroid(&poly);
            // centroid, and points pulled towards a random vertex
            let vi = rng.below(count);
            let k = rng.range(0.0, 0.95);
            let origin = c2v {
                x: c.x + (poly.verts[vi].x - c.x) * k,
                y: c.y + (poly.verts[vi].y - c.y) * k,
            };
            let A = c2Ray {
                p: origin,
                d: if rng.below(3) == 0 { AXIS_DIRS[rng.below(4)] } else { rng.v_dir() },
                t: rng.range(0.0, 40.0),
            };
            for bx in [None, Some({ let k = rng.below(6); rand_bx(&mut rng, k) })] {
                // note: `origin` is in the polygon's LOCAL space, so when bx is
                // non-identity the world-space ray is transformed accordingly
                d.ray(
                    "c2RaytoPoly(inside)",
                    call_poly(&p.c, A, &poly, bx.as_ref()),
                    call_poly(&p.rs, A, &poly, bx.as_ref()),
                );
            }
        }
    }
    d.finish("row 57: c2RaytoPoly origin inside");
}

/// Rows 58-60: `den == 0` cases. Axis-aligned quads with axis-aligned rays
/// make `c2Dot(norms[i], d)` exactly zero for two of the four edges every
/// time, exercising both the `num < 0` reject and the `num >= 0` pass-through.
#[test]
fn row58_to_row60_poly_parallel_edges() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5860);
    unsafe {
        for _ in 0..(N * 4) {
            let poly = make_axis_quad(&mut rng);
            let c = poly_centroid(&poly);
            let hw = poly.verts[0].x - c.x;
            let hh = poly.verts[1].y - c.y;
            for dir in AXIS_DIRS {
                // offsets: inside the slab, exactly on the edge plane, outside
                for off in [0.0f32, 0.5, 1.0, 1.000_001, -1.0, -1.000_001, 2.0] {
                    let origin = if dir.x != 0.0 {
                        c2v { x: c.x - dir.x * 30.0, y: c.y + off * hh }
                    } else {
                        c2v { x: c.x + off * hw, y: c.y - dir.y * 30.0 }
                    };
                    for t in [0.0f32, 15.0, 30.0, 30.0 + hw.abs(), 60.0] {
                        let A = c2Ray { p: origin, d: dir, t };
                        for bx in [None, Some(rand_bx(&mut rng, 0))] {
                            d.ray(
                                "c2RaytoPoly(parallel)",
                                call_poly(&p.c, A, &poly, bx.as_ref()),
                                call_poly(&p.rs, A, &poly, bx.as_ref()),
                            );
                        }
                    }
                }
            }
        }
    }
    d.finish("rows 58-60: c2RaytoPoly den == 0 / axis-aligned");
}

/// Row 59 (explicit): ray origin exactly on a vertex or on an edge plane, so
/// `num == 0`.
#[test]
fn row59_poly_origin_on_boundary() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x59);
    unsafe {
        for _ in 0..(N * 4) {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let c = poly_centroid(&poly);
            for i in 0..count {
                let j = (i + 1) % count;
                let pts = [
                    poly.verts[i],
                    c2v {
                        x: (poly.verts[i].x + poly.verts[j].x) * 0.5,
                        y: (poly.verts[i].y + poly.verts[j].y) * 0.5,
                    },
                ];
                for origin in pts {
                    // inward, outward and tangential directions
                    let inward = c2v { x: c.x - origin.x, y: c.y - origin.y };
                    let dirs = [
                        inward,
                        c2v { x: -inward.x, y: -inward.y },
                        poly.norms[i],
                        c2v { x: -poly.norms[i].x, y: -poly.norms[i].y },
                        c2v { x: -poly.norms[i].y, y: poly.norms[i].x },
                        rng.v_dir(),
                    ];
                    for dd in dirs {
                        let A = c2Ray { p: origin, d: dd, t: rng.range(0.0, 40.0) };
                        d.ray(
                            "c2RaytoPoly(on-boundary)",
                            call_poly(&p.c, A, &poly, None),
                            call_poly(&p.rs, A, &poly, None),
                        );
                    }
                }
            }
        }
    }
    d.finish("row 59: c2RaytoPoly origin on vertex / edge");
}

/// Row 61: `A.t` pinned to the exact hit distance the C reports, plus one ULP
/// either side — the boundary of the `num < hi * den` clip.
#[test]
fn row61_poly_t_boundary() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x61);
    unsafe {
        for _ in 0..(N * 4) {
            let count = 3 + rng.below(6);
            let poly = make_convex_poly(&mut rng, count);
            let c = poly_centroid(&poly);
            let dir = rng.v_dir();
            let start = 30.0f32;
            let A0 = c2Ray {
                p: c2v { x: c.x - dir.x * start, y: c.y - dir.y * start },
                d: dir,
                t: 1.0e6,
            };
            let (hit, out) = call_poly(&p.c, A0, &poly, None);
            let ts: Vec<f32> = if hit != 0 {
                vec![out.t, ulp_down(out.t), ulp_up(out.t), out.t * 0.999_9, out.t * 1.000_1, 0.0]
            } else {
                vec![0.0, start, 1.0e6]
            };
            for t in ts {
                let A = c2Ray { p: A0.p, d: A0.d, t };
                d.ray(
                    "c2RaytoPoly(t-bound)",
                    call_poly(&p.c, A, &poly, None),
                    call_poly(&p.rs, A, &poly, None),
                );
            }
        }
    }
    d.finish("row 61: c2RaytoPoly A.t boundary");
}

/// Row 62: the C never validates convexity, winding or normal length, so
/// arbitrary garbage vert/norm data is a legal input.
#[test]
fn row62_poly_garbage() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x62);
    unsafe {
        for _ in 0..(N * 6) {
            let count = 1 + rng.below(8);
            let mut poly = c2Poly::default();
            poly.count = count as c_int;
            for i in 0..8 {
                poly.verts[i] = if rng.below(6) == 0 { rng.v_mixed() } else { rng.v_small() };
                poly.norms[i] = if rng.below(6) == 0 { rng.v_mixed() } else { rng.v_dir() };
            }
            let A = c2Ray {
                p: if rng.below(6) == 0 { rng.v_mixed() } else { rng.v_small() },
                d: if rng.below(6) == 0 { rng.v_mixed() } else { rng.v_dir() },
                t: if rng.below(6) == 0 { rng.f_mixed() } else { rng.range(-5.0, 40.0) },
            };
            for bx in [None, Some({ let k = rng.below(6); rand_bx(&mut rng, k) })] {
                d.ray(
                    "c2RaytoPoly(garbage)",
                    call_poly(&p.c, A, &poly, bx.as_ref()),
                    call_poly(&p.rs, A, &poly, bx.as_ref()),
                );
            }
        }
        // reversed winding (inward normals) — legal input, different branch mix
        for _ in 0..(N * 2) {
            let count = 3 + rng.below(6);
            let mut poly = make_convex_poly(&mut rng, count);
            for i in 0..8 {
                poly.norms[i] = c2v { x: -poly.norms[i].x, y: -poly.norms[i].y };
            }
            let A = rand_ray(&mut rng);
            d.ray(
                "c2RaytoPoly(inward-normals)",
                call_poly(&p.c, A, &poly, None),
                call_poly(&p.rs, A, &poly, None),
            );
        }
        // zero-length normals -> den == 0 && num == 0 on every edge
        for _ in 0..(N * 2) {
            let count = 3 + rng.below(6);
            let mut poly = make_convex_poly(&mut rng, count);
            for i in 0..8 {
                poly.norms[i] = c2v { x: 0.0, y: 0.0 };
            }
            let A = rand_ray(&mut rng);
            d.ray(
                "c2RaytoPoly(zero-normals)",
                call_poly(&p.c, A, &poly, None),
                call_poly(&p.rs, A, &poly, None),
            );
        }
    }
    d.finish("row 62: c2RaytoPoly unvalidated / garbage data");
}

// ---------------------------------------------------------------------------
// Branch-coverage guard for c2RaytoPoly.
// ---------------------------------------------------------------------------

/// Exit ids:
/// 0 = `den == 0 && num < 0` early reject   1 = `hi < lo` reject
/// 2 = hit (`index != ~0`)                  3 = fell through with `index == ~0`
fn classify_poly(poly: &c2Poly, A: &c2Ray) -> usize {
    let mut lo = 0.0f32;
    let mut hi = A.t;
    let mut index: i32 = !0;
    let n = poly.count;
    let mut i: i32 = 0;
    while i < n {
        let idx = i as usize;
        if idx >= 8 {
            return 9; // out of declared capacity; not classified
        }
        let ni = poly.norms[idx];
        let vi = poly.verts[idx];
        let sx = vi.x - A.p.x;
        let sy = vi.y - A.p.y;
        let num = ni.x * sx + ni.y * sy;
        let den = ni.x * A.d.x + ni.y * A.d.y;
        if den == 0.0 && num < 0.0 {
            return 0;
        }
        if den < 0.0 && num < lo * den {
            lo = num / den;
            index = i;
        } else if den > 0.0 && num < hi * den {
            hi = num / den;
        }
        if hi < lo {
            return 1;
        }
        i += 1;
    }
    if index != !0 {
        2
    } else {
        3
    }
}

#[test]
fn poly_branch_coverage() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x9999_0F01);
    let mut seen = [0usize; 4];
    unsafe {
        for _ in 0..40_000 {
            let count = 1 + rng.below(8);
            let poly = if rng.bool() {
                make_convex_poly(&mut rng, count)
            } else {
                make_axis_quad(&mut rng)
            };
            let mode = rng.below(4);
            let c = poly_centroid(&poly);
            let A = match mode {
                0 => rand_ray(&mut rng),
                1 => c2Ray { p: c, d: rng.v_dir(), t: rng.range(0.0, 40.0) },
                2 => {
                    let dir = rng.v_dir();
                    c2Ray {
                        p: c2v { x: c.x - dir.x * 30.0, y: c.y - dir.y * 30.0 },
                        d: dir,
                        t: rng.range(0.0, 60.0),
                    }
                }
                _ => c2Ray {
                    p: c2v { x: c.x - 30.0, y: c.y + rng.sym(12.0) },
                    d: AXIS_DIRS[rng.below(4)],
                    t: rng.range(0.0, 60.0),
                },
            };
            let k = classify_poly(&poly, &A);
            if k < 4 {
                seen[k] += 1;
            }
            d.ray(
                "c2RaytoPoly(cov)",
                call_poly(&p.c, A, &poly, None),
                call_poly(&p.rs, A, &poly, None),
            );
        }
    }
    eprintln!("poly exit histogram = {seen:?}");
    for (i, &n) in seen.iter().enumerate() {
        assert!(n > 0, "c2RaytoPoly exit {i} never reached; coverage is vacuous");
    }
    d.finish("c2RaytoPoly branch coverage (all 4 exits)");
}

fn ulp_up(x: f32) -> f32 {
    if !x.is_finite() {
        return x;
    }
    if x >= 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn ulp_down(x: f32) -> f32 {
    if !x.is_finite() {
        return x;
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() - 1)
    } else {
        f32::from_bits(x.to_bits() + 1)
    }
}
