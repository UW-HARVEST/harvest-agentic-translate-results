//! Focused search for the highest `iterations` value `c2GJK` can report, to
//! establish whether the C's `while (iter < 20)` cap (ERRORS.md row 17) is
//! reachable at all with this library's shape set. Also a broad differential
//! check that C and Rust always agree on `iterations`.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn search_max_gjk_iterations() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0x17_17);
    let mut best = 0;
    let mut best_desc = String::new();
    let mut checked = 0usize;

    for round in 0..300_000 {
        let ta = VALID_TYPES[(round % 3) as usize];
        let tb = VALID_TYPES[((round / 3) % 3) as usize];
        let a = rand_shape(&mut g, ta);
        let b = rand_shape(&mut g, tb);
        // Mix in wild transforms and warm caches, which are the only levers a
        // caller has on the iteration count.
        let ax = if round % 3 == 0 { Some(g.xform()) } else { None };
        let bx = if round % 4 == 0 { Some(g.xform()) } else { None };
        let na = match ta {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let nb = match tb {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let mut cache = if round % 5 == 0 {
            Some(c2GJKCache {
                metric: g.coord(),
                count: 1 + g.below(3) as c_int,
                iA: [
                    g.below(na) as c_int,
                    g.below(na) as c_int,
                    g.below(na) as c_int,
                ],
                iB: [
                    g.below(nb) as c_int,
                    g.below(nb) as c_int,
                    g.below(nb) as c_int,
                ],
                div: g.coord(),
            })
        } else {
            None
        };
        let mut cache2 = cache;

        let mut oa = c2v::default();
        let mut ob = c2v::default();
        let mut it: c_int = -1;
        let mut oa2 = c2v::default();
        let mut ob2 = c2v::default();
        let mut it2: c_int = -1;
        let ur = (round % 2) as c_int;

        let cd = unsafe {
            (c.c2GJK)(
                a.as_ptr(),
                ta,
                ax.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
                b.as_ptr(),
                tb,
                bx.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
                &mut oa,
                &mut ob,
                ur,
                &mut it,
                cache
                    .as_mut()
                    .map(|x| x as *mut c2GJKCache)
                    .unwrap_or(std::ptr::null_mut()),
            )
        };
        let rd = unsafe {
            (r.c2GJK)(
                a.as_ptr(),
                ta,
                ax.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
                b.as_ptr(),
                tb,
                bx.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
                &mut oa2,
                &mut ob2,
                ur,
                &mut it2,
                cache2
                    .as_mut()
                    .map(|x| x as *mut c2GJKCache)
                    .unwrap_or(std::ptr::null_mut()),
            )
        };
        eq_f32("iter-search dist", cd, rd);
        eq_v("iter-search outA", oa, oa2);
        eq_v("iter-search outB", ob, ob2);
        eq_int("iter-search iterations", it, it2);
        if let (Some(x), Some(y)) = (cache, cache2) {
            eq_cache("iter-search cache", &x, &y);
        }
        checked += 1;
        if it > best {
            best = it;
            best_desc = format!(
                "{}/{} ur={ur} ax={} bx={} cache={}",
                type_name(ta),
                type_name(tb),
                ax.is_some(),
                bx.is_some(),
                cache.is_some()
            );
        }
    }
    println!(
        "checked {checked} c2GJK calls; max iterations = {best} ({best_desc}); \
         the C's cap is 20"
    );
    // Whatever the reachable maximum is, the two builds agreed on it every time.
    assert!(
        best <= 20,
        "iterations must never exceed the C's cap of 20, saw {best}"
    );
}
