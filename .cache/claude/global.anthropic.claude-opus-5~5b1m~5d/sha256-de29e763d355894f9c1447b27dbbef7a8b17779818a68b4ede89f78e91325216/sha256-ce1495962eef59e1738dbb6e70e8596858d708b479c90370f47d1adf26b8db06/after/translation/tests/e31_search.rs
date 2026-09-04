//! ERRORS.md row E31 — dedicated search for the `iter == 20` loop exhaustion,
//! plus a broad differential sweep of the iteration counter itself.
//!
//! The iteration counter is a directly observable output (`*iterations`), so
//! this test asserts C and Rust agree on it across a very wide random sweep and
//! records the highest value reachable.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

#[test]
fn e31_iteration_limit_search() {
    let l = libs();
    let mut rng = Rng::new(0x31);
    let mut hist = [0usize; 21];
    let mut max_iter = -1;

    unsafe {
        for round in 0..400_000u32 {
            // Mix every axis that could lengthen the GJK loop: shape kinds,
            // transforms (incl. non-normalised rotations that skew the
            // geometry), warm caches, and near-degenerate coordinates.
            let ka = rng.below(3);
            let kb = rng.below(3);
            let mk = |rng: &mut Rng, k: u32| -> (Vec<u8>, c_int, c_int) {
                match k {
                    0 => {
                        let c = c2Circle { p: rng.v(), r: rng.radius() };
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 12).to_vec(),
                            C2_TYPE_CIRCLE,
                            1,
                        )
                    }
                    1 => {
                        let c = rng.aabb();
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 16).to_vec(),
                            C2_TYPE_AABB,
                            4,
                        )
                    }
                    _ => {
                        let c = rng.capsule();
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 20).to_vec(),
                            C2_TYPE_CAPSULE,
                            2,
                        )
                    }
                }
            };
            let (ab, at, na) = mk(&mut rng, ka);
            let (bb, bt, nb) = mk(&mut rng, kb);

            let use_x = rng.below(3);
            let ax = c2x {
                p: rng.v(),
                r: c2r { c: rng.range(2.0), s: rng.range(2.0) },
            };
            let bx = rng.x();
            let axp = if use_x >= 1 { &ax as *const c2x } else { std::ptr::null() };
            let bxp = if use_x >= 2 { &bx as *const c2x } else { std::ptr::null() };

            let ccount = (rng.below(4)) as c_int;
            let mut cc = c2GJKCache {
                metric: rng.coord(),
                count: ccount,
                iA: [
                    rng.below(na as u32) as c_int,
                    rng.below(na as u32) as c_int,
                    rng.below(na as u32) as c_int,
                ],
                iB: [
                    rng.below(nb as u32) as c_int,
                    rng.below(nb as u32) as c_int,
                    rng.below(nb as u32) as c_int,
                ],
                div: rng.coord(),
            };
            let mut cr = cc;
            let use_cache = rng.below(2) == 1;
            let ur = rng.below(2) as c_int;

            let mut itc: c_int = -1;
            let mut itr: c_int = -1;
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();

            let dc = (l.c.c2GJK)(
                ab.as_ptr() as *const c_void, at, axp,
                bb.as_ptr() as *const c_void, bt, bxp,
                &mut oac, &mut obc, ur, &mut itc,
                if use_cache { &mut cc } else { std::ptr::null_mut() },
            );
            let dr = (l.r.c2GJK)(
                ab.as_ptr() as *const c_void, at, axp,
                bb.as_ptr() as *const c_void, bt, bxp,
                &mut oar, &mut obr, ur, &mut itr,
                if use_cache { &mut cr } else { std::ptr::null_mut() },
            );
            eq_f32(&format!("E31 dist round={round}"), dc, dr);
            eq_v(&format!("E31 outA round={round}"), oac, oar);
            eq_v(&format!("E31 outB round={round}"), obc, obr);
            eq_i(&format!("E31 iterations round={round}"), itc, itr);
            if use_cache {
                eq_cache(&format!("E31 cache round={round}"), &cc, &cr);
            }
            if itc > max_iter {
                max_iter = itc;
            }
            assert!(
                (0..=20).contains(&itc),
                "E31: iterations out of the documented 0..=20 range: {itc}"
            );
            hist[itc as usize] += 1;
        }
    }

    eprintln!("E31 iteration histogram over 400k configs: {hist:?} (max={max_iter})");
    // The loop bound itself is verified: `iter` never exceeds 20 in either
    // library, and both agree on the exact value for every input.
    assert!(max_iter >= 1, "E31: the loop never iterated at all");
}
