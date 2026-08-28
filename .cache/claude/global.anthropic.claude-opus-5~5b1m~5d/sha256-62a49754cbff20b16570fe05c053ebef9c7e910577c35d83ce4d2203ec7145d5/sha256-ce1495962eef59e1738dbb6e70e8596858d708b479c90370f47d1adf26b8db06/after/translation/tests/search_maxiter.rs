//! Diagnostic (not an assertion test): how many GJK iterations are reachable?
//! Used to decide whether `ERRORS.md` row 19 (the `iter < 20` cap) is a
//! reachable rejection at all. Run with `--ignored --nocapture`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(align(4))]
struct Buf([u8; 20]);

#[test]
#[ignore]
fn search_max_iterations() {
    let p = &pairs()[0];
    let c = &p.c;
    let mut rng = Rng::new(0xABCDEF);
    let mut best = -1i32;
    let mut hist = [0usize; 32];
    let mut best_desc = String::new();
    for round in 0..2_000_000u64 {
        let ta = VALID_TYPES[(rng.below(3)) as usize];
        let tb = VALID_TYPES[(rng.below(3)) as usize];
        let (sa, sb) = match rng.below(5) {
            0 => (Shape::random(&mut rng, ta, 1.0), Shape::random(&mut rng, tb, 1.0)),
            1 => (Shape::random(&mut rng, ta, 1e6), Shape::random(&mut rng, tb, 1e-6)),
            2 => (Shape::random_degenerate(&mut rng, ta, 10.0), Shape::random_degenerate(&mut rng, tb, 10.0)),
            3 => (Shape::random_extreme(&mut rng, ta), Shape::random_extreme(&mut rng, tb)),
            _ => (Shape::random(&mut rng, ta, 100.0), Shape::random(&mut rng, tb, 100.0)),
        };
        let ax = rng.xform_weird(50.0);
        let bx = rng.xform_weird(50.0);
        let use_ax = rng.bool();
        let use_bx = rng.bool();
        // Also try a hostile cache, since it seeds the starting simplex.
        let use_cache = rng.below(3) == 0;
        let mut cache = c2GJKCache {
            metric: rng.special_no_nan(),
            count: rng.below(4) as c_int,
            iA: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
            iB: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
            div: rng.special_no_nan(),
        };
        let ba = Buf(sa.bytes());
        let bb = Buf(sb.bytes());
        let mut it: c_int = -1;
        unsafe {
            (c.c2GJK)(
                ba.0.as_ptr() as *const c_void,
                sa.ty(),
                if use_ax { &ax } else { std::ptr::null() },
                bb.0.as_ptr() as *const c_void,
                sb.ty(),
                if use_bx { &bx } else { std::ptr::null() },
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                (rng.below(2)) as c_int,
                &mut it,
                if use_cache { &mut cache } else { std::ptr::null_mut() },
            );
        }
        if (0..32).contains(&it) {
            hist[it as usize] += 1;
        }
        if it > best {
            best = it;
            best_desc = format!("round={round} it={it} A={sa:?} B={sb:?} ax={use_ax} bx={use_bx} cache={use_cache}");
        }
    }
    println!("max iterations observed = {best}");
    println!("histogram = {hist:?}");
    println!("best case: {best_desc}");
}
