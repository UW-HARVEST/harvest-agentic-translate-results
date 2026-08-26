//! Phase B — CONFIGS.md rows 10..13: `stbds_arrgrowf` / `stbds_arrfreef`.
//!
//! `stbds_arrgrowf` is the lowest-level allocator of the library and the base of
//! every map: it is driven directly here, not through a convenience wrapper.

mod common;

use common::*;
use std::ffi::c_void;

/// Snapshot everything observable about an `stbds_arrgrowf` result.
#[derive(Debug, PartialEq, Eq)]
struct ArrState {
    is_null: bool,
    length: usize,
    capacity: usize,
    hash_table_null: bool,
    temp: isize,
    payload: Vec<u8>,
}

unsafe fn arr_state(a: *mut c_void, elemsize: usize, nelem: usize) -> ArrState {
    if a.is_null() {
        return ArrState {
            is_null: true,
            length: 0,
            capacity: 0,
            hash_table_null: true,
            temp: 0,
            payload: Vec::new(),
        };
    }
    let h = (a as *mut ArrayHeader).sub(1);
    ArrState {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        hash_table_null: (*h).hash_table.is_null(),
        temp: (*h).temp,
        payload: std::slice::from_raw_parts(a as *const u8, elemsize * nelem).to_vec(),
    }
}

/// Row 10 — `a == NULL` across the full cross product of
/// `elemsize` × `addlen` × `min_cap`, covering the three `min_cap` adjustment
/// branches (`min_len > min_cap`, `min_cap < 2*cap`, `min_cap < 4`).
#[test]
fn cfg_10_arrgrowf_from_null() {
    let p = Pair::new();
    for &elemsize in &[1usize, 2, 4, 8, 16, 24, 33] {
        for &addlen in &[0usize, 1, 2, 3, 4, 7, 8, 100] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 8, 17, 1000] {
                let (ac, ar) = unsafe {
                    (
                        (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap),
                        (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap),
                    )
                };
                let (sc, sr) = unsafe { (arr_state(ac, elemsize, 0), arr_state(ar, elemsize, 0)) };
                assert_eq!(
                    sc, sr,
                    "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) diverged"
                );
                unsafe {
                    if !ac.is_null() {
                        (p.c.arrfreef)(ac);
                    }
                    if !ar.is_null() {
                        (p.r.arrfreef)(ar);
                    }
                }
            }
        }
    }
}

/// Row 11 — non-NULL `a`: repeated re-grows, including the no-op branch and the
/// doubling branch, with the payload checked for preservation across `realloc`.
#[test]
fn cfg_11_arrgrowf_regrow() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_11);

    for &elemsize in &[1usize, 4, 8, 16, 24] {
        for &(addlen, min_cap) in &[
            (0usize, 0usize),
            (0, 1),
            (1, 0),
            (1, 1),
            (1, 4),
            (1, 5),
            (2, 3),
            (3, 3),
            (4, 0),
            (5, 0),
            (8, 0),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 9),
            (0, 100),
            (100, 0),
            (7, 7),
        ] {
            // bootstrap identically on both sides
            let mut ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
            let mut ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };

            // fill 4 elements with a deterministic pattern + set length/temp
            let pat = rng.bytes(elemsize * 4);
            unsafe {
                std::ptr::copy_nonoverlapping(pat.as_ptr(), ac as *mut u8, pat.len());
                std::ptr::copy_nonoverlapping(pat.as_ptr(), ar as *mut u8, pat.len());
                (*(ac as *mut ArrayHeader).sub(1)).length = 4;
                (*(ar as *mut ArrayHeader).sub(1)).length = 4;
                (*(ac as *mut ArrayHeader).sub(1)).temp = -3;
                (*(ar as *mut ArrayHeader).sub(1)).temp = -3;
            }

            // several successive grows
            for round in 0..4 {
                let al = addlen + round;
                let mc = min_cap;
                // `min_cap <= stbds_arrcap(a)` (L286) => the C returns `a`
                // itself with no realloc; both sides must agree on that.
                let expect_noop = unsafe {
                    let h = (ac as *mut ArrayHeader).sub(1);
                    let min_len = (*h).length + al;
                    min_len.max(mc) <= (*h).capacity
                };
                let (nc, nr) = unsafe {
                    (
                        (p.c.arrgrowf)(ac, elemsize, al, mc),
                        (p.r.arrgrowf)(ar, elemsize, al, mc),
                    )
                };
                if expect_noop {
                    assert!(
                        nc == ac && nr == ar,
                        "arrgrowf must return `a` unchanged when min_cap <= cap \
                         (elemsize={elemsize} addlen={al} min_cap={mc})"
                    );
                }
                ac = nc;
                ar = nr;
                let (sc, sr) =
                    unsafe { (arr_state(ac, elemsize, 4), arr_state(ar, elemsize, 4)) };
                assert_eq!(
                    sc, sr,
                    "arrgrowf(a, {elemsize}, {al}, {mc}) round {round} diverged"
                );
                assert_eq!(sc.payload, pat, "payload not preserved across realloc");
            }
            unsafe {
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

/// Row 12 — `elemsize == 0`: header-only allocation.
#[test]
fn cfg_12_arrgrowf_elemsize_zero() {
    let p = Pair::new();
    for &addlen in &[0usize, 1, 4, 100] {
        for &min_cap in &[0usize, 1, 4, 5, 1_000_000] {
            let (ac, ar) = unsafe {
                (
                    (p.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap),
                    (p.r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap),
                )
            };
            let (sc, sr) = unsafe { (arr_state(ac, 0, 0), arr_state(ar, 0, 0)) };
            assert_eq!(sc, sr, "arrgrowf(NULL, 0, {addlen}, {min_cap}) diverged");
            unsafe {
                if !ac.is_null() {
                    (p.c.arrfreef)(ac);
                }
                if !ar.is_null() {
                    (p.r.arrfreef)(ar);
                }
            }
        }
    }
}

/// Row 13 — grow / free round trips, many randomized shapes.
#[test]
fn cfg_13_arrgrowf_arrfreef_roundtrip() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_13);
    for _ in 0..400 {
        let elemsize = 1 + rng.below(40);
        let mut ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let mut ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        for _ in 0..1 + rng.below(6) {
            let addlen = rng.below(20);
            let min_cap = rng.below(40);
            unsafe {
                ac = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
                ar = (p.r.arrgrowf)(ar, elemsize, addlen, min_cap);
                // simulate arrmaybegrow's length bookkeeping
                let hc = (ac as *mut ArrayHeader).sub(1);
                let hr = (ar as *mut ArrayHeader).sub(1);
                let newlen = ((*hc).length + addlen).min((*hc).capacity);
                (*hc).length = newlen;
                (*hr).length = newlen;
            }
            let (sc, sr) = unsafe { (arr_state(ac, elemsize, 0), arr_state(ar, elemsize, 0)) };
            assert_eq!(sc, sr, "roundtrip grow diverged (elemsize={elemsize})");
        }
        unsafe {
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}
