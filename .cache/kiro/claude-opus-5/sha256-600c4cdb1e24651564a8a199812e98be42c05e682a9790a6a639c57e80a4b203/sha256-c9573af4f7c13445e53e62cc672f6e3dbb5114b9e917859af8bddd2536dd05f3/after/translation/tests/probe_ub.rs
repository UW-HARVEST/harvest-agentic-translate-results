//! Probe: does an out-of-vertex-count cache index actually diverge?
//!
//! `ERRORS.md` row 26 / row 36. In C, `c2GJK` declares `c2Proxy pA;` on the
//! stack and `c2MakeProxy` only writes `verts[0..count)`, so a cache index
//! beyond the shape's real vertex count reads UNINITIALISED memory. This probe
//! measures whether the divergence is real, so the decision is data-driven
//! rather than assumed. Run with `--ignored --nocapture`.

mod common;

use common::*;

#[test]
#[ignore = "diagnostic probe: reads uninitialised C stack (UB), run explicitly"]
fn probe_cache_index_beyond_vertex_count() {
    let p = pair();
    let mut diverged = 0u32;
    let mut total = 0u32;
    let mut rng = Rng::new(26);
    for ty in TYPES {
        let real = match ty {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_AABB => 4,
            _ => 2,
        };
        for idx in real..8i32 {
            for _ in 0..16 {
                let sa = gen_shape(&mut rng, ty, Class::Near, false);
                let sb = gen_shape(&mut rng, ty, Class::Near, true);
                let cache = c2GJKCache {
                    metric: 0.0,
                    count: 1,
                    iA: [idx, 0, 0],
                    iB: [0, 0, 0],
                    div: 1.0,
                };
                let oc = run_gjk(p.c, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
                let or = run_gjk(p.rs, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
                total += 1;
                if oc.bits() != or.bits() {
                    diverged += 1;
                    if diverged <= 3 {
                        eprintln!("  ty={} idx={idx}\n    C   ={oc:?}\n    Rust={or:?}", type_name(ty));
                    }
                }
            }
        }
    }
    eprintln!("cache-index-beyond-count: {diverged}/{total} diverged");
}
