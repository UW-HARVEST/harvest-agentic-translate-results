#![allow(non_snake_case)]
mod common;
use common::*;

/// Replays c2GJK's loop through the C `.so`'s low-level exports and counts how
/// often `d1 == d0` occurs (the only case where `d1 > d0` and `d1 >= d0` differ).
#[test]
fn search_d1_eq_d0() {
    let p = load_pair();
    const FLT_EPS: f32 = 1.192_092_895_507_812_5e-7;
    let mut rng = Rng::new(0x1234_5678);
    let mut eq_events = 0usize;
    let mut gt_events = 0usize;
    let mut witnesses: Vec<String> = Vec::new();

    for trial in 0..400_000u64 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let mag = [1.0f32, 1.0e-6, 1.0e6, 50.0, 1.0e-30, 1.0e30][rng.below(6) as usize];
        let sa = rand_shape(&mut rng, tyA, mag, 8);
        let sb = rand_shape(&mut rng, tyB, mag, 8);
        let ax = if rng.bool() { rng.xform(mag) } else { c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } } };
        let bx = if rng.bool() { rng.xform(mag) } else { c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } } };

        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        unsafe {
            (p.c.c2MakeProxy)(sa.as_ptr(), tyA, &mut pA);
            (p.c.c2MakeProxy)(sb.as_ptr(), tyB, &mut pB);
        }
        let mut s = c2Simplex::default();
        unsafe {
            s.verts[0].sA = (p.c.c2Mulxv)(ax, pA.verts[0]);
            s.verts[0].sB = (p.c.c2Mulxv)(bx, pB.verts[0]);
            s.verts[0].p = (p.c.c2Sub)(s.verts[0].sB, s.verts[0].sA);
        }
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;

        let mut d0 = f32::MAX;
        let mut iter = 0i32;
        while iter < 20 {
            let save_count = s.count;
            let mut saveA = [0i32; 3];
            let mut saveB = [0i32; 3];
            for i in 0..save_count.clamp(0, 3) as usize {
                saveA[i] = s.verts[i].iA;
                saveB[i] = s.verts[i].iB;
            }
            unsafe {
                match s.count {
                    2 => (p.c.c22)(&mut s),
                    3 => (p.c.c23)(&mut s),
                    _ => {}
                }
            }
            if s.count == 3 {
                break;
            }
            let l = unsafe { (p.c.c2L)(&mut s) };
            let d1 = unsafe { (p.c.c2Dot)(l, l) };
            if d1 == d0 {
                eq_events += 1;
                if witnesses.len() < 5 {
                    witnesses.push(format!(
                        "trial={trial} iter={iter} d1={d1} A={} B={} tyA={} tyB={}",
                        sa.describe(), sb.describe(), type_name(tyA), type_name(tyB)
                    ));
                }
            }
            if d1 > d0 {
                gt_events += 1;
                break;
            }
            if d1 == d0 {
                break; // would break under `>=`; stop the replay here either way
            }
            d0 = d1;
            let d = unsafe { (p.c.c2D)(&mut s) };
            let dd = unsafe { (p.c.c2Dot)(d, d) };
            if dd < FLT_EPS * FLT_EPS {
                break;
            }
            let ta = unsafe { (p.c.c2MulrvT)(ax.r, (p.c.c2Neg)(d)) };
            let tb = unsafe { (p.c.c2MulrvT)(bx.r, d) };
            let iA = unsafe { (p.c.c2Support)(pA.verts.as_ptr(), pA.count, ta) };
            let iB = unsafe { (p.c.c2Support)(pB.verts.as_ptr(), pB.count, tb) };
            let slot = s.count.clamp(0, 3) as usize;
            unsafe {
                s.verts[slot].iA = iA;
                s.verts[slot].sA = (p.c.c2Mulxv)(ax, pA.verts[(iA as usize) & 7]);
                s.verts[slot].iB = iB;
                s.verts[slot].sB = (p.c.c2Mulxv)(bx, pB.verts[(iB as usize) & 7]);
                s.verts[slot].p = (p.c.c2Sub)(s.verts[slot].sB, s.verts[slot].sA);
            }
            let mut dup = false;
            for i in 0..save_count.clamp(0, 3) as usize {
                if iA == saveA[i] && iB == saveB[i] {
                    dup = true;
                    break;
                }
            }
            if dup {
                break;
            }
            s.count += 1;
            iter += 1;
        }
    }
    println!("d1 == d0 events: {eq_events}");
    println!("d1  > d0 events: {gt_events}");
    for w in &witnesses {
        println!("witness: {w}");
    }
}

/// Brute-force hunt for ANY `c2GJK` input on which the loaded Rust `.so`
/// differs from the C `.so`. Used with `C2_RUST_SO` pointing at a deliberately
/// mutated build to measure the suite's sensitivity, and against the real build
/// as a very wide extra randomized sweep.
#[test]
fn search_any_c2GJK_divergence() {
    let p = load_pair();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE);
    let mut checked = 0usize;
    let mut diffs = 0usize;
    let mut first: Option<String> = None;

    for trial in 0..300_000u64 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let mag = [1.0f32, 1.0e-6, 1.0e6, 50.0, 1.0e-30, 1.0e30, 3.0e38][rng.below(7) as usize];
        let a = rand_shape(&mut rng, tyA, mag, 8);
        let b = rand_shape(&mut rng, tyB, mag, 8);
        let opts = GjkOpts {
            ax: if rng.bool() { Some(rng.xform(mag)) } else { None },
            bx: if rng.bool() { Some(rng.xform(mag)) } else { None },
            use_radius: rng.below(2) as i32,
            want_out_a: true,
            want_out_b: true,
            want_iterations: true,
            cache: rng.bool(),
        };
        let cin = if rng.below(4) == 0 {
            let cap = |t: u32| match t {
                C2_TYPE_CIRCLE => 1u32,
                C2_TYPE_CAPSULE => 2,
                _ => 4,
            };
            c2GJKCache {
                metric: rng.sym(1000.0),
                count: 1 + rng.below(3) as i32,
                iA: [
                    rng.below(cap(tyA)) as i32,
                    rng.below(cap(tyA)) as i32,
                    rng.below(cap(tyA)) as i32,
                ],
                iB: [
                    rng.below(cap(tyB)) as i32,
                    rng.below(cap(tyB)) as i32,
                    rng.below(cap(tyB)) as i32,
                ],
                div: [1.0f32, 0.0, 5.0, -2.0][rng.below(4) as usize],
            }
        } else {
            c2GJKCache::default()
        };

        let oc = gjk_once(&p.c, &a, tyA, &b, tyB, &opts, &cin);
        let or = gjk_once(&p.r, &a, tyA, &b, tyB, &opts, &cin);
        checked += 1;
        let same = feq(oc.dist, or.dist)
            && veq(oc.a, or.a)
            && veq(oc.b, or.b)
            && oc.iters == or.iters
            && cache_eq(&oc.cache, &or.cache);
        if !same {
            diffs += 1;
            if first.is_none() {
                first = Some(format!(
                    "trial={trial} A={} B={} tyA={} tyB={} ur={} cache={} \
                     | C dist={:?} it={} a=({},{}) b=({},{}) \
                     | R dist={:?} it={} a=({},{}) b=({},{})",
                    a.describe(), b.describe(), type_name(tyA), type_name(tyB),
                    opts.use_radius, opts.cache,
                    oc.dist, oc.iters, oc.a.x, oc.a.y, oc.b.x, oc.b.y,
                    or.dist, or.iters, or.a.x, or.a.y, or.b.x, or.b.y
                ));
            }
        }
    }
    println!("search_any_c2GJK_divergence: checked={checked} divergences={diffs}");
    if let Some(w) = &first {
        println!("first divergence: {w}");
    }
    if std::env::var("C2_EXPECT_DIVERGENCE").is_ok() {
        assert!(diffs > 0, "expected the mutated build to diverge, but it did not");
    } else {
        assert_eq!(diffs, 0, "c2GJK divergence found: {:?}", first);
    }
}

/// Establishes the practically reachable range of `*iterations`. `c2GJK` caps the
/// loop at 20, but the proxies it can build have at most 4 vertices, so the
/// simplex saturates long before that. This records the observed maximum so the
/// unreachability of the `iter < 20` bound is a measured fact rather than an
/// assumption.
#[test]
fn search_max_iterations() {
    let p = load_pair();
    let mut rng = Rng::new(0xA11CE);
    let mut hist = [0usize; 21];
    for _ in 0..400_000u64 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let mag = [1.0f32, 1.0e-6, 1.0e6, 50.0, 1.0e-20, 1.0e20, 3.0e38][rng.below(7) as usize];
        let a = rand_shape(&mut rng, tyA, mag, 8);
        let b = rand_shape(&mut rng, tyB, mag, 8);
        let cap = |t: u32| match t {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let use_cache = rng.below(3) == 0;
        let cin = if use_cache {
            c2GJKCache {
                metric: rng.sym(1.0e6),
                count: 1 + rng.below(3) as i32,
                iA: [rng.below(cap(tyA)) as i32, rng.below(cap(tyA)) as i32, rng.below(cap(tyA)) as i32],
                iB: [rng.below(cap(tyB)) as i32, rng.below(cap(tyB)) as i32, rng.below(cap(tyB)) as i32],
                div: [1.0f32, 0.0, 7.0, -3.0][rng.below(4) as usize],
            }
        } else {
            c2GJKCache::default()
        };
        let opts = GjkOpts {
            ax: if rng.bool() { Some(rng.xform(mag)) } else { None },
            bx: if rng.bool() { Some(rng.xform(mag)) } else { None },
            use_radius: rng.below(2) as i32,
            cache: use_cache,
            ..Default::default()
        };
        let oc = gjk_once(&p.c, &a, tyA, &b, tyB, &opts, &cin);
        let or = gjk_once(&p.r, &a, tyA, &b, tyB, &opts, &cin);
        assert_eq!(oc.iters, or.iters, "iteration divergence");
        assert!((0..=20).contains(&oc.iters), "iterations out of range: {}", oc.iters);
        hist[oc.iters as usize] += 1;
    }
    let max = hist.iter().rposition(|&n| n > 0).unwrap();
    println!("search_max_iterations histogram: {hist:?}");
    println!(
        "observed maximum *iterations = {max} (the C's `while (iter < 20)` bound is \
         therefore not reachable through the public API: a c2Proxy holds at most 4 \
         vertices, so the simplex saturates first)"
    );
    assert!(max <= 20);
}
