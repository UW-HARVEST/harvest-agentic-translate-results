//! Phase B — CONFIGS.md rows 13..=17: `stbds_arrgrowf` / `stbds_arrfreef`,
//! the lowest-level entry points in the library.

mod common;
use common::*;
use std::ffi::c_void;

const ELEMSIZES: &[usize] = &[1, 4, 8, 16, 20];
/// `elemsize == 0` is a legal (if useless) argument: `elemsize*min_cap` is 0 so
/// only the header is allocated.  Included in the from-NULL cross product.
const ELEMSIZES_WITH_ZERO: &[usize] = &[0, 1, 4, 8, 16, 20, 24, 32];

/// row 13 — `a == NULL`, full cross product of elemsize / addlen / min_cap
#[test]
fn r13_arrgrowf_from_null_cross_product() {
    let p = common::libs();
    for &elemsize in ELEMSIZES_WITH_ZERO {
        for addlen in [0usize, 1, 2, 3, 4, 5, 7, 16, 33] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 8, 17, 64] {
                let ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let sc = unsafe { arr_snap(ac, elemsize, 0) };
                let sr = unsafe { arr_snap(ar, elemsize, 0) };
                assert_eq!(
                    sc, sr,
                    "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) header mismatch"
                );
                assert_eq!(
                    ac.is_null(),
                    ar.is_null(),
                    "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) null-ness mismatch"
                );
                if !ac.is_null() {
                    unsafe { (p.c.arrfreef)(ac) };
                }
                if !ar.is_null() {
                    unsafe { (p.r.arrfreef)(ar) };
                }
            }
        }
    }
}

/// row 14 — `min_cap <= arrcap` early return: same pointer, header untouched
#[test]
fn r14_arrgrowf_early_return_identity() {
    let p = common::libs();
    for &elemsize in ELEMSIZES {
        // create a capacity-8 array on both sides
        let mut ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        let mut ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        // give it a length and a payload
        unsafe {
            let hc = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
            let hr = (ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
            (*hc).length = 5;
            (*hr).length = 5;
            for i in 0..(5 * elemsize) {
                *(ac as *mut u8).add(i) = (i as u8).wrapping_mul(7).wrapping_add(3);
                *(ar as *mut u8).add(i) = (i as u8).wrapping_mul(7).wrapping_add(3);
            }
        }
        let before_c = unsafe { arr_snap(ac, elemsize, 5) };
        let before_r = unsafe { arr_snap(ar, elemsize, 5) };
        assert_eq!(before_c, before_r);

        for (addlen, min_cap) in [
            (0usize, 0usize),
            (0, 1),
            (0, 5),
            (0, 8),
            (1, 0),
            (2, 0),
            (3, 8),
            (1, 6),
        ] {
            let nc = unsafe { (p.c.arrgrowf)(ac, elemsize, addlen, min_cap) };
            let nr = unsafe { (p.r.arrgrowf)(ar, elemsize, addlen, min_cap) };
            // min_len = 5 + addlen <= 8 and min_cap <= 8 -> early return
            assert_eq!(nc, ac, "C should have returned the same pointer");
            assert_eq!(nr, ar, "Rust should have returned the same pointer");
            let sc = unsafe { arr_snap(nc, elemsize, 5) };
            let sr = unsafe { arr_snap(nr, elemsize, 5) };
            assert_eq!(sc, before_c, "C header changed on early return");
            assert_eq!(sr, before_r, "Rust header changed on early return");
            assert_eq!(sc, sr);
            ac = nc;
            ar = nr;
        }
        unsafe {
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

/// row 15 — the doubling path: repeated `arrput`-like growth, capacity trace
#[test]
fn r15_arrgrowf_doubling_trace() {
    let p = common::libs();
    for &elemsize in ELEMSIZES {
        let mut ac: *mut c_void = std::ptr::null_mut();
        let mut ar: *mut c_void = std::ptr::null_mut();
        for n in 0..200usize {
            // emulate stbds_arrmaybegrow(a,1) + a[len++] = v
            let need_c = unsafe {
                ac.is_null() || {
                    let h = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    (*h).length + 1 > (*h).capacity
                }
            };
            let need_r = unsafe {
                ar.is_null() || {
                    let h = (ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    (*h).length + 1 > (*h).capacity
                }
            };
            assert_eq!(need_c, need_r, "grow decision diverged at n={n}");
            if need_c {
                ac = unsafe { (p.c.arrgrowf)(ac, elemsize, 1, 0) };
                ar = unsafe { (p.r.arrgrowf)(ar, elemsize, 1, 0) };
            }
            unsafe {
                let hc = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                let hr = (ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                let ic = (*hc).length;
                (*hc).length = ic + 1;
                (*hr).length = ic + 1;
                for k in 0..elemsize {
                    let b = (n as u8).wrapping_mul(31).wrapping_add(k as u8);
                    *(ac as *mut u8).add(ic * elemsize + k) = b;
                    *(ar as *mut u8).add(ic * elemsize + k) = b;
                }
            }
            let sc = unsafe { arr_snap(ac, elemsize, n + 1) };
            let sr = unsafe { arr_snap(ar, elemsize, n + 1) };
            assert_eq!(sc, sr, "elemsize={elemsize} n={n}");
        }
        unsafe {
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

/// row 16 — `min_cap >= 2*arrcap` (jump growth), randomized
#[test]
fn r16_arrgrowf_jump_growth() {
    let p = common::libs();
    let mut rng = Rng::new(0xBEEF_0016);
    for &elemsize in ELEMSIZES {
        for _ in 0..60 {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            let mut len = 0usize;
            for _ in 0..10 {
                let min_cap = len + 1 + rng.below(500);
                ac = unsafe { (p.c.arrgrowf)(ac, elemsize, 0, min_cap) };
                ar = unsafe { (p.r.arrgrowf)(ar, elemsize, 0, min_cap) };
                let sc0 = unsafe { arr_snap(ac, elemsize, len) };
                let sr0 = unsafe { arr_snap(ar, elemsize, len) };
                assert_eq!(sc0, sr0, "elemsize={elemsize} min_cap={min_cap}");
                // extend the length and payload up to the new capacity
                let newlen = sc0.capacity.min(len + 1 + rng.below(50));
                unsafe {
                    let hc = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    let hr = (ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    (*hc).length = newlen;
                    (*hr).length = newlen;
                    for i in (len * elemsize)..(newlen * elemsize) {
                        let b = (rng.next_u64() >> 20) as u8;
                        *(ac as *mut u8).add(i) = b;
                        *(ar as *mut u8).add(i) = b;
                    }
                }
                len = newlen;
                let sc = unsafe { arr_snap(ac, elemsize, len) };
                let sr = unsafe { arr_snap(ar, elemsize, len) };
                assert_eq!(sc, sr);
            }
            unsafe {
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

/// row 17 — grow N times with random `addlen`, full payload compared, then free
#[test]
fn r17_arrgrowf_random_addlen_sequences() {
    let p = common::libs();
    let mut rng = Rng::new(0xBEEF_0017);
    for &elemsize in ELEMSIZES {
        for _ in 0..80 {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            let mut len = 0usize;
            for _ in 0..25 {
                let addlen = rng.below(9);
                let min_cap = if rng.next_u64() & 1 == 0 {
                    0
                } else {
                    rng.below(40)
                };
                // emulate stbds_arraddnptr: maybegrow(a, addlen) then length += addlen
                let need = ac.is_null()
                    || unsafe {
                        let h = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                        (*h).length + addlen > (*h).capacity
                    };
                if need || min_cap > 0 {
                    ac = unsafe { (p.c.arrgrowf)(ac, elemsize, addlen, min_cap) };
                    ar = unsafe { (p.r.arrgrowf)(ar, elemsize, addlen, min_cap) };
                }
                if ac.is_null() {
                    assert!(ar.is_null());
                    continue;
                }
                let cap = unsafe { arr_snap(ac, elemsize, 0).capacity };
                let newlen = (len + addlen).min(cap);
                unsafe {
                    let hc = (ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    let hr = (ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
                    (*hc).length = newlen;
                    (*hr).length = newlen;
                    for i in (len * elemsize)..(newlen * elemsize) {
                        let b = (rng.next_u64() >> 20) as u8;
                        *(ac as *mut u8).add(i) = b;
                        *(ar as *mut u8).add(i) = b;
                    }
                }
                len = newlen;
                let sc = unsafe { arr_snap(ac, elemsize, len) };
                let sr = unsafe { arr_snap(ar, elemsize, len) };
                assert_eq!(
                    sc, sr,
                    "elemsize={elemsize} addlen={addlen} min_cap={min_cap} len={len}"
                );
            }
            if !ac.is_null() {
                unsafe {
                    (p.c.arrfreef)(ac);
                    (p.r.arrfreef)(ar);
                }
            }
        }
    }
}
