//! High-volume randomized differential stress over the whole public surface.
//!
//! Complements the per-row tests: the CONFIGS.md rows pin down each documented
//! configuration, this file throws a large randomized cross-product of shapes,
//! transforms, options and cache states at `c2GJK` / `c2Collided` and compares
//! every observable output, plus the full `iterations` histogram.

#![allow(non_snake_case)]

mod common;

use common::*;

/// Number of vertices `c2MakeProxy` initialises for this shape kind. A cached
/// support index must stay below it: `c2GJK` builds its `c2Proxy` on an
/// UNINITIALISED stack slot, so `pA.verts[i]` for `i >= count` reads garbage in
/// C and is not a specifiable behaviour.
fn proxy_vert_count(s: &Shape) -> u32 {
    match s {
        Shape::Circle(_) => 1,
        Shape::Aabb(_) => 4,
        Shape::Capsule(_) => 2,
    }
}

fn wild_shape(rng: &mut Rng, t: C2_TYPE) -> Shape {
    match t {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.wild_vec(), r: rng.wild() }),
        C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.wild_vec(), max: rng.wild_vec() }),
        _ => Shape::Capsule(c2Capsule { a: rng.wild_vec(), b: rng.wild_vec(), r: rng.wild() }),
    }
}

#[test]
fn stress_gjk_cross_product_and_iteration_histogram() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    let mut hist_c = [0u64; 32];
    let mut hist_r = [0u64; 32];
    let mut hit_zero_dist = 0u64;
    let mut nonzero_dist = 0u64;

    for _ in 0..250_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let wild = rng.below(4) == 0;
        let a = if wild { wild_shape(&mut rng, ta) } else { rand_shape(&mut rng, ta) };
        let b = if wild { wild_shape(&mut rng, tb) } else { rand_shape(&mut rng, tb) };
        let o = GjkOpts {
            ax: if rng.below(2) == 0 { None } else { Some(rng.xform()) },
            bx: if rng.below(2) == 0 { None } else { Some(rng.xform()) },
            use_radius: rng.below(2) as i32,
            want_a: rng.below(4) != 0,
            want_b: rng.below(4) != 0,
            want_iters: true,
            cache: match rng.below(3) {
                0 => None,
                1 => Some(c2GJKCache::default()),
                _ => {
                    let na = proxy_vert_count(&a);
                    let nb = proxy_vert_count(&b);
                    Some(c2GJKCache {
                        metric: rng.range(-2.0e8, 2.0e8),
                        count: rng.below(4) as i32,
                        // Indices must stay below the proxy's initialised
                        // vertex count (see `proxy_vert_count`).
                        iA: [
                            rng.below(na) as i32,
                            rng.below(na) as i32,
                            rng.below(na) as i32,
                        ],
                        iB: [
                            rng.below(nb) as i32,
                            rng.below(nb) as i32,
                            rng.below(nb) as i32,
                        ],
                        div: rng.range(0.1, 8.0),
                    })
                }
            },
        };
        let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
        same(
            "stress c2GJK",
            &format!("A={} B={} [{}]", a.show(), b.show(), o.show()),
            rc,
            rr,
        );
        let ic = rc.iters.unwrap();
        let ir = rr.iters.unwrap();
        if (0..32).contains(&ic) {
            hist_c[ic as usize] += 1;
        }
        if (0..32).contains(&ir) {
            hist_r[ir as usize] += 1;
        }
        if rc.dist == 0.0 {
            hit_zero_dist += 1;
        } else {
            nonzero_dist += 1;
        }
    }

    assert_eq!(
        hist_c, hist_r,
        "the C and Rust `iterations` histograms diverge:\n  C   : {hist_c:?}\n  Rust: {hist_r:?}"
    );
    // The suite must be exercising both the collapse-to-zero and the true
    // separation branches.
    assert!(hit_zero_dist > 1000, "only {hit_zero_dist} zero-distance results");
    assert!(nonzero_dist > 1000, "only {nonzero_dist} non-zero-distance results");
    // Documented invariant: the loop cap is 20; record what is actually reachable.
    assert!(
        hist_c[6..].iter().all(|&n| n == 0),
        "iteration counts above 5 became reachable; extend the ERRORS.md row-15 \
         analysis: {hist_c:?}"
    );
}

#[test]
fn stress_collided_cross_product() {
    let (cf, rf) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 0xCAFE);
    let mut trues = 0u64;
    let mut falses = 0u64;
    for _ in 0..250_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let wild = rng.below(5) == 0;
        let a = if wild { wild_shape(&mut rng, ta) } else { rand_shape(&mut rng, ta) };
        let b = if wild { wild_shape(&mut rng, tb) } else { rand_shape(&mut rng, tb) };
        let (cv, rv) = unsafe {
            (
                cf(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()),
                rf(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()),
            )
        };
        same(
            "stress c2Collided",
            &format!("A={} ({}) B={} ({})", a.show(), ta, b.show(), tb),
            cv,
            rv,
        );
        if cv != 0 {
            trues += 1
        } else {
            falses += 1
        }
    }
    assert!(trues > 1000 && falses > 1000, "one-sided results: {trues}/{falses}");
}

#[test]
fn stress_simplex_solvers() {
    // Randomized c22/c23/c2D/c2L/c2Witness/c2GJKSimplexMetric over the same
    // simplex, comparing the full mutated struct.
    let (c22c, c22r) = sym::<FnSimplexVoid>("c22");
    let (c23c, c23r) = sym::<FnSimplexVoid>("c23");
    let (cdc, cdr) = sym::<FnSimplexV>("c2D");
    let (clc, clr) = sym::<FnSimplexV>("c2L");
    let (cmc, cmr) = sym::<FnSimplexF>("c2GJKSimplexMetric");
    let (cwc, cwr) = sym::<FnWitness>("c2Witness");
    let mut rng = Rng::new(SEED ^ 0xF00D);
    for _ in 0..200_000 {
        let count = rng.below(5) as i32 - 1; // -1 .. 3
        let mut s = rng.simplex(count);
        // occasionally use non-finite simplex points
        if rng.below(6) == 0 {
            for i in 0..4 {
                s.verts[i].p = rng.wild_vec();
                s.verts[i].sA = rng.wild_vec();
                s.verts[i].sB = rng.wild_vec();
                s.verts[i].u = rng.wild();
            }
            s.div = rng.wild();
        }
        let desc = format!("count={count} {}", s.show());
        unsafe {
            let (mut x, mut y) = (s, s);
            c22c(&mut x);
            c22r(&mut y);
            same("stress c22", &desc, x, y);
            // chain: run c23 on the c22 result
            let (mut x2, mut y2) = (x, y);
            c23c(&mut x2);
            c23r(&mut y2);
            same("stress c22->c23", &desc, x2, y2);
            let (mut a, mut b) = (x2, y2);
            same("stress c2D", &desc, cdc(&mut a), cdr(&mut b));
            let (mut a, mut b) = (x2, y2);
            same("stress c2L", &desc, clc(&mut a), clr(&mut b));
            let (mut a, mut b) = (x2, y2);
            same("stress c2GJKSimplexMetric", &desc, cmc(&mut a), cmr(&mut b));
            let (mut a, mut b) = (x2, y2);
            let mut wa1 = c2v::default();
            let mut wb1 = c2v::default();
            let mut wa2 = c2v::default();
            let mut wb2 = c2v::default();
            cwc(&mut a, &mut wa1, &mut wb1);
            cwr(&mut b, &mut wa2, &mut wb2);
            same("stress c2Witness", &desc, (wa1, wb1), (wa2, wb2));
        }
    }
}
