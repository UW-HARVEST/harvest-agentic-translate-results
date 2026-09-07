//! Phase B rows 11-17: `stbds_arrgrowf` / `stbds_arrfreef` (the lowest-level
//! allocator entry point of the library).
mod common;
use common::*;
use std::ffi::c_void;

/// Header view helpers that only look at initialised fields.
unsafe fn hdr(a: *mut c_void) -> ArrayHeader {
    *((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)
}

/// `None` when the library returned NULL (which `stbds_arrgrowf` does for
/// `a == NULL, addlen == 0, min_cap == 0`: `min_cap <= arrcap(NULL) == 0`).
fn describe(a: *mut c_void) -> Option<(usize, usize, isize, bool)> {
    if a.is_null() {
        return None;
    }
    unsafe {
        let h = hdr(a);
        Some((h.length, h.capacity, h.temp, h.hash_table.is_null()))
    }
}

/// `stbds_arrfreef(NULL)` is UB in the C (`free((char*)NULL - 32)`), so skip.
unsafe fn free_both(ca: *mut c_void, ra: *mut c_void) {
    let l = libs();
    if !ca.is_null() {
        (l.c.arrfreef)(ca);
    }
    if !ra.is_null() {
        (l.r.arrfreef)(ra);
    }
}

// row 11: a == NULL, addlen == 0, sweep min_cap and elemsize
#[test]
fn row11_grow_from_null_mincap_sweep() {
    let _s = session(0x31415926);
    let l = libs();
    for &elemsize in &[1usize, 2, 3, 4, 7, 8, 16, 32, 64] {
        for &min_cap in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 100, 4095, 65536] {
            unsafe {
                let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                assert_eq!(ca.is_null(), ra.is_null(), "NULL-ness must match");
                assert_eq!(
                    describe(ca),
                    describe(ra),
                    "elemsize={elemsize} min_cap={min_cap}"
                );
                if min_cap == 0 {
                    assert!(ca.is_null(), "min_cap 0 from NULL must return NULL");
                }
                free_both(ca, ra);
            }
        }
    }
}

// row 12: a == NULL, addlen sweep, min_cap == 0
#[test]
fn row12_grow_from_null_addlen_sweep() {
    let _s = session(0x31415926);
    let l = libs();
    for &elemsize in &[1usize, 4, 8, 16, 24] {
        for addlen in 0..65usize {
            unsafe {
                let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                assert_eq!(ca.is_null(), ra.is_null(), "NULL-ness must match");
                assert_eq!(
                    describe(ca),
                    describe(ra),
                    "elemsize={elemsize} addlen={addlen}"
                );
                free_both(ca, ra);
            }
        }
    }
}

// row 13: request that already fits => byte-identical no-op, same pointer back
#[test]
fn row13_grow_noop_when_it_fits() {
    let _s = session(0x31415926);
    let l = libs();
    let elemsize = 8usize;
    unsafe {
        let mut ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        let mut ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        // set a known length so `arrlen` participates in min_len
        (*((ca as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = 5;
        (*((ra as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = 5;
        for &(addlen, min_cap) in &[
            (0usize, 0usize),
            (0, 1),
            (0, 16),
            (1, 0),
            (11, 0),
            (0, 15),
            (5, 10),
        ] {
            let cb = (l.c.arrgrowf)(ca, elemsize, addlen, min_cap);
            let rb = (l.r.arrgrowf)(ra, elemsize, addlen, min_cap);
            assert_eq!(cb, ca, "C must return the same pointer (addlen={addlen} min_cap={min_cap})");
            assert_eq!(rb, ra, "Rust must return the same pointer (addlen={addlen} min_cap={min_cap})");
            assert_eq!(describe(cb), describe(rb));
            ca = cb;
            ra = rb;
        }
        free_both(ca, ra);
    }
}

// row 14: request cap+1 => the `2*cap` doubling floor kicks in
#[test]
fn row14_grow_doubling_floor() {
    let _s = session(0x31415926);
    let l = libs();
    for &elemsize in &[4usize, 8, 16] {
        unsafe {
            let mut ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let mut ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            for _ in 0..14 {
                let cap = hdr(ca).capacity;
                assert_eq!(cap, hdr(ra).capacity);
                ca = (l.c.arrgrowf)(ca, elemsize, 0, cap + 1);
                ra = (l.r.arrgrowf)(ra, elemsize, 0, cap + 1);
                assert_eq!(describe(ca), describe(ra), "elemsize={elemsize} cap={cap}");
                assert_eq!(hdr(ca).capacity, 2 * cap, "doubling floor");
            }
            free_both(ca, ra);
        }
    }
}

// row 15: min_cap far beyond 2*cap => exact min_cap
#[test]
fn row15_grow_exact_when_far() {
    let _s = session(0x31415926);
    let l = libs();
    let elemsize = 8usize;
    unsafe {
        let mut ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let mut ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        for &min_cap in &[9usize, 100, 1000, 100_000, 250_001] {
            ca = (l.c.arrgrowf)(ca, elemsize, 0, min_cap);
            ra = (l.r.arrgrowf)(ra, elemsize, 0, min_cap);
            assert_eq!(describe(ca), describe(ra), "min_cap={min_cap}");
            assert_eq!(hdr(ca).capacity, min_cap);
        }
        free_both(ca, ra);
    }
}

// row 16: randomized grow chain, element payloads compared each step
#[test]
fn row16_grow_random_chain() {
    let _s = session(0x31415926);
    let l = libs();
    let mut rng = Rng::with(16);
    for trial in 0..40 {
        let elemsize = [1usize, 2, 4, 8, 12, 16, 24, 32][(trial % 8) as usize];
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            let mut model: Vec<u8> = Vec::new(); // logical payload
            for _ in 0..60 {
                // addlen >= 1 guarantees min_len >= 1 so NULL is never returned
                let addlen = 1 + rng.below(8) as usize;
                let min_cap = rng.below(40) as usize;
                ca = (l.c.arrgrowf)(ca, elemsize, addlen, min_cap);
                ra = (l.r.arrgrowf)(ra, elemsize, addlen, min_cap);
                assert_eq!(describe(ca), describe(ra), "elemsize={elemsize}");

                // grow the logical length by addlen, filling deterministically
                let old_len = hdr(ca).length;
                let new_len = (old_len + addlen).min(hdr(ca).capacity);
                for i in old_len..new_len {
                    for k in 0..elemsize {
                        let b = ((i * 31 + k * 7 + trial) & 0xff) as u8;
                        *(ca as *mut u8).add(i * elemsize + k) = b;
                        *(ra as *mut u8).add(i * elemsize + k) = b;
                        model.push(b);
                    }
                }
                (*((ca as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = new_len;
                (*((ra as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = new_len;

                let cs = std::slice::from_raw_parts(ca as *const u8, new_len * elemsize);
                let rs = std::slice::from_raw_parts(ra as *const u8, new_len * elemsize);
                assert_eq!(cs, rs, "payload mismatch");
                assert_eq!(cs, &model[..new_len * elemsize], "payload vs model");
            }
            free_both(ca, ra);
        }
    }
}

// row 17: arrfreef on a live array (valgrind-free smoke: must not corrupt heap)
#[test]
fn row17_arrfreef() {
    let _s = session(0x31415926);
    let l = libs();
    for &elemsize in &[1usize, 8, 64] {
        for _ in 0..200 {
            unsafe {
                let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
                let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
                assert_eq!(describe(ca), describe(ra));
                free_both(ca, ra);
            }
        }
    }
}
