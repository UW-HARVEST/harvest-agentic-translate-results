//! Phase B — CONFIGS.md rows 14-22: `stbds_arrgrowf` / `stbds_arrfreef`,
//! the lowest-level entry points in the library.

mod common;

use common::*;
use std::ffi::c_void;

unsafe fn hdr(a: *mut c_void) -> (usize, usize, bool, isize) {
    let h = (a as *const u8).sub(HEADER);
    (
        std::ptr::read_unaligned(h as *const usize),
        std::ptr::read_unaligned(h.add(8) as *const usize),
        !std::ptr::read_unaligned(h.add(16) as *const *const u8).is_null(),
        std::ptr::read_unaligned(h.add(24) as *const isize),
    )
}

/// rows 14-16, 19, 21: fresh allocations for every `(addlen, min_cap, elemsize)`
/// class the C code distinguishes.
#[test]
fn cfg_14_16_19_21_arrgrowf_fresh() {
    let _g = lock();
    let (c, r) = libs();

    for elemsize in [0usize, 1, 2, 4, 8, 12, 16, 24, 64] {
        for addlen in [0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 8, 16, 1000] {
                unsafe {
                    let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let what = format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
                    same(&format!("{what} null-ness"), ca.is_null(), ra.is_null());
                    if ca.is_null() {
                        // row 15: addlen == 0 && min_cap == 0 => NULL, no alloc
                        assert!(addlen == 0 && min_cap == 0, "{what} unexpectedly NULL");
                        continue;
                    }
                    same(&format!("{what} header"), hdr(ca), hdr(ra));
                    (c.arrfreef)(ca);
                    (r.arrfreef)(ra);
                }
            }
        }
    }
}

/// row 17: `min_cap <= arrcap(a)` — early return, pointer and header untouched.
#[test]
fn cfg_17_arrgrowf_early_return() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            same("row17 initial header", hdr(ca), hdr(ra));
            for min_cap in [0usize, 1, 4, 8, 15, 16] {
                let ca2 = (c.arrgrowf)(ca, elemsize, 0, min_cap);
                let ra2 = (r.arrgrowf)(ra, elemsize, 0, min_cap);
                same("row17 identity C", ca2 == ca, ra2 == ra);
                assert!(ca2 == ca, "row17: C reallocated on an early-return call");
                same("row17 header", hdr(ca2), hdr(ra2));
                ca = ca2;
                ra = ra2;
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

/// rows 18-20: the doubling branch, the exact-`min_cap` branch and a long
/// `addlen = 1` growth chain, for every element size.
#[test]
fn cfg_18_20_arrgrowf_growth_chain() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0xA0);

    unsafe {
        for elemsize in [1usize, 4, 8, 12, 16, 24] {
            // addlen = 1 chain: 0 -> 4 -> 8 -> 16 -> ... (doubling branch)
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            same("row20 first", hdr(ca), hdr(ra));
            let mut len = 0usize;
            for step in 0..12 {
                // emulate arrmaybegrow + length++ the way stbds_arrput does
                len += 1;
                let (_, cap, _, _) = hdr(ca);
                if len > cap {
                    ca = (c.arrgrowf)(ca, elemsize, 1, 0);
                    ra = (r.arrgrowf)(ra, elemsize, 1, 0);
                }
                std::ptr::write_unaligned((ca as *mut u8).sub(HEADER) as *mut usize, len);
                std::ptr::write_unaligned((ra as *mut u8).sub(HEADER) as *mut usize, len);
                same(&format!("row20 step {step}"), hdr(ca), hdr(ra));
            }
            // row 19: min_cap >= 2*cap -> exact min_cap
            for target in [64usize, 1000, 4096] {
                ca = (c.arrgrowf)(ca, elemsize, 0, target);
                ra = (r.arrgrowf)(ra, elemsize, 0, target);
                same(&format!("row19 target {target}"), hdr(ca), hdr(ra));
            }
            // row 18: min_cap < 2*cap -> doubling
            for _ in 0..6 {
                let (_, cap, _, _) = hdr(ca);
                let target = cap + 1 + rng.below(cap.max(1));
                ca = (c.arrgrowf)(ca, elemsize, 0, target);
                ra = (r.arrgrowf)(ra, elemsize, 0, target);
                same(&format!("row18 target {target}"), hdr(ca), hdr(ra));
            }
            // random addlen growth
            for _ in 0..30 {
                let addlen = rng.range(0, 64);
                let min_cap = rng.below(4096);
                ca = (c.arrgrowf)(ca, elemsize, addlen, min_cap);
                ra = (r.arrgrowf)(ra, elemsize, addlen, min_cap);
                same(&format!("row20 rnd({addlen},{min_cap})"), hdr(ca), hdr(ra));
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

/// row 22: grow then `stbds_arrfreef` on a valid array — must not fault, and
/// the payload bytes written by the caller survive every realloc identically.
#[test]
fn cfg_22_arrgrowf_payload_survives() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0xA2);
    unsafe {
        for elemsize in [8usize, 16] {
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let mut len = 0usize;
            for _ in 0..200 {
                let (_, cap, _, _) = hdr(ca);
                if len + 1 > cap {
                    ca = (c.arrgrowf)(ca, elemsize, 1, 0);
                    ra = (r.arrgrowf)(ra, elemsize, 1, 0);
                }
                let payload = rng.bytes(elemsize);
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ca as *mut u8).add(len * elemsize),
                    elemsize,
                );
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ra as *mut u8).add(len * elemsize),
                    elemsize,
                );
                len += 1;
                std::ptr::write_unaligned((ca as *mut u8).sub(HEADER) as *mut usize, len);
                std::ptr::write_unaligned((ra as *mut u8).sub(HEADER) as *mut usize, len);
                same(
                    "row22 array state",
                    snap_arr(ca, elemsize, KeyKind::Binary, false),
                    snap_arr(ra, elemsize, KeyKind::Binary, false),
                );
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}
