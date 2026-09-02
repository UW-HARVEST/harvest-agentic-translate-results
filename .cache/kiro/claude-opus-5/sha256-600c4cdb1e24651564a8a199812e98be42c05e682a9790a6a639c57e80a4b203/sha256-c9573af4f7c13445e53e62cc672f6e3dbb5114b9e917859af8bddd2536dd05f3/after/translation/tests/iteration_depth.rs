//! Exploratory: how deep can `c2GJK`'s iteration loop actually go?
//!
//! `ERRORS.md` row 27 / `CONFIGS.md` row 88 concern the `while (iter < 20)`
//! cap. This test brute-forces a very large random space (shapes, transforms,
//! warm-start caches) and records the histogram of `*iterations`, so the claim
//! about the reachable range is measured rather than assumed. It also asserts
//! C and Rust agree on the count for every sample.

mod common;

use common::*;

#[test]
fn iteration_depth_histogram() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE);
    let mut hist = std::collections::BTreeMap::<i32, u64>::new();

    for _ in 0..60_000 {
        let tya = TYPES[rng.below(3) as usize];
        let tyb = TYPES[rng.below(3) as usize];
        let class = ALL_CLASSES[rng.below(ALL_CLASSES.len() as u32) as usize];
        let sa = gen_shape(&mut rng, tya, class, false);
        let sb = gen_shape(&mut rng, tyb, class, true);
        let ax = match rng.below(4) {
            0 => None,
            1 => Some(rng.xform_translation()),
            2 => Some(rng.xform_full()),
            _ => Some(rng.xform_unnormalised()),
        };
        let bx = match rng.below(4) {
            0 => None,
            1 => Some(rng.xform_translation()),
            2 => Some(rng.xform_full()),
            _ => Some(rng.xform_unnormalised()),
        };
        let ur = (rng.below(2)) as i32;

        // Warm-start caches with every legal index combination.
        let cache = match rng.below(3) {
            0 => None,
            1 => Some(c2GJKCache::default()),
            _ => {
                let count = 1 + rng.below(3) as i32;
                let na = match tya { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                let nb = match tyb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                let mut c = c2GJKCache { count, div: 1.0, metric: rng.coord(), ..Default::default() };
                for k in 0..count as usize {
                    c.iA[k] = rng.below(na) as i32;
                    c.iB[k] = rng.below(nb) as i32;
                }
                Some(c)
            }
        };

        let oc = run_gjk(p.c, &sa, ax, &sb, bx, ur, OutSel::ALL, cache);
        let or = run_gjk(p.rs, &sa, ax, &sb, bx, ur, OutSel::ALL, cache);
        same("iteration depth sweep", oc.clone(), or);
        *hist.entry(oc.iters.unwrap()).or_default() += 1;
    }

    eprintln!("c2GJK iteration histogram: {hist:?}");
    // The reduction always terminates via `hit`, the `dup` guard or the
    // `d1 > d0` guard long before 20; assert only that the values are inside
    // the cap the C code enforces.
    for (&k, _) in &hist {
        assert!((0..=20).contains(&k), "iteration count {k} outside [0,20]");
    }
    assert!(hist.len() >= 3, "iteration depth not varied: {hist:?}");
}
